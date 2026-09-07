---
phase: 01-raster-engine-and-determinism-proof
verified: 2026-09-07T13:35:00Z
status: passed
score: 10/10 must-haves verified
covered_files:
  - ".github/workflows/determinism.yml"
  - ".planning/PROJECT.md"
  - ".planning/REQUIREMENTS.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-01-PLAN.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-01-SUMMARY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-02-PLAN.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-02-SUMMARY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-03-PLAN.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-03-SUMMARY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-04-PLAN.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-04-SUMMARY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-05-PLAN.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-05-SUMMARY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-06-PLAN.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-06-SUMMARY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-07-PLAN.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-07-SUMMARY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-08-PLAN.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-08-SUMMARY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-09-PLAN.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-09-SUMMARY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-REVIEW.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-SECURITY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-UAT.md"
  - "crates/chrys-cli/src/main.rs"
  - "crates/chrys-cli/tests/alpha.rs"
  - "crates/chrys-core/src/classify/antialias.rs"
  - "crates/chrys-core/src/classify/colour.rs"
  - "crates/chrys-core/src/classify/kind.rs"
  - "crates/chrys-core/src/classify/label.rs"
  - "crates/chrys-core/src/classify/mod.rs"
  - "crates/chrys-core/src/hash.rs"
  - "crates/chrys-core/src/lib.rs"
  - "crates/chrys-core/src/register/block_match.rs"
  - "crates/chrys-core/src/register/confidence.rs"
  - "crates/chrys-core/src/register/integral.rs"
  - "crates/chrys-core/src/register/luma.rs"
  - "crates/chrys-core/src/register/mod.rs"
  - "crates/chrys-core/src/register/phase_correlation.rs"
  - "crates/chrys-core/src/register/subpixel.rs"
  - "crates/chrys-core/src/register/warp.rs"
  - "crates/chrys-core/src/register/window.rs"
  - "crates/chrys-core/src/residual.rs"
  - "crates/chrys-core/src/sequence.rs"
  - "crates/chrys-core/src/verdict.rs"
  - "crates/chrys-core/tests/block_match.rs"
  - "crates/chrys-core/tests/classify.rs"
  - "crates/chrys-source-raster/examples/make-fixtures.rs"
  - "crates/chrys-source-raster/src/decode.rs"
  - "crates/chrys-source-raster/src/lib.rs"
  - "crates/chrys-source-raster/src/normalize.rs"
  - "crates/chrys-source/src/lib.rs"
  - "scripts/cross-arch-hash.sh"
  - "scripts/determinism-drill.sh"
  - "tests/golden/pair-01/expected-digest.sha256"
covered_digest: "v1:sha256:1764e60d6182a0d062d1db93bad826f6ff424e6a91cb7d226fd6897a0c3accbf"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 9/10
  gaps_closed:
    - "The tool compares two raster images and names each change by kind: moved, added, removed, recoloured, or resized. It never reports a bare pixel count. (alpha-only differences were reported `identical`, exit 0; now reported `Removed`, exit 1, on the committed regression fixture)"
  gaps_remaining: []
  regressions: []
---

# Phase 1: Raster Engine and Determinism Proof Verification Report

**Phase Goal:** A person compares two raster images and gets a structural verdict that is byte-identical across OS and architecture.
**Verified:** 2026-09-07T13:35:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure (plan 01-09, gap G-01-1)

## What this re-verification did differently

