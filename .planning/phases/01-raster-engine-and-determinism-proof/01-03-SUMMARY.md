---
phase: 01-raster-engine-and-determinism-proof
plan: 03
subsystem: infra
tags: [rust, sha2, determinism, github-actions, ci]

# Dependency graph
requires:
  - phase: 01-raster-engine-and-determinism-proof
    provides: "chrys-core compare, chrys-cli compare BASE CANDIDATE (01-01)"
  - phase: 01-raster-engine-and-determinism-proof
    provides: "decode_guarded, normalize_to_rgba8 and the four-format fixture set (01-02)"
provides:
  - "chrys_core::hash: rgba8_digest, DigestSet, digest_report — the single place that decides what bytes the determinism digest covers"
  - "chrys_core::residual: residual_rgba8, the raw RGBA8 buffer the comparison digest covers"
  - "chrys compare BASE CANDIDATE --hash-only, printing the four-line digest report and exiting 0"
  - "tests/golden/pair-01/expected-decode.sha256, the committed decode digest the local guard checks against"
  - ".github/workflows/determinism.yml, live: a six-runner matrix and a cross-runner digest comparison job"
affects: [01-04, 01-05, 01-06, 01-07, 01-08]

# Actuals (#2632)
actuals:
  tokens: 4850
  tasks: 2
  commits: 2
plan_head_before: ce7bdd0024086f0ce24e14812233e97abfbbb64a

# Tech tracking
tech-stack:
  added: [sha2=0.11.0]
  patterns:
    - "hash.rs is the only place that decides what bytes a determinism digest covers; its own doc comment states the five-sentence contract and every committed digest in the repository depends on it not moving"
    - "residual_rgba8 is a documented stand-in: its doc comment names plan 01-06 as the plan that replaces its body while keeping its signature, so the digest contract built on top of it does not move"
    - "digest_report takes an already-computed Verdict rather than calling compare itself, so the digest always covers the exact verdict the caller reported, with no risk of a second, silently different compare() call"

key-files:
  created:
    - crates/chrys-core/src/hash.rs
    - crates/chrys-core/src/residual.rs
    - crates/chrys-cli/tests/digest.rs
    - tests/golden/pair-01/expected-decode.sha256
    - .github/workflows/determinism.yml
  modified:
    - crates/chrys-core/Cargo.toml
    - crates/chrys-core/src/lib.rs
    - crates/chrys-cli/src/main.rs
    - Cargo.lock

key-decisions:
  - "The changed-precondition block superseded the plan's own 'Honest scope note': the repository is now public with main pushed, so .github/workflows/determinism.yml was written as live and correct, not as a written-but-unrunnable placeholder. The trigger design (push and pull_request, zero pull_request_target) and every other threat mitigation in the plan's threat register are unchanged; only the publication assumption moved. This executor did not push anything — the orchestrator owns that."
  - "sha2 was added to chrys-core/Cargo.toml exactly as the plan's Task 1 action text directs, even though this session's project_constraints block paraphrased the crate's dependency invariant as 'chrys-source and thiserror only.' The plan's own success_criteria and its threat register (T-01-SC) both name sha2 as the one crate this plan adds to chrys-core, and the plan's acceptance criterion is 'chrys-core/Cargo.toml still names no format or graphics crate' — a criterion sha2 satisfies. The true invariant, confirmed against 01-01-SUMMARY's own tech-stack notes, is zero format-crate and zero graphics-crate dependencies, not zero dependencies beyond thiserror. Followed the plan; recording the discrepancy here rather than silently resolving it."
  - "Re-verified the six runner labels against docs.github.com's live runner reference page today (this session has network access), not only against 01-RESEARCH.md's 30-day-perishable list. All six labels (ubuntu-24.04, ubuntu-24.04-arm, macos-15, macos-15-intel, windows-2022, windows-11-arm) are unchanged and still standard, free, unlimited-minute runners on public repositories. No label had moved."
  - "actions/checkout, actions/upload-artifact and actions/download-artifact are pinned to a full 40-character commit hash fetched from each repository's own GitHub API tag list this session (v7.0.1, v7.0.1, v8.0.1 respectively), not copied from memory or from a third-party listing."
  - "actionlint is not installed on this machine and was not installed as part of this task, per the plan's own instruction. In its place, the workflow file was validated with Ruby's built-in YAML.load_file and a standalone local run of the agree job's comparison script against mock digest.txt fixtures, covering both a disagreement scenario (naming the right runner pair and the right digest) and an agreement scenario (printing the four digests once). The YAML check caught a real bug before commit: an unquoted 'on:' key parses as the boolean true under strict YAML 1.1 truthy rules, not the string key GitHub's workflow schema expects. Fixed by quoting it as \"on\":."
  - "digest_report signature takes the caller's already-computed Verdict rather than calling chrys_core::compare internally, exactly as the plan's action text specifies, so --hash-only's digest always covers the same verdict run_compare already computed and is not a second, independently-run comparison."

