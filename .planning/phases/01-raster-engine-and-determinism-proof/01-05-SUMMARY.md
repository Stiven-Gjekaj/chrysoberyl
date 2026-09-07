---
phase: 01-raster-engine-and-determinism-proof
plan: 05
subsystem: core
tags: [rust, phase-correlation, refusal, calibration, determinism]

# Dependency graph
requires:
  - phase: 01-raster-engine-and-determinism-proof
    provides: "chrys_core::register: phase_correlate, CorrelationSurface, unwrap_bin_index (01-04)"
provides:
  - "chrys_core::register::confidence: PeakConfidence, assess_peak, REFUSAL_THRESHOLD"
  - "chrys_core::register::peak_index, recovering the raw surface index a CoarseOffset came from"
  - "Verdict::Refused(RefusalReason::PeakConfidenceTooLow), wired into compare before any classification runs"
  - "The tests/golden/refuse-01/ calibration corpus: eight should-register and eight should-refuse pairs"
affects: [01-06, 01-07, 01-08]

# Actuals (#2632)
actuals:
  tokens: 10595
  tasks: 2
  commits: 2
plan_head_before: d3ed9397102426f97ce8cb9dac6831443c8e3fa6

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "The peak-confidence floor is measured with wrapped (toroidal) distance from the peak, because a phase-correlation surface is periodic and a peak near one edge has its true neighbours on the opposite edge"
    - "The floor sum walks the surface once, row by row and column by column, in a fixed order, because floating-point addition is not associative and this sum feeds a boolean refusal decision"
    - "The refusal threshold is a midpoint derived from a committed, measured corpus (tests/golden/refuse-01/), never a literal chosen by inspection, with a doc comment recording the two bounds and the rule"
    - "compare's fixed step order is now: shape check, then registration, then the confidence test, then classification; the confidence test's early return is commented as the point, not an optimisation"
    - "A binary-target-only crate (chrys-cli) cannot be a dependency (dev or otherwise) of another crate's tests, because Cargo requires a lib target for that; CARGO_BIN_EXE_<name> is scoped to the package that declares the binary, so CLI-exit-code tests for chrys live in chrys-cli's own tests/, not chrys-core's"

key-files:
  created:
    - crates/chrys-core/src/register/confidence.rs
    - crates/chrys-core/tests/refusal.rs
    - crates/chrys-cli/tests/refusal.rs
    - tests/golden/refuse-01/should-register/pair-01..08/{base,candidate}.png
    - tests/golden/refuse-01/should-refuse/pair-01..08/{base,candidate}.png
  modified:
    - crates/chrys-core/src/lib.rs
    - crates/chrys-core/src/register/mod.rs
    - crates/chrys-core/src/register/phase_correlation.rs
    - crates/chrys-core/src/verdict.rs
    - crates/chrys-core/Cargo.toml
    - crates/chrys-source-raster/examples/make-fixtures.rs

key-decisions:
  - "assess_peak's zero-floor case departs from the plan's literal behaviour text. An exactly-identical pair drives the floor to exact zero (a bit-exact constant cross-power spectrum) while the peak stays enormous; returning ratio 0 there, as the plan's own words say, would rank the sharpest possible peak below every noisy should-refuse pair, inverting the separation this task exists to prove. assess_peak now returns f32::MAX (finite, never NaN or infinite) for a real peak over a zero floor, and 0 only when the peak is zero too."
  - "chrys-core gained chrys-source-raster as a dev-dependency only, so tests/refusal.rs can decode the committed corpus through the real raster decoder, the same way chrys-cli already depends on it for its own tests. chrys-core's production dependency graph is unchanged: chrys-source, thiserror, sha2, rustfft, libm."
  - "peak_index(offset, resolution) was added as a public function in register::phase_correlation, undoing unwrap_bin_index on both axes. Without it, neither compare nor an external test could recover the raw surface index assess_peak needs from the CoarseOffset phase_correlate reports; this is Rule 2, the missing link the plan's own key_link (compare -> assess_peak) depends on."
  - "The CLI exit-code/wording test from Task 2's behaviour list lives in a new crates/chrys-cli/tests/refusal.rs, not chrys-core/tests/refusal.rs as the plan's action text describes. CARGO_BIN_EXE_chrys is only set for the package that declares that binary target (chrys-cli), and chrys-cli cannot be listed as a dependency of chrys-core at all, in any dependency section, because it has no lib target for Cargo to link against. chrys-core/tests/refusal.rs still carries six tests, clearing the plan's own numeric gate, by adding four more refusal-behaviour tests scoped to the library level."
  - "The pre-existing a_recoloured_rectangle_returns_one_recoloured_region unit test (predating this plan) used a flat 8x8 frame with only the one changed patch. Registration correctly refused that content at ratio ~15, regardless of how much the frame was scaled up, because a flat frame gives phase correlation no edges to lock onto. The fixture was changed to include four anchor rectangles that stay identical between base and candidate, matching the synthetic_frame style already used in phase_correlation.rs's, subpixel.rs's and register.rs's own tests, so the test exercises classification on content the new gate can genuinely register."