The stale report (2026-09-07T09:33:22Z) found one blocking gap: a pair whose
RGB bytes are pixel-identical and whose alpha differs was reported `identical`,
exit 0. Plan 01-09 landed a fix. This report does not carry the stale verdict
forward. Every claim below was independently re-measured against commit
`315b1b3` (current `HEAD`, matching `origin/main`), not read from
01-09-SUMMARY.md or from the orchestrator's own hand-off message.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Compares two raster images and names each change by kind; never a bare pixel count | ✓ VERIFIED | Live run on the committed `tests/golden/alpha-01` pair (RGB identical, alpha differs in 480 pixels): `Removed region at x=48, y=48, width=24, height=20`, exit 1 — reproduced myself, not read from SUMMARY. The stale report's own defect (`identical`, exit 0) no longer reproduces. |
| 2 | Reports a translated region's offset in pixels, and a colour change's difference value and both colours | ✓ VERIFIED | Live run, unchanged from before the fix: `Moved region at x=40, y=40, width=50, height=40, moved by (-2, 1)`; `Recoloured region at x=64, y=64, width=96, height=64, colour delta 111.83 (base [40, 90, 200, 255], candidate [200, 90, 40, 255])` |
| 3 | Does not report an antialiasing difference a person cannot see (at risk from this fix — alpha now feeds the brightness function the rule reads) | ✓ VERIFIED | `brightness` in `antialias.rs` composites onto a fixed opaque white reference in exact integer arithmetic (`luma * alpha + 255 * (255 - alpha)`, no division, no float, no `mul_add` — confirmed by grep, count 0 for both). Live: `cargo test -p chrys-core --test classify` — `suppress_antialiasing_on_an_alpha_expressed_diagonal_edge_yields_no_region` (an alpha-only antialiased ramp is suppressed) and `suppress_antialiasing_keeps_a_solid_alpha_only_change` (a solid alpha-only change is NOT suppressed) both pass. Confirmed the two test fixtures are genuinely different shapes (a diagonal alpha ramp vs. a solid alpha block), not a duplicate assertion. The rule neither under- nor over-suppresses after the fix. |
| 4 | Refuses a pair it cannot register and states why | ✓ VERIFIED | Live run, all 8 `should-refuse` pairs, unchanged confidence values from before the fix: e.g. pair-01 `peak confidence 976.82 is below the threshold 2313.88`, exit 2 |
| 5 | CI hashes raw RGBA8 output of the same pair on Linux, macOS, Windows, x86-64 and aarch64, and the hash matches on every commit | ✓ VERIFIED | `gh run view 34115454943 --json headSha,jobs`: `headSha` is `315b1b3c361f6ce843ac2ad7855ed382d5516908`, exactly this repository's current `HEAD` and `origin/main` (checked directly, not assumed) — this is the CI run for the CURRENT tip, not an orphaned commit. All 8 jobs `success`: `guards`, `digest (ubuntu-24.04)`, `digest (ubuntu-24.04-arm)`, `digest (macos-15)`, `digest (macos-15-intel)`, `digest (windows-2022)`, `digest (windows-11-arm)`, `agree`. `git reflog` shows a `filter-branch: rewrite` in this branch's history (main@{2}, main@{3}), confirming history WAS rewritten since earlier commits, but the run checked here is the run for the exact current tip, so the rewrite does not undermine this evidence. |
| 6 | No GPU or format-decoding crate enters `chrys-core`'s dependency graph (DET-04) | ✓ VERIFIED | `cargo tree -p chrys-core -e normal` unchanged; guard `no_gpu_or_format_crate_enters_chrys_cores_dependency_graph` passes live |
| 7 | The comparison path calls no platform transcendental function (DET-06) | ✓ VERIFIED | Guard `the_comparison_path_calls_no_forbidden_transcendental` passes live; the new alpha arithmetic (`abs_diff`, a plain multiply, a plain add, one shift) is exact integer, confirmed by grep for `f32`/`f64`/`mul_add` in `antialias.rs` (0 matches) |
| 8 | Two decodes/runs of the same pair produce byte-identical digests | ✓ VERIFIED | `cargo run -- compare tests/golden/pair-01/... --hash-only` reproduced live: `decode-base 5a5f5994...`, `decode-candidate 99bf5b95...`, `residual dd3587b3...`, `verdict 33142d19...` — matches the SUMMARY's own claimed values exactly, and matches the pre-fix `decode-base`/`decode-candidate`/`verdict` lines from the stale report byte for byte (only `residual` moved, from `f812da6a...`, as predicted). `cargo test --workspace --release`: 184 passed, 0 failed (counted myself from the raw per-binary `test result:` lines, not taken from the SUMMARY) |
| 9 | Every determinism guard has been watched failing on purpose, and the drill is re-runnable | ✓ VERIFIED | `sh scripts/determinism-drill.sh` run live: `3 of 3 drills behaved as expected`, exit 0; `sh scripts/cross-arch-hash.sh` run live inside the drill: `aarch64-apple-darwin and x86_64-apple-darwin agree on all four digests`, matching the residual/verdict digests above |
| 10 | PNG, JPEG, WebP and TIFF each decode to a `Frame` with the same shape contract | ✓ VERIFIED | `cargo test -p chrys-source-raster --release --test formats`: 5/5 pass live |

