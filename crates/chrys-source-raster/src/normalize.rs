//! EXIF orientation, straight alpha, and bit-depth normalization to RGBA8.
//!
//! Every decoded image passes through `normalize_to_rgba8` before it
//! becomes a `Frame`. This is where the canonical shape the whole project
//! compares against is decided: the orientation a person sees, 8 bits per
//! channel, straight alpha.
//!
//! The returned buffer is straight alpha. No `image` crate filter may run
//! on it without converting first: at least one filter in that crate
//! assumes premultiplied alpha, and this buffer is not that.

use image::DynamicImage;

/// The eight EXIF orientation values.
///
/// Compatible with the Exif `Orientation` tag
/// (<https://web.archive.org/web/20200412005226/https://www.impulseadventure.com/photo/exif-orientation.html>).
/// A variant names the transform applied to a stored buffer to reach the
/// orientation a person sees: a rotation, then a flip, in that order,
/// matching the order Exif defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// No rotation or flip. Exif value 1.
    Normal,
    /// Flip horizontally. Exif value 2.
    FlipHorizontal,
    /// Rotate 180 degrees. Exif value 3.
    Rotate180,
    /// Flip vertically. Exif value 4.
    FlipVertical,
    /// Rotate 90 degrees clockwise, then flip horizontally. Exif value 5.
    Rotate90FlipHorizontal,
    /// Rotate 90 degrees clockwise. Exif value 6.
    Rotate90,
    /// Rotate 270 degrees clockwise, then flip horizontally. Exif value 7.
    Rotate270FlipHorizontal,
    /// Rotate 270 degrees clockwise. Exif value 8.
    Rotate270,
}

impl Orientation {
    /// Build an `Orientation` from a raw Exif orientation tag value.
    ///
    /// Exif defines the tag as a 16-bit value, though only 1 to 8 carry a
    /// meaning. Any other value, including 0, returns
    /// `Orientation::Normal`: a source with no orientation metadata
    /// reports no rotation.
    pub fn from_exif_u16(value: u16) -> Self {
        match value {
            1 => Orientation::Normal,
            2 => Orientation::FlipHorizontal,
            3 => Orientation::Rotate180,
            4 => Orientation::FlipVertical,
            5 => Orientation::Rotate90FlipHorizontal,
            6 => Orientation::Rotate90,
            7 => Orientation::Rotate270FlipHorizontal,
            8 => Orientation::Rotate270,
            _ => Orientation::Normal,
        }
    }

    /// Convert to the `image` crate's own orientation type, so this
    /// module's transform reuses that crate's tested, integer-only rotate
    /// and flip operations instead of a hand-rolled pixel copy.
    fn to_image_orientation(self) -> image::metadata::Orientation {
        match self {
            Orientation::Normal => image::metadata::Orientation::NoTransforms,
            Orientation::FlipHorizontal => image::metadata::Orientation::FlipHorizontal,
            Orientation::Rotate180 => image::metadata::Orientation::Rotate180,
            Orientation::FlipVertical => image::metadata::Orientation::FlipVertical,
            Orientation::Rotate90FlipHorizontal => image::metadata::Orientation::Rotate90FlipH,
            Orientation::Rotate90 => image::metadata::Orientation::Rotate90,
            Orientation::Rotate270FlipHorizontal => image::metadata::Orientation::Rotate270FlipH,
            Orientation::Rotate270 => image::metadata::Orientation::Rotate270,
        }
    }
}

/// Apply `orientation`, then convert to 8-bit RGBA with straight alpha.
///
/// The orientation transform runs first, as an integer pixel move: a
/// rotation or a flip, never a resample and never a floating-point
/// computation. The bit-depth conversion that follows takes the same path
/// for every input, so a 16-bit-per-channel source and an 8-bit source
/// that encode the same colour produce the same output bytes.
///
/// Returns the pixel buffer, its width and its height, in that order. The
/// width and height are the final ones, after the orientation transform:
/// a 90-degree rotation swaps them relative to the stored buffer.
pub fn normalize_to_rgba8(image: DynamicImage, orientation: Orientation) -> (Vec<u8>, u32, u32) {
    let mut image = image;
    image.apply_orientation(orientation.to_image_orientation());
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    (rgba.into_raw(), width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn from_exif_u16_maps_every_documented_value() {
        assert_eq!(Orientation::from_exif_u16(1), Orientation::Normal);
        assert_eq!(Orientation::from_exif_u16(2), Orientation::FlipHorizontal);
        assert_eq!(Orientation::from_exif_u16(3), Orientation::Rotate180);
        assert_eq!(Orientation::from_exif_u16(4), Orientation::FlipVertical);
        assert_eq!(
            Orientation::from_exif_u16(5),
            Orientation::Rotate90FlipHorizontal
        );
        assert_eq!(Orientation::from_exif_u16(6), Orientation::Rotate90);
        assert_eq!(
            Orientation::from_exif_u16(7),
            Orientation::Rotate270FlipHorizontal
        );
        assert_eq!(Orientation::from_exif_u16(8), Orientation::Rotate270);
    }

    #[test]
    fn from_exif_u16_defaults_an_undocumented_value_to_normal() {
        assert_eq!(Orientation::from_exif_u16(0), Orientation::Normal);
        assert_eq!(Orientation::from_exif_u16(9), Orientation::Normal);
        assert_eq!(Orientation::from_exif_u16(65535), Orientation::Normal);
    }

    #[test]
    fn normalize_to_rgba8_with_exif_orientation_6_swaps_width_and_height() {
        let stored = RgbaImage::from_pixel(6, 4, Rgba([1, 2, 3, 255]));
        let (_, width, height) = normalize_to_rgba8(
            DynamicImage::ImageRgba8(stored),
            Orientation::from_exif_u16(6),
        );
        assert_eq!((width, height), (4, 6));
    }

    #[test]
    fn normalize_to_rgba8_with_no_transform_keeps_the_stored_shape() {
        let stored = RgbaImage::from_pixel(6, 4, Rgba([1, 2, 3, 255]));
        let (_, width, height) =
            normalize_to_rgba8(DynamicImage::ImageRgba8(stored), Orientation::Normal);
        assert_eq!((width, height), (6, 4));
    }

    #[test]
    fn normalize_to_rgba8_on_a_sixteen_bit_source_returns_eight_bits_per_channel() {
        let stored = image::ImageBuffer::<Rgba<u16>, Vec<u16>>::from_pixel(
            3,
            2,
            Rgba([65535, 0, 32768, 65535]),
        );
        let (pixels, width, height) =
            normalize_to_rgba8(DynamicImage::ImageRgba16(stored), Orientation::Normal);
        assert_eq!(pixels.len(), (width * height * 4) as usize);
    }

    #[test]
    fn normalize_to_rgba8_is_deterministic_across_repeated_calls() {
        let stored = RgbaImage::from_pixel(5, 5, Rgba([9, 8, 7, 6]));
        let first = normalize_to_rgba8(
            DynamicImage::ImageRgba8(stored.clone()),
            Orientation::Rotate90,
        );
        let second = normalize_to_rgba8(DynamicImage::ImageRgba8(stored), Orientation::Rotate90);
        assert_eq!(first, second);
    }
}
