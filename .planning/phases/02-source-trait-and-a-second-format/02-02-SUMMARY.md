---
phase: 02-source-trait-and-a-second-format
plan: 02
subsystem: engine
tags: [rust, determinism-guard, digest-fixture, cli, sequence]

# Dependency graph
requires:
  - phase: 02-source-trait-and-a-second-format
    provides: "compare_sequence, the one comparison pipeline; compare as its thin wrapper (02-01)"
provides:
  - "the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it, a sixth static guard in crates/chrys-core/tests/determinism.rs"
  - "tests/golden/pair-01/expected-digest.sha256, the committed four-line phase 1 digest fixture"
  - "the_four_digests_match_the_committed_digest_file, the regression test that reads it"
  - "--hash-only reporting one four-line digest block per frame index, headed by `frame {index}`, for a sequence pair"
  - "the engine tree object id this phase's later waves check against: 63aad81ddee9939047b1436a33eed8f0896da409"
affects: [02-03, 02-04, 02-05]

# Actuals (#2632)
actuals:
  tokens: 3755
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A second-pipeline guard counts named entry points per file rather than banning a single symbol, so a legitimate partial caller (residual.rs, at two of six) stays green while four or more trips it."
    - "A guard proven only by inspection is a belief; this one was proven by planting its own defect in a disposable git worktree, watching it fail by name, then watching it pass again."

key-files:
  created:
    - tests/golden/pair-01/expected-digest.sha256
  modified:
    - crates/chrys-core/tests/determinism.rs
    - crates/chrys-cli/tests/digest.rs
    - crates/chrys-cli/src/main.rs

key-decisions:
  - "The second-pipeline threshold is 4 of the 6 entry points, one whole entry point above residual.rs's measured count of 2 (phase_correlate, block_match), read directly from that file's source before the constant was set."
  - "The single-pair vs sequence branch in --hash-only's new code reuses the exact rule (`verdicts.len() == 1`) the verdict-text branch a few lines below already uses, rather than inventing a second rule for the same distinction."
  - "The printed frame header uses the frame's own `index` field, not the loop position, so the label stays correct even if a future source adapter's frames are not stored in index order."

patterns-established:
  - "A committed digest fixture carries its own `#`-prefixed header comment stating that any edit to it is a behaviour change a commit message must name; the test that reads it skips lines starting with `#`."

requirements-completed: [SRC-02, SRC-08]

coverage:
  - id: D1
    description: "A sixth static guard fails the build if a second copy of the comparison pipeline's call chain appears in the engine outside sequence.rs, or if compare stops delegating to compare_sequence, naming the offending file in its failure message."
    requirement: SRC-08
    verification:
      - kind: unit
        ref: "crates/chrys-core/tests/determinism.rs#the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it"
        status: pass
      - kind: other
        ref: "hand drill in a disposable git worktree: red run naming lib.rs, then green run after reverting (both recorded verbatim below)"
        status: pass
    human_judgment: false
  - id: D2
    description: "All four of phase 1's tests/golden/pair-01 digests are committed and checked on every test run, so a future engine edit that moves one is caught on the machine that made it."
    requirement: SRC-08
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/digest.rs#the_four_digests_match_the_committed_digest_file"
        status: pass
      - kind: other
        ref: "cargo run -q --release -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png --hash-only, output compared byte for byte against the four values in this prompt"
        status: pass
    human_judgment: false
  - id: D3
    description: "--hash-only reports one four-line digest block per frame index for a multi-frame pair, headed by the frame's own index, and the unchanged four lines for a single-frame pair."
    requirement: SRC-02
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/digest.rs#a_sequence_reports_one_digest_block_per_frame_index"
        status: pass
      - kind: integration
        ref: "crates/chrys-cli/tests/digest.rs#a_second_run_over_the_sequence_gives_byte_identical_stdout"
        status: pass
      - kind: other
        ref: "cargo run ... pair-01 --hash-only | wc -l => 4; cargo run ... sequence-01 --hash-only | wc -l => 55"
        status: pass
    human_judgment: false

duration: unmeasured (PLAN_START_TIME not captured at launch; commit span 01:52-02:01 +0200 covers only the three task commits, not the reading, building and testing before them)
completed: 2026-09-07
status: complete
---

# Phase 2 Plan 02: Guard the one pipeline, commit the four digests, and report a sequence's digests, Summary

**A sixth static guard makes "one comparison pipeline" a compile-time fact rather than a habit, all four of phase 1's `tests/golden/pair-01` digests are now committed and checked on every test run, and `--hash-only` reports one four-line block per frame index for a sequence pair.**

## Performance

- **Duration:** unmeasured (see frontmatter note)
- **Completed:** 2026-09-07
- **Tasks:** 3 / 3
- **Files modified:** 4 (3 modified, 1 new fixture)

## Accomplishments

