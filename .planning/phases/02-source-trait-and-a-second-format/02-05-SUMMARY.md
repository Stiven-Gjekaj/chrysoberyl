---
phase: 02-source-trait-and-a-second-format
plan: 05
subsystem: source-adapter
tags: [rust, toml, serde, cli, region-hint, guard-test]

# Dependency graph
requires:
  - phase: 02-source-trait-and-a-second-format
    provides: "the engine tree object id 63aad81ddee9939047b1436a33eed8f0896da409, the anchor waves 3, 4 and 5 must not move (02-02)"
provides:
  - "Frame::crop_to_region and RegionOutOfBounds in chrys-source, a pure data transformation with checked-arithmetic bounds"
  - "chrys_source_raster::hints::read_hints_sidecar, the <stem>.hints.toml reader, called from both RasterSource::load and the sequence adapter"
  - "--region <NAME> on chrys compare, cropping both sides to a named hint before the unmodified compare_sequence runs"
  - "tests/golden/hint-01, a committed fixture whose --region logo run and whole-frame run disagree"
  - "chrys_source_declares_no_dependency, a manifest line-scan guard proven able to go red on a planted dependency"
affects: []

# Actuals (#2632)
actuals:
  tokens: 8620
  tasks: 3
  commits: 3
  plan_head_before: 963e25331f82c9ff3842e01e9fa672ecfaaed245

# Tech tracking
tech-stack:
  added:
    - "toml =1.1.5 (workspace dependency, used only by chrys-source-raster)"
    - "serde =1.0.229 with the derive feature (workspace dependency, used only by chrys-source-raster)"
  patterns:
    - "Crop, then compare: the engine never learns about a hint. A cropped Frame is indistinguishable from any other Frame to compare_sequence."
    - "The sidecar's own deserialize structs (HintsDocument, HintRow) live next to the parser, private to chrys-source-raster, and convert into chrys_source::RegionHint by a hand-written From impl, so the derive that needs serde never touches the type chrys-core depends on."
    - "A test whose full path must equal a plan's --exact filter argument lives at the crate root, not inside a nested mod tests block, because cargo test --exact matches the whole qualified path."

key-files:
  created:
    - crates/chrys-source-raster/src/hints.rs
    - crates/chrys-cli/tests/region_hint.rs
    - crates/chrys-source/tests/manifest.rs
    - tests/golden/hint-01/base.png
    - tests/golden/hint-01/base.hints.toml
    - tests/golden/hint-01/candidate.png
    - tests/golden/hint-01/candidate.hints.toml
  modified:
    - Cargo.toml
    - crates/chrys-source/src/lib.rs
    - crates/chrys-source-raster/Cargo.toml
    - crates/chrys-source-raster/src/lib.rs
    - crates/chrys-source-raster/examples/make-fixtures.rs
    - crates/chrys-source-sequence/src/lib.rs
    - crates/chrys-source-sequence/src/sequence.rs
    - crates/chrys-cli/src/main.rs

key-decisions:
  - "toml and serde are added to chrys-source-raster's own Cargo.toml, never to chrys-source: RegionHint stays a plain struct with no derive, and the sidecar's own private structs (HintsDocument, HintRow) carry the derive instead, converted by hand. Confirmed with cargo tree -p chrys-core -e normal, which names neither crate."
  - "deny_unknown_fields and the duplicate-region-name refusal (Task 2's own hardening, per the plan's task split) were written directly into hints.rs during Task 1's commit, since the parser could not have been correctly designed without them (a misspelled key silently cropping the wrong rectangle is a Rule 2 correctness gap, not an optional extra). Task 2 added the tests that prove both, rather than the behaviour itself. Documented here since the plan describes them as two separate tasks; the code was correct from Task 1 onward."
  - "The crop_to_region and hint out-of-bounds tests needed to be a top-level fn (not inside mod tests) for Task 1's plan-mandated cargo test -p chrys-source crop_to_region -- --exact to pass: cargo's --exact matches the whole qualified test path, and a test inside mod tests is named tests::crop_to_region, not crop_to_region. This is documented as a deviation below."
  - "The hint-01 fixture uses a colour change (not a shift) outside the named region, so no translation the block-matcher's own ±24px local search window can find would hide the difference. A pure translation was tried mentally first and rejected: BLOCK_SIDE=32 with a 24px reachable local offset can absorb a small shift per-block, which would have made the whole-frame run agree with the region run and proven nothing."
  - "The sequence adapter's per-frame hint-read failure gets its own SequenceError::Hint variant, distinct from the existing Decode variant, so a sidecar failure reads 'cannot read hints for <file>' rather than the confusing 'cannot decode <file>: cannot parse <sidecar path>' a reused Decode variant would have produced."

