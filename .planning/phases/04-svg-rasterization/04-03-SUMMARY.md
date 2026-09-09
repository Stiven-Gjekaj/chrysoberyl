---
phase: 04-svg-rasterization
plan: 03
subsystem: ci
tags: [github-actions, determinism, svg, tiny-skia, resvg, cross-platform]

# Dependency graph
requires:
  - phase: 04-svg-rasterization
    provides: "04-01's SVG adapter and committed fixture pair, and 04-02's boundary guards, both anchored at crates/chrys-core's tree id c97a6fb778c4b1373e5c4dc563481cc18e4c98c0"
provides:
  - "The six-runner determinism matrix hashes tests/golden/formats/svg on every push, appended to the digest job with no edit to the agree job"
  - "A written falsification protocol, in the workflow file itself, for what to do the first time and the second time the SVG line disagrees"
  - "A measured, cross-platform, six-runner result for the claim that resvg/tiny-skia render an SVG document bit-identically across three operating systems and two architectures: GREEN"
affects: []

# Actuals (#2632)
actuals:
  tokens: 631
  tasks: 2
  commits: 1
  plan_head_before: ef19a81bcac700726b46560eb536601c478a614c

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A phase's exit gate that depends on a remote matrix is a designed human checkpoint, not a task an executor completes alone: the plan's own action text says not to push, because this repository's process rule is that nothing reaches origin unless the author asks. The executor prepares and proves everything short of the push, then stops and waits."
    - "A reported CI result is re-verified independently before being written into a SUMMARY, with the same tool the report used (gh run view --json, gh run view --job --log), rather than copied from a relayed message. The agree job's own printed digest lines were compared byte-for-byte against the four lines this plan's own local hash-only run had already produced, and they matched exactly."
  removed: []

key-files:
  created: []
  modified:
    - .github/workflows/determinism.yml

key-decisions:
  - "The matrix came back green on the first push, so the plan's branch one (accept) applies. No file was changed for Task 2: Cargo.toml, named in this plan's own frontmatter as the one file the fallback branch might touch, was not touched, because the fallback is conditional on a red matrix and the matrix is green."
  - "The push itself was not performed by this executor. AGENTS.md and the operator's own global conventions both say a remote is not touched unless the author asks. The coordinator relayed that the human had authorized and completed the push; this executor independently re-verified the reported run (gh run view --json against run id 34347989464, and gh run view --job --log against the agree job's own job id) before recording it here, rather than transcribing the report unverified."

requirements-completed: [SRC-04, DET-05]

coverage:
  - id: D1
    description: "The digest job's 'Run the digest report on the committed fixture pairs' step gains one more echo/cargo-run block, appended after the five existing blocks, in the same shape they use, over tests/golden/formats/svg/base.svg and candidate.svg with --hash-only."
    requirement: DET-05
    verification:
      - kind: other
        ref: "grep -q 'tests/golden/formats/svg/base.svg' .github/workflows/determinism.yml, recorded below"
        status: pass
      - kind: integration
        ref: "cargo run -q --release -p chrys-cli -- compare tests/golden/formats/svg/base.svg tests/golden/formats/svg/candidate.svg --hash-only, exit 0, four lines, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D2
    description: "The agree job's own section is byte-identical to commit 199477d, proving phase 2's whole-file cmp -s rewrite needed no edit for this fixture, and all six runner labels are unchanged."
    requirement: DET-05
    verification:
      - kind: other
        ref: "git show 199477d:.github/workflows/determinism.yml, sliced from ^  agree: and cmp -s against the same slice of HEAD: exit 0"
        status: pass
      - kind: other
        ref: "for-loop over the six runner labels against .github/workflows/determinism.yml: all six matched exactly"
        status: pass
    human_judgment: false
  - id: D3
    description: "A comment block above the new lines records what the line gates, why the claim is not assumed, the SIMD fallback to try first, and that a further disagreement is withdrawn in writing by a person, and states that one green machine is not evidence."
    requirement: DET-05
    verification: []
    human_judgment: true
    rationale: "Whether the comment reads clearly to a person who has never read this plan is a judgment about prose, not something a command can assert. The comment text itself is quoted in full below for that read."
  - id: D4
    description: "The matrix has run on all six labels and its result is recorded verbatim: GREEN. The agree job passed, and its own printed reference digest carries the tests/golden/formats/svg section, matching this plan's local run byte-for-byte, which is the falsifiable part of the claim actually being exercised rather than a vacuous pass."
    requirement: DET-05
    verification:
      - kind: other
        ref: "gh run view 34347989464 --json conclusion,jobs: 8/8 jobs success, including agree"
        status: pass
      - kind: other
        ref: "gh run view --job <agree-job-id> --log: printed tests/golden/formats/svg section matches this plan's own local cargo run output line for line"
        status: pass
    human_judgment: false
  - id: D5
    description: "The engine boundary drill is green over the whole phase's commit range (04-01's first commit through this plan's last commit)."
    requirement: (project-wide invariant, D-03)
    verification:
      - kind: other
        ref: "scripts/engine-boundary-drill.sh e70c85bbb5d76db131b1ae1c03d73dbd219499cd 89aa74f2436efe1938bde49ca85550e5430389a6, recorded verbatim below: 2 of 2 drills behaved as expected"
        status: pass
    human_judgment: false
  - id: D6
    description: "No file under crates/chrys-core/ changes anywhere in this plan's commit range."
    requirement: (project-wide invariant, D-03)
    verification:
      - kind: other
        ref: "git rev-parse HEAD:crates/chrys-core, checked before this plan's commit and after it: c97a6fb778c4b1373e5c4dc563481cc18e4c98c0, unmoved"
        status: pass
    human_judgment: false
  - id: D7
    description: "cargo test --workspace --all-features, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --check all pass; the per-crate determinism and svg_boundary_guard test suites stay green."
    requirement: (project-wide invariant)
    verification:
      - kind: other
        ref: "cargo test --workspace --all-features: 0 failed; cargo clippy: clean; cargo fmt --check: clean; cargo test -p chrys-core --test determinism: 6 passed; cargo test -p chrys-cli --test svg_boundary_guard: 2 passed"
        status: pass
    human_judgment: false

