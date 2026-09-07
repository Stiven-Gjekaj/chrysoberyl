---
phase: 03-baseline-rules-and-ci-gate
plan: 03
subsystem: cli-report
tags: [rust, toml, ci-artifact, static-guard]

# Dependency graph
requires:
  - phase: 03-baseline-rules-and-ci-gate
    provides: "plan 03-01's chrys_rule::RuleOutcome/RegionOutcome and evaluate(verdict, hints, rules, frame_size), and plan 03-02's mask-scoped rules, all already wired into crates/chrys-cli/src/main.rs before this plan started"
provides:
  - "chrys_source::Source::load_named, a defaulted trait method (default body uses std::path only, no new dependency) pairing every frame with the name of the file it came from; SequenceSource overrides it so a directory is listed once and decoded once, with load itself now implemented in terms of load_named"
  - "chrys compare --report <PATH>: a TOML artifact naming the base and candidate paths, the rule file when one was given, and, per frame, its own index, its base-side source file name (not only its index), its verdict, and, for a changed frame, one [[frame.change]] table per region carrying kind, bounding box, size, its named region when one applies, a moved change's offset, a recoloured change's colour delta (including the alpha term read from the two colours' own fourth bytes), and, only when --rule was given, that change's own rule_outcome"
  - "The report is written before any exit-code-bearing branch of run_compare returns, so a non-zero exit and a refused pair both still produce it; a refused frame carries reason and no change table; a write that fails fails the run and names the path"
  - "crates/chrys-cli/tests/decode_limits_guard.rs: a static guard walking every .rs file under every crate's own src directory, stripping comments and strings, and asserting the only two files that call image::ImageReader::open( directly are the two this project already knows about, with their exact call-site counts, failing when a count moves either way or an unlisted file calls it"
affects: []

# Actuals (#2632)
actuals:
  tokens: 12571
  tasks: 3
  commits: 3
  plan_head_before: e836653dddf781cb17c0a2db6925e5d347f28275

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A frame's own file name rides a defaulted trait method (Source::load_named), not a new Frame field, so the 45 Frame { .. } literals under crates/chrys-core/ stay untouched and the engine never learns a frame has a name. The same pattern the plan itself names as the alternative it rejected."
    - "The report's rule_outcome key is omitted entirely (serde skip_serializing_if) rather than given a default value when --rule was not given, so an absent key can only mean 'nothing asked this question', never 'something asked and the answer was no'."
    - "A CI artifact a caller asked for is written before any early-return branch of the function that would otherwise skip it, and a failure to write it propagates as a hard error naming the path, rather than being dropped silently. Task 1's own restructuring of run_compare (computing rule outcomes and writing the report before the hash_only/print branches) already delivered this by the time Task 2 started; Task 2 added the five tests that prove it, and no further production code changed."
    - "A guard whose own subject is source text under crates/*/src/ cannot live inside crates/chrys-core/tests/ once a plan-wide invariant forbids editing anything there (D-03), even to export a helper of exactly the right shape. crates/chrys-cli/tests/decode_limits_guard.rs carries a second, smaller comment-and-string stripper rather than importing crates/chrys-core/tests/determinism.rs's own, and states why in its own module comment."
  removed: []

key-files:
  created:
    - crates/chrys-cli/src/report.rs
    - crates/chrys-cli/tests/report.rs
    - crates/chrys-cli/tests/decode_limits_guard.rs
  modified:
    - crates/chrys-source/src/lib.rs
    - crates/chrys-source-sequence/src/lib.rs
    - crates/chrys-source-sequence/tests/sequence.rs
    - crates/chrys-source-raster/src/lib.rs
    - crates/chrys-source-animation/tests/animation.rs
    - crates/chrys-cli/Cargo.toml
    - crates/chrys-cli/src/main.rs
    - Cargo.lock

