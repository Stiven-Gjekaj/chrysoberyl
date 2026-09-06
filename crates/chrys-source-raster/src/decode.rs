//! The single guarded decode entry point for every raster format.
//!
//! `decode_guarded` is the only place in this crate that opens an
//! `image::ImageReader` or builds an `image` decoder. It reads the file's
//! declared dimensions and rejects them against the configured limits
//! before it decodes a single pixel. This is the mitigation for the
//! decompression-bomb class of attack (CVE-2023-29408).

use std::path::{Path, PathBuf};

use image::ImageDecoder as _;

use crate::normalize::Orientation;
use crate::{DecodeLimits, RasterError};

/// Decode the raster file at `path`, with `limits` enforced before any
/// pixel buffer is allocated.
///
/// The format is guessed from the file's content, not from its extension,
/// so a file whose name does not match its bytes still decodes, and a
/// mismatched extension cannot pick a different decoder than the one the
/// content calls for.
///
/// Returns the decoded image alongside the orientation the decoder's own
/// metadata reports. This crate reads that orientation through
/// `image::ImageDecoder::orientation`, never by parsing Exif bytes by
/// hand.
pub fn decode_guarded(
    path: &Path,
    limits: &DecodeLimits,
) -> Result<(image::DynamicImage, Orientation), RasterError> {
    let path_buf = path.to_path_buf();
    let img_limits = limits.to_image_limits();

    let mut reader = image::ImageReader::open(path).map_err(|source| RasterError::Io {
        path: path_buf.clone(),
        source,
    })?;
    reader.limits(img_limits.clone());

    let reader = reader
        .with_guessed_format()
        .map_err(|source| RasterError::Io {
            path: path_buf.clone(),
            source,
        })?;

    // `into_decoder` builds the format-specific decoder and calls
    // `ImageDecoder::set_limits` on it with the limits set above, so a
    // declared width or height above the limit is rejected here, before
    // this function allocates a pixel buffer.
    let mut decoder = match reader.into_decoder() {
        Ok(decoder) => decoder,
        Err(image::ImageError::Limits(_)) => {
            let (width, height) = probe_dimensions(path).unwrap_or((0, 0));
            return Err(too_large(path_buf, width, height, limits));
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

    // Set the limits a second time, explicitly, on the decoder this
    // function built. `into_decoder` already applied them once inside the
    // `image` crate; this call is this module's own visible guard, and it
    // reads the declared dimensions this decoder found and rejects them
    // against the limits before this function decodes a single pixel.
    if let Err(source) = decoder.set_limits(img_limits) {
        let (width, height) = decoder.dimensions();
        return Err(match source {
            image::ImageError::Limits(_) => too_large(path_buf, width, height, limits),
            other => RasterError::Decode {
                path: path_buf,
                message: other.to_string(),
            },
        });
    }

    // Read the orientation through the decoder's own metadata accessor,
    // never by parsing Exif bytes in this crate. Every format SRC-01
    // names exposes it in the pinned `image` 0.25.10: PNG through its
    // default Exif-chunk-based implementation, and JPEG, WebP and TIFF
    // each through their own `orientation()` override. No format in this
    // plan defaulted to `Orientation::Normal` for lack of an accessor.
    let orientation = decoder
        .orientation()
        .map(|found| Orientation::from_exif_u16(u16::from(found.to_exif())))
        .unwrap_or(Orientation::Normal);

    let (width, height) = decoder.dimensions();
    let dynamic = image::DynamicImage::from_decoder(decoder).map_err(|source| match source {
        image::ImageError::Limits(_) => too_large(path_buf.clone(), width, height, limits),
        other => RasterError::Decode {
            path: path_buf,
            message: other.to_string(),
        },
    })?;

    Ok((dynamic, orientation))
}

fn too_large(path: PathBuf, width: u32, height: u32, limits: &DecodeLimits) -> RasterError {
    RasterError::TooLarge {
        path,
        width,
        height,
        limit: limits.max_width.max(limits.max_height),
    }
}

/// Read only the header dimensions of the file at `path`, with no width or
/// height limit applied, so a `TooLarge` error can report the true size.
/// This never decodes pixel data, so it stays safe against a crafted
/// header that claims an enormous image.
fn probe_dimensions(path: &Path) -> Option<(u32, u32)> {
    let mut reader = image::ImageReader::open(path).ok()?;
    reader.no_limits();
    let reader = reader.with_guessed_format().ok()?;
    reader.into_dimensions().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_path(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("chrys-decode-test-{}-{suffix}", std::process::id()))
    }

    #[test]
    fn decode_guarded_on_a_zero_byte_file_returns_a_decode_error() {
        let path = temp_path("zero-byte.png");
        std::fs::File::create(&path).expect("create an empty temp file");

        let result = decode_guarded(&path, &DecodeLimits::default());
        std::fs::remove_file(&path).ok();

        match result {
            Err(RasterError::Decode { .. }) => {}
            other => panic!("expected RasterError::Decode, got {other:?}"),
        }
    }

    #[test]
    fn decode_guarded_on_a_truncated_png_returns_a_decode_error() {
        let mut full = Vec::new();
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            4,
            4,
            image::Rgba([1, 2, 3, 4]),
        ))
        .write_to(
            &mut std::io::Cursor::new(&mut full),
            image::ImageFormat::Png,
        )
        .expect("encode a small PNG for this test");

        let truncated_len = full.len().min(16);
        let path = temp_path("truncated.png");
        {
            let mut file = std::fs::File::create(&path).expect("create temp file");
            file.write_all(&full[..truncated_len])
                .expect("write a truncated PNG for this test");
        }

        let result = decode_guarded(&path, &DecodeLimits::default());
        std::fs::remove_file(&path).ok();

        match result {
            Err(RasterError::Decode { .. }) => {}
            other => panic!("expected RasterError::Decode, got {other:?}"),
        }
    }

    #[test]
    fn decode_guarded_rejects_a_declared_size_above_the_configured_limit() {
        let path = temp_path("over-large.png");
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            4,
            4,
            image::Rgba([1, 2, 3, 4]),
        ))
        .save(&path)
        .expect("write a small PNG for this test");

        let tiny_limits = DecodeLimits {
            max_width: 1,
            max_height: 1,
            max_alloc: DecodeLimits::default().max_alloc,
        };

        let result = decode_guarded(&path, &tiny_limits);
        std::fs::remove_file(&path).ok();

        match result {
            Err(RasterError::TooLarge { width, height, .. }) => {
                assert_eq!((width, height), (4, 4));
            }
            other => panic!("expected RasterError::TooLarge, got {other:?}"),
        }
    }

    #[test]
    fn decode_guarded_on_the_same_path_twice_returns_identical_pixels() {
        let path = temp_path("repeat.png");
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            6,
            5,
            image::Rgba([9, 8, 7, 6]),
        ))
        .save(&path)
        .expect("write a small PNG for this test");

        let limits = DecodeLimits::default();
        let (first, _) = decode_guarded(&path, &limits).expect("first decode");
        let (second, _) = decode_guarded(&path, &limits).expect("second decode");
        std::fs::remove_file(&path).ok();

        assert_eq!(first.to_rgba8().into_raw(), second.to_rgba8().into_raw());
    }
}
