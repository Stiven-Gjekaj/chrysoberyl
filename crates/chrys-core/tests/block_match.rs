//! Integration tests for the register stage's local step: the whole-pixel
//! warp, the summed-area table, and the coarse-to-fine block match.
//!
//! Plan 01-06 Task 1 covers `warp_by_offset`, `difference_image` and
//! `IntegralImage`. Task 2 adds `block_match`, `BlockOffset` and
//! `ResidualField` to this same file.

use chrys_core::register::{IntegralImage, difference_image, warp_by_offset};
use chrys_source::Frame;

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
            pixels.extend_from_slice(&[value, value.wrapping_add(1), value.wrapping_add(2), 255]);
        }
    }
    frame_from(width, height, pixels)
}

/// A small, fixed integer sequence generator (xorshift32), seeded with a
/// fixed constant, never with the clock, so a red result here is
/// reproducible. Not a cryptographic generator; used only to pick test
/// rectangles and pixel values.
struct FixedSequence {
    state: u32,
}

impl FixedSequence {
    fn new(seed: u32) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn next_below(&mut self, bound: usize) -> usize {
        (self.next_u32() as usize) % bound.max(1)
    }
}

fn direct_sum(samples: &[u8], width: usize, x: usize, y: usize, w: usize, h: usize) -> u64 {
    let mut total = 0u64;
    for row in y..y + h {
        for col in x..x + w {
            total += u64::from(samples[row * width + col]);
        }
    }
    total
}

#[test]
fn warp_by_offset_with_a_zero_offset_returns_a_buffer_equal_to_its_input() {
    let frame = ramp_frame(9, 7);
    let warped = warp_by_offset(&frame, 0, 0);
    assert_eq!(warped, frame.pixels);
}

#[test]
fn warp_by_offset_with_a_positive_offset_moves_content_by_whole_pixels_and_fills_the_edge() {
    let frame = frame_from(
        3,
        1,
        vec![10, 10, 10, 255, 20, 20, 20, 255, 30, 30, 30, 255],
    );
    let warped = warp_by_offset(&frame, 1, 0);
    // Output x=0 has no source pixel: filled, not wrapped from x=2.
    assert_eq!(&warped[0..4], &[0, 0, 0, 0]);
    // Output x=1 takes input x=0; output x=2 takes input x=1.
    assert_eq!(&warped[4..8], &frame.pixels[0..4]);
    assert_eq!(&warped[8..12], &frame.pixels[4..8]);
}

#[test]
fn warp_by_offset_never_resamples_every_output_byte_is_an_input_byte_or_the_fill_value() {
    let frame = ramp_frame(10, 10);
    let warped = warp_by_offset(&frame, -3, 4);
    for &byte in &warped {
        assert!(byte == 0 || frame.pixels.contains(&byte));
    }
}

#[test]
fn difference_image_on_two_identical_buffers_returns_all_zeros() {
    let frame = ramp_frame(6, 6);
    let diff = difference_image(&frame.pixels, &frame.pixels);
    for chunk in diff.chunks_exact(4) {
        assert_eq!(chunk[0], 0);
        assert_eq!(chunk[1], 0);
        assert_eq!(chunk[2], 0);
        assert_eq!(chunk[3], 255);
    }
}

#[test]
fn difference_image_is_symmetric_in_its_two_arguments() {
    let base = ramp_frame(5, 4);
    let warped = warp_by_offset(&base, 2, -1);
    assert_eq!(
        difference_image(&base.pixels, &warped),
        difference_image(&warped, &base.pixels)
    );
}

#[test]
fn integral_image_window_sum_matches_a_direct_sum_over_twenty_random_rectangles() {
    let width = 41;
    let height = 33;
    let mut sequence = FixedSequence::new(0xB16B_00B5);
    let samples: Vec<u8> = (0..width * height)
        .map(|_| (sequence.next_u32() % 256) as u8)
        .collect();
    let table = IntegralImage::from_luma(&samples, width, height);

    for _ in 0..20 {
        let w = 1 + sequence.next_below(width);
        let h = 1 + sequence.next_below(height);
        let x = sequence.next_below(width - w + 1);
        let y = sequence.next_below(height - h + 1);
        assert_eq!(
            table.window_sum(x, y, w, h),
            direct_sum(&samples, width, x, y, w, h),
            "mismatch at x={x} y={y} w={w} h={h}"
        );
    }
}

#[test]
fn integral_image_window_sum_over_a_one_pixel_rectangle_equals_that_pixel() {
    let width = 5;
    let height = 5;
    let samples: Vec<u8> = (0..width * height).map(|i| (i * 7 % 250) as u8).collect();
    let table = IntegralImage::from_luma(&samples, width, height);
    assert_eq!(
        table.window_sum(3, 2, 1, 1),
        u64::from(samples[2 * width + 3])
    );
}

#[test]
fn integral_image_window_sum_over_the_whole_image_equals_the_total() {
    let width = 8;
    let height = 6;
    let samples: Vec<u8> = (0..width * height).map(|i| (i % 250) as u8).collect();
    let table = IntegralImage::from_luma(&samples, width, height);
    let expected: u64 = samples.iter().map(|&b| u64::from(b)).sum();
    assert_eq!(table.window_sum(0, 0, width, height), expected);
}

#[test]
fn integral_image_at_the_maximum_working_size_with_the_largest_byte_value_does_not_overflow() {
    // 4200 * 4200 * 255 = 4_498_200_000, already past u32::MAX
    // (4_294_967_295), proving the u64 accumulator holds a value a
    // narrower integer could not, without allocating gigabytes of memory
    // to reach the real decode-time ceiling (chrys-source-raster's
    // DecodeLimits::default(), 16384 by 16384, whose own total scales the
    // same way to about 6.87e10, still over eight orders of magnitude
    // below u64::MAX).
    let width = 4200usize;
    let height = 4200usize;
    let samples = vec![u8::MAX; width * height];
    let table = IntegralImage::from_luma(&samples, width, height);
    let expected = width as u64 * height as u64 * u64::from(u8::MAX);
    assert!(expected > u64::from(u32::MAX));
    assert_eq!(table.window_sum(0, 0, width, height), expected);
}
