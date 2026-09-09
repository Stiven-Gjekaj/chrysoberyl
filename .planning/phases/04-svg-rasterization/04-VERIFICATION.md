---
phase: 04-svg-rasterization
verified: 2026-09-09T00:00:00Z
status: passed
score: 10/10 must-haves verified
covered_files: [".github/workflows/determinism.yml", ".planning/REQUIREMENTS.md", ".planning/phases/04-svg-rasterization/04-01-PLAN.md", ".planning/phases/04-svg-rasterization/04-01-SUMMARY.md", ".planning/phases/04-svg-rasterization/04-02-PLAN.md", ".planning/phases/04-svg-rasterization/04-02-SUMMARY.md", ".planning/phases/04-svg-rasterization/04-03-PLAN.md", ".planning/phases/04-svg-rasterization/04-03-SUMMARY.md", ".planning/phases/04-svg-rasterization/04-RESEARCH.md", ".planning/phases/04-svg-rasterization/04-VALIDATION.md", "Cargo.toml", "crates/chrys-cli/Cargo.toml", "crates/chrys-cli/src/main.rs", "crates/chrys-cli/tests/svg_boundary_guard.rs", "crates/chrys-cli/tests/svg_dispatch.rs", "crates/chrys-cli/tests/system_font_guard.rs", "crates/chrys-cli/tests/unsafe_guard.rs", "crates/chrys-source-svg/Cargo.toml", "crates/chrys-source-svg/examples/render-svg.rs", "crates/chrys-source-svg/fonts/NotoSans-Regular.ttf", "crates/chrys-source-svg/fonts/OFL.txt", "crates/chrys-source-svg/src/fonts.rs", "crates/chrys-source-svg/src/lib.rs", "crates/chrys-source-svg/src/sniff.rs", "crates/chrys-source-svg/tests/svg.rs", "tests/golden/formats/svg/base.svg", "tests/golden/formats/svg/candidate.svg"]
covered_digest: "v1:sha256:5db331e14e547838d3f9fe534310ed875244aae58bac0a0eebc7e27bf81b54ee"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 4: SVG Rasterization Verification Report

