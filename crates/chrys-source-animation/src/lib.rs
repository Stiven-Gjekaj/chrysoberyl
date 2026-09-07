//! The animation adapter: GIF, APNG and lossless animated WebP.
//!
//! This is the only crate that knows a file holds more than one composited
//! frame. It depends on `chrys-source` for the `Frame` and `Source` shapes
//! and on `image`'s `AnimationDecoder` trait for GIF, APNG and animated
//! WebP decode. It writes no compositing code of its own: every decoder
//! this crate builds already resolves its own format's disposal and blend
//! rules internally and hands back a full-canvas RGBA8 buffer through
//! `into_frames()`. This crate only reshapes that buffer into a
//! `chrys_source::Frame`.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use chrys_source::{Frame, Source};
use image::{AnimationDecoder as _, ImageDecoder as _};

pub mod sniff;

/// Resource limits applied to an animation load, so a crafted container
/// cannot force an unbounded allocation before this crate has read a
/// single frame.
///
/// `max_width`, `max_height` and `max_alloc` default to the same numbers
/// `chrys_source_raster::DecodeLimits::default()` uses, so a single huge
/// frame is rejected the same way a single huge still image already is. A
/// unit test in this module asserts the three numbers stay equal.
/// `max_frames` and `max_total_pixels` default to the same 512 and
/// 134,217,728 `chrys_source_sequence::SequenceLimits` uses, checked as
/// `into_frames()` yields rather than after the whole animation has
/// decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationLimits {
    /// The largest width, in pixels, this crate will decode.
    pub max_width: u32,
    /// The largest height, in pixels, this crate will decode.
    pub max_height: u32,
    /// The largest sum of allocations, in bytes, this crate will make
    /// while decoding one file.
    pub max_alloc: u64,
    /// The largest number of frames this crate will load from one file.
    pub max_frames: usize,
    /// The largest cumulative pixel count, summed across every frame,
    /// this crate will load from one file.
    pub max_total_pixels: usize,
}

impl Default for AnimationLimits {
    fn default() -> Self {
        AnimationLimits {
            max_width: 16384,
            max_height: 16384,
            max_alloc: 512 * 1024 * 1024,
            max_frames: 512,
            max_total_pixels: 134_217_728,
        }
    }
}

impl AnimationLimits {
    fn to_image_limits(self) -> image::Limits {
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(self.max_width);
        limits.max_image_height = Some(self.max_height);
        limits.max_alloc = Some(self.max_alloc);
        limits
    }
}

/// A `Source` that decodes a GIF, an APNG or a lossless animated WebP file
/// into a `Vec<Frame>`, one entry per already-composited animation frame.
pub struct AnimationSource {
    limits: AnimationLimits,
}

impl AnimationSource {
    /// Build an `AnimationSource` with the default limits.
    pub fn new() -> Self {
        AnimationSource {
            limits: AnimationLimits::default(),
        }
    }

    /// Build an `AnimationSource` with the given limits.
    pub fn with_limits(limits: AnimationLimits) -> Self {
        AnimationSource { limits }
    }
}

impl Default for AnimationSource {
    fn default() -> Self {
        Self::new()
    }
}

/// The error an `AnimationSource` returns on a failed load.
#[derive(Debug, thiserror::Error)]
pub enum AnimationError {
    /// The path could not be read from disk.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// The file is not an animation this crate decodes.
    #[error("{path} is not an animation this crate decodes")]
    NotAnimation {
        /// The path that is not a decodable animation.
        path: PathBuf,
    },
    /// A frame failed to decode, naming its index. Index 0 also covers a
    /// failure before any frame was read, such as a malformed container
    /// header.
    #[error("cannot decode frame {index} of {path}: {message}")]
    Decode {
        /// The path that failed to decode.
        path: PathBuf,
        /// The index of the frame that failed to decode.
        index: usize,
        /// The decoder's error message.
        message: String,
    },
    /// The frame count exceeds `AnimationLimits::max_frames`.
    #[error("{path} holds more than {limit} frames")]
    TooManyFrames {
        /// The path that exceeded the frame count limit.
        path: PathBuf,
        /// The limit that was exceeded.
        limit: usize,
    },
    /// The cumulative pixel count across every frame exceeds
    /// `AnimationLimits::max_total_pixels`.
    #[error("{path} holds more than {limit} pixels total")]
    TooManyPixels {
        /// The path that exceeded the pixel budget.
        path: PathBuf,
        /// The limit that was exceeded.
        limit: usize,
    },
}

