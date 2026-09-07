---
phase: 03-baseline-rules-and-ci-gate
plan: 02
subsystem: rule-engine
tags: [rust, mask, png, path-traversal, ci-gate]

# Dependency graph
requires:
  - phase: 03-baseline-rules-and-ci-gate
    provides: "plan 03-01's chrys_rule::load_rules, Scope (with Scope::Mask(PathBuf) already a valid, unresolved variant), LoadedRules, and evaluate, plus the engine tree object id c97a6fb778c4b1373e5c4dc563481cc18e4c98c0"
provides:
  - "chrys_rule::mask::load_mask and chrys_rule::mask::Mask: a mask image decoded through chrys_source_raster::decode::decode_guarded, reduced to one boolean per pixel by the white-and-opaque membership rule (D-02), with Mask::tolerates(bbox) using the same majority-overlap convention overlapping_hint_name already uses for a named region"
  - "chrys_rule::mask::resolve_mask_path (pub(crate)): a lexical check (absolute path, any parent-directory component) followed by a canonical check (resolved path must sit inside the rule file's own canonical directory), so a mask path cannot leave the rule file's own directory (T-03-08)"
  - "load_rules resolves, safety-checks and decodes every Scope::Mask path while the rule file loads, storing decoded masks in LoadedRules keyed by the exact PathBuf a rule carries, so a broken rule file fails before any comparison runs (T-03-12)"
  - "evaluate(verdict, hints, rules, frame_size) -> Result<RuleOutcome, RuleError>: a mask-scoped rule now matches a region via Mask::tolerates, and a mask whose size differs from the frame's is refused (RuleError::MaskSizeMismatch, naming both sizes) rather than scaled, cropped or padded (T-03-11)"
  - "tests/golden/rule-01/masks/{badge.png,white-on-transparency.png} and mask-{tolerate,transparent}.toml: the committed proof that a white-on-transparency mask tolerates nothing while an equivalent white-and-opaque mask tolerates the same recoloured region a named region already does"
  - "crates/chrys-rule/tests/mask.rs: six path-safety and decode-limit cases over load_rules and load_mask"
  - "examples/rules/example.toml and examples/rules/masks/scrollbar-track.png: RULE-04's shipped example, a region-scoped rule and a mask-scoped rule, no unscoped rule and no global threshold"
affects: []

# Actuals (#2632)
actuals:
  tokens: 14369
  tasks: 3
  commits: 3
  plan_head_before: 4f5d2ae66a4200eed3d76957660b3b77c261de94

# Tech tracking
tech-stack:
  added:
    - "chrys-rule now depends on chrys-source-raster by path (workspace crate, no new external package), the same crate the CLI and every other adapter already reads a raster file through"
  patterns:
    - "A mask decodes through the one guarded entry point every other raster decode in this project already uses (decode_guarded + normalize_to_rgba8), never through a second reader chrys-rule opens itself — the same adapter-never-opens-its-own-decoder rule phase 2 recorded."
    - "A path field inside structured input gets two independent checks, not one: a lexical check (absolute, any `..` component) that refuses before any file is touched, and a canonical check (resolved path must sit inside the parent directory's own canonicalized form) that catches a symbolic link the lexical check cannot see. The same defence-in-depth pairing phase 1 recorded for a source-level guard plus one that depends on the environment."
    - "A mask's own decoded pixels live in a LoadedRules field keyed by the exact PathBuf a Scope::Mask rule carries, not inside Scope::Mask itself — Scope stays a lightweight, directly-comparable enum, and evaluate looks the decoded mask up by that same key."
    - "evaluate's own signature grew a frame_size parameter and its return type became Result<RuleOutcome, RuleError>, a deviation from plan 03-01's own next-phase-readiness note (which predicted evaluate's signature would not change). See Deviations."
  removed: []

