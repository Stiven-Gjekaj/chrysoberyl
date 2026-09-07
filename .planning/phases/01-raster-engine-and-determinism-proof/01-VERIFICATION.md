---
phase: 01-raster-engine-and-determinism-proof
verified: 2026-09-07T16:10:00Z
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
  - "crates/chrys-cli/tests/sequence_report.rs"
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
covered_digest: "v1:sha256:97fec856e8aee4a4a88374da52d6e021b7850f20fae92a2de9b34e3ff378cb96"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 10/10
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 1: Raster Engine and Determinism Proof Verification Report

**Phase Goal:** A person compares two raster images and gets a structural verdict that is byte-identical across OS and architecture.
**Verified:** 2026-09-07T16:10:00Z
**Status:** passed
**Re-verification:** Yes — the prior report (`verified: 2026-09-07T13:35:00Z`, commit `315b1b3`) was stale. Two commits landed on files inside phase 1's covered set since then: `a6eba0b` (rewrites six SUMMARY files' commit ids after a history rewrite orphaned them — documentation only, no behaviour change) and `d6c1970` (changes `crates/chrys-cli/src/main.rs` to print only the changed frames of a multi-frame sequence, plus a tail count — a phase-2 feature that touches a file phase 1 owns). This report re-derives every truth against the current `HEAD` (`3a1d211`), independently, rather than carrying the prior verdict forward.

## What this re-verification checked that the prior one could not

The prior report's evidence predates `d6c1970`. The one live risk from that
commit is whether it changed anything about a **single-pair** comparison,
since phase 1's contract is single-pair only (multi-frame sequence reporting
belongs to phase 2). I read `main.rs`'s current source, then independently
re-ran the binary rather than trusting either the prior report or `d6c1970`'s
own commit message.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Compares two raster images and names each change by kind; never a bare pixel count | ✓ VERIFIED | Live run, `tests/golden/pair-01`: `Recoloured region at x=64, y=64, width=96, height=64, colour delta 111.83 (base [40, 90, 200, 255], candidate [200, 90, 40, 255])`, exit 1. Live run, `tests/golden/alpha-01` (RGB identical, alpha differs — the regression phase 1's own gap closure added): `Removed region at x=48, y=48, width=24, height=20`, exit 1 — still correctly named by kind, not `identical`. |
| 2 | Reports a translated region's offset in pixels, and a colour change's difference value and both colours | ✓ VERIFIED | Unaffected by `d6c1970` (no code in `verdict.rs` or `classify/` changed). `01-UAT.md` tests 2, 3, 5 pass; test 5's `Moved region ... moved by (-2, 1)` string is produced by code this commit did not touch. |
| 3 | Does not report an antialiasing difference a person cannot see | ✓ VERIFIED | `cargo test -p chrys-core --release --test classify`: 28/28 pass live, including both alpha-antialiasing regression tests (`suppress_antialiasing_on_an_alpha_expressed_diagonal_edge_yields_no_region`, `suppress_antialiasing_keeps_a_solid_alpha_only_change`). `antialias.rs` is untouched since the prior verification. |
| 4 | Refuses a pair it cannot register and states why | ✓ VERIFIED | Live run, `tests/golden/refuse-01/should-refuse/pair-01`: `refused: the pair is too different to register: peak confidence 976.82 is below the threshold 2313.88; this engine compares near-identical pairs only`, exit 2. `cargo test -p chrys-cli --release --test refusal`: 2/2 pass live. |
| 5 | CI hashes raw RGBA8 output of the same pair on Linux, macOS, Windows, x86-64 and aarch64, and the hash matches on every commit | ✓ VERIFIED | `git rev-parse HEAD` = `3a1d2114ba4817b80606a7b4e38f3683913c2ba7`. `gh run list --json headSha,...` shows the newest run (`34129949869`, created `2026-09-07T13:53:48Z`) has `headSha` exactly matching current `HEAD` — not an orphaned pre-rewrite SHA. `gh run view 34129949869 --json headSha,jobs`: all 8 jobs `success` — `guards`, `digest (ubuntu-24.04)`, `digest (ubuntu-24.04-arm)`, `digest (macos-15)`, `digest (macos-15-intel)`, `digest (windows-2022)`, `digest (windows-11-arm)`, `agree`. No run is in progress; this is the latest run and it is complete. `git reflog` confirms a `filter-branch: rewrite` produced the current `HEAD` (the telemetry-file removal the task described), and the CI evidence cited is for that rewritten tip, not a pre-rewrite ancestor. |

**Single-pair stdout is unaffected by `d6c1970` (the specific risk this re-verification was opened to check):**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 6 | A single-pair comparison prints no frame header and no tail line; the CLI's single-pair contract from phase 1 is untouched | ✓ VERIFIED | Read `crates/chrys-cli/src/main.rs:150-151` live: the `verdicts.len() == 1` branch is still exactly `print!("{}", verdicts[0]);`, with no header/tail code reachable on that path. Live run on `tests/golden/pair-01`: `grep -c "^frame "` → 0, `grep -c "frames changed"` → 0. Built the pre-`d6c1970` binary from `d6c1970^` in a disposable worktree and diffed stdout byte-for-byte on both `pair-01` and `alpha-01`: `cmp` reports **identical**, and both exit codes (1, 1) are unchanged. `--hash-only` output on `pair-01` is also byte-identical pre/post. The committed regression test `crates/chrys-cli/tests/sequence_report.rs::a_single_pair_prints_neither_a_header_nor_a_tail` asserts the same and passes live (`cargo test -p chrys-cli --release --test sequence_report`: 6/6 pass). |
| 7 | Exit codes still carry the verdict: 0 identical, 1 differs, 2 refused, 3 unreadable (UAT test 10) | ✓ VERIFIED | Re-ran live, not read from UAT.md: identical pair → exit 0; `pair-01` differs → exit 1; `refuse-01/should-refuse/pair-01` → exit 2, with a stated reason on stdout; `chrys compare /nonexistent/... /nonexistent/...` → exit 3, "cannot read ... No such file or directory" on stderr. All four match. |
| 8 | Two decodes/runs of the same pair produce byte-identical digests; the full suite is green | ✓ VERIFIED | `cargo test --workspace --release`: summed every `test result:` line's passed count myself — 192 passed, 0 failed (up from the prior report's 184, because `d6c1970` added 6 new tests in `sequence_report.rs`, all passing). `cargo clippy --workspace --all-targets --release`: 0 warnings. `cargo fmt --check`: exit 0. |
| 9 | Every determinism guard and drill still pass, unedited | ✓ VERIFIED | `cargo test -p chrys-core --release --test determinism` unaffected (no file it covers changed). `crates/chrys-cli/tests/refusal.rs`, `crates/chrys-source-raster/tests/formats.rs` (5/5) both pass live. No new crate dependency: `d6c1970` only edits `main.rs` and adds a CLI test file. |
| 10 | The five open code-review WARNINGS (WR-01..WR-05) still do not block the phase goal | ✓ VERIFIED | Re-read `01-REVIEW.md` live. None of the five target files touched by `d6c1970` (`main.rs`'s CLI dispatch is not among WR-01..WR-05's cited files: `determinism.rs`, `decode.rs`/`lib.rs`, `warp.rs`/`integral.rs`, `kind.rs`). `d6c1970`'s own diff introduces no `debug_assert`-only public API, no new transcendental call, and no new format-detection code — none of the five warning categories apply to the new code. Re-judged unchanged from the prior report: latent/theoretical, scoped to an unused configuration path, or explicitly out of scope; none blocks the phase goal. |

