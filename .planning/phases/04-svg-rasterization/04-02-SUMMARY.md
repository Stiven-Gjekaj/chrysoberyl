---
phase: 04-svg-rasterization
plan: 02
subsystem: svg-adapter
tags: [rust, svg, resvg, cargo-tree, dependency-graph, determinism, guard]

# Dependency graph
requires:
  - phase: 04-svg-rasterization
    provides: "04-01's chrys-source-svg crate: SvgSource, SvgLimits, SvgError, the resvg pin (default-features = false, features = [\"text\"]) in the workspace Cargo.toml, and the engine-boundary anchor c97a6fb778c4b1373e5c4dc563481cc18e4c98c0"
provides:
  - "SvgSource::load refuses an oversized declared file, and a bounded_read function factored out of it so the read-time bound is callable and testable apart from the metadata check"
  - "Two size-refusal tests and a boundary case proving the canvas refusal at max_width = 1"
  - "crates/chrys-cli/tests/svg_boundary_guard.rs: the resolved-feature-graph guard (only_the_text_feature_of_resvg_is_enabled) and the engine-dependency-graph guard (no_svg_crate_enters_chrys_cores_dependency_graph)"
  - "crates/chrys-cli/tests/system_font_guard.rs: a workspace-wide source-level search for load_system_fonts, independent of the Cargo-feature guard"
affects: ["04-svg-rasterization/04-03"]

# Actuals (#2632)
actuals:
  tokens: 7934
  tasks: 3
  commits: 4
  plan_head_before: 06c59de157d820008e9db390c8d10eea66eabe28

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A guard that reads cargo tree output fails, never skips, when the command cannot run or exits non-zero: run_cargo (svg_boundary_guard.rs) and the file-walk floor (system_font_guard.rs) both panic rather than return an empty result, mirroring determinism.rs's own no_gpu_or_format_crate_enters_chrys_cores_dependency_graph."
    - "A search needle that must survive a string-and-comment stripping pass, in the one file that legitimately names it, is generated with stringify!(bare_identifier) rather than typed as a quoted literal: the identifier is ordinary code, not a string, so it is not itself erased by the same pass it feeds."
    - "A memory-risking drill (an unbounded canvas allocation) is run as a compiled test binary directly, under a caller-managed timeout with a hard kill, rather than through a shell timeout/ulimit -v, because macOS does not implement RLIMIT_AS and ships no timeout(1); the drill's own outcome (does not complete, killed) is recorded as what happened, per the plan's own instruction not to smooth an OOM-shaped result into a tidy assertion failure."
  removed: []

key-files:
  created:
    - crates/chrys-cli/tests/svg_boundary_guard.rs
    - crates/chrys-cli/tests/system_font_guard.rs
  modified:
    - crates/chrys-source-svg/src/lib.rs
    - crates/chrys-source-svg/tests/svg.rs

key-decisions:
  - "The limits-agreement unit test this plan's Task 1 specifies (SvgLimits::default() vs DecodeLimits::default(), plus max_file_bytes = 8 MiB) was already committed by plan 04-01 (commit 9b6a5db), in the exact shape this plan's action text asks for. No duplicate test was added; Task 1's own work here is the two size-refusal tests, the canvas-refusal tests, and factoring bounded_read out of SvgSource::load so it is directly callable."
  - "only_the_text_feature_of_resvg_is_enabled checks the resolved cargo tree -e features graph before the workspace-manifest text, the reverse of the order the plan's own prose describes the two layers in. Restoring resvg's default features in the manifest trips both layers at once; checking the manifest line first would have made the drill's own failure message name a missing declaration rather than the actual forbidden feature that came back (system-fonts), which the plan's own <behavior> line requires the message to name."
  - "Task 3's own deliberate deviation from 04-VALIDATION.md's two suggested homes for the source-level guard (extending crates/chrys-core/tests/determinism.rs, or adding a chrys-source-svg-only search) is recorded in system_font_guard.rs's own module header, exactly as the plan instructs, not only here."

requirements-completed: [SRC-04, DET-05]

