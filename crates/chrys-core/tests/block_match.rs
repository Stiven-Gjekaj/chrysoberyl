//! Integration tests for the register stage's local step: the whole-pixel
//! warp, the summed-area table, and the coarse-to-fine block match.
//!
//! Plan 01-06 Task 1 covers `warp_by_offset`, `difference_image` and
//! `IntegralImage`. Task 2 adds `block_match`, `BlockOffset` and
//! `ResidualField` to this same file.

use chrys_core::register::{
    BLOCK_SIDE, CoarseOffset, IntegralImage, block_match, difference_image, warp_by_offset,
};
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
        assert_eq!(chunk[3], 0);
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

/// A frame with genuine per-pixel texture, built from the fixed sequence
/// generator, so a block's own content is never confusable with a
/// neighbouring block's or with a flat background.
fn textured_frame(width: u32, height: u32, seed: u32) -> Frame {
    let mut sequence = FixedSequence::new(seed);
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        let value = (sequence.next_u32() % 256) as u8;
        pixels.extend_from_slice(&[value, value.wrapping_add(1), value.wrapping_add(2), 255]);
    }
    frame_from(width, height, pixels)
}

/// Deliberately reads the first three bytes only, never the fourth: the two
/// tests that call this helper recolour a region on its colour bytes alone,
/// against frames that are fully opaque everywhere, so an alpha byte can
/// never be the reason a pixel here counts as changed. Widening this to four
/// bytes would test nothing more on this file's own fixtures; the fourth
/// byte's own behaviour is covered directly by
/// `block_match_on_an_identical_pair_reports_zero_offset_and_an_all_zero_residual`
/// above.
fn count_nonzero_colour_pixels(field_samples: &[u8], width: usize) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();
    for (index, chunk) in field_samples.chunks_exact(4).enumerate() {
        if chunk[0] != 0 || chunk[1] != 0 || chunk[2] != 0 {
            positions.push((index % width, index / width));
        }
    }
    positions
}

#[test]
fn block_match_on_an_identical_pair_reports_zero_offset_and_an_all_zero_residual() {
    let base = textured_frame(128, 128, 0xA5A5_0001);
    let candidate = base.clone();
    let field = block_match(&base, &candidate, CoarseOffset { dx: 0, dy: 0 }).unwrap();

    assert!(field.blocks.iter().all(|b| b.dx == 0 && b.dy == 0));
    for chunk in field.samples.chunks_exact(4) {
        assert_eq!(chunk[0], 0);
        assert_eq!(chunk[1], 0);
        assert_eq!(chunk[2], 0);
        assert_eq!(chunk[3], 0);
    }
}

#[test]
fn block_match_reports_a_single_moved_blocks_own_offset() {
    let width = 128u32;
    let height = 128u32;
    let base = textured_frame(width, height, 0xC0DE_0002);
    let mut candidate = base.clone();

    // Move the (block_x=1, block_y=1) block's content right by 4 pixels:
    // copy the 32x32 patch that sits 4 pixels to its left in the base into
    // the candidate at the block's own position, so
    // candidate(x, y) = base(x - 4, y) for x, y inside the block, matching
    // the "positive dx = content sits dx pixels right of the base's"
    // convention CoarseOffset already uses.
    let block_x0 = BLOCK_SIDE;
    let block_y0 = BLOCK_SIDE;
    for y in block_y0..block_y0 + BLOCK_SIDE {
        for x in block_x0..block_x0 + BLOCK_SIDE {
            let src_x = x - 4;
            let dst_idx = (y * width as usize + x) * 4;
            let src_idx = (y * width as usize + src_x) * 4;
            candidate.pixels[dst_idx..dst_idx + 4]
                .copy_from_slice(&base.pixels[src_idx..src_idx + 4]);
        }
    }

    let field = block_match(&base, &candidate, CoarseOffset { dx: 0, dy: 0 }).unwrap();
    for block in &field.blocks {
        if block.block_x == 1 && block.block_y == 1 {
            assert_eq!(block.dx, 4, "the moved block should report dx=4");
            assert_eq!(block.dy, 0, "the moved block should report dy=0");
        } else {
            assert_eq!(
                (block.dx, block.dy),
                (0, 0),
                "block ({}, {}) should report zero offset",
                block.block_x,
                block.block_y
            );
        }
    }
}

