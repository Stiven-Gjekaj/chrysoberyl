---
phase: 02-source-trait-and-a-second-format
plan: 01
subsystem: engine
tags: [rust, cargo-workspace, natord, sequence-comparison, source-adapter]

# Dependency graph
requires:
  - phase: 01-raster-engine-and-determinism-proof
    provides: decode_guarded, normalize_to_rgba8, DecodeLimits, the RasterSource Source impl, the digest contract
provides:
  - "chrys_core::sequence::compare_sequence, the one comparison pipeline, pairing frames by index"
  - "chrys_core::compare, now a thin wrapper over compare_sequence with a slice of one"
  - "RefusalReason::FrameCountMismatch and RefusalReason::EmptySequence"
  - "chrys-source-sequence, the numbered-frame-sequence Source adapter, with SequenceLimits and SequenceError"
  - "the committed tests/golden/sequence-01 fixture: 11 non-padded frames per side, one index recoloured"
  - "chrys compare dispatching a directory argument to the sequence adapter"
affects: [02-02, 05, 07, 08]

# Actuals (#2632)
actuals:
  tokens: 9987
  tasks: 2
  commits: 2

# Tech tracking
tech-stack:
  added: ["natord = \"=1.0.9\" (workspace dependency, used by chrys-source-sequence)"]
  patterns:
    - "One comparison pipeline: compare_sequence is the only implementation; compare delegates to it with std::slice::from_ref on both sides."
    - "A sequence adapter opens no image::ImageReader of its own; every decode routes through the raster crate's one guarded entry point."
    - "Per-index isolation: a sequence-wide refusal (empty, count mismatch) is the only all-or-nothing outcome; any other per-frame failure (DimensionMismatch) is scoped to its own index."

key-files:
  created:
    - crates/chrys-core/src/sequence.rs
    - crates/chrys-source-sequence/Cargo.toml
    - crates/chrys-source-sequence/src/lib.rs
    - crates/chrys-source-sequence/src/sequence.rs
    - crates/chrys-source-sequence/tests/sequence.rs
    - tests/golden/sequence-01/base/frame1.png .. frame11.png
    - tests/golden/sequence-01/candidate/frame1.png .. frame11.png
  modified:
    - Cargo.toml
    - crates/chrys-core/src/lib.rs
    - crates/chrys-core/src/verdict.rs
    - crates/chrys-cli/Cargo.toml
    - crates/chrys-cli/src/main.rs
    - crates/chrys-source-raster/examples/make-fixtures.rs

key-decisions:
  - "compare's empty-vector case (impossible for a one-against-one sequence) is handled with Vec::pop().unwrap_or(EmptySequence-refusal) rather than an unwrap or an index, so an engine that somehow produced no verdict still returns a value instead of panicking."
  - "SequenceSource::with_limits takes only SequenceLimits and keeps DecodeLimits at its default, since no test or use site in this plan needed to vary both at once."
  - "The CLI's exit code over a sequence is the worst verdict across every index (refused above changed above identical), keeping the three-way exit code contract a single-pair caller already relies on."

patterns-established:
  - "A new Source adapter crate depends only on chrys-source and the raster crate's guarded decode/normalize functions, never opening its own image::ImageReader."
  - "A sequence-wide refusal returns Ok(vec![Verdict::Refused { .. }]) with exactly one element, distinguishing it from a per-index Err, which still propagates as an error."

requirements-completed: [SRC-02, SRC-08]

