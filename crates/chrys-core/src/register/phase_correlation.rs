//! The global registration step: reduce a pair to the working grid, take
//! one FFT-based path through it, and read the translation off the peak of
//! the resulting surface.

use rustfft::FftDirection;
use rustfft::num_complex::Complex32;

use chrys_source::Frame;

use crate::CompareError;
use crate::register::luma::to_luma_downsampled;
use crate::register::window::{WORKING_RESOLUTION, cached_hann_table, plan_scalar_fft};

/// The whole-pixel translation `phase_correlate` reports, at full-frame
/// scale.
///
/// A positive `dx` means the candidate's content sits to the right of the
/// base's; a positive `dy` means it sits below the base's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoarseOffset {
    /// The horizontal offset, in pixels.
    pub dx: i32,
    /// The vertical offset, in pixels.
    pub dy: i32,
}

/// The result of one phase correlation: the inverse-transform surface a
/// coarse peak search reads its answer from, and the frequency-domain
/// cross-power spectrum that surface was built from.
///
/// The spectrum is carried alongside the surface, not because the coarse
/// stage in this file needs it again, but because plan 01-04's subpixel
/// refinement step needs the frequency-domain data to build its own small
/// upsampled neighbourhood directly, without paying for a second, full
/// high-resolution inverse transform.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrelationSurface {
    /// The magnitude of each bin of the inverse transform, row major, one
    /// row of `resolution` bins per row.
    pub magnitudes: Vec<f32>,
    /// The side length of the square surface, in bins. Always
    /// `WORKING_RESOLUTION`.
    pub resolution: usize,
    /// The normalized cross-power spectrum, row major, one row of
    /// `resolution` bins per row.
    pub spectrum: Vec<Complex32>,
}

/// Register `candidate` against `base` and report the coarse translation
/// between them, in full-frame pixels, alongside the surface that
/// translation was read from.
///
/// The steps: downsample both frames through `to_luma_downsampled`; window
/// each row and column with the cached Hann table; run the forward 2D
/// transform, a row pass followed by a column pass, every 1D plan coming
/// from `plan_scalar_fft`; form the cross-power spectrum by multiplying the
/// baseline spectrum by the conjugate of the candidate spectrum, normalized
/// by its own magnitude (computed with `sqrt`, which Rust's precision
/// documentation states is guaranteed not to change); run the inverse 2D
/// transform the same way; and take the largest magnitude in the surface,
/// breaking a tie by the lowest linear index.
pub fn phase_correlate(
    base: &Frame,
    candidate: &Frame,
) -> Result<(CoarseOffset, CorrelationSurface), CompareError> {
    let base_luma = to_luma_downsampled(base);
    let candidate_luma = to_luma_downsampled(candidate);

    let window = cached_hann_table();
    let base_spectrum = windowed_forward_transform(&base_luma, window);
    let candidate_spectrum = windowed_forward_transform(&candidate_luma, window);

    let n = WORKING_RESOLUTION;
    let mut cross_power = vec![Complex32::new(0.0, 0.0); n * n];
    for i in 0..cross_power.len() {
        let product = base_spectrum[i] * candidate_spectrum[i].conj();
        let magnitude = complex_magnitude(product);
        cross_power[i] = if magnitude == 0.0 {
            Complex32::new(0.0, 0.0)
        } else {
            product / magnitude
        };
    }

    let mut inverse = cross_power.clone();
    transform_2d(&mut inverse, n, FftDirection::Inverse);

    let magnitudes: Vec<f32> = inverse.iter().map(|&c| complex_magnitude(c)).collect();

    let (peak_index, _) = magnitudes.iter().enumerate().fold(
        (0usize, f32::MIN),
        |(best_index, best_value), (index, &value)| {
            if value > best_value {
                (index, value)
            } else {
                (best_index, best_value)
            }
        },
    );

    let peak_x = peak_index % n;
    let peak_y = peak_index / n;
    let coarse = CoarseOffset {
        dx: to_signed_shift(peak_x, n),
        dy: to_signed_shift(peak_y, n),
    };

    Ok((
        coarse,
        CorrelationSurface {
            magnitudes,
            resolution: n,
            spectrum: cross_power,
        },
    ))
}

