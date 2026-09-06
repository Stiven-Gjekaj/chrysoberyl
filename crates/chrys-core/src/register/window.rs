//! The working resolution, the window coefficient table, and the FFT plan
//! factory. This module holds two of this phase's named determinism traps:
//! the FFT planner (see `plan_scalar_fft`) and the window's cosine (see
//! `hann_table`).

use std::sync::{Arc, OnceLock};

use rustfft::{Fft, FftDirection, FftPlannerScalar};

/// The fixed side length, in pixels, of the square grid every pair is
/// reduced to before the register stage runs any transform.
///
/// This value is a determinism input, not a performance knob: it fixes the
/// size of the window coefficient table below, and the length of every FFT
/// `plan_scalar_fft` plans. Changing it invalidates every digest this
/// repository has committed, here and in any later plan's baseline
/// manifest, because every one of them depends on a transform of this exact
/// length.
pub const WORKING_RESOLUTION: usize = 512;

static HANN_TABLE: OnceLock<Vec<f32>> = OnceLock::new();

/// Return the Hann window coefficient table at `WORKING_RESOLUTION`,
/// computed once per process.
///
/// The first call builds the table through `hann_table` and caches it in a
/// `OnceLock`. Every later call, on every thread, returns that same `Vec`
/// without recomputing it, so the table is never rebuilt per pixel and
/// never rebuilt per call.
pub fn cached_hann_table() -> &'static [f32] {
    HANN_TABLE.get_or_init(|| hann_table(WORKING_RESOLUTION))
}

/// Build a raised-cosine (Hann) window of `n` coefficients:
/// `w(i) = 0.5 - 0.5 * cos(2 * pi * i / (n - 1))`.
///
/// Each coefficient is computed in `f64`, through the pure-Rust `libm`
/// crate's cosine function, and cast to `f32`. Never call the standard
/// library's cosine method here or anywhere else in the register stage:
/// Rust's own precision documentation lists that method as having
/// unspecified precision that varies by platform, by Rust version, and even
/// between two calls in one execution. Building this table with a platform
/// cosine would make the window itself a source of cross-machine drift,
/// before the transform it feeds ever runs.
///
/// The table is symmetric about its centre, and its first and last
/// coefficients are always zero. A table of one or zero coefficients has no
/// centre to raise a cosine around, so this returns a table of zeros for
/// `n <= 1` rather than dividing by zero.
pub fn hann_table(n: usize) -> Vec<f32> {
    if n <= 1 {
        return vec![0.0; n];
    }
    let denominator = (n - 1) as f64;
    (0..n)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / denominator;
            let cosine = libm::cos(angle);
            (0.5 - 0.5 * cosine) as f32
        })
        .collect()
}

/// Construct an FFT plan of length `len`, in `direction`, through the
/// scalar (non-SIMD) planner.
///
/// `FftPlannerScalar` is the only planner this crate ever constructs. The
/// auto-dispatching planner (`rustfft::FftPlanner`) detects AVX, SSE and
/// NEON at run time and switches algorithm depending on what the running
/// CPU supports, so two machines running the same build can take different
/// arithmetic through the same call. The scalar planner never dispatches,
/// so every machine this project's CI matrix covers takes the same
/// arithmetic path.
pub fn plan_scalar_fft(len: usize, direction: FftDirection) -> Arc<dyn Fft<f32>> {
    let mut planner = FftPlannerScalar::new();
    planner.plan_fft(len, direction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hann_table_returns_n_coefficients() {
        assert_eq!(hann_table(8).len(), 8);
    }

    #[test]
    fn hann_table_is_zero_at_both_ends() {
        let table = hann_table(16);
        assert_eq!(table[0], 0.0);
        assert_eq!(table[15], 0.0);
    }

    #[test]
    fn hann_table_is_symmetric_about_its_centre() {
        let table = hann_table(9);
        for i in 0..table.len() {
            let mirror = table.len() - 1 - i;
            assert!((table[i] - table[mirror]).abs() < f32::EPSILON * 8.0);
        }
    }

    #[test]
    fn hann_table_returns_the_same_values_on_every_call() {
        assert_eq!(hann_table(64), hann_table(64));
    }

    #[test]
    fn cached_hann_table_matches_a_fresh_computation_at_working_resolution() {
        assert_eq!(
            cached_hann_table(),
            hann_table(WORKING_RESOLUTION).as_slice()
        );
    }

    #[test]
    fn plan_scalar_fft_forward_and_inverse_plan_the_requested_length() {
        let forward = plan_scalar_fft(64, FftDirection::Forward);
        let inverse = plan_scalar_fft(64, FftDirection::Inverse);
        assert_eq!(forward.len(), 64);
        assert_eq!(inverse.len(), 64);
    }
}