coverage:
  - id: D1
    description: "An SVG file larger than SvgLimits::max_file_bytes is refused with a typed error naming the file's size and the limit, and the refusal does not depend on the file's declared metadata length alone."
    requirement: SRC-04
    verification:
      - kind: unit
        ref: "crates/chrys-source-svg/tests/svg.rs#an_oversized_file_is_refused_before_parsing"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-svg/tests/svg.rs#the_bounded_read_refuses_more_than_its_cap_without_consulting_metadata"
        status: pass
      - kind: other
        ref: "planted-defect drill in a disposable worktree, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D2
    description: "An SVG declaring a canvas above SvgLimits is refused with a typed error naming the width, the height and the limit, before any pixel buffer is allocated, including the boundary case at max_width = 1."
    requirement: SRC-04
    verification:
      - kind: unit
        ref: "crates/chrys-source-svg/tests/svg.rs#an_oversized_declared_canvas_is_refused"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-svg/tests/svg.rs#a_max_width_of_one_refuses_the_committed_fixture"
        status: pass
      - kind: other
        ref: "planted-defect drill in a disposable worktree, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D3
    description: "SvgLimits::default()'s width, height and allocation numbers equal DecodeLimits::default()'s, asserted rather than restated (already committed by plan 04-01; reconfirmed still passing here)."
    requirement: SRC-04
    verification:
      - kind: unit
        ref: "crates/chrys-source-svg/src/lib.rs#tests::default_limits_match_the_raster_crates_own_numbers, #tests::default_max_file_bytes_is_eight_mebibytes"
        status: pass
    human_judgment: false
  - id: D4
    description: "resvg resolves with only its text feature: the resolved feature graph of chrys-source-svg names text and names neither system-fonts nor memmap-fonts."
    requirement: DET-05
    verification:
      - kind: unit
        ref: "crates/chrys-cli/tests/svg_boundary_guard.rs#only_the_text_feature_of_resvg_is_enabled"
        status: pass
      - kind: other
        ref: "planted-defect drill in a disposable worktree, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D5
    description: "No SVG crate enters chrys-core's own dependency graph, checked by a guard that lives outside crates/chrys-core/."
    requirement: DET-05
    verification:
      - kind: unit
        ref: "crates/chrys-cli/tests/svg_boundary_guard.rs#no_svg_crate_enters_chrys_cores_dependency_graph"
        status: pass
      - kind: other
        ref: "planted-defect drill in a disposable worktree, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D6
    description: "No source file in this workspace names load_system_fonts, outside the one file that names it as its own search constant, with a non-empty stated reason."
    requirement: DET-05
    verification:
      - kind: unit
        ref: "crates/chrys-cli/tests/system_font_guard.rs#no_source_file_in_the_workspace_names_the_host_font_loader, #every_allow_listed_file_names_its_own_reason"
        status: pass
      - kind: other
        ref: "planted-defect drill in a disposable worktree, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D7
    description: "Every guard this plan adds has been watched going red against a planted defect, and each red message names what was planted."
    requirement: (project-wide invariant)
    verification:
      - kind: other
        ref: "five drills, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D8
    description: "No file under crates/chrys-core/ changes anywhere in this plan's commit range."
    requirement: (D-03, project-wide invariant)
    verification:
      - kind: other
        ref: "git rev-parse HEAD:crates/chrys-core read c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 before this plan's first commit, after each task commit, and after every one of the five drills"
        status: pass
    human_judgment: false
  - id: D9
    description: "cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --check all pass; every already-committed fixture pair still exits 1 through the adapter it used before this plan."
    requirement: (project-wide invariant)
    verification:
      - kind: other
        ref: "cargo test --workspace: 0 failed; cargo clippy --workspace --all-targets -- -D warnings: clean; cargo fmt --check: clean; all eight committed fixture pairs (png, jpeg, webp, tiff, gif, apng, webp-anim, svg) re-run and recorded below"
        status: pass
    human_judgment: false

duration: not captured (PLAN_START_TIME was not recorded at launch)
completed: 2026-09-09
status: complete
---

# Phase 4 Plan 02: Bound the SVG adapter and prove the SVG stack cannot reach the engine or the host font database Summary

**Two size refusals proved and drilled on `SvgSource`, and two independent DET-05 guards — a resolved-Cargo-feature-graph check and a workspace-wide source search — each drilled red against a planted defect in a disposable worktree.**

```
first commit: dead932da44dad04ede7a62ff344f60536049ec8
last commit: c319c15044ce1421af08bca6345f6dd6d1db2973
```

## Performance

