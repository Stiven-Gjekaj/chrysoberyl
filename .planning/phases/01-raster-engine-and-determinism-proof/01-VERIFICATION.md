---
phase: 01-raster-engine-and-determinism-proof
verified: 2026-09-07T09:33:22Z
status: gaps_found
score: 9/10 must-haves verified
covered_files:
  - ".github/workflows/determinism.yml"
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
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-REVIEW.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-SECURITY.md"
  - ".planning/phases/01-raster-engine-and-determinism-proof/01-UAT.md"
  - "crates/chrys-cli/src/main.rs"
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
  - "crates/chrys-source-raster/src/decode.rs"
  - "crates/chrys-source-raster/src/lib.rs"
  - "crates/chrys-source-raster/src/normalize.rs"
  - "crates/chrys-source/src/lib.rs"
  - "scripts/cross-arch-hash.sh"
  - "scripts/determinism-drill.sh"
covered_digest: "v1:sha256:1ee682ea93e0c4e8874703684592c29aec20b595a636fc9c909d87176151f83f"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "The tool compares two raster images and names each change by kind: moved, added, removed, recoloured, or resized. It never reports a bare pixel count."
    status: failed
    reason: >
      A pair whose RGB bytes are pixel-identical but whose alpha channel
      differs is reported `identical` with exit code 0, even though
      `Frame` is documented everywhere ("RGBA8, straight alpha",
      chrys-source/src/lib.rs:13) as including alpha in the compared
      pixel data, and even though the two frames' own `--hash-only`
      decode digests differ. `difference_image`
      (register/warp.rs:69-83) loops `for channel in 0..3` and forces
      the residual's alpha byte to `u8::MAX`; `label_regions`
      (classify/label.rs:159-173) then reads only the R, G, B residual
      bytes when deciding whether a pixel is foreground. Alpha never
      contributes to change detection anywhere on the comparison path, so
      an alpha-only edit (a transparency change, a mask edit, a
      compositing change — realistic edits in PNG, WebP and TIFF, the
      formats this phase decodes) is not merely mis-named, it is never
      detected: zero labelled regions, `Verdict::Identical`. This is a
      silent wrong answer on a documented part of the pixel format, not
      an undocumented, deliberately-scoped-out limitation — no doc
      comment anywhere states alpha is out of scope. It is exactly the
      failure mode CORE-06/CORE-07's refuse-rather-than-guess principle
      exists to prevent, except here the tool does not even signal
      uncertainty; it affirmatively asserts sameness for content that
      changed.
    artifacts:
      - path: "crates/chrys-core/src/register/warp.rs"
        issue: "difference_image (lines 69-83) only diffs channels 0..3 (R,G,B) and hardcodes the residual's alpha byte to u8::MAX, discarding any real alpha difference before it reaches classification."
      - path: "crates/chrys-core/src/classify/label.rs"
        issue: "label_regions' foreground test (lines 159-173) reads residual.samples[idx], [idx+1], [idx+2] only — R, G, B — never the alpha byte, so even an undiscarded alpha residual would not be seen."
      - path: "crates/chrys-core/src/sequence.rs"
        issue: "compare_pair has no raw-byte-equality shortcut and reports Verdict::Identical exactly when label_regions returns zero regions (lines 32-85), so the RGB-only residual is the sole basis for the identical verdict."
    missing:
      - "Either extend difference_image and label_regions' foreground test to include the alpha channel (the code review's suggested fix: difference alpha too, and let the magnitude test consider r.max(g).max(b).max(a)), or, if alpha is deliberately out of scope for phase 1, make that an explicit, tested, documented decision on Frame/difference_image/compare_pair, and add a guard that fails loudly (or reports a distinct, non-identical verdict) when RGB channels agree but raw frame bytes do not."
      - "A test pair whose RGB is pixel-identical and whose alpha differs, asserting the verdict is not Identical (or is an explicit, named refusal/limitation), added to crates/chrys-core/tests/classify.rs or crates/chrys-cli/tests/."
---

# Phase 1: Raster Engine and Determinism Proof Verification Report

**Phase Goal:** A person compares two raster images and gets a structural verdict that is byte-identical across OS and architecture.
**Verified:** 2026-09-07T09:33:22Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## A note on ROADMAP mode

