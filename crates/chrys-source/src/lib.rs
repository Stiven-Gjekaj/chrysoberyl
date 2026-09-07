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

    /// Crop this frame to the rectangle `hint` names, and return a new
    /// frame holding exactly those pixels.
    ///
    /// The bounds test uses checked arithmetic, so a hint whose `x + width`
    /// or `y + height` would overflow a `u32` is refused rather than
    /// wrapping into a rectangle that appears to fit. A hint of zero width
    /// or zero height is refused too. The returned frame keeps this
    /// frame's own `index`, and carries an empty `hints` list, because a
    /// cropped frame's coordinate space is not the one the source frame's
    /// hints were written in.
    pub fn crop_to_region(&self, hint: &RegionHint) -> Result<Frame, RegionOutOfBounds> {
        let fits = hint.width > 0
            && hint.height > 0
            && hint
                .x
                .checked_add(hint.width)
                .is_some_and(|right| right <= self.width)
            && hint
                .y
                .checked_add(hint.height)
                .is_some_and(|bottom| bottom <= self.height);

        if !fits {
            return Err(RegionOutOfBounds {
                name: hint.name.clone(),
                rectangle: (hint.x, hint.y, hint.width, hint.height),
                frame_size: (self.width, self.height),
            });
        }

        let mut pixels = Vec::with_capacity((hint.width as usize) * (hint.height as usize) * 4);
        for row in hint.y..(hint.y + hint.height) {
            let row_start = ((row * self.width + hint.x) as usize) * 4;
            let row_end = row_start + (hint.width as usize) * 4;
            pixels.extend_from_slice(&self.pixels[row_start..row_end]);
        }

        Ok(Frame {
            pixels,
            width: hint.width,
            height: hint.height,
            index: self.index,
            hints: Vec::new(),
        })
    }
}

/// The rectangle a hint names does not fit inside the frame it was read
/// against.
///
/// This carries the hint's own name, its rectangle, and the frame's size,
/// so a person holding a refusal can tell which of the two is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionOutOfBounds {
    /// The hint's own name.
    pub name: String,
    /// The hint's rectangle: `(x, y, width, height)`, in pixels.
    pub rectangle: (u32, u32, u32, u32),
    /// The frame's own size: `(width, height)`, in pixels.
    pub frame_size: (u32, u32),
}

impl std::fmt::Display for RegionOutOfBounds {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (x, y, width, height) = self.rectangle;
        let (frame_width, frame_height) = self.frame_size;
        write!(
            formatter,
            "region \"{}\" is {width}x{height} at ({x}, {y}), which does not fit a {frame_width}x{frame_height} frame",
            self.name
        )
    }
}

impl std::error::Error for RegionOutOfBounds {}

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

    /// Load the file at `path` and return its frames, each paired with the
    /// name of the file it came from.
    ///
    /// The default calls `load`, then pairs every returned frame with
    /// `path`'s own file name, cloned once for every frame. That default
    /// is correct for a source that reads one file into one frame, and it
    /// is correct and honest for a source whose frames are composited out
    /// of one container file and have no file of their own: every frame
    /// of such a source shares that one container's own name, because
    /// none of them has a name any more specific than that.
    ///
    /// This method exists instead of a `source_name` field on `Frame`
    /// itself. A field on `Frame` would need a value in every one of the
    /// struct literals that build one, and `chrys-core` alone builds
    /// dozens; `Frame` stays exactly the shape it already is, and only the
    /// one caller that needs a name (`chrys-cli`, writing its report)
    /// asks for one, through this method, rather than every caller
    /// carrying a name it does not use.
    ///
    /// Falls back to `path`'s own display form when `path` has no file
    /// name component (for instance, when `path` is `.` or `/`), so this
    /// method never returns an empty label.
    fn load_named(&self, path: &Path) -> Result<Vec<(String, Frame)>, Self::Error> {
        let name = file_name_or_display(path);
        let frames = self.load(path)?;
        Ok(frames
            .into_iter()
            .map(|frame| (name.clone(), frame))
            .collect())
    }
}

