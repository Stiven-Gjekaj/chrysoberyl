//! The one format sniff that decides whether a file is an animation.
//!
//! Every format branch lives inside `is_animation`. A caller asks one
//! question and gets one answer; it never needs to know which format the
//! file turned out to hold.

use std::path::Path;

use crate::{AnimationError, open_guessed};

/// Return true when the file at `path` holds more than one frame this
/// crate can decode.
///
/// The format is guessed from the file's content, never from its
/// extension, the same rule `chrys_source_raster::decode::decode_guarded`
/// already follows. A GIF is always an animation for this adapter's
/// purposes. A PNG is one when `PngDecoder::is_apng()` returns true. A
/// WebP is one when `WebPDecoder::has_animation()` returns true. Anything
/// else, including a still PNG, JPEG, WebP or TIFF, is false.
pub fn is_animation(path: &Path) -> Result<bool, AnimationError> {
    let (format, file) = open_guessed(path)?;

    match format {
        Some(image::ImageFormat::Gif) => Ok(true),
        Some(image::ImageFormat::Png) => {
            let decoder = image::codecs::png::PngDecoder::new(file).map_err(|source| {
                AnimationError::Decode {
                    path: path.to_path_buf(),
                    index: 0,
                    message: source.to_string(),
                }
            })?;
            decoder.is_apng().map_err(|source| AnimationError::Decode {
                path: path.to_path_buf(),
                index: 0,
                message: source.to_string(),
            })
        }
        Some(image::ImageFormat::WebP) => {
            let decoder = image::codecs::webp::WebPDecoder::new(file).map_err(|source| {
                AnimationError::Decode {
                    path: path.to_path_buf(),
                    index: 0,
                    message: source.to_string(),
                }
            })?;
            Ok(decoder.has_animation())
        }
        _ => Ok(false),
    }
}