patterns-established:
  - "A test fixture fed through compare() must carry real background structure (multiple anchor edges), not a flat colour plus one patch, because phase correlation now gates every comparison and a flat frame has nothing for it to lock onto regardless of scale."

requirements-completed: [CORE-06, CORE-07]

coverage:
  - id: D1
    description: "The peak-confidence metric (PeakConfidence, assess_peak) scores how sharply a correlation surface peaks, excluding a wrapped window around the peak from its own floor, in a fixed summation order."
    requirement: "CORE-06"
    verification:
      - kind: unit
        ref: "crates/chrys-core/src/register/confidence.rs#tests::a_sharp_peak_over_a_flat_floor_reports_the_floor_value_and_the_ratio"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/register/confidence.rs#tests::a_peak_at_the_surface_origin_excludes_its_wrapped_neighbours_from_the_floor"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/register/confidence.rs#tests::two_runs_on_the_same_surface_return_the_same_numbers"
        status: pass
    human_judgment: false
  - id: D2
    description: "A committed, measured calibration corpus (tests/golden/refuse-01/) separates should-register pairs from should-refuse pairs with a strictly positive gap, and REFUSAL_THRESHOLD is the midpoint of that gap, not a literal chosen by inspection."
    requirement: "CORE-06"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/refusal.rs#should_register_pairs_all_score_above_every_should_refuse_pair"
        status: pass
    human_judgment: false
  - id: D3
    description: "compare refuses every should-refuse pair with Verdict::Refused(RefusalReason::PeakConfidenceTooLow), carrying the measured ratio and the threshold, and never refuses a should-register pair; the Display text names the pair as too different to register."
    requirement: "CORE-07"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/refusal.rs#every_should_refuse_pair_is_refused_with_peak_confidence_too_low"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/refusal.rs#no_should_register_pair_is_refused"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/refusal.rs#a_refusal_names_the_measured_ratio_and_the_threshold_it_failed"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/refusal.rs#a_should_refuse_verdicts_display_text_names_the_pair_too_different_to_register"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/verdict.rs#tests::peak_confidence_too_low_prints_the_ratio_the_threshold_and_the_words_too_different"
        status: pass
    human_judgment: false
  - id: D4
    description: "The CLI exits 2 on a should-refuse pair (with the refusal text on stdout) and 1 on a should-register pair, and the pre-existing golden pair-01 and all four format pairs still produce a Changed verdict (exit 1), never a refusal."
    requirement: "CORE-07"
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/refusal.rs#the_cli_exits_2_and_names_the_refusal_on_a_should_refuse_pair"
        status: pass
      - kind: integration
        ref: "crates/chrys-cli/tests/refusal.rs#the_cli_exits_1_on_a_should_register_pair"
        status: pass
      - kind: manual_procedural
        ref: "cargo run -p chrys-cli -- compare tests/golden/pair-01/{base,candidate}.png (exit 1) and the four tests/golden/formats/*/{base,candidate}.* pairs (all exit 1), run interactively during this plan"
        status: pass
    human_judgment: false

duration: 74min
completed: 2026-09-06
status: complete
---

# Phase 1 Plan 5: The refusal gate — peak confidence and CORE-06/CORE-07 Summary

**`compare` now measures how sharply its own phase-correlation peak stands over the surface's noise floor before it classifies anything, and refuses a pair whose peak falls below a threshold derived from a committed, measured corpus, in words and with exit code 2.**

## Performance

- **Duration:** 74 min (measured between this plan's final commit and 01-04's final commit)
- **Tasks:** 2
- **Files modified:** 10 code/test files, plus 32 corpus PNGs (16 pairs) newly committed under `tests/golden/refuse-01/`

## Accomplishments

