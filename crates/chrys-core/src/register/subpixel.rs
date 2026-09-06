//! Subpixel refinement of the coarse phase-correlation peak.
//!
//! This module implements the upsampled discrete Fourier transform of
//! Guizar-Sicairos, Thurman and Fienup, "Efficient subpixel image
//! registration algorithms", Optics Letters 2008, the algorithm behind
//! scikit-image's `phase_cross_correlation`. Rather than computing a full,
//! high-resolution inverse transform, it evaluates the inverse transform by
//! direct matrix multiplication, at only the small neighbourhood of
//! fractional-pixel positions around the coarse peak this refinement needs.
//! The search runs in two passes, a coarse pass across the whole
//! plus-or-minus-half-pixel range the integer coarse peak already bounds
//! the true offset to, then a fine pass at the caller's requested
//! resolution around the coarse pass's own best point, so the direct
//! matrix multiply never has to evaluate more than a few dozen candidate
//! positions per pass, whatever `upsample_factor` is.
//!
//! Every complex exponential this step needs comes from the pure-Rust
//! `libm` crate's sine and cosine functions, for the same reason the window
//! table in `window.rs` does.

use rustfft::num_complex::Complex32;

use crate::register::phase_correlation::{CoarseOffset, CorrelationSurface, unwrap_bin_index};

/// The resolution the register stage's own refinement fixes the search to:
/// one hundredth of a pixel. This is a tuning constant, not a correctness
/// input; changing it changes only how finely a fractional offset is
/// reported, never which whole pixel `phase_correlate` reports.
pub const UPSAMPLE_FACTOR: u32 = 100;

/// The number of candidate positions per axis each search pass evaluates.
/// A larger table finds a sharper peak within the pass's own window, at the
/// cost of one more row or column of the direct matrix multiply below; 21
/// is enough to keep the coarse pass's own quantisation well inside the
/// fine pass's window in every case this module is built for.
const SEARCH_SAMPLES: usize = 21;

/// The whole-pixel offset plus a fractional part per axis, refined below
/// one pixel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefinedOffset {
    /// The whole-pixel offset. Identical in value and meaning to the
    /// `CoarseOffset` this refinement started from. The fractional part
    /// below exists for the block-match stage in plan 01-06 and for a
    /// future warp, not for the printed verdict; a caller that wants
    /// pixels is never handed a fraction through this field.
    pub whole: CoarseOffset,
    /// The fractional horizontal offset, in pixels, in the range
    /// `-0.5..=0.5`.
    pub fractional_x: f32,
    /// The fractional vertical offset, in pixels, in the range
    /// `-0.5..=0.5`.
    pub fractional_y: f32,
}

/// Refine `coarse` to subpixel accuracy against `surface`, at
/// `1 / upsample_factor` of a pixel.
///
/// This builds the small upsampled neighbourhood around the coarse peak by
/// a direct matrix-multiply transform over that neighbourhood only, never
/// by upsampling the whole surface, and takes the maximum of the upsampled
/// neighbourhood as the refined peak, breaking a tie by the lowest linear
/// index, matching the coarse stage's own convention.
pub fn refine_peak(
    surface: &CorrelationSurface,
    coarse: CoarseOffset,
    upsample_factor: u32,
) -> RefinedOffset {
    let n = surface.resolution;
    let px = unwrap_bin_index(coarse.dx, n);
    let py = unwrap_bin_index(coarse.dy, n);

    let (coarse_x, coarse_y) = search_window(
        &surface.spectrum,
        n,
        px as f64,
        py as f64,
        0.5,
        SEARCH_SAMPLES,
    );

    let fine_step = 0.5 / f64::from(upsample_factor.max(1));
    let fine_span = (fine_step * ((SEARCH_SAMPLES - 1) / 2) as f64).min(0.5);
    let (refined_x, refined_y) = search_window(
        &surface.spectrum,
        n,
        coarse_x,
        coarse_y,
        fine_span,
        SEARCH_SAMPLES,
    );

    RefinedOffset {
        whole: coarse,
        fractional_x: (px as f64 - refined_x) as f32,
        fractional_y: (py as f64 - refined_y) as f32,
    }
}

