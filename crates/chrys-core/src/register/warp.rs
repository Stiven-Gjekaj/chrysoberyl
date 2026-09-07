//! Whole-pixel warp of a frame, and the absolute-difference image between
//! two aligned buffers.
//!
//! Neither function resamples. `warp_by_offset` copies bytes; it never
//! interpolates between them, so every output byte equals some input byte
//! or the fill constant below. This keeps the warp step on the same exact
//! arithmetic every other step in this stage already commits to.

use chrys_source::Frame;

/// The byte value a vacated edge is filled with after `warp_by_offset`.
///
/// A vacated edge is filled, never wrapped. Wrapping would copy content
/// from the far edge of the frame into the near edge, inventing a change
/// that is not there; filling with a fixed, documented constant instead
/// reports "no data here" honestly.
pub const FILL_VALUE: u8 = 0;

/// Move `frame`'s content by exactly `dx` pixels horizontally and `dy`
/// pixels vertically, and return the resulting RGBA8 buffer.
///
/// A positive `dx` moves content to the right; a positive `dy` moves
/// content down, matching the sign convention `CoarseOffset` already
/// reports elsewhere in this stage: output pixel `(x, y)` takes the value
/// of input pixel `(x - dx, y - dy)`. A pixel whose source position falls
/// outside the frame is filled with `FILL_VALUE`, on all four channels,
/// rather than wrapped from the opposite edge.
///
/// A zero offset returns a buffer equal to the input, because every output
/// pixel's source position is itself.
pub fn warp_by_offset(frame: &Frame, dx: i32, dy: i32) -> Vec<u8> {
    let width = frame.width as usize;
    let height = frame.height as usize;
    let pixels = frame.rgba8();

    let mut out = vec![FILL_VALUE; pixels.len()];
    for y in 0..height {
        let src_y = y as i32 - dy;
        if src_y < 0 || src_y as usize >= height {
            continue;
        }
        let src_y = src_y as usize;
        for x in 0..width {
            let src_x = x as i32 - dx;
            if src_x < 0 || src_x as usize >= width {
                continue;
            }
            let src_x = src_x as usize;
            let src_idx = (src_y * width + src_x) * 4;
            let dst_idx = (y * width + x) * 4;
            out[dst_idx..dst_idx + 4].copy_from_slice(&pixels[src_idx..src_idx + 4]);
        }
    }
    out
}

/// Return the per-byte absolute difference of two RGBA8 buffers of equal
/// length, over all four channels the `Frame` contract carries.
///
/// `u8::abs_diff` is exact integer arithmetic on every platform, so this
/// buffer is a measurement, not a picture: unlike an image meant for
/// viewing, its fourth byte is not a usable alpha channel, it is the
/// alpha difference.
///
/// `base` and `warped` must have the same length; a debug build asserts
/// this, because the only two call sites in this crate always hand it a
/// pair of buffers already known to describe the same frame shape, so no
/// error type is needed here for a mismatch a caller cannot otherwise
/// produce.
///
/// This is symmetric: swapping the two arguments gives the same result,
/// because `u8::abs_diff` is symmetric.
pub fn difference_image(base: &[u8], warped: &[u8]) -> Vec<u8> {
    debug_assert_eq!(
        base.len(),
        warped.len(),
        "difference_image requires two buffers of equal length"
    );
    let mut out = Vec::with_capacity(base.len());
    for chunk_start in (0..base.len()).step_by(4) {
        for channel in 0..4 {
            out.push(base[chunk_start + channel].abs_diff(warped[chunk_start + channel]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_from(width: u32, height: u32, pixels: Vec<u8>) -> Frame {
        Frame {
            pixels,
            width,
            height,
            index: 0,
            hints: Vec::new(),
        }
    }

    fn ramp_frame(width: u32, height: u32) -> Frame {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let value = ((y * width + x) % 256) as u8;
                pixels.extend_from_slice(&[
                    value,
                    value.wrapping_add(1),
                    value.wrapping_add(2),
                    255,
                ]);
            }
        }
        frame_from(width, height, pixels)
    }

    #[test]
    fn a_zero_offset_returns_the_input_unchanged() {
        let frame = ramp_frame(6, 5);
        let warped = warp_by_offset(&frame, 0, 0);
        assert_eq!(warped, frame.pixels);
    }

    #[test]
    fn a_positive_horizontal_offset_moves_content_right_and_fills_the_left_edge() {
        let frame = ramp_frame(4, 1);
        let warped = warp_by_offset(&frame, 1, 0);
        // Output x=0 has no source (src_x = -1): filled.
        assert_eq!(&warped[0..4], &[FILL_VALUE; 4]);
        // Output x=1 takes input x=0.
        assert_eq!(&warped[4..8], &frame.pixels[0..4]);
        // Output x=3 takes input x=2.
        assert_eq!(&warped[12..16], &frame.pixels[8..12]);
    }

    #[test]
    fn a_positive_vertical_offset_moves_content_down_and_fills_the_top_edge() {
        let frame = ramp_frame(1, 4);
        let warped = warp_by_offset(&frame, 0, 1);
        assert_eq!(&warped[0..4], &[FILL_VALUE; 4]);
        assert_eq!(&warped[4..8], &frame.pixels[0..4]);
    }

    #[test]
    fn a_vacated_edge_is_filled_not_wrapped() {
        let frame = frame_from(
            3,
            1,
            vec![10, 10, 10, 255, 20, 20, 20, 255, 30, 30, 30, 255],
        );
        let warped = warp_by_offset(&frame, -1, 0);
        // Output x=2 has no source (src_x = 3, out of range): filled, never
        // wrapped to the far-left pixel (10, 10, 10, 255).
        assert_eq!(&warped[8..12], &[FILL_VALUE; 4]);
    }

    #[test]
    fn every_output_byte_is_an_input_byte_or_the_fill_constant() {
        let frame = ramp_frame(8, 8);
        let warped = warp_by_offset(&frame, 3, -2);
        for &byte in &warped {
            assert!(frame.pixels.contains(&byte) || byte == FILL_VALUE);
        }
    }

    #[test]
    fn difference_image_of_a_pair_with_itself_is_all_zero_on_every_channel() {
        let frame = ramp_frame(5, 5);
        let diff = difference_image(&frame.pixels, &frame.pixels);
        for chunk in diff.chunks_exact(4) {
            assert_eq!(chunk[0], 0);
            assert_eq!(chunk[1], 0);
            assert_eq!(chunk[2], 0);
            assert_eq!(chunk[3], 0);
        }
    }

    #[test]
    fn difference_image_is_symmetric_in_its_two_arguments() {
        let a = ramp_frame(4, 4);
        let b = warp_by_offset(&a, 1, 1);
        assert_eq!(
            difference_image(&a.pixels, &b),
            difference_image(&b, &a.pixels)
        );
    }
}