- Added `the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it` to `crates/chrys-core/tests/determinism.rs`, next to the dependency guard. It reads the engine's own stripped source (`strip_comments_and_strings`, `find_substring_occurrences`, `rust_files_under`, all reused from the file's existing helpers, not reinvented) and asserts three facts: `sequence.rs` holds all six pipeline entry points (`phase_correlate`, `peak_index`, `assess_peak`, `block_match`, `suppress_antialiasing`, `label_regions`); no other file under `src/`, outside `register/` and `classify/`, holds four or more of the six; and `lib.rs` holds `compare_sequence` at least once and none of the six. `residual.rs` legitimately calls two of the six (`phase_correlate`, `block_match`, confirmed by direct grep before the threshold constant was set) and stays two whole entry points below the four-of-six threshold.
- Proved the guard by hand in a disposable git worktree created from HEAD (`git worktree add --detach`), with the drill's own uncommitted guard copy carried into it: planting the six-line call chain as a new function in a copy of `lib.rs` made the guard fail and name `lib.rs` by path; reverting that file with `git checkout --` made the guard pass again. Both runs are recorded verbatim below. The worktree was removed on completion; the real working tree was never touched by the plant.
- Created `tests/golden/pair-01/expected-digest.sha256`, holding all four of phase 1's digest lines (the two decode digests already committed, plus `residual` and `verdict`, the two that carry the comparison's own arithmetic), typed from the plan rather than generated from the binary under test. Added `the_four_digests_match_the_committed_digest_file` to `crates/chrys-cli/tests/digest.rs`, which runs the existing `run_hash_only()` helper and compares its output against the committed file line by line, naming which of the four lines moved on a mismatch. `tests/golden/pair-01/expected-decode.sha256` and its own two-line test are untouched.
- Extended the `--hash-only` branch of `run_compare` in `crates/chrys-cli/src/main.rs`: a one-against-one pair still prints exactly the four lines it always has (`verdicts.len() == 1`, the same rule the verdict-text branch below it already uses), byte-identical to before this plan. A pair holding more than one frame now prints a `frame {index}` line before each four-line block, with `{index}` read from that frame's own `index` field rather than the loop position. `digest_report` itself is unchanged. Added two tests: `a_sequence_reports_one_digest_block_per_frame_index`, asserting 55 lines (11 headers, 44 digests) over the committed `tests/golden/sequence-01` fixture, every digest 64 lowercase hex characters, and header indices running 0 through 10 in order; and `a_second_run_over_the_sequence_gives_byte_identical_stdout`, the sequence counterpart of the existing single-pair repeat test.

## Task Commits

Each task was committed atomically:

1. **Task 1: Guard that the engine keeps one comparison pipeline** - `7aea562`
2. **Task 2: Commit the four phase 1 digests as a regression fixture** - `ddc41c4`
3. **Task 3: Report one digest block per frame index** - `31ed55f`

