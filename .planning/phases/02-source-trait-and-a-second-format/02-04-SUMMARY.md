---
phase: 02-source-trait-and-a-second-format
plan: 04
subsystem: testing
tags: [ci, determinism, drill, github-actions, bash]

# Dependency graph
requires:
  - phase: 02-source-trait-and-a-second-format
    provides: "chrys-source-animation, the animation Source adapter, and its commit range (31ec06e..9547de2) which changed no file under crates/chrys-core/ (02-03)"
provides:
  - "scripts/engine-boundary-drill.sh, a re-runnable drill that plants a cross-boundary defect and proves the engine-boundary check can go red and name the file"
  - "the recorded live measurement of the engine-boundary check over plan 02-03's real commit range"
  - "the determinism digest job extended to hash all five committed fixture pairs (pair-01, sequence-01, gif, apng, webp-anim) on all six runners"
  - "the agree job's comparison rewritten to a whole-file byte comparison, so a fixture added later needs no further edit to the comparison logic"
affects: []

# Actuals (#2632)
actuals:
  tokens: 2809
  tasks: 2
  commits: 3
  plan_head_before: e78e224a6c4e175d4a22eb93d616fb1e7750f8c1

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A wave-boundary claim gets its own drill script, separate from the determinism-guard drill, because it checks a different kind of property (commit-history hygiene, not comparison-path behaviour) and folding the two together would give one script two reasons to change."
    - "The agree job's comparison is rewritten as a whole-file byte comparison (cmp -s) against the first runner's report, rather than a fixed set of named line numbers, so a fixture the digest job adds later is covered with no further edit to the comparison logic."

key-files:
  created:
    - scripts/engine-boundary-drill.sh
  modified:
    - .github/workflows/determinism.yml

key-decisions:
  - "The planted defect in drill one touches crates/chrys-core/src/lib.rs (an append-only comment line) together with a comment line in crates/chrys-source-animation/src/lib.rs, in one commit, so the planted commit looks like a real wave commit that happens to reach across the boundary, rather than an engine-only edit that would not test the check's discrimination between files."
  - "The digest job's new fixture sections are introduced by an '== path ==' header line written into digest.txt itself (not a separate CI log message), so a downloaded digest.txt artifact is self-describing without cross-referencing the workflow file."
  - "The agree job's rewritten comparison uses cmp -s for the pass/fail decision and diff only on a runner that disagrees, avoiding a full diff computation on the five runners that already match."

patterns-established:
  - "Any guard whose silence is read as evidence gets a drill that plants a real defect against it and confirms two things, not one: that it goes red, and that its message names the specific thing that was planted."

requirements-completed: [SRC-02, SRC-03, SRC-08]

