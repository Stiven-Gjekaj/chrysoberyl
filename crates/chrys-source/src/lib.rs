//! The format-blind boundary between a source and the engine.
//!
//! This crate defines the one shape every adapter hands to the engine. It
//! depends on nothing outside the standard library, and it names no file
//! format.

#![forbid(unsafe_code)]

use std::path::Path;

/// One decoded frame, in the canonical pixel format.
///
/// The buffer is RGBA8, straight alpha, row major, with no row padding.
/// No `image` crate filter may run on this buffer without a conversion
/// first. At least one filter in that crate assumes premultiplied alpha
/// instead of straight alpha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// The pixel buffer. RGBA8, straight alpha, row major, no row padding.
    pub pixels: Vec<u8>,
    /// The frame width, in pixels.
    pub width: u32,
    /// The frame height, in pixels.
    pub height: u32,
    /// The frame index inside a sequence. A single still image reports 0.
    pub index: usize,
    /// Named regions a producer supplies as a hint. Empty when the source
    /// gives no hint.
    pub hints: Vec<RegionHint>,
}

impl Frame {
    /// Return the number of pixels in this frame.
    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Return the pixel buffer as a slice of RGBA8 bytes.
    pub fn rgba8(&self) -> &[u8] {
        &self.pixels
    }

    /// Return true when this frame has the same width and height as
    /// `other`.
    pub fn same_shape_as(&self, other: &Frame) -> bool {
        self.width == other.width && self.height == other.height
    }
}

/// A named region a producer supplies as a hint to the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionHint {
    /// The name the producer gave this region.
    pub name: String,
    /// The left edge of the region, in pixels.
    pub x: u32,
    /// The top edge of the region, in pixels.
    pub y: u32,
    /// The width of the region, in pixels.
    pub width: u32,
    /// The height of the region, in pixels.
    pub height: u32,
}

/// A source that decodes a path on disk into one or more frames.
///
/// A source adapter is the only place that knows a file format. The engine
/// never sees a format name, only `Frame` values.
pub trait Source {
    /// The error type this source returns on a failed load.
    type Error;

    /// Load the file at `path` and return its frames.
    fn load(&self, path: &Path) -> Result<Vec<Frame>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(width: u32, height: u32) -> Frame {
        Frame {
            pixels: vec![0; (width * height * 4) as usize],
            width,
            height,
            index: 0,
            hints: Vec::new(),
        }
    }

    #[test]
    fn pixel_count_multiplies_width_by_height() {
        let f = frame(4, 3);
        assert_eq!(f.pixel_count(), 12);
    }

    #[test]
    fn rgba8_returns_the_pixel_buffer() {
        let f = frame(2, 2);
        assert_eq!(f.rgba8().len(), 16);
    }

    #[test]
    fn same_shape_as_compares_width_and_height_only() {
        let a = frame(4, 3);
        let b = frame(4, 3);
        let c = frame(4, 4);
        assert!(a.same_shape_as(&b));
        assert!(!a.same_shape_as(&c));
    }
}