patterns-established:
  - "A guard whose silence is evidence gets a drill: chrys_source_declares_no_dependency was proven able to go red (planting serde = \"1\" into a disposable worktree's copy of chrys-source/Cargo.toml, naming the exact line) before being trusted, following the same discipline plans 02-02 and 02-04 established for their own guards."

requirements-completed: [SRC-09]

coverage:
  - id: D1
    description: "Frame::crop_to_region validates a hint's rectangle with checked arithmetic and returns a frame of exactly the hint's own pixels, keeping the source frame's index and carrying no hints of its own."
    requirement: SRC-09
    verification:
      - kind: unit
        ref: "crates/chrys-source/src/lib.rs#crop_to_region"
        status: pass
      - kind: unit
        ref: "crates/chrys-source/src/lib.rs#tests::a_hint_wider_than_the_frame_is_refused"
        status: pass
      - kind: unit
        ref: "crates/chrys-source/src/lib.rs#tests::a_hint_whose_x_plus_width_overflows_a_u32_is_refused_rather_than_wrapping"
        status: pass
    human_judgment: false
  - id: D2
    description: "read_hints_sidecar reads <stem>.hints.toml, returns an empty vector when the file is missing, and is called from both RasterSource::load and the sequence adapter, one function for both."
    requirement: SRC-09
    verification:
      - kind: unit
        ref: "crates/chrys-source-raster/src/hints.rs#tests::a_missing_sidecar_returns_an_empty_vector"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-raster/src/hints.rs#tests::a_present_sidecar_parses_into_region_hints"
        status: pass
    human_judgment: false
  - id: D3
    description: "A malformed sidecar (invalid TOML, an unknown key, a negative coordinate, a non-integer coordinate) fails loudly with a message naming the line and, where applicable, the field; a duplicate region name is refused."
    requirement: SRC-09
    verification:
      - kind: unit
        ref: "crates/chrys-source-raster/src/hints.rs#tests::invalid_toml_fails_with_a_message_naming_the_line"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-raster/src/hints.rs#tests::an_unknown_key_fails_rather_than_being_ignored"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-raster/src/hints.rs#tests::a_negative_coordinate_fails_with_a_message_naming_the_field"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-raster/src/hints.rs#tests::a_non_integer_coordinate_fails_with_a_message_naming_the_field"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-raster/src/hints.rs#tests::a_sidecar_naming_the_same_region_twice_is_refused"
        status: pass
    human_judgment: false
  - id: D4
    description: "chrys compare --region logo over tests/golden/hint-01 exits 0 and prints the identical verdict; the same pair without --region does not exit 0. An unknown region name exits non-zero and stderr lists the declared names."
    requirement: SRC-09
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/region_hint.rs#region_hint"
        status: pass
      - kind: integration
        ref: "crates/chrys-cli/tests/region_hint.rs#an_unknown_region_name_exits_non_zero_and_names_the_declared_regions"
        status: pass
    human_judgment: false
  - id: D5
    description: "chrys_source_declares_no_dependency reads chrys-source/Cargo.toml with a hand line scan (no TOML crate) and fails naming the offending line; proven able to go red on a planted dependency in a disposable worktree before being trusted."
    requirement: SRC-09
    verification:
      - kind: unit
        ref: "crates/chrys-source/tests/manifest.rs#chrys_source_declares_no_dependency"
        status: pass
      - kind: other
        ref: "disposable-worktree drill: planted serde = \"1\" into Cargo.toml, test failed naming 'line 12: serde = \"1\"'; reverted, test passed again (recorded verbatim below)"
        status: pass
    human_judgment: false
  - id: D6
    description: "No file under crates/chrys-core/ changed anywhere in this plan's commit range, and cargo tree -p chrys-core -e normal names neither serde nor toml."
    requirement: SRC-09
    verification:
      - kind: other
        ref: "git diff --name-only 6633ece^..7ad4829 -- crates/chrys-core/ (empty output); cargo tree -p chrys-core -e normal (no serde, no toml, no format crate)"
        status: pass
    human_judgment: false