- `crates/chrys-core/src/register/confidence.rs`: `PeakConfidence { peak, floor, ratio }` and `assess_peak(surface, peak_index)`, citing Kuglin and Hines (1975) and Reddy and Chatterji (1996). The floor excludes a `5`-bin-wide square window around the peak, measured with wrapped (toroidal) distance since a phase-correlation surface is periodic, and sums in a fixed row-major order for determinism. `REFUSAL_THRESHOLD: f32 = 2313.88`, the midpoint of the two bounds the separation test measured, with a doc comment recording both bounds and the re-measurement rule.
- A new calibration corpus, `tests/golden/refuse-01/`, built entirely from deterministic integer formulas in `crates/chrys-source-raster/examples/make-fixtures.rs`: eight `should-register` pairs (identical, three shift magnitudes, two recolour magnitudes, one added element, one combined shift+recolour) and eight `should-refuse` pairs (two different-shape-set pairs, two mirrors, two noise seeds, two solid colours). The generator's existing `pair-01` and `formats/` output is byte-identical to before this plan.
- **Measured ratios** (`crates/chrys-core/tests/refusal.rs`, real PNG decode through `chrys-source-raster`, run against the committed corpus):
  - `should-register`: pair-01 (identical) `f32::MAX`, pair-02 `16284.24`, pair-03 `9185.38`, pair-04 `35396.97`, pair-05 `36059.18`, pair-06 `7784.68`, pair-07 `36514.25`, pair-08 (combined) `3041.24` — the lowest of the group.
  - `should-refuse`: pair-01 `976.82`, pair-02 `1435.27`, pair-03 `25.82`, pair-04 `7.84`, pair-05 `899.11`, pair-06 (mirrored, vertical) `1586.52` — the highest of the group, pair-07 `28.02`, pair-08 `8.02`.
  - **Gap:** `3041.24 - 1586.52 = 1454.72`. `REFUSAL_THRESHOLD` is the midpoint, `2313.88`.
- `chrys_core::register::peak_index(offset, resolution)`, a new public function in `phase_correlation.rs` that undoes `unwrap_bin_index` on both axes, recovering the raw surface index `assess_peak` needs from the `CoarseOffset` `phase_correlate` already reports.
- `RefusalReason::PeakConfidenceTooLow { ratio, threshold }` in `crates/chrys-core/src/verdict.rs`, whose `Display` text states the pair is too different to register, gives both numbers to two decimal places, and states the engine compares near-identical pairs only.
- `compare` in `crates/chrys-core/src/lib.rs` now runs, in fixed order: shape check, `phase_correlate`, `assess_peak`, the threshold test, and only then classification. The early return on a failing confidence test is commented as the point, not an optimisation; nothing after it runs once a refusal is decided.
- The CLI's own exit-code/wording behaviour (exit 2 on a refusal, stdout containing "too different") is covered by a new `crates/chrys-cli/tests/refusal.rs`, verified interactively against `pair-01` and all four `formats/` pairs (all exit 1, never refused).

## Task Commits

1. **Task 1: Measure peak confidence across a corpus a person has judged** - `f26ca15` (feat + test: `chrys-core/src/register/confidence.rs`, `chrys-core/src/register/{mod,phase_correlation}.rs`, `chrys-core/Cargo.toml`, `chrys-core/tests/refusal.rs`, `chrys-source-raster/examples/make-fixtures.rs`, `tests/golden/refuse-01/`, `Cargo.lock`)
2. **Task 2: Refuse a pair the engine cannot register, and say why** - `9c44dc6` (feat + test: `chrys-core/src/{lib,verdict}.rs`, `chrys-core/src/register/{mod,confidence}.rs`, `chrys-core/tests/refusal.rs`, `chrys-cli/tests/refusal.rs`)

**Plan metadata:** commit pending (this SUMMARY — orchestrator owns STATE.md/ROADMAP.md writes per the objective given to this executor)

## Files Created/Modified

- `crates/chrys-core/src/register/confidence.rs` - `PeakConfidence`, `assess_peak`, `REFUSAL_THRESHOLD`
- `crates/chrys-core/src/register/phase_correlation.rs` - `peak_index`, and its round-trip test
- `crates/chrys-core/src/register/mod.rs` - re-exports `confidence::{PeakConfidence, REFUSAL_THRESHOLD, assess_peak}` and `phase_correlation::peak_index`
- `crates/chrys-core/src/verdict.rs` - `RefusalReason::PeakConfidenceTooLow`, its `Display` text, and its test
- `crates/chrys-core/src/lib.rs` - `compare`'s new registration-then-confidence-then-classify order, and a restructured `a_recoloured_rectangle_returns_one_recoloured_region` fixture
- `crates/chrys-core/Cargo.toml` - `chrys-source-raster` as a dev-dependency only
- `crates/chrys-core/tests/refusal.rs` - the separation test and five refusal-behaviour tests
- `crates/chrys-cli/tests/refusal.rs` - the CLI exit-code/wording tests
- `crates/chrys-source-raster/examples/make-fixtures.rs` - the `refuse-01` corpus generator
- `tests/golden/refuse-01/should-register/pair-01..08/`, `tests/golden/refuse-01/should-refuse/pair-01..08/` - the committed calibration corpus (32 PNGs)