coverage:
  - id: D1
    description: "scripts/engine-boundary-drill.sh plants a real cross-boundary defect inside a disposable git worktree, and the engine-boundary check goes red naming the exact planted engine file."
    requirement: SRC-08
    verification:
      - kind: other
        ref: "sh scripts/engine-boundary-drill.sh 31ec06eecd82bb0aff70b3bc9fe7b69aeee75de1 9547de222e33370ae72a10bd4653dfee81e1b7d0 (drill one: 'drill ok: planted-defect drill: the check went red and named crates/chrys-core/src/lib.rs')"
        status: pass
    human_judgment: false
  - id: D2
    description: "The same script, run over plan 02-03's real commit range, finds nothing under crates/chrys-core/, which is the pass condition SRC-08 rests on."
    requirement: SRC-08
    verification:
      - kind: other
        ref: "sh scripts/engine-boundary-drill.sh 31ec06eecd82bb0aff70b3bc9fe7b69aeee75de1 9547de222e33370ae72a10bd4653dfee81e1b7d0 (drill two: 'drill ok: clean-range drill: the check stayed silent over plan 02-03's own range')"
        status: pass
    human_judgment: false
  - id: D3
    description: "The determinism digest job hashes all five committed fixture pairs (pair-01, sequence-01, gif, apng, webp-anim) with compare --hash-only, one digest.txt per runner, with pair-01's four lines unprefixed and unchanged."
    requirement: SRC-02
    verification:
      - kind: other
        ref: "grep -v '^\\s*#' .github/workflows/determinism.yml | grep -c 'sequence-01' (2); grep -v '^\\s*#' .github/workflows/determinism.yml | grep -c 'webp-anim' (2)"
        status: pass
      - kind: other
        ref: "bash -c the five-pair local hash loop from 02-04-PLAN.md's Task 2 verify block (all five pairs hash with exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The agree job compares whole digest reports byte for byte rather than four named lines, proven able to go red and name the differing line on a locally corrupted copy before being trusted."
    requirement: SRC-08
    verification:
      - kind: other
        ref: "local run of the rewritten agree-job comparison script over two digest.txt copies differing in one character (see 'Local red run of the rewritten agree comparison' below): exits 1, names the differing line"
        status: pass
    human_judgment: false
  - id: D5
    description: "The workflow's trigger, permissions, action pins and fail-fast are unchanged from phase 1."
    requirement: SRC-02
    verification:
      - kind: other
        ref: "manual diff review of .github/workflows/determinism.yml: 'on: push/pull_request' unchanged, no pull_request_target, permissions: contents: read unchanged, both actions/checkout and actions/upload-artifact and actions/download-artifact still pinned to their existing 40-character hashes, fail-fast: false unchanged"
        status: pass
    human_judgment: false
  - id: D6
    description: "The six-runner matrix is green on this commit, confirmed by reading the Actions tab."
    requirement: SRC-02
    verification: []
    human_judgment: true
    rationale: "This needs a push to the remote, which this plan's executor is explicitly not permitted to do (the orchestrator pushes and watches the matrix per this plan's own environment note). No automated verification is possible from inside this worktree."

duration: unmeasured (PLAN_START_TIME not captured at launch)
completed: 2026-09-07
status: complete
---

# Phase 2 Plan 04: Prove the engine boundary and extend the determinism matrix Summary

**A new `scripts/engine-boundary-drill.sh` plants a real cross-boundary defect in a disposable worktree, watches the engine-boundary check name the planted file, then confirms the same check finds nothing over plan 02-03's real commit range; the determinism CI workflow now hashes all five committed fixture pairs on six runners, and its `agree` job's comparison was rewritten to a byte-for-byte whole-file check, proven able to fail on a locally corrupted digest before being trusted.**

## Performance

- **Duration:** unmeasured (PLAN_START_TIME not captured at launch)
- **Completed:** 2026-09-07
- **Tasks:** 2 / 2
- **Files modified:** 2 (1 new script, 1 modified workflow)

## Accomplishments

