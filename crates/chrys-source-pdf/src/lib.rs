//! The PDF adapter: a CPU rasterizer for the pages of one PDF file.
//!
//! This is the only crate that knows a file can be a PDF document.
//! It uses `hayro`, a pure-Rust PDF rasterizer.
//! Each page becomes one `Frame`, so a document is a sequence.
//!
//! Three settings remove a known source of drift between machines:
//!
//! - The rasterizer runs at the scalar SIMD level.
//!   The default level is found at run time on x86-64, so two machines of
//!   the same architecture can select different code.
//! - The rasterizer runs on the calling thread only.
//! - The rasterizer uses the integer pipeline, not the `f32` pipeline.
//!
//! The fonts come from `hayro` itself. It embeds the fourteen standard
//! fonts and reads no font from the host.
//!
//! One gap stays open. `hayro` scales an embedded image at the SIMD level
//! that it finds itself, and this crate cannot set that level.
//! The determinism gate holds no image for this reason.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use chrys_source::{Frame, Source};
use fearless_simd::Level;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::kurbo::Affine;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::vello_cpu::{
    Pixmap, RasterizerSettings, RenderContext, RenderMode, RenderSettings, Resources, TargetInit,
};
use hayro::{RenderCache, render_into};

/// The number of pixels for one PDF point.
///
/// A PDF point is 1/72 inch, so this scale gives 144 pixels per inch.
/// The value is fixed. A change to it changes every PDF baseline.
pub const PIXELS_PER_POINT: f32 = 2.0;

/// The resource limits for one PDF load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfLimits {
    /// The largest page width, in pixels, after the scale.
    pub max_width: u32,
    /// The largest page height, in pixels, after the scale.
    pub max_height: u32,
    /// The largest number of pages in one document.
    pub max_pages: usize,
    /// The largest file size, in bytes, that this crate reads.
    pub max_file_bytes: u64,
}

impl Default for PdfLimits {
    fn default() -> Self {
        PdfLimits {
            max_width: 16384,
            max_height: 16384,
            max_pages: 512,
            max_file_bytes: 64 * 1024 * 1024,
        }
    }
}

/// The error that a `PdfSource` returns on a failed load.
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    /// The path cannot be read from disk.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The path that cannot be read.
        path: PathBuf,
        /// The I/O error.
        source: std::io::Error,
    },
    /// The file is larger than `PdfLimits::max_file_bytes`.
    #[error("{path} is {size} bytes, which exceeds the limit of {limit} bytes")]
    FileTooLarge {
        /// The path.
        path: PathBuf,
        /// The size that was read, in bytes.
        size: u64,
        /// The limit.
        limit: u64,
    },
    /// The file does not parse as a PDF document.
    #[error("cannot parse {path} as a PDF document: {message}")]
    Parse {
        /// The path.
        path: PathBuf,
        /// The message from the parser.
        message: String,
    },
    /// The document holds no page.
    #[error("{path} holds no page")]
    NoPages {
        /// The path.
        path: PathBuf,
    },
    /// The document holds more pages than `PdfLimits::max_pages`.
    #[error("{path} holds {pages} pages, which exceeds the limit of {limit}")]
    TooManyPages {
        /// The path.
        path: PathBuf,
        /// The number of pages.
        pages: usize,
        /// The limit.
        limit: usize,
    },
    /// One page is larger than the canvas limit, or is empty.
    #[error(
        "{path} page {page} is {width}x{height} pixels, which is outside 1x1 to {max_width}x{max_height}"
    )]
    BadPageSize {
        /// The path.
        path: PathBuf,
        /// The one-based page number.
        page: usize,
        /// The width after the scale, in pixels.
        width: u32,
        /// The height after the scale, in pixels.
        height: u32,
        /// The width limit.
        max_width: u32,
        /// The height limit.
        max_height: u32,
    },
}

/// A `Source` that rasterizes each page of one PDF file into one `Frame`.
pub struct PdfSource {
    limits: PdfLimits,
}

impl PdfSource {
    /// Make a `PdfSource` with the default limits.
    pub fn new() -> Self {
        PdfSource {
            limits: PdfLimits::default(),
        }
    }

    /// Make a `PdfSource` with the given limits.
    pub fn with_limits(limits: PdfLimits) -> Self {
        PdfSource { limits }
    }
}

impl Default for PdfSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Return true when the first 1024 bytes of `path` hold the `%PDF-` marker.
///
/// The PDF specification lets a reader find the marker anywhere in the
/// first 1024 bytes. This test reads the content only, not the extension.
pub fn looks_like_pdf(path: &Path) -> std::io::Result<bool> {
    let mut head = Vec::with_capacity(1024);
    File::open(path)?.take(1024).read_to_end(&mut head)?;
    Ok(head.windows(5).any(|window| window == b"%PDF-"))
}

