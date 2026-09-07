---
phase: 02-source-trait-and-a-second-format
verified: 2026-09-07T14:45:00Z
status: gaps_found
score: 6/7 must-haves verified
covered_files:
  - ".github/workflows/determinism.yml"
  - ".planning/REQUIREMENTS.md"
  - ".planning/ROADMAP.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-01-PLAN.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-01-SUMMARY.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-02-PLAN.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-02-SUMMARY.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-03-PLAN.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-03-SUMMARY.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-04-PLAN.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-04-SUMMARY.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-05-PLAN.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-05-SUMMARY.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-REVIEW.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-SECURITY.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-UAT.md"
  - ".planning/phases/02-source-trait-and-a-second-format/02-VALIDATION.md"
  - "crates/chrys-cli/src/main.rs"
  - "crates/chrys-cli/tests/digest.rs"
  - "crates/chrys-cli/tests/region_hint.rs"
  - "crates/chrys-core/src/lib.rs"
  - "crates/chrys-core/src/sequence.rs"
  - "crates/chrys-core/src/verdict.rs"
  - "crates/chrys-core/tests/determinism.rs"
  - "crates/chrys-source-animation/examples/make-fixtures.rs"
  - "crates/chrys-source-animation/src/lib.rs"
  - "crates/chrys-source-animation/src/sniff.rs"
  - "crates/chrys-source-animation/tests/animation.rs"
  - "crates/chrys-source-raster/examples/make-fixtures.rs"
  - "crates/chrys-source-raster/src/hints.rs"
  - "crates/chrys-source-raster/src/lib.rs"
  - "crates/chrys-source-sequence/src/lib.rs"
  - "crates/chrys-source-sequence/src/sequence.rs"
  - "crates/chrys-source-sequence/tests/sequence.rs"
  - "crates/chrys-source/src/lib.rs"
  - "crates/chrys-source/tests/manifest.rs"
  - "scripts/engine-boundary-drill.sh"
covered_digest: "v1:sha256:9cba13afc7acfef0c985bad9a2696406bba8d4c6531696097dcebf29930d9773"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "The hints sidecar reader bounds its allocation against an oversized or malicious sidecar, the same way every other decode path in this project is bounded (CVE-2023-29408-class decompression-bomb mitigation)."
    status: failed
    reason: >-
      read_hints_sidecar (crates/chrys-source-raster/src/hints.rs) reads the
      whole <stem>.hints.toml file with std::fs::read_to_string before parsing,
      with no size cap anywhere in the function. Confirmed live: grep for any
      size/limit constant in hints.rs returns zero matches. The code review
      (CR-01, rated critical) reproduced a 314,572,802-byte sidecar driving
      peak RSS to 318,324,736 bytes. This file is called for every raster file
      and every file in a sequence directory, on the same attacker-controlled
      trust boundary chrys-source-raster's own DecodeLimits and
      AnimationLimits/SequenceLimits already defend for image bytes. The
      security file (02-SECURITY.md) names this explicitly in its Deviations
      section and states it is "carried into 02-VERIFICATION.md as a gap, not
      resolved here" rather than accepting it as a risk.
    artifacts:
      - path: "crates/chrys-source-raster/src/hints.rs"
        issue: "read_hints_sidecar has no MAX_HINTS_SIDECAR_BYTES-style cap before std::fs::read_to_string"
    missing:
      - "A size check (std::fs::metadata) before the full read, returning a named error (e.g. RasterError::HintsTooLarge) above a small fixed ceiling (the code review suggests 1 MiB), matching the bound every other decode path in this crate already enforces."
