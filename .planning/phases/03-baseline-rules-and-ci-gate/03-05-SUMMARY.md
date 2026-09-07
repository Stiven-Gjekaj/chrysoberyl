---
phase: 03-baseline-rules-and-ci-gate
plan: 05
subsystem: cli-rule-baseline
tags: [rust, cli-gate, coordinate-space, staged-write, path-traversal]

# Dependency graph
requires:
  - phase: 03-baseline-rules-and-ci-gate
    provides: "03-01 through 03-04's own rule engine, mask loader, report writer and baseline store, all merged before this plan started (base commit 6a7a1e1)"
provides:
  - "run_compare's own single coordinate space: base_frames/candidate_frames stay uncropped for the whole function; compared_base_frames/compared_candidate_frames hold the cropped pixels the comparison actually reads; translate_verdicts adds each frame's own crop origin back onto every reported Region.bbox, on both the flag-less and the --region path, so a rule reads the frame it was authored against and every printed rectangle is stated in that frame's own coordinates (CR-01, T-03-27, T-03-28)."
  - "report::Meta.region: the --region name a run compared inside, when one was given, omitted entirely otherwise (CLI-02, T-03-29)."
  - "write_baseline's staged write: the new baseline is built in <root>/.<name>.accept-tmp and the new manifest in MANIFEST.toml.tmp, both complete, before either is swapped into place by rename; the old baseline directory is renamed aside rather than deleted and removed only after both swaps succeed (BASE-04, T-03-30, T-03-31)."
  - "resolve_mask_path returns the canonical path it verified, not the merely-joined one the caller used to open a second time (RULE-02, T-03-33, WR-01)."
  - "Scope's own doc comment names validate_row and RuleRow as the place and the type an unscoped rule is actually refused by (T-03-34, IN-01)."
affects: []