duration: not captured (PLAN_START_TIME was not recorded at launch; this plan paused at a checkpoint mid-execution and resumed in a later turn)
completed: 2026-09-09
status: complete
---

# Phase 4 Plan 03: Settle the cross-platform SVG determinism claim on the six-runner matrix Summary

**Criterion 3 is met: the six-runner GitHub Actions matrix hashed the text-bearing SVG fixture and agreed byte-for-byte across ubuntu-24.04, ubuntu-24.04-arm, macos-15, macos-15-intel, windows-2022 and windows-11-arm on the first push (run 34347989464), so tiny-skia's default-on SIMD did not make cross-platform rendering diverge, and the fallback branch was not needed.**

```
first commit: 89aa74f2436efe1938bde49ca85550e5430389a6
last commit: 89aa74f2436efe1938bde49ca85550e5430389a6
```

## Performance

- **Duration:** not captured (see frontmatter note; the plan halted at Task 2's own designed human checkpoint — the push authorization — and resumed in a later turn)
- **Completed:** 2026-09-09
- **Tasks:** 2 / 2
- **Files modified:** 1

## Accomplishments

### Task 1 — the two lines and the falsification protocol

- Appended one more `echo`/`cargo run` block to the `digest` job's fixture-pairs step, in the same shape as the five blocks above it, over `tests/golden/formats/svg/base.svg` and `candidate.svg` with `--hash-only`.
- Wrote a comment block immediately above the new lines: what the line gates (phase 4's exit gate for the font and curve determinism claim, and the template every later format gate reuses), why the claim is not assumed (tiny-skia's SIMD dispatch is resolved at compile time per architecture, not at run time per CPU, so it cannot reproduce phase 1's auto-dispatching-FFT-planner failure mode, but no run against a glyph had happened yet), the documented fallback (disable tiny-skia's SIMD feature and re-measure), and what happens if that still disagrees (write the finding down and withdraw the claim, a decision for a person).
- Ran the exact invocation locally before committing it: exit 0, four digest lines (recorded verbatim below).
- Did not touch the `agree` job or the six runner labels. Both verified byte-identical/unchanged.
- Committed as `89aa74f`.

### Task 2 — the gate, run and settled

- Ran the two checks that need no remote: `cargo test --workspace --all-features` (0 failed) and `scripts/engine-boundary-drill.sh` over this phase's whole commit range (04-01's first commit `e70c85b` through this plan's own last commit `89aa74f`), both drills behaving as expected.
- Confirmed the precondition (`git remote -v` names `origin`) before starting.
- **Did not push.** This repository's process rule and the plan's own action text both require the push to be a human step; this executor prepared everything short of it and returned a `checkpoint:human-action`.
- The coordinator reported that the human authorized and performed the push, and relayed the matrix result. This executor independently re-verified that report against the GitHub API (`gh run view 34347989464 --json ...` and `gh run view --job <agree-job-id> --log`) rather than transcribing it, and the SVG digest lines the `agree` job printed matched this plan's own local run byte-for-byte.
- **The matrix is GREEN.** All eight jobs (`guards`, six `digest` jobs, `agree`) completed `success`. Criterion 3 is met.

## Task Commits

1. **Task 1: Hash the SVG fixture on the six determinism runners** — `89aa74f` (feat)

**Plan metadata:** pending (this SUMMARY and STATE.md/ROADMAP.md updates are committed next).

_Task 2 made no code commit. The accept branch, which is the branch that ran, changes no file — `Cargo.toml` is only touched by the fallback branch, and the fallback is conditional on a red matrix, which this run was not._

## Files Created/Modified

- `.github/workflows/determinism.yml` — one more `digest`-job block hashing the SVG fixture, plus the falsification-protocol comment above it.

## Decisions Made

See `key-decisions` in frontmatter.

## The Exact Invocation, Run Locally Before It Was Committed

```
$ cargo build --release -p chrys-cli
    Finished `release` profile [optimized] target(s) in 11.20s

$ cargo run -q --release -p chrys-cli -- compare tests/golden/formats/svg/base.svg tests/golden/formats/svg/candidate.svg --hash-only
decode-base 736966a7b688035c994cffc72c8f176cdd3f5befbcfecec79bc0d62ed9ce334f
decode-candidate 2ecf4c9fb92d838e11647241741a18b42743421749aa67edb30a06a2f1282aea
residual 89903e9e5c5a2f170abf5d8193d812f6a0660fd015a6b5959e1e8df284556105
verdict a70068e7d5ecfd237312b84d78f9d7d9a9573b71696481c88f2607f18ad9edc3
$ echo $?
0
```

## The Engine Boundary Drill, Over The Phase's Whole Commit Range, Recorded Verbatim

Range: `e70c85bbb5d76db131b1ae1c03d73dbd219499cd` (04-01's first commit) through `89aa74f2436efe1938bde49ca85550e5430389a6` (this plan's own last commit).

```
$ sh scripts/engine-boundary-drill.sh e70c85bbb5d76db131b1ae1c03d73dbd219499cd 89aa74f2436efe1938bde49ca85550e5430389a6
engine-boundary-drill: drill one: checked range 89aa74f2436efe1938bde49ca85550e5430389a6..48f81a4b49488c12f616fe0acedfd0f259a07614
engine-boundary-drill: drill one: check output:
crates/chrys-core/src/lib.rs
drill ok: planted-defect drill: the check went red and named crates/chrys-core/src/lib.rs
engine-boundary-drill: drill two: checked range e70c85bbb5d76db131b1ae1c03d73dbd219499cd^..89aa74f2436efe1938bde49ca85550e5430389a6
engine-boundary-drill: drill two: check output:
(empty)
drill ok: clean-range drill: the check stayed silent over plan 02-03's own range

engine-boundary-drill: 2 of 2 drills behaved as expected
```

Exit 0. The drill's own second-drill message still names the range it was written for ("plan 02-03's own range") because the script's comments were authored once, in phase 2, for a different plan's range; it is invoked here, unmodified, over phase 4's range, and its assertions (a named planted file on drill one, silence on drill two) are what was checked, not its prose.

## The Matrix Run, Read By A Person And Independently Re-Verified

- **Run:** [github.com/Stiven-Gjekaj/chrysoberyl/actions/runs/34347989464](https://github.com/Stiven-Gjekaj/chrysoberyl/actions/runs/34347989464)
- **Run id:** 34347989464
- **Head SHA:** `89aa74f2436efe1938bde49ca85550e5430389a6`
- **Event:** push
- **Started:** 2026-09-09T11:54:32Z
- **Conclusion:** success

All eight jobs completed `success`, confirmed via `gh run view 34347989464 --repo Stiven-Gjekaj/chrysoberyl --json conclusion,jobs`:

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

The `agree` job's own log (`gh run view --job 102455227989 --log`), the "Compare the six digest reports" step, printed the reference digest in full because all six runners' `digest.txt` files were byte-identical (`cmp -s` never set `disagreement`). Its final section, the one this plan added:

```
== tests/golden/formats/svg ==
decode-base 736966a7b688035c994cffc72c8f176cdd3f5befbcfecec79bc0d62ed9ce334f
decode-candidate 2ecf4c9fb92d838e11647241741a18b42743421749aa67edb30a06a2f1282aea
residual 89903e9e5c5a2f170abf5d8193d812f6a0660fd015a6b5959e1e8df284556105
verdict a70068e7d5ecfd237312b84d78f9d7d9a9573b71696481c88f2607f18ad9edc3
```

These four lines are byte-for-byte identical to the local run recorded above, run on this developer's own machine before the workflow line was ever committed. That local agreement is not the evidence for the claim (04-VALIDATION.md and this plan's own text both say so); the evidence is that the `agree` job's own copy, built by six independent runners across three operating systems and two architectures, prints the same four lines. Per-runner job logs (spot-checked: `windows-11-arm` and `ubuntu-24.04`) both show the "Run the digest report on the committed fixture pairs" step actually executing the `compare tests/golden/formats/svg/base.svg tests/golden/formats/svg/candidate.svg --hash-only` command, so the `agree` pass reflects a real SVG comparison on every runner, not an artifact upload that happened to carry no new content.

## The Claim, Settled

Before this run, the cross-platform bit-identity of this project's SVG rasterization path (resvg + tiny-skia + fontdb, with tiny-skia's SIMD paths for SSE2, AVX2 and NEON enabled by default) had never been measured against a glyph on more than one machine. It is now measured on six: `ubuntu-24.04`, `ubuntu-24.04-arm`, `macos-15`, `macos-15-intel`, `windows-2022`, `windows-11-arm`. All six agree. The x86-64 and AArch64 SIMD paths did not round differently from each other on this fixture. **Criterion 3 is met.**