key-files:
  created:
    - crates/chrys-rule/src/mask.rs
    - crates/chrys-rule/tests/mask.rs
    - examples/rules/example.toml
    - examples/rules/masks/scrollbar-track.png
    - tests/golden/rule-01/masks/badge.png
    - tests/golden/rule-01/masks/white-on-transparency.png
    - tests/golden/rule-01/mask-tolerate.toml
    - tests/golden/rule-01/mask-transparent.toml
  modified:
    - crates/chrys-rule/Cargo.toml
    - crates/chrys-rule/src/lib.rs
    - crates/chrys-rule/src/evaluate.rs
    - crates/chrys-rule/tests/rule_file.rs
    - crates/chrys-cli/src/main.rs
    - crates/chrys-source-raster/examples/make-fixtures.rs
    - Cargo.lock

key-decisions:
  - "evaluate's signature changed from `evaluate(verdict, hints, rules) -> RuleOutcome` to `evaluate(verdict, hints, rules, frame_size) -> Result<RuleOutcome, RuleError>`, and crates/chrys-cli/src/main.rs (not listed in this plan's own files_modified) was updated to pass the frame's size and propagate the new Result via `?`. See Deviations for why this was necessary rather than optional."
  - "The mask-resolution split matches this plan's own two-task structure: Task 1's load_rules resolves a mask path with a plain `rule_dir.join(mask_path)` (no safety check yet); Task 2 replaces that call with `mask::resolve_mask_path`, which adds the lexical and canonical checks. Task 1's own commit and verify pass without the safety check, exactly as the plan's task split implies."
  - "A mask-scoped rule's decoded pixels are stored in a new private LoadedRules field (`masks: HashMap<PathBuf, mask::Mask>`), not inside Scope::Mask's own PathBuf variant, so Scope keeps its existing shape and every one of plan 03-01's own Scope-constructing tests keeps compiling unchanged."
  - "Mask::tolerates clamps a bounding box to the mask's own width and height for the tolerated-pixel count, but never for the denominator (the box's own full area), so a box that is only partly inside the mask can never be tolerated by pixels outside the mask's own bounds. This is the same wording the plan's own action text specifies, implemented literally."
  - "image was added as a dev-dependency (not a normal dependency) of chrys-rule, used only to construct temporary test-fixture PNGs (in mask.rs's, evaluate.rs's and the new tests/mask.rs's own test modules). cargo tree -p chrys-core -e normal is unaffected, since dev-dependencies never appear in a normal-edge dependency tree, and chrys-rule was never itself a dependency chrys-core reads."

requirements-completed: [RULE-02, RULE-03, RULE-04, CLI-04]