**Plan metadata:** pending (this SUMMARY and STATE.md/ROADMAP.md updates are committed by the orchestrator, per this plan's execution instructions)

## Files Created/Modified

- `crates/chrys-core/tests/determinism.rs` - added the sixth guard, `the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it`, plus its supporting constants
- `tests/golden/pair-01/expected-digest.sha256` - new: the committed four-line phase 1 digest fixture, with a `#`-prefixed header comment
- `crates/chrys-cli/tests/digest.rs` - added `the_four_digests_match_the_committed_digest_file`, `a_sequence_reports_one_digest_block_per_frame_index`, and `a_second_run_over_the_sequence_gives_byte_identical_stdout`, plus the `run_hash_only_sequence()` helper
- `crates/chrys-cli/src/main.rs` - `--hash-only` now prints one four-line block per frame index for a multi-frame pair, headed by `frame {index}`

## Decisions Made

- The second-pipeline threshold is 4 of 6 entry points: one whole entry point of margin above `residual.rs`'s measured count of 2, so a real second pipeline is caught without flagging existing, correct code.
- `--hash-only`'s single-vs-sequence branch reuses `verdicts.len() == 1`, the exact rule the unchanged verdict-text branch already applies a few lines below, rather than introducing a second way to draw the same line.
- The printed frame header reads the frame's own `index` field, not the loop position, so a future source whose frames arrive out of index order still prints a correct label.

## Deviations from Plan

None - plan executed exactly as written. One formatting fixup: `cargo fmt` reflowed two `assert_eq!` calls and one `let` binding that this plan's own hand-typed code exceeded the configured line width on; applied via `cargo fmt` before each task's verification and commit, with no behaviour change.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## The Hand Drill, Recorded Verbatim

Performed inside a disposable git worktree created from HEAD (`git worktree add --detach --quiet <tmp> HEAD`), with this plan's own uncommitted `determinism.rs` (holding the new guard) copied into it before the plant, since the guard did not yet exist in a committed HEAD to check out. The real working tree was never edited; the worktree was removed with `git worktree remove --force` on completion.

**Baseline, before the plant** (`cargo test -p chrys-core --test determinism the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it -- --exact`):

```
running 1 test
test the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.01s
```

**The plant:** appended a new function to the worktree's own copy of `crates/chrys-core/src/lib.rs`, calling all six pipeline entry points in the crate root:

```rust
#[allow(dead_code)]
fn drill_planted_second_pipeline(base: &Frame, candidate: &Frame) -> Result<Verdict, CompareError> {
    let (refined, surface) = register::phase_correlate(base, candidate)?;
    let peak_index = register::peak_index(refined.whole, surface.resolution);
    let confidence = register::assess_peak(&surface, peak_index);
    let mut field = register::block_match(base, candidate, refined.whole)?;
    classify::suppress_antialiasing(&mut field, base, candidate);
    let labelled_regions = classify::label_regions(&field);
    let _ = confidence;
    let _ = labelled_regions;
    Ok(Verdict::Identical)
}
```

**Red run** (same command, against the planted file):

```
running 1 test
test the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it ... FAILED

failures:

---- the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it stdout ----

thread 'the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it' (1333881) panicked at crates/chrys-core/tests/determinism.rs:628:5:
a second copy of the comparison pipeline's call chain has grown outside sequence.rs, the file that owns it. A file at or above 4 of the six entry points is treated as a second pipeline; `residual.rs` legitimately holds two and must stay below this line. Offending file(s):
/private/var/folders/3m/cvzr3lwj0gq1n6_9cljynby80000gn/T/tmp.unpFR5sIQ8/crates/chrys-core/src/lib.rs: phase_correlate(, peak_index(, assess_peak(, block_match(, suppress_antialiasing(, label_regions(
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s
```

The message names `lib.rs` and lists every entry point it holds, as the plan's acceptance criteria require.

**Restore:** `git -C <tmp> checkout -- crates/chrys-core/src/lib.rs`

**Green run** (same command, after the restore):

```
running 1 test
test the_engine_has_one_comparison_pipeline_and_compare_delegates_to_it ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.01s
```

## The Two Entry Points `residual.rs` Uses

Confirmed by direct grep of the tree before the threshold constant was set: `residual_rgba8` calls `register::phase_correlate` (to find the global shift) and `register::block_match` (to build the residual buffer's own local-shift-corrected samples). It calls none of the other four (`peak_index`, `assess_peak`, `suppress_antialiasing`, `label_regions`), because it renders a residual buffer for the digest report, not a verdict, so it never reaches confidence assessment or classification.

## The Sequence Digest Report, Verbatim Line Count

```
cargo run -q --release -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png --hash-only | wc -l
4
cargo run -q --release -p chrys-cli -- compare tests/golden/sequence-01/base tests/golden/sequence-01/candidate --hash-only | wc -l
55
```

55 is 11 `frame {index}` headers plus 44 digest lines, headers running 0 through 10 in order; frame 4 (the index plan 02-01 recoloured on the candidate side) is the only index whose decode, residual and verdict digests differ from the other ten.

## Full Verification, This Plan's End State

```
cargo test -p chrys-core --test determinism: 6 passed.
cargo test -p chrys-cli --test digest: 6 passed.
cargo test --workspace: 149 passed, 0 failed (0+6+2+59+14+21+6+6+11+3+13+5+1+2, summed across every test target; phase 1's own baseline was 145, plus this plan's four new tests).
cargo clippy --workspace --all-targets -- -D warnings: clean.
cargo fmt --check: clean.
```

## engine tree: 63aad81ddee9939047b1436a33eed8f0896da409

That id is `git rev-parse HEAD:crates/chrys-core`, read after this plan's last commit (`31ed55f`), which touched only `crates/chrys-cli` and `tests/golden/`. It is identical to the tree id measured immediately after Task 1's commit (`7aea562`, the last commit in this plan that touched a file under `crates/chrys-core/`), confirmed by `git rev-parse 7aea562:crates/chrys-core` returning the same value. This is the last plan in this phase permitted to change a file under `crates/chrys-core/`; from here on, "no engine change" for waves 3, 4 and 5 means this tree id does not move.

## Next Phase Readiness

- The engine's one-pipeline claim is now a guarded, drill-proven fact: a second copy of the call chain, or a `compare` that stops delegating, is a red test that names the offending file.
- All four of phase 1's `tests/golden/pair-01` digests are committed and checked on every `cargo test -p chrys-cli --test digest` run.
- `--hash-only` now reports a sequence the same shape the six-runner matrix needs (plan 02-04): one `frame {index}` header plus a four-line digest block per index.
- The engine tree object id above (`63aad81ddee9939047b1436a33eed8f0896da409`) is the anchor plans 02-03, 02-04 and 02-05 (waves 3, 4, 5) check against; none of them may move it.

## Self-Check: PASSED

All created/modified files verified present on disk (`tests/golden/pair-01/expected-digest.sha256`, `crates/chrys-core/tests/determinism.rs`, `crates/chrys-cli/tests/digest.rs`, `crates/chrys-cli/src/main.rs`); all three task commit hashes (`7aea562`, `ddc41c4`, `31ed55f`) verified present in `git log --oneline`.

---
*Phase: 02-source-trait-and-a-second-format*
*Completed: 2026-09-07*