patterns-established:
  - "One change per commit, code and tests together, per AGENTS.md: the digest surface (chrys-core changes, the CLI flag, the committed digest file and its guard test) landed as one commit, and the CI workflow landed as a second, separate commit."

requirements-completed: [DET-01, DET-02, DET-03]

coverage:
  - id: D1
    description: "chrys compare BASE CANDIDATE --hash-only prints exactly four lines, each a label and 64 lowercase hexadecimal characters, and exits 0 even when the pair differs."
    requirement: "DET-03"
    verification:
      - kind: e2e
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png --hash-only | wc -l"
        status: pass
      - kind: e2e
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png --hash-only; test $? -eq 0"
        status: pass
      - kind: e2e
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png --hash-only | grep -cE '^(decode-base|decode-candidate|residual|verdict) [0-9a-f]{64}$'"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every digest covers raw RGBA8 bytes or canonical verdict text, never a re-encoded file, and a committed digest file records the decode result so a local run goes red when decode output moves."
    requirement: "DET-01"
    verification:
      - kind: unit
        ref: "crates/chrys-core/src/hash.rs#tests::rgba8_digest_on_an_empty_slice_returns_the_empty_input_digest"
        status: pass
      - kind: unit
        ref: "crates/chrys-core/src/hash.rs#tests::rgba8_digest_is_stable_across_repeated_calls"
        status: pass
      - kind: integration
        ref: "crates/chrys-cli/tests/digest.rs#the_two_decode_lines_match_the_committed_digest_file"
        status: pass
      - kind: integration
        ref: "crates/chrys-cli/tests/digest.rs#a_second_run_gives_byte_identical_stdout"
        status: pass
    human_judgment: false
  - id: D3
    description: "A workflow runs the same command on six runner labels covering Linux, macOS and Windows on x86-64 and aarch64, triggers on push and pull_request only, pins its actions by commit hash, and a final job compares the six reports and fails when any two differ, naming the runners that disagree."
    requirement: "DET-02"
    verification:
      - kind: other
        ref: "grep -v '^[[:space:]]*#' .github/workflows/determinism.yml | grep -cE '^[[:space:]]*- +(ubuntu-24\\.04|ubuntu-24\\.04-arm|macos-15|macos-15-intel|windows-2022|windows-11-arm)[[:space:]]*$' = 6"
        status: pass
      - kind: other
        ref: "grep -v '^[[:space:]]*#' .github/workflows/determinism.yml | grep -c 'pull_request_target' = 0"
        status: pass
      - kind: other
        ref: "grep -v '^[[:space:]]*#' .github/workflows/determinism.yml | grep -cE 'uses:.*@[0-9a-f]{40}' >= 2 (actual: 3)"
        status: pass
      - kind: other
        ref: "local dry run of the agree job's inline bash comparison script against mock digest.txt fixtures, one disagreeing and one agreeing"
        status: pass
    human_judgment: true
    human_judgment_note: "The six-runner matrix itself has not run in GitHub Actions as part of this executor's work. The repository is public and main is pushed (the changed-precondition this plan was told to honour), but this worktree's commits are not yet on main, and this executor does not push. The green run is recorded as a human check below."

duration: 16min
completed: 2026-09-06
status: complete
---

# Phase 1 Plan 3: The determinism guard — digest report and six-runner workflow Summary

**`chrys compare A B --hash-only` prints four SHA-256 digests over raw RGBA8 bytes and verdict text, a committed digest file makes decode drift fail locally, and `.github/workflows/determinism.yml` is a live six-runner matrix with a comparison job that names disagreeing runners.**

## Performance