key-decisions:
  - "load_named is a defaulted method on chrys_source::Source, added in the same commit as SequenceSource's own override, so every existing implementor (RasterSource, AnimationSource) keeps compiling with no change of its own, and gets a correct default: RasterSource's one frame, and every one of AnimationSource's composited frames, is named after the one container file it came from."
  - "SequenceSource::load is now implemented in terms of load_named (list once, decode once), rather than the reverse, so the directory-listing order and the frame-to-name pairing cannot compute independently and drift apart (T-03-17)."
  - "The report's rule-outcome computation and the write of the report itself were both moved to before run_compare's hash_only/print branches, in Task 1's own commit, so the report composes with every existing flag and is written on every exit path, including a refused pair and a non-zero exit, without a second, separate restructuring in Task 2. Task 2's own five named tests prove this rather than change it further. See Deviations."
  - "crates/chrys-cli/tests/decode_limits_guard.rs carries its own, second copy of a comment-and-string stripper rather than importing crates/chrys-core/tests/determinism.rs's own strip_comments_and_strings, because that function is private to a test binary inside crates/chrys-core/tests/, and D-03 forbids editing anything there, including its own test directory, to export it. The module's own doc comment states this and names the authoritative copy."
  - "The static guard's allow-list asserts an exact call-site count per file (2 for crates/chrys-source-raster/src/decode.rs, 1 for crates/chrys-source-animation/src/lib.rs), not only a ceiling, so a count moving in either direction goes red, and a companion test (every_allow_listed_file_names_its_own_reason) proves the allow-list's own reason field is never left empty."

requirements-completed: [CLI-02, CLI-04]

