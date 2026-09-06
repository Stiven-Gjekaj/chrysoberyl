---
phase: "2"
slug: "source-trait-and-a-second-format"
status: validated
nyquist_compliant: true
wave_0_complete: false
created: "2026-09-07"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

This file is seeded from the `## Validation Architecture` section of
`02-RESEARCH.md`, before the planner runs. Phase 1 shipped without a validation
contract because the researcher was never asked for that section, and the
plan-checker's own gate for the missing file did not fire. Both are fixed here by
ordering: the section was requested, and this file exists before a plan does.

`nyquist_compliant` stays `false` until the plans exist and the five checks run
against them.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test`, built into the toolchain. No new framework. |
| **Config file** | none new. This phase adds crates under the existing workspace. |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo test --workspace --all-features` |
| **Estimated runtime** | not measured. Phase 1 left this box unticked for the same reason and it stays unticked until a suite has run. |

---

## Sampling Rate

- **After every task commit:** `cargo test --workspace`
- **After every plan wave:** `cargo test --workspace --all-features`
- **After the animation wave specifically:** `git diff --name-only <wave-start>^..<wave-end> -- crates/chrys-core/`, which must print nothing
- **Before `/gsd-verify-work`:** the extended six-runner matrix green, and a clean run of `scripts/engine-boundary-drill.sh`
- **Max feedback latency:** not measured

---

## Requirement to Test Map

Lifted from `02-RESEARCH.md`. Every entry is Wave 0 except the one that already
exists, because this phase creates two crates that do not exist yet.

| Req | Behaviour | Type | Automated command | Exists |
|-----|-----------|------|-------------------|--------|
| SRC-02 | A numbered frame sequence pair compares frame by frame | integration | `cargo test -p chrys-source-sequence` | W0 |
| SRC-02 | Natural sort puts `frame2.png` before `frame10.png` | unit | `cargo test -p chrys-source-sequence sequence::tests::sorts_by_natural_order -- --exact` | W0 |
| SRC-03 | A GIF pair decodes to a frame vector matching its frame count | integration | `cargo test -p chrys-source-animation gif -- --exact` | W0 |
| SRC-03 | An APNG pair decodes with disposal and blend already resolved | integration | `cargo test -p chrys-source-animation apng -- --exact` | W0 |
| SRC-03 | An animated WebP pair decodes, and the hand-assembled fixture round-trips | integration | `cargo test -p chrys-source-animation webp_anim -- --exact` | W0 |
| SRC-08 | `compare_sequence` refuses on a frame-count mismatch and on an empty side | unit | `cargo test -p chrys-core --lib sequence` | W0 |
| SRC-08 | No file under `crates/chrys-core/` changed across the animation wave | drill | `scripts/engine-boundary-drill.sh` | W0 |
| SRC-08 | The dependency guard stays green with no change | unit | `cargo test -p chrys-core --test determinism no_gpu_or_format_crate_enters_chrys_cores_dependency_graph` | exists |
| SRC-09 | `Frame::crop_to_region` returns exactly the hint's rectangle | unit | `cargo test -p chrys-source crop_to_region -- --exact` | W0 |
| SRC-09 | A comparison with a named region registers inside it | integration | `cargo test -p chrys-cli region_hint -- --exact` | W0 |
| all | Cross-OS and cross-architecture digest agreement on the new fixtures | CI | the six-runner matrix, extended with one `compare --hash-only` per new fixture pair | W0 |

---

## Wave 0 Requirements

- [ ] `crates/chrys-source-sequence/` and `crates/chrys-source-animation/`
- [ ] `crates/chrys-core/src/sequence.rs` and its test module
- [ ] `Frame::crop_to_region` in `crates/chrys-source/src/lib.rs`
- [ ] `<name>.hints.toml` parsing, and `Deserialize` on `RegionHint`
- [ ] `tests/golden/sequence-01/` and `tests/golden/formats/{gif,apng,webp-anim}/`, with their generators
- [ ] `scripts/engine-boundary-drill.sh`
- [ ] The digest job in `.github/workflows/determinism.yml` extended for the new fixtures

---

## Manual-Only Verifications

| Behaviour | Requirement | Why manual | Instructions |
|-----------|-------------|------------|--------------|
| The six-runner matrix is green on the new fixtures | SRC-02, SRC-03 | A GitHub Actions run needs a push | Push, then confirm all six digest jobs and `agree` are green |
| The engine-boundary guard goes red when a defect is planted | SRC-08 | A person reads the failure message to confirm it names the file that changed | Run the drill and read its output |

---

## The Two Risks This Phase Carries

Recorded here because a validation contract that lists only what passes is not a
contract.

**The animated WebP fixture is new, unreviewed code.** No crate encodes an
animated WebP, so the fixture generator assembles a RIFF container by hand. The
byte layout came from a specification and has not been round-tripped. The fixture
must not be trusted before its own round-trip test passes.

**Lossy animated WebP determinism is unproven.** Phase 1's committed WebP fixture
is lossless. This phase uses lossless fixtures only, and the lossy case stays an
open question rather than an assumed pass.

---

## Validation Sign-Off

- [x] All tasks have an automated verify or a Wave 0 dependency. 13 of 13, 64 commands.
- [x] Sampling continuity: no 3 consecutive tasks without an automated verify. Cannot fail; all 13 carry one.
- [x] Wave 0 covers every missing reference. Zero `MISSING` sentinels.
- [x] No watch-mode flags. Zero matches for watch, nodemon, playwright, cypress, selenium.
- [ ] Feedback latency measured. Not done, and it needs one green run first.
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-09-07, on the measured checks above.

## A command in this contract was wrong

The requirement-to-test map first carried
`cargo test -p chrys-core --lib sequence::tests -- --exact`, lifted from
`02-RESEARCH.md` without being run. `--exact` matches a whole test name, not a
module prefix, so that command reports `0 passed; 54 filtered out` and exits
zero. It is a green result from running nothing, which is the failure `AGENTS.md`
names under what a test can hold on to.

The planner found it and used `cargo test -p chrys-core --lib sequence` instead.
The map now carries the working command. The lesson is that a command written
into a contract is not evidence until someone has watched it run.
