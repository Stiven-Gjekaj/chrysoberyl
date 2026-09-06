---
phase: 01-raster-engine-and-determinism-proof
plan: 04
subsystem: core
tags: [rust, rustfft, libm, fft, phase-correlation, subpixel, determinism]

# Dependency graph
requires:
  - phase: 01-raster-engine-and-determinism-proof
    provides: "chrys-core compare, Frame buffer layout (01-01)"
  - phase: 01-raster-engine-and-determinism-proof
    provides: "rgba8_digest, the single digest contract every committed digest depends on (01-03)"
provides:
  - "chrys_core::register: to_luma_downsampled, hann_table, plan_scalar_fft, phase_correlate, refine_peak"
  - "CoarseOffset, RefinedOffset, CorrelationSurface — the whole-pixel and subpixel translation between a near-identical pair"
  - "A pinned transform digest guarding the FFT arithmetic path, and two grep gates guarding the planner and the cosine implementation"
affects: [01-05, 01-06, 01-07, 01-08]

# Actuals (#2632)
actuals:
  tokens: 12860
  tasks: 2
  commits: 2
plan_head_before: 649f7c208536d56ae5de66396097525bbb26be92

# Tech tracking
tech-stack:
  added: [rustfft=6.4.1, libm=0.2.16]
  patterns:
    - "plan_scalar_fft is the only FFT plan factory chrys-core ever calls; the auto-dispatching FftPlanner never appears outside a comment, enforced by a grep gate at verify time"
    - "Every transcendental in the register stage routes through libm's free functions, never the standard library's method; two grep gates count the standard library method at zero"
    - "The Hann window and the working-resolution constant are fixed, one-time, process-wide inputs (a OnceLock table, a named usize constant), not parameters threaded through call sites"
    - "A fixed synthetic input's transform digest is pinned as a test constant, so a change to the FFT arithmetic path is visible in a unit test, not only in the six-runner CI matrix"
    - "Subpixel refinement evaluates the inverse transform by direct matrix multiplication over a small neighbourhood, in two passes (coarse, then fine), so the transcendental count and the multiply-add count are both functions of the neighbourhood size, never the 512x512 surface size"

key-files:
  created:
    - crates/chrys-core/src/register/mod.rs
    - crates/chrys-core/src/register/luma.rs
    - crates/chrys-core/src/register/window.rs
    - crates/chrys-core/src/register/phase_correlation.rs
    - crates/chrys-core/src/register/subpixel.rs
    - crates/chrys-core/tests/register.rs
  modified:
    - crates/chrys-core/Cargo.toml
    - crates/chrys-core/src/lib.rs