coverage:
  - id: D1
    description: "The report over tests/golden/sequence-01 names frame1.png at index 0 and frame11.png at the last index (index 10), not only their zero-based positions (success criterion 6)."
    requirement: CLI-02
    verification:
      - kind: integration
        ref: "manual run recorded verbatim below: cargo run -q -p chrys-cli -- compare tests/golden/sequence-01/base tests/golden/sequence-01/candidate --report ...; grep -q 'source = \"frame1.png\"'"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-sequence/tests/sequence.rs#load_named_pairs_each_frame_with_its_own_file_name_in_order"
        status: pass
    human_judgment: false
  - id: D2
    description: "Each change in the report carries its kind, its bounding box and size, and the named region it fell inside when one applies; a recoloured change carries its colour delta and an alpha term read from the two colours' own fourth bytes (CLI-02)."
    requirement: CLI-02
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/report.rs#the_report_names_kind_region_and_size_per_change"
        status: pass
    human_judgment: false
  - id: D3
    description: "The report is written before run_compare's own exit-code-bearing branches return, so it exists on a non-zero exit and on a refused pair; a refused frame carries reason and holds no change table; a report path in a directory that does not exist fails the run loudly, naming the path."
    requirement: (T-03-15, this plan's own threat register)
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/report.rs#the_report_is_written_when_the_run_exits_non_zero"
        status: pass
      - kind: integration
        ref: "crates/chrys-cli/tests/report.rs#a_refused_pair_is_reported_with_its_reason"
        status: pass
      - kind: integration
        ref: "crates/chrys-cli/tests/report.rs#a_report_path_that_cannot_be_written_fails_loudly"
        status: pass
    human_judgment: false
  - id: D4
    description: "rule_outcome is absent from the document entirely when --rule was not given, and present on every change when it was (T-03-16)."
    requirement: (T-03-16, this plan's own threat register)
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/report.rs#the_rule_outcome_is_absent_without_a_rule_file"
        status: pass
    human_judgment: false
  - id: D5
    description: "Every direct call to image::ImageReader::open( anywhere under a crate's own src directory lives inside one of two allow-listed files, with the exact count each holds; a drill in a disposable worktree proves the guard goes red on an unexpected call site, naming it, and a second drill proves it goes red when a call site is deleted (CLI-04)."
    requirement: CLI-04
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/decode_limits_guard.rs#only_allow_listed_call_sites_open_an_image_reader_directly"
        status: pass
      - kind: other
        ref: "two planted-defect drills in disposable git worktrees, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D6
    description: "No file under crates/chrys-core/ changes anywhere in this plan's commit range, and cargo tree -p chrys-core -e normal names neither chrys-cli, chrys-rule, serde, toml, nor any format crate."
    requirement: (D-03, project-wide invariant)
    verification:
      - kind: other
        ref: "git rev-parse HEAD:crates/chrys-core read c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 before this plan's first commit and after each of its three commits; cargo tree -p chrys-core -e normal (no serde, no toml, no chrys-cli, no chrys-rule, no format crate); scripts/engine-boundary-drill.sh d62b662 5239e48 (2 of 2 drills behaved as expected, recorded verbatim below)"
        status: pass
    human_judgment: false
  - id: D7
    description: "cargo test --workspace and cargo test -p chrys-core --test determinism (all six guards) stay green at every commit boundary; the workspace count only grows (233 to 243), never regresses; cargo clippy --workspace --all-targets -- -D warnings and cargo fmt --check both pass."
    requirement: (CLI-03's regression guard, project-wide invariant)
    verification:
      - kind: other
        ref: "cargo test --workspace: 243 passed, 0 failed, at HEAD (5239e48); cargo test -p chrys-core --test determinism: 6 passed, 0 failed; cargo test -p chrys-cli --test digest: 6 passed, 0 failed (CLI-03 unmoved)"
        status: pass
    human_judgment: false

duration: not captured (PLAN_START_TIME was not recorded at launch; see git log for the three task commits' own timestamps)
completed: 2026-09-07
status: complete
---

# Phase 3 Plan 03: Name the file a frame came from in the report Summary

**`chrys compare --report <PATH>` writes a TOML artifact that names each frame by the file it came from (not only its index), records every change's kind, region and size, survives a non-zero exit and a refused pair, and a static guard proves every direct image-reader call site in the workspace is one of the two this project already knows about — drilled red twice before it is trusted.**

## Performance

- **Duration:** not captured (see frontmatter note)
- **Completed:** 2026-09-07
- **Tasks:** 3 / 3
- **Files modified:** 11 (8 modified, 3 new)

## Accomplishments

- Added `chrys_source::Source::load_named(&self, path) -> Result<Vec<(String, Frame)>, Self::Error>`, a defaulted method whose default body calls `self.load(path)`, takes the path's own file name once (falling back to the path's whole display form when it has no file name component), and pairs it with every returned frame. `chrys-source`'s own dependency table stays empty (`crates/chrys-source/tests/manifest.rs` still passes), and `Frame` still declares exactly the five fields it declared before this plan.
- Overrode `load_named` in `impl Source for SequenceSource`: it lists the directory once (`sequence::list_sorted`), decodes once (`sequence::decode_all`), and zips each file's own name with its decoded frame. `SequenceSource::load` is now implemented in terms of `load_named`, so there is exactly one listing path and one decode path — the frame-to-name pairing cannot drift between two independently computed sorted orders (T-03-17).
- Replaced `chrys-cli`'s own `frames_for` with `named_frames_for`, keeping the same three-way dispatch (directory → `SequenceSource`, sniffed animation → `AnimationSource`, otherwise → `RasterSource`) but returning `(Vec<String>, Vec<Frame>)` through `load_named`. Region cropping still operates on `Frame` only; the base side's own names are carried alongside, indexed the same way, into the report.
- Added `--report <PATH>` to `compare`, and `crates/chrys-cli/src/report.rs` (new): `Report`, `Meta`, `FrameReport`, `ChangeReport` (serialized with `toml`/`serde`, both added to `chrys-cli`'s `Cargo.toml` from the workspace table), and `pub fn write_report`. A `[[frame]]` table per compared index carries `index`, `source` (the base side's own file name), `verdict`, and, for a refused frame, `reason`. A changed frame carries an array of `[[frame.change]]` tables, each with `kind`, `x`/`y`/`width`/`height`, `size` (width × height), `region` (via `chrys_rule::overlapping_hint_name`, the same majority-overlap rule a named region already uses), a `moved` change's `offset_x`/`offset_y`, and a `recoloured` change's `delta_e`/`base_colour`/`candidate_colour`/`alpha_delta` (the alpha term read from the two colours' own fourth bytes, never from `delta_e`, per `chrys_core::ColourDelta`'s own doc comment).
- Restructured `run_compare` (in Task 1's own commit) so the rule outcome is computed, and the report is written, **before** the `hash_only` early return and the verdict-text print branches — not after them. This single restructuring is what makes the report compose with every existing flag and survive every exit path: a non-zero exit, `--hash-only`, `--all-frames`, and a refused pair (which exits 2) all still write the report, and `rule_outcome` is present on every change only when `--rule` was given (via `#[serde(skip_serializing_if = "Option::is_none")]`, so the key is absent, not defaulted, otherwise). `write_report`'s own I/O and serialization errors propagate via `anyhow::Context`, naming the path, and fail the run.
- Task 2 added `crates/chrys-cli/tests/report.rs` (five named tests, matching the plan's own names): `the_report_names_kind_region_and_size_per_change`, `the_report_is_written_when_the_run_exits_non_zero`, `a_refused_pair_is_reported_with_its_reason`, `the_rule_outcome_is_absent_without_a_rule_file`, `a_report_path_that_cannot_be_written_fails_loudly`. Every test parses the written file with `toml::Value` rather than searching its text. No production code changed in Task 2's own commit: Task 1's restructuring already delivered the behaviour these tests prove. See Deviations.
- Task 3 added `crates/chrys-cli/tests/decode_limits_guard.rs` (new): walks every `.rs` file under every crate's own `src` directory (never `tests/`), strips line comments, nested block comments, raw strings and ordinary strings with a second, smaller copy of `crates/chrys-core/tests/determinism.rs`'s own stripper (D-03 forbids importing the original), and asserts the identifier `ImageReader::open(` appears only inside `crates/chrys-source-raster/src/decode.rs` (2 sites: `decode_guarded` and `probe_dimensions`) and `crates/chrys-source-animation/src/lib.rs` (1 site: `open_guessed`), each with its own recorded reason. The count is asserted exactly, in both directions, and an unlisted file calling it is named in the failure message.
- Ran both planted-defect drills the plan's own action text requires, each inside a disposable git worktree created from HEAD and removed on every exit path: one planting an unexpected, `#[cfg(any())]`-guarded call to `ImageReader::open(` inside `crates/chrys-rule/src/mask.rs` (so the planted item parses but is never type-checked, needing no new `image` dependency in that crate); the other deleting `crates/chrys-source-raster/src/decode.rs`'s own `probe_dimensions` call site. Both drills went red and named exactly what was planted (recorded verbatim below).
- `git rev-parse HEAD:crates/chrys-core` read `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` before this plan's first commit and after each of its three commits. `cargo tree -p chrys-core -e normal` names neither `serde`, `toml`, `chrys-cli`, `chrys-rule`, nor any format crate. `scripts/engine-boundary-drill.sh d62b662 5239e48` ran both of its own drills and both behaved as expected (recorded verbatim below).
- `cargo test --workspace` (243 passed, 0 failed, up from the 233 baseline this plan started from), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --check` all pass, at every commit boundary. `cargo test -p chrys-core --test determinism` passes with all six guards, and `cargo test -p chrys-cli --test digest` passes unchanged (CLI-03's regression guard).

## Task Commits

Each task was committed atomically:

1. **Task 1: Name the file a frame came from in the report** — `d62b662`
2. **Task 2: Write the report even when the run exits non-zero** — `7824322`
3. **Task 3: Guard every direct image reader call site** — `5239e48`

**Plan metadata:** pending (this SUMMARY and STATE.md/ROADMAP.md updates are committed by the orchestrator, per this plan's execution instructions)

## Files Created/Modified

- `crates/chrys-source/src/lib.rs` — `Source::load_named` (defaulted method) and `file_name_or_display`
- `crates/chrys-source-sequence/src/lib.rs` — `SequenceSource::load_named` (override), `load` rewritten to call it
- `crates/chrys-source-sequence/tests/sequence.rs` — `load_named_pairs_each_frame_with_its_own_file_name_in_order`
- `crates/chrys-source-raster/src/lib.rs` — `load_named_on_a_single_file_names_the_frame_after_the_file` (unit test of the default body)
- `crates/chrys-source-animation/tests/animation.rs` — `load_named_names_every_frame_after_the_containers_own_file`
- `crates/chrys-cli/Cargo.toml` — added `toml` and `serde` from the workspace table
- `crates/chrys-cli/src/main.rs` — `--report` flag, `named_frames_for` (replaces `frames_for`), `run_compare` restructured to compute rule outcomes and write the report before every exit-code-bearing branch
- `crates/chrys-cli/src/report.rs` — new: `Report`, `Meta`, `FrameReport`, `ChangeReport`, `write_report`, `frame_report`, `change_report`
- `crates/chrys-cli/tests/report.rs` — new: 5 integration tests
- `crates/chrys-cli/tests/decode_limits_guard.rs` — new: the static guard, its own comment-and-string stripper, and 2 tests
- `Cargo.lock` — `toml`/`serde` edges added to `chrys-cli`; no new external crate versions

## Decisions Made

- `load_named` is a defaulted trait method, not a new `Frame` field, matching the plan's own explicit instruction and the reason it states (45 `Frame { .. }` literals under `crates/chrys-core/` would need editing otherwise, which D-03 forbids).
- The rule-outcome computation and the report write were both relocated to before `run_compare`'s `hash_only`/print branches, inside Task 1's own commit, because Task 1's own behaviour requirement ("the flag composes with every existing flag") could not otherwise be met: the original code computed rule outcomes only after the early `hash_only` return, which would have silently skipped the report on that path. See Deviations.
- `crates/chrys-cli/tests/decode_limits_guard.rs` holds its own copy of a comment-and-string stripper, matching the plan's own instruction, because `crates/chrys-core/tests/determinism.rs`'s copy is private to a test binary inside a directory D-03 forbids editing.
- The two planted-defect drills used a disposable `git worktree` created from HEAD (matching `scripts/engine-boundary-drill.sh`'s own established pattern) rather than mutating this worktree's own tracked files, so the real working tree was never at risk and no `git clean`/`git reset --hard` was needed to recover from either plant.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 2's own behaviour was already delivered by Task 1's commit; Task 2 added only tests, no further production code**
- **Found during:** Task 1, while implementing the plan's own instruction that `--report` "composes with every existing flag and changes no exit code and no stdout."
- **Issue:** The rule-outcome computation (`outcomes`) and the exit-code computation (`worst`) both lived, in the pre-existing code, after the `hash_only` branch's own early `return`. A report built only at that point would never be written when `--hash-only` was given, and more importantly, a `Refused` verdict and a non-zero-exit `Changed` verdict both still reach that point today, but the plan's own Task 2 explicitly assigns "write the report even when the run exits non-zero" and "a refused pair still writes the report" to a second, later commit — implying Task 1's own report-writing code would need a second pass to reach those paths. Implementing Task 1's own literal instruction ("changes no exit code and no stdout" — i.e., composes with `--hash-only` too) required moving the outcome computation and the report write earlier than either task's own text placed them, in one motion, inside Task 1's commit.
- **Fix:** Moved the `outcomes` computation and the `report::write_report` call to immediately after `verdicts` is computed, before the `hash_only` branch, inside Task 1's own commit (`d62b662`). This one change already satisfies every behaviour Task 2's own action text separately describes: the report is written before every early return, so it exists on `--hash-only`, on a non-zero exit, and on a refused pair, and `rule_outcome` is already correctly absent or present per-change. Task 2's own commit (`7824322`) therefore added the five named tests with no further change to `report.rs` or `main.rs`.
- **Files modified:** `crates/chrys-cli/src/main.rs`, `crates/chrys-cli/src/report.rs` (both already in Task 1's own `files_modified` list; no file outside the plan's own list was touched).
- **Verification:** All five of Task 2's own named tests (`the_report_is_written_when_the_run_exits_non_zero`, `a_refused_pair_is_reported_with_its_reason`, `the_rule_outcome_is_absent_without_a_rule_file`, plus the other two) pass against the code as it stood after Task 1's own commit, with zero further production-code changes in Task 2's commit. `cargo test -p chrys-cli --test digest` (CLI-03's regression guard) stays green throughout, proving `--report`'s absence still changes nothing.
- **Committed in:** `d62b662` (Task 1 commit); proven by tests added in `7824322` (Task 2 commit).
- **Precedent:** The same shape 03-02-SUMMARY.md recorded: a task's own `<verify>` commands, run against the real, wired-up binary, sometimes require production code the plan's own task split assigned to a later task's own action text, because the earlier task's own words ("composes with every existing flag") already implied it.

---

**Total deviations:** 1 auto-fixed (blocking, Rule 3). No file outside either task's own `files_modified` list was touched; every file this plan changes is exactly the set the plan's own frontmatter names, split across the three tasks' commits (`report.rs`/`main.rs` in Task 1, `tests/report.rs` in Task 2, `tests/decode_limits_guard.rs` in Task 3).

## Issues Encountered

None beyond the deviation recorded above.

## User Setup Required

None — no external service configuration required.

## The Report Over `tests/golden/sequence-01`, First Two and Last Frame Entries, Recorded Verbatim

`cargo run -q -p chrys-cli -- compare tests/golden/sequence-01/base tests/golden/sequence-01/candidate --report /tmp/chrys-report-tracer.toml`:

```toml
[meta]
base = "tests/golden/sequence-01/base"
candidate = "tests/golden/sequence-01/candidate"

[[frame]]
index = 0
source = "frame1.png"
verdict = "identical"

[[frame]]
index = 1
source = "frame2.png"
verdict = "identical"

...

[[frame]]
index = 10
source = "frame11.png"
verdict = "identical"
```

`grep -q 'source = "frame1.png"'` exits 0 against this file. Index 0 names `frame1.png`, not `0`; the last entry (index 10) names `frame11.png`, not `10` — success criterion 6.

## The Report Over the Refused Pair, Recorded Verbatim

`cargo run -q -p chrys-cli -- compare tests/golden/refuse-01/should-refuse/pair-01/base.png tests/golden/refuse-01/should-refuse/pair-01/candidate.png --report /tmp/chrys-refused-report.toml` (exit code `2`):

```toml
[meta]
base = "tests/golden/refuse-01/should-refuse/pair-01/base.png"
candidate = "tests/golden/refuse-01/should-refuse/pair-01/candidate.png"

[[frame]]
index = 0
source = "base.png"
verdict = "refused"
reason = "the pair is too different to register: peak confidence 976.82 is below the threshold 2313.88; this engine compares near-identical pairs only"
```

The refused frame carries `reason` and no `[[frame.change]]` table at all.

## The Two Planted-Defect Drills for `decode_limits_guard.rs`, Recorded Verbatim

Both drills ran inside a disposable `git worktree add --detach --quiet <path> HEAD`, created after Task 3's own commit (`5239e48`) so the guard test itself already existed to be drilled, and removed with `git worktree remove --force <path>` on completion. The real working tree held no uncommitted changes before or after either drill.

**Drill one:** planted an unexpected call site inside `crates/chrys-rule/src/mask.rs`, guarded by `#[cfg(any())]` so the planted item parses (and is found by the guard's own text search) but is never type-checked, needing no new `image` dependency in that crate:

```
running 1 test

thread 'only_allow_listed_call_sites_open_an_image_reader_directly' panicked at crates/chrys-cli/tests/decode_limits_guard.rs:325:5:
the following file(s) call `ImageReader::open(` directly but are not in this guard's own allow-list: /.../crates/chrys-cli/../../crates/chrys-rule/src/mask.rs (1 site(s)). A decode that does not go through decode_guarded (or the animation adapter's own equivalent) reopens the decompression-bomb risk CLI-04 exists to close; either route this call through the existing guarded entry point, or, if a new guarded entry point is genuinely needed, add it to ALLOW_LIST in this file, deliberately, in the same change.
test only_allow_listed_call_sites_open_an_image_reader_directly ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.03s
```

The guard went red and named `crates/chrys-rule/src/mask.rs`, the exact file the drill planted the call site into.

**Drill two:** deleted `crates/chrys-source-raster/src/decode.rs`'s own `probe_dimensions` call site (dropping that file's own count from 2 to 1), in a fresh disposable worktree:

```
running 1 test

thread 'only_allow_listed_call_sites_open_an_image_reader_directly' panicked at crates/chrys-cli/tests/decode_limits_guard.rs:311:17:
assertion `left == right` failed: crates/chrys-source-raster/src/decode.rs holds 1 call site(s) to `ImageReader::open(`, but the allow-list expects exactly 2. A decode that does not go through decode_guarded reopens the decompression-bomb risk CLI-04 exists to close; if this file's own call count moved on purpose, the allow-list's own `count` must be updated to match, deliberately, in the same change.
  left: 1
 right: 2
test only_allow_listed_call_sites_open_an_image_reader_directly ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.02s
```

The guard's own exact-count assertion went red and named the exact file whose call site was deleted, with the found count (1) and the expected count (2).

## `git diff --stat`, This Plan's Own Commit Range, `crates/chrys-core/`

`git diff --stat d62b662^..5239e48 -- crates/chrys-core/`: empty output. No file under `crates/chrys-core/` changed anywhere in this plan's own three commits.

## `scripts/engine-boundary-drill.sh`, This Plan's Own Commit Range

`scripts/engine-boundary-drill.sh d62b662 5239e48`, full output:

```
engine-boundary-drill: drill one: checked range 5239e481b9932406d736ea57b547078b577a62b1..14279e57817a2e938f9f5f61c357f7b818cdf115
engine-boundary-drill: drill one: check output:
crates/chrys-core/src/lib.rs
drill ok: planted-defect drill: the check went red and named crates/chrys-core/src/lib.rs
engine-boundary-drill: drill two: checked range d62b662^..5239e48
engine-boundary-drill: drill two: check output:
(empty)
drill ok: clean-range drill: the check stayed silent over plan 02-03's own range

engine-boundary-drill: 2 of 2 drills behaved as expected
```

Both drills behaved as expected: the planted-defect drill went red and named the exact file planted, and the clean-range drill stayed silent over this plan's own real commit range (`d62b662^..5239e48`, all three of this plan's task commits). The working tree was clean before and after the drill ran; `git rev-parse HEAD:crates/chrys-core` still reads `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` afterward.

## Full Verification, This Plan's End State

```
cargo test -p chrys-source (incl. load_named default-body test): passes.
cargo test -p chrys-source-sequence --test sequence: 3 passed, 0 failed (incl. load_named_pairs_each_frame_with_its_own_file_name_in_order).
cargo test -p chrys-source --test manifest: 1 passed, 0 failed (chrys-source still declares no dependency).
cargo test -p chrys-cli --test report: 5 passed, 0 failed.
cargo test -p chrys-cli --test decode_limits_guard: 2 passed, 0 failed.
cargo test -p chrys-cli --test digest: 6 passed, 0 failed (CLI-03 unmoved).
cargo test --workspace: 243 passed, 0 failed (up from the 233 baseline this plan started from; +10 new tests: 3 load_named tests across chrys-source/chrys-source-sequence/chrys-source-animation, 5 in tests/report.rs, 2 in tests/decode_limits_guard.rs).
cargo test -p chrys-core --test determinism: 6 passed, 0 failed (all six guards green).
cargo clippy --workspace --all-targets -- -D warnings: clean.
cargo fmt --check: clean.
cargo tree -p chrys-core -e normal: no serde, no toml, no chrys-cli, no chrys-rule, no format/GPU crate.
git rev-parse HEAD:crates/chrys-core: c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 (unmoved, checked before the first commit and after each of the three commits).
scripts/engine-boundary-drill.sh d62b662 5239e48: 2 of 2 drills behaved as expected.
```

## Predictions That Turned Out Wrong

- None. Every prediction and instruction in this plan (the defaulted-trait-method route for a frame's own name, the report's own key names and structure, the two-file allow-list and its exact counts, the need for a second comment-and-string stripper rather than a shared one) matched exactly. The one deviation recorded above is a task-boundary shift (Task 1's own commit already delivering behaviour Task 2's own action text separately describes), not an incorrect prediction about what the code should do.

## Next Phase Readiness

- CLI-02 (the report names kind, region and size, and is written on a failing run) and CLI-04 (the static, twice-drilled guard over every direct decode call site) are both delivered and observable.
- Success criterion 6 (the report names the file a frame came from, not only its index) is delivered: `Source::load_named` and the report's own `source` field are the shape any later plan reading a sequence's own frames by name must now match.
- `chrys-core`'s tree object id (`c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`) and dependency graph (no `serde`, no `toml`, no `chrys-cli`, no `chrys-rule`) are unmoved, checked at every commit boundary and by the engine-boundary drill over this plan's own commit range.
- This closes the three-plan phase 3 wave (03-01 rule schema and evaluation, 03-02 mask-scoped rules, 03-03 the report and the decode guard); `crates/chrys-cli/src/report.rs`'s own `Report`/`FrameReport`/`ChangeReport` shapes are the reference implementation for any later plan that needs to add a new field to the CI artifact.

## Self-Check: PASSED

All created files verified present on disk (`crates/chrys-cli/src/report.rs`, `crates/chrys-cli/tests/report.rs`, `crates/chrys-cli/tests/decode_limits_guard.rs`); all three task commit hashes (`d62b662`, `7824322`, `5239e48`) verified present in `git log --oneline`; `git rev-parse HEAD:crates/chrys-core` verified reading `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` after the final commit; working tree verified clean (`git status --short`) after both planted-defect drills' worktrees were removed.

---
*Phase: 03-baseline-rules-and-ci-gate*
*Completed: 2026-09-07*