#[test]
fn two_runs_of_block_match_on_the_same_pair_produce_a_byte_identical_residual_field() {
    let width = 96u32;
    let height = 96u32;
    let base = textured_frame(width, height, 0xFEED_0003);
    let mut candidate = base.clone();
    for y in 32..64u32 {
        for x in 32..64u32 {
            let idx = ((y * width + x) * 4) as usize;
            candidate.pixels[idx] = candidate.pixels[idx].wrapping_add(37);
        }
    }

    let coarse = CoarseOffset { dx: 0, dy: 0 };
    let field_a = block_match(&base, &candidate, coarse).unwrap();
    let field_b = block_match(&base, &candidate, coarse).unwrap();
    assert_eq!(field_a.samples, field_b.samples);
    assert_eq!(field_a.blocks, field_b.blocks);
}

#[test]
fn block_match_ties_on_equal_scores_in_favour_of_the_smaller_offset_magnitude() {
    // A flat, textureless block scores identically (zero) at every
    // translation within the flat region: the tie-break rule must still
    // pick the smallest-magnitude offset, zero, rather than an
    // arbitrary tied alternative.
    let width = 96u32;
    let height = 96u32;
    let base = frame_from(width, height, vec![120u8; (width * height * 4) as usize]);
    let candidate = base.clone();
    let field = block_match(&base, &candidate, CoarseOffset { dx: 0, dy: 0 }).unwrap();
    assert!(field.blocks.iter().all(|b| b.dx == 0 && b.dy == 0));
}

#[test]
fn residual_field_non_zero_pixels_lie_inside_the_bounding_boxes_of_unmatched_blocks() {
    let width = 96u32;
    let height = 96u32;
    let base = textured_frame(width, height, 0x1357_9024);
    let mut candidate = base.clone();
    // Recolour a region entirely inside the (block_x=2, block_y=0) block,
    // leaving every other block byte-identical to the base.
    for y in 4..20u32 {
        for x in 68..84u32 {
            let idx = ((y * width + x) * 4) as usize;
            candidate.pixels[idx] = candidate.pixels[idx].wrapping_add(80);
        }
    }

    let field = block_match(&base, &candidate, CoarseOffset { dx: 0, dy: 0 }).unwrap();
    let nonzero = count_nonzero_colour_pixels(&field.samples, width as usize);
    assert!(!nonzero.is_empty());

    // Every block that matched perfectly (score 0) must contribute no
    // non-zero pixel to the residual field; a non-zero pixel may only lie
    // inside the bounding box of a block that did not match.
    for block in &field.blocks {
        if block.score == 0 {
            let block_x0 = block.block_x * BLOCK_SIDE;
            let block_y0 = block.block_y * BLOCK_SIDE;
            assert!(
                !nonzero.iter().any(|&(x, y)| x >= block_x0
                    && x < block_x0 + BLOCK_SIDE
                    && y >= block_y0
                    && y < block_y0 + BLOCK_SIDE),
                "matched block ({}, {}) unexpectedly contains a non-zero residual pixel",
                block.block_x,
                block.block_y
            );
        }
    }

    // The recoloured block itself must be the (or one of the) unmatched
    // blocks, and must account for at least one non-zero pixel.
    let recoloured_block = field
        .blocks
        .iter()
        .find(|b| b.block_x == 2 && b.block_y == 0)
        .unwrap();
    assert!(recoloured_block.score > 0);
    let block_x0 = recoloured_block.block_x * BLOCK_SIDE;
    let block_y0 = recoloured_block.block_y * BLOCK_SIDE;
    assert!(nonzero.iter().any(|&(x, y)| x >= block_x0
        && x < block_x0 + BLOCK_SIDE
        && y >= block_y0
        && y < block_y0 + BLOCK_SIDE));
}