**Score:** 10/10 truths verified

### Independent Adversarial Spot-Check (not requested by the plan, run to stress-test the fix)

I built two throwaway (uncommitted, deleted after use) fixture pairs to check
whether the "Recoloured ... alpha delta 255" wording quoted in the hand-off
message is genuine or fabricated, since it does not match the wording produced
by the committed `alpha-01` regression fixture (`Removed`).

| Scenario | Base pixel | Candidate pixel | Frame shape | Result |
|---|---|---|---|---|
| Committed `alpha-01` | rectangle content, distinct from frame background | fully transparent | distinct background + rectangle (the `should-register` corpus's own structured canvas) | `Removed region at x=48, y=48, width=24, height=20` |
| Ad hoc repro A (flat 64x64 canvas, one colour everywhere) | `[120,130,140,255]` | `[120,130,140,0]` in a 20x20 block | uniform colour, no distinct background | `Recoloured region at x=20, y=20, width=20, height=20, colour delta 0.00 (base [120, 130, 140, 255], candidate [120, 130, 140, 0]), alpha delta 255` |
| Ad hoc repro B (same colours, but on a distinct 240-grey background, inside a rectangle) | same | same | distinct background + rectangle | `Removed region at x=20, y=20, width=20, height=20` |

**Finding:** the hand-off message's quoted string is genuine — I reproduced it
byte for byte — but it describes a different, more degenerate input (a frame
that is one flat colour everywhere) than the committed `alpha-01` fixture. The
reason is `classify_kind`'s absence test: a pixel counts as absent when it
equals the frame's own modal colour OR its alpha is zero. On a flat canvas the
changed region already equals the frame's modal colour before alpha changes
anything, so it is "absent" in the base frame too, and the pair falls through
to the `Recoloured` branch instead of `Removed`. This is a genuine, narrow edge
case: a region that becomes fully transparent is named `Recoloured` (with an
explicit, non-zero alpha delta) rather than `Removed`, specifically when its
own colour already equals the frame's background colour. It does not violate
Truth 1 — the change is still named by kind, with a bounding box, both colours,
and a printed alpha delta, never a bare count — and a frame that is one flat
colour everywhere is exactly the shape of input the confidence gate is most
likely to refuse in practice (no distinguishing structure to register against).
Recorded here as an observation for a future rule refinement, not as a gap
against this phase's stated success criteria.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chrys-core/src/register/warp.rs` | `difference_image` over all four RGBA8 channels | ✓ VERIFIED | Read live: loops `for channel in 0..4`, `abs_diff` on every byte, no constant write. Doc comment rewritten to describe a four-channel measurement, the old "viewable image" sentence is gone. |
| `crates/chrys-core/src/register/block_match.rs` | No dead alpha-255 initializer; `BlockOffset::score` stays three-channel, documented | ✓ VERIFIED | Read live: no `255` constant write into `samples`; `score` still sums R+G+B, with a doc comment stating the search/detection separation |
| `crates/chrys-core/src/classify/label.rs` | Foreground test reads the largest of four residual bytes | ✓ VERIFIED | Read live: `let magnitude = r.max(g).max(b).max(a);` |
| `crates/chrys-core/src/classify/antialias.rs` | Suppression zeroes four bytes; brightness reads alpha, no float/div/mul_add | ✓ VERIFIED | Read live; grep confirms 0 occurrences of `mul_add`, `f32`, `f64` outside comments |
| `crates/chrys-core/src/classify/kind.rs` | Absence includes full transparency | ✓ VERIFIED | Read live: `is_pixel_absent(pixel, background) = pixel == background \|\| pixel[3] == 0` |
| `crates/chrys-core/src/verdict.rs` | Alpha term printed in the colour clause, only when non-zero | ✓ VERIFIED | Read live: `alpha_delta = delta.base[3].abs_diff(delta.candidate[3]); if alpha_delta != 0 { write!(..., ", alpha delta {alpha_delta}") }` |
| `tests/golden/alpha-01/base.png`, `candidate.png` | Committed regression fixture | ✓ VERIFIED | Present on disk; decodes and compares as documented |
| `crates/chrys-cli/tests/alpha.rs` | End-to-end regression test | ✓ VERIFIED | 2/2 tests pass live |
| `.github/workflows/determinism.yml` | Alpha pair hashed in the digest matrix; six labels, `agree`, `guards` unchanged | ✓ VERIFIED | grep confirms `golden/alpha-01` appears exactly once (in the compare command); the six runner labels, `agree` and `guards` jobs are byte-identical to before this plan |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `register/warp.rs` (`difference_image`) | `classify/label.rs` (`label_regions`) | the residual's alpha byte carries a real difference, read by the foreground test | ✓ WIRED | Live: alpha-only fixture produces a non-empty region; RGB-only unit synthetic fields in `tests/classify.rs` also confirm the alpha byte alone can trigger foreground |
| `classify/antialias.rs` (`suppress_antialiasing`) | `classify/label.rs` | suppression zeroes all four bytes so a suppressed pixel cannot rejoin through alpha | ✓ WIRED | Live: `suppression_runs_before_labelling_so_an_edge_pixel_never_joins_a_region` passes; the alpha-ramp suppression test yields zero regions |
| `crates/chrys-cli/src/main.rs` | `hash.rs` | `--hash-only` prints `digest_report` | ✓ WIRED | Live `--hash-only` run on `pair-01` prints all four digest lines, matching the committed baseline |
| `.github/workflows/determinism.yml` | `chrys-cli` | matrix runs `chrys compare --hash-only`, including on `alpha-01` | ✓ WIRED | Confirmed in workflow source and in the live `gh run view` job list, all green on current `HEAD` |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Alpha-only difference is now detected and named, on the committed fixture | `chrys compare tests/golden/alpha-01/{base,candidate}.png` | `Removed region at x=48, y=48, width=24, height=20`, exit 1 | ✓ PASS |
| Alpha-only difference is NOT detected, if reverted (sanity: this is what the stale report found) | n/a — not re-tested by reverting; the pre-fix behaviour is independently confirmed by the unchanged `decode-base`/`decode-candidate` digests differing from the `verdict`'s prior `identical` result recorded in the stale report | — | (historical, not re-run) |
| An antialiased edge expressed only in alpha is still suppressed | `cargo test -p chrys-core --test classify -- suppress_antialiasing_on_an_alpha_expressed_diagonal_edge_yields_no_region` (run inside the full `classify` binary) | pass | ✓ PASS |
| A solid alpha-only change survives suppression (the fix does not over-suppress) | same binary, `suppress_antialiasing_keeps_a_solid_alpha_only_change` | pass | ✓ PASS |
| Full workspace test suite is green | `cargo test --workspace --release` | 184 passed, 0 failed (counted from raw `test result:` lines) | ✓ PASS |
| `cargo clippy --workspace --all-targets --release` is clean | as run | 0 warnings | ✓ PASS |
| `cargo fmt --check` is clean | as run | exits 0, no output | ✓ PASS |
| Determinism guards pass unedited | `cargo test -p chrys-core --test determinism` | 6/6 pass | ✓ PASS |
| Determinism drill and cross-arch script both still pass | `sh scripts/determinism-drill.sh` | `3 of 3 drills behaved as expected`, exit 0, embeds a passing `cross-arch-hash.sh` run | ✓ PASS |
| Live CI on the exact current commit is green | `gh run view 34115454943` | `headSha` = current `HEAD`; 8/8 jobs `success` | ✓ PASS |
| All 8 refusal-corpus pairs report the same confidence values as before the fix | `chrys compare` on each `should-refuse` pair | all 8 confidences unchanged (e.g. pair-01: 976.82) | ✓ PASS |
| The three UAT headline verdict strings are byte-identical to before the fix | `chrys compare` on `pair-01`, `should-register/pair-08`, `should-register/pair-07` | Recoloured/Moved/Added strings match UAT tests 1, 5, 6 verbatim | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| CORE-01 | 01-01, 01-07, 01-09 | Names each change by kind, never a bare pixel count | ✓ SATISFIED | Alpha-only change now named `Removed` on the committed fixture; RGB-visible changes unaffected |
| CORE-02 | 01-04, 01-06 | Registers a translated region and reports the offset | ✓ SATISFIED | Unchanged; live run confirms |
| CORE-03 | 01-07 | Reports a colour change as a value + two colours | ✓ SATISFIED | Unchanged; live run confirms |
| CORE-04 | 01-07, 01-09 | Groups changed pixels into labelled regions with bounding boxes | ✓ SATISFIED | 28/28 `classify.rs` tests pass, including the three new alpha-absence tests |
| CORE-05 | 01-07, 01-09 | Does not report an antialiasing difference a person cannot see | ✓ SATISFIED | Both new alpha-antialiasing tests pass; no regression in the pre-existing antialiasing tests (all ran unedited) |
| CORE-06 | 01-05 | Refuses a pair it cannot register, states why | ✓ SATISFIED | Unchanged; live run confirms |
| CORE-07 | 01-05 | Compares only near-identical pairs, says so when too different | ✓ SATISFIED | Unchanged; all 8 refusal confidences identical to pre-fix |
| DET-01 | 01-03, 01-08 | Identical verdict on Linux, macOS, Windows | ✓ SATISFIED | 6-runner CI `agree` job green on current `HEAD` |
| DET-02 | 01-03, 01-08 | Identical verdict on x86-64 and aarch64 | ✓ SATISFIED | Same CI evidence; local `cross-arch-hash.sh` also green live |
| DET-03 | 01-03, 01-08, 01-09 | CI proves DET-01/DET-02 on every commit, hashing raw RGBA8, now including the alpha pair | ✓ SATISFIED | Live `gh run view` on current `HEAD`; alpha-01 added to the matrix, confirmed by grep |
| DET-04 | 01-01, 01-08 | No pixel entering comparison is GPU-produced | ✓ SATISFIED | Guard passes live; no crate added by 01-09 |
| DET-06 | 01-04, 01-08, 01-09 | No platform transcendental on the comparison path; FP contraction disabled | ✓ SATISFIED | Guard passes live; new alpha arithmetic confirmed exact-integer by grep |
| SRC-01 | 01-01, 01-02 | Raster pair compared (PNG, JPEG, WebP, TIFF) | ✓ SATISFIED | `formats.rs` 5/5 tests pass live |

No orphaned requirements: the 13 IDs declared across the nine plans exactly match REQUIREMENTS.md's Phase 1 traceability rows.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers in any file this phase (through plan 01-09) modified or added. No stub patterns found.

**The code review's five WARNINGS remain unfixed.** Judged individually against the phase goal:

| ID | Issue | Blocks the phase goal? | Reasoning |
|---|---|---|---|
| WR-01 | Transcendental/`mul_add` guard misses `Type::method(x)` call syntax, only matches `.method(` | No — acceptable debt | No call site in the tree uses this form today (confirmed: the new alpha arithmetic in `antialias.rs` and `warp.rs` uses `abs_diff`, plain `*`/`+`, never a qualified call). This is a latent gap in the guard's own coverage, not a live determinism violation. Recommend a follow-up, not a phase blocker. |
| WR-02 | Block-comment line-counting shifts line numbers in a guard's failure message | No — acceptable debt | Cosmetic: a diagnostic message could point at the wrong line if a violation existed. No violation exists today. Does not affect pass/fail, only debuggability of a hypothetical future failure. |
| WR-03 | `RasterError::TooLarge` can misreport which axis (`width`/`height`) was exceeded under an asymmetric `DecodeLimits` | No — acceptable debt | The CLI's own decode path (and every fixture in this phase) uses the default, symmetric limit. Only a caller of `RasterSource::with_limits` with asymmetric bounds can hit this, and even then the failure mode is a misleading error message, not a wrong verdict or a crash. |
| WR-04 | `difference_image`/`IntegralImage::window_sum` are public but only `debug_assert!`-guarded; a release build handed a mismatched pair panics with a generic message instead of a `Result` | No — acceptable debt | Confirmed still true: both remain `pub`, both still use `debug_assert_eq!`/`debug_assert!` only. Rust's bounds checking still prevents memory corruption; the failure mode is an ungraceful panic, not a silent wrong answer, and today's only two call sites (inside this crate) already guarantee equal lengths/in-bounds rectangles. This is a real robustness gap for a hypothetical external caller of `chrys-core`'s public API, worth fixing before that API is advertised as stable, but it does not affect what a person running `chrys compare` sees today. |
| WR-05 | `frame_background` rescans the whole frame once per labelled region | No — explicitly out of scope | The review itself notes this is a performance observation, explicitly excluded from this review's scope. |

**Judgment:** none of the five WARNINGS blocks the phase goal. All five are either latent/theoretical (no live violation exists), scoped to an unused configuration path, or explicitly out of scope. They are legitimate debt for a follow-up, most pressingly WR-04 if and when `chrys-core` is ever consumed by an external caller. Not raised as a gap here because raising a pre-existing, already-documented, non-regressing WARNING as a blocking gap on a re-verification pass would go beyond what plan 01-09 was asked to close (gap G-01-1 only), and none of the five affects a truth this phase's own success criteria assert.

## Deferred Items

None.

## Human Verification Required

None. Every truth above was resolved by direct code inspection, live test runs, a live CI check via `gh run view` against the exact current commit SHA, and independently-constructed adversarial reproductions (not sourced from SUMMARY.md's or the orchestrator's claims alone).

## Gaps Summary

None. The one gap from the stale report (G-01-1: an alpha-only difference reported `identical`) is closed and independently re-confirmed:

- The exact reproduction case from the stale report's own spot-check (`identical`, exit 0) no longer occurs; the committed regression fixture now produces `Removed region at x=48, y=48, width=24, height=20`, exit 1.
- The one success criterion at greatest risk from this fix (Criterion 3, antialiasing suppression) was independently re-derived: `brightness`'s new alpha-weighted arithmetic is exact integer, monotone, and a factor of 255 of the old value on fully opaque frames; the two new tests (a genuinely alpha-only antialiased ramp, and a genuinely solid alpha-only change) confirm the rule neither under- nor over-suppresses.
- CI is green on the exact current commit (`315b1b3`, matching `origin/main`), not an orphaned commit from before a history rewrite that did occur in this branch's reflog.
- 184 tests pass workspace-wide (counted directly, not from SUMMARY), clippy and fmt are clean, the drill and cross-arch script both pass live.
- The five pre-existing code-review WARNINGS remain unfixed but are judged, individually, not to block the phase goal — see the Anti-Patterns section above for the reasoning on each.
- One narrow naming edge case was found during adversarial spot-checking (a fully-transparent region whose colour already equals the frame's own background colour is named `Recoloured` with an alpha delta, not `Removed`) — recorded as an observation, not a gap, because the change is still correctly named by kind with a bounding box and never as a bare count.

The phase goal — a person compares two raster images and gets a structural verdict that is byte-identical across OS and architecture — holds, including for the alpha-channel case the stale report found broken.

---

_Verified: 2026-09-07T13:35:00Z_
_Verifier: Claude (gsd-verifier)_
