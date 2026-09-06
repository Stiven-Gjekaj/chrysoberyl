//! The residual buffer: the per-pixel difference between two frames, once
//! both the global shift and each block's own local shift are removed.

use crate::CompareError;
use crate::register;
use chrys_source::Frame;

/// Return the residual RGBA8 buffer between `base` and `candidate`.
///
/// The buffer has the same length as the two input frames. This runs the
/// full registration pipeline (`phase_correlate`, then `block_match`) and
/// renders the `ResidualField` it produces: each of the red, green and
/// blue bytes is the absolute difference of the corresponding bytes once
/// the pair is aligned, both globally and at the block that pixel falls
/// in, and the alpha byte is always 255.
///
/// Returns `CompareError::ShapeMismatch` when the frames differ in size.
pub fn residual_rgba8(base: &Frame, candidate: &Frame) -> Result<Vec<u8>, CompareError> {
    if !base.same_shape_as(candidate) {
        return Err(CompareError::ShapeMismatch {
            base: (base.width, base.height),
            candidate: (candidate.width, candidate.height),
        });
    }

    let (refined, _surface) = register::phase_correlate(base, candidate)?;
    let field = register::block_match(base, candidate, refined.whole)?;
    Ok(field.samples)
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