**Score:** 10/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chrys-cli/src/main.rs` | Single-pair (`verdicts.len() == 1`) branch stays exactly `print!("{}", verdicts[0])`, untouched by the sequence-reporting change | ✓ VERIFIED | Read live: confirmed at lines 150-151; the new `all_frames`/tail logic is only reachable when `verdicts.len() != 1` |
| `crates/chrys-cli/tests/sequence_report.rs` | New file; holds a dedicated single-pair regression test plus the 5 sequence-report shape tests | ✓ VERIFIED | 6/6 tests pass live, including `a_single_pair_prints_neither_a_header_nor_a_tail` |
| `crates/chrys-cli/tests/alpha.rs` | End-to-end alpha regression test, from the prior gap closure | ✓ VERIFIED | 2/2 tests pass live, unaffected by `d6c1970` |
| `.github/workflows/determinism.yml` | Six-runner digest matrix, `agree`, `guards`, unaffected by the CLI-only change | ✓ VERIFIED | Confirmed green on current `HEAD` via `gh run view`; workflow file untouched since the prior verification |
| `.planning/phases/.../01-0{1..9}-SUMMARY.md` | Commit ids point at live history after the force-push | ✓ VERIFIED | `a6eba0b`'s rewrite is a documentation-only edit; spot-checked one rewritten id (`01-09-SUMMARY.md`) resolves with `git cat-file -e`, confirming it is a real, reachable commit |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `crates/chrys-cli/src/main.rs` (`run_compare`) | `chrys_core::compare_sequence` | single-pair path returns exactly 1 verdict, taking the untouched print branch | ✓ WIRED | Live: `pair-01` and `alpha-01` both produce `verdicts.len() == 1`, confirmed by the absence of any `frame` header in stdout |
| `crates/chrys-cli/src/main.rs` | `hash.rs::digest_report` | `--hash-only` on a single pair still prints the four-line digest report, unaffected by the sequence-reporting change | ✓ WIRED | Live `--hash-only` run byte-identical to the pre-`d6c1970` binary's output |
| `.github/workflows/determinism.yml` | `chrys-cli` | matrix runs `chrys compare --hash-only`, digest jobs unaffected by a CLI change that only touches the non-hash-only verdict-text path | ✓ WIRED | Confirmed in workflow source (`--hash-only` always used) and in the live green `gh run view` job list on current `HEAD` |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Single-pair stdout, byte-for-byte, before vs. after `d6c1970` | Built `d6c1970^` in a disposable worktree; `cmp` against current binary's stdout on `pair-01` and `alpha-01` | identical on both pairs; exit codes unchanged (1, 1) | ✓ PASS |
| Single-pair `--hash-only`, byte-for-byte, before vs. after `d6c1970` | Same worktree comparison on `pair-01 --hash-only` | identical | ✓ PASS |
| Exit code 0 (identical) | `chrys compare pair-01/base.png pair-01/base.png` | `identical`, exit 0 | ✓ PASS |
| Exit code 1 (differs) | `chrys compare pair-01/base.png pair-01/candidate.png` | `Recoloured region ...`, exit 1 | ✓ PASS |
| Exit code 2 (refused) | `chrys compare refuse-01/should-refuse/pair-01/{base,candidate}.png` | `refused: ... peak confidence 976.82 is below the threshold 2313.88 ...`, exit 2 | ✓ PASS |
| Exit code 3 (unreadable) | `chrys compare /nonexistent/base.png /nonexistent/candidate.png` | `cannot read /nonexistent/base.png: No such file or directory`, exit 3 | ✓ PASS |
| Alpha-only regression still detected, not `identical` | `chrys compare alpha-01/{base,candidate}.png` | `Removed region at x=48, y=48, width=24, height=20`, exit 1 | ✓ PASS |
| Full workspace test suite is green | `cargo test --workspace --release` | 192 passed, 0 failed (counted from raw `test result:` lines; up from 184 because `d6c1970` added 6 tests) | ✓ PASS |
| `cargo clippy --workspace --all-targets --release` is clean | as run | 0 warnings | ✓ PASS |
| `cargo fmt --check` is clean | as run | exit 0, no output | ✓ PASS |
| `chrys-core` classify suite, incl. both alpha-antialiasing tests | `cargo test -p chrys-core --release --test classify` | 28/28 pass | ✓ PASS |
| `chrys-cli` refusal suite | `cargo test -p chrys-cli --release --test refusal` | 2/2 pass | ✓ PASS |
| `chrys-source-raster` format suite | `cargo test -p chrys-source-raster --release --test formats` | 5/5 pass | ✓ PASS |
| `chrys-cli` sequence-report suite, incl. the single-pair guard | `cargo test -p chrys-cli --release --test sequence_report` | 6/6 pass | ✓ PASS |
| Live CI on the exact current commit is green | `gh run view 34129949869 --json headSha,jobs` | `headSha` = current `HEAD` (`3a1d211`); 8/8 jobs `success`; no run in progress | ✓ PASS |
| No debt markers in the files this re-verification's risk targets | `grep -n -E "TBD\|FIXME\|XXX\|TODO\|HACK\|PLACEHOLDER"` on `main.rs`, `sequence_report.rs` | 0 matches | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| CORE-01 | 01-01, 01-07, 01-09 | Names each change by kind, never a bare pixel count | ✓ SATISFIED | Unaffected by `d6c1970`; live run confirms, incl. the alpha regression |
| CORE-02 | 01-04, 01-06 | Registers a translated region and reports the offset | ✓ SATISFIED | Unaffected; verified via unchanged code + UAT test 5 |
| CORE-03 | 01-07 | Reports a colour change as a value + two colours | ✓ SATISFIED | Unaffected; live run confirms |
| CORE-04 | 01-07, 01-09 | Groups changed pixels into labelled regions with bounding boxes | ✓ SATISFIED | 28/28 `classify.rs` tests pass live |
| CORE-05 | 01-07, 01-09 | Does not report an antialiasing difference a person cannot see | ✓ SATISFIED | Both alpha-antialiasing tests pass; `antialias.rs` untouched since prior verification |
| CORE-06 | 01-05 | Refuses a pair it cannot register, states why | ✓ SATISFIED | Live run confirms, unaffected by `d6c1970` |
| CORE-07 | 01-05 | Compares only near-identical pairs, says so when too different | ✓ SATISFIED | Live run on `pair-01` refusal case confirms; `refusal.rs` 2/2 pass |
| DET-01 | 01-03, 01-08 | Identical verdict on Linux, macOS, Windows | ✓ SATISFIED | 6-runner CI `agree` job green on current `HEAD` |
| DET-02 | 01-03, 01-08 | Identical verdict on x86-64 and aarch64 | ✓ SATISFIED | Same CI evidence, on current `HEAD` |
| DET-03 | 01-03, 01-08, 01-09 | CI proves DET-01/DET-02 on every commit, hashing raw RGBA8 | ✓ SATISFIED | Live `gh run view` on current `HEAD` (`3a1d211`), 8/8 jobs green, confirmed not an orphaned pre-rewrite commit |
| DET-04 | 01-01, 01-08 | No pixel entering comparison is GPU-produced | ✓ SATISFIED | No dependency change from `d6c1970` (edits `main.rs` and adds a test file only); guard unaffected |
| DET-06 | 01-04, 01-08, 01-09 | No platform transcendental on the comparison path; FP contraction disabled | ✓ SATISFIED | `d6c1970` touches only integer frame-index bookkeeping and string formatting in the CLI; no arithmetic added on the comparison path |
| SRC-01 | 01-01, 01-02 | Raster pair compared (PNG, JPEG, WebP, TIFF) | ✓ SATISFIED | `formats.rs` 5/5 tests pass live |

No orphaned requirements: cross-referenced against `.planning/REQUIREMENTS.md`'s Phase 1 traceability table (13 rows, all marked Complete) — the 13 IDs match exactly.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers in `crates/chrys-cli/src/main.rs` or `crates/chrys-cli/tests/sequence_report.rs`, the two files newly relevant to this re-verification. No stub patterns found in the new `all_frames`/tail-count code (both branches produce output derived from real `verdicts`, not a hardcoded value).

**The code review's five WARNINGS remain unfixed, re-judged against `d6c1970`:**

| ID | Issue | Still applies after `d6c1970`? | Blocks the phase goal? |
|---|---|---|---|
| WR-01 | Transcendental/`mul_add` guard misses qualified-call syntax | Unaffected — `d6c1970` adds no arithmetic on the comparison path | No — acceptable debt, unchanged reasoning from the prior verification |
| WR-02 | Block-comment line-counting shifts a guard's diagnostic line numbers | Unaffected — `determinism.rs` untouched | No — cosmetic, unchanged reasoning |
| WR-03 | `RasterError::TooLarge` can misreport the exceeded axis | Unaffected — `decode.rs`/`lib.rs` untouched | No — unused configuration path, unchanged reasoning |
| WR-04 | `difference_image`/`window_sum` rely on `debug_assert!` only | Unaffected — `warp.rs`/`integral.rs` untouched | No — unchanged reasoning; still a real gap for a hypothetical external caller, not a live defect |
| WR-05 | `frame_background` rescans per region | Unaffected — `kind.rs` untouched, explicitly out of scope | No — unchanged reasoning |

**Judgment:** `d6c1970` touches none of the five warning sites and introduces no new instance of any of the five categories. The prior judgment (none of the five blocks the phase goal) stands unchanged.

## Deferred Items

None.

## Human Verification Required

None. Every truth above was resolved by direct code inspection, live test runs, a live CI check via `gh run view` against the exact current commit SHA, and an independent binary-diff against a disposable pre-`d6c1970` build (not sourced from SUMMARY.md's or any hand-off message's claims alone).

## Gaps Summary

None. Re-verification confirms:

- The one real risk this re-verification was opened to check — that `d6c1970`'s multi-frame sequence reporting change might have altered single-pair output — does not hold. Single-pair stdout is byte-for-byte identical before and after `d6c1970`, on both `pair-01` and the alpha regression fixture `alpha-01`, and `--hash-only` output is likewise unchanged. This was independently measured by building the pre-`d6c1970` binary in a disposable worktree and diffing raw bytes, not inferred from reading the diff alone.
- UAT test 10 (exit codes 0/1/2/3) was re-run live, not read from `01-UAT.md`: all four codes still fire on the expected inputs.
- All five phase-1 success criteria still hold, including the alpha-only case the prior gap closure (plan 01-09) added.
- CI is green on the exact current commit (`3a1d211`, matching `origin/main`), not an orphaned commit from before the second history rewrite (which removed a telemetry file, per the task brief) — verified by comparing `gh run list`'s newest run's `headSha` against `git rev-parse HEAD` directly, and confirming no run is currently in progress.
- The five pre-existing code-review WARNINGS remain unfixed but, re-judged specifically against `d6c1970`'s diff, none of the five sites was touched and none of the five categories applies to the new code; the phase-goal judgment is unchanged from the prior verification.
- The trivial documentation commit `a6eba0b` (rewriting stale commit ids in six SUMMARY files after a force-push) changes no behaviour and introduces no gap; one rewritten id was spot-checked and resolves to a real, reachable commit.

The phase goal — a person compares two raster images and gets a structural verdict that is byte-identical across OS and architecture — holds on the current `HEAD`, unaffected by the phase-2 work that landed on a file phase 1 owns.

---

_Verified: 2026-09-07T16:10:00Z_
_Verifier: Claude (gsd-verifier)_
