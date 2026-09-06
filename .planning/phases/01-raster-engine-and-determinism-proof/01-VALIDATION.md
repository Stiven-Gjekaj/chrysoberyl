---
phase: "1"
slug: "raster-engine-and-determinism-proof"
status: validated
nyquist_compliant: true
wave_0_complete: false
created: "2026-09-06"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

This file was written after the plans, not before them. The plan-checker's
Dimension 8 check 8e should have refused the phase because this file was
absent, and it did not. The checks below were then run by hand against the
eight plans. Every one passes, so the plans were compliant and only the record
was missing. The order was wrong; the result is measured, not assumed.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test`, built into the toolchain |
| **Config file** | none yet. Plan 01-01 creates `Cargo.toml` and the four crate manifests. |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo test --workspace --all-features` |
| **Estimated runtime** | not measured. No timing figure in the research is verified, so none is stated here. |

The research proposed `insta` for snapshots and `criterion` for benchmarks. The
planner dropped both on purpose. A SHA-256 digest over raw RGBA8 replaces
`insta`, because DET-03 asks for byte-exact agreement between machines and a
snapshot review loop cannot give that. `criterion` is dropped because no phase 1
requirement covers speed.

---

## Sampling Rate

- **After every task commit:** `cargo test --workspace`
- **After every plan wave:** `cargo test --workspace --all-features`
- **Before `/gsd-verify-work`:** the full suite is green, and the six-runner
  determinism matrix is green.
- **Max feedback latency:** not measured yet. No command in the plans uses a
  watch mode or an end-to-end browser framework, so no known long pole exists.

---

## Per-Task Verification Map

90 automated commands over 17 tasks. One representative command per task
is shown. Every task carries at least four.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | SRC-01, CORE-01, DET-04 | T-01-01 | see plan | unit | `cargo build --workspace` | ❌ W0 | ⬜ pending |
| 01-02-01 | 02 | 2 | SRC-01 | T-01-02 | see plan | unit | `cargo test -p chrys-source-raster` | ❌ W0 | ⬜ pending |
| 01-02-02 | 02 | 2 | SRC-01 | T-01-02 | see plan | unit | `cargo test -p chrys-source-raster --test formats` | ❌ W0 | ⬜ pending |
| 01-03-01 | 03 | 3 | DET-01, DET-02, DET-03 | T-01-03 | see plan | unit | `cargo test -p chrys-core` | ❌ W0 | ⬜ pending |
| 01-03-02 | 03 | 3 | DET-01, DET-02, DET-03 | T-01-03 | see plan | unit | `test -f .github/workflows/determinism.yml` | ❌ W0 | ⬜ pending |
| 01-04-01 | 04 | 4 | CORE-02, DET-06 | T-01-04 | see plan | unit | `cargo test -p chrys-core --test register` | ❌ W0 | ⬜ pending |
| 01-04-02 | 04 | 4 | CORE-02, DET-06 | T-01-04 | see plan | unit | `cargo test -p chrys-core --test register` | ❌ W0 | ⬜ pending |
| 01-05-01 | 05 | 5 | CORE-06, CORE-07 | T-01-05 | see plan | unit | `cargo test -p chrys-core --test refusal` | ❌ W0 | ⬜ pending |
| 01-05-02 | 05 | 5 | CORE-06, CORE-07 | T-01-05 | see plan | unit | `cargo test -p chrys-core --test refusal` | ❌ W0 | ⬜ pending |
| 01-06-01 | 06 | 6 | CORE-02 | T-01-06 | see plan | unit | `cargo test -p chrys-core --test block_match` | ❌ W0 | ⬜ pending |
| 01-06-02 | 06 | 6 | CORE-02 | T-01-06 | see plan | unit | `cargo test -p chrys-core --test block_match` | ❌ W0 | ⬜ pending |
| 01-07-01 | 07 | 7 | CORE-01, CORE-03, CORE-04, CORE-05 | T-01-07 | see plan | unit | `cargo test -p chrys-core --test classify` | ❌ W0 | ⬜ pending |
| 01-07-02 | 07 | 7 | CORE-01, CORE-03, CORE-04, CORE-05 | T-01-07 | see plan | unit | `cargo test -p chrys-core --test classify` | ❌ W0 | ⬜ pending |
| 01-07-03 | 07 | 7 | CORE-01, CORE-03, CORE-04, CORE-05 | T-01-07 | see plan | unit | `cargo test -p chrys-core --test classify` | ❌ W0 | ⬜ pending |
| 01-08-01 | 08 | 8 | DET-01, DET-02, DET-03, DET-04, DET-06 | T-01-08 | see plan | unit | `cargo test -p chrys-core --test determinism` | ❌ W0 | ⬜ pending |
| 01-08-02 | 08 | 8 | DET-01, DET-02, DET-03, DET-04, DET-06 | T-01-08 | see plan | unit | `cargo test -p chrys-core --test determinism` | ❌ W0 | ⬜ pending |
| 01-08-03 | 08 | 8 | DET-01, DET-02, DET-03, DET-04, DET-06 | T-01-08 | see plan | unit | `sh scripts/determinism-drill.sh` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Plan 01-01 is the Wave 0 of this phase. It creates every path the later plans
verify against.

- [ ] `Cargo.toml` workspace root, and the four crate manifests
- [ ] `crates/chrys-core`, `crates/chrys-source`, `crates/chrys-source-raster`,
      `crates/chrys-cli`
- [ ] `tests/golden/pair-01/` with two real PNG files on disk
- [ ] `.github/workflows/` determinism matrix, written in plan 01-03

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The six-runner matrix is green | DET-01, DET-02, DET-03 | A GitHub Actions run needs a push, and no local command can prove agreement between six runners | Push the branch, open the Actions tab, and confirm all six jobs report the same digest |
| A guard goes red when its defect is planted | DET-01 to DET-06 | Plan 01-08 task 3 plants the defect, and a person reads the failure message to confirm it names what was planted | Run the drill script and read each failure message |

---

## Nyquist Checks, Run by Hand

| Check | Rule | Result |
|-------|------|--------|
| 8a Automated verify presence | Every task has an `<automated>` command or a Wave 0 link | PASS. 17 of 17 tasks carry one. Zero `MISSING` sentinels. |
| 8b Feedback latency | No watch-mode flag, no end-to-end browser framework | PASS. Zero matches for `--watch`, `--watchAll`, `nodemon`, playwright, cypress, selenium. |
| 8c Sampling continuity | No window of 3 consecutive tasks with fewer than 2 automated verifies | PASS. All 17 tasks carry an automated verify, so no window can fail. |
| 8e VALIDATION.md exists | The file is present | FAILED before this file. PASS now. |
| 8f Failing direction | Every runnable command states its failure signal | PASS. `check verify-failure-directions 1` reports 90 commands, 0 blockers, 0 warnings. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references. There are none.
- [x] No watch-mode flags
- [ ] Feedback latency measured. Not done. It needs one green run first.
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-09-06, on the measured checks above.
