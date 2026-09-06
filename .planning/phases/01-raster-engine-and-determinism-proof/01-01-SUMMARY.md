---
phase: 01-raster-engine-and-determinism-proof
plan: 01
subsystem: infra
tags: [rust, cargo-workspace, image, clap, thiserror, anyhow, cli]

# Dependency graph
requires: []
provides:
  - "Cargo workspace with resolver 3, pinned toolchain and four crates"
  - "chrys-source: Frame, RegionHint, Source trait, no format knowledge"
  - "chrys-source-raster: RasterSource decoding PNG with guarded limits"
  - "chrys-core: Verdict, Region, ChangeKind, compare(base, candidate)"
  - "chrys-cli: chrys compare BASE CANDIDATE with documented exit codes"
  - "tests/golden/pair-01: the first real PNG pair every later plan reads"
affects: [01-02, 01-03, 01-04, 01-05, 01-06, 01-07, 01-08]

# Actuals (#2632)
actuals:
  tokens: 9849
  tasks: 1
  commits: 7
plan_head_before: 62c23937677c41d5512b900ebba246b452aeb66a

# Tech tracking
tech-stack:
  added: [image=0.25.10, thiserror=2.0.20, anyhow=1.0.104, clap=4.6.6]
  patterns:
    - "Source trait boundary: only chrys-source-raster names a file format"
    - "chrys-core has zero format-crate and zero graphics-crate dependencies"
    - "Decode limits set on the reader before decode runs; a Limits error is resolved to exact dimensions by a header-only probe that never allocates a pixel buffer"

key-files:
  created:
    - Cargo.toml
    - rust-toolchain.toml
    - crates/chrys-source/src/lib.rs
    - crates/chrys-source-raster/src/lib.rs
    - crates/chrys-source-raster/examples/make-fixtures.rs
    - crates/chrys-core/src/lib.rs
    - crates/chrys-core/src/verdict.rs
    - crates/chrys-cli/src/main.rs
    - tests/golden/pair-01/base.png
    - tests/golden/pair-01/candidate.png
  modified:
    - .gitignore

key-decisions:
  - "All nine pinned crate versions from 01-RESEARCH.md were re-verified against the live registry today with cargo search and matched exactly. No version drifted, so no pin changed from the plan."
  - "The rust-toolchain.toml channel accepts edition 2024, confirmed by cargo build succeeding on rustc 1.97.1."
  - "A Limits error from image::ImageReader::decode is resolved to an exact TooLarge{width,height,limit} by a second, header-only probe (into_dimensions with no width/height limit set). This never decodes pixel data, so it stays safe against a crafted header, and it gives a precise error message instead of reporting zeros."
  - "chrys-core's unit tests build synthetic in-memory Frame values rather than loading the real PNG fixtures, so the engine crate never gains a dependency on chrys-source-raster, even in dev-dependencies. The real-pair path (Recoloured, matching bounding box) is proven by the plan's own CLI verify commands instead."

patterns-established:
  - "One change per commit, code and tests together, per AGENTS.md: workspace skeleton, then trait crate, then adapter, then engine, then CLI, then fixture pair, matching the plan's suggested commit order."

requirements-completed: [SRC-01, CORE-01, DET-04]

coverage:
  - id: D1
    description: "A person runs chrys compare on two real PNG files on disk and reads a verdict naming a change kind on stdout."
    requirement: "CORE-01"
    verification:
      - kind: e2e
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png | grep -qi 'recoloured'"
        status: pass
    human_judgment: false
  - id: D2
    description: "The command exits 1 on a differing pair and 0 on an identical pair."
    requirement: "CORE-01"
    verification:
      - kind: e2e
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png; test $? -eq 1"
        status: pass
      - kind: e2e
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/base.png; test $? -eq 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "The workspace builds on the pinned toolchain with a committed lock file, and chrys-core names no format or graphics crate."
    requirement: "DET-04"
    verification:
      - kind: unit
        ref: "cargo build --workspace"
        status: pass
      - kind: other
        ref: "crates/chrys-core/Cargo.toml [dependencies] lists only chrys-source and thiserror"
        status: pass
    human_judgment: false
  - id: D4
    description: "RasterSource::load decodes a raster file with limits set before decode runs, and never panics on a non-image file."
    requirement: "SRC-01"
    verification:
      - kind: unit
        ref: "crates/chrys-source-raster/src/lib.rs#tests::load_on_a_non_image_file_returns_decode_error"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-raster/src/lib.rs#tests::load_on_a_missing_path_returns_io_error"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-09-06