/// Evaluate the inverse transform of `spectrum` by direct matrix
/// multiplication, at `sample_count` positions per axis spanning
/// `center - span ..= center + span`, and return the position of the
/// largest magnitude found, breaking a tie by the lowest linear index.
fn search_window(
    spectrum: &[Complex32],
    n: usize,
    center_x: f64,
    center_y: f64,
    span: f64,
    sample_count: usize,
) -> (f64, f64) {
    let step = if sample_count > 1 {
        2.0 * span / (sample_count - 1) as f64
    } else {
        0.0
    };
    let x_positions: Vec<f64> = (0..sample_count)
        .map(|i| center_x - span + step * i as f64)
        .collect();
    let y_positions: Vec<f64> = (0..sample_count)
        .map(|i| center_y - span + step * i as f64)
        .collect();

    // Precompute the twiddle values for the neighbourhood once per call:
    // each position needs exactly one sine-and-cosine pair to build its own
    // base value, and every other value in its row of the table comes from
    // one complex multiplication, so the transcendental count this
    // function pays is `sample_count`, never `n`.
    let x_twiddles = twiddle_table(&x_positions, n);
    let y_twiddles = twiddle_table(&y_positions, n);

    // Step 1: for every frequency row `v`, sum across the `u` axis against
    // every x candidate, so `row_sums[v][m]` holds the x-axis partial
    // transform at candidate `m`.
    let mut row_sums = vec![Complex32::new(0.0, 0.0); n * sample_count];
    for v in 0..n {
        let row = &spectrum[v * n..v * n + n];
        for (m, twiddle_row) in x_twiddles.iter().enumerate() {
            let mut acc = Complex32::new(0.0, 0.0);
            for u in 0..n {
                acc += row[u] * twiddle_row[u];
            }
            row_sums[v * sample_count + m] = acc;
        }
    }

    // Step 2: finish the transform along the y axis, for every (y, x)
    // candidate pair, and track the largest magnitude.
    let mut best_magnitude = -1.0f32;
    let mut best_x = x_positions[0];
    let mut best_y = y_positions[0];
    for (row_index, y_twiddle_row) in y_twiddles.iter().enumerate() {
        for (m, &x_position) in x_positions.iter().enumerate() {
            let mut acc = Complex32::new(0.0, 0.0);
            for v in 0..n {
                acc += row_sums[v * sample_count + m] * y_twiddle_row[v];
            }
            let magnitude = (acc.re * acc.re + acc.im * acc.im).sqrt();
            if magnitude > best_magnitude {
                best_magnitude = magnitude;
                best_x = x_position;
                best_y = y_positions[row_index];
            }
        }
    }

    (best_x, best_y)
}