ROADMAP.md marks this phase `Mode: mvp`, which normally requires the phase
goal to be a literal User Story (`As a ..., I want to ..., so that ....`).
The stored goal text ("A person compares two raster images and gets a
structural verdict...") fails that format check
(`gsd_run query user-story.validate` returns `valid: false`), even though
every PLAN's own `<objective>` block *does* carry a well-formed User Story
("As a person who reviews visual changes, I want to compare two raster
images and read a structural verdict, so that the verdict is byte-identical
on every operating system and CPU architecture."). This looks like a roadmap
metadata mismatch (mode set, goal field never reformatted), not a planning
defect — ROADMAP.md already carries five well-formed, testable Success
Criteria in the classic goal-backward shape, which is exactly what standard
verification needs. Given a fully executed, reviewed, security-audited and
UAT'd phase sitting behind this metadata field, refusing to verify outright
would block a legitimate adversarial audit over a labelling issue unrelated
to code quality, so this report proceeds with standard goal-backward
verification against the five stated Success Criteria. Recommend fixing the
`Mode`/goal mismatch in ROADMAP.md (either drop `mode: mvp` for this phase,
or reformat the goal to the User Story already written in the PLANs) before
the next phase that sets `mode: mvp` is verified.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Compares two raster images and names each change by kind; never a bare pixel count | ✗ FAILED | A pair differing only in alpha reports `identical`, exit 0, despite differing decode digests. See Gaps. |
| 2 | Reports a translated region's offset in pixels, and a colour change's difference value and both colours | ✓ VERIFIED | Live run: `Moved region at x=40, y=40, width=50, height=40, moved by (-2, 1)`; `Recoloured region ... colour delta 111.83 (base [40, 90, 200, 255], candidate [200, 90, 40, 255])` (UAT tests 1-2, 5; code: classify/kind.rs, classify/colour.rs) |
| 3 | Does not report an antialiasing difference a person cannot see | ✓ VERIFIED | `classify/antialias.rs` implements pixelmatch's `antialiased()`/`hasManySiblings()` heuristic with its two blind spots documented in the module doc comment; UAT test 13 (human-accepted) confirms refusal/suppression wording holds under live review |
| 4 | Refuses a pair it cannot register and states why, instead of giving an unsupported verdict | ✓ VERIFIED | Live run on `refuse-01/should-refuse/pair-01`: `refused: the pair is too different to register: peak confidence 976.82 is below the threshold 2313.88; this engine compares near-identical pairs only`, exit 2 |
| 5 | CI hashes raw RGBA8 output of the same pair on Linux, macOS, Windows, x86-64 and aarch64, and the hash matches on every commit | ✓ VERIFIED | `gh run view` on the latest push (commit `1719283`, run `34073282985`): `digest` jobs green on `ubuntu-24.04`, `ubuntu-24.04-arm`, `macos-15`, `macos-15-intel`, `windows-2022`, `windows-11-arm`; `agree` job (byte-compares all six `digest.txt` reports) green. `.github/workflows/determinism.yml`'s digest step runs `chrys compare ... --hash-only`, which prints decode/residual/verdict digests over raw RGBA8, never a re-encoded file. |
| 6 | No GPU or format-decoding crate enters `chrys-core`'s dependency graph (DET-04) | ✓ VERIFIED | `cargo tree -p chrys-core -e normal` lists only `chrys-source, libm, palette, rustfft, sha2, thiserror` and their non-format, non-GPU transitives; guard `no_gpu_or_format_crate_enters_chrys_cores_dependency_graph` passes live |
| 7 | The comparison path calls no platform transcendental function (DET-06) | ✓ VERIFIED | Guard `the_comparison_path_calls_no_forbidden_transcendental` passes live; `palette` pinned with `default-features = false, features = ["libm"]`; `FftPlannerScalar` only, verified by grep and by the `the_comparison_path_never_constructs_the_auto_dispatching_fft_planner` guard |
| 8 | Two decodes/runs of the same pair produce byte-identical digests (decode + residual determinism) | ✓ VERIFIED | `crates/chrys-cli/tests/digest.rs` compares a live run against the committed `tests/golden/pair-01/expected-decode.sha256`; `crates/chrys-core/tests/register.rs`'s `two_runs_of_phase_correlate_return_bit_identical_correlation_surfaces` and `two_runs_of_refine_peak_on_the_same_input_agree_exactly` pass; full workspace test run (`cargo test --workspace --release`) is green, 0 failures |
| 9 | Every determinism guard has been watched failing on purpose, and the drill is re-runnable by anyone (not a one-off session) | ✓ VERIFIED | `scripts/determinism-drill.sh` (153 lines) plants one defect per guard in a disposable, self-cleaning git worktree (`trap cleanup EXIT INT TERM`); 01-08-SUMMARY.md records a real drill run (D8) with `human_judgment: true` and the transcript of a person confirming each guard's failure message named the planted defect; the drill's first run itself found and closed a real gap (Rosetta cannot detect an auto-dispatching FFT planner), evidencing this was actually exercised, not merely written |
| 10 | PNG, JPEG, WebP and TIFF each decode to a `Frame` with the same shape contract, behind a guarded, limit-checked decode entry point (SRC-01) | ✓ VERIFIED | `crates/chrys-source-raster/tests/formats.rs`: 5/5 tests pass (`png`, `jpeg`, `webp`, `tiff`, plus content-sniffed mismatched-extension case); `decode_guarded` sets `image::Limits` before allocation (decode.rs:74); UAT test 4 confirms all four formats report the same region live |

**Score:** 9/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chrys-source/src/lib.rs` | Frame, RegionHint, Source trait, no format dependency | ✓ VERIFIED | Present, documents "RGBA8, straight alpha" |
| `crates/chrys-source-raster/src/decode.rs` | Guarded decode entry point | ✓ VERIFIED | `decode_guarded` sets limits before allocation |
| `crates/chrys-source-raster/src/normalize.rs` | EXIF orientation, straight-alpha, bit-depth normalization | ✓ VERIFIED | `normalize_to_rgba8`, `Orientation` present |
| `crates/chrys-core/src/hash.rs` | Digest report over raw RGBA8/verdict text | ✓ VERIFIED | `rgba8_digest`, `DigestSet`, `digest_report` present and wired to `--hash-only` |
| `crates/chrys-core/src/register/{window,luma,phase_correlation,subpixel}.rs` | Global registration, pure-Rust FFT path | ✓ VERIFIED | `libm::cos` in `hann_table`; `plan_scalar_fft`; known-shift tests pass |
| `crates/chrys-core/src/register/confidence.rs` | Peak-confidence metric and calibrated refusal threshold | ✓ VERIFIED | `REFUSAL_THRESHOLD` (2313.88) carries measured bounds in its doc comment; `tests/refusal.rs` re-derives them from the corpus |
| `crates/chrys-core/src/register/{warp,integral,block_match}.rs` | Local registration, summed-area table, residual field | ⚠️ PARTIAL (see gap) | Present, wired, integer-exact and constant-cost per the review, but `difference_image` silently drops the alpha channel from the residual it produces — see gap on Truth 1 |
| `crates/chrys-core/src/classify/{label,antialias,colour,kind}.rs` | Labelled regions, antialiasing suppression, colour delta, kind classification | ✓ VERIFIED (RGB only) | `label_regions` (owned union-find, no `imageproc`/`image` transitively), `colour_delta` (CIE76 Lab via `palette`+`libm`), `classify_kind`; all present and wired. Foreground test is RGB-only, per the gap above. |
| `crates/chrys-core/tests/determinism.rs` | Transcendental, FMA, dependency, FFT-planner guards | ✓ VERIFIED | 6 guards present, all pass live |
| `.github/workflows/determinism.yml` | Six-runner matrix + cross-runner agree job | ✓ VERIFIED | Confirmed green on latest push via `gh run view` |
| `scripts/cross-arch-hash.sh`, `scripts/determinism-drill.sh` | Re-runnable local checks | ✓ VERIFIED | Present, substantive, self-cleaning |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `chrys-source-raster/src/lib.rs` | `decode.rs` | `Source::load` calls `decode_guarded` | ✓ WIRED | Confirmed by grep and by `formats.rs` passing |
| `crates/chrys-cli/src/main.rs` | `crates/chrys-core/src/hash.rs` | `--hash-only` prints `digest_report` | ✓ WIRED | Live `--hash-only` run on the alpha-only fixture printed all four digest lines |
| `.github/workflows/determinism.yml` | `crates/chrys-cli/src/main.rs` | matrix runs `chrys compare --hash-only` | ✓ WIRED | Confirmed in workflow source and in live `gh run view` job list |
| `crates/chrys-core/src/lib.rs` | `register/confidence.rs` | `compare` calls `assess_peak` before warp/classify | ✓ WIRED | Live refusal on `should-refuse/pair-01` confirms this path runs before a verdict is produced |
| `crates/chrys-core/src/residual.rs` | `register/block_match.rs` | `residual_rgba8` renders the real `ResidualField` | ✓ WIRED | Digest of the alpha-only fixture's `residual` line differs from an all-identical case, confirming a live field, not a static stub — but see gap: the field itself is RGB-only |
| `crates/chrys-core/src/lib.rs` | `classify/mod.rs` | `compare` hands `ResidualField` to classify | ✓ WIRED | Confirmed by `sequence.rs` call chain and by classify tests passing |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Recoloured region reported with delta + both colours | `chrys compare tests/golden/pair-01/{base,candidate}.png` | `Recoloured region at x=64, y=64, width=96, height=64, colour delta 111.83 (base [40, 90, 200, 255], candidate [200, 90, 40, 255])` | ✓ PASS |
| Unregisterable pair is refused with exit 2 | `chrys compare tests/golden/refuse-01/should-refuse/pair-01/{base,candidate}.png` | `refused: ... peak confidence 976.82 is below the threshold 2313.88 ...`, exit 2 | ✓ PASS |
| **Alpha-only difference is detected** | Independently generated 64x64 RGBA PNG pair, identical RGB, a 20x20 block with alpha 0 vs 255 (via a temporary `image`-crate example, not committed); `chrys compare base.png candidate.png` and `--hash-only` | `identical`, exit 0, while `decode-base` and `decode-candidate` digests differ (`a8a8fc5b...` vs `1d629 5ef...`) | ✗ FAIL — this is the CONFIRMED DEFECT (CR-01) |
| Full workspace test suite is green | `cargo test --workspace --release` | 0 failures across ~23 test binaries (unit + integration), matching the orchestrator's reported 175-test count | ✓ PASS |
| `cargo clippy --workspace --all-targets --release` is clean | as run | No warnings, finished clean | ✓ PASS |
| `chrys-core` dependency graph excludes GPU/format crates | `cargo tree -p chrys-core -e normal` | Only `chrys-source, libm, palette, rustfft, sha2, thiserror` + non-format/non-GPU transitives | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| CORE-01 | 01-01, 01-07 | Names each change by kind, never a bare pixel count | ✗ BLOCKED (partial) | Holds for RGB-visible changes (UAT 1, 6); fails for alpha-only changes (CR-01), which are not named at all, let alone by kind |
| CORE-02 | 01-04, 01-06 | Registers a translated region and reports the offset | ✓ SATISFIED | UAT test 5; `register.rs` known-shift tests; `block_match.rs` |
| CORE-03 | 01-07 | Reports a colour change as a value + two colours | ✓ SATISFIED | UAT test 2; `classify/colour.rs` |
| CORE-04 | 01-07 | Groups changed pixels into labelled regions with bounding boxes | ⚠️ PARTIAL | Holds for RGB residuals (21/21 `classify.rs` tests pass); does not hold for alpha-only residuals (CR-01) |
| CORE-05 | 01-07 | Does not report an antialiasing difference a person cannot see | ✓ SATISFIED | `classify/antialias.rs`; UAT test 13 (human-accepted) |
| CORE-06 | 01-05 | Refuses a pair it cannot register, states why | ✓ SATISFIED | UAT test 7; live refusal reproduced independently |
| CORE-07 | 01-05 | Compares only near-identical pairs, says so when too different | ✓ SATISFIED | `REFUSAL_THRESHOLD` calibrated from corpus; `tests/refusal.rs` |
| DET-01 | 01-03, 01-08 | Identical verdict on Linux, macOS, Windows | ✓ SATISFIED | 6-runner CI `agree` job green |
| DET-02 | 01-03, 01-08 | Identical verdict on x86-64 and aarch64 | ✓ SATISFIED | 6-runner CI `agree` job green; local `cross-arch-hash.sh` also confirmed by SUMMARY |
| DET-03 | 01-03, 01-08 | CI proves DET-01/DET-02 on every commit, hashing raw RGBA8 | ✓ SATISFIED | Live `gh run list`: green on every recent push; `--hash-only` hashes raw decode/residual bytes |
| DET-04 | 01-01, 01-08 | No pixel entering comparison is GPU-produced | ✓ SATISFIED | `cargo tree` measured clean; dependency guard passes live |
| DET-06 | 01-04, 01-08 | No platform transcendental on the comparison path; FP contraction disabled | ✓ SATISFIED | Guards pass live; zero `f32`/`f64` in `integral.rs`; `palette` pinned to `libm` feature |
| SRC-01 | 01-01, 01-02 | Raster pair compared (PNG, JPEG, WebP, TIFF) | ✓ SATISFIED | `formats.rs` 5/5 tests pass; UAT test 4 |

No orphaned requirements: the 13 IDs declared across the eight plans (SRC-01, CORE-01–07, DET-01, DET-02, DET-03, DET-04, DET-06) exactly match REQUIREMENTS.md's Phase 1 traceability row and the phase's declared requirement list.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/chrys-core/src/register/warp.rs` | 69-83 | `difference_image` silently discards the alpha channel (`for channel in 0..3`, alpha forced to `u8::MAX`) with no doc comment stating this is deliberate | 🛑 Blocker | Root cause of CR-01/Truth 1 failure |
| `crates/chrys-core/src/classify/label.rs` | 159-173 | Foreground test reads only R, G, B residual bytes | 🛑 Blocker (same root cause) | Same as above |

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers found in any file modified by this phase (`crates/`, `scripts/`, `.github/workflows/determinism.yml`).

No stub patterns (`return null`, empty handlers, hardcoded empty data flowing to output) were found — every truth that failed, failed on a real, wired, tested code path producing a genuinely wrong answer, not an unimplemented stub.

## Deferred Items

None. Alpha-channel handling is not mentioned anywhere in ROADMAP.md, REQUIREMENTS.md or PROJECT.md as scoped to a later phase, so this is not a deferred item — it is an unaddressed gap in the current phase.

## Human Verification Required

None. All items above were resolved by direct code inspection, a live full test run, a live CI check via `gh run view`, and an independently reproduced repro of the alpha-channel defect (not sourced from SUMMARY.md's or the code review's claims alone).

## Gaps Summary

Nine of ten observable truths hold, with strong, independently-reproduced
evidence: the four-format decode path, global and local registration, colour
and kind classification (for RGB-visible changes), the refusal gate and its
calibrated threshold, and — the phase's stated centrepiece — genuine,
currently-green, six-runner cross-platform/cross-architecture determinism on
raw RGBA8 output, confirmed live via `gh run view` rather than taken on
report or on the stale "no remote yet" note still sitting in ROADMAP.md.

The one gap is real and blocking. `chrys compare` on two PNGs whose RGB bytes
are pixel-identical and whose alpha channel differs (a realistic edit in
every format this phase decodes) prints `identical` and exits `0` —
independently reproduced in this verification, not merely quoted from the
code review. `Frame`'s own documented contract is "RGBA8, straight alpha,"
with no carve-out anywhere for alpha being out of scope, so this is a silent,
wrong "no change" verdict on a documented part of the compared data, not a
requirement the phase never promised. It is exactly the class of failure the
phase's own CORE-06/CORE-07 refuse-rather-than-guess principle exists to
catch, except here the tool signals no uncertainty at all — it asserts
sameness. This blocks the phase goal ("a person compares two raster images
and gets a structural verdict") for the alpha-channel case, and blocks
CORE-01/CORE-04 for that case, so the phase cannot be marked passed as is.

The code review (01-REVIEW.md) already found and fully diagnosed this as its
one Critical issue (CR-01) with a proposed fix; no commit since the review
(`386d9cd`, `8c8cf29`, HEAD) has closed it. This gap is close to done — a
narrow, well-understood fix in `difference_image` and `label_regions`,
plus one new test — not a redesign.

---

_Verified: 2026-09-07T09:33:22Z_
_Verifier: Claude (gsd-verifier)_
