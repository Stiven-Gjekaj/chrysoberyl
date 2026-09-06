//! Integration tests for the register stage's global step: the working
//! grid, the window table, and phase correlation's coarse offset.

use chrys_core::hash::rgba8_digest;
use chrys_core::register::{
    WORKING_RESOLUTION, hann_table, phase_correlate, plan_scalar_fft, refine_peak,
};
use chrys_source::Frame;
use rustfft::FftDirection;
use rustfft::num_complex::Complex32;

fn paint_rect(
    pixels: &mut [u8],
    stride: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    colour: [u8; 4],
) {
    for row in y..y + height {
        for col in x..x + width {
            let idx = ((row * stride + col) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&colour);
        }
    }
}

/// A frame at exactly `WORKING_RESOLUTION` on each side, with a few
/// high-contrast rectangles painted onto a flat background, so the content
/// has structure a correlation can lock onto.
fn synthetic_frame() -> Frame {
    let side = WORKING_RESOLUTION as u32;
    let mut pixels = vec![20u8; (side * side * 4) as usize];
    for i in 0..pixels.len() / 4 {
        pixels[i * 4 + 3] = 255;
    }
    paint_rect(&mut pixels, side, 40, 40, 60, 60, [220, 30, 30, 255]);
    paint_rect(&mut pixels, side, 200, 120, 90, 40, [30, 220, 30, 255]);
    paint_rect(&mut pixels, side, 320, 300, 50, 120, [30, 30, 220, 255]);
    Frame {
        pixels,
        width: side,
        height: side,
        index: 0,
        hints: Vec::new(),
    }
}