coverage:
  - id: D1
    description: "A rule scoped to a mask image tolerates a change inside the mask's own white-and-opaque drawn area, and does not tolerate a change outside it. Mask::tolerates uses the same majority-overlap (at least half the box's own area) convention overlapping_hint_name already uses for a named region, in pure u64 integer arithmetic."
    requirement: RULE-02
    verification:
      - kind: unit
        ref: "crates/chrys-rule/src/evaluate.rs#tests::mask_scoped_rule_matches_overlapping_region"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/src/evaluate.rs#tests::mask_scoped_rule_does_not_match_a_region_over_untolerated_pixels"
        status: pass
      - kind: integration
        ref: "manual run recorded verbatim below: tests/golden/rule-01/mask-tolerate.toml exits 0 over the committed rule-01 pair"
        status: pass
    human_judgment: false
  - id: D2
    description: "A mask pixel counts as tolerated only when its red, green and blue bytes are all at or above 250 and its alpha byte equals 255 exactly (D-02). A mask drawn as white on transparency (255, 255, 255, 0 everywhere) tolerates nothing, proven by a committed fixture rather than believed."
    requirement: RULE-02
    verification:
      - kind: unit
        ref: "crates/chrys-rule/src/mask.rs#tests::a_white_pixel_that_is_not_opaque_is_not_tolerated"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/src/mask.rs#tests::a_white_and_opaque_pixel_is_tolerated"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/src/mask.rs#tests::a_channel_just_below_the_floor_is_not_tolerated"
        status: pass
      - kind: integration
        ref: "manual run recorded verbatim below: tests/golden/rule-01/mask-transparent.toml exits 1 over the same pair, because the mask is white everywhere and opaque nowhere"
        status: pass
    human_judgment: false
  - id: D3
    description: "A mask path that leaves the rule file's own directory is refused before any byte of it is read: an absolute path, a path holding any parent-directory component (a single one or a multi-segment traversal shaped like ../../etc/passwd), and a symbolic link that reaches outside through a name that is itself neither absolute nor holding a `..` component. A path naming a file in a subdirectory below the rule file is accepted."
    requirement: (T-03-08, this plan's own threat register)
    verification:
      - kind: integration
        ref: "crates/chrys-rule/tests/mask.rs#a_mask_path_that_leaves_the_rule_file_directory_is_refused"
        status: pass
      - kind: integration
        ref: "crates/chrys-rule/tests/mask.rs#a_mask_path_with_a_single_leading_parent_component_is_refused"
        status: pass
      - kind: integration
        ref: "crates/chrys-rule/tests/mask.rs#an_absolute_mask_path_is_refused"
        status: pass
      - kind: integration
        ref: "crates/chrys-rule/tests/mask.rs#a_mask_path_that_reaches_outside_through_a_symbolic_link_is_refused"
        status: pass
      - kind: integration
        ref: "crates/chrys-rule/tests/mask.rs#a_mask_path_in_a_subdirectory_below_the_rule_file_is_accepted"
        status: pass
    human_judgment: false
  - id: D4
    description: "A mask decodes through decode_guarded with an explicit DecodeLimits, and a mask above a supplied limit is refused by that guarded decoder, not by a second limit check written in chrys-rule (CLI-04)."
    requirement: CLI-04
    verification:
      - kind: integration
        ref: "crates/chrys-rule/tests/mask.rs#a_mask_above_the_decode_limits_is_refused"
        status: pass
    human_judgment: false
  - id: D5
    description: "The shipped examples/rules/example.toml holds a region-scoped rule and a mask-scoped rule, parses through load_rules, and names no unscoped rule; its own mask (masks/scrollbar-track.png) exists beside it and decodes."
    requirement: RULE-04
    verification:
      - kind: integration
        ref: "crates/chrys-rule/tests/rule_file.rs#the_shipped_example_rule_file_parses_and_is_fully_scoped"
        status: pass
      - kind: other
        ref: "human check: examples/rules/example.toml read top to bottom answers what a rule applies to, how a region name is made to exist, and which mask pixels count, with no second file needed — recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D6
    description: "No file under crates/chrys-core/ changes anywhere in this plan's commit range, and cargo tree -p chrys-core -e normal names neither chrys-rule, chrys-source-raster, serde, toml, nor any format crate."
    requirement: (D-03, project-wide invariant)
    verification:
      - kind: other
        ref: "git rev-parse HEAD:crates/chrys-core read c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 before this plan's first commit and after each of its three commits; cargo tree -p chrys-core -e normal (no serde, no toml, no chrys-rule, no chrys-source-raster, no format crate); scripts/engine-boundary-drill.sh 73f6275 69d282f (2 of 2 drills behaved as expected, recorded verbatim below)"
        status: pass
    human_judgment: false
  - id: D7
    description: "cargo test --workspace and cargo test -p chrys-core --test determinism (all six guards) stay green at every commit boundary; the workspace count only grows (219 to 233), never regresses."
    requirement: (CLI-03's regression guard, project-wide invariant)
    verification:
      - kind: other
        ref: "cargo test --workspace: 233 passed, 0 failed, at HEAD (69d282f); cargo test -p chrys-core --test determinism: 6 passed, 0 failed"
        status: pass
    human_judgment: false

duration: not captured (PLAN_START_TIME was not recorded at launch; see git log for the three task commits' own timestamps)
completed: 2026-09-07
status: complete
---

# Phase 3 Plan 02: Scope a tolerance to a mask of white opaque pixels Summary

**A rule can now name a mask image instead of a named region: the mask decodes through the project's one guarded raster entry point, a pixel counts as tolerated only when it is white on every channel and fully opaque (never white alone), a mask path cannot leave the rule file's own directory by two independent checks, and the shipped `examples/rules/example.toml` demonstrates both scope kinds with no unscoped rule anywhere.**

## Performance

- **Duration:** not captured (see frontmatter note)
- **Completed:** 2026-09-07
- **Tasks:** 3 / 3
- **Files modified:** 15 (7 modified, 8 new)

## Accomplishments

- Created `crates/chrys-rule/src/mask.rs`: `pub fn load_mask(path, limits) -> Result<Mask, RuleError>` calls `chrys_source_raster::decode::decode_guarded` and `chrys_source_raster::normalize::normalize_to_rgba8` and opens no reader of its own. `Mask` holds one `bool` per pixel (`true` when the pixel's RGB bytes are all at or above `TOLERATED_CHANNEL_FLOOR` (250) AND its alpha byte equals `TOLERATED_ALPHA` (255) exactly, D-02) plus its own width and height. `Mask::tolerates(bbox)` counts tolerated pixels inside `bbox`, clamped to the mask's own bounds for the numerator only, and returns true when that count is at least half `bbox`'s own full area — the same majority rule `overlapping_hint_name` already uses, in `u64` integer arithmetic, no `f32`/`f64` anywhere in `mask.rs`.
- `load_rules` (in `lib.rs`) now resolves and decodes every `Scope::Mask` path while the rule file loads: it walks the validated `rules` list, and for each `Scope::Mask(path)` it has not already decoded, resolves the path against the rule file's own directory, decodes it, and stores the result in a new private `LoadedRules::masks: HashMap<PathBuf, mask::Mask>` field, keyed by the exact `PathBuf` the rule carries. A rule file naming a missing, unreadable, oversized or path-unsafe mask now fails inside `load_rules`, before any comparison runs (T-03-12).
- `evaluate` (in `evaluate.rs`) gained a `frame_size: (u32, u32)` parameter and its return type changed to `Result<RuleOutcome, RuleError>`. A mask-scoped rule now matches a region when `Mask::tolerates(region.bbox)` is true, after checking the mask's own width and height against `frame_size`; a mismatch returns `RuleError::MaskSizeMismatch`, naming both sizes, rather than scaling, cropping or padding the mask to fit (T-03-11). `crates/chrys-cli/src/main.rs` was updated to pass each frame's own size and propagate the new `Result` via `?` — a change this plan's own `files_modified` list did not name, but one the plan's own verify commands (the two `chrys compare --rule` runs) could not pass without. See Deviations.
- Task 2 added `mask::resolve_mask_path(rule_dir, mask_path) -> Result<PathBuf, RuleError>`: a lexical check (refuses an absolute path or any parent-directory component, before any file is touched) followed by a canonical check (`rule_dir` and the joined path are both canonicalized, and the mask's canonical path must sit inside the directory's canonical path), catching a symbolic link the lexical check alone cannot see. `RuleError::MaskPathEscapesDirectory { rule_dir, mask_path }` carries both the boundary and what crossed it. `load_rules` was updated to call this instead of a plain `rule_dir.join(mask_path)`.
- `crates/chrys-rule/tests/mask.rs` (new): six integration tests, each building its own temporary directory — a multi-segment traversal (`../../etc/passwd`), a single leading `..` component, an absolute path, a symbolic link pointing outside the rule file's directory (ran to completion, not skipped, on this machine), an accepted path in a subdirectory below the rule file, and a mask above a caller-supplied `DecodeLimits` refused by `load_mask` itself.
- Extended `write_rule_01()` in `crates/chrys-source-raster/examples/make-fixtures.rs` with `write_rule_01_masks()`: writes `tests/golden/rule-01/masks/badge.png` (white opaque over the same `RULE_01_REGION` the hints sidecar already names) and `masks/white-on-transparency.png` (white on every channel, zero alpha, everywhere — the concrete failure D-02 exists to remove), plus `mask-tolerate.toml` and `mask-transparent.toml` (the same measured `max_delta_e = 116.0` `tolerate.toml` already records, scoped by each mask in turn).
- Ran both mask rule files over the committed `rule-01` pair and read the real exit codes (recorded verbatim below): `mask-tolerate.toml` exits 0, `mask-transparent.toml` exits 1, over the identical `Recoloured` verdict (colour delta 115.65) plan 03-01 already measured.
- Created `examples/rules/example.toml` (RULE-04's shipped example): a header paragraph stating the schema has no whole-frame rule shape and why, a region-scoped rule (`recoloured`, `region = "badge"`, with a comment showing the `<stem>.hints.toml` sidecar shape that makes the region name exist), and a mask-scoped rule (`moved`, `mask = "masks/scrollbar-track.png"`, with a comment stating the white-and-opaque convention and its white-on-transparency failure case plainly). Added `write_example_rules_mask()` to `make-fixtures.rs`, writing `examples/rules/masks/scrollbar-track.png` (a narrow vertical white opaque stripe on transparent black), resolved via a new `examples_rules_root()` helper built the same way `golden_root()` is (from `CARGO_MANIFEST_DIR`, not the process's current directory).
- Added `the_shipped_example_rule_file_parses_and_is_fully_scoped` to `crates/chrys-rule/tests/rule_file.rs`: loads the committed example and asserts only that it holds at least two rules and that each one's `scope` matches `Scope::Region` or `Scope::Mask` (D-01 already makes a third case unrepresentable) — no tolerance value, region name or exact rule count asserted, so an author improving the example's wording never has to update this test.
- `git rev-parse HEAD:crates/chrys-core` read `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` before this plan's first commit and after each of its three commits. `cargo tree -p chrys-core -e normal` names neither `serde`, `toml`, `chrys-rule`, `chrys-source-raster`, nor any format crate. `scripts/engine-boundary-drill.sh 73f6275 69d282f` ran both drills and both behaved as expected (recorded verbatim below).
- `cargo test --workspace` (233 passed, 0 failed, up from the 219 baseline this plan started from), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --check` all pass, at every commit boundary. `cargo test -p chrys-core --test determinism` passes with all six guards.

## Task Commits

Each task was committed atomically:

1. **Task 1: Scope a tolerance to a mask of white opaque pixels** — `73f6275`
2. **Task 2: Refuse a mask path that leaves the rule file's directory** — `0c61a15`
3. **Task 3: Ship an example rule file that scopes every exclusion** — `69d282f`

**Plan metadata:** pending (this SUMMARY and STATE.md/ROADMAP.md updates are committed by the orchestrator, per this plan's execution instructions)

## Files Created/Modified

- `crates/chrys-rule/Cargo.toml` — added `chrys-source-raster` (path dependency) and `image` (dev-dependency, test fixtures only)
- `crates/chrys-rule/src/mask.rs` — new: `load_mask`, `Mask`, `TOLERATED_CHANNEL_FLOOR`, `TOLERATED_ALPHA`, `resolve_mask_path` (`pub(crate)`), and 6 unit tests
- `crates/chrys-rule/src/lib.rs` — `pub mod mask;`, `RuleError::{MaskDecode, MaskSizeMismatch, MaskPathEscapesDirectory}`, `LoadedRules::masks` field and `mask_for` lookup, `load_rules` resolves/decodes every mask
- `crates/chrys-rule/src/evaluate.rs` — `evaluate`'s signature gained `frame_size` and now returns `Result<RuleOutcome, RuleError>`; `rule_tolerates` reads a mask-scoped rule's decoded mask from `LoadedRules` and checks its size; 3 new unit tests
- `crates/chrys-rule/tests/mask.rs` — new: 6 integration tests over path safety and decode limits
- `crates/chrys-rule/tests/rule_file.rs` — added `the_shipped_example_rule_file_parses_and_is_fully_scoped`
- `crates/chrys-cli/src/main.rs` — `evaluate` call site updated to pass each frame's own size and propagate the new `Result`
- `crates/chrys-source-raster/examples/make-fixtures.rs` — `write_rule_01_masks()`, `write_example_rules_mask()`, `examples_rules_root()`
- `Cargo.lock` — `image` added to `chrys-rule`'s own dev-dependency edge; no new external crate versions
- `tests/golden/rule-01/masks/{badge.png,white-on-transparency.png}`, `mask-{tolerate,transparent}.toml` — new committed fixtures
- `examples/rules/example.toml`, `examples/rules/masks/scrollbar-track.png` — new shipped example

## Decisions Made

- `evaluate`'s signature changed (`frame_size` parameter added, return type became `Result<RuleOutcome, RuleError>`), and `crates/chrys-cli/src/main.rs` was updated accordingly, even though neither is in this plan's own `files_modified` list. See Deviations.
- The path-safety check was built in Task 2, not Task 1: Task 1's `load_rules` resolves a mask path with a plain `rule_dir.join(mask_path)`; Task 2 replaces that call with `mask::resolve_mask_path`. This matches the plan's own two-task split and both tasks' own commits pass their own verify commands independently.
- A mask's decoded pixels live in a new `LoadedRules::masks: HashMap<PathBuf, mask::Mask>` field, not inside `Scope::Mask`'s own `PathBuf` variant, so `Scope`'s shape (and every existing test constructing it) is unchanged.
- `image` was added as a `chrys-rule` dev-dependency (not a normal one), used only inside `#[cfg(test)]` modules to write temporary PNG fixtures; this has no effect on `cargo tree -p chrys-core -e normal`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `evaluate`'s signature changed, and `crates/chrys-cli/src/main.rs` was updated to match, though neither is named in this plan's `files_modified`**
- **Found during:** Task 1, while implementing the "Size agreement" behaviour the plan's own action text requires: "A mask whose size differs from the frame's is refused, with a message naming both sizes... The check needs the frame's size, so `evaluate` performs it when it applies a mask-scoped rule."
- **Issue:** `evaluate`'s existing signature (`evaluate(verdict, hints, rules) -> RuleOutcome`) has no way to receive a frame's size, and `RuleOutcome` has no representation for "refused" distinct from "violation." Plan 03-01's own next-phase-readiness note predicted `evaluate`'s signature would not need to change, but this plan's own Task 1 action text requires a check that literally cannot be implemented without either a new parameter or a new return shape. Task 1's own `<verify>` block runs the two `chrys compare --rule tests/golden/rule-01/mask-{tolerate,transparent}.toml` commands end to end through the real CLI binary, which meant `crates/chrys-cli/src/main.rs`'s own call site had to compile and behave correctly for Task 1 to be verifiable at all, even though `main.rs` is not in this plan's `files_modified` list.
- **Fix:** Changed `evaluate`'s signature to `evaluate(verdict, hints, rules, frame_size: (u32, u32)) -> Result<RuleOutcome, RuleError>`, added `RuleError::MaskSizeMismatch { mask_path, mask_width, mask_height, frame_width, frame_height }` (naming both sizes, per the plan's own words), and updated `crates/chrys-cli/src/main.rs`'s single call site to read each frame's own `(width, height)` from `base_frames` (already in scope) and propagate the `Result` via `.collect::<Result<Vec<_>, _>>()` and `.transpose()?` — `run_compare` already returns `anyhow::Result<ExitCode>`, so `RuleError` (already `thiserror::Error`) converts automatically.
- **Files modified:** `crates/chrys-rule/src/evaluate.rs`, `crates/chrys-rule/src/lib.rs` (both already in this plan's `files_modified`), `crates/chrys-cli/src/main.rs` (not listed).
- **Verification:** Both `chrys compare --rule tests/golden/rule-01/mask-tolerate.toml` (exit 0) and `--rule tests/golden/rule-01/mask-transparent.toml` (exit 1) run through the real built binary and match the plan's own predicted exit codes, recorded verbatim below. `cargo test --workspace` (233 passed, 0 failed) and `cargo clippy --workspace --all-targets -- -D warnings` both stay clean with `main.rs`'s new call site in place. A new unit test, `a_mask_sized_differently_from_the_frame_is_refused_naming_both_sizes`, proves the size-mismatch refusal directly and asserts the message contains both sizes.
- **Committed in:** `73f6275` (Task 1 commit).
- **Precedent:** The same shape of choice 03-01-SUMMARY.md recorded for its own Task 1/Task 2 schema split: production code that a task's own action text requires cannot always be confined to only the files a plan's frontmatter happened to list, when the task's own `<verify>` commands exercise the real, wired-up binary.

---

**Total deviations:** 1 auto-fixed (blocking, Rule 3). No scope creep beyond what Task 1's own action text and verify commands already required; every other file this plan touches is exactly the set its own `files_modified` list names.

## Issues Encountered

None beyond the deviation recorded above.

## User Setup Required

None — no external service configuration required.

## The Two Mask Rule Runs Over `tests/golden/rule-01`, Recorded Verbatim

`cargo run -q -p chrys-cli -- compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png --rule tests/golden/rule-01/mask-tolerate.toml`:

```
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 (base [200, 40, 40, 255], candidate [40, 200, 200, 255])
```

Exit code: `0`.

`cargo run -q -p chrys-cli -- compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png --rule tests/golden/rule-01/mask-transparent.toml`:

```
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 (base [200, 40, 40, 255], candidate [40, 200, 200, 255])
```

Exit code: `1`.

`mask-tolerate.toml` names `masks/badge.png`, which paints the same `(30, 30, 70, 60)` region the `badge` hints sidecar already names, fully white and opaque; the recoloured rectangle at `(40, 40, 50, 40)` sits entirely inside it, so every one of its 2000 pixels is tolerated and the rule matches. `mask-transparent.toml` names `masks/white-on-transparency.png`, which is white on every colour channel and has alpha 0 everywhere; zero pixels are tolerated anywhere on that mask, so the same recoloured region is a violation and the comparison exits 1. Both mask files carry the same `max_delta_e = 116.0` `tolerate.toml` already records from the real, measured colour delta of `115.65` — the mask, not the tolerance number, is what changes the answer between these two runs.

## The Symbolic-Link Test, This Machine

`a_mask_path_that_reaches_outside_through_a_symbolic_link_is_refused` ran to completion on this machine (macOS, unix `cfg`), not skipped: `std::os::unix::fs::symlink` succeeded, the rule file named the link, and `chrys_rule::load_rules` returned `RuleError::MaskPathEscapesDirectory`, exactly as the test asserts. No skip message was printed. The test's own skip path (a named `eprintln!` followed by an early `return`, never a silent pass) exists for a platform where creating a symbolic link needs a privilege the test process does not have, which this machine did not exercise.

## `scripts/engine-boundary-drill.sh`, This Plan's Own Commit Range

`scripts/engine-boundary-drill.sh 73f6275 69d282f`, full output:

```
engine-boundary-drill: drill one: checked range 69d282f99523ac29b9d93dc3c87774482c177165..2f336e7dadc93d00617d929c6f1d67618954229d
engine-boundary-drill: drill one: check output:
crates/chrys-core/src/lib.rs
drill ok: planted-defect drill: the check went red and named crates/chrys-core/src/lib.rs
engine-boundary-drill: drill two: checked range 73f6275^..69d282f
engine-boundary-drill: drill two: check output:
(empty)
drill ok: clean-range drill: the check stayed silent over plan 02-03's own range

engine-boundary-drill: 2 of 2 drills behaved as expected
```

Both drills behaved as expected: the planted-defect drill went red and named the exact file planted, and the clean-range drill stayed silent over this plan's own real commit range (`73f6275^..69d282f`, all three of this plan's task commits). The working tree was clean before and after the drill ran; `git rev-parse HEAD:crates/chrys-core` still reads `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` afterward.

## The Human Check: `examples/rules/example.toml` Read Top to Bottom

Reading only `examples/rules/example.toml`, with no other file open, the three required questions are each answered directly:

1. **What does a rule apply to?** The header paragraph (lines 3-9) states a rule always names a scope, a named region or a mask image, and that a rule cannot apply to a whole frame, explaining why: a whole-frame rule could hide a regression under one generous threshold.
2. **How does a person make a region name exist?** The first rule's comment (lines 13-23) states plainly that a producer writes a `<stem>.hints.toml` file beside its image, naming the rectangle, and shows the exact `[[hint]]` table shape with the same name (`"badge"`) the rule itself uses.
3. **Which pixels of a mask count?** The second rule's comment (lines 38-47) states a pixel counts as tolerated only when it is white on every colour channel and fully opaque, and states the white-on-transparency failure case by name: such a mask is white everywhere and opaque nowhere, and tolerates nothing at all.

All three answers came from the file alone; no second file was needed for any of them.

## Full Verification, This Plan's End State

```
cargo test -p chrys-rule: 22 (lib, incl. mask.rs + evaluate.rs mask tests) + 6 (tests/mask.rs) + 11 (tests/rule_file.rs) = 39 passed, 0 failed.
cargo test --workspace: 233 passed, 0 failed (up from the 219 baseline this plan started from; +14 new tests: 4 in mask.rs, 3 in evaluate.rs, 6 in tests/mask.rs, 1 in tests/rule_file.rs).
cargo test -p chrys-core --test determinism: 6 passed, 0 failed (all six guards green).
cargo clippy --workspace --all-targets -- -D warnings: clean.
cargo fmt --check: clean.
cargo tree -p chrys-core -e normal: no serde, no toml, no chrys-rule, no chrys-source-raster, no format/GPU crate.
git rev-parse HEAD:crates/chrys-core: c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 (unmoved, checked before the first commit and after each of the three commits).
scripts/engine-boundary-drill.sh 73f6275 69d282f: 2 of 2 drills behaved as expected.
```

## Predictions That Turned Out Wrong

- Plan 03-01's own next-phase-readiness note predicted `evaluate`'s signature would not need to change once masks were wired in. This plan's own Task 1 action text ("the check needs the frame's size, so `evaluate` performs it") required exactly the change that note predicted would not happen: a new `frame_size` parameter and a `Result` return type, with a corresponding update to `crates/chrys-cli/src/main.rs`'s call site. See Deviations for the full reasoning; every other prediction and instruction in this plan (the D-02 membership rule, the path-traversal check shape, the fixture layout, the example's own two-rule structure) matched exactly.

## Next Phase Readiness

- RULE-02 (both halves, region and mask), RULE-03 (the schema still holds no computable field; `mask` is a plain `PathBuf`), RULE-04 (both halves: the type-level scope guard from 03-01, and this plan's shipped `examples/rules/example.toml`), and CLI-04 (the mask decode carries the same guarded entry point and the same limits every other decode already uses) are all delivered and observable.
- `chrys-core`'s tree object id (`c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`) and dependency graph (no `serde`, no `toml`, no `chrys-rule`, no `chrys-source-raster`) are unmoved, checked at every commit boundary and by the engine-boundary drill over this plan's own commit range.
- `evaluate`'s new `Result<RuleOutcome, RuleError>` return type and `frame_size` parameter are the shape any later plan reading a `Verdict` against a rule set must now match; `crates/chrys-cli/src/main.rs`'s own call site is the reference implementation for how to supply `frame_size` and propagate the `Result`.

## Self-Check: PASSED

All created files verified present on disk (`crates/chrys-rule/src/mask.rs`, `crates/chrys-rule/tests/mask.rs`, `crates/chrys-rule/tests/rule_file.rs`, `crates/chrys-cli/src/main.rs`, `crates/chrys-source-raster/examples/make-fixtures.rs`, `examples/rules/example.toml`, `examples/rules/masks/scrollbar-track.png`, `tests/golden/rule-01/masks/badge.png`, `tests/golden/rule-01/masks/white-on-transparency.png`, `tests/golden/rule-01/mask-tolerate.toml`, `tests/golden/rule-01/mask-transparent.toml`); all three task commit hashes (`73f6275`, `0c61a15`, `69d282f`) verified present in `git log --oneline`; `git rev-parse HEAD:crates/chrys-core` verified reading `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` after the final commit.

---
*Phase: 03-baseline-rules-and-ci-gate*
*Completed: 2026-09-07*