key-decisions:
  - "The realfft open question from 01-RESEARCH.md is settled, not assumed: realfft 3.5.0's source (fetched directly from GitHub at the tag's pinned commit, d0d4eee) shows every one of its FFT constructors takes a `&mut rustfft::FftPlanner<T>`, the concrete auto-dispatching type, with no generic parameter and no scalar-planner override point. realfft is not added to chrys-core's Cargo.toml; phase_correlate calls plan_scalar_fft directly on complex input, exactly the fallback 01-RESEARCH.md named for this outcome."
  - "CorrelationSurface carries the normalized cross-power spectrum (`spectrum: Vec<Complex32>`) alongside the magnitude surface Task 1's own must_haves list names. This is an addition beyond that list, not a substitution: Task 2's subpixel refinement needs the frequency-domain data to build its own small upsampled neighbourhood, and there is no way to recover it from the magnitude-only surface after the fact."
  - "phase_correlate's return type changed from `(CoarseOffset, CorrelationSurface)` to `(RefinedOffset, CorrelationSurface)` in Task 2's commit, per that task's own action text ('phase_correlate returns the refined offset alongside the coarse one'). RefinedOffset::whole carries the exact CoarseOffset value; no separate CoarseOffset element was kept in the tuple, since that would have been the same value carried twice. All of Task 1's own tests were updated in Task 2's commit to destructure the new shape, which is the same file Task 2's own <files> list already names as one it modifies."
  - "The cross-power spectrum is built as `base * conj(candidate)`, exactly as the plan's action text specifies. Working through the resulting sign algebra by hand (and checking it against a small pure-Python DFT before writing the Rust) shows this convention places the coarse peak at `(resolution - shift) mod resolution`, not at `shift` itself; `to_signed_shift` and `unwrap_bin_index` both encode and undo that specific relationship, documented inline so a future reader does not have to re-derive it."
  - "Complex::norm() is not used anywhere in the register stage, even though num_complex provides it, because its own source routes through `.hypot()`, a method Rust's precision documentation does not list among the guaranteed-reproducible ones (only sqrt and mul_add are guaranteed). Every magnitude in this plan is instead `(re*re + im*im).sqrt()`, calling out the guaranteed method by name in a doc comment on the one function that computes it."
  - "The half-pixel subpixel test builds its shifted candidate through an ideal frequency-domain fractional delay (FFT, phase ramp, inverse FFT), not by averaging two adjacent columns as the plan's own action text suggested. A two-tap average is only a clean half-sample delay for a narrow band of frequencies; this content's own several low frequencies still carried enough of the average's own distortion to miss the required tolerance in practice. The ideal delay needs no resampling library either, only the same scalar FFT plan this module already depends on, and was verified independently before being used as a test fixture."

patterns-established:
  - "A determinism-guarded module's own test helpers are held to the same standard library trigonometric ban as its production code: two std sin/cos calls that first appeared only in a subpixel.rs test fixture were routed through libm before commit, so the module's own grep gates hold for the whole file, not just the non-test lines."

requirements-completed: [CORE-02, DET-06]

coverage:
  - id: D1
    description: "A pair whose content is shifted by a known number of pixels reports that shift, with the correct sign on both axes."
    requirement: "CORE-02"
    verification:
      - kind: unit
        ref: "crates/chrys-core/src/register/phase_correlation.rs#tests::a_shift_right_and_down_reports_the_correct_positive_offset"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/register/phase_correlation.rs#tests::the_reversed_shift_reports_the_negative_offset"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/register.rs#a_shift_right_and_down_reports_a_positive_dx_and_dy"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/register.rs#the_reversed_shift_reports_the_negative_offset_on_both_axes"
        status: pass
    human_judgment: false
  - id: D2
    description: "The register stage takes one code path on every CPU: FftPlannerScalar is the only planner constructed, and the auto-dispatching planner's constructor never appears in the engine outside a comment."
    requirement: "DET-06"
    verification:
      - kind: other
        ref: "grep -q 'FftPlannerScalar' crates/chrys-core/src/register/window.rs"
        status: pass
      - kind: other
        ref: "grep -rv comments crates/chrys-core/src/ | grep -c 'FftPlanner::new' = 0"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/register/window.rs#tests::plan_scalar_fft_forward_and_inverse_plan_the_requested_length"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every cosine and sine the register stage needs comes from libm, never the standard library, in both production code and its own test fixtures."
    requirement: "DET-06"
    verification:
      - kind: other
        ref: "grep -rv comments crates/chrys-core/src/register/ | grep -c '\\.cos()' = 0"
        status: pass
      - kind: other
        ref: "grep -rv comments crates/chrys-core/src/register/ | grep -c '\\.sin()' = 0"
        status: pass
      - kind: other
        ref: "grep -q 'libm::cos' crates/chrys-core/src/register/window.rs"
        status: pass
      - kind: other
        ref: "grep -qE 'libm::(sin|cos)' crates/chrys-core/src/register/subpixel.rs"
        status: pass
    human_judgment: false
  - id: D4
    description: "A fixed synthetic input transformed through plan_scalar_fft digests to a pinned constant, so a change to the FFT arithmetic path is visible in a unit test."
    verification:
      - kind: unit
        ref: "crates/chrys-core/tests/register.rs#a_fixed_synthetic_input_digests_to_the_pinned_transform_constant"
        status: pass
    human_judgment: false
  - id: D5
    description: "The correlation peak is located below one pixel by the algorithm the architecture research names (Guizar-Sicairos), with CoarseOffset still reporting whole pixels separately from the fraction."
    requirement: "CORE-02"
    verification:
      - kind: unit
        ref: "crates/chrys-core/src/register/subpixel.rs#tests::a_peak_exactly_on_a_sample_refines_to_a_zero_fractional_part_on_both_axes"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/register/subpixel.rs#tests::a_whole_pixel_shift_refines_to_within_a_hundredth_of_a_pixel_of_zero"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/register/subpixel.rs#tests::a_synthetic_half_pixel_shift_refines_to_within_a_tenth_of_a_pixel_of_one_half"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/register/subpixel.rs#tests::two_runs_on_the_same_input_agree_exactly"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/register/subpixel.rs#tests::coarse_offset_still_reports_whole_pixels_separately_from_the_fraction"
        status: pass
      - kind: other
        ref: "grep -q 'Guizar' crates/chrys-core/src/register/subpixel.rs"
        status: pass
    human_judgment: false