duration: unmeasured (PLAN_START_TIME not captured at launch; commit span 03:12:14-03:19:20 +0200 covers only the three task commits, not the reading, building and testing before and between them)
completed: 2026-09-07
status: complete
---

# Phase 2 Plan 05: Crop a frame to a named hint and compare inside it, Summary

**A producer names a region in a `<stem>.hints.toml` sidecar; `Frame::crop_to_region` crops both sides to that rectangle before the unmodified `compare_sequence` runs, and `chrys compare --region logo` over the committed `tests/golden/hint-01` fixture reaches a different verdict than the same pair compared whole-frame, with the engine's own tree object id and dependency graph unmoved.**

## Performance

- **Duration:** unmeasured (see frontmatter note; commit span was about 7 minutes)
- **Completed:** 2026-09-07
- **Tasks:** 3 / 3
- **Files modified:** 15 (8 modified, 7 new: 3 source files, 4 fixture files)

## Accomplishments

- Added `Frame::crop_to_region(&self, hint: &RegionHint) -> Result<Frame, RegionOutOfBounds>` to `crates/chrys-source/src/lib.rs`. It validates the hint's rectangle with checked `u32` arithmetic (so a hint near `u32::MAX` cannot wrap into a rectangle that appears to fit), refuses a zero-width or zero-height hint, copies the named rows into a new RGBA8 buffer with no padding, keeps the source frame's own `index`, and returns a frame with an empty `hints` list. `RegionOutOfBounds` carries the hint's name, its rectangle, and the frame's own size, with `Display` and `std::error::Error` implemented by hand. `chrys-source/Cargo.toml` still declares no dependency.
- Built `crates/chrys-source-raster/src/hints.rs`: `read_hints_sidecar(image_path: &Path) -> Result<Vec<RegionHint>, RasterError>` looks for `<stem>.hints.toml` next to the image, returns an empty vector when it is missing, and parses a present file with `toml` + `serde` into private `HintsDocument`/`HintRow` structs (both `deny_unknown_fields`), converted into `chrys_source::RegionHint` by a hand-written `From` impl — `RegionHint` itself derives nothing. A duplicate region name is refused. `toml =1.1.5` and `serde =1.0.229` (`derive` feature) were added to the workspace `Cargo.toml` and to `chrys-source-raster/Cargo.toml` only. `RasterSource::load` and `crates/chrys-source-sequence/src/sequence.rs`'s `decode_all` both call it — one function for both adapters, with a new `SequenceError::Hint` variant naming the frame file on failure.
- Extended `crates/chrys-source-raster/examples/make-fixtures.rs` with `write_hint_01()`, which writes `tests/golden/hint-01/{base,candidate}.png` and their matching `.hints.toml` sidecars, both naming a `logo` region at `(64, 64)`, 128x128. The region holds four rectangles of different colours and sizes (`HINT_REGION_RECTS`), byte-identical between the two files. Outside the region, two rectangles are recoloured (not shifted) between the two files, a difference no translation the engine's block match (`BLOCK_SIDE`=32, largest reachable local offset 24px) could hide.
- Added `--region <NAME>` to `chrys compare`. When present, `crop_frames_to_region` finds, per frame index, a hint of that name in `frame.hints` on each side, crops through `crop_to_region`, and hands the cropped frames to the unmodified `compare_sequence`. A side missing the named hint fails with a message naming the path, the frame index, and the region names that side does declare.
- Added five out-of-bounds tests to `chrys-source/src/lib.rs`'s `mod tests` (wider than the frame, taller than it, origin outside it, `x + width` overflow, zero width), each asserting on the error's fields and on its `Display` message naming the rectangle and the frame size.
- Added six hardening tests to `hints.rs`: a present/missing sidecar (Task 1), and invalid TOML, an unknown key, a negative coordinate, a non-integer coordinate, and a duplicate region name (Task 2). Each builds its own temporary file; no broken sidecar is committed.
- Wrote `crates/chrys-cli/tests/region_hint.rs`: `region_hint` runs the built binary twice over `tests/golden/hint-01`, asserting the `--region logo` run exits 0 and prints `identical\n`, and the same pair without it does not exit 0 and does not print that line. A second test, `an_unknown_region_name_exits_non_zero_and_names_the_declared_regions`, asserts a mistyped region exits non-zero and stderr names `logo`.
- Wrote `crates/chrys-source/tests/manifest.rs`: `chrys_source_declares_no_dependency` reads `chrys-source/Cargo.toml` as text and, by a hand line scan (no TOML crate), asserts every line from the `[dependencies]` header to the next section header is blank or a comment. Proved it can go red in a disposable git worktree before trusting it (recorded verbatim below).
- `git rev-parse HEAD:crates/chrys-core` read `63aad81ddee9939047b1436a33eed8f0896da409` after every commit in this plan, and `git diff --name-only 6633ece^..7ad4829 -- crates/chrys-core/` printed nothing across the whole plan.
- `cargo build --workspace`, `cargo test --workspace` (175 tests, 0 failed), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --check` all pass.

## Task Commits

Each task was committed atomically:

1. **Task 1: Crop a frame to a named hint and compare inside it** - `6633ece`
2. **Task 2: Refuse a hint that does not fit the frame** - `d8bd229`
3. **Task 3: Prove the engine registers inside the named region** - `7ad4829`

**Plan metadata:** pending (this SUMMARY and STATE.md/ROADMAP.md updates are committed by the orchestrator, per this plan's execution instructions)

## Files Created/Modified

- `crates/chrys-source/src/lib.rs` - `Frame::crop_to_region`, `RegionOutOfBounds`, and their tests (five out-of-bounds cases, one positive `crop_to_region` case at the crate root)
- `crates/chrys-source-raster/src/hints.rs` - new: `read_hints_sidecar`, private `HintsDocument`/`HintRow`, and eight tests
- `crates/chrys-source-raster/src/lib.rs` - `RasterSource::load` now calls `read_hints_sidecar`; two new `RasterError` variants (`MalformedHints`, `DuplicateHintName`)
- `crates/chrys-source-raster/examples/make-fixtures.rs` - `write_hint_01()`, the `hint-01` fixture generator
- `crates/chrys-source-raster/Cargo.toml` - added `toml` and `serde`
- `crates/chrys-source-sequence/src/lib.rs` - new `SequenceError::Hint` variant
- `crates/chrys-source-sequence/src/sequence.rs` - `decode_all` now calls `read_hints_sidecar` per frame
- `crates/chrys-cli/src/main.rs` - `--region <NAME>` flag, `crop_frames_to_region`
- `crates/chrys-cli/tests/region_hint.rs` - new: `region_hint`, `an_unknown_region_name_exits_non_zero_and_names_the_declared_regions`
- `crates/chrys-source/tests/manifest.rs` - new: `chrys_source_declares_no_dependency`
- `Cargo.toml` - added `toml =1.1.5` and `serde =1.0.229` (`derive`) as workspace dependencies
- `tests/golden/hint-01/{base,candidate}.png`, `{base,candidate}.hints.toml` - new committed fixture

## Decisions Made

- `toml`/`serde` live only in `chrys-source-raster`'s manifest, never in `chrys-source`'s; `RegionHint` derives nothing, and the sidecar's own private structs carry the derive instead, converted by hand. Confirmed with `cargo tree -p chrys-core -e normal`.
- `deny_unknown_fields` and the duplicate-name refusal were implemented in Task 1's commit rather than Task 2's, since a sidecar parser without them would silently crop the wrong rectangle on a typo — a Rule 2 correctness gap the parser could not ship without. Task 2 added the tests that prove both.
- `crop_to_region` and its positive test live at the crate root, not inside `mod tests`, because Task 1's plan-mandated verify command (`cargo test -p chrys-source crop_to_region -- --exact`) matches the whole qualified test path, and a test inside a module is named `tests::crop_to_region`.
- The `hint-01` fixture's outside-region difference is a colour change, not a shift, because the block matcher's own ±24px local search window can absorb a small per-block translation — a shift alone would risk both runs reporting `identical` and proving nothing.
- The sequence adapter's own hint-read failure gets `SequenceError::Hint`, not the existing `Decode` variant, so its message reads "cannot read hints for `<file>`" rather than nesting a sidecar path inside a "cannot decode" message meant for image bytes.

## Deviations from Plan

**1. [Rule 3 - Blocking] `crop_to_region`'s positive test moved to the crate root**
- **Found during:** Task 1, verifying `cargo test -p chrys-source crop_to_region -- --exact`
- **Issue:** The test was first written inside `mod tests`, giving it the qualified name `tests::crop_to_region`. `cargo test <filter> -- --exact` matches the whole qualified path, so the plan's own verify command reported `0 passed` against a test that existed and worked correctly under a substring filter.
- **Fix:** Moved the test (and its `patterned_frame` helper) to the crate root, guarded by `#[cfg(test)]`, so its full path is the bare `crop_to_region` the `--exact` filter expects.
- **Files modified:** `crates/chrys-source/src/lib.rs`
- **Verification:** `cargo test -p chrys-source crop_to_region -- --exact` now reports `1 passed`.
- **Committed in:** `6633ece` (Task 1 commit)