SRC-04 stood independently of this answer before this run, and still does: the adapter compared SVG pairs correctly on any one machine regardless of how the matrix came back, as 04-01 already proved. This run additionally settles DET-05's cross-platform half for SVG.

## Full Verification, This Plan's End State

```
cargo test --workspace --all-features: 0 failed.
cargo clippy --workspace --all-targets -- -D warnings: clean.
cargo fmt --check: clean.
cargo test -p chrys-core --test determinism: 6 passed, 0 failed (all six guards green, unmoved).
cargo test -p chrys-cli --test svg_boundary_guard: 2 passed, 0 failed, unmoved.
git rev-parse HEAD:crates/chrys-core: c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 (unmoved, checked
  before this plan's commit and after it).
git remote -v: origin present (precondition satisfied).
The determinism.yml agree job's own section: byte-identical to commit 199477d.
All six runner labels present, each matched exactly.
GitHub Actions run 34347989464: 8/8 jobs success, independently re-verified via gh run view.
```

## Deviations from Plan

None - plan executed exactly as written. The matrix came back green, so the accept branch ran, exactly as the plan's three named outcomes anticipated as one of the acceptable results, and the fallback branch's own conditional trigger (a red matrix) never occurred.

## Issues Encountered

**Task 2 paused mid-execution at its own designed checkpoint.** The task's action text is explicit that pushing to the remote is a human step, and this repository's `AGENTS.md` and the operator's own global conventions both forbid pushing without being asked. This executor completed every check that did not require a remote, then returned a `checkpoint:human-action` rather than push on its own or report a fabricated result. The human authorized and performed the push in a later turn; this executor resumed, independently re-verified the reported run against the GitHub API, and completed the plan. This is documented here as the normal checkpoint flow the plan itself designed, not as a deviation.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 4's exit gate (criterion 3, the cross-platform SVG determinism claim) is met and recorded against a real, independently-verified matrix run, not a local approximation.
- `crates/chrys-core`'s tree object id is unmoved (`c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`) across this plan's own commit and the whole phase's commit range.
- This phase's own commit range for the engine-boundary claim is `e70c85bbb5d76db131b1ae1c03d73dbd219499cd`..`89aa74f2436efe1938bde49ca85550e5430389a6`, drilled green above.
- The falsification protocol now written into `determinism.yml` (what to try first if a later runner disagrees, and that a further disagreement is a person's decision to withdraw the claim) is available as the template the next format-specific gate (PDF, phase 5) can reuse.
- No open question remains for SVG cross-platform determinism. `04-RESEARCH.md`'s Assumptions Log A1 moves from an open question to a measured, green result.

## Self-Check: PASSED

`.github/workflows/determinism.yml` verified modified via `git diff ef19a81..HEAD --stat` (1 file changed, 32 insertions). Commit `89aa74f` verified present in `git log --oneline`. `git rev-parse HEAD:crates/chrys-core` verified reading `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`. Working tree verified clean of this plan's own changes (`git status --short` shows only pre-existing, unrelated `.planning/state.json`/`.planning/milestone.lock` runtime bookkeeping, restored to their pre-session state after the engine-boundary drill's own disposable worktree needed a clean tree, and left untouched by this plan's own commit). GitHub Actions run 34347989464 verified via `gh run view --json` and `gh run view --job --log`, independent of the coordinator's relayed report.

---
*Phase: 04-svg-rasterization*
*Completed: 2026-09-09*