status: complete
---

# Phase 1 Plan 1: Raster engine and determinism proof — the tracer Summary

**A Cargo workspace of four crates and one working path: `chrys compare BASE CANDIDATE` reads two real PNG files, reports one `Recoloured` region with an exact bounding box, and exits 1.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-09-06T15:45:00Z
- **Completed:** 2026-09-06T15:59:42Z
- **Tasks:** 1 (tracer task, landed as an ordered group of commits)
- **Files modified:** 16 (15 created, 1 modified — see key-files)

## Accomplishments
- A Cargo workspace (`resolver = "3"`, four members, pinned workspace dependencies) that builds on `rustc 1.97.1`, the version `rust-toolchain.toml` pins.
- `chrys-source`: `Frame`, `RegionHint` and `Source`, with no dependency outside the standard library.
- `chrys-source-raster`: `RasterSource` decoding PNG through `image` 0.25.10, with `DecodeLimits` set on the reader before every decode call, so a crafted header cannot force an unbounded allocation (CVE-2023-29408 mitigation, T-01-01/T-01-02 in the plan's threat register).
- `chrys-core`: `Verdict`, `Region`, `ChangeKind`, `BoundingBox`, `ColourDelta`, `RefusalReason` and `compare(base, candidate)`, with zero format-crate and zero graphics-crate dependencies in `Cargo.toml` (T-01-03 mitigation, a compile-time fact, not a review rule).
- `chrys-cli`: the `chrys` binary. `chrys compare BASE CANDIDATE` prints the verdict's `Display` form to stdout and exits 0/1/2/3 for identical/changed/refused/error, with every error going to stderr only.
- The committed golden pair `tests/golden/pair-01/{base,candidate}.png` and the generator that built them, `crates/chrys-source-raster/examples/make-fixtures.rs`.

## Task Commits

Landed as an ordered group of commits, per AGENTS.md (one change per commit, code and tests together):

1. **Workspace root and toolchain pin** - `29ed804` (Cargo.toml, rust-toolchain.toml, .gitignore)
2. **chrys-source crate** - `35dc93c` (feat + test, 3 tests)
3. **chrys-source-raster adapter** - `c1c50ef` (feat + test, 3 tests)
4. **chrys-core engine** - `a48aab8` (feat + test, 6 tests)
5. **chrys-cli binary** - `25e3d94` (feat)
6. **Golden pair and generator** - `c2da725` (the fixture files)
7. **cargo fmt cleanup** - `3f7d2f2` (formatting only, no behaviour change)

**Plan metadata:** commit pending (this SUMMARY, STATE.md, ROADMAP.md — orchestrator owns state writes per the objective given to this executor)

## Files Created/Modified
- `Cargo.toml` - workspace root, resolver 3, pinned workspace dependencies, lints
- `rust-toolchain.toml` - pins channel 1.97.1, the determinism input for transcendental precision
- `.gitignore` - ignores `/target` and `**/*.rs.bk`; `Cargo.lock` stays tracked
- `crates/chrys-source/src/lib.rs` - `Frame`, `RegionHint`, `Source`, no format dependency
- `crates/chrys-source-raster/src/lib.rs` - `RasterSource`, `RasterError`, `DecodeLimits`, guarded PNG decode
- `crates/chrys-source-raster/examples/make-fixtures.rs` - builds and writes the golden pair
- `crates/chrys-core/src/verdict.rs` - `Verdict`, `Region`, `ChangeKind`, `BoundingBox`, `ColourDelta`, `RefusalReason`
- `crates/chrys-core/src/lib.rs` - `compare(base, candidate) -> Result<Verdict, CompareError>`
- `crates/chrys-cli/src/main.rs` - `chrys compare BASE CANDIDATE`, exit codes 0/1/2/3
- `tests/golden/pair-01/base.png`, `tests/golden/pair-01/candidate.png` - the real committed pair

## Decisions Made
- Re-ran `cargo search <name> --limit 1` for every pinned crate before writing `Cargo.toml`. All nine versions from 01-RESEARCH.md (`image` 0.25.10, `rustfft` 6.4.1, `realfft` 3.5.0, `imageproc` 0.27.0, `palette` 0.7.7, `libm` 0.2.16, `sha2` 0.11.0, `thiserror` 2.0.20, `anyhow` 1.0.104, `clap` 4.6.6) matched the live registry exactly. No deviation was needed.
- `rustfft`, `realfft`, `imageproc`, `palette` and `libm` are pinned in the workspace table for the plans that need them (01-04 onward) but are not yet a dependency of any crate in this plan; only `image`, `thiserror`, `anyhow` and `clap` are wired into a `Cargo.toml` today.
- The Rust edition accepted by the pinned toolchain is `2024`, as set in `[workspace.package]` and confirmed by a clean `cargo build --workspace` on `rustc 1.97.1`.
- `image::Limits` is `#[non_exhaustive]`, so `DecodeLimits::to_image_limits` builds it by mutating a `Limits::default()` rather than a struct literal.
- A `Limits` error from `decode()` is resolved to an exact `TooLarge { width, height, limit }` via a second, header-only dimension probe (`into_dimensions` with no width/height limit). This never allocates a pixel buffer, so it keeps the same DoS-safety property while giving a precise error message instead of reporting zeros.
- `chrys-core`'s tests build synthetic `Frame` values in memory rather than depending on `chrys-source-raster`, even as a dev-dependency, so the engine crate's dependency graph stays exactly what the threat register promises. The real-file path is proven end to end by the CLI, not by a `chrys-core` unit test.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Style/cleanup] Ran `cargo fmt` over three files after the plan's own tests already passed**
- **Found during:** post-task self-check, before writing this SUMMARY
- **Issue:** `cargo fmt --check` reported a diff in `crates/chrys-core/src/lib.rs`, `crates/chrys-core/src/verdict.rs` and `crates/chrys-source-raster/src/lib.rs` — long `assert!`/`assert_eq!` calls and one long `file.write_all(...)` call were not wrapped to the project's line width.
- **Fix:** Ran `cargo fmt` and committed the reformatted files as their own commit, with no behaviour change.
- **Files modified:** the three files above
- **Verification:** `cargo fmt --check` now reports no diff; `cargo test --workspace` still reports every test passing (12 tests total, 0 failed).
- **Committed in:** `3f7d2f2`

