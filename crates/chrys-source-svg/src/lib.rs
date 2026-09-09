//! The SVG adapter: a CPU rasterizer for one SVG document into one `Frame`.
//!
//! This is the only crate that knows a file might be an SVG document. It
//! depends on `chrys-source` for the `Frame` and `Source` shapes and on
//! `resvg` (with `default-features = false, features = ["text"]`) for the
//! parse and the raster. `resvg`'s `system-fonts` feature is off, so the
//! host machine's own font database is not merely unused: the code path
//! that would read it is not compiled into this binary at all. Every
//! glyph this crate renders comes from `fonts::pinned_fontdb`, the one
//! font this repository commits.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrys_source::{Frame, Source};

pub mod fonts;
pub mod sniff;

pub use sniff::looks_like_svg;

/// Resource limits applied to an SVG load, so a crafted document cannot
/// force an unbounded allocation before this crate has read a single
/// pixel.
///
/// `max_width`, `max_height` and `max_alloc` default to the same numbers
/// `chrys_source_raster::DecodeLimits::default()` uses, so a declared SVG
/// canvas is rejected the same way an oversized raster image already is.
/// `max_file_bytes` has no raster-crate counterpart, because a raster
/// decoder is bounded by its own declared pixel dimensions long before its
/// file size matters; an SVG document is XML text, so its own file size is
/// the first and cheapest thing to bound before a byte of it is parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvgLimits {
    /// The largest width, in pixels, this crate will rasterize.
    pub max_width: u32,
    /// The largest height, in pixels, this crate will rasterize.
    pub max_height: u32,
    /// The largest sum of allocations, in bytes, this crate will make
    /// while rasterizing one document.
    pub max_alloc: u64,
    /// The largest file size, in bytes, this crate will read.
    pub max_file_bytes: u64,
}

impl Default for SvgLimits {
    fn default() -> Self {
        SvgLimits {
            max_width: 16384,
            max_height: 16384,
            max_alloc: 512 * 1024 * 1024,
            max_file_bytes: 8 * 1024 * 1024,
        }
    }
}

/// A `Source` that rasterizes one SVG document into one `Frame`.
pub struct SvgSource {
    limits: SvgLimits,
}

impl SvgSource {
    /// Build an `SvgSource` with the default limits.
    pub fn new() -> Self {
        SvgSource {
            limits: SvgLimits::default(),
        }
    }

    /// Build an `SvgSource` with the given limits.
    pub fn with_limits(limits: SvgLimits) -> Self {
        SvgSource { limits }
    }
}

impl Default for SvgSource {
    fn default() -> Self {
        Self::new()
    }
}

/// The error an `SvgSource` returns on a failed load.
#[derive(Debug, thiserror::Error)]
pub enum SvgError {
    /// The path could not be read from disk.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// The file is larger than `SvgLimits::max_file_bytes`.
    #[error("{path} is {size} bytes, which exceeds the limit of {limit} bytes")]
    FileTooLarge {
        /// The path that exceeded the file-size limit.
        path: PathBuf,
        /// The size that was measured, in bytes.
        size: u64,
        /// The limit that was exceeded.
        limit: u64,
    },
    /// The document did not parse as an SVG.
    #[error("cannot parse {path}: {message}")]
    Parse {
        /// The path that failed to parse.
        path: PathBuf,
        /// The parser's own error message.
        message: String,
    },
    /// The declared canvas is larger than `SvgLimits::max_width` or
    /// `SvgLimits::max_height`.
    #[error(
        "{path} declares a {width}x{height} canvas, which exceeds the limit of {max_width}x{max_height}"
    )]
    CanvasTooLarge {
        /// The path that declared the oversized canvas.
        path: PathBuf,
        /// The declared width, in pixels.
        width: u32,
        /// The declared height, in pixels.
        height: u32,
        /// The width limit that was exceeded.
        max_width: u32,
        /// The height limit that was exceeded.
        max_height: u32,
    },
    /// The declared canvas would allocate more than `SvgLimits::max_alloc`
    /// bytes.
    #[error(
        "{path} declares a {width}x{height} canvas, which would allocate {alloc} bytes, exceeding the limit of {max_alloc} bytes"
    )]
    AllocTooLarge {
        /// The path that declared the oversized canvas.
        path: PathBuf,
        /// The declared width, in pixels.
        width: u32,
        /// The declared height, in pixels.
        height: u32,
        /// The number of bytes the declared canvas would allocate.
        alloc: u64,
        /// The allocation limit that was exceeded.
        max_alloc: u64,
    },
    /// The declared canvas is empty: `tiny_skia::Pixmap::new` refused it.
    #[error("{path} declares an empty canvas")]
    EmptyCanvas {
        /// The path that declared the empty canvas.
        path: PathBuf,
    },
}