duration: 61min
completed: 2026-09-06
status: complete
---

# Phase 1 Plan 4: The register stage — phase correlation and subpixel refinement Summary

**`chrys_core::register` reports a pair's translation in pixels, refined to a hundredth of a pixel, through a scalar FFT plan pinned by a transform digest and a Hann window built entirely from libm, with two grep gates holding both determinism traps shut.**

## Performance

- **Duration:** 61 min (measured between this plan's final commit and 01-03's final commit)
- **Completed:** 2026-09-06T19:45:27+02:00
- **Tasks:** 2
- **Files modified:** 8 (6 created, 2 modified)

## Accomplishments

- `crates/chrys-core/src/register/window.rs`: `WORKING_RESOLUTION: usize = 512`, `hann_table(n)` (a raised-cosine table computed once in `f64` through `libm::cos`, cast to `f32`), `cached_hann_table()` (a `OnceLock`-backed accessor computed once per process), and `plan_scalar_fft(len, direction)` (the only FFT plan factory this crate ever calls, constructing `rustfft::FftPlannerScalar` exclusively).
- `crates/chrys-core/src/register/luma.rs`: `to_luma_downsampled(frame)`, an integer-only RGBA8-to-luma conversion (fixed-point weights 77/150/29, shift right 8) followed by an integer box downsample to a fixed `WORKING_RESOLUTION` by `WORKING_RESOLUTION` grid, with no floating-point type anywhere in the file.
- `crates/chrys-core/src/register/phase_correlation.rs`: `CoarseOffset { dx, dy }`, `CorrelationSurface { magnitudes, resolution, spectrum }`, and `phase_correlate(base, candidate)`, which downsamples both frames, windows them with the cached Hann table, runs a row-then-column forward 2D FFT through `plan_scalar_fft`, forms the normalized cross-power spectrum (magnitude via `sqrt`, never `Complex::norm`'s `hypot`), runs the inverse transform the same way, and reads the whole-pixel translation off the surface's largest magnitude, tie-broken by the lowest linear index.
- `crates/chrys-core/src/register/subpixel.rs`: `RefinedOffset { whole, fractional_x, fractional_y }` and `refine_peak(surface, coarse, upsample_factor)`, implementing the upsampled-DFT algorithm of Guizar-Sicairos, Thurman and Fienup (Optics Letters 2008) via a direct matrix-multiply transform over a small neighbourhood, in two passes (a coarse pass across the whole ±0.5-pixel range, then a fine pass around that pass's own best point), with every twiddle value built from exactly one `libm::sin`/`libm::cos` pair per candidate position, never per surface bin.
- `phase_correlate` now calls `refine_peak` at a fixed `UPSAMPLE_FACTOR` (100, one hundredth of a pixel) and returns `(RefinedOffset, CorrelationSurface)`; `RefinedOffset::whole` is the exact value CORE-02 reports, unchanged in type and meaning from `CoarseOffset`.
- `crates/chrys-core/tests/register.rs`: 11 integration tests covering the working-grid size, the Hann table's symmetry and stability, four sign/offset behaviours of `phase_correlate`, the pinned transform digest, and four `refine_peak` behaviours (exact-sample peak, whole-pixel accuracy, repeatability, and the CoarseOffset/fraction separation).
- `crates/chrys-core/Cargo.toml` gains `rustfft` and `libm`, both workspace-pinned and both `OK` in 01-RESEARCH.md's Package Legitimacy Audit. It still names no format crate and no graphics crate.

## Task Commits

1. **Task 1: Correlate a pair on one arithmetic path and report the shift in pixels** - `9246e84` (feat + test: `chrys-core/Cargo.toml`, `chrys-core/src/lib.rs`, `chrys-core/src/register/{mod,luma,window,phase_correlation}.rs`, `chrys-core/tests/register.rs`, `Cargo.lock`)
2. **Task 2: Refine the correlation peak below one pixel** - `48747c6` (feat + test: `chrys-core/src/register/{mod,phase_correlation,subpixel}.rs`, `chrys-core/tests/register.rs`)

**Plan metadata:** commit pending (this SUMMARY — orchestrator owns STATE.md/ROADMAP.md writes per the objective given to this executor)

## Files Created/Modified

- `crates/chrys-core/src/register/mod.rs` - declares and re-exports the four child modules
- `crates/chrys-core/src/register/luma.rs` - `to_luma_downsampled`
- `crates/chrys-core/src/register/window.rs` - `WORKING_RESOLUTION`, `hann_table`, `cached_hann_table`, `plan_scalar_fft`
- `crates/chrys-core/src/register/phase_correlation.rs` - `CoarseOffset`, `CorrelationSurface`, `phase_correlate`, `to_signed_shift`, `unwrap_bin_index`
- `crates/chrys-core/src/register/subpixel.rs` - `RefinedOffset`, `refine_peak`, `UPSAMPLE_FACTOR`
- `crates/chrys-core/tests/register.rs` - the 11 integration tests, the pinned transform digest
- `crates/chrys-core/Cargo.toml` - adds `rustfft.workspace = true`, `libm.workspace = true`
- `crates/chrys-core/src/lib.rs` - `pub mod register;`
- `Cargo.lock` - picked up `rustfft` and `libm` and their dependency trees

## Decisions Made

See `key-decisions` in the frontmatter. In short: the `realfft` planner-injection question is answered directly from 3.5.0's own source (no injection point exists; `realfft` is not added), `CorrelationSurface` carries the cross-power spectrum in addition to the magnitudes Task 1's must_haves named, `phase_correlate`'s return type grew a `RefinedOffset` in Task 2's commit exactly as that task's action text specified, the cross-power spectrum's sign convention (`base * conj(candidate)`) is documented inline with the derivation that makes `to_signed_shift`/`unwrap_bin_index` correct, `Complex::norm()` is avoided everywhere in favour of an explicit `sqrt`-based magnitude, and the half-pixel subpixel test uses an ideal frequency-domain shift instead of the plan's suggested column-averaging construction, because the averaging construction did not meet the required tolerance for this content.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Subpixel refinement used the raw (unsigned) frequency-bin index instead of the signed frequency**
- **Found during:** Task 2, while testing `refine_peak` against a genuinely fractional (non-integer) synthetic shift
- **Issue:** The neighbourhood twiddle table in `subpixel.rs` built each value as `exp(i * 2 * pi * position * k / n)` using the raw bin index `k` (`0..n`) directly. A bin above the Nyquist bin represents a negative frequency; the raw index and the signed frequency (`k - n` there) agree only at integer sample positions. Every whole-pixel test in this plan only ever evaluates the surface at integer positions, so this bug passed every one of them, and only produced a visibly wrong (order-of-magnitude, not a small rounding error) fractional part once a truly fractional shift was exercised.
- **Fix:** Built each position's twiddle table from the signed frequency: `k` for `k <= n/2`, `k - n` above that, using one extra `libm::cos`/`libm::sin` pair per position (a correction factor), not one per surface bin, so the neighbourhood-sized transcendental-count property this module documents still holds.
- **Files modified:** `crates/chrys-core/src/register/subpixel.rs`
- **Verification:** Independently reproduced and confirmed in a standalone Python script (a plain DFT, no Rust or rustfft involved) before porting the fix back; after the fix, `several_fractional_shifts_all_refine_close_to_their_true_value` (shifts of 1.5, 12.5 and -3.5 pixels, built via an exact frequency-domain delay) all refine within 0.1 pixel of their true value, and the required half-pixel test passes.
- **Committed in:** `48747c6` (the file was fixed before its first commit, so no separate commit was needed)

**2. [Rule 1 - Bug] A test-only ideal-shift fixture had the same raw-index bug, independently of the module bug above**
- **Found during:** Task 2, while building a debug fixture (`ideal_fractional_shift_right`) to isolate the module bug from a possibly-imprecise test construction
- **Issue:** The fixture applied a fractional-delay phase ramp using the raw FFT bin index, which breaks Hermitian symmetry for a non-integer shift and corrupts the shifted signal's real part once the imaginary part is discarded. At exactly a half-pixel shift this collapsed the signal to its DC value.
- **Fix:** Applied the same signed-frequency correction as deviation 1, independently derived and verified against a plain-Python DFT before being written into the Rust test helper.
- **Files modified:** `crates/chrys-core/src/register/subpixel.rs` (test-only code)
- **Verification:** `ideal_fractional_shift_right(frame, 0.0)` is byte-identical to `frame`; `ideal_fractional_shift_right(frame, 1.0)` is byte-identical to an integer `shift_frame(frame, 1, 0)`; intermediate fractional shifts interpolate smoothly between the two.
- **Committed in:** `48747c6`

---

**Total deviations:** 2 auto-fixed (both Rule 1, both the same underlying signed-frequency mistake caught by this plan's own tests before commit, one in production code and one in a test fixture)
**Impact on plan:** No scope creep. Both fixes are corrections to code written within this plan's own two tasks; neither touches a file or a behaviour this plan did not already own. The plan's own required tolerances (a hundredth of a pixel for a whole-pixel shift, a tenth of a pixel for a half-pixel shift) are met after the fix, not loosened to accommodate it.

## Issues Encountered

None beyond the two auto-fixed deviations above, which were caught and resolved entirely within this session before any commit.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `chrys_core::register` exposes `phase_correlate` and `refine_peak` as the stable global-registration surface plan 01-05 onward builds on (block matching, per-region refinement).
- `CorrelationSurface`'s `spectrum` field is available to any future stage that needs the frequency-domain data without recomputing it.
- The transform digest in `tests/register.rs` and the two grep gates (`FftPlannerScalar` present, `FftPlanner::new` and the standard library trigonometric methods absent) are load-bearing for every later plan in this phase; changing the arithmetic path they guard requires updating the pinned constant and justifying it here or in a later SUMMARY, not silently regenerating it.
- `WORKING_RESOLUTION = 512` is now a committed input every future digest in this repository depends on; changing it is a `reversibility="costly"` decision per this plan's own Task 1.
- No blockers.

## Self-Check: PASSED

All files below were verified present on disk and all commit hashes verified present in `git log --oneline` on this branch before this line was written:
- `crates/chrys-core/src/register/mod.rs`, `luma.rs`, `window.rs`, `phase_correlation.rs`, `subpixel.rs` — FOUND
- `crates/chrys-core/tests/register.rs` — FOUND
- Commits `9246e84`, `48747c6` — FOUND in `git log --oneline`

---
*Phase: 01-raster-engine-and-determinism-proof*
*Completed: 2026-09-06*
