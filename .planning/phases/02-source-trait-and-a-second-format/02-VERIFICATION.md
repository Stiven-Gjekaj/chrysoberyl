---
phase: 02-source-trait-and-a-second-format
verified: 2026-09-07T12:59:40Z
status: human_needed
score: 7/7 must-haves verified
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
covered_digest: "v1:sha256:d18ff3e1fa85b3ac5c039e7084c222ee95a2279a56f3bb919a86ad4593a2806f"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 6/7
  gaps_closed:
    - "The hints sidecar reader bounds its allocation against an oversized or malicious sidecar, the same way every other decode path in this project is bounded (CVE-2023-29408-class decompression-bomb mitigation)."
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Run `chrys compare` over a sequence or animation pair with on the order of 100 frames and scan the output for the changed ones."
    expected: "A person can find the changed frames without excessive scrolling or re-reading."
    why_human: "This is a judgment about output ergonomics at a scale no fixture in this phase exercises (the largest committed fixture is 11 frames). 02-UAT.md itself records this as `status: [pending]`, unchanged since the prior report — not a failure, and not something this fix touched."
---

# Phase 2: Source Trait and a Second Format Verification Report

**Phase Goal:** A new kind of input is added without a change to the comparison engine.
**Verified:** 2026-09-07T12:59:40Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure (commit `494fd99`)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A numbered frame sequence pair is compared frame by frame, at the same quality as a single raster pair (SC1, SRC-02) | ✓ VERIFIED | Regression check: `chrys compare tests/golden/sequence-01/{base,candidate}` still reports the eleven-frame shape from the prior round; `cargo test --workspace` includes the sequence adapter's own tests, all green. No file touched by the gap-closure commit lies on this path. |
| 2 | An animation pair (GIF, APNG, animated WebP) is compared frame by frame (SC2, SRC-03) | ✓ VERIFIED | Regression check: full workspace test run (186 passed, 0 failed) includes `chrys-source-animation`'s own test binary (6 tests, all green), unrelated to the hints fix. |
| 3 | The animation adapter is added with no change to any file in the comparison engine (SC3, SRC-08) — **the phase's central claim** | ✓ VERIFIED | Re-measured independently this round, not carried from the prior report: `git rev-parse "<commit>:crates/chrys-core"` returns the identical tree hash `63aad81ddee9939047b1436a33eed8f0896da409` at `d81381b`, `522575d`, `5b4c451`, and `d1c1e51` (all reachable from `HEAD`; `git merge-base --is-ancestor d1c1e51 HEAD` succeeds). `HEAD` itself resolves to a different tree (`c97a6fb7...`), and `git log --oneline d1c1e51..HEAD -- crates/chrys-core/` shows the only intervening commits are phase 1's own later alpha-channel fix (`0e6cccc`..`07d0788`), not anything from phase 2's waves — exactly the legitimate divergence the task brief named in advance. Ran `scripts/engine-boundary-drill.sh d81381b d1c1e51` live myself: exit 0, both drills behave as expected (planted-defect drill on range `494fd99..884287c` goes red and names `crates/chrys-core/src/lib.rs`; clean-range drill on `d81381b^..d1c1e51` stays silent). Also independently checked the two commit ids the orchestrator said stay unresolvable on purpose (`git cat-file -t` on both resolves, confirming they are real commit objects, and 02-04-PLAN.md/02-04-SUMMARY.md corroborate they name a planted-defect drill inside a disposable worktree) — consistent with commit `a6eba0b`'s own stated scope. |
| 4 | An input source supplies a named region as a hint, and the engine registers inside that region instead of searching for it (SC4, SRC-09) | ✓ VERIFIED, with the same honest caveat as before | Regression check: `cargo test -p chrys-cli --test region_hint` passes live (2/2), including the unknown-region-name refusal test. The mechanism is unchanged from the prior round — crop-then-compare, orchestrated in `chrys-cli`, with `chrys-core` gaining no concept of "region." This is unaffected by the sidecar fix, which only changes how the hint file is *read*, not how it is used. |
| 5 | The hints sidecar reader bounds its allocation against an oversized or malicious sidecar, the same way every other decode path in this project is bounded (CVE-2023-29408-class decompression-bomb mitigation) | ✓ VERIFIED — the prior FAILED gap is now closed | Direct code read of `crates/chrys-source-raster/src/hints.rs` confirms `read_hints_sidecar` calls `std::fs::metadata` and returns `RasterError::HintsTooLarge` **before** `std::fs::read_to_string` runs — the check genuinely gates the read, it is not merely present elsewhere in the file. `MAX_SIDECAR_BYTES = 1024 * 1024` (1 MiB) is neither too low (the real committed sidecar, `tests/golden/hint-01/base.hints.toml`, is 62 bytes — over 16,000x headroom) nor too high to bound anything (a sidecar names rectangles; a thousand-region file stays a few tens of KB). Live-reproduced the review's original attack, independently, not from a saved log: wrote a fresh 314,572,801-byte sidecar next to a real PNG and ran the actual `chrys` binary under `/usr/bin/time -l` — peak resident set size was 5,554,176 bytes (5.4 MiB), not the 318,324,736 bytes (303 MiB) the code review measured against the unfixed code. Also confirmed the refusal reaches the CLI as a clear, user-facing error naming the path, size, and limit (`/tmp/oversize.hints.toml is 1048587 bytes, which exceeds the sidecar limit of 1048576 bytes; a hints file names rectangles and does not reach this size`), exit non-zero — not silently swallowed. Ran the two new unit tests live: `a_sidecar_larger_than_the_limit_is_refused_before_it_is_read` and `a_sidecar_at_the_limit_is_still_read`, both pass, and the second proves the limit is not so low it refuses a valid sidecar at the boundary. |
| 6 | `compare_sequence` is the one comparison pipeline; a second copy failing the build, `compare` delegating to it, and a sequence pair with a mismatched frame count (or a malformed sidecar, or a hint that doesn't fit) refusing with a message naming the specifics | ✓ VERIFIED | `cargo test -p chrys-core --test determinism` passes live. Hint-validation unit tests in `crates/chrys-source/src/lib.rs` and the malformed/duplicate-name tests in `crates/chrys-source-raster/src/hints.rs` all pass live, none touched by the gap-closure commit's diff (`git show 494fd99` touches only `hints.rs` and `lib.rs`'s error enum). |
| 7 | `--hash-only` reports one digest block per frame index for a multi-frame pair, and the unchanged four lines for a single-frame pair, with the phase 1 digest regression (`tests/golden/pair-01/expected-digest.sha256`) still matching live output | ✓ VERIFIED | `cargo test --workspace` run live this round: 186 passed, 0 failed (summed myself from the raw per-binary `test result:` lines — two more than the prior round's 184, exactly the two new hints-size tests; no test disappeared or newly failed). `crates/chrys-cli/tests/digest.rs` is part of that total. |

**Score:** 7/7 truths verified

### Deferred Items

None.

### Advisory (Carried Forward From The Prior Round, Not New This Round)

These two items were already recorded as advisory in the stale report. Re-checked this round to confirm they have not become live violations; neither is new-scope from this re-verification's own anti-pattern scan, so neither is listed under the re-verification `advisory:` frontmatter key (which is reserved for unevidenced new-scope findings).

| # | Finding | Category | Status this round |
|---|---------|----------|--------------------|
| 1 | `DENIED_DEPENDENCY_CRATES` (`crates/chrys-core/tests/determinism.rs`) names 21 GPU/format crates but no `serde` or `toml` — WR-02 | architectural | Re-confirmed no live violation: `cargo tree -p chrys-core -e normal \| grep -iE "serde\|toml"` returns zero matches, same as the prior round. Still a blind spot in a guard, not a current defect. **I agree it stays advisory** — the gap-closure commit did not touch `chrys-core`'s dependency graph, and the invariant it would guard remains unbroken by measurement, not by luck of an untested path. |
| 2 | ROADMAP.md marks this phase `Mode: mvp`, but the goal text does not fit the User Story format and the phase was executed as standard goal-backward waves | other | Unchanged. Not re-litigated; not this round's concern. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chrys-source-raster/src/hints.rs` | `read_hints_sidecar`, bounded before the read | ✓ VERIFIED | `MAX_SIDECAR_BYTES` constant present; `std::fs::metadata` check runs and returns before `std::fs::read_to_string`; confirmed by direct read and by live reproduction of the original 314 MB attack (peak RSS 5.4 MiB, not 303 MiB) |
| `crates/chrys-source-raster/src/lib.rs` | `RasterError::HintsTooLarge { path, size, limit }` | ✓ VERIFIED | Present; error message names all three fields, confirmed live at the CLI |
| All artifacts carried from the prior round (`compare_sequence`, `SequenceSource`, `AnimationSource`, `Frame::crop_to_region`, `engine-boundary-drill.sh`, the CI workflow, the golden fixtures) | — | ✓ VERIFIED (regression only) | Full workspace test suite green (186/186); none of these files appear in the gap-closure commit's diff (`git show --stat 494fd99` touches exactly two files) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `crates/chrys-source-raster/src/lib.rs` (`RasterSource::load`) | `hints.rs` (`read_hints_sidecar`) | fills `Frame.hints` | ✓ WIRED | Unchanged from prior round; regression-confirmed live via the region-hint CLI test |
| `crates/chrys-source-sequence/src/sequence.rs` | `hints.rs` (`read_hints_sidecar`) | per-frame sidecar read through the same function | ✓ WIRED | Confirmed by code read: still calls the identical function; the gap-closure commit changed the function's internals, not its call sites |
| CLI (`main.rs`) | `RasterError::HintsTooLarge` | error propagates to a user-facing message and non-zero exit | ✓ WIRED | Confirmed live: ran the actual `chrys` binary against a real oversized sidecar, got the exact error text and a non-zero exit, not a panic or a silent pass |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Size check runs before the read, not after | direct code read of `hints.rs:76-95` | `std::fs::metadata` match arm returns `Err` before the `std::fs::read_to_string` call is reached | ✓ PASS |
| A sidecar matching the original review's exact attack size is refused cheaply | wrote a fresh 314,572,801-byte sidecar next to a real PNG; ran `target/debug/chrys compare` under `/usr/bin/time -l` | peak RSS 5,554,176 bytes (was 318,324,736 bytes in the code review's own reproduction against the unfixed code) | ✓ PASS |
| A sidecar exactly at the limit is still accepted (limit is not too aggressive) | `cargo test -p chrys-source-raster --lib hints::tests::a_sidecar_at_the_limit_is_still_read` | 1 passed | ✓ PASS |
| The refusal was watched failing (not a coincidentally-green test) | `git show 494fd99` commit message states the refusal test was run against the size check removed and went red alone | corroborated by re-reading the diff: the check is a single early-return `match` arm, trivially removable, and the other eight hints tests do not depend on it | ✓ PASS (documented, not independently re-broken this round — re-breaking the fix to re-prove a negative was judged unnecessary given the code is a single, easily-inspected early-return) |
| Full workspace test suite is green | `cargo test --workspace` | 186 passed, 0 failed (summed myself from raw per-binary lines) | ✓ PASS |
| `cargo fmt --check` clean | `cargo fmt --check` | exit 0, no output | ✓ PASS |
| `cargo clippy --workspace --all-targets` clean | `cargo clippy --workspace --all-targets` | no warnings | ✓ PASS |
| `chrys-core` tree byte-identical across the animation + drill wave, HEAD legitimately differs | `git rev-parse "<commit>:crates/chrys-core"` at 5 refs | `d81381b`/`522575d`/`5b4c451`/`d1c1e51` all `63aad81d...`; `HEAD` differs (`c97a6fb7...`), attributed to phase 1's alpha commits only | ✓ PASS |
| Engine-boundary drill passes on the real range | `bash scripts/engine-boundary-drill.sh d81381b d1c1e51` | `2 of 2 drills behaved as expected`, exit 0 | ✓ PASS |
| Dependency guard blind spot (WR-02) still has no live violation | `cargo tree -p chrys-core -e normal \| grep -iE "serde\|toml"` | no output (0 matches) | ✓ PASS (advisory confirmed still non-live) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| SRC-02 | 02-01, 02-02, 02-04 | A numbered frame sequence pair is compared frame by frame | ✓ SATISFIED | Truth 1, regression-confirmed |
| SRC-03 | 02-03, 02-04 | An animation pair (GIF, APNG, animated WebP) is compared frame by frame | ✓ SATISFIED | Truth 2, regression-confirmed |
| SRC-08 | 02-01, 02-02, 02-03, 02-04 | A new input family is added without a change to any code in the comparison engine | ✓ SATISFIED | Truth 3, independently re-measured this round at 5 refs including live `HEAD` |
| SRC-09 | 02-05 | An input source can supply named regions as a hint, and the engine registers inside a named region instead of searching for it | ✓ SATISFIED — the prior open gap is now closed | Truth 4 (feature works, regression-confirmed) and Truth 5 (the sidecar reader that delivers the hint is now bounded, live-reproduced) both hold |

No orphaned requirements: the four IDs declared across the five plans (SRC-02, SRC-03, SRC-08, SRC-09) exactly match REQUIREMENTS.md's Phase 2 traceability rows. (Note: REQUIREMENTS.md's own tracking table still marks all four `Pending` rather than `Complete` — this is the project's standard practice of updating that column at ship/milestone-completion time, not a verification defect; phase 1's rows show the same pattern was `Pending` until its own completion step.)

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers in `hints.rs` or `lib.rs` (the two files the gap-closure commit touched), confirmed by direct grep.

| ID | Issue | File | Severity | Status this round |
|---|---|---|---|---|
| CR-01 | Unbounded sidecar read | `crates/chrys-source-raster/src/hints.rs` | Critical | **CLOSED.** Fixed in `494fd99`; re-verified live with the original attack size, peak RSS drops from 303 MiB to 5.4 MiB. |
| WR-02 | `DENIED_DEPENDENCY_CRATES` omits `serde`/`toml` | `crates/chrys-core/tests/determinism.rs` | Warning | Unchanged, re-confirmed no live violation. Remains advisory by agreement (see Advisory section). |
| WR-01 | `engine-boundary-drill.sh`'s own `git diff` exit status is unchecked | `scripts/engine-boundary-drill.sh` | Warning | Unchanged, not touched by this round's fix; not re-litigated |
| WR-03 | `crop_to_region`'s pixel-copy loop repeats an overflow-prone `u32` multiply | `crates/chrys-source/src/lib.rs` | Warning | Unchanged, not touched by this round's fix; not re-litigated |
| WR-04 | `frame N` header differs between `--hash-only` and plain-text multi-frame output | `crates/chrys-cli/src/main.rs` | Warning | Unchanged, not touched by this round's fix; not re-litigated |
| IN-01 | `AnimationSource` attaches no hints to any frame (undocumented) | `crates/chrys-source-animation/src/lib.rs` | Info | Unchanged, not touched by this round's fix; not re-litigated |
| IN-02 | Animation/sequence limit-parity test checks hardcoded literals | `crates/chrys-source-animation/src/lib.rs` | Info | Unchanged, not touched by this round's fix; not re-litigated |

## Human Verification Required

### 1. Per-frame output readability at length (UAT test 12)

**Test:** Run `chrys compare` over a sequence or animation pair with on the order of 100 frames and scan the output for the changed ones.
**Expected:** A person can find the changed frames without excessive scrolling or re-reading.
**Why human:** Judgment about output ergonomics at a scale no fixture in this phase exercises (the largest committed fixture is 11 frames). 02-UAT.md still records `status: [pending]` — unchanged since the prior round, and untouched by the sidecar fix. Not treated as a failure; this is the sole reason overall status is `human_needed` rather than `passed`, per the decision tree (a non-empty human-verification section routes here even when every truth is otherwise verified).

## Gaps Summary

No gaps remain. The single gap from the prior verification round is closed:

- **CR-01 (sidecar DoS) — CLOSED.** `read_hints_sidecar` now calls `std::fs::metadata` and returns `RasterError::HintsTooLarge` before any read happens. This is not a check added elsewhere while the vulnerable read stays reachable — I confirmed the ordering directly in the source and then confirmed it empirically by re-running the code review's own attack independently: a fresh 314,572,801-byte sidecar against the live binary now peaks at 5.4 MiB resident memory, not 303 MiB. The 1 MiB limit is well-calibrated: the real committed fixture sidecar is 62 bytes (four orders of magnitude of headroom), and a test proves the exact boundary byte is still accepted, so the fix is not simply "refuse everything."

Everything else from the prior round still holds, re-confirmed rather than carried forward blindly:

- **Criterion 3 (SC3/SRC-08), the phase's central claim, still holds** — re-measured this round at `d81381b`, `522575d`, `5b4c451`, `d1c1e51` (all four now correctly citing commits reachable from `HEAD`, per commit `a6eba0b`'s repair), all four resolve to the identical `chrys-core` tree hash `63aad81d...`. `HEAD` itself legitimately differs, attributed by direct `git log` inspection to phase 1's own later alpha-channel commits only. The engine-boundary drill was re-run live this round and passes.
- **Criteria 1, 2, and 4 hold** — regression-confirmed via the full workspace test suite (186/186 green) and the region-hint CLI test, neither touched by the gap-closure commit's two-file diff.
- **WR-02 remains genuinely advisory** — re-measured this round, zero live violation, unchanged from before. I agree with the prior report's judgment that it stays advisory.
- **The two commit ids the orchestrator flagged as permanently unresolvable are correctly out of scope** — they name objects inside a disposable worktree created specifically to drill the engine-boundary check with a planted defect (02-04-PLAN.md/02-04-SUMMARY.md corroborate this), not history that ever belonged to the real branch.
- **UAT test 12 remains a legitimate, unjudgeable-by-machine item**, unchanged and untouched by this fix — carried forward as the sole human-verification item, which is why overall status is `human_needed` rather than `passed`.

---

_Verified: 2026-09-07T12:59:40Z_
_Verifier: Claude (gsd-verifier)_