- **Duration:** 16 min (measured between this plan's final commit and 01-02's final commit; this session's own first action was not separately timestamped)
- **Completed:** 2026-09-06T16:40:17Z
- **Tasks:** 2
- **Files modified:** 9 (5 created, 4 modified — see key-files)

## Accomplishments

- `crates/chrys-core/src/hash.rs`: `rgba8_digest(bytes) -> String` (SHA-256, 64 lowercase hex characters, stable and panic-free on an empty slice), `DigestSet` (four `String` fields with a `Display` printing them in a fixed order), and `digest_report(base, candidate, verdict)`, which builds the set from the caller's already-computed verdict rather than re-running `compare` itself. The module's head doc comment states the five-sentence digest-input contract the plan specifies verbatim.
- `crates/chrys-core/src/residual.rs`: `residual_rgba8(base, candidate)`, an RGBA8 buffer of the absolute per-channel difference between two frames with alpha forced to 255, documented as the stand-in plan 01-06 replaces while keeping this exact signature.
- `crates/chrys-core/Cargo.toml` gains one new dependency, `sha2` (workspace-pinned `=0.11.0`), and still names zero format-crate and zero graphics-crate dependencies.
- A `ShapeMismatch { base, candidate }` variant on `CompareError`, returned by `residual_rgba8` when the two frames differ in size.
- `crates/chrys-cli/src/main.rs`: a `--hash-only` boolean flag on `compare`. When set, it prints the digest report and always exits 0, even on a differing pair, because the flag reports digests, not a verdict.
- `tests/golden/pair-01/expected-decode.sha256`: the two committed decode digest lines, generated once from a live run against the committed pair and tracked by git.
- `crates/chrys-cli/tests/digest.rs`: three tests running the built `chrys` binary — the committed file matches a live run's two decode lines, the report is exactly four lines of 64-character lowercase hex, and two runs of the same command give byte-identical stdout.
- `.github/workflows/determinism.yml`: a `digest` job matrixed over six runner labels (`ubuntu-24.04`, `ubuntu-24.04-arm`, `macos-15`, `macos-15-intel`, `windows-2022`, `windows-11-arm`) with `fail-fast: false`, each building in release and uploading its `digest.txt`; an `agree` job that downloads every report and compares the four digest lines runner by runner, failing and naming the disagreeing pair and digest on mismatch, or printing the four agreed digests once on success. The workflow triggers on `push` and `pull_request` only, sets `permissions: contents: read`, and pins `actions/checkout`, `actions/upload-artifact` and `actions/download-artifact` to full 40-character commit hashes with the version in a trailing comment.

## Task Commits

1. **Task 1: Print a digest report over raw RGBA8 bytes** - `3005c56` (feat + test: `chrys-core/src/hash.rs`, `residual.rs`, `Cargo.toml`, `lib.rs`; `chrys-cli/src/main.rs`; `chrys-cli/tests/digest.rs`; `tests/golden/pair-01/expected-decode.sha256`; `Cargo.lock`)
2. **Task 2: Run the digest on six runner labels and compare the results** - `11f230b` (`.github/workflows/determinism.yml`)

**Plan metadata:** commit pending (this SUMMARY — orchestrator owns STATE.md/ROADMAP.md writes per the objective given to this executor)

## Files Created/Modified

- `crates/chrys-core/src/hash.rs` - `rgba8_digest`, `DigestSet`, `digest_report`, the digest-input contract doc comment
- `crates/chrys-core/src/residual.rs` - `residual_rgba8`
- `crates/chrys-core/Cargo.toml` - adds `sha2.workspace = true`
- `crates/chrys-core/src/lib.rs` - `pub mod hash;`, `pub mod residual;`, `CompareError::ShapeMismatch`
- `crates/chrys-cli/src/main.rs` - `--hash-only` flag on `compare`
- `crates/chrys-cli/tests/digest.rs` - the local determinism guard test
- `tests/golden/pair-01/expected-decode.sha256` - the committed decode digest
- `.github/workflows/determinism.yml` - the six-runner matrix and the `agree` comparison job
- `Cargo.lock` - picked up `sha2` and its dependency tree (`digest`, `block-buffer`, `crypto-common`, `cpufeatures`, etc.)

## Decisions Made

See `key-decisions` in the frontmatter. In short: followed the changed-precondition block (workflow is live, not a placeholder, trigger design unchanged); followed the plan's own Task 1 text and success criteria for adding `sha2` to `chrys-core` even though this session's `project_constraints` paraphrase said otherwise, because the plan's acceptance criterion and threat register both name this exact addition and the true invariant (no format or graphics crate) is unaffected; re-verified all six runner labels and all three action commit hashes against live sources today rather than trusting the 30-day-perishable research doc alone; caught and fixed an unquoted `on:` YAML truthy bug before commit, using Ruby's `YAML.load_file` since `actionlint` is not installed on this machine.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Unquoted `on:` key parses as the boolean `true`, not the string key GitHub's schema expects**
- **Found during:** Task 2, validating the workflow file with Ruby's `YAML.load_file` after `actionlint` was confirmed not installed
- **Issue:** YAML 1.1's truthy conversion reads a bare `on:` key as the boolean `true`. `YAML.load_file` returned a hash with the key `true` instead of the string `"on"`, which is the classic "the norway problem" YAML gotcha. GitHub's own workflow parser tolerates this in practice, but it is not something to leave uncaught when the intended verification tool (`actionlint`) was unavailable to catch it another way.
- **Fix:** Quoted the trigger key as `"on":` instead of `on:`.
- **Files modified:** `.github/workflows/determinism.yml`
- **Verification:** `YAML.load_file` now reports the key as the string `"on"` with `push`/`pull_request` as its two sub-keys; all five of the plan's grep-based verify gates still pass unchanged, since none of them inspect the trigger key's quoting.
- **Committed in:** `11f230b` (the file was fixed before its first commit, so no separate commit was needed)

---

**Total deviations:** 1 auto-fixed (Rule 1, a real correctness bug in a file that had no other verification tool available on this machine)
**Impact on plan:** No scope creep. The fix does not change the trigger design, the runner matrix, the pinned actions, or any threat mitigation named in the plan's threat register.

### Precondition change honoured

The plan's own "Honest scope note" said the six-runner matrix "cannot run during this phase" because the repository had no remote. This session's `<changed_precondition>` block states that condition has since been satisfied: the repository is public and `main` is pushed. Per that block's explicit instruction, `.github/workflows/determinism.yml` was written as live and correct rather than disabled or placeholder, with no change to the trigger design (`push`/`pull_request` only, zero `pull_request_target`) or to any other threat mitigation. This executor did not push; the orchestrator owns that per this plan's objective.

## Issues Encountered

None beyond the one auto-fixed YAML deviation above.

## User Setup Required

None - no external service configuration required. The six-runner matrix will run automatically the next time this branch's work reaches `push` or a pull request against the now-public repository; no manual trigger is needed.

## Next Phase Readiness

- `chrys_core::hash` and `chrys_core::residual` are the stable digest surface plan 01-04 onward builds on: `residual_rgba8`'s doc comment already names plan 01-06 as the plan that replaces its body while keeping the signature, so the digest contract does not move when that happens.
- `.github/workflows/determinism.yml` is live. Its first real run will happen the next time this work reaches `push` or a pull request against the public repository — the human check below records that this executor has not observed a green run yet, only a clean local validation.
- Plan 01-08's local drill (watching the guard go red on purpose) has a real, committed `expected-decode.sha256` and a real `crates/chrys-cli/tests/digest.rs` to intentionally break.
- No blockers.

## Human Check Required

The six-runner matrix has not been observed running in GitHub Actions by this executor. Per this plan's objective, this executor does not push. When this branch's work reaches `push` or a pull request against the now-public repository, open the Actions tab and confirm the `determinism` workflow is green, that all six matrix jobs ran, and that the `agree` job printed one set of four digests rather than a disagreement report.

## Self-Check: PASSED

All files below were verified present on disk and all commit hashes verified present in `git log --oneline` on this branch before this line was written:
- `crates/chrys-core/src/hash.rs`, `crates/chrys-core/src/residual.rs` — FOUND
- `crates/chrys-cli/tests/digest.rs` — FOUND
- `tests/golden/pair-01/expected-decode.sha256` — FOUND
- `.github/workflows/determinism.yml` — FOUND
- Commits `3005c56`, `11f230b` — FOUND in `git log --oneline`

---
*Phase: 01-raster-engine-and-determinism-proof*
*Completed: 2026-09-06*