# Actuals (#2632)
actuals:
  tokens: 12631
  tasks: 3
  commits: 4
  plan_head_before: 6a7a1e1b4f4c9c313dee491863e6037671e3eee8

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A cropped comparison and an uncropped rule/report share one code path: crop_frames_to_region returns each cropped frame paired with its own crop origin (the hint's x, y), and translate_verdicts adds that origin back onto every Region.bbox unconditionally, with origin (0, 0) on the flag-less path, so there is exactly one place a coordinate space could diverge and it does not."
    - "A staged write never deletes the last good state before the new one is complete: write_baseline builds the new baseline in a sibling '.<name>.accept-tmp' directory and the new manifest in 'MANIFEST.toml.tmp', and only renames both into place, one at a time, after both are done. A stale staging directory is cleared before use (it could leak a file); a stale temporary manifest is not (fs::write truncates it, so it cannot)."
    - "A path-traversal check returns the exact path it verified, not a second, unchecked resolution of the same text: resolve_mask_path now returns canonical_mask, so the caller's own open resolves nothing this function has not already checked."
  removed: []

key-files:
  created:
    - crates/chrys-cli/tests/region_rule.rs
    - tests/golden/rule-01/mask-cropped-size.toml
    - tests/golden/rule-01/masks/badge-cropped.png
  modified:
    - crates/chrys-cli/src/main.rs
    - crates/chrys-cli/src/report.rs
    - crates/chrys-baseline/src/golden.rs
    - crates/chrys-baseline/tests/store.rs
    - crates/chrys-rule/src/mask.rs
    - crates/chrys-rule/src/lib.rs
    - crates/chrys-source-raster/examples/make-fixtures.rs

key-decisions:
  - "write_baseline's staged-write logic stayed inside one function (a local closure holding every fs:: call, invoked once, with cleanup after) rather than being split into a helper function. The write-call guard's own function_body_span scan only recognises calls textually inside fn write_baseline's own brace-matched body under that literal name; a separate fn write_baseline_staged would have put every fs:: call outside that span and made the guard flag all of them as offenders. This is a deviation from a first draft, corrected before the guard was ever run in anger; see Deviations."
  - "make-fixtures.rs (crates/chrys-source-raster/examples/make-fixtures.rs), not in this plan's own <files> list, was extended to generate masks/badge-cropped.png and mask-cropped-size.toml, following the exact pattern every earlier committed fixture in this repository already uses (a committed generator, run with cargo run -p chrys-source-raster --example make-fixtures, so a reader can reproduce every byte). Writing a one-off, uncommitted script would have broken that project-wide convention for the one fixture this plan needed most. See Deviations."
  - "crop_frames_to_region's return type changed from Vec<Frame> to Vec<(Frame, (u32, u32))>, pairing each cropped frame with its own crop origin at the one place the hint is read, exactly as the plan's own action text specifies, rather than threading the origin through a second parallel lookup later."

requirements-completed: [CLI-01, CLI-02, BASE-04, RULE-02]

coverage:
  - id: D1
    description: "Each of the four rule-01 fixture rule files gives the same exit code with and without --region badge, and each equals its own expected absolute code: mask-cropped-size.toml exits 3, mask-tolerate.toml exits 0, tolerate.toml exits 0, too-tight.toml exits 1 (CLI-01)."
    requirement: CLI-01
    verification:
      - kind: unit
        ref: "crates/chrys-cli/tests/region_rule.rs#a_rule_gives_the_same_exit_code_with_and_without_a_region"
        status: pass
      - kind: integration
        ref: "manual run recorded verbatim below over a freshly built release binary"
        status: pass
    human_judgment: false
  - id: D2
    description: "A change's rectangle is reported in the base frame's own coordinates (x=40, y=40), not the crop-local ones (x=10, y=10), with --region given."
    requirement: CLI-01
    verification:
      - kind: unit
        ref: "crates/chrys-cli/tests/region_rule.rs#a_region_run_reports_the_rectangle_in_frame_coordinates"
        status: pass
    human_judgment: false
  - id: D3
    description: "A report written with --region names the region each change falls in, and meta.region names the region the run compared inside."
    requirement: CLI-02
    verification:
      - kind: unit
        ref: "crates/chrys-cli/tests/region_rule.rs#a_region_report_names_the_region_each_change_falls_in"
        status: pass
    human_judgment: false
  - id: D4
    description: "An accept whose manifest write fails partway (after every candidate file is already staged) leaves the previously accepted baseline's own bytes, and MANIFEST.toml's own bytes, unchanged; resolve still returns the previous candidate's bytes."
    requirement: BASE-04
    verification:
      - kind: unit
        ref: "crates/chrys-baseline/tests/store.rs#a_failed_accept_leaves_the_previous_baseline_intact"
        status: pass
      - kind: other
        ref: "planted-defect drill in a disposable worktree, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D5
    description: "A stale staging directory left by an earlier crashed accept contributes no file to the next accepted baseline; the write-call guard's own constant names fs::rename and is drilled red against a rename planted outside write_baseline."
    requirement: BASE-04
    verification:
      - kind: unit
        ref: "crates/chrys-baseline/tests/store.rs#a_stale_staging_directory_contributes_no_file_to_the_next_accept, only_the_accept_path_writes_to_the_store"
        status: pass
      - kind: other
        ref: "planted-defect drill in a disposable worktree, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D6
    description: "resolve_mask_path returns the canonical path it verified, not the merely joined one; LoadedRules::mask_for's own key is unaffected, confirmed by reading load_rules and mask_for directly."
    requirement: RULE-02
    verification:
      - kind: unit
        ref: "crates/chrys-rule/src/mask.rs#resolve_mask_path_returns_the_path_it_verified"
        status: pass
      - kind: other
        ref: "planted-defect drill in a disposable worktree, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D7
    description: "No file under crates/chrys-core/ changes anywhere in this plan's commit range, and the whole-phase engine-boundary drill stays silent over the whole phase's own commit range."
    requirement: (D-03, project-wide invariant)
    verification:
      - kind: other
        ref: "git rev-parse HEAD:crates/chrys-core read c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 before this plan's first commit and after each of its four commits; scripts/engine-boundary-drill.sh 29f5add 3ca8ddc (2 of 2 drills behaved as expected once the last-commit argument was passed as a resolved hash, recorded verbatim below)"
        status: pass
    human_judgment: false
  - id: D8
    description: "cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --check all pass at every commit boundary; the workspace test count only grows (254 to 261)."
    requirement: (project-wide invariant)
    verification:
      - kind: other
        ref: "cargo test --workspace: 261 passed, 0 failed, at HEAD (3ca8ddc); cargo clippy --workspace --all-targets -- -D warnings: clean; cargo fmt --check: clean; cargo test -p chrys-core --test determinism: 6 passed, 0 failed"
        status: pass
    human_judgment: false

duration: not captured (PLAN_START_TIME was not recorded at launch)
completed: 2026-09-08
status: complete
---

# Phase 3 Plan 05: Close the region/rule coordinate mismatch and the destructive baseline write Summary

**One coordinate space for a compared region (the false green under `--region` plus `--rule` is gone), a staged baseline write that never deletes the last good state before the new one exists, a mask loader that opens the path its own traversal check verified, and a doc comment that names the function that actually refuses an unscoped rule.**

## Performance

- **Duration:** not captured (see frontmatter note)
- **Completed:** 2026-09-08
- **Tasks:** 3 / 3 (four commits: Task 3 split into its own two commits, as its own action text specifies)
- **Files modified:** 10 (7 modified, 3 new)

## Accomplishments

### Task 1 — CR-01 (the false green) and T-03-28/T-03-29

- `run_compare` no longer shadows `base_frames`/`candidate_frames` with the region-cropped versions. Those two names stay bound to the uncropped frames `named_frames_for` returned, for the whole function; `compared_base_frames`/`compared_candidate_frames` (backed by a new `RegionCrop` type alias, `(Vec<Frame>, Vec<Frame>, Vec<(u32, u32)>)`) hold the cropped pixels the pixel comparison actually reads.
- `crop_frames_to_region` now returns `Vec<(Frame, (u32, u32))>`: each cropped frame paired with its own crop origin (the hint's `x`, `y`), produced at the one place the hint is looked up.
- A new `translate_verdicts`/`translate_verdict`/`translate_region` chain adds each frame's own origin back onto every `Region.bbox.x`/`.y` (never `offset_px`, `width`, or `height`), using `checked_add` with a message naming both numbers on overflow, unconditionally on both the flag-less path (origin `(0, 0)`, a no-op by value) and the `--region` path. This is what makes the two paths share one code path instead of two that can diverge.
- The `--hash-only` branch now points explicitly at `compared_base_frames`/`compared_candidate_frames`, since it no longer inherits a cropped binding by accident.
- `report::Meta` gained an optional `region: Option<String>` field (`skip_serializing_if = "Option::is_none"`, matching `rule`), populated from `--region` when given.
- `crates/chrys-cli/tests/region_rule.rs` (new, 4 tests): `a_rule_gives_the_same_exit_code_with_and_without_a_region` (all four `rule-01` rule files, exit code asserted against its own expected absolute value, not just the two runs against each other), `a_region_run_reports_the_rectangle_in_frame_coordinates`, `a_region_report_names_the_region_each_change_falls_in`, and `the_mask_scoped_rule_refuses_the_untolerated_change_under_a_region`.
- New fixtures: `tests/golden/rule-01/masks/badge-cropped.png` (70x60, white and opaque, sized to the crop rather than the frame) and `tests/golden/rule-01/mask-cropped-size.toml` (`mask = "masks/badge-cropped.png"`, `max_delta_e = 116.0`), both generated by an extension to `crates/chrys-source-raster/examples/make-fixtures.rs`'s own `write_rule_01_masks`, run with `cargo run -p chrys-source-raster --example make-fixtures`, and committed. Regenerating every other fixture the same run touches produced byte-identical output (`git status --short tests/golden/` showed only the two new files).

### Task 2 — CR-02 (the destructive write) and T-03-30/T-03-31/T-03-32

- `write_baseline` no longer starts with `fs::remove_dir_all` on the live baseline directory. It now: (1) clears and recreates a sibling staging directory `<root>/.<name>.accept-tmp`; (2) copies every candidate file into it (same byte-for-byte, non-recursive rule as before); (3) writes the new manifest to `<root>/MANIFEST.toml.tmp` (`fs::write` truncates a stale one, so it is not cleared first — the doc comment states this asymmetry against the staging directory, which is cleared because it could otherwise leak a leftover file); (4) only then renames the old baseline directory aside to `<root>/.<name>.previous-tmp` (when one exists), renames the staging directory into the baseline path, renames the temporary manifest over `MANIFEST.toml`, and removes the directory moved aside.
- The whole staged sequence lives inside one closure defined and called inside `write_baseline` itself (not a second, separate function — see Deviations), so every `fs::` call the static guard scans for still falls inside the one span the guard's `function_body_span(_, "write_baseline")` recognises.
- The doc comment states the one window this does not close (between the two swap-renames, the baseline directory and `MANIFEST.toml` briefly disagree) and why it is safe (`verify_baseline_digests` fails every later `compare --baseline` loudly for as long as they do).
- `WRITE_CALLS` in `crates/chrys-baseline/tests/store.rs` gained `"fs::rename("`.
- Two new tests: `a_failed_accept_leaves_the_previous_baseline_intact` (plants a directory at `MANIFEST.toml.tmp`'s own path, so `fs::write` fails after every candidate file is already staged; asserts the previous baseline's own files and `MANIFEST.toml`'s own bytes are unchanged, and `resolve` still returns the first candidate's bytes) and `a_stale_staging_directory_contributes_no_file_to_the_next_accept` (plants a leftover file inside `.<name>.accept-tmp` before an accept; asserts the accepted baseline holds only the real candidate's file).

### Task 3 — RULE-02/WR-01 and T-03-34/IN-01 (two commits)

- `resolve_mask_path` now returns `canonical_mask` (the path it already verified) instead of `joined` (the path it did not). Confirmed by reading both call sites: `LoadedRules::mask_for` and `load_rules`'s own `masks` map are keyed by the rule row's own as-written `PathBuf`, never by this function's return value, which `load_rules` uses only to call `mask::load_mask`. The key is unaffected.
- New unit test `resolve_mask_path_returns_the_path_it_verified` asserts the returned path equals the joined path's own canonical form, printing both on failure.
- `Scope`'s doc comment no longer claims an unscoped rule is refused "at parse time, at the type level." It now names `Scope` as the type this crate hands to the evaluator, `RuleRow` as the permissive deserialised type (`region`/`mask` both `Option`, both absent parses without complaint), and `validate_row` as the function that converts one into the other, refusing an unscoped rule before any comparison runs (D-01). No second overstated occurrence was found near `validate_row` itself. This commit changed no behaviour and added no test.

## Task Commits

1. **Task 1: Compare a region in the frame's own coordinate space** — `986f663`
2. **Task 2: Stage a baseline before it replaces the last good one** — `4766b1d`
3. **Task 3a: Open the mask path the traversal check verified** — `4c16083`
4. **Task 3b: Say which type refuses an unscoped rule** — `3ca8ddc`

**Plan metadata:** pending (this SUMMARY and STATE.md/ROADMAP.md updates are committed by the orchestrator).

## Files Created/Modified

- `crates/chrys-cli/src/main.rs` — `crop_frames_to_region` (new return shape), `translate_verdicts`/`translate_verdict`/`translate_region` (new), `run_compare`'s region-crop block, `--hash-only` branch pointed at cropped frames explicitly
- `crates/chrys-cli/src/report.rs` — `Meta.region` field
- `crates/chrys-cli/tests/region_rule.rs` — new: 4 tests
- `tests/golden/rule-01/mask-cropped-size.toml` — new fixture
- `tests/golden/rule-01/masks/badge-cropped.png` — new fixture (70x60, white opaque)
- `crates/chrys-source-raster/examples/make-fixtures.rs` — `write_rule_01_masks` extended
- `crates/chrys-baseline/src/golden.rs` — `write_baseline` rewritten as a staged write
- `crates/chrys-baseline/tests/store.rs` — `WRITE_CALLS` gained `fs::rename(`; two new durability tests
- `crates/chrys-rule/src/mask.rs` — `resolve_mask_path` returns the canonical path; new unit test
- `crates/chrys-rule/src/lib.rs` — `Scope`'s doc comment corrected

## Decisions Made

See `key-decisions` in frontmatter.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A first draft split `write_baseline`'s staged logic into a second function, which the write-call guard cannot see**
- **Found during:** Task 2, immediately before running `cargo clippy`/`cargo test` the first time.
- **Issue:** The first implementation moved every `fs::` call into a new `write_baseline_staged` helper, called once from `write_baseline`. `only_the_accept_path_writes_to_the_store`'s own `function_body_span(&golden_stripped, "write_baseline")` scans only the byte range between `fn write_baseline(`'s own brace-matched body; a call inside a differently-named function is outside that span by construction, so every `fs::` call in the helper would have been flagged as an offender the first time the guard ran, defeating the guard the plan explicitly requires this task to keep proving.
- **Fix:** Merged the staged-write logic back into `write_baseline` as one function, using a local closure (`stage_and_swap`) that holds every `fs::` call and is invoked once; the closure's own braces are still inside `write_baseline`'s own brace-matched span, so the guard's scan covers it correctly.
- **Files modified:** `crates/chrys-baseline/src/golden.rs`.
- **Verification:** `cargo test -p chrys-baseline --test store` (9 passed, including `only_the_accept_path_writes_to_the_store`); both Task 2 drills (recorded below) behaved as expected against this merged shape.
- **Committed in:** `4766b1d` (the merged shape is what was committed; the two-function draft was never committed).

**2. [Rule 2 - Missing critical functionality] `masks/badge-cropped.png` needed a reproducible generator, and this repository's own convention for that is a committed script, not an ad hoc one**
- **Found during:** Task 1, before writing the adversarial mask fixture.
- **Issue:** The plan's own action text requires generating the mask "once locally" and recording "the exact generating command verbatim in the summary so a later reader can rebuild it." Every other fixture in this repository (all ten prior plans' worth) is built by extending the committed `crates/chrys-source-raster/examples/make-fixtures.rs`, specifically so a reader can reproduce every byte without trusting a one-off shell session. A throwaway script outside that file, or a manually-encoded PNG, would have been the one fixture in the repository not built that way.
- **Fix:** Extended `write_rule_01_masks` in `make-fixtures.rs` with the new mask and rule-file writes, then ran `cargo run -p chrys-source-raster --example make-fixtures` and committed the two new outputs. Every other file the same run regenerated was confirmed byte-identical (`git status --short tests/golden/` showed only the two new paths as untracked before staging).
- **Files modified:** `crates/chrys-source-raster/examples/make-fixtures.rs` (not in this plan's own `<files>` list for Task 1).
- **Verification:** the four rule-01 behaviours in `region_rule.rs` all pass against the generated fixture; `git status --short tests/golden/` confirmed no other committed fixture moved.
- **Committed in:** `986f663`.

---

**Total deviations:** 2 auto-fixed (1 bug/Rule 1, 1 missing functionality/Rule 2). No file outside the plan's own stated intent was touched; the one file added to the touched set (`make-fixtures.rs`) exists to generate a fixture the plan's own `<files>` list already names.

## Issues Encountered

**`scripts/engine-boundary-drill.sh`'s own last-commit argument must be a resolved hash, not the literal string `HEAD`.** The first invocation of this plan's own required phase-gate drill was run as `engine-boundary-drill.sh 29f5add HEAD` and reported the clean-range drill FAILED, naming `crates/chrys-core/src/lib.rs` as changed over `29f5add^..HEAD`. This was not a real defect: `git diff --name-only 29f5add^..HEAD -- crates/chrys-core/`, run directly against the real repository, was empty. The script's own drill one commits a planted defect inside a disposable worktree before drill two runs; drill two then re-resolves its own `$2` argument (`"HEAD"`) *inside that same worktree*, whose `HEAD` had by then advanced past the real range's own last commit to the planted-defect commit drill one had just made. Re-running with the literal resolved hash (`3ca8ddccc4a4cb6fcefe84b036edb3ef447c7fda`) in place of `HEAD` produced the correct, expected result: 2 of 2 drills passed. This is a caveat about the script's own argument handling, not a finding against this plan's own code; it is recorded here, not fixed, because the script itself is outside this plan's `<files>`.

## User Setup Required

None — no external service configuration required.

## The Re-Driven CR-01 Reproduction, On A Freshly Built Release Binary, Recorded Verbatim

```
$ cargo build --release
    Finished `release` profile [optimized] target(s) in 1.24s

$ ./target/release/chrys compare tests/golden/rule-01/base.png \
    tests/golden/rule-01/candidate.png \
    --rule tests/golden/rule-01/mask-cropped-size.toml
mask masks/badge-cropped.png is 70x60, which does not match the 256x256 frame it is scoped against
$ echo $?
3

$ ./target/release/chrys compare tests/golden/rule-01/base.png \
    tests/golden/rule-01/candidate.png \
    --rule tests/golden/rule-01/mask-cropped-size.toml --region badge
mask masks/badge-cropped.png is 70x60, which does not match the 256x256 frame it is scoped against
$ echo $?
3
```

Both exit 3. Both name 70x60 against 256x256. The second command no longer exits 0: the false green CR-01 named is gone.

## The Command That Generated `masks/badge-cropped.png`

```
cargo run -p chrys-source-raster --example make-fixtures
```

The mask itself is built in `write_rule_01_masks` (`crates/chrys-source-raster/examples/make-fixtures.rs`) as `RgbaImage::from_pixel(mask_width, mask_height, Rgba([255, 255, 255, 255]))`, where `(mask_width, mask_height)` is `RULE_01_REGION`'s own width and height (`(70, 60)`, the `badge` hint's own cropped size), not `WIDTH`/`HEIGHT` (the frame's `256x256`). This is the one property under test.

## The Four Rule Files, With and Without `--region badge`, Exit Codes Recorded Verbatim

| Rule file | Plain exit | `--region badge` exit | Expected |
|---|---|---|---|
| `mask-cropped-size.toml` | 3 | 3 | 3 |
| `mask-tolerate.toml` | 0 | 0 | 0 |
| `tolerate.toml` | 0 | 0 | 0 |
| `too-tight.toml` | 1 | 1 | 1 |

Before this plan's fix, `mask-tolerate.toml` exited 3 under `--region` (the mask-size check ran against the 70x60 crop instead of the 256x256 frame it was authored against) and `tolerate.toml` exited 1 under `--region` (a cropped frame carries no hints, so the region-scoped rule silently stopped matching). Both are now 0 in both runs, matching the whole-frame behaviour.

## The Region Report's `region` Value, Recorded Verbatim

```toml
[meta]
base = "tests/golden/rule-01/base.png"
candidate = "tests/golden/rule-01/candidate.png"
region = "badge"

[[frame.change]]
kind = "recoloured"
x = 40
y = 40
...
region = "badge"
```

`meta.region` and the one change's own `region` both read `"badge"`.

## The Four Planted-Defect Drills, Recorded Verbatim

Every drill ran inside a disposable `git worktree add --detach --quiet <path> HEAD`, made after the relevant task's own commit, removed with `git worktree remove --force <path>` on completion. The real working tree held no uncommitted changes before or after any drill.

**Drill 1 (Task 1): the shadowing rebinding restored.** `base_frames`/`candidate_frames` rebound to the region-cropped values, `compare_sequence` run on those directly, no origin translation:

```
test a_rule_gives_the_same_exit_code_with_and_without_a_region ... FAILED
thread 'a_rule_gives_the_same_exit_code_with_and_without_a_region' panicked at crates/chrys-cli/tests/region_rule.rs:113:9:
assertion `left == right` failed: mask-cropped-size.toml: the --region run exited 0, expected 3. stderr:
  left: 0
 right: 3
```

The test went red and its message names the observed exit code (0) against the expected code (3), exactly as the plan requires.

**Drill 2 (Task 2, durability): `write_baseline`'s destructive first step restored.** `fs::remove_dir_all(&baseline_dir)` reinstated at the top of the staged closure, before any staging or renaming:

```
test a_failed_accept_leaves_the_previous_baseline_intact ... FAILED
thread 'a_failed_accept_leaves_the_previous_baseline_intact' panicked at crates/chrys-baseline/tests/store.rs:268:33:
cannot read /var/folders/.../chrys-baseline-test-failed-accept-leaves-previous-intact-4036-0/pair-01: No such file or directory (os error 2)
```

The previously accepted baseline directory (`pair-01`) was deleted before the manifest write failed, so the durability test's own read of it fails, naming that path.

**Drill 3 (Task 2, static guard): a `fs::rename` planted outside `write_baseline`.** Injected at the top of `GoldenFileStore::resolve`'s own body:

```
test only_the_accept_path_writes_to_the_store ... FAILED
thread 'only_the_accept_path_writes_to_the_store' panicked at crates/chrys-baseline/tests/store.rs:632:5:
the following write call(s) live outside write_baseline, the one function every write in this crate must go through: /private/tmp/chrys-drill-task2b/crates/chrys-baseline/src/golden.rs@3433, inside fn resolve: `fs::rename(`
```

The guard names `resolve` and `fs::rename(` exactly.

**Drill 4 (Task 3): `resolve_mask_path` reverted to returning `joined`.**

```
test mask::tests::resolve_mask_path_returns_the_path_it_verified ... FAILED
thread 'mask::tests::resolve_mask_path_returns_the_path_it_verified' panicked at crates/chrys-rule/src/mask.rs:299:9:
assertion `left == right` failed: resolve_mask_path returned /var/folders/.../m.png, but the path it verified was /private/var/folders/.../m.png
  left: "/var/folders/.../m.png"
 right: "/private/var/folders/.../m.png"
```

The test went red and both differing paths are printed. This drill also confirms, concretely on this machine, the exact macOS `/var` → `/private/var` resolution the plan warned about: the two paths differ only in that prefix. Against the fixed code (returning `canonical_mask`), no test message in `cargo test -p chrys-rule` or `cargo test -p chrys-cli --test rule_gate` changed or referenced a resolved path text; the whole `chrys-rule` and `rule_gate` suites pass unchanged.

## `LoadedRules::mask_for`'s Key: Unaffected

Read directly, not assumed: `load_rules` (`crates/chrys-rule/src/lib.rs`) calls `mask::resolve_mask_path(rule_dir, mask_path)` to get the path it opens with `mask::load_mask`, but inserts into its own `masks: HashMap<PathBuf, mask::Mask>` with `masks.insert(mask_path.clone(), decoded)` — the rule row's own as-written path, never the resolved one. `mask_for` (same file) looks up by that same as-written value, from `Scope::Mask(mask_path)`. `resolve_mask_path`'s return value flows only into the `mask::load_mask` call; the map key never sees it.

## `scripts/engine-boundary-drill.sh`, The Whole Phase's Own Commit Range

First commit of the phase (plan 03-01's own first commit): `29f5add`. Last commit of the phase (this plan's own last commit): `3ca8ddc` (`3ca8ddccc4a4cb6fcefe84b036edb3ef447c7fda`).

`scripts/engine-boundary-drill.sh 29f5add 3ca8ddccc4a4cb6fcefe84b036edb3ef447c7fda`, full output (the resolved hash was required for `<last-commit>`; see Issues Encountered above for why the literal string `HEAD` gave a false failure on the first attempt):

```
engine-boundary-drill: drill one: checked range 3ca8ddccc4a4cb6fcefe84b036edb3ef447c7fda..39971c4c6ec6944ff755870161ae29b0001563f4
engine-boundary-drill: drill one: check output:
crates/chrys-core/src/lib.rs
drill ok: planted-defect drill: the check went red and named crates/chrys-core/src/lib.rs
engine-boundary-drill: drill two: checked range 29f5add^..3ca8ddccc4a4cb6fcefe84b036edb3ef447c7fda
engine-boundary-drill: drill two: check output:
(empty)
drill ok: clean-range drill: the check stayed silent over plan 02-03's own range

engine-boundary-drill: 2 of 2 drills behaved as expected
```

Both drills behaved as expected over the whole of phase 3's own commit range (`29f5add^..3ca8ddc`, all seventeen of phase 3's own task commits across plans 03-01 through 03-05). This closes the phase gate: no file under `crates/chrys-core/` moved anywhere in phase 3.

## Full Verification, This Plan's End State

```
cargo test -p chrys-cli --test region_rule: 4 passed, 0 failed.
cargo test -p chrys-baseline --test store: 9 passed, 0 failed.
cargo test -p chrys-rule: 40 passed, 0 failed (lib + mask.rs + rule_file.rs).
cargo test -p chrys-cli --test digest: 6 passed, 0 failed (CLI-03 unmoved).
cargo test -p chrys-cli --test region_hint: 2 passed, 0 failed (unmoved).
cargo test -p chrys-cli --test report: 5 passed, 0 failed (unmoved).
cargo test -p chrys-cli --test rule_gate: 2 passed, 0 failed (unmoved).
cargo test -p chrys-cli --test baseline: 3 passed, 0 failed (unmoved).
cargo test -p chrys-cli --test unsafe_guard: 1 passed, 0 failed.
cargo test -p chrys-core --test determinism: 6 passed, 0 failed (all six guards green).
cargo test --workspace: 261 passed, 0 failed (up from the 254 baseline this plan started from).
cargo clippy --workspace --all-targets -- -D warnings: clean.
cargo fmt --check: clean.
cargo tree -p chrys-core -e normal: no format crate, no GPU crate, no serde, no toml, no chrys-rule, no chrys-baseline.
git rev-parse HEAD:crates/chrys-core: c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 (unmoved, checked before this plan's first commit and after each of its four commits).
git status --porcelain chrys-baselines: empty (running the suite left the committed store untouched).
git ls-files --error-unmatch tests/golden/rule-01/masks/badge-cropped.png tests/golden/rule-01/mask-cropped-size.toml: both paths tracked.
scripts/engine-boundary-drill.sh 29f5add 3ca8ddc (the whole phase's own range): 2 of 2 drills behaved as expected.
```

## Predictions That Turned Out Wrong

- None of the plan's own predicted arithmetic or behaviour turned out wrong: the badge hint at (30, 30), the change at frame-global (40, 40), the reproduced crop-local (10, 10), and 30 + 10 = 40 all matched exactly as stated, both before and after the fix.
- One instruction needed correcting before it could be trusted, the same shape 03-04-SUMMARY.md already recorded once for its own drill: this plan's own literal first draft of `write_baseline`'s staged logic (a separate `write_baseline_staged` function) would have made the write-call guard flag every real write call as an offender, because the guard's own scan is scoped to the literal function named `write_baseline`. This was caught and fixed before either Task 2 drill was run, by merging the logic back into one function via a local closure. See Deviations.
- `scripts/engine-boundary-drill.sh`'s own argument handling has a gotcha this plan's own verification instructions did not warn about: passing the literal string `HEAD` as `<last-commit>` resolves inside the script's own disposable worktree, whose `HEAD` has by then moved past drill one's own planted commit, producing a false failure on drill two. Passing a resolved commit hash instead gives the correct result. See Issues Encountered.

## Next Phase Readiness

- CLI-01, CLI-02, BASE-04 and RULE-02 are all closed: the gate this phase exists to build no longer exits 0 on a real, untolerated change under `--region`, a report written with `--region` names the region it compared, an accept that fails destroys nothing, and the mask loader opens the exact path its own traversal check verified.
- Phase 3's own headline claim (D-03: the rule reader, the mask, the report and the baseline store all arrived without the engine learning that any of them exist) is unmoved and re-confirmed over the whole phase's own commit range by `scripts/engine-boundary-drill.sh`.
- `03-SECURITY.md`'s "Two decisions this phase settled before planning" section repeats the same overstated type-level claim about `Scope` that `Scope`'s own doc comment (IN-01) has now been corrected to avoid. This plan leaves that dated audit record as written, per its own instruction not to rewrite a signed record to match today; a human should decide whether to append a correction to that file's own corrections section.

## Self-Check: PASSED

All created files verified present on disk (`crates/chrys-cli/tests/region_rule.rs`, `tests/golden/rule-01/mask-cropped-size.toml`, `tests/golden/rule-01/masks/badge-cropped.png`); all four commit hashes (`986f663`, `4766b1d`, `4c16083`, `3ca8ddc`) verified present in `git log --oneline`; `git rev-parse HEAD:crates/chrys-core` verified reading `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` after the final commit; working tree verified clean (`git status --short`) after every disposable drill worktree was removed.

---
*Phase: 03-baseline-rules-and-ci-gate*
*Completed: 2026-09-08*