/// Read `path` through a `Read::take`-bounded reader capped at
/// `max_file_bytes`, and refuse with `SvgError::FileTooLarge` when the read
/// itself produces more bytes than the cap allows.
///
/// This bound applies to what was actually read, not to a length the file
/// merely declares about itself in its metadata: a length read from
/// metadata and a length read from the file are two measurements of two
/// moments, and only this one bounds what this process actually allocated
/// (AGENTS.md: "a size is not a state"). `SvgSource::load` calls this after
/// its own metadata check; it is a standalone function, not a private
/// helper, so a test can call it directly and prove the bound holds even
/// when the declared and the actual length disagree.
pub fn bounded_read(path: &Path, max_file_bytes: u64) -> Result<Vec<u8>, SvgError> {
    let path_buf = path.to_path_buf();
    let mut file = File::open(path).map_err(|source| SvgError::Io {
        path: path_buf.clone(),
        source,
    })?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(max_file_bytes + 1)
        .read_to_end(&mut buffer)
        .map_err(|source| SvgError::Io {
            path: path_buf.clone(),
            source,
        })?;
    if buffer.len() as u64 > max_file_bytes {
        return Err(SvgError::FileTooLarge {
            path: path_buf,
            size: buffer.len() as u64,
            limit: max_file_bytes,
        });
    }
    Ok(buffer)
}

/// Build the `usvg::Options` every load uses: the pinned font database,
/// with the pinned family's own name set as the fallback family, so a
/// text node naming no family, or a family this database does not carry,
/// still resolves to a glyph this repository chose rather than to
/// nothing. `usvg::Options::default()` names a family this one-font
/// database cannot supply, so this fallback is set explicitly, from the
/// database's own contents rather than a string literal typed twice.
fn build_options() -> resvg::usvg::Options<'static> {
    let db = fonts::pinned_fontdb();
    let family = fonts::pinned_family_name(&db);
    resvg::usvg::Options {
        fontdb: Arc::new(db),
        font_family: family,
        ..resvg::usvg::Options::default()
    }
}

impl Source for SvgSource {
    type Error = SvgError;

    fn load(&self, path: &Path) -> Result<Vec<Frame>, Self::Error> {
        let path_buf = path.to_path_buf();

        // 1. The fast, clear refusal: the file system's own declared
        // length, read before a single byte of the file is opened.
        let metadata = std::fs::metadata(path).map_err(|source| SvgError::Io {
            path: path_buf.clone(),
            source,
        })?;
        if metadata.len() > self.limits.max_file_bytes {
            return Err(SvgError::FileTooLarge {
                path: path_buf,
                size: metadata.len(),
                limit: self.limits.max_file_bytes,
            });
        }

        // 2. The second, bounding measurement: a length read from
        // metadata and a length read from the file are two measurements
        // of two moments, and only this one bounds what this process
        // actually allocated (AGENTS.md: a size is not a state).
        let buffer = bounded_read(path, self.limits.max_file_bytes)?;

        // 3. Parse.
        let options = build_options();
        let tree =
            resvg::usvg::Tree::from_data(&buffer, &options).map_err(|source| SvgError::Parse {
                path: path_buf.clone(),
                message: source.to_string(),
            })?;

        // 4. Refuse an oversized declared canvas before any allocation.
        // `usvg::Options` carries no size-limit field of its own, so this
        // check is the only thing between a document declaring a million
        // pixels a side and a `Pixmap` of that size.
        let int_size = tree.size().to_int_size();
        let (width, height) = (int_size.width(), int_size.height());
        if width > self.limits.max_width || height > self.limits.max_height {
            return Err(SvgError::CanvasTooLarge {
                path: path_buf,
                width,
                height,
                max_width: self.limits.max_width,
                max_height: self.limits.max_height,
            });
        }
        let alloc = (width as u64) * (height as u64) * 4;
        if alloc > self.limits.max_alloc {
            return Err(SvgError::AllocTooLarge {
                path: path_buf,
                width,
                height,
                alloc,
                max_alloc: self.limits.max_alloc,
            });
        }

        // 5. Allocate.
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(width, height).ok_or_else(|| SvgError::EmptyCanvas {
                path: path_buf.clone(),
            })?;

        // 6. Render.
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::identity(),
            &mut pixmap.as_mut(),
        );

        // 7. `take_demultiplied` is the whole of the alpha conversion:
        // `tiny_skia::Pixmap` stores premultiplied alpha, and `Frame`
        // documents straight alpha. No per-pixel divide loop is written
        // here: a hand-rolled divide needs its own zero-alpha case or it
        // divides by zero, and this one call is already correct there.
        let pixels = pixmap.take_demultiplied();

        Ok(vec![Frame {
            pixels,
            width,
            height,
            index: 0,
            hints: Vec::new(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_match_the_raster_crates_own_numbers() {
        let svg = SvgLimits::default();
        let raster = chrys_source_raster::DecodeLimits::default();
        assert_eq!(svg.max_width, raster.max_width);
        assert_eq!(svg.max_height, raster.max_height);
        assert_eq!(svg.max_alloc, raster.max_alloc);
    }

    #[test]
    fn default_max_file_bytes_is_eight_mebibytes() {
        assert_eq!(SvgLimits::default().max_file_bytes, 8 * 1024 * 1024);
    }
}
