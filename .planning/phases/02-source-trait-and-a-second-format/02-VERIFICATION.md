---
phase: 02-source-trait-and-a-second-format
verified: 2026-09-07T16:10:00Z
status: passed
score: 8/8 must-haves verified
covered_files:
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
  - "crates/chrys-cli/tests/sequence_report.rs"
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
covered_digest: "v1:sha256:36aa470536b73130778b43516962128bc6bcabf3b8d93a7d7e9ad1779ac13574"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 7/7
  gaps_closed:
    - "UAT acceptance test 12 (per-frame output readability at scale) was pending in the prior round. It has since been run, failed against the committed shape, fixed in commit d6c1970, and passed in commit 233e34b. The deferred item was removed from ROADMAP.md's Phase 3 success criteria and from STATE.md's deferred list."
  gaps_remaining: []
  regressions: []
---

# Phase 2: Source Trait and a Second Format Verification Report

**Phase Goal:** A new kind of input is added without a change to the comparison engine.
**Verified:** 2026-09-07T16:10:00Z
**Status:** passed
**Re-verification:** Yes — after UAT test 12 was fixed and passed (commits `d6c1970`, `233e34b`), replacing the prior `human_needed` verdict.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A numbered frame sequence pair is compared frame by frame, at the same quality as a single raster pair (SC1, SRC-02) | ✓ VERIFIED | `cargo test -p chrys-source-sequence` and `cargo test -p chrys-cli --test sequence_report` pass live (11-frame committed fixture, frame 4 recolour reported). None of the sequence-adapter or engine code was touched by the two new commits; only `chrys-cli`'s report formatting changed. |
| 2 | An animation pair (GIF, APNG, animated WebP) is compared frame by frame (SC2, SRC-03) | ✓ VERIFIED | `cargo test -p chrys-source-animation` passes live (8 tests). Untouched by the readability commits. |
| 3 | The animation adapter is added with no change to any file in the comparison engine (SC3, SRC-08) — the phase's central claim | ✓ VERIFIED | Re-measured independently this round: `git rev-parse "<rev>:crates/chrys-core"` at `d81381b`, `522575d`, `5b4c451`, `d1c1e51` all resolve to the identical tree `63aad81ddee9939047b1436a33eed8f0896da409`; `HEAD:crates/chrys-core` differs (`c97a6fb7...`), and `git log --oneline d1c1e51..HEAD -- crates/chrys-core/` shows the only intervening commits are phase 1's own later alpha-channel work (`0e6cccc`..`07d0788`) — not the readability fix, which touched only `crates/chrys-cli`. Ran `bash scripts/engine-boundary-drill.sh d81381b d1c1e51` live: exit 0, both drills behave as expected (planted-defect drill names `crates/chrys-core/src/lib.rs`; clean-range drill stays silent). |
| 4 | An input source supplies a named region as a hint, and the engine registers inside that region instead of searching for it (SC4, SRC-09) | ✓ VERIFIED | `cargo test -p chrys-cli --test region_hint` passes live (2/2). Unaffected by the readability commits — the region-crop mechanism lives entirely in `chrys-cli`/`chrys-source`, neither of which was touched by `d6c1970`/`233e34b` outside of `main.rs`'s report-printing branch, which runs after cropping and comparison are already done. |
| 5 | The hints sidecar reader bounds its allocation against an oversized or malicious sidecar (CVE-2023-29408-class decompression-bomb mitigation, from the prior round's gap closure) | ✓ VERIFIED | Independently re-reproduced this round, not carried from the last report: wrote a fresh 314,572,802-byte sidecar next to a real PNG and ran the live release binary. Refused with `... is 314572802 bytes, which exceeds the sidecar limit of 1048576 bytes; a hints file names rectangles and does not reach this size`, exit code 3 (non-zero, not a panic), peak RSS ~3.3 MB (`/usr/bin/time -l`), not the hundreds of megabytes an unbounded read would cost. `cargo test -p chrys-source-raster --lib hints::` passes live, 9/9, including the at-limit and over-limit boundary tests. |
| 6 | A multi-frame default report prints only the frames whose verdict is not `Identical`, then a `{changed} of {total} frames changed` tail — so a person can find the changed frames in a long sequence without scrolling past hundreds of unchanged lines (UAT test 12, this round's central re-derivation) | ✓ VERIFIED | Read `crates/chrys-cli/src/main.rs`'s default branch: it skips `Verdict::Identical` frames and always prints the tail, even at zero changes. Ran the real committed `sequence-01` fixture live: default output is 11 lines (one header + one verdict line for the single changed frame, plus the tail); `--all-frames` on the same pair is 22 lines (2 lines × 11 frames). Built an independent 100-frame fixture (not committed, generated fresh this round) with 5 frames differing from base: default output is 11 lines naming exactly frames 5, 21, 22, 59, 86 plus `5 of 100 frames changed`; `--all-frames` on the same pair is 200 lines. This matches the exact 200→11 reduction UAT test 12 and the commit message claim, reproduced independently rather than taken on the report's word. Also confirmed a no-change pair prints `0 of 11 frames changed`, not empty output. |
| 7 | The regression is genuinely tested, not merely claimed: reverting the default branch to the old per-frame shape makes exactly the tests that guard the new shape fail, and none of the others | ✓ VERIFIED | Live-reproduced the "watched failing" claim myself rather than trusting the commit message: temporarily edited `main.rs`'s default branch back to the pre-fix per-frame-with-no-tail shape and ran `cargo test -p chrys-cli --test sequence_report`. Exactly 3 of 6 tests failed (`a_sequence_prints_only_the_frames_that_changed`, `a_sequence_closes_with_a_count_of_every_frame`, `a_sequence_with_no_change_still_says_how_much_it_compared`); the other 3 (`all_frames_returns_the_line_per_frame_shape`, `the_digest_report_still_covers_every_frame`, `a_single_pair_prints_neither_a_header_nor_a_tail`) stayed green, as expected since they guard shapes the regression did not touch. Restored the file with `git checkout` afterward; tree is clean and all 6 tests pass again. |
| 8 | `--hash-only` is unaffected by the readability change: it still reports every frame's four-line digest block regardless of which frames changed, so CI's cross-runner digest comparison cannot move because of this fix (judge item 1) | ✓ VERIFIED | Read the `hash_only` branch of `run_compare` (lines 107–145): it contains no reference to `all_frames` or to `Verdict::Identical` filtering, and is byte-for-byte the same code both before and after `d6c1970` (the diff for that commit touches only the non-`hash_only` branch and the CLI flag definition). Ran `cargo test -p chrys-cli --test sequence_report the_digest_report_still_covers_every_frame` live: passes, confirming `--hash-only` on the 11-frame fixture prints 11 `frame N` headers and no tail. `.github/workflows/determinism.yml`'s `digest` job builds `digest.txt` from `--hash-only` output and the `agree` job compares runners byte-for-byte against each other, not against a stored golden value, so there is no committed digest this change could have invalidated even if it had touched that path (it did not). |

**Score:** 8/8 truths verified

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | The report should name the file a frame came from, not only its zero-based index (fixture files are named from one; no fixed offset corrects the mismatch because the producer chooses file names) | Phase 3 | ROADMAP.md Phase 3 success criterion 6: "The report names the file a frame came from, not only its index. Phase 2 printed a zero-based index beside files named from one, and no fixed offset fixes that, because the producer names the files." Commit `233e34b` narrowed this criterion in the same commit that closed UAT test 12, and removed the broader "readability" wording that used to sit there. STATE.md's Deferred Items table is empty, confirming nothing else was left open. |

### Advisory (Carried Forward, Not New This Round)

| # | Finding | Category | Status this round |
|---|---------|----------|--------------------|
| 1 | `DENIED_DEPENDENCY_CRATES` (`crates/chrys-core/tests/determinism.rs`) names 21 GPU/format crates but no `serde` or `toml` — WR-02 | architectural | Re-confirmed no live violation: `cargo tree -p chrys-core -e normal \| grep -iE "serde\|toml"` returns zero matches. Neither readability commit touched `chrys-core`'s dependency graph or this test file. Still a blind spot in a guard, not a current defect. Remains advisory. |
| 2 | ROADMAP.md marks this phase `Mode: mvp`, but the goal text ("A new kind of input is added without a change to the comparison engine.") does not fit the User Story format, and the phase was executed as standard goal-backward waves | other | Unchanged from the prior round. Not re-litigated — this re-verification's scope is the UAT-12 closure, and this pre-existing condition is untouched by it. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chrys-cli/src/main.rs` | Default multi-frame report prints changed frames only, plus a tail; `--all-frames` restores the old shape; `--hash-only` untouched | ✓ VERIFIED | Read live; confirmed via `git show d6c1970` that the `hash_only` branch has zero diff lines |
| `crates/chrys-cli/tests/sequence_report.rs` | Six tests proving the new default shape, the `--all-frames` escape hatch, the `--hash-only` invariance, and the single-pair invariance | ✓ VERIFIED | New file, 6/6 tests pass live; 3 of them independently confirmed to fail when the fix is reverted |
| All artifacts carried from the prior round (`compare_sequence`, `SequenceSource`, `AnimationSource`, `Frame::crop_to_region`, `hints.rs`, `engine-boundary-drill.sh`, the CI workflow, the golden fixtures) | — | ✓ VERIFIED (regression) | Full workspace test suite green (192/192, summed live from raw per-binary `test result:` lines); none of these files appear in the `d6c1970`/`233e34b` diffs except `main.rs`'s report-printing branch |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `crates/chrys-cli/src/main.rs` (`run_compare`) | `chrys_core::compare_sequence` | unchanged call, `verdicts` vector then re-formatted for display | ✓ WIRED | Confirmed by direct read: the call site (`let verdicts = chrys_core::compare_sequence(...)`) is untouched by the readability commits; only the loop that prints `verdicts` afterward changed |
| `crates/chrys-cli/src/main.rs` (default branch) | `chrys_core::Verdict::Identical` | `matches!` filter decides which frames print | ✓ WIRED | Confirmed by direct read at `main.rs:164` |
| CLI `--hash-only` | `chrys_core::hash::digest_report` | per-frame digest block, one per index, regardless of verdict | ✓ WIRED | Confirmed live: `cargo test -p chrys-cli --test sequence_report the_digest_report_still_covers_every_frame` passes; all 11 frames produce a header in `--hash-only` mode |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Default output on the committed 11-frame fixture is short and names the changed frame | `./target/release/chrys compare tests/golden/sequence-01/{base,candidate}` | `frame 4` block + `1 of 11 frames changed`, exit 1 | ✓ PASS |
| Default output on a fresh 100-frame/5-change fixture is 11 lines, `--all-frames` is 200 | built fixture, ran release binary, `wc -l` | 11 vs 200, changed frames 5/21/22/59/86 all named | ✓ PASS |
| `--hash-only` is unaffected by the fix (no reference to `all_frames` in that branch) | direct code read of `main.rs:107-145` | confirmed no diff between pre- and post-fix hash_only branch (`git show d6c1970`) | ✓ PASS |
| Reverting the default branch to the old shape makes exactly 3 of 6 new tests fail | live-edited `main.rs`, ran `cargo test -p chrys-cli --test sequence_report`, then `git checkout` to restore | 3 failed (the ones guarding the new default), 3 passed (the ones guarding unrelated shapes); tree restored clean afterward | ✓ PASS |
| Sidecar DoS bound holds under a live-reproduced 314 MB attack | wrote fresh oversized sidecar, ran release binary under `/usr/bin/time -l` | refused with path/size/limit message, exit 3, peak RSS ~3.3 MB | ✓ PASS |
| `chrys-core` tree byte-identical across the animation + drill wave, `HEAD` legitimately differs | `git rev-parse "<rev>:crates/chrys-core"` at 5 refs | 4 wave-boundary refs identical (`63aad81d...`); `HEAD` differs, attributed only to phase 1's alpha commits | ✓ PASS |
| Engine-boundary drill passes on the real range | `bash scripts/engine-boundary-drill.sh d81381b d1c1e51` | `2 of 2 drills behaved as expected`, exit 0 | ✓ PASS |
| Full workspace test suite is green | `cargo test --workspace` | 192 passed, 0 failed (summed live; was 186 in the prior round, +6 new `sequence_report.rs` tests) | ✓ PASS |
| `cargo fmt --check` clean | `cargo fmt --check` | exit 0, no output | ✓ PASS |
| `cargo clippy --workspace --all-targets` clean | `cargo clippy --workspace --all-targets` | no warnings | ✓ PASS |
| Dependency guard blind spot (WR-02) still has no live violation | `cargo tree -p chrys-core -e normal \| grep -iE "serde\|toml"` | no output (0 matches) | ✓ PASS (advisory confirmed still non-live) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| SRC-02 | 02-01, 02-02, 02-04 | A numbered frame sequence pair is compared frame by frame | ✓ SATISFIED | Truth 1, regression-confirmed live |
| SRC-03 | 02-03, 02-04 | An animation pair (GIF, APNG, animated WebP) is compared frame by frame | ✓ SATISFIED | Truth 2, regression-confirmed live |
| SRC-08 | 02-01, 02-02, 02-03, 02-04 | A new input family is added without a change to any code in the comparison engine | ✓ SATISFIED | Truth 3, independently re-measured this round at 5 refs including live `HEAD` |
| SRC-09 | 02-05 | An input source can supply named regions as a hint, and the engine registers inside a named region instead of searching for it | ✓ SATISFIED | Truth 4 (feature works) and Truth 5 (the sidecar reader is bounded) both hold, live-reproduced |

No orphaned requirements: the four IDs declared across the five plans (SRC-02, SRC-03, SRC-08, SRC-09) exactly match REQUIREMENTS.md's Phase 2 traceability rows. REQUIREMENTS.md's tracking table still marks all four `Pending` rather than `Complete` — this is the project's standard practice of updating that column at ship/milestone-completion time (phase 1's rows showed the same pattern), not a verification defect.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers found in `crates/chrys-cli/src/main.rs`, `crates/chrys-cli/tests/sequence_report.rs`, `crates/chrys-source-raster/src/hints.rs`, or `crates/chrys-source-raster/src/lib.rs` (the files touched since the stale report, plus the previously-fixed sidecar files), confirmed by direct grep.

| ID | Issue | File | Severity | Status this round |
|---|---|---|---|---|
| CR-01 | Unbounded sidecar read | `crates/chrys-source-raster/src/hints.rs` | Critical | Remains **CLOSED**. Independently re-reproduced this round with a fresh 314 MB attack: peak RSS ~3.3 MB, not hundreds of MB. |
| WR-02 | `DENIED_DEPENDENCY_CRATES` omits `serde`/`toml` | `crates/chrys-core/tests/determinism.rs` | Warning | Unchanged, re-confirmed no live violation. Remains advisory. |
| WR-01 | `engine-boundary-drill.sh`'s own `git diff` exit status is unchecked | `scripts/engine-boundary-drill.sh` | Warning | Unchanged, not touched by the readability fix; not re-litigated |
| WR-03 | `crop_to_region`'s pixel-copy loop repeats an overflow-prone `u32` multiply | `crates/chrys-source/src/lib.rs` | Warning | Unchanged, not touched by the readability fix; not re-litigated |
| WR-04 | `frame N` header differs between `--hash-only` and plain-text multi-frame output | `crates/chrys-cli/src/main.rs` | Warning | Unchanged in shape; the readability fix adds a third header context (changed-only default) alongside the two the review already flagged. Not re-litigated as a new finding — same root cause. |
| IN-01 | `AnimationSource` attaches no hints to any frame (undocumented) | `crates/chrys-source-animation/src/lib.rs` | Info | Unchanged, not touched by the readability fix; not re-litigated |
| IN-02 | Animation/sequence limit-parity test checks hardcoded literals | `crates/chrys-source-animation/src/lib.rs` | Info | Unchanged, not touched by the readability fix; not re-litigated |

## Human Verification Required

None. The single item that previously routed this phase to `human_needed` — per-frame output readability at length (UAT test 12) — is now resolved: 02-UAT.md records `result: pass` with concrete before/after evidence (200 lines → 11 lines on a hundred-frame fixture), and this round independently reproduced the same shape and ratio on a freshly built 100-frame fixture rather than trusting the recorded observation. No other item in this phase requires human judgment.

## Gaps Summary

No gaps. The phase's prior blocking-for-humans item is closed:

- **UAT test 12 (per-frame readability at scale) — CLOSED.** Failed first against the committed 200-line-per-hundred-frames shape, fixed in `d6c1970` (only changed frames print, plus a `{changed} of {total} frames changed` tail; `--all-frames` preserves the old shape; `--hash-only` is untouched), and passed in the same round. This report independently reproduced the fix's core claim on a fresh 100-frame/5-change fixture (not the one referenced in the commit message) and got the same 200→11 reduction. It also independently reverted the fix and confirmed exactly the 3 tests meant to guard it go red, and none of the other 3 do.

Everything else already held and is re-confirmed rather than carried forward blindly:

- **Criterion 3 (SC3/SRC-08), the phase's central claim, still holds** — re-measured this round at `d81381b`, `522575d`, `5b4c451`, `d1c1e51`, all four resolve to the identical `chrys-core` tree hash. `HEAD` legitimately differs, attributed by direct `git log` inspection to phase 1's own later alpha-channel commits only, not to anything in the readability fix (which touches only `chrys-cli`). The engine-boundary drill was re-run live and passes.
- **Criteria 1, 2, and 4 hold** — regression-confirmed via the full workspace test suite (192/192 green) and the region-hint CLI test, neither meaningfully touched by the two readability commits (whose diff is confined to `chrys-cli`'s report-printing branch and its own test file).
- **The sidecar DoS bound (from the prior gap closure) remains closed** — independently re-attacked this round with a fresh 314 MB file; peak RSS ~3.3 MB, not hundreds of MB.
- **WR-02 remains genuinely advisory** — re-measured this round, zero live violation, unchanged from before.
- **The frame-naming deferral is correctly scoped to Phase 3** — ROADMAP.md's Phase 3 success criterion 6 names it explicitly, and STATE.md's Deferred Items table is empty, confirming nothing else was silently dropped.

---

_Verified: 2026-09-07T16:10:00Z_
_Verifier: Claude (gsd-verifier)_
