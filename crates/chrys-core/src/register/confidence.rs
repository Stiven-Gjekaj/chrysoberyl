//! The peak-confidence metric: how sharp the phase-correlation peak is
//! relative to the rest of the correlation surface.
//!
//! A near-identical pair leaves a narrow, tall peak standing over a low,
//! flat floor. An unrelated pair leaves a flat surface with no peak that
//! stands out from its own noise. Kuglin and Hines, "The phase correlation
//! image alignment method" (1975 IEEE Conference on Cybernetics and
//! Society), and Reddy and Chatterji, "An FFT-based technique for
//! translation, rotation, and scale-invariant image registration" (IEEE
//! Transactions on Image Processing, 1996), both read this peak-to-floor
//! sharpness as the signal that a phase-correlation result can be trusted.
//! This module turns that signal into one number `compare` tests before it
//! warps or classifies anything.

use crate::register::phase_correlation::CorrelationSurface;

/// The side, in bins, of the square window excluded from the noise-floor
/// average, centred on the peak.
///
/// A sharp peak carries its own energy into any average that includes it,
/// so every bin inside this window, including the peak bin itself, is left
/// out of the floor sum. The window is measured with wrapped distance (see
/// `toroidal_distance`), because a phase-correlation surface is periodic:
/// a peak sitting at row or column zero has its true neighbours on the
/// opposite edge of the surface, not off the edge of the array.
const EXCLUSION_WINDOW_SIDE: usize = 5;

/// The peak magnitude, the surrounding noise floor, and their ratio, for
/// one correlation surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakConfidence {
    /// The magnitude at the surface's peak bin.
    pub peak: f32,
    /// The mean magnitude of every bin outside the exclusion window.
    pub floor: f32,
    /// `peak / floor`, a finite number in every case, never infinite and
    /// never NaN.
    ///
    /// A surface with no signal at all (`peak` and `floor` both zero)
    /// reports a ratio of zero: there is nothing here to be confident
    /// about. A surface with a real peak and an exactly-zero floor (an
    /// exactly-identical pair can drive the floor to exact zero; see the
    /// module tests) reports `f32::MAX`, the largest confidence this type
    /// can hold, rather than the undefined result of dividing by zero.
    /// Reporting zero there instead would rank the sharpest possible peak
    /// below every noisy, low-confidence surface that has a nonzero
    /// floor, which inverts the ordering this metric exists to preserve.
    pub ratio: f32,
}

/// Score how sharply `surface` peaks at `peak_index`, relative to the rest
/// of the surface.
///
/// The floor is the mean magnitude over every bin outside a square,
/// `EXCLUSION_WINDOW_SIDE`-wide window centred on the peak. The sum that
/// builds the floor walks the surface once, row by row and, within a row,
/// column by column, always in that order. Floating-point addition is not
/// associative, so a sum built in a different order is a different `f64`
/// value and, after rounding to `f32`, sometimes a different floor. This
/// function reaches a threshold test that decides a refusal, so its own
/// determinism depends on that fixed order never changing.
pub fn assess_peak(surface: &CorrelationSurface, peak_index: usize) -> PeakConfidence {
    let resolution = surface.resolution;
    let peak = surface.magnitudes[peak_index];
    let peak_x = peak_index % resolution;
    let peak_y = peak_index / resolution;
    let half_window = EXCLUSION_WINDOW_SIDE / 2;

    let mut floor_sum = 0.0f64;
    let mut floor_count: usize = 0;
    for y in 0..resolution {
        let dy = toroidal_distance(y, peak_y, resolution);
        for x in 0..resolution {
            let dx = toroidal_distance(x, peak_x, resolution);
            if dx <= half_window && dy <= half_window {
                continue;
            }
            floor_sum += f64::from(surface.magnitudes[y * resolution + x]);
            floor_count += 1;
        }
    }

    let floor = if floor_count == 0 {
        0.0
    } else {
        (floor_sum / floor_count as f64) as f32
    };
    let ratio = if floor != 0.0 {
        peak / floor
    } else if peak != 0.0 {
        f32::MAX
    } else {
        0.0
    };

    PeakConfidence { peak, floor, ratio }
}