deferred: []
advisory:
  - finding: "DENIED_DEPENDENCY_CRATES (crates/chrys-core/tests/determinism.rs) lists 21 GPU/format crate names but no entry for \"serde\" or \"toml\", so the transitive half of the project's own invariant (\"chrys-core declares no serde and no toml\") has no automated guard beyond chrys-source's own manifest line-scan."
    category: architectural
    reason: >-
      No live violation exists today (measured: 48 crates in chrys-core's
      dependency graph, zero matching serde or toml). This is a blind spot in
      a guard, not a current defect, and matches the judgment phase 1's own
      verification applied to comparable guard-coverage gaps (WR-02 there).
      Recorded for visibility, not counted as a gap.
    evidence_status: "confirmed present; no live violation measured"
  - finding: "ROADMAP.md marks this phase \"Mode: mvp\", but the phase goal (\"A new kind of input is added without a change to the comparison engine.\") does not match the User Story format (`gsd_run query user-story.validate` returns valid=false), and the five plans were structured as standard execute-phase waves with artifact/key-link must_haves, not SPIDR-sliced user stories."
    category: other
    reason: >-
      The phase's own artifacts (PLAN frontmatter, ROADMAP success criteria)
      are written in the conventional goal-backward shape this verification
      ran against, and produced a concrete, checkable verdict. Refusing to
      verify over a stale/mismatched mode tag would have discarded that work
      for a metadata inconsistency unrelated to what was actually built.
      Flagged so the mode tag can be corrected, not treated as a phase defect.
    evidence_status: "confirmed: mode tag present, user-story validator rejects the literal goal text"
---

# Phase 2: Source Trait and a Second Format Verification Report