- **Duration:** not captured (see frontmatter note)
- **Completed:** 2026-09-09
- **Tasks:** 3 / 3 (four commits: Task 2 split into its own two commits, as its own action text specifies)
- **Files modified:** 4 (2 modified, 2 new)

## Accomplishments

### Task 1 — SRC-04's size refusals, proved and drilled

- Found that the limits-agreement unit test this task's action text asks for (`SvgLimits::default()`'s three shared numbers equal `DecodeLimits::default()`'s, and `max_file_bytes` is 8 MiB) was already committed by plan 04-01 in the exact shape requested. No duplicate was added; this task confirmed it still passes and moved on to the refusals themselves.
- Factored the bounded read out of `SvgSource::load` into a standalone `pub fn bounded_read(path, max_file_bytes)`, so it is callable — and testable — apart from the metadata check that runs before it.
- Added `an_oversized_file_is_refused_before_parsing`: `SvgSource::with_limits` with `max_file_bytes` one byte below `base.svg`'s own 1113-byte size refuses it, naming both the measured size and the limit.
- Added `the_bounded_read_refuses_more_than_its_cap_without_consulting_metadata`: calls `bounded_read` directly, once below the fixture's size (refused) and once above it (returns the file's whole content, byte-for-byte).
- Added `an_oversized_declared_canvas_is_refused`: a temporary SVG declaring `width="100000" height="100000"` is refused with the canvas variant, naming the declared width, height and the limit. **100000 produced the canvas variant on the first try** — no smaller or larger value had to be tried, and no value produced the parse variant instead.
- Added `a_max_width_of_one_refuses_the_committed_fixture`: `SvgSource::with_limits` with `max_width: 1` refuses the committed 256x256 fixture with the same canvas variant, a case that cannot depend on how `usvg` handles a very large declared number.
- Drilled both refusals (see "The Five Planted-Defect Drills" below).

### Task 2 — DET-05's compile-time half, two commits

- Commit 1, `Enable one feature of the SVG renderer and no other`: added `crates/chrys-cli/tests/svg_boundary_guard.rs` with `only_the_text_feature_of_resvg_is_enabled`. It checks the resolved `cargo tree -e features -p chrys-source-svg` graph first (names `text`, names neither `system-fonts` nor `memmap-fonts`), then the workspace `Cargo.toml`'s own `resvg` line (`default-features = false`, features include `"text"`). See Deviations for why the resolved-graph check runs first.
- Commit 2, `Deny every SVG crate in the engine dependency graph`: added `no_svg_crate_enters_chrys_cores_dependency_graph` to the same file. It runs `cargo tree -p chrys-core -e normal` and asserts none of `resvg`, `usvg`, `tiny-skia`, `fontdb`, `rustybuzz`, `ttf-parser` or `roxmltree` appear, and asserts the output does name `chrys-core` itself so an empty tree cannot read as a pass.
- Both guards fail, rather than skip, when their own `cargo tree` invocation cannot run or exits non-zero.
- Drilled both (see below).

### Task 3 — DET-05's source-level half, workspace-wide

- Added `crates/chrys-cli/tests/system_font_guard.rs`. `no_source_file_in_the_workspace_names_the_host_font_loader` walks every `.rs` file under every crate's `src`, `tests` and `examples` directories (sorted), strips comments and every string/byte-string/character literal from each (a third copy of the technique `decode_limits_guard.rs` already carries, for the same reason that file's own header gives: the original lives inside `crates/chrys-core/tests/`, which this phase may not edit), and searches the result for `load_system_fonts`.
- The search needle (`HOST_FONT_LOADER`) is generated with `stringify!(load_system_fonts)` rather than typed as a quoted string literal, so this guard's own file's one legitimate occurrence of the symbol is not itself erased by the same string-stripping pass it applies to every other file. See Deviations / key-decisions.
- The one-entry `ALLOW_LIST` names this guard's own file and a non-empty reason; `every_allow_listed_file_names_its_own_reason` asserts both the single-entry count and the non-empty reason.
- **The measured floor.** `find crates -type d \( -name src -o -name tests -o -name examples \) -exec find {} -name "*.rs" \; | wc -l` reported **68** files at authoring time. `MIN_FILES_VISITED` is set to **60**, a little below the measurement, per the plan's own instruction not to guess.
- Recorded, in the file's own module header, the deliberate deviation from `04-VALIDATION.md`'s two suggested homes for this guard (extending `determinism.rs`, or a `chrys-source-svg`-only search), exactly as the plan's action text requires.
- Drilled (see below).