/// The shortest distance between position `a` and position `b` on a ring
/// of `resolution` positions: the smaller of the direct distance and the
/// distance the other way around the ring.
fn toroidal_distance(a: usize, b: usize, resolution: usize) -> usize {
    let direct = a.abs_diff(b);
    direct.min(resolution - direct)
}

/// The ratio below which `compare` refuses a pair instead of classifying
/// it, because CORE-06 and CORE-07 require the engine to say so, not to
/// guess, when a pair falls outside the near-identical assumption.
///
/// This number came from `crates/chrys-core/tests/refusal.rs`, not from
/// inspection. On the corpus committed under `tests/golden/refuse-01/`,
/// the lowest `should-register` ratio measured 3041.24 and the highest
/// `should-refuse` ratio measured 1586.52; this constant is the midpoint
/// of those two, rounded to two decimal places. Changing this number
/// without re-running that test against the corpus is editing a result to
/// fit a case, not measuring one; if the corpus changes, re-run the test,
/// read the two new bounds it prints, and update this constant and this
/// comment together.
pub const REFUSAL_THRESHOLD: f32 = 2313.88;

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_surface(
        resolution: usize,
        floor_value: f32,
        peak_index: usize,
        peak_value: f32,
    ) -> CorrelationSurface {
        let mut magnitudes = vec![floor_value; resolution * resolution];
        magnitudes[peak_index] = peak_value;
        CorrelationSurface {
            magnitudes,
            resolution,
            spectrum: Vec::new(),
        }
    }

    #[test]
    fn a_sharp_peak_over_a_flat_floor_reports_the_floor_value_and_the_ratio() {
        let surface = flat_surface(16, 2.0, 40, 20.0);
        let confidence = assess_peak(&surface, 40);
        assert_eq!(confidence.peak, 20.0);
        assert_eq!(confidence.floor, 2.0);
        assert_eq!(confidence.ratio, 10.0);
    }

    #[test]
    fn a_zero_floor_with_no_peak_either_reports_a_zero_ratio() {
        let surface = flat_surface(16, 0.0, 0, 0.0);
        let confidence = assess_peak(&surface, 0);
        assert_eq!(confidence.floor, 0.0);
        assert_eq!(confidence.ratio, 0.0);
        assert!(confidence.ratio.is_finite());
    }

    #[test]
    fn a_zero_floor_with_a_real_peak_reports_the_largest_finite_ratio_not_infinity() {
        let surface = flat_surface(16, 0.0, 0, 5.0);
        let confidence = assess_peak(&surface, 0);
        assert_eq!(confidence.floor, 0.0);
        assert_eq!(confidence.ratio, f32::MAX);
        assert!(confidence.ratio.is_finite());
    }

    #[test]
    fn a_peak_at_the_surface_origin_excludes_its_wrapped_neighbours_from_the_floor() {
        let resolution = 16;
        let mut magnitudes = vec![2.0f32; resolution * resolution];
        // The far edge of a periodic surface is the peak's own neighbour.
        // Raise it well above the floor and confirm it is excluded, not
        // averaged in.
        magnitudes[resolution - 1] = 200.0;
        magnitudes[(resolution - 1) * resolution] = 200.0;
        magnitudes[0] = 20.0;
        let surface = CorrelationSurface {
            magnitudes,
            resolution,
            spectrum: Vec::new(),
        };
        let confidence = assess_peak(&surface, 0);
        assert_eq!(confidence.floor, 2.0);
    }

    #[test]
    fn two_runs_on_the_same_surface_return_the_same_numbers() {
        let surface = flat_surface(32, 3.0, 500, 40.0);
        let a = assess_peak(&surface, 500);
        let b = assess_peak(&surface, 500);
        assert_eq!(a, b);
    }
}
