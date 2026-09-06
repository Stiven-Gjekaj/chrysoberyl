//! The residual buffer: the per-pixel difference between two frames.

use crate::CompareError;
use chrys_source::Frame;

/// Return the residual RGBA8 buffer between `base` and `candidate`.
///
/// The buffer has the same length as the two input frames. Each of the
/// red, green and blue bytes is the absolute difference of the
/// corresponding input bytes, and the alpha byte is always 255.
///
/// Plan 01-06 replaces the body of this function with the real residual
/// field this project's classify stage needs, and keeps this signature, so
/// the digest contract built on top of it does not move.
///
/// Returns `CompareError::ShapeMismatch` when the frames differ in size.
pub fn residual_rgba8(base: &Frame, candidate: &Frame) -> Result<Vec<u8>, CompareError> {
    if !base.same_shape_as(candidate) {
        return Err(CompareError::ShapeMismatch {
            base: (base.width, base.height),
            candidate: (candidate.width, candidate.height),
        });
    }

    let base_pixels = base.rgba8();
    let candidate_pixels = candidate.rgba8();
    let mut residual = Vec::with_capacity(base_pixels.len());

    for chunk_index in 0..base.pixel_count() {
        let idx = chunk_index * 4;
        for channel in 0..3 {
            let a = base_pixels[idx + channel];
            let b = candidate_pixels[idx + channel];
            residual.push(a.abs_diff(b));
        }
        residual.push(255);
    }

    Ok(residual)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_frame(width: u32, height: u32, colour: [u8; 4]) -> Frame {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            pixels.extend_from_slice(&colour);
        }
        Frame {
            pixels,
            width,
            height,
            index: 0,
            hints: Vec::new(),
        }
    }

    #[test]
    fn identical_frames_return_a_residual_of_all_zero_colour_channels() {
        let a = solid_frame(2, 2, [10, 20, 30, 255]);
        let b = solid_frame(2, 2, [10, 20, 30, 255]);
        let residual = residual_rgba8(&a, &b).unwrap();
        assert_eq!(residual.len(), a.pixels.len());
        for chunk in residual.chunks_exact(4) {
            assert_eq!(chunk[0], 0);
            assert_eq!(chunk[1], 0);
            assert_eq!(chunk[2], 0);
            assert_eq!(chunk[3], 255);
        }
    }

    #[test]
    fn differing_frames_return_the_absolute_channel_difference() {
        let a = solid_frame(1, 1, [10, 20, 30, 255]);
        let b = solid_frame(1, 1, [30, 10, 30, 0]);
        let residual = residual_rgba8(&a, &b).unwrap();
        assert_eq!(residual, vec![20, 10, 0, 255]);
    }

    #[test]
    fn a_shape_mismatch_returns_shape_mismatch_error() {
        let a = solid_frame(2, 2, [0, 0, 0, 255]);
        let b = solid_frame(3, 2, [0, 0, 0, 255]);
        assert!(matches!(
            residual_rgba8(&a, &b),
            Err(CompareError::ShapeMismatch {
                base: (2, 2),
                candidate: (3, 2),
            })
        ));
    }
}
