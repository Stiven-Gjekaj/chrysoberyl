//! The raster source adapter. This is the only crate that knows a raster
//! file format. It depends on `chrys-source` and on the `image` crate, and
//! it never depends on `chrys-core`.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use chrys_source::{Frame, Source};

/// Resource limits applied to a decode, so a crafted file cannot force an
/// unbounded allocation before this crate has read a single pixel.
///
/// This is the mitigation for the decompression-bomb class of attack
/// (CVE-2023-29408): the limits are built into an `image::Limits` value and
/// handed to the reader before `decode` is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeLimits {
    /// The largest width, in pixels, this crate will decode.
    pub max_width: u32,
    /// The largest height, in pixels, this crate will decode.
    pub max_height: u32,
    /// The largest sum of allocations, in bytes, this crate will make while
    /// decoding one image.
    pub max_alloc: u64,
}

impl Default for DecodeLimits {
    fn default() -> Self {
        DecodeLimits {
            max_width: 16384,
            max_height: 16384,
            max_alloc: 512 * 1024 * 1024,
        }
    }
}

impl DecodeLimits {
    fn to_image_limits(self) -> image::Limits {
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(self.max_width);
        limits.max_image_height = Some(self.max_height);
        limits.max_alloc = Some(self.max_alloc);
        limits
    }
}

/// A `Source` that decodes a raster image file into one `Frame`.
pub struct RasterSource {
    limits: DecodeLimits,
}

impl RasterSource {
    /// Build a `RasterSource` with the default decode limits.
    pub fn new() -> Self {
        RasterSource {
            limits: DecodeLimits::default(),
        }
    }

    /// Build a `RasterSource` with the given decode limits.
    pub fn with_limits(limits: DecodeLimits) -> Self {
        RasterSource { limits }
    }
}

impl Default for RasterSource {
    fn default() -> Self {
        Self::new()
    }
}

/// The error a `RasterSource` returns on a failed load.
#[derive(Debug, thiserror::Error)]
pub enum RasterError {
    /// The path could not be read from disk.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// The file was read, but its bytes did not decode as a supported
    /// image.
    #[error("cannot decode {path}: {message}")]
    Decode {
        /// The path that failed to decode.
        path: PathBuf,
        /// The decoder's error message.
        message: String,
    },
    /// The image exceeds the configured decode limits.
    #[error("{path} is {width}x{height}, which exceeds the limit of {limit} pixels per side")]
    TooLarge {
        /// The path that exceeded the limit.
        path: PathBuf,
        /// The image width the decoder found.
        width: u32,
        /// The image height the decoder found.
        height: u32,
        /// The limit that was exceeded.
        limit: u32,
    },
    /// The file's format is not one this crate decodes.
    #[error("{path} is not a supported raster format")]
    Unsupported {
        /// The path with the unsupported format.
        path: PathBuf,
    },
}

impl Source for RasterSource {
    type Error = RasterError;

    fn load(&self, path: &Path) -> Result<Vec<Frame>, Self::Error> {
        let path_buf = path.to_path_buf();
        let img_limits = self.limits.to_image_limits();

        // Every decode call in this crate sets limits before it runs, so
        // there is no unguarded path from a file on disk to a pixel buffer.
        let mut reader = image::ImageReader::open(path).map_err(|source| RasterError::Io {
            path: path_buf.clone(),
            source,
        })?;
        reader.limits(img_limits);

        let dynamic = match reader.decode() {
            Ok(dynamic) => dynamic,
            Err(image::ImageError::Limits(_)) => {
                let (width, height) = probe_dimensions(path).unwrap_or((0, 0));
                return Err(RasterError::TooLarge {
                    path: path_buf,
                    width,
                    height,
                    limit: self.limits.max_width.max(self.limits.max_height),
                });
            }
            Err(image::ImageError::Unsupported(_)) => {
                return Err(RasterError::Unsupported { path: path_buf });
            }
            Err(source) => {
                return Err(RasterError::Decode {
                    path: path_buf,
                    message: source.to_string(),
                });
            }
        };

        let rgba = dynamic.to_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(vec![Frame {
            pixels: rgba.into_raw(),
            width,
            height,
            index: 0,
            hints: Vec::new(),
        }])
    }
}

/// Read only the header dimensions of the file at `path`, with no width or
/// height limit applied, so a `TooLarge` error can report the true size.
/// This never decodes pixel data, so it stays safe against a crafted
/// header that claims an enormous image.
fn probe_dimensions(path: &Path) -> Option<(u32, u32)> {
    let mut reader = image::ImageReader::open(path).ok()?;
    reader.no_limits();
    reader.into_dimensions().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_on_a_non_image_file_returns_decode_error() {
        let dir = std::env::temp_dir();
        let path = dir.join("chrys-source-raster-test-not-an-image.png");
        {
            let mut file = std::fs::File::create(&path).expect("create temp file");
            file.write_all(b"this is not a png file")
                .expect("write temp file");
        }

        let source = RasterSource::new();
        let result = source.load(&path);

        std::fs::remove_file(&path).ok();

        match result {
            Err(RasterError::Decode { .. }) => {}
            other => panic!("expected RasterError::Decode, got {other:?}"),
        }
    }

    #[test]
    fn load_on_a_missing_path_returns_io_error() {
        let path = Path::new("/no/such/path/chrys-does-not-exist.png");
        let source = RasterSource::new();

        match source.load(path) {
            Err(RasterError::Io { .. }) => {}
            other => panic!("expected RasterError::Io, got {other:?}"),
        }
    }

    #[test]
    fn decode_limits_default_matches_documented_values() {
        let limits = DecodeLimits::default();
        assert_eq!(limits.max_width, 16384);
        assert_eq!(limits.max_height, 16384);
        assert_eq!(limits.max_alloc, 512 * 1024 * 1024);
    }
}