/// `path`'s own file name, or its whole display form when it has no file
/// name component. Shared by every default and override of
/// `Source::load_named`, so a path with no file name is handled the same
/// way everywhere in this crate's own default body.
fn file_name_or_display(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
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

    fn hint(name: &str, x: u32, y: u32, width: u32, height: u32) -> RegionHint {
        RegionHint {
            name: name.to_string(),
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn a_hint_wider_than_the_frame_is_refused() {
        let f = frame(100, 100);
        let h = hint("wide", 0, 0, 200, 50);
        let error = f.crop_to_region(&h).expect_err("does not fit");
        assert_eq!(error.name, "wide");
        assert_eq!(error.rectangle, (0, 0, 200, 50));
        assert_eq!(error.frame_size, (100, 100));
        let message = error.to_string();
        assert!(message.contains("wide"));
        assert!(message.contains("200x50"));
        assert!(message.contains("100x100"));
    }

    #[test]
    fn a_hint_taller_than_the_frame_is_refused() {
        let f = frame(100, 100);
        let h = hint("tall", 0, 0, 50, 200);
        let error = f.crop_to_region(&h).expect_err("does not fit");
        let message = error.to_string();
        assert!(message.contains("tall"));
        assert!(message.contains("50x200"));
        assert!(message.contains("100x100"));
    }

    #[test]
    fn a_hint_whose_origin_sits_outside_the_frame_is_refused() {
        let f = frame(100, 100);
        let h = hint("outside", 90, 90, 20, 20);
        let error = f.crop_to_region(&h).expect_err("does not fit");
        assert_eq!(error.name, "outside");
    }

    #[test]
    fn a_hint_whose_x_plus_width_overflows_a_u32_is_refused_rather_than_wrapping() {
        let f = frame(100, 100);
        let h = hint("overflow", u32::MAX - 5, 0, 10, 10);
        let error = f
            .crop_to_region(&h)
            .expect_err("checked arithmetic refuses this");
        assert_eq!(error.name, "overflow");
    }

    #[test]
    fn a_hint_of_zero_width_is_refused() {
        let f = frame(100, 100);
        let h = hint("empty", 10, 10, 0, 20);
        let error = f.crop_to_region(&h).expect_err("zero width is refused");
        assert_eq!(error.name, "empty");
    }
}

/// A frame whose pixel `(x, y)` holds a byte value derived from its own
/// position, so a crop can be checked against the exact bytes it should
/// have copied, not only against its width and height.
///
/// This lives at the crate root, not inside `mod tests`, so the test below
/// carries the bare name `crop_to_region` rather than `tests::crop_to_region`.
#[cfg(test)]
fn patterned_frame(width: u32, height: u32) -> Frame {
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx] = (x % 256) as u8;
            pixels[idx + 1] = (y % 256) as u8;
            pixels[idx + 2] = 7;
            pixels[idx + 3] = 255;
        }
    }
    Frame {
        pixels,
        width,
        height,
        index: 3,
        hints: Vec::new(),
    }
}

#[cfg(test)]
#[test]
fn crop_to_region() {
    let f = patterned_frame(256, 256);
    let hint = RegionHint {
        name: "logo".to_string(),
        x: 64,
        y: 64,
        width: 128,
        height: 128,
    };

    let cropped = f.crop_to_region(&hint).expect("hint fits the frame");

    assert_eq!(cropped.width, 128);
    assert_eq!(cropped.height, 128);
    assert_eq!(cropped.pixels.len(), 128 * 128 * 4);
    assert_eq!(cropped.index, f.index);
    assert!(cropped.hints.is_empty());

    // Every pixel the crop copied matches the source frame's own pixel at
    // the corresponding absolute position.
    for y in 0..128u32 {
        for x in 0..128u32 {
            let cropped_idx = ((y * 128 + x) * 4) as usize;
            let source_idx = (((y + 64) * 256 + (x + 64)) * 4) as usize;
            assert_eq!(
                cropped.pixels[cropped_idx..cropped_idx + 4],
                f.pixels[source_idx..source_idx + 4]
            );
        }
    }
}