/// Shift `frame`'s content right by `dx` and down by `dy`, wrapping around
/// at the edges. A negative value shifts left or up.
fn shift_frame(frame: &Frame, dx: i32, dy: i32) -> Frame {
    let width = frame.width;
    let height = frame.height;
    let mut pixels = vec![0u8; frame.pixels.len()];
    for y in 0..height {
        for x in 0..width {
            let src_x = (x as i32 - dx).rem_euclid(width as i32) as u32;
            let src_y = (y as i32 - dy).rem_euclid(height as i32) as u32;
            let src_idx = ((src_y * width + src_x) * 4) as usize;
            let dst_idx = ((y * width + x) * 4) as usize;
            pixels[dst_idx..dst_idx + 4].copy_from_slice(&frame.pixels[src_idx..src_idx + 4]);
        }
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
fn to_luma_downsampled_returns_exactly_the_working_grid_size() {
    let frame = synthetic_frame();
    let luma = chrys_core::register::to_luma_downsampled(&frame);
    assert_eq!(luma.len(), WORKING_RESOLUTION * WORKING_RESOLUTION);
}

#[test]
fn hann_table_is_symmetric_with_zero_ends_and_is_stable_across_calls() {
    let table = hann_table(WORKING_RESOLUTION);
    assert_eq!(table.len(), WORKING_RESOLUTION);
    assert_eq!(table[0], 0.0);
    assert_eq!(table[WORKING_RESOLUTION - 1], 0.0);
    for i in 0..table.len() {
        let mirror = table.len() - 1 - i;
        assert!((table[i] - table[mirror]).abs() < f32::EPSILON * 8.0);
    }
    assert_eq!(table, hann_table(WORKING_RESOLUTION));
}

#[test]
fn a_shift_right_and_down_reports_a_positive_dx_and_dy() {
    let base = synthetic_frame();
    let candidate = shift_frame(&base, 12, 5);
    let (refined, _surface) = phase_correlate(&base, &candidate).unwrap();
    assert_eq!(refined.whole.dx, 12);
    assert_eq!(refined.whole.dy, 5);
}

#[test]
fn the_reversed_shift_reports_the_negative_offset_on_both_axes() {
    let base = synthetic_frame();
    let candidate = shift_frame(&base, -12, -5);
    let (refined, _surface) = phase_correlate(&base, &candidate).unwrap();
    assert_eq!(refined.whole.dx, -12);
    assert_eq!(refined.whole.dy, -5);
}

#[test]
fn two_identical_frames_report_a_zero_offset() {
    let base = synthetic_frame();
    let (refined, _surface) = phase_correlate(&base, &base.clone()).unwrap();
    assert_eq!(refined.whole.dx, 0);
    assert_eq!(refined.whole.dy, 0);
}

#[test]
fn two_runs_of_phase_correlate_return_bit_identical_correlation_surfaces() {
    let base = synthetic_frame();
    let candidate = shift_frame(&base, 12, 5);
    let (_refined_a, surface_a) = phase_correlate(&base, &candidate).unwrap();
    let (_refined_b, surface_b) = phase_correlate(&base, &candidate).unwrap();
    assert_eq!(surface_a.magnitudes, surface_b.magnitudes);
}

#[test]
fn a_peak_exactly_on_a_sample_refines_to_a_zero_fractional_part_on_both_axes() {
    let base = synthetic_frame();
    let (refined, surface) = phase_correlate(&base, &base.clone()).unwrap();
    let coarse = refined.whole;
    let refined_again = refine_peak(&surface, coarse, 50);
    assert_eq!(refined_again.whole, coarse);
    assert!(refined_again.fractional_x.abs() < 0.01);
    assert!(refined_again.fractional_y.abs() < 0.01);
}

#[test]
fn a_whole_pixel_shift_refines_to_within_a_hundredth_of_a_pixel_of_zero() {
    let base = synthetic_frame();
    let candidate = shift_frame(&base, 12, 5);
    let (refined, surface) = phase_correlate(&base, &candidate).unwrap();
    let refined_again = refine_peak(&surface, refined.whole, 50);
    assert!(refined_again.fractional_x.abs() < 0.01);
    assert!(refined_again.fractional_y.abs() < 0.01);
}

#[test]
fn two_runs_of_refine_peak_on_the_same_input_agree_exactly() {
    let base = synthetic_frame();
    let candidate = shift_frame(&base, 12, 5);
    let (refined, surface) = phase_correlate(&base, &candidate).unwrap();
    let refined_a = refine_peak(&surface, refined.whole, 50);
    let refined_b = refine_peak(&surface, refined.whole, 50);
    assert_eq!(refined_a, refined_b);
}

#[test]
fn coarse_offset_still_reports_whole_pixels_separately_from_the_fraction() {
    let base = synthetic_frame();
    let candidate = shift_frame(&base, 12, 5);
    let (refined, surface) = phase_correlate(&base, &candidate).unwrap();
    let refined_again = refine_peak(&surface, refined.whole, 50);
    assert_eq!(refined_again.whole.dx, 12);
    assert_eq!(refined_again.whole.dy, 5);
}

/// A fixed 512-sample input, from a deterministic integer formula, holds
/// the arithmetic path `plan_scalar_fft` takes in place. This constant
/// guards that path: a change to it must be justified in a SUMMARY, never
/// regenerated to make a red test go green.
const TRANSFORM_DIGEST: &str = "69c8c85bc4fb222c8e1dd08f08e245e4249bd2263e7def3ea5e58aa0c7c5115d";

#[test]
fn a_fixed_synthetic_input_digests_to_the_pinned_transform_constant() {
    let input: Vec<Complex32> = (0..512)
        .map(|i| Complex32::new((i % 17) as f32, (i % 5) as f32))
        .collect();
    let mut buffer = input;
    let fft = plan_scalar_fft(512, FftDirection::Forward);
    fft.process(&mut buffer);

    let mut bytes = Vec::with_capacity(buffer.len() * 8);
    for c in &buffer {
        bytes.extend_from_slice(&c.re.to_le_bytes());
        bytes.extend_from_slice(&c.im.to_le_bytes());
    }
    let digest = rgba8_digest(&bytes);
    assert_eq!(digest, TRANSFORM_DIGEST);
}