---

**Total deviations:** 1 auto-fixed (formatting only)
**Impact on plan:** No scope creep. The fix is purely mechanical and does not change any behaviour, error type, or public API named in the plan.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `chrys-source`, `chrys-source-raster`, `chrys-core` and `chrys-cli` all exist and build, so plan 01-02 (more raster formats) and 01-03 (the digest guard) have a real crate to extend rather than a new one to create.
- `crates/chrys-source-raster/examples/make-fixtures.rs` and the pattern of committing both a generator and its output are ready to reuse for `tests/golden/formats/` in 01-02.
- The single-region, whole-buffer-diff implementation of `compare` is explicitly a placeholder (documented in its own doc comment) for 01-07's connected-component labelling; nothing about its current shape blocks that replacement.
- No blockers.

## Self-Check: PASSED

All files below were verified present on disk (`ls -la`) and all commit hashes verified present in `git log --oneline` on this branch before this line was written:
- `Cargo.toml`, `rust-toolchain.toml`, `.gitignore` — FOUND
- `crates/chrys-source/src/lib.rs`, `crates/chrys-source/Cargo.toml` — FOUND
- `crates/chrys-source-raster/src/lib.rs`, `crates/chrys-source-raster/Cargo.toml`, `crates/chrys-source-raster/examples/make-fixtures.rs` — FOUND
- `crates/chrys-core/src/lib.rs`, `crates/chrys-core/src/verdict.rs`, `crates/chrys-core/Cargo.toml` — FOUND
- `crates/chrys-cli/src/main.rs`, `crates/chrys-cli/Cargo.toml` — FOUND
- `tests/golden/pair-01/base.png`, `tests/golden/pair-01/candidate.png` — FOUND
- Commits `29ed804`, `35dc93c`, `c1c50ef`, `a48aab8`, `25e3d94`, `c2da725`, `3f7d2f2` — FOUND in `git log --oneline`

---
*Phase: 01-raster-engine-and-determinism-proof*
*Completed: 2026-09-06*