**2. [Observation, not a code change] The plan's `sed`-based engine-tree check does not match 02-02-SUMMARY.md's actual heading**
- **Found during:** verifying the `test "$(git rev-parse HEAD:crates/chrys-core)" = "$(sed -n 's/^engine tree: //p' ...)"` command every task's `<verify>` block specifies.
- **Issue:** `02-02-SUMMARY.md` records the hash under the Markdown heading `## engine tree: 63aad81...`, not a bare `engine tree: 63aad81...` line. The plan's `sed` pattern `s/^engine tree: //p` requires the line to start with `engine tree: ` with no `## ` prefix, so it matches nothing and the right-hand side of the `test` comparison is always empty.
- **Resolution:** Not fixed, since fixing it would mean editing a previous plan's committed SUMMARY.md, which this plan does not own. Verified the actual invariant directly instead: `git rev-parse HEAD:crates/chrys-core` was read after every commit in this plan and compared by eye against the hash `02-02-SUMMARY.md` states in its own prose (`63aad81ddee9939047b1436a33eed8f0896da409`); all four readings (baseline plus after each of the three commits) matched exactly.
- **Files modified:** none.
- **Committed in:** not applicable (documentation-only observation).

---

**Total deviations:** 1 auto-fixed (blocking), 1 observed and worked around without a code change.
**Impact on plan:** No scope creep. Both are mechanical: one test-naming fix required by the plan's own verify command, and one workaround for a pre-existing formatting mismatch in a prior plan's summary that this plan does not have authority to edit.