/// Compute `|c|` with `sqrt`, never with `Complex::norm`, which routes
/// through `hypot`. Rust's precision documentation guarantees `sqrt` is
/// bit-reproducible; it makes no such guarantee for `hypot`.
fn complex_magnitude(c: Complex32) -> f32 {
    (c.re * c.re + c.im * c.im).sqrt()
}

/// Convert a raw, unwrapped bin index into a signed offset, treating
/// indices above half the working resolution as negative.
///
/// The cross-power spectrum in `phase_correlate` is built as `base *
/// conj(candidate)`, so the surface this function reads peaks at
/// `(resolution - shift) mod resolution` for a candidate whose content
/// moved by `shift`, not at `shift` itself. This function undoes both
/// steps in one pass: it centres the raw index into the
/// `-(resolution/2)..=(resolution/2)` range, then negates it to recover the
/// shift.
fn to_signed_shift(peak_index: usize, resolution: usize) -> i32 {
    let half = resolution / 2;
    let raw = if peak_index > half {
        peak_index as i32 - resolution as i32
    } else {
        peak_index as i32
    };
    -raw
}

/// Window `luma` by `window` on both axes and run the forward 2D
/// transform: a row pass, then a column pass, every 1D plan coming from
/// `plan_scalar_fft`.
fn windowed_forward_transform(luma: &[u8], window: &[f32]) -> Vec<Complex32> {
    let n = WORKING_RESOLUTION;
    let mut data: Vec<Complex32> = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let value = f32::from(luma[y * n + x]) * window[x] * window[y];
            data.push(Complex32::new(value, 0.0));
        }
    }
    transform_2d(&mut data, n, FftDirection::Forward);
    data
}

/// Run a 2D transform over a row-major `n` by `n` buffer of complex
/// samples, in `direction`: a row pass, then a column pass, every 1D plan
/// coming from `plan_scalar_fft`.
fn transform_2d(data: &mut [Complex32], n: usize, direction: FftDirection) {
    let row_fft = plan_scalar_fft(n, direction);
    for row in data.chunks_mut(n) {
        row_fft.process(row);
    }

    let column_fft = plan_scalar_fft(n, direction);
    let mut column_buffer = vec![Complex32::new(0.0, 0.0); n];
    for x in 0..n {
        for (y, sample) in column_buffer.iter_mut().enumerate() {
            *sample = data[y * n + x];
        }
        column_fft.process(&mut column_buffer);
        for (y, sample) in column_buffer.iter().enumerate() {
            data[y * n + x] = *sample;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a frame at exactly `WORKING_RESOLUTION` on each side, filled
    /// with a background colour, with a few high-contrast rectangles
    /// painted onto it so the content has structure a correlation can
    /// lock onto.
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

    /// Shift `frame`'s content right by `dx` and down by `dy`, wrapping
    /// around at the edges so the shifted content never runs off the
    /// frame. A negative value shifts left or up.
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
    fn a_shift_right_and_down_reports_the_correct_positive_offset() {
        let base = synthetic_frame();
        let candidate = shift_frame(&base, 12, 5);
        let (offset, _surface) = phase_correlate(&base, &candidate).unwrap();
        assert_eq!(offset.dx, 12);
        assert_eq!(offset.dy, 5);
    }

    #[test]
    fn the_reversed_shift_reports_the_negative_offset() {
        let base = synthetic_frame();
        let candidate = shift_frame(&base, -12, -5);
        let (offset, _surface) = phase_correlate(&base, &candidate).unwrap();
        assert_eq!(offset.dx, -12);
        assert_eq!(offset.dy, -5);
    }

    #[test]
    fn two_identical_frames_report_a_zero_offset() {
        let base = synthetic_frame();
        let (offset, _surface) = phase_correlate(&base, &base.clone()).unwrap();
        assert_eq!(offset.dx, 0);
        assert_eq!(offset.dy, 0);
    }

    #[test]
    fn two_runs_on_the_same_pair_return_bit_identical_surfaces() {
        let base = synthetic_frame();
        let candidate = shift_frame(&base, 12, 5);
        let (_offset_a, surface_a) = phase_correlate(&base, &candidate).unwrap();
        let (_offset_b, surface_b) = phase_correlate(&base, &candidate).unwrap();
        assert_eq!(surface_a.magnitudes, surface_b.magnitudes);
    }
}