**Phase Goal:** A new kind of input is added without a change to the comparison engine.
**Verified:** 2026-09-07T14:45:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A numbered frame sequence pair is compared frame by frame, at the same quality as a single raster pair (SC1, SRC-02) | ✓ VERIFIED | Live run: `chrys compare tests/golden/sequence-01/{base,candidate}` — 10 frame blocks, index 4 reports `Recoloured region at x=45, y=40, width=45, height=40, colour delta 115.65 ...`, exit 0 (rank reflects `Changed`, matches UAT test 1/2 exactly). `compare` is a thin wrapper over `compare_sequence` with a one-element slice (`crates/chrys-core/src/lib.rs`), so a single raster pair and a one-frame "sequence" go through the identical pipeline — "same quality" is not a separate claim to verify, it is architecturally the same code path. |
| 2 | An animation pair (GIF, APNG, animated WebP) is compared frame by frame (SC2, SRC-03) | ✓ VERIFIED | Live runs on all three committed fixtures (`tests/golden/formats/{gif,apng,webp-anim}`): all three report identical 5-frame shape, frame 3 `Recoloured region at x=43, y=43, width=50, height=40, colour delta 77.99 (base [200, 40, 40, 255], candidate [250, 200, 40, 255])`, exit 1 — byte-identical verdict text across all three containers, as the hand-off and UAT claim. |
| 3 | The animation adapter is added with no change to any file in the comparison engine (SC3, SRC-08) — **the phase's central claim** | ✓ VERIFIED | Independently re-measured, not taken from the hand-off: `git rev-parse "<commit>:crates/chrys-core"` returns the identical tree hash `63aad81ddee9939047b1436a33eed8f0896da409` at `d81381b` (GIF), `522575d` (APNG), and `5b4c451` (drill). One correction to the hand-off's own citation: the fourth hash it gave, `d1c1e51` (animated WebP), is an **orphaned commit** — `git merge-base --is-ancestor d1c1e51 HEAD` fails, confirming it is not reachable from the current branch (this repo's history was rewritten once already, per phase 1's own verification notes on its reflog). The live commit carrying the same change, `d1c1e51` ("Assemble an animated WebP fixture and prove it round-trips"), **is** an ancestor of `HEAD` and independently produces the same tree hash, `63aad81d...`. The underlying claim holds on the actual branch history; only the citation needed correcting. Also confirmed: `git diff --name-only d1c1e51/d1c1e51..HEAD -- crates/chrys-core/` shows the only post-wave-3 `chrys-core` changes are phase 1's own later alpha-channel gap-closure commits, not anything from the region-hint wave (02-05) either — `git log --oneline c11a9ef~1..27f7687 -- crates/chrys-core/` is empty. Also ran `scripts/engine-boundary-drill.sh d81381b d1c1e51` live myself: both drills pass (planted-defect drill names the planted file; clean-range drill over the real wave stays silent), exit 0. |
| 4 | An input source supplies a named region as a hint, and the engine registers inside that region instead of searching for it (SC4, SRC-09) | ✓ VERIFIED, with an honest caveat | Live run on `tests/golden/hint-01`: whole-frame compare reports two changed regions (one inside, one outside the "logo" rectangle named in `base.hints.toml`/`candidate.hints.toml`), exit 1; `--region logo` on the same pair reports `identical`, exit 0 — the region-scoped answer genuinely differs from the whole-frame answer, which is what "registers inside that region instead of searching for it" cashes out to. **Caveat, asked for directly:** the mechanism is crop-then-compare, orchestrated entirely in `chrys-cli` (`crop_frames_to_region` calls `Frame::crop_to_region`, then hands the cropped frames to the unmodified `compare_sequence`) — confirmed zero `chrys-core` diff across the whole wave 5 commit range. `chrys-core` itself has no concept of "region" or "hint" at all; it never gains the ability to register *within* a subregion of a larger frame it's handed. What actually happens is that the engine's own registration/search code (`register/block_match.rs`, `register/phase_correlation.rs`) only ever sees the pixels inside the named rectangle, because the input itself was narrowed before the engine ran. Given this phase's overriding architectural constraint (SRC-08: the engine gains zero format- or hint-specific knowledge), this is the only design that could satisfy both SC3 and SC4 at once, and it is not a rename of a different feature — the region-scoped comparison materially differs from the whole-frame one, on a real fixture, live. But a reader expecting the engine itself to have learned to target a region, rather than simply never seeing pixels outside it, should know that is not what was built. |
| 5 | The hints sidecar reader bounds its allocation against an oversized/malicious sidecar, the way every other decode path in this project already does (derived from 02-SECURITY.md's own T-02-20 invariant and its Deviations section, which explicitly defers this decision to this report) | ✗ FAILED | Confirmed by direct code read: `read_hints_sidecar` (`crates/chrys-source-raster/src/hints.rs:62-80`) calls `std::fs::read_to_string(&sidecar_path)` with no size check anywhere before it — `grep -c "MAX_HINTS\|max_size\|limit" hints.rs` returns 0. This is the code review's one Critical finding (CR-01), reproduced by the reviewer (a 314,572,802-byte sidecar drives peak RSS to 318,324,736 bytes) and independently confirmed unfixed in the tree at the current `HEAD`. `read_hints_sidecar` runs on every raster file and every file in a sequence directory, the identical attacker-controlled trust boundary this project's own `DecodeLimits`/`AnimationLimits`/`SequenceLimits` are built to defend. 02-SECURITY.md's own Deviations section states plainly that this is "carried into 02-VERIFICATION.md as a gap, not resolved here" — the project's own artifacts ask this report to make the call, and the call is: unfixed, reproducible, tied directly to the SRC-09 deliverable, and contradicts a documented project invariant. See Gaps Summary and frontmatter `gaps`. |
| 6 | `compare_sequence` is the one comparison pipeline; a second copy failing the build, `compare` delegating to it, and a sequence pair with a mismatched frame count (or a malformed sidecar, or a hint that doesn't fit) refusing with a message naming the specifics | ✓ VERIFIED | `cargo test -p chrys-core --test determinism` passes live, including `one_comparison_pipeline`-class guards. Unit tests in `crates/chrys-source/src/lib.rs` (`a_hint_wider_than_the_frame_is_refused`, `a_hint_whose_origin_sits_outside_the_frame_is_refused`, `a_hint_whose_x_plus_width_overflows_a_u32_is_refused_rather_than_wrapping`, `a_hint_of_zero_width_is_refused`) and `crates/chrys-source-raster/src/hints.rs` (`invalid_toml_fails_with_a_message_naming_the_line`, `an_unknown_key_fails_rather_than_being_ignored`, `a_sidecar_naming_the_same_region_twice_is_refused`) all pass live, each asserting the refusal names the specific field/rectangle/line. |
| 7 | `--hash-only` reports one digest block per frame index for a multi-frame pair, and the unchanged four lines for a single-frame pair, with the phase 1 digest regression (`tests/golden/pair-01/expected-digest.sha256`) still matching live output | ✓ VERIFIED | `cargo test --workspace`: 184 passed, 0 failed, counted directly from the raw per-binary `test result:` lines, matching the orchestrator's own count. `crates/chrys-cli/tests/digest.rs` (part of that total) exercises both branches and the committed four-line regression file. |

**Score:** 6/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chrys-core/src/sequence.rs` | `compare_sequence`, the one comparison pipeline | ✓ VERIFIED | Present; `compare` in `lib.rs` delegates to it via `std::slice::from_ref` |
| `crates/chrys-source-sequence/src/lib.rs`, `sequence.rs` | `SequenceSource`, natural-order directory listing | ✓ VERIFIED | `impl Source for SequenceSource` present; natural sort confirmed live (`frame2.png` before `frame10.png`, UAT test 2) |
| `crates/chrys-source-animation/src/lib.rs`, `sniff.rs` | `AnimationSource`, GIF/APNG/WebP sniff | ✓ VERIFIED | `impl Source for AnimationSource` present; `is_animation` reads content signature, not extension (confirmed by code read) |
| `crates/chrys-source/src/lib.rs` | `Frame::crop_to_region`, `RegionOutOfBounds` | ✓ VERIFIED | Present; checked-arithmetic bounds test confirmed by read; 6 unit tests pass live |
| `crates/chrys-source-raster/src/hints.rs` | `read_hints_sidecar`, the `<stem>.hints.toml` reader | ⚠️ VERIFIED BUT UNSAFE | Present, wired, and functionally correct for well-formed input — but see Truth 5/Gap above: no size bound |
| `scripts/engine-boundary-drill.sh` | The drill proving the boundary check can fail | ✓ VERIFIED | Ran live myself with real commit range; both drills pass, exit 0 |
| `.github/workflows/determinism.yml` | Six-runner digest job extended with new fixtures | ✓ VERIFIED | `sequence-01`, `formats/gif`, `formats/apng`, `formats/webp-anim` all present in the job; `gh run view 34116770150` (commit `27f34ff`, a descendant of every feature commit including region-hint wave 5) shows all 8 jobs (`guards`, 6×`digest`, `agree`) green |
| `tests/golden/sequence-01/`, `tests/golden/formats/{gif,apng,webp-anim}/`, `tests/golden/hint-01/` | Committed fixtures | ✓ VERIFIED | All present on disk; all exercised live above |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `crates/chrys-cli/src/main.rs` (`frames_for`) | `SequenceSource` / `AnimationSource` / `RasterSource` | dispatch by directory / sniff / fallback | ✓ WIRED | Live: directory, GIF/APNG/WebP-anim, and plain PNG inputs each reach the correct adapter |
| `crates/chrys-cli/src/main.rs` (`run_compare`) | `chrys_core::compare_sequence` | unmodified pipeline call, after optional `crop_frames_to_region` | ✓ WIRED | Live: region-scoped and whole-frame runs both reach `compare_sequence`; only the input frames differ |
| `crates/chrys-source-raster/src/lib.rs` | `hints.rs` (`read_hints_sidecar`) | `RasterSource::load` fills `Frame.hints` | ✓ WIRED | Confirmed by region-hint live run: `base.hints.toml`/`candidate.hints.toml` hints reach `Frame.hints` and are found by name |
| `crates/chrys-source-sequence/src/sequence.rs` | `hints.rs` (`read_hints_sidecar`) | per-frame sidecar read through the same function | ✓ WIRED | Confirmed by code read: `sequence.rs:99` calls the same function as the raster adapter |
| `crates/chrys-source-animation/src/lib.rs` | `hints.rs` | *(none — see Anti-Patterns)* | ⚠️ NOT WIRED (by design, undocumented) | `collect_frames` always pushes `hints: Vec::new()`; confirmed by code read. `--region` against any animated file always refuses (no region named). Plausibly intentional scope, but asserted by no test and stated in no doc comment (code review IN-01). Does not affect any of the four roadmap success criteria, which do not claim hints work on animated input — recorded as information, not a gap. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Sequence pair compares frame by frame | `chrys compare tests/golden/sequence-01/{base,candidate}` | 10 frames, index 4 `Recoloured...`, exit 0 | ✓ PASS |
| GIF pair compares frame by frame | `chrys compare tests/golden/formats/gif/{base,candidate}.gif` | 5 frames, frame 3 `Recoloured...`, exit 1 | ✓ PASS |
| APNG pair compares frame by frame | `chrys compare tests/golden/formats/apng/{base,candidate}.png` | identical shape to GIF result | ✓ PASS |
| Animated WebP pair compares frame by frame | `chrys compare tests/golden/formats/webp-anim/{base,candidate}.webp` | identical shape to GIF result | ✓ PASS |
| Whole-frame vs. region-scoped comparison genuinely differ | `chrys compare tests/golden/hint-01/{base,candidate}.png [--region logo]` | whole: 2 regions, exit 1; region: `identical`, exit 0 | ✓ PASS |
| Engine-boundary drill can fail and does pass on the real wave | `bash scripts/engine-boundary-drill.sh d81381b d1c1e51` | `2 of 2 drills behaved as expected`, exit 0 | ✓ PASS |
| Full workspace test suite is green | `cargo test --workspace` | 184 passed, 0 failed (summed from raw per-binary lines myself) | ✓ PASS |
| `chrys-core` tree is byte-identical across the animation + drill wave | `git rev-parse "<commit>:crates/chrys-core"` at 4 commits | all four: `63aad81ddee9939047b1436a33eed8f0896da409` | ✓ PASS |
| Hints sidecar reader has a size cap | `grep -c "MAX_HINTS\|max_size\|limit" crates/chrys-source-raster/src/hints.rs` | `0` | ✗ FAIL (CR-01) |
| `DENIED_DEPENDENCY_CRATES` names `serde`/`toml` | `grep -c '"serde"\|"toml"' crates/chrys-core/tests/determinism.rs` | `0` | ✗ FAIL (advisory, WR-02 — no live violation) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| SRC-02 | 02-01, 02-02, 02-04 | A numbered frame sequence pair is compared frame by frame | ✓ SATISFIED | Truth 1, live run |
| SRC-03 | 02-03, 02-04 | An animation pair (GIF, APNG, animated WebP) is compared frame by frame | ✓ SATISFIED | Truth 2, live runs on all three formats |
| SRC-08 | 02-01, 02-02, 02-03, 02-04 | A new input family is added without a change to any code in the comparison engine | ✓ SATISFIED | Truth 3, independently re-measured tree-hash proof, corrected citation, own drill run |
| SRC-09 | 02-05 | An input source can supply named regions as a hint, and the engine registers inside a named region instead of searching for it | ⚠️ SATISFIED WITH AN OPEN GAP | Truth 4 (feature genuinely works, live) but Truth 5 (the sidecar reader that delivers the hint is unbounded against a DoS) is FAILED |

No orphaned requirements: the four IDs declared across the five plans (SRC-02, SRC-03, SRC-08, SRC-09) exactly match REQUIREMENTS.md's Phase 2 traceability rows.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers in any file this phase's five plans declared as modified.

| ID | Issue | File | Severity | Blocks the phase goal? |
|---|---|---|---|---|
| CR-01 | Unbounded sidecar read (`std::fs::read_to_string`, no size cap) | `crates/chrys-source-raster/src/hints.rs` | Critical | **Yes — recorded as a gap.** Reproducible DoS in new code this phase shipped for SRC-09, on the project's own named trust boundary, explicitly deferred to this report by 02-SECURITY.md. |
| WR-02 | `DENIED_DEPENDENCY_CRATES` omits `serde`/`toml` | `crates/chrys-core/tests/determinism.rs` | Warning | No — advisory only. No live violation exists (0 of 48 crates in `chrys-core`'s graph match). A guard blind spot, not a current defect; phase 1's verification made the same call on its own comparable guard-coverage gaps. |
| WR-01 | `engine-boundary-drill.sh`'s own `git diff` exit status is unchecked | `scripts/engine-boundary-drill.sh` | Warning | No — the drill still passed live, twice, on real and planted ranges, during this verification. A false "clean pass" requires a malformed commit-range argument, which this verification did not supply. |
| WR-03 | `crop_to_region`'s pixel-copy loop repeats an overflow-prone `u32` multiply `pixel_count` avoids | `crates/chrys-source/src/lib.rs` | Warning | No — not reachable through any current adapter; `DecodeLimits` (16384×16384) keeps every produced `Frame` well under the wrap threshold |
| WR-04 | `frame N` header differs between `--hash-only` and plain-text multi-frame output | `crates/chrys-cli/src/main.rs` | Warning | No — the two branches agree today because every adapter assigns `index` from `enumerate()` in order; a future non-contiguous `Source` could surface this, not a live defect |
| IN-01 | `AnimationSource` attaches no hints to any frame (undocumented) | `crates/chrys-source-animation/src/lib.rs` | Info | No — no roadmap success criterion claims hints work on animated input |
| IN-02 | Animation/sequence limit-parity test checks hardcoded literals, not a cross-crate comparison | `crates/chrys-source-animation/src/lib.rs` | Info | No — no live drift exists |

## Human Verification Required

### 1. Per-frame output readability at length (UAT test 12)

**Test:** Run `chrys compare` over a sequence or animation pair with on the order of 100 frames and scan the output for the changed ones.
**Expected:** A person can find the changed frames without excessive scrolling or re-reading.
**Why human:** This is a judgment about output ergonomics at a scale no fixture in this phase exercises (the largest committed fixture is 11 frames). 02-UAT.md itself records this as `status: [pending]`, not a failure — carried forward here rather than re-litigated, per the UAT's own framing.

## Gaps Summary

One gap blocks clean completion of this phase's SRC-09 deliverable:

- **CR-01 (sidecar DoS).** `read_hints_sidecar` reads an entire `<stem>.hints.toml` into memory with no size limit, on the same attacker-controlled trust boundary (a directory a producer or attacker can plant files into) every other decode path in this project explicitly bounds. The code reviewer reproduced the failure mode directly (a ~300 MB sidecar driving peak RSS past 300 MB); this report confirmed the code is still unfixed at the current `HEAD` by direct read. 02-SECURITY.md's own Deviations section states this is deliberately left open for this report to decide, rather than accepted as a risk alongside R-05/R-06/R-07. The decision: this blocks. It is reproducible, critical-severity, new code this phase shipped specifically for one of its four required deliverables (SRC-09), and it contradicts a documented, otherwise-universal project invariant.

Everything else checked out:

- **Criterion 3, the phase's central claim, holds** — independently re-measured at four commits (one of the orchestrator's four citations, `d1c1e51`, turned out to be an orphaned pre-rewrite hash; the live equivalent commit, `d1c1e51`, was checked instead and produces the identical tree hash, so the underlying claim is unaffected).
- **Criteria 1, 2 and 4 hold**, demonstrated with my own live runs against the committed fixtures, not taken from SUMMARY.md or the hand-off.
- **Criterion 4's mechanism (crop-then-compare) is genuine, not a rename** — the region-scoped and whole-frame comparisons produce materially different, live-verified answers on the same pair — but it is worth a reader knowing explicitly that `chrys-core` itself gains no concept of "region"; narrowing happens entirely before the engine runs, which is the only design compatible with SRC-08's zero-engine-change constraint.
- The regression gate passed: 184 of 184 workspace tests, both engine-boundary drills, and CI green on a commit (`27f34ff`) confirmed to be a descendant of every feature wave including the region-hint wave.
- WR-02 (dependency-guard blind spot) and four other code-review Warnings/Infos are real but judged, individually, not to block the phase goal, on the same standard phase 1's own verification applied to comparable findings.
- UAT test 12 is a legitimate, unjudgeable-by-machine item, not treated as a failure.
- One metadata inconsistency (ROADMAP.md's `Mode: mvp` tag vs. a non-user-story goal and a phase actually planned/executed in the standard goal-backward shape) is recorded as advisory, not acted on as a refusal, since the phase's own artifacts supported a full, concrete verification.

---

_Verified: 2026-09-07T14:45:00Z_
_Verifier: Claude (gsd-verifier)_