## Issues Encountered

None beyond the two items recorded under Deviations above.

## User Setup Required

None - no external service configuration required.

## The Manifest Guard Drill, Recorded Verbatim

Performed inside a disposable git worktree created from HEAD (`git worktree add --detach --quiet <tmp> HEAD`), with this plan's own uncommitted `crates/chrys-source/tests/manifest.rs` copied into it before the plant, since the guard did not yet exist in a committed HEAD to check out. The real working tree was never edited; the worktree was removed with `git worktree remove --force` on completion.

**Baseline, before the plant** (`cargo test -p chrys-source --test manifest -- --exact chrys_source_declares_no_dependency`):

```
running 1 test
test chrys_source_declares_no_dependency ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**The plant:** appended `serde = "1"` to the worktree's own copy of `crates/chrys-source/Cargo.toml`'s `[dependencies]` section.

**Red run** (same command, against the planted file):

```
running 1 test
test chrys_source_declares_no_dependency ... FAILED

failures:

---- chrys_source_declares_no_dependency stdout ----

thread 'chrys_source_declares_no_dependency' (1465492) panicked at crates/chrys-source/tests/manifest.rs:51:5:
/private/var/folders/.../tmp.CQCvP2MUEm/crates/chrys-source/../../crates/chrys-source/Cargo.toml declares a dependency, but chrys-source is a dependency of chrys-core and must depend on nothing outside the standard library. Offending line(s): line 12: serde = "1"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    chrys_source_declares_no_dependency

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The message names the exact planted line (`line 12: serde = "1"`), as the plan's acceptance criteria require.