**Phase Goal:** "Vector input is rasterized deterministically on the CPU, proving the Core Value against real curves, gradients, and text."
**Verified:** 2026-09-09
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: An SVG pair is rasterized on the CPU with resvg/tiny-skia and compared at the same quality as raster input | ✓ VERIFIED | Live run: `cargo run -p chrys-cli -- compare tests/golden/formats/svg/{base,candidate}.svg` exits 1, prints `Recoloured region at x=192, y=16, width=48, height=48, colour delta 199.12 (base [0, 204, 0, 255], candidate [204, 0, 204, 255])` — same shape (named kind + rectangle + colour delta) as raster path (`Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 ...` for `tests/golden/rule-01/{base,candidate}.png`). `crates/chrys-cli/tests/svg_dispatch.rs#an_svg_pair_reports_a_recoloured_region` asserts this equivalence programmatically by reading the raster pair's own kind string rather than a hardcoded literal. |
| 2 | SC1 (adapter): SVG rasterizes via `resvg`/`tiny-skia` on the CPU, straight-alpha `Frame` | ✓ VERIFIED | `crates/chrys-source-svg/src/lib.rs` implements `Source for SvgSource` calling `resvg::render` into a `tiny_skia::Pixmap`, then `Pixmap::take_demultiplied()`. Live-ran `cargo test -p chrys-source-svg alpha_is_straight_not_premultiplied` — passed (translucent pixel red exceeds alpha by >100, opaque pixel alpha=255). |
| 3 | SC2 (negative): text rasterization uses only the pinned font set; the tool never reads the host font database | ✓ VERIFIED | Two independent, genuinely load-bearing guards, both live-run and passing: `cargo test -p chrys-cli --test svg_boundary_guard` (resolved `cargo tree -e features -p chrys-source-svg` names no `system-fonts`/`memmap-fonts`, and `cargo tree -p chrys-core -e normal` names no SVG-path crate) and `cargo test -p chrys-cli --test system_font_guard` (workspace-wide source search for `load_system_fonts`, comments/strings stripped, one named allow-list entry). Independently confirmed at the crate-source level: `fontdb-0.24.0`'s `load_system_fonts` is gated `#[cfg(feature = "fs")]`; `usvg`'s `text` feature enables `fontdb` with `default-features = false` and does **not** turn on `fs`; `cargo tree -e features -p chrys-source-svg` (run live) shows no `fs`/`memmap`/`fontconfig`/`system-fonts` feature anywhere in the resolved graph — the function is not compiled into this binary at all, not merely unused. Both guards were drilled red in 04-02-SUMMARY.md against planted defects (restored default features; a stub `load_system_fonts` function), and each red message named the exact defect. |
| 4 | SC2 (rendering half): every glyph comes from the pinned font, including unmatched/absent `font-family` | ✓ VERIFIED | Live-ran `cargo test -p chrys-source-svg an_unmatched_font_family_falls_back_to_the_pinned_font` — passed. Asserts three documents (pinned family, `Arial`, no family attribute) render byte-identical, non-empty buffers. `crates/chrys-source-svg/src/fonts.rs` registers the pinned font as every one of `fontdb`'s five generic families, closing the gap the executor found and fixed during Task 3 (04-01-SUMMARY.md, "Predictions That Turned Out Wrong"). |
| 5 | SC3: a cross-OS golden-hash test on the text-bearing SVG fixture passes on Linux, macOS and Windows, and is the phase's exit gate | ✓ VERIFIED | Independently re-confirmed via `gh run view 34347989464 --repo Stiven-Gjekaj/chrysoberyl --json conclusion,jobs`: all 8 jobs (`guards`, six `digest` runners, `agree`) report `conclusion: success`. `.github/workflows/determinism.yml` (lines ~78-124) carries the SVG digest lines plus a written falsification protocol (what the line gates, why the claim is not assumed, the SIMD fallback, and that a further disagreement is a person's decision to withdraw the claim, in writing, above the new lines). |
| 6 | The `agree` job needed no edit to pick up the new fixture (proves phase 2's whole-file `cmp -s` rewrite) | ✓ VERIFIED | Live: `git show 199477d:.github/workflows/determinism.yml`'s `agree:` section, diffed against HEAD's own `agree:` section via `cmp -s` — byte-identical. |
| 7 | All six runner labels remain declared, none dropped | ✓ VERIFIED | Live grep of `.github/workflows/determinism.yml`: `ubuntu-24.04`, `ubuntu-24.04-arm`, `macos-15`, `macos-15-intel`, `windows-2022`, `windows-11-arm` all present. |
| 8 | SVG dispatch decides on content, never extension; every existing fixture still reaches its own adapter | ✓ VERIFIED | `named_frames_for` (`crates/chrys-cli/src/main.rs:660-671`) checks `chrys_source_svg::looks_like_svg(path)` before the animation sniff, content-only (reads a bounded 1024-byte prefix, requires `<` + `<svg`). Live-ran `cargo test -p chrys-cli --test svg_dispatch` — 3/3 passed, including `no_committed_fixture_is_taken_from_the_adapter_it_already_used` (all 14 committed PNG/JPEG/WebP/TIFF/GIF/APNG/animated-WebP fixtures sniff false). |
| 9 | No file under `crates/chrys-core/` changes | ✓ VERIFIED | Live: `git rev-parse HEAD:crates/chrys-core` = `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`, matching the pre-phase anchor read at commit `199477d`. |
| 10 | The unsafe-code guard covers the ninth crate it now protects | ✓ VERIFIED | `crates/chrys-cli/tests/unsafe_guard.rs` asserts `checked >= 9`; live `ls crates/` = 9 directories; live-ran `cargo test -p chrys-cli --test unsafe_guard` — passed. |

**Score:** 10/10 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chrys-source-svg/src/lib.rs` | `SvgSource`, `SvgError`, `SvgLimits`, `impl Source for SvgSource` | ✓ VERIFIED | Present, substantive (6-variant error type, ordered refusal checks, `take_demultiplied`), wired into `chrys-cli` |
| `crates/chrys-source-svg/src/fonts.rs` | the one `include_bytes!`, empty-by-default font DB filled once | ✓ VERIFIED | Present; `pinned_fontdb()` builds `Database::new()` + one `load_font_data` + 5 generic-family sets |
| `crates/chrys-source-svg/fonts/NotoSans-Regular.ttf` | pinned, committed font | ✓ VERIFIED | Tracked by git; 621,572 bytes; SHA-256 `478c558e...` matches SUMMARY's recorded pin exactly |
| `crates/chrys-source-svg/fonts/OFL.txt` | SIL OFL 1.1 text | ✓ VERIFIED | Tracked by git, present beside the font |
| `crates/chrys-source-svg/examples/render-svg.rs` | one-path PNG renderer for human inspection | ✓ VERIFIED | Present; loads through `SvgSource`, writes via `png` crate, no second render path |
| `tests/golden/formats/svg/base.svg` / `candidate.svg` | text/curve/gradient-bearing fixture pair, one rectangle recoloured | ✓ VERIFIED | Present; diff confirms exactly one `fill` attribute changed (`#00cc00` → `#cc00cc`), all else byte-identical |
| `crates/chrys-cli/tests/svg_dispatch.rs` | dispatch tests | ✓ VERIFIED | Present, 3 tests, all live-run and passing |
| `crates/chrys-cli/tests/svg_boundary_guard.rs` | Cargo-feature-graph and engine-graph guards | ✓ VERIFIED | Present, 2 tests, fail (not skip) on command failure, live-run and passing |
| `crates/chrys-cli/tests/system_font_guard.rs` | workspace-wide source search for the host font loader | ✓ VERIFIED | Present, 2 tests, live-run and passing |
| `.github/workflows/determinism.yml` | six-runner matrix extended + falsification protocol | ✓ VERIFIED | Present; SVG lines appended after the five existing blocks; comment block records the protocol |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `crates/chrys-source-svg/src/lib.rs` | `resvg::tiny_skia::Pixmap` | `take_demultiplied()` | ✓ WIRED | Live-confirmed via `alpha_is_straight_not_premultiplied` test pass |
| `crates/chrys-source-svg/src/lib.rs` | `crates/chrys-source-svg/src/fonts.rs` | `build_options()` sets `fontdb`/`font_family` before every parse | ✓ WIRED | `build_options()` calls `fonts::pinned_fontdb()` and `fonts::pinned_family_name()` each `load` call |
| `crates/chrys-cli/src/main.rs` | `crates/chrys-source-svg/src/lib.rs` | `named_frames_for`'s SVG branch | ✓ WIRED | Confirmed by reading `main.rs:660-671`; live end-to-end CLI run over the SVG fixture pair produced a real verdict |
| `.github/workflows/determinism.yml` digest job | `tests/golden/formats/svg/base.svg` | hash-only CLI invocation appended to `digest.txt` | ✓ WIRED | Confirmed present in workflow file; per-runner job logs (spot-checked in 04-03-SUMMARY.md) show the step executing |
| `.github/workflows/determinism.yml` digest job | `agree` job | whole-file `cmp -s`, unedited | ✓ WIRED | Live `cmp -s` against commit `199477d`'s `agree:` section — byte-identical |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| SVG pair compares at raster quality | `cargo run -q -p chrys-cli -- compare tests/golden/formats/svg/{base,candidate}.svg` | exit 1, `Recoloured region at x=192, y=16, width=48, height=48, colour delta 199.12 ...` | ✓ PASS |
| Raster pair for shape comparison | `cargo run -q -p chrys-cli -- compare tests/golden/rule-01/{base,candidate}.png` | exit 1, `Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 ...` — same output shape | ✓ PASS |
| Alpha conversion is straight, not premultiplied | `cargo test -p chrys-source-svg alpha_is_straight_not_premultiplied` | 1 passed | ✓ PASS |
| Unmatched/absent font family falls back to pinned font | `cargo test -p chrys-source-svg an_unmatched_font_family_falls_back_to_the_pinned_font` | 1 passed | ✓ PASS |
| Feature-graph guard (DET-05 compile-time half) | `cargo test -p chrys-cli --test svg_boundary_guard` | 2 passed | ✓ PASS |
| Source-search guard (DET-05 source-level half) | `cargo test -p chrys-cli --test system_font_guard` | 2 passed | ✓ PASS |
| Dispatch tests (content-based, no fixture stolen) | `cargo test -p chrys-cli --test svg_dispatch` | 3 passed | ✓ PASS |
| Unsafe-code guard floor at nine | `cargo test -p chrys-cli --test unsafe_guard` | 1 passed | ✓ PASS |
| Engine boundary unmoved | `test "$(git rev-parse HEAD:crates/chrys-core)" = "c97a6fb778c4b1373e5c4dc563481cc18e4c98c0"` | matched | ✓ PASS |
| chrys-core's own dependency graph names no SVG crate | `cargo tree -p chrys-core -e normal` | no `resvg`/`usvg`/`tiny-skia`/`fontdb`/etc. present | ✓ PASS |
| `fontdb`'s `load_system_fonts` is compile-time absent, not just unused | inspected `fontdb-0.24.0/src/lib.rs` (`#[cfg(feature = "fs")]`) and `usvg-0.48.1`/`resvg-0.48.1` `Cargo.toml` (their `text` feature never turns on `fontdb/fs`); `cargo tree -e features -p chrys-source-svg` names no `fs`/`memmap`/`fontconfig`/`system-fonts` anywhere | confirmed absent from resolved graph | ✓ PASS |

### CI Determinism Matrix

| Job | Conclusion |
|-----|------------|
| guards | success |
| digest (ubuntu-24.04) | success |
| digest (ubuntu-24.04-arm) | success |
| digest (macos-15) | success |
| digest (macos-15-intel) | success |
| digest (windows-2022) | success |
| digest (windows-11-arm) | success |
| agree | success |

Independently re-confirmed via `gh run view 34347989464 --repo Stiven-Gjekaj/chrysoberyl --json conclusion,jobs`, run at head SHA `89aa74f2436efe1938bde49ca85550e5430389a6` (the two commits after it, `69965c5` and `3ffbf5c`, touch only `.planning/` bookkeeping — confirmed via `git diff --stat 89aa74f HEAD`).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| SRC-04 | 04-01, 04-02, 04-03 | An SVG pair is rasterized on the CPU and compared | ✓ SATISFIED | Truths 1, 2, 8; live CLI run and dispatch tests |
| DET-05 | 04-01, 04-02, 04-03 | Text rasterization uses a pinned font set, never the host font database | ✓ SATISFIED | Truths 3, 4, 5; two independent live-passing guards plus source-level confirmation that the host loader is not compiled in |

No orphaned requirements found: `.planning/REQUIREMENTS.md`'s traceability table maps only SRC-04 and DET-05 to Phase 4, and both appear in every plan's `requirements` frontmatter field.

### Anti-Patterns Found

None. Scanned every file this phase created or modified (`crates/chrys-source-svg/{src,tests,examples}/*`, `crates/chrys-cli/src/main.rs`, `crates/chrys-cli/tests/{svg_dispatch,svg_boundary_guard,system_font_guard,unsafe_guard}.rs`, `.github/workflows/determinism.yml`) for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers and placeholder-shaped prose — none found.

### Human Verification Required

None. The two human-check items the plans identified (visually inspecting the rendered fixture for correct glyphs/curve/gradient/translucency; reading the pushed CI run's `agree` job result) were both already performed during phase execution and independently re-verified by this verification pass rather than left open: the rendered-fixture inspection is recorded verbatim in 04-01-SUMMARY.md ("Chrysoberyl" / "Rasterized", every glyph drew, curve and gradient visible, translucent shape visibly translucent), and the CI run was independently re-queried here via `gh run view` rather than trusted from the SUMMARY's own report.

### Gaps Summary

None. All ten observable truths derived from the three plans' `must_haves` (deduplicated against the roadmap's three success criteria) are VERIFIED against live test runs, direct source inspection, and an independent GitHub API query — not against SUMMARY.md's own claims. The one negative claim (DET-05, "the tool never reads the host font database") was traced past the project's own two guards into `fontdb`'s own source to confirm `load_system_fonts` is gated behind a Cargo feature (`fs`) that this workspace's resolved dependency graph never turns on for `chrys-source-svg` — the capability is absent from the compiled binary, not merely unused, which is the stronger claim DET-05 makes.

---

_Verified: 2026-09-09_
_Verifier: Claude (gsd-verifier)_