/// Open `path`, guess its format from its content, and return the format
/// alongside the buffered file handle, ready for a format-specific decoder
/// to consume. The format is never guessed from the file's extension.
pub(crate) fn open_guessed(
    path: &Path,
) -> Result<(Option<image::ImageFormat>, BufReader<File>), AnimationError> {
    let reader = image::ImageReader::open(path).map_err(|source| AnimationError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let reader = reader
        .with_guessed_format()
        .map_err(|source| AnimationError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    let format = reader.format();
    Ok((format, reader.into_inner()))
}

impl Source for AnimationSource {
    type Error = AnimationError;

    fn load(&self, path: &Path) -> Result<Vec<Frame>, Self::Error> {
        let img_limits = self.limits.to_image_limits();
        let (format, file) = open_guessed(path)?;

        match format {
            Some(image::ImageFormat::Gif) => {
                let mut decoder = image::codecs::gif::GifDecoder::new(file).map_err(|source| {
                    AnimationError::Decode {
                        path: path.to_path_buf(),
                        index: 0,
                        message: source.to_string(),
                    }
                })?;
                decoder
                    .set_limits(img_limits)
                    .map_err(|source| AnimationError::Decode {
                        path: path.to_path_buf(),
                        index: 0,
                        message: source.to_string(),
                    })?;
                collect_frames(path, decoder.into_frames(), &self.limits)
            }
            Some(image::ImageFormat::Png) => {
                let png_decoder = image::codecs::png::PngDecoder::with_limits(file, img_limits)
                    .map_err(|source| AnimationError::Decode {
                        path: path.to_path_buf(),
                        index: 0,
                        message: source.to_string(),
                    })?;
                let is_apng = png_decoder
                    .is_apng()
                    .map_err(|source| AnimationError::Decode {
                        path: path.to_path_buf(),
                        index: 0,
                        message: source.to_string(),
                    })?;
                if !is_apng {
                    return Err(AnimationError::NotAnimation {
                        path: path.to_path_buf(),
                    });
                }
                let apng_decoder = png_decoder
                    .apng()
                    .map_err(|source| AnimationError::Decode {
                        path: path.to_path_buf(),
                        index: 0,
                        message: source.to_string(),
                    })?;
                collect_frames(path, apng_decoder.into_frames(), &self.limits)
            }
            Some(image::ImageFormat::WebP) => {
                let mut decoder =
                    image::codecs::webp::WebPDecoder::new(file).map_err(|source| {
                        AnimationError::Decode {
                            path: path.to_path_buf(),
                            index: 0,
                            message: source.to_string(),
                        }
                    })?;
                if !decoder.has_animation() {
                    return Err(AnimationError::NotAnimation {
                        path: path.to_path_buf(),
                    });
                }
                decoder
                    .set_limits(img_limits)
                    .map_err(|source| AnimationError::Decode {
                        path: path.to_path_buf(),
                        index: 0,
                        message: source.to_string(),
                    })?;
                collect_frames(path, decoder.into_frames(), &self.limits)
            }
            _ => Err(AnimationError::NotAnimation {
                path: path.to_path_buf(),
            }),
        }
    }
}

/// Pull every frame from `frames`, checking `limits` against the
/// accumulated frame count and pixel count as each frame arrives, not
/// after the whole animation has decoded. `frames` is a lazy iterator, so
/// a limit checked here works on a file declaring a million frames.
fn collect_frames(
    path: &Path,
    frames: image::Frames<'_>,
    limits: &AnimationLimits,
) -> Result<Vec<Frame>, AnimationError> {
    let mut result = Vec::new();
    let mut total_pixels: usize = 0;

    for (index, frame) in frames.enumerate() {
        if index >= limits.max_frames {
            return Err(AnimationError::TooManyFrames {
                path: path.to_path_buf(),
                limit: limits.max_frames,
            });
        }

        let frame = frame.map_err(|source| AnimationError::Decode {
            path: path.to_path_buf(),
            index,
            message: source.to_string(),
        })?;
        let buffer = frame.into_buffer();
        let (width, height) = buffer.dimensions();

        total_pixels = total_pixels.saturating_add((width as usize) * (height as usize));
        if total_pixels > limits.max_total_pixels {
            return Err(AnimationError::TooManyPixels {
                path: path.to_path_buf(),
                limit: limits.max_total_pixels,
            });
        }

        result.push(Frame {
            pixels: buffer.into_raw(),
            width,
            height,
            index,
            hints: Vec::new(),
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_limits_match_the_raster_crates_own_numbers() {
        let animation = AnimationLimits::default();
        let raster = chrys_source_raster::DecodeLimits::default();
        assert_eq!(animation.max_width, raster.max_width);
        assert_eq!(animation.max_height, raster.max_height);
        assert_eq!(animation.max_alloc, raster.max_alloc);
    }

    #[test]
    fn frame_and_pixel_limits_match_the_sequence_crates_own_numbers() {
        let limits = AnimationLimits::default();
        assert_eq!(limits.max_frames, 512);
        assert_eq!(limits.max_total_pixels, 134_217_728);
    }
}