**Restore:** `git checkout -- crates/chrys-source/Cargo.toml` (run inside the drill worktree)

**Green run** (same command, after the restore):

```
running 1 test
test chrys_source_declares_no_dependency ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

After this drill: `git status --porcelain` in the real worktree printed only the plan's own pending files, and `git worktree list` showed only this agent's own worktree and the primary checkout — no orphaned drill worktree remained.

## The Two Runs Over `tests/golden/hint-01`, Recorded Verbatim

`cargo run -q -p chrys-cli -- compare tests/golden/hint-01/base.png tests/golden/hint-01/candidate.png --region logo`:

```
identical
```

Exit code: `0`.

`cargo run -q -p chrys-cli -- compare tests/golden/hint-01/base.png tests/golden/hint-01/candidate.png`:

```
Recoloured region at x=16, y=16, width=40, height=30, colour delta 111.83 (base [40, 90, 200, 255], candidate [200, 90, 40, 255])
Recoloured region at x=196, y=196, width=40, height=30, colour delta 111.83 (base [40, 90, 200, 255], candidate [200, 90, 40, 255])
```

Exit code: `1`.

The two runs over the same pair disagree, which is the observable meaning of SRC-09's "registers inside that region instead of searching for it": the hinted run reports `identical` because the engine only ever sees the byte-identical 128x128 crop; the whole-frame run sees the two recoloured rectangles the hint let the first run ignore.

## Engine Boundary, This Plan's Own Commit Range

`git diff --name-only 6633ece^..7ad4829 -- crates/chrys-core/`:

```
(empty)
```

`git rev-parse HEAD:crates/chrys-core` after this plan's last commit (`7ad4829`): `63aad81ddee9939047b1436a33eed8f0896da409`, identical to the id `02-02-SUMMARY.md` recorded and to the id read after each of this plan's own three commits.

## Full Verification, This Plan's End State

```
cargo build --workspace: clean.
cargo test -p chrys-source: 9 passed.
cargo test -p chrys-source --test manifest: 1 passed.
cargo test -p chrys-source-raster hints: 7 passed.
cargo test -p chrys-cli --test region_hint: 2 passed.
cargo test -p chrys-core --test determinism: 6 passed.
cargo test --workspace: 175 passed, 0 failed.
cargo clippy --workspace --all-targets -- -D warnings: clean.
cargo fmt --check: clean.
cargo tree -p chrys-core -e normal: no serde, no toml, no format or GPU crate.
```

## Next Phase Readiness

- SRC-09 is delivered and observable: a producer names a region in a sidecar, and the tool compares inside it, with a fixture proving the two answers (hinted vs. whole-frame) genuinely differ.
- This is the last plan in phase 2 permitted to check the engine tree id; `63aad81ddee9939047b1436a33eed8f0896da409` held across all five waves of this phase, and `chrys-core`'s dependency graph carries no `serde`, `toml`, or format crate anywhere in this phase's history.
- All four of this phase's `must_haves.truths` are met: a named region reaches the comparison, the region and whole-frame answers differ, an out-of-bounds hint and a malformed sidecar are both refused with an actionable message, and no file under `crates/chrys-core/` changed while `chrys-source` still depends on nothing outside the standard library.
- Phase 2 (source trait and a second format) is now feature-complete across all five plans; the next phase can build against `Source`, `compare_sequence`, and the hint channel exactly as they stand today, with no further change expected to any of the three.

## Self-Check: PASSED

All created files verified present on disk (`crates/chrys-source-raster/src/hints.rs`, `crates/chrys-cli/tests/region_hint.rs`, `crates/chrys-source/tests/manifest.rs`, `tests/golden/hint-01/base.png`, `tests/golden/hint-01/base.hints.toml`, `tests/golden/hint-01/candidate.png`, `tests/golden/hint-01/candidate.hints.toml`); all three task commit hashes (`6633ece`, `d8bd229`, `7ad4829`) verified present in `git log --oneline`; `git rev-parse HEAD:crates/chrys-core` verified reading `63aad81ddee9939047b1436a33eed8f0896da409` after the final commit.

---
*Phase: 02-source-trait-and-a-second-format*
*Completed: 2026-09-07*