## Task Commits

1. **Task 1: Bound the size of an SVG before it is parsed or drawn** — `dead932`
2. **Task 2a: Enable one feature of the SVG renderer and no other** — `1430157`
3. **Task 2b: Deny every SVG crate in the engine dependency graph** — `a6fefca`
4. **Task 3: Deny the host font loader across the workspace source** — `c319c15`

**Plan metadata:** pending (this SUMMARY is committed by this executor; STATE.md/ROADMAP.md updates are committed by the orchestrator per worktree mode).

## Files Created/Modified

See `key-files` in frontmatter.

## Decisions Made

See `key-decisions` in frontmatter.

## Deviations from Plan

### Auto-fixed / Adjusted

**1. [Rule 1-adjacent — corrected drill outcome] Reordered the two layers inside `only_the_text_feature_of_resvg_is_enabled` so the drill's own red message names the forbidden feature**
- **Found during:** Task 2, first drill run.
- **Issue:** The plan's own `<behavior>` line requires: "Restoring `resvg`'s default features turns the first guard red, and the red message names the feature that came back." With the manifest-text check (Layer 1, per the plan's own prose order) running before the resolved-feature-graph check (Layer 2), removing `default-features = false` from the workspace manifest tripped the manifest-text assertion first — its own message says only that the declaration is missing, and cannot name a specific feature, because a manifest-text check has no feature name to report.
- **Fix:** Reordered the test body to check the resolved `cargo tree -e features` graph first, then the manifest text. A drill restoring default features now fails on the graph check, whose message names `system-fonts` explicitly — the outcome the plan's own behavior line specifies.
- **Files modified:** `crates/chrys-cli/tests/svg_boundary_guard.rs`.
- **Verification:** re-drilled after the reorder; the new failure message reads "the resolved feature graph of chrys-source-svg names the `system-fonts` feature..." (recorded verbatim below).
- **Committed in:** `1430157` (Task 2a's own first-draft commit already carried the reordered version; no separate fix commit was needed since this was caught before either commit landed).

**2. [Documentation note, not a fix] Task 1's limits-agreement test already existed**
- **Found during:** Task 1, before writing any new test.
- **Issue:** The plan's action text asks for a unit test asserting `SvgLimits::default()`'s three shared numbers equal `DecodeLimits::default()`'s, and `max_file_bytes` is 8 MiB. `crates/chrys-source-svg/src/lib.rs`'s own `#[cfg(test)] mod tests` already carried `default_limits_match_the_raster_crates_own_numbers` and `default_max_file_bytes_is_eight_mebibytes`, committed by plan 04-01 (commit `9b6a5db`), matching the requested shape exactly.
- **Fix:** None needed; no duplicate test was written. Confirmed both still pass as part of this plan's own verification.
- **Files modified:** none for this item.
- **Committed in:** n/a (pre-existing).

---

**Total deviations:** 2 (1 test-ordering correction to match the plan's own specified drill outcome; 1 documentation note that a required test already existed). No architectural change; no scope beyond this plan's own stated files.

## Issues Encountered

**The second Task 1 drill (deleting the canvas check) could not fail cleanly, exactly as the plan's own action text anticipated.** With the canvas-size check removed, `SvgSource::load` attempts to allocate a `tiny_skia::Pixmap` sized 100000 x 100000 x 4 bytes (≈37 GiB) against a 16 GiB development machine. macOS implements neither `RLIMIT_AS` (`ulimit -v` fails with "setrlimit failed: invalid argument") nor ships a `timeout(1)` binary, so the drill was run as a pre-compiled test binary directly, under a Python-managed 25-second timeout with a hard `kill()` on expiry, rather than through a shell-level resource limit. The process did not complete within 25 seconds and was killed; `vm_stat`/`vm.swapusage` afterward showed no lingering memory or swap pressure, confirming the kill was clean and the host machine was not otherwise affected. This is recorded as the drill's actual outcome, per the plan's own instruction not to smooth an OOM-shaped result into a tidy assertion failure.

## User Setup Required

None — no external service configuration required.

## The Declared Canvas Size That Produced The Canvas Refusal

`width="100000" height="100000"` produced `SvgError::CanvasTooLarge` on the first attempt. No other value was tried: the plan's own instruction was to reduce the value only if the first attempt produced the parse variant instead, and it did not.

## The Measured File Count And The Chosen Floor

- **Measured** (independent shell count, matching the guard's own walk): `find crates -type d \( -name src -o -name tests -o -name examples \) -exec find {} -name "*.rs" \; | wc -l` → **68**.
- **Chosen floor** (`MIN_FILES_VISITED`): **60**.

## The Five Planted-Defect Drills, Recorded Verbatim

All five drills ran inside a disposable `git worktree add <path> HEAD`, each with the currently-uncommitted test/source changes for its own task copied in by hand (each drill ran before its own task's commit landed), removed with `git worktree remove --force <path>` on completion. `git rev-parse HEAD:crates/chrys-core` read `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` before this plan's first commit, after each task commit, and after every one of these five drills. The real working tree held no uncommitted changes before or after any drill.

**Drill 1 (Task 1): both file-size refusals deleted from `SvgSource::load`**, replaced with an unbounded `std::fs::read`, at `/tmp/chrys-filesize-drill`:

```
thread 'an_oversized_file_is_refused_before_parsing' panicked at crates/chrys-source-svg/tests/svg.rs:287:18:
expected SvgError::FileTooLarge, got Ok([Frame { pixels: [...], width: 256, height: 256, index: 0, hints: [] }])
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.02s
```

The test went red and its message names the variant it got instead (`Ok(Frame)`), exactly as the plan requires.

**Drill 2 (Task 1): the canvas-dimension and allocation-size checks deleted from `SvgSource::load`**, at `/tmp/chrys-canvas-drill`, running the compiled `an_oversized_declared_canvas_is_refused` test binary directly under a 25-second managed timeout (see Issues Encountered for why a shell timeout/`ulimit -v` was not available):

```
running 1 test
TIMED OUT after 25.00s (process killed)
```

The run did not fail cleanly with an assertion; it did not complete at all within the 25-second budget, consistent with the 100000x100000x4-byte (~37 GiB) allocation the deleted check exists to refuse, on a 16 GiB machine. `vm_stat`/`vm.swapusage` immediately afterward showed no lingering memory or swap pressure — the kill was clean. This is the outcome itself, recorded as it happened rather than smoothed into a tidy assertion failure, per the plan's own instruction.

**Drill 3 (Task 2, commit 1): `default-features = false` removed from the workspace `resvg` line**, at `/tmp/chrys-feature-drill`:

```
thread 'only_the_text_feature_of_resvg_is_enabled' panicked at crates/chrys-cli/tests/svg_boundary_guard.rs:108:5:
the resolved feature graph of chrys-source-svg names the `system-fonts` feature, which compiles the host-font-database reader into the binary (DET-05):
chrys-source-svg v0.1.0 (...)
[... cargo tree -e features output ...]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.05s
```

The test went red and its message names the exact feature that came back (`system-fonts`), matching the plan's own `<behavior>` line. See Deviations for why the test checks the resolved graph before the manifest text.

**Drill 4 (Task 2, commit 2): `resvg.workspace = true` added to `crates/chrys-core/Cargo.toml`'s own `[dependencies]`**, at `/tmp/chrys-engine-graph-drill`:

```
thread 'no_svg_crate_enters_chrys_cores_dependency_graph' panicked at crates/chrys-cli/tests/svg_boundary_guard.rs:174:5:
chrys-core's own dependency graph names a crate on the SVG-rasterization path (see this test file's own SVG_PATH_CRATES doc comment). Offending crate(s): fontdb, resvg, roxmltree, tiny-skia, usvg. The tree below shows the path that pulled each one in:
chrys-core v0.1.0 (...)
[... cargo tree output ...]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.05s
```

The test went red and its message names `resvg` and every transitive SVG-path crate the drill pulled in with it. Editing the engine's manifest inside the throwaway worktree did not move the real tree object id; confirmed above.

**Drill 5 (Task 3): a stub `fn load_system_fonts() {}` appended to `crates/chrys-source-svg/src/fonts.rs`**, at `/tmp/chrys-font-loader-drill`:

```
thread 'no_source_file_in_the_workspace_names_the_host_font_loader' panicked at crates/chrys-cli/tests/system_font_guard.rs:345:5:
the following source location(s) name `load_system_fonts`, the host-font-database loader DET-05 forbids, outside this guard's own one-entry allow-list:
crates/chrys-source-svg/src/fonts.rs:91
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.04s
```

The test went red and its message names both the file (`fonts.rs`) and the exact planted line (`91`), matching the plan's own requirement.

## Full Verification, This Plan's End State

```
cargo test --workspace: 0 failed (43 "test result: ok" lines across the workspace).
cargo clippy --workspace --all-targets -- -D warnings: clean.
cargo fmt --check: clean.
cargo test -p chrys-core --test determinism: 6 passed, 0 failed (all six guards green,
  unmoved by this plan).
cargo test -p chrys-cli --test unsafe_guard: 1 passed, floor at nine, unmoved.
cargo test -p chrys-cli --test decode_limits_guard: 2 passed, 0 failed, unmoved.
cargo test -p chrys-cli --test svg_boundary_guard: 2 passed, 0 failed.
cargo test -p chrys-cli --test system_font_guard: 2 passed, 0 failed.
cargo test -p chrys-source-svg --lib: 4 passed (the two limits-agreement tests from 04-01,
  the two fonts.rs tests).
cargo test -p chrys-source-svg --test svg: 8 passed (4 from 04-01, 4 new: the two file-size
  tests, the two canvas tests).
cargo tree -p chrys-source-svg -e normal: no chrys-source-raster, no png (both stay
  dev-only).
cargo tree -e features -p chrys-source-svg: names "text" (2 occurrences: resvg and usvg),
  names neither "system-fonts" nor "memmap-fonts".
cargo tree -p chrys-core -e normal: chrys-source, libm, palette (+ palette_derive,
  palette_math), rustfft, sha2, thiserror, and their own transitive dependencies only — no
  resvg, usvg, tiny-skia, fontdb, rustybuzz, ttf-parser, roxmltree, and no GPU or other
  format-decoding crate.
git rev-parse HEAD:crates/chrys-core: c97a6fb778c4b1373e5c4dc563481cc18e4c98c0, unmoved,
  checked before this plan's first commit, after each task commit, and after every one of
  the five drills.
Every committed fixture pair re-run through a fresh cargo build --release binary:
  png exit 1 (Recoloured region x=64,y=64,96x64), jpeg exit 1 (same region, JPEG rounding),
  webp exit 1 (same region), tiff exit 1 (same region), gif exit 1 (frame 3, 1 of 5 frames
  changed), apng exit 1 (frame 3, 1 of 5 frames changed), webp-anim exit 1 (frame 3, 1 of 5
  frames changed), svg exit 1 (Recoloured region x=192,y=16,48x48, colour delta 199.12,
  matching 04-01's own recorded run).
```

## Next Phase Readiness

- SRC-04's adapter now refuses both size dimensions the research flagged as unbounded upstream (file bytes and declared canvas), each refusal drilled red.
- DET-05 is enforced at two independent layers — the resolved Cargo feature graph and a workspace-wide source search — with a documented, drilled reason neither replaces the other.
- The four new crates (`resvg`, `usvg`, `tiny-skia`, `fontdb`, plus their own transitive tree) cannot reach `chrys-core`'s dependency graph without a red test, despite the engine's own `DENIED_DEPENDENCY_CRATES` list being unextendable this phase.
- `crates/chrys-core`'s tree object id is unmoved (`c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`), so plan 04-03's own phase gate starts from the same anchor this plan started from.
- This plan's own commit range (`dead932`..`c319c15`) is the range plan 04-03's phase gate reads.

## Self-Check: PASSED

All created files verified present on disk (`crates/chrys-cli/tests/svg_boundary_guard.rs`, `crates/chrys-cli/tests/system_font_guard.rs`); both modified files (`crates/chrys-source-svg/src/lib.rs`, `crates/chrys-source-svg/tests/svg.rs`) verified changed via `git diff --stat`; all four commit hashes (`dead932`, `1430157`, `a6fefca`, `c319c15`) verified present in `git log --oneline`; `git rev-parse HEAD:crates/chrys-core` verified reading `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` after the final commit; working tree verified clean (`git status --short`) after every one of the five disposable drill worktrees was removed.

---
*Phase: 04-svg-rasterization*
*Completed: 2026-09-09*