## Decisions Made

See `key-decisions` in the frontmatter. In short: `assess_peak`'s zero-floor case returns `f32::MAX` for a real peak over an exactly-zero floor rather than the plan's literal "ratio of zero," because the latter would refuse a bit-exact identical pair (`f26ca15`'s corpus measurement surfaced this before any threshold existed); `chrys-source-raster` became a `chrys-core` dev-dependency so the separation test can decode real PNGs without adding a format crate to the production graph; `peak_index` was added as a small public function to close the gap between `phase_correlate`'s signed offset and `assess_peak`'s raw-index parameter; the CLI-exit-code test moved to `chrys-cli/tests/refusal.rs` because `CARGO_BIN_EXE_chrys` is scoped to the package declaring that binary and `chrys-cli` cannot be a dependency of `chrys-core` at all (no lib target); and the pre-existing recoloured-rectangle test's flat fixture was given real background structure so it exercises classification on content the new gate can register.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `assess_peak` returning ratio 0 on a real peak over a zero floor inverted the separation property**
- **Found during:** Task 1, first run of the separation test against the committed corpus
- **Issue:** The plan's own behaviour text says `assess_peak` "returns a ratio of zero... when the floor is zero." An exactly bit-identical pair (the `should-register/pair-01` "identical" case) drives the floor to exact zero, because the cross-power spectrum becomes a perfectly constant array whose inverse transform cancels to exact zero everywhere except the peak bin, in this project's floating-point FFT. Returning ratio 0 there, following the plan's literal words, made the single most confident pair in the whole corpus score the same as (or below) every noisy `should-refuse` pair, which would have made the separation test impossible to pass honestly, and would have made a real bit-identical comparison refuse in production.
- **Fix:** `assess_peak` now distinguishes "no signal at all" (`peak == 0.0 && floor == 0.0`, ratio 0) from "a real, unmeasurably sharp peak" (`peak != 0.0 && floor == 0.0`, ratio `f32::MAX`, finite and never NaN or infinite).
- **Files modified:** `crates/chrys-core/src/register/confidence.rs`
- **Verification:** `register::confidence::tests::a_zero_floor_with_no_peak_either_reports_a_zero_ratio`, `register::confidence::tests::a_zero_floor_with_a_real_peak_reports_the_largest_finite_ratio_not_infinity`; the separation test (`crates/chrys-core/tests/refusal.rs`) passes with the identical pair correctly scoring the maximum ratio in the corpus.
- **Committed in:** `f26ca15`

**2. [Rule 2 - Missing functionality] `peak_index` did not exist, and `compare` could not call `assess_peak` without it**
- **Found during:** Task 1, while wiring the separation test to `phase_correlate`'s output
- **Issue:** `assess_peak` takes a raw linear surface index; `phase_correlate` only reports a signed `CoarseOffset`. Nothing in the crate converted one to the other outside `subpixel.rs`'s private use of `unwrap_bin_index`, so neither an external test nor `compare` itself (in Task 2) could locate the peak `assess_peak` needs to score.
- **Fix:** Added `pub fn peak_index(offset: CoarseOffset, resolution: usize) -> usize` to `phase_correlation.rs`, undoing `unwrap_bin_index` on both axes, and re-exported it from `register::mod`.
- **Files modified:** `crates/chrys-core/src/register/phase_correlation.rs`, `crates/chrys-core/src/register/mod.rs`
- **Verification:** `register::phase_correlation::tests::peak_index_recovers_the_raw_linear_index_a_coarse_offset_came_from`
- **Committed in:** `f26ca15`

**3. [Rule 3 - Blocking issue] `CARGO_BIN_EXE_chrys` is not defined in `chrys-core`'s own test binaries**
- **Found during:** Task 2, writing the plan-specified CLI-exit-code test into `crates/chrys-core/tests/refusal.rs`
- **Issue:** The plan's action text asks for a test in `chrys-core/tests/refusal.rs` that drives the built `chrys` binary via `env!("CARGO_BIN_EXE_chrys")`. Cargo only sets that variable for integration tests belonging to the package that itself declares the `[[bin]]` target, which is `chrys-cli`. `chrys-cli` also cannot be added as a `chrys-core` dependency at all, in any section, because it has no `[lib]` target for Cargo to link against (`cargo` reported "ignoring invalid dependency `chrys-cli` which is missing a lib target" on the attempt).
- **Fix:** Moved the CLI-level test to a new `crates/chrys-cli/tests/refusal.rs`, where the package already declares the `chrys` binary. `chrys-core/tests/refusal.rs` keeps the library-level refusal behaviour (five tests) plus the separation test (one), six total, clearing the plan's own "at least six tests" gate without the infeasible test.
- **Files modified:** `crates/chrys-cli/tests/refusal.rs` (new), `crates/chrys-core/tests/refusal.rs` (module doc comment updated to point at the new file)
- **Verification:** `cargo test -p chrys-cli --test refusal` (2 tests, pass); `cargo test -p chrys-core --test refusal` (6 tests, pass)
- **Committed in:** `9c44dc6`

**4. [Rule 1 - Bug, pre-existing test broken by this plan's own change] The recoloured-rectangle unit test's flat fixture could not clear the new refusal gate**
- **Found during:** Task 2, first `cargo test -p chrys-core --lib` run after wiring registration into `compare`
- **Issue:** `a_recoloured_rectangle_returns_one_recoloured_region` (written before this plan, testing classification/bbox logic in isolation) used a flat 8x8 solid-colour frame with only a 2x2 recoloured patch. Scaling the same proportions up to 256x256 gave the identical ratio (`14.99`), proving the failure was structural, not a resolution artifact: a flat frame gives phase correlation almost no edges to lock onto, regardless of size, so it read as unregisterable even though the change itself is small and near-identical.
- **Fix:** Gave the fixture four anchor rectangles (in `frame_with_anchor_rects`) that stay byte-identical between base and candidate, matching the `synthetic_frame`-with-several-rectangles pattern already used in `phase_correlation.rs`, `subpixel.rs` and `register.rs`'s own tests. The recoloured region and its bbox assertions moved to `(96, 96)`, `64x64`, clear of the anchors; measured ratio after the fix is well above `REFUSAL_THRESHOLD`.
- **Files modified:** `crates/chrys-core/src/lib.rs` (test module only)
- **Verification:** `cargo test -p chrys-core --lib tests::a_recoloured_rectangle_returns_one_recoloured_region` passes; `cargo test --workspace` reports 84 tests passing, 0 failing.
- **Committed in:** `9c44dc6`

---

**Total deviations:** 4 auto-fixed (1 Rule 1 metric-design bug found during calibration, 1 Rule 2 missing-link function, 1 Rule 3 infeasible-as-written test placement, 1 Rule 1 pre-existing-test fixture broken by this plan's own change)
**Impact on plan:** No scope creep beyond what CORE-06/CORE-07 and this plan's own must_haves require. All four fixes are corrections within files this plan already owns or a direct, necessary consequence of wiring registration into `compare`. `REFUSAL_THRESHOLD` itself is exactly the midpoint the corpus measured; no fix touched that number to make a case pass.

## Issues Encountered

None beyond the four auto-fixed deviations above, all caught and resolved within this session before the relevant task's commit.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `chrys_core::register::confidence` (`PeakConfidence`, `assess_peak`, `REFUSAL_THRESHOLD`) and `RefusalReason::PeakConfidenceTooLow` are the stable refusal surface plan 01-06 onward builds on; a future block-match or per-region stage inherits the same "refuse before classify" order `compare` now enforces.
- `tests/golden/refuse-01/` is committed and load-bearing: `REFUSAL_THRESHOLD`'s doc comment and the separation test both point back to it, and changing the corpus requires re-running `cargo test -p chrys-core --test refusal` and updating the constant and its comment together, not editing the number in isolation.
- `peak_index` is now part of `chrys_core::register`'s public surface, alongside `phase_correlate`, `assess_peak` and `refine_peak`; a future stage that needs the raw surface index no longer has to re-derive it from a `CoarseOffset` by hand.
- No blockers.

## Self-Check: PASSED

All files below were verified present on disk and all commit hashes verified present in `git log --oneline` on this branch before this line was written:
- `crates/chrys-core/src/register/confidence.rs` — FOUND
- `crates/chrys-core/tests/refusal.rs` — FOUND
- `crates/chrys-cli/tests/refusal.rs` — FOUND
- `tests/golden/refuse-01/should-register/pair-01/base.png`, `tests/golden/refuse-01/should-refuse/pair-01/base.png` — FOUND
- Commits `f26ca15`, `9c44dc6` — FOUND in `git log --oneline`

---
*Phase: 01-raster-engine-and-determinism-proof*
*Completed: 2026-09-06*