/// Read `path`, and refuse a file larger than `max_file_bytes`.
///
/// The bound applies to the bytes that this function reads, not to the
/// length that the file system reports.
fn bounded_read(path: &Path, max_file_bytes: u64) -> Result<Vec<u8>, PdfError> {
    let io = |source| PdfError::Io {
        path: path.to_path_buf(),
        source,
    };
    let mut buffer = Vec::new();
    File::open(path)
        .map_err(io)?
        .take(max_file_bytes.saturating_add(1))
        .read_to_end(&mut buffer)
        .map_err(io)?;
    if buffer.len() as u64 > max_file_bytes {
        return Err(PdfError::FileTooLarge {
            path: path.to_path_buf(),
            size: buffer.len() as u64,
            limit: max_file_bytes,
        });
    }
    Ok(buffer)
}

/// The settings that make the rasterizer the same on each machine.
fn pinned_render_settings() -> RenderSettings {
    RenderSettings {
        level: Level::fallback(),
        num_threads: 0,
    }
}

impl Source for PdfSource {
    type Error = PdfError;

    fn load(&self, path: &Path) -> Result<Vec<Frame>, Self::Error> {
        let path_buf = path.to_path_buf();
        let data = bounded_read(path, self.limits.max_file_bytes)?;
        let pdf = Pdf::new(data).map_err(|error| PdfError::Parse {
            path: path_buf.clone(),
            message: format!("{error:?}"),
        })?;

        let pages = pdf.pages();
        if pages.is_empty() {
            return Err(PdfError::NoPages { path: path_buf });
        }
        if pages.len() > self.limits.max_pages {
            return Err(PdfError::TooManyPages {
                path: path_buf,
                pages: pages.len(),
                limit: self.limits.max_pages,
            });
        }

        let cache = RenderCache::new();
        let interpreter_settings = InterpreterSettings::default();
        let render_settings = hayro::RenderSettings {
            hinting: false,
            ..hayro::RenderSettings::default()
        };

        let mut frames = Vec::with_capacity(pages.len());
        for (index, page) in pages.iter().enumerate() {
            let (page_width, page_height) = page.render_dimensions();
            let width = scaled(page_width);
            let height = scaled(page_height);
            if width == 0
                || height == 0
                || width > self.limits.max_width
                || height > self.limits.max_height
            {
                return Err(PdfError::BadPageSize {
                    path: path_buf,
                    page: index + 1,
                    width,
                    height,
                    max_width: self.limits.max_width,
                    max_height: self.limits.max_height,
                });
            }
            // The limit check above keeps both values inside a u16.
            let (width16, height16) = (width as u16, height as u16);

            let mut context = RenderContext::new_with(width16, height16, pinned_render_settings());
            let transform = Affine::scale(PIXELS_PER_POINT as f64)
                * hayro::hayro_interpret::util::TransformExt::to_kurbo(
                    &page.initial_transform(true),
                );
            render_into(
                page,
                &cache,
                &interpreter_settings,
                &render_settings,
                &mut context,
                transform,
            );
            context.flush();

            let mut pixmap = Pixmap::new(width16, height16);
            context.render_with(
                &mut pixmap,
                &mut Resources::default(),
                RasterizerSettings {
                    render_mode: RenderMode::OptimizeSpeed,
                    target_init: TargetInit::Clear(WHITE),
                    ..RasterizerSettings::default()
                },
            );

            frames.push(Frame {
                pixels: pixmap.take_rgba8(hayro::vello_cpu::peniko::ImageAlphaType::Alpha),
                width,
                height,
                index,
                hints: Vec::new(),
            });
        }
        Ok(frames)
    }
}

/// Scale a page dimension in points to whole pixels.
///
/// The value rounds down, the same as the `hayro::render` function.
/// A negative or a non-finite value gives 0, which the caller refuses.
fn scaled(points: f32) -> u32 {
    let pixels = points * PIXELS_PER_POINT;
    if pixels.is_finite() && pixels >= 0.0 {
        // `as` saturates a float that is too large for a u32.
        pixels as u32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_refuses_a_value_that_is_not_finite() {
        assert_eq!(scaled(f32::NAN), 0);
        assert_eq!(scaled(f32::INFINITY), 0);
        assert_eq!(scaled(-10.0), 0);
        assert_eq!(scaled(320.0), 640);
    }

    #[test]
    fn the_render_settings_use_the_scalar_level_and_one_thread() {
        let settings = pinned_render_settings();
        assert!(settings.level.is_fallback());
        assert_eq!(settings.num_threads, 0);
    }
}