- Built `scripts/engine-boundary-drill.sh` as a sibling of `scripts/determinism-drill.sh`, following its exact shape: `set -u`, repo-root resolution from `$0`, a dirty-tree refusal that explains why, a `mktemp -d` worktree created with `git worktree add --detach --quiet`, a `cleanup` function installed with `trap cleanup EXIT INT TERM`, and pass/fail counters printing `drill ok:` or `DRILL FAILED:` per drill.
- Drill one plants a defect inside the worktree: an appended comment line in `crates/chrys-core/src/lib.rs` committed together with a comment line in `crates/chrys-source-animation/src/lib.rs`, so the planted commit looks like a real wave commit that happens to reach the engine. The check (`git diff --name-only <range> -- crates/chrys-core/`) went red and named `crates/chrys-core/src/lib.rs` exactly — not merely non-empty output, the specific planted path.
- Drill two runs the identical check over plan 02-03's real commit range (`31ec06eecd82bb0aff70b3bc9fe7b69aeee75de1^..9547de222e33370ae72a10bd4653dfee81e1b7d0`, read from `02-03-SUMMARY.md`'s recorded first/last commit lines) and confirms empty output — the pass condition SRC-08's claim rests on.
- The script takes the commit range as `$1` and `$2` with a usage message naming both when missing, holding no hash of its own, so it stays re-runnable against any wave's range.
- Verified `git status --porcelain` prints nothing and `git worktree list` shows no orphaned worktree after a full run, in both the pass and the no-args-usage case.
- Extended the `digest` job in `.github/workflows/determinism.yml`: `tests/golden/pair-01`'s four lines stay first, unprefixed and byte-identical to phase 1; four new sections follow, each introduced by an `== path ==` header line written into `digest.txt` itself, for `sequence-01`, `gif`, `apng` and `webp-anim`.
- Rewrote the `agree` job's comparison from four named-line-number field reads to a whole-file `cmp -s` against the first runner's `digest.txt`, printing a `diff` only for a runner that disagrees. A fixture the digest job adds later needs no edit to this comparison.
- Proved the rewritten comparison can fail before trusting it: built two copies of a real five-pair digest report, changed one character of one digest in one copy, ran the exact comparison script locally, and confirmed it exited 1 and named the differing line (recorded verbatim below).
- Changed nothing else about the workflow: triggers stay `push`/`pull_request` with no `pull_request_target`, `permissions: contents: read` unchanged, every `uses:` still pinned to its existing 40-character commit hash, `fail-fast: false` unchanged, no toolchain-install action added.
- `git rev-parse HEAD:crates/chrys-core` read `63aad81ddee9939047b1436a33eed8f0896da409` after every commit in this plan, matching the hash recorded by plan 02-02 and confirmed unchanged by plan 02-03.
- `cargo build --workspace`, `cargo test --workspace` (159 tests, 0 failed), `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` all pass with no changes needed.

## Task Commits

Each task was committed atomically:

1. **Task 1: Drill the engine boundary check with a planted defect** - `6ff077b16969f02e0024b696866dae70ba02b16d`
2. **Task 2: Extend the determinism matrix with the new fixture pairs** - `c4a001abf84bdc957a5f2e605b694a202dca8d71`

**Plan metadata:** pending (STATE.md and ROADMAP.md updates are the orchestrator's, per this plan's own execution instructions; this SUMMARY.md is committed directly by this plan)

## Boundary drill: full verbatim output

Command run: `sh scripts/engine-boundary-drill.sh 31ec06eecd82bb0aff70b3bc9fe7b69aeee75de1 9547de222e33370ae72a10bd4653dfee81e1b7d0`

```
engine-boundary-drill: drill one: checked range c4a001abf84bdc957a5f2e605b694a202dca8d71..b0413d0fe700556457503a87fc41c38829787119
engine-boundary-drill: drill one: check output:
crates/chrys-core/src/lib.rs
drill ok: planted-defect drill: the check went red and named crates/chrys-core/src/lib.rs
engine-boundary-drill: drill two: checked range 31ec06eecd82bb0aff70b3bc9fe7b69aeee75de1^..9547de222e33370ae72a10bd4653dfee81e1b7d0
engine-boundary-drill: drill two: check output:
(empty)
drill ok: clean-range drill: the check stayed silent over plan 02-03's own range

engine-boundary-drill: 2 of 2 drills behaved as expected
```

Exit code: `0`.

Note: the range printed for drill one (`c4a001ab..b0413d0f`) is the planted commit and its parent, both created and destroyed inside the disposable worktree on this particular run; a re-run creates a fresh pair of throwaway hashes. Drill two's range (`31ec06e^..9547de2`) is plan 02-03's real, permanent commit range and is the one this plan measures.

After this run: `git status --porcelain` printed nothing, and `git worktree list` showed only this agent's own worktree — no orphaned drill worktree remained.

## Local red run of the rewritten agree comparison

Before trusting the rewritten `agree` job comparison, it was run locally against two digest reports differing in exactly one character.