coverage:
  - id: D1
    description: "compare_sequence is the one comparison pipeline: it pairs frames by index, refuses on an empty side or a frame-count mismatch naming both counts, and isolates a per-index failure so other indices still report their own verdict."
    requirement: SRC-02
    verification:
      - kind: unit
        ref: "crates/chrys-core/src/sequence.rs#sequence::tests::empty_sequence_refuses_with_a_single_verdict_naming_no_frame"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/sequence.rs#sequence::tests::unequal_length_sequences_refuse_and_name_both_counts"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/sequence.rs#sequence::tests::one_index_refusing_on_size_leaves_other_indices_reporting_their_own_verdict"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/sequence.rs#sequence::tests::a_zero_pixel_frame_inside_a_non_empty_sequence_returns_compare_error"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/sequence.rs#sequence::tests::compare_and_compare_sequence_agree_over_identical_changed_mismatched_and_empty"
        status: pass
    human_judgment: false
  - id: D2
    description: "A numbered frame sequence pair compares frame by frame through the CLI, over the committed tests/golden/sequence-01 fixture, ordered by natural sort."
    requirement: SRC-08
    verification:
      - kind: unit
        ref: "crates/chrys-source-sequence/src/sequence.rs#sequence::tests::sorts_by_natural_order"
        status: pass
      - kind: integration
        ref: "crates/chrys-source-sequence/tests/sequence.rs#loads_eleven_frames_in_natural_filename_order"
        status: pass
      - kind: integration
        ref: "crates/chrys-source-sequence/tests/sequence.rs#a_frame_count_limit_below_the_fixture_size_refuses_and_names_the_limit"
        status: pass
      - kind: other
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/sequence-01/base tests/golden/sequence-01/candidate"
        status: pass
    human_judgment: false

duration: unmeasured (PLAN_START_TIME not captured at launch; commit span 01:37-01:41 +0200 covers only the two task commits, not the reading, building and testing before them)
completed: 2026-09-07
status: complete
---

# Phase 2 Plan 01: Make the sequence the only unit and compare one directory pair, Summary

**One comparison pipeline (`compare_sequence`) now serves both a single pair and an 11-frame numbered sequence, proven end to end through a new `chrys-source-sequence` adapter and a committed non-padded fixture.**

## Performance

- **Duration:** unmeasured (see frontmatter note)
- **Completed:** 2026-09-07
- **Tasks:** 2 / 2
- **Files modified:** 34 (6 modified, 6 new source/manifest files, 22 new committed PNG fixtures)

## Accomplishments

- Moved phase 1's whole `compare` pipeline, verbatim, into a private `compare_pair` inside a new `chrys_core::sequence` module, and added `compare_sequence(&[Frame], &[Frame]) -> Result<Vec<Verdict>, CompareError>` as the one pipeline both a single pair and a sequence now run through.
- `compare` is now a thin wrapper: `sequence::compare_sequence(std::slice::from_ref(base), std::slice::from_ref(candidate))` with the vector's single element unwrapped (or `EmptySequence` on the impossible empty case). A delegation test asserts the two agree over an identical pair, a changed pair, a dimension-mismatched pair and a zero-pixel frame.
- Added `RefusalReason::FrameCountMismatch { base, candidate }` and `RefusalReason::EmptySequence`, each with a `Display` arm and a unit test asserting the returned vector holds exactly one verdict whose text names the numbers it carries.
- Proved per-index isolation: an equal-length sequence where one index differs in size returns a vector as long as the sequence, with only that index refusing and every other index reporting its own verdict. A zero-pixel frame inside a non-empty sequence still returns `Err(CompareError::EmptyFrame)`, kept distinct from the whole-sequence `EmptySequence` refusal.
- Built the `chrys-source-sequence` adapter crate: it lists a directory's immediate regular files, sorts them by `natord::compare` over the file name (proven by `sequence::tests::sorts_by_natural_order`, the exact test name the phase's validation contract commits to), and decodes each file through `chrys_source_raster::decode::decode_guarded` followed by `normalize_to_rgba8`, opening no `image::ImageReader` of its own. `SequenceLimits` bounds both the frame count (512) and the cumulative pixel count (134,217,728), checked as frames accumulate; proven by an integration test that `max_frames: 3` refuses the 11-frame fixture and names the limit.
- Extended the committed fixture generator with `write_sequence_01`, writing 11 non-padded frames (`frame1.png` .. `frame11.png`, no zero padding) per side to `tests/golden/sequence-01`, with frame 5 recoloured on the candidate side only. An integration test loads the base side and proves, by direct decode, that the frame at index 1 holds `frame2.png`'s own canvas, not only that eleven frames loaded.
- Wired the CLI: `frames_for` dispatches a directory to `SequenceSource` and any other path to `RasterSource`; `run_compare` calls `compare_sequence` once. A one-against-one pair still prints byte-identical stdout to before this plan; a longer sequence prints a `frame {index}` line before each verdict block, and the exit code is the worst outcome across every verdict (refused above changed above identical).

## Task Commits