/// Build, for each position in `positions`, the `n` complex exponential
/// values `exp(i * 2 * pi * position * signed_k(k) / n)` for `k` in `0..n`,
/// where `signed_k(k)` is `k` for `k <= n / 2` and `k - n` above that.
///
/// A spectrum's bins above the Nyquist bin represent negative frequencies.
/// The raw index `k` and the signed frequency `k - n` agree at every
/// integer position, because `exp(i * 2 * pi * (k - n) * x / n)` and
/// `exp(i * 2 * pi * k * x / n)` differ by `exp(-i * 2 * pi * x)`, which is
/// exactly `1` for integer `x`. This refinement step exists precisely to
/// evaluate positions that are not integers, so the two indices no longer
/// agree there, and using the raw index would fold each high-frequency bin
/// in at the wrong phase, corrupting the very peak this step is built to
/// sharpen.
///
/// Each position needs exactly one sine-and-cosine pair to build its own
/// base value, and a second pair to build the correction the upper half of
/// the table needs; every other value in the table comes from one complex
/// multiplication. The transcendental count this function pays is
/// therefore the neighbourhood size (`positions.len()`), never `n`.
fn twiddle_table(positions: &[f64], n: usize) -> Vec<Vec<Complex32>> {
    let half = n / 2;
    positions
        .iter()
        .map(|&position| {
            let angle = 2.0 * std::f64::consts::PI * position / n as f64;
            let base = Complex32::new(libm::cos(angle) as f32, libm::sin(angle) as f32);
            let correction_angle = -2.0 * std::f64::consts::PI * position;
            let correction = Complex32::new(
                libm::cos(correction_angle) as f32,
                libm::sin(correction_angle) as f32,
            );
            let mut table = Vec::with_capacity(n);
            let mut current = Complex32::new(1.0, 0.0);
            for k in 0..n {
                let value = if k <= half {
                    current
                } else {
                    current * correction
                };
                table.push(value);
                current *= base;
            }
            table
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::phase_correlation::phase_correlate;
    use crate::register::window::WORKING_RESOLUTION;
    use chrys_source::Frame;

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

    /// A frame whose content is a handful of low-frequency sinusoids, not
    /// one, so the correlation the fractional-shift tests below drive has
    /// a single, unambiguous peak: a single pure sinusoid is itself
    /// periodic, which gives phase correlation several equally tall,
    /// equally spaced peaks to choose between.
    fn low_frequency_frame() -> Frame {
        let side = WORKING_RESOLUTION as u32;
        let mut pixels = vec![0u8; (side * side * 4) as usize];
        // A handful of low-frequency components, not one, so the
        // correlation this test drives has a single, unambiguous peak. One
        // pure sinusoid is itself periodic, which gives phase correlation
        // several equally tall, equally spaced peaks to choose between.
        let components: [(f64, f64, f64); 6] = [
            (3.0, 40.0, 0.3),
            (7.0, 30.0, 1.1),
            (11.0, 25.0, 2.0),
            (17.0, 20.0, 0.7),
            (23.0, 15.0, 2.6),
            (5.0, 22.0, 1.9),
        ];
        for y in 0..side {
            for x in 0..side {
                let mut value = 128.0;
                for &(cycles, amplitude, phase_offset) in &components {
                    let phase = 2.0 * std::f64::consts::PI * cycles * f64::from(x)
                        / f64::from(side)
                        + phase_offset;
                    value += amplitude * libm::sin(phase);
                }
                let value = value.clamp(0.0, 255.0) as u8;
                let idx = ((y * side + x) * 4) as usize;
                pixels[idx] = value;
                pixels[idx + 1] = value;
                pixels[idx + 2] = value;
                pixels[idx + 3] = 255;
            }
        }
        Frame {
            pixels,
            width: side,
            height: side,
            index: 0,
            hints: Vec::new(),
        }
    }

    #[test]
    fn a_peak_exactly_on_a_sample_refines_to_a_zero_fractional_part_on_both_axes() {
        let base = synthetic_frame();
        let (base_refined, surface) = phase_correlate(&base, &base.clone()).unwrap();
        let coarse = base_refined.whole;
        let refined = refine_peak(&surface, coarse, 50);
        assert_eq!(refined.whole, coarse);
        assert!(refined.fractional_x.abs() < 0.01);
        assert!(refined.fractional_y.abs() < 0.01);
    }

    #[test]
    fn a_whole_pixel_shift_refines_to_within_a_hundredth_of_a_pixel_of_zero() {
        let base = synthetic_frame();
        let candidate = shift_frame(&base, 12, 5);
        let (base_refined, surface) = phase_correlate(&base, &candidate).unwrap();
        let coarse = base_refined.whole;
        let refined = refine_peak(&surface, coarse, 50);
        assert!(refined.fractional_x.abs() < 0.01);
        assert!(refined.fractional_y.abs() < 0.01);
    }

    /// Shift every row of `frame` right by exactly `shift` pixels (may be
    /// fractional), using an ideal frequency-domain fractional delay: FFT
    /// each row, multiply by a linear phase ramp, inverse FFT, and take the
    /// real part. This needs no resampling library, only the scalar FFT
    /// plan this module's own tests already depend on, and it is exact
    /// (up to floating rounding and the final cast to `u8`), unlike
    /// averaging two adjacent columns, which is only a clean half-sample
    /// delay for a narrow band of frequencies.
    fn ideal_fractional_shift_right(frame: &Frame, shift: f64) -> Frame {
        use rustfft::FftPlannerScalar;
        use rustfft::num_complex::Complex64;
        let width = frame.width as usize;
        let height = frame.height as usize;
        let mut planner = FftPlannerScalar::<f64>::new();
        let forward = planner.plan_fft_forward(width);
        let inverse = planner.plan_fft_inverse(width);
        let mut pixels = vec![0u8; frame.pixels.len()];
        for y in 0..height {
            for channel in 0..3 {
                let mut row: Vec<Complex64> = (0..width)
                    .map(|x| {
                        let idx = (y * width + x) * 4 + channel;
                        Complex64::new(f64::from(frame.pixels[idx]), 0.0)
                    })
                    .collect();
                forward.process(&mut row);
                for (k, value) in row.iter_mut().enumerate() {
                    // Bins above the Nyquist bin represent negative
                    // frequencies; a real fractional delay must use the
                    // signed frequency here, or Hermitian symmetry breaks
                    // for a non-integer shift and the inverse transform's
                    // real part is no longer the shifted signal.
                    let signed_k = if k <= width / 2 {
                        k as f64
                    } else {
                        k as f64 - width as f64
                    };
                    let angle = -2.0 * std::f64::consts::PI * signed_k * shift / width as f64;
                    let ramp = Complex64::new(libm::cos(angle), libm::sin(angle));
                    *value *= ramp;
                }
                inverse.process(&mut row);
                for (x, value) in row.iter().enumerate() {
                    let idx = (y * width + x) * 4 + channel;
                    let scaled = value.re / width as f64;
                    pixels[idx] = scaled.round().clamp(0.0, 255.0) as u8;
                }
            }
            for x in 0..width {
                pixels[(y * width + x) * 4 + 3] = 255;
            }
        }
        Frame {
            pixels,
            width: frame.width,
            height: frame.height,
            index: 0,
            hints: Vec::new(),
        }
    }

    #[test]
    fn several_fractional_shifts_all_refine_close_to_their_true_value() {
        let base = low_frequency_frame();
        for true_shift in [1.5, 12.5, -3.5] {
            let candidate = ideal_fractional_shift_right(&base, true_shift);
            let (base_refined, surface) = phase_correlate(&base, &candidate).unwrap();
            let coarse = base_refined.whole;
            let refined = refine_peak(&surface, coarse, 50);
            let total = f64::from(coarse.dx) + f64::from(refined.fractional_x);
            assert!(
                (total - true_shift).abs() < 0.1,
                "true_shift={true_shift} refined_total={total}"
            );
        }
    }

    #[test]
    fn a_synthetic_half_pixel_shift_refines_to_within_a_tenth_of_a_pixel_of_one_half() {
        let base = low_frequency_frame();
        // A shift of two and a half pixels, built by an ideal
        // frequency-domain fractional delay rather than by averaging two
        // adjacent columns: a two-tap average is only a clean half-sample
        // delay for a narrow band of frequencies, and this content's own
        // several low frequencies still carry enough distortion through it
        // to miss the tolerance below. The ideal delay needs no
        // resampling library either, only the same scalar FFT plan this
        // module already depends on. Two and a half pixels, not one half,
        // keeps the true offset away from the wrap-around boundary between
        // the lowest and the highest bin, which two whole-pixel bins are
        // exactly tied to be nearest to.
        let candidate = ideal_fractional_shift_right(&base, 2.5);
        let (base_refined, surface) = phase_correlate(&base, &candidate).unwrap();
        let coarse = base_refined.whole;
        let refined = refine_peak(&surface, coarse, 50);
        let total = f64::from(coarse.dx) + f64::from(refined.fractional_x);
        assert!((total - 2.5).abs() < 0.1);
    }

    #[test]
    fn two_runs_on_the_same_input_agree_exactly() {
        let base = synthetic_frame();
        let candidate = shift_frame(&base, 12, 5);
        let (base_refined, surface) = phase_correlate(&base, &candidate).unwrap();
        let coarse = base_refined.whole;
        let refined_a = refine_peak(&surface, coarse, 50);
        let refined_b = refine_peak(&surface, coarse, 50);
        assert_eq!(refined_a, refined_b);
    }

    #[test]
    fn coarse_offset_still_reports_whole_pixels_separately_from_the_fraction() {
        let base = synthetic_frame();
        let candidate = shift_frame(&base, 12, 5);
        let (base_refined, surface) = phase_correlate(&base, &candidate).unwrap();
        let coarse = base_refined.whole;
        let refined = refine_peak(&surface, coarse, 50);
        assert_eq!(refined.whole.dx, 12);
        assert_eq!(refined.whole.dy, 5);
    }
}