Setup: ran `compare --hash-only` for all five committed fixture pairs (`pair-01`, `sequence-01`, `gif`, `apng`, `webp-anim`) into `reports/digest-runner-a/digest.txt`, exactly as the `digest` job's step does; copied that file to `reports/digest-runner-b/digest.txt`; changed the first hex digit of `decode-base`'s digest on line 1 of the copy only (`5a5f59948...` to `6a5f59948...`).

Command run (the `agree` job's comparison step, extracted verbatim and pointed at the two-runner `reports/` directory above):

```
The six runners do not agree. A red agree job is a
portability bug in the comparison path, and a release
blocker, not a flaky test to retry.

== diff: runner-a vs runner-b ==
1c1
< decode-base 5a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc
---
> decode-base 6a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc

```

Exit code: `1`. The comparison went red and named the exact differing line, confirming the rewritten logic can fail before its silence over six real runners is trusted as evidence.

## Files Created/Modified

- `scripts/engine-boundary-drill.sh` - the re-runnable drill: plants a cross-boundary defect in a disposable worktree, confirms the boundary check names it, then confirms the same check is silent over a real clean range
- `.github/workflows/determinism.yml` - digest job now hashes five fixture pairs into one `digest.txt` per runner; agree job's comparison rewritten to a whole-file byte comparison

## Decisions Made

- The planted defect touches `crates/chrys-core/src/lib.rs` and `crates/chrys-source-animation/src/lib.rs` in one commit, mimicking a real wave commit that reaches across the boundary, rather than an engine-only edit that would not exercise the check's ability to discriminate one file from another in a mixed commit.
- New digest sections are introduced by an `== path ==` line written into `digest.txt` itself, making a downloaded artifact self-describing.
- The rewritten `agree` comparison uses `cmp -s` for the pass/fail decision and computes a `diff` only for a runner that actually disagrees, avoiding unnecessary full diffs on the runners that already match.

## Deviations from Plan

None - plan executed exactly as written. The verify command in Task 2 that invokes `bash -c '...for p in ...'` relies on bash's default word-splitting of an unquoted variable inside a `for` loop; this environment's interactive shell is zsh, where unquoted variables do not word-split by default, so the loop must be run through an explicit `bash -c` invocation (as the plan itself specifies) rather than run directly in the ambient shell. This is a shell-dialect fact, not a plan defect, and is recorded here only because it cost a debugging pass.

## Issues Encountered

None beyond the zsh/bash word-splitting note above, which is an environment fact rather than a defect in the plan or the workflow.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SRC-08's headline claim — the animation wave changed no file under `crates/chrys-core/` — is now measured by a drill that has been watched failing on a planted defect, not merely asserted from a single clean `git diff`.
- The determinism matrix now covers every fixture this phase committed. Deliverable D6 (the six-runner matrix green on this commit, with a downloaded `digest.txt` holding all five fixture sections) requires a push to the remote and a human reading the Actions tab — this plan's executor does not push, per its own environment constraint, so this is the one deliverable left for the orchestrator to confirm after pushing.
- `scripts/engine-boundary-drill.sh` is reusable groundwork: any later phase adding an input family with the same no-engine-change claim has a worked, re-runnable drill to point at its own commit range.
- The `agree` job's whole-file comparison means a future fixture pair needs only a new `compare --hash-only` line in the `digest` job; no further edit to the `agree` job is needed.

## Self-Check: PASSED

`scripts/engine-boundary-drill.sh` verified present on disk and executable; both task commit hashes (`6ff077b16969f02e0024b696866dae70ba02b16d`, `c4a001abf84bdc957a5f2e605b694a202dca8d71`) verified present in `git log --oneline`; `git rev-parse HEAD:crates/chrys-core` verified reading `63aad81ddee9939047b1436a33eed8f0896da409` after both task commits.

---
*Phase: 02-source-trait-and-a-second-format*
*Completed: 2026-09-07*