Each task was committed atomically:

1. **Task 1: Make the sequence the only unit and compare one directory pair** - `51947f6` (feat)
2. **Task 2: Prove the pairing rules and the natural order a sequence needs** - `f6d2b61` (test)

**Plan metadata:** pending (this SUMMARY, STATE.md and ROADMAP.md updates are committed by the orchestrator, per this plan's execution instructions)

## Files Created/Modified

- `Cargo.toml` - added `natord = "=1.0.9"` to the workspace dependency table
- `crates/chrys-core/src/lib.rs` - registered `pub mod sequence;`, re-exported `compare_sequence`, reduced `compare` to a wrapper
- `crates/chrys-core/src/sequence.rs` - the one comparison pipeline: `compare_pair` (moved verbatim), `compare_sequence`, and its own test module
- `crates/chrys-core/src/verdict.rs` - added `RefusalReason::FrameCountMismatch` and `RefusalReason::EmptySequence`, with `Display` arms
- `crates/chrys-source-sequence/Cargo.toml` - new adapter crate manifest
- `crates/chrys-source-sequence/src/lib.rs` - `SequenceLimits`, `SequenceSource`, `SequenceError`, the `Source` impl
- `crates/chrys-source-sequence/src/sequence.rs` - the directory listing, natural sort and per-file decode
- `crates/chrys-source-sequence/tests/sequence.rs` - the fixture integration test and the limits test
- `crates/chrys-source-raster/examples/make-fixtures.rs` - added `write_sequence_01`
- `crates/chrys-cli/Cargo.toml` - added `chrys-source-sequence` as a path dependency
- `crates/chrys-cli/src/main.rs` - `frames_for` replaces `load_first_frame`; `run_compare` calls `compare_sequence` once
- `tests/golden/sequence-01/{base,candidate}/frame1.png` .. `frame11.png` - the committed sequence fixture

## Decisions Made

- `compare`'s impossible empty-vector case is handled with `Vec::pop().unwrap_or(..)` rather than an unwrap or an index, keeping the project's no-panic-on-a-reachable-path rule.
- `SequenceSource::with_limits` takes only `SequenceLimits`; `DecodeLimits` stays at its default, since nothing in this plan needed to vary both together.
- The CLI's exit code over a sequence is the worst verdict across every index, so a caller branching on the exit code keeps the same three meanings it had for a single pair.

## Deviations from Plan

None - plan executed exactly as written. One test-authoring adjustment: an early version of `a_zero_pixel_frame_inside_a_non_empty_sequence_returns_compare_error` used a 64x64 non-empty frame, which is too small for the anchor-rectangle helper's fixed rectangle offsets and caused an out-of-bounds panic; fixed inline by using 128x128, matching the plan's other anchor-rect tests, before the commit. This is a self-caught test-fixture-sizing bug, not a deviation from the plan's design.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Measured Digest (for the next plan)

The verbatim output of
`cargo run -q -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png --hash-only`,
measured after both tasks in this plan, so the next plan compares against a
measured value rather than a memory:

```
decode-base 5a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc
decode-candidate 99bf5b9508b222f45fa5badf610e21cdb464401b26bc30dda345780dc7d2f59f
residual f812da6aa3625e87bba393bb9742f6f24da5ca2a69d10434f364bab3b2fcaea0
verdict 33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823
```

## Next Phase Readiness

- `compare_sequence` is the one comparison pipeline `chrys-core` exposes; plan 02-02 (per this plan's `affects`) can add a static guard asserting `compare` delegates to it, with no second implementation to find.
- The generic pairing code lives in `chrys-core` now, in the wave where a `chrys-core` change is still allowed; waves 3, 4 and 5 (phases 5, 7, 8's later plans) can add a new input family's adapter with no engine change.
- The four phase-1 digests of `tests/golden/pair-01` were verified byte-identical after every task; `chrys-core/Cargo.toml` gained no new dependency; `cargo tree -p chrys-core -e normal` still names no format or GPU crate.

## Self-Check: PASSED

All created files verified present on disk; both task commit hashes (`51947f6`, `f6d2b61`) verified present in `git log --oneline --all`.

---
*Phase: 02-source-trait-and-a-second-format*
*Completed: 2026-09-07*
