---
phase: 03-baseline-rules-and-ci-gate
verified: 2026-09-07T00:00:00Z
status: gaps_found
score: 4/6 must-haves verified
covered_files: [".planning/REQUIREMENTS.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-01-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-01-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-02-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-02-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-03-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-03-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-04-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-04-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-REVIEW.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-SECURITY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-VALIDATION.md", "crates/chrys-baseline/src/golden.rs", "crates/chrys-baseline/src/lib.rs", "crates/chrys-cli/src/main.rs", "crates/chrys-cli/src/report.rs", "crates/chrys-rule/src/evaluate.rs", "crates/chrys-rule/src/lib.rs", "crates/chrys-rule/src/mask.rs", "crates/chrys-source/src/lib.rs", "examples/rules/example.toml"]
covered_digest: "v1:sha256:381d00b0733767647fa79297987cea91b306def750d856e3b01527b337383c4e"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "The CLI compares two paths, prints the verdict, exits non-zero on a rule failure, and writes a report artifact that names each change by kind, region, and size."
    status: failed
    reason: >
      CR-01 (unfixed) is not merely a broken flag combination that fails
      closed. Live-driven on commit fd4d78d: the same mask-scoped rule file,
      against the same real recoloured change (colour delta 115.65), gives
      exit 3 (loud refusal — mask does not match the frame) when run
      without --region, and exit 0 (silent pass, verdict text printed but
      the process reports success) when --region is added, because
      run_compare shadows base_frames/candidate_frames with the
      region-cropped versions before computing rule outcomes, so a
      mask-scoped rule's frame_size check runs against the cropped
      rectangle instead of the frame the mask was authored against. A CI
      job that gates on this exit code goes green on a real, untolerated
      change. This is the exact unsafe-direction failure mode
      judge_specifically Q4 asks to rule in or out; it is ruled in, with a
      command transcript. The same shadowing also makes the --report
      writer report `region: None` for every change whenever --region is
      combined with --report (hints are always empty on the cropped
      frame), so the report's "region" field is wrong in the same
      combination.
    artifacts:
      - path: "crates/chrys-cli/src/main.rs"
        issue: "run_compare (lines ~238-321) shadows base_frames/candidate_frames with crop_frames_to_region's output before computing rule outcomes and the report; a mask-scoped rule's frame_size and a region-scoped rule's hints are read from the cropped frame, not the frame the rule/mask was authored against."
    missing:
      - "Keep the original, uncropped frames alive under their own names for hint/size lookups; use a separate binding for the frames actually compared pixel-for-pixel (CR-01's own suggested fix)."
      - "At minimum, until the coordinate-space question is resolved, refuse the combination of --region with --rule or --report with a loud, non-zero error rather than silently producing a wrong verdict."
      - "A test exercising --region together with --rule and --report together (zero exist today, confirmed by grep across crates/chrys-cli/tests/)."
  - truth: "A person accepts a new baseline explicitly through the CLI; the baseline store is durable and does not silently lose the last-known-good state."
    status: failed
    reason: >
      CR-02 (unfixed, Critical in 03-REVIEW.md). GoldenFileStore::write_baseline
      deletes the previous baseline directory before the new one is fully
      written and before MANIFEST.toml is rewritten, with no staging and no
      atomic rename. An I/O failure partway through (a full disk, a
      permission error, a failed fs::copy on the Nth of M frames) destroys
      the last-known-good baseline and leaves neither the old, complete
      baseline nor a new, complete one. This does not falsify a CI verdict
      by itself (verify_baseline_digests, T-03-21, will loudly catch the
      resulting inconsistency on the *next* compare), but it directly
      undermines BASE-04's own stated rationale ("nothing in this tool can
      tell a correct accept from a mistaken one" presumes the prior correct
      accept survives a failed attempt), and it is a genuine, reproducible
      data-loss path in the one operation (`accept`) this phase's success
      criterion 5 names explicitly.
    artifacts:
      - path: "crates/chrys-baseline/src/golden.rs"
        issue: "write_baseline (lines 266-361): fs::remove_dir_all on the existing baseline_dir runs before fs::create_dir_all, the per-file fs::copy loop, and the MANIFEST.toml rewrite — none of which are staged or atomically renamed into place."
    missing:
      - "Stage the new baseline in a sibling temporary directory and the new MANIFEST.toml in a temporary file; only then atomically replace the old state with fs::rename for both, so a failure before the final rename leaves the previous, complete baseline untouched (CR-02's own suggested fix)."
deferred: []
---

# Phase 3: Baseline, rules and CI gate — Verification Report

**Phase Goal:** A person commits a baseline, scopes tolerance by rule, and gates CI on the verdict.
**Verified:** 2026-09-07
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A TOML rule file scopes tolerance by kind of change and by a named region or mask; the shipped example shows a scoped exclusion, never a bare global threshold. | ✓ VERIFIED | `Scope` (`crates/chrys-rule/src/lib.rs`) has exactly two variants, `Region` and `Mask`, no `Option`, no unscoped variant possible. `examples/rules/example.toml` read directly: every `[[rule]]` names `region` or `mask`; none is bare. `cargo tree -p chrys-core -e normal` (re-confirmed) shows zero `serde`/`toml`/`chrys-rule` edges. |
| 2 | An unknown key or a malformed rule fails loudly and names the line. | ✓ VERIFIED | Ran `cargo test -p chrys-rule an_unknown_key_fails_naming_the_line -- --exact` myself: 1 passed. Test asserts the error message contains both "line" and the offending field name "bogus". `deny_unknown_fields` confirmed on the rule row type by direct read. |
| 3 | A baseline hash manifest is committed to the repository. Committed golden files are the default store, reached through a trait, so a second backend needs no change to the engine. | ✓ VERIFIED | `chrys-baselines/MANIFEST.toml` exists and is not gitignored (`.gitignore` has no `baseline` entry). `pub trait BaselineStore` (`crates/chrys-baseline/src/lib.rs:22`) declares `resolve`/`accept` with an associated `Error` type; `GoldenFileStore` is the sole implementor referenced by `chrys-cli`. `chrys-baseline` depends on neither `chrys-core` nor `chrys-source` (re-confirmed by reading `Cargo.toml`). See gap on durability under truth 5/CR-02, which is a related but distinct defect from this criterion's literal text. |
| 4 | The CLI compares two paths, prints the verdict, exits non-zero on a rule failure, and writes a report artifact that names each change by kind, region, and size. | ✗ FAILED | **Live-reproduced myself.** See "CR-01 Live Reproduction" below: the same real, untolerated recoloured change gives exit 0 (silent pass) when `--region` is combined with a mask-scoped `--rule`, versus exit 3 (loud refusal) without `--region`. The exit code is not reliably non-zero on a rule failure — it can be wrongly zero. The report's `region` field is also wrong in the same combination (always `None`). |
| 5 | A person accepts a new baseline explicitly through the CLI. Nothing updates a baseline on its own, and every decode call sets an explicit memory limit. | ⚠️ Partially verified — see gap | The explicit-only, no-auto-update property is intact and drive-tested (orchestrator: `--baseline never-accepted` refused exit 3, creates nothing; `run_accept` is the sole call site of `BaselineStore::accept` in the workspace). Every decode call site is guarded (`decode_guarded`/`DecodeLimits`), confirmed by `#![forbid(unsafe_code)]` in 8/8 crates and the static call-site guard drilled red twice per 03-SECURITY.md. **However**, CR-02 (unfixed) means a failed `accept` can destroy the previously accepted, correct baseline with no staging or atomic rename — a durability defect directly against this criterion's spirit. See gap. |
| 6 | The report names the file a frame came from, not only its index. | ✓ VERIFIED | `report::frame_report` receives `source` from `base_names`, which is captured from `named_frames_for(base_path)` **before** the `--region` shadowing point, so this criterion is unaffected by CR-01. Orchestrator's own drive test: report carries `index = 4` AND `source = "frame5.png"`. Confirmed unaffected by reading `main.rs`'s ordering of `base_names` vs. the later `base_frames` rebinding. |

**Score:** 4/6 truths verified (2 failed: #4 outright, #5 partially, tracked together as 2 gaps in frontmatter)

### CR-01 Live Reproduction (judge_specifically Q1, Q4)

Reproduced directly against commit `fd4d78d` (HEAD), using `tests/golden/rule-01/base.png` and `candidate.png` (a real, substantial "recoloured" change: colour delta 115.65 at x=40,y=40,50x40) and a mask-scoped rule whose mask is fully white-opaque (tolerates everything) sized 70x60 — deliberately chosen to equal the `badge` hint's cropped dimensions:

```
$ chrys compare base.png candidate.png --rule rule70.toml
mask masks/mask70x60_white.png is 70x60, which does not match the 256x256 frame it is scoped against
exit: 3

$ chrys compare base.png candidate.png --rule rule70.toml --region badge
Recoloured region at x=10, y=10, width=50, height=40, colour delta 115.65 (base [200, 40, 40, 255], candidate [40, 200, 200, 255])
exit: 0
```

The identical rule file, against the identical real change, flips from a loud refusal to a silent pass purely because `--region` was added. This is a live-confirmed answer to judge_specifically Q4: **yes, the exit code can be wrong in the unsafe direction** — zero exit on a real, untolerated change — whenever `--region` is combined with a mask-scoped `--rule` and the mask's own dimensions happen to equal the region's cropped dimensions (the review calls this "a realistic authoring choice," and this reproduction shows it requires no unusual setup — a mask sized to match the region it is meant to scope is the natural way to author one). No test in the workspace exercises this combination (confirmed: zero matches for `region.*rule` patterns across `crates/chrys-cli/tests/`).

This goes further than 03-REVIEW.md's own characterization and further than the orchestrator's framing ("It fails closed here, which is the safe direction"): that framing holds only for `Scope::Region` rules (hints become empty under a crop, so those always fail closed, as the orchestrator's own repro with `tests/golden/rule-01/tolerate.toml` showed). It does not hold for `Scope::Mask` rules, which fail *open* under the reproduced conditions.

**Verdict: CR-01 blocks the phase goal.** The phase's entire purpose is "gates CI on the verdict," and this is a concretely demonstrated false-green path.

### T-03-11 Investigation (judge_specifically Q2)

Reproduced directly against commit `fd4d78d` (HEAD), matching the security file's own described scenario ("a 10x10 mask against a 256x256 frame, with no region flag") as closely as its wording allows, using a synthesized 10x10 RGBA PNG mask and a rule of `kind = "recoloured"` (matching the real change's own kind, which is a precondition for `rule_tolerates` ever reaching the mask-size check at all):

```
$ chrys compare base.png candidate.png --rule rule.toml
(stdout: empty)
(stderr): mask masks/mask10x10.png is 10x10, which does not match the 256x256 frame it is scoped against
exit: 3

$ chrys compare base.png candidate.png --rule rule.toml --report report.toml
(stderr): mask masks/mask10x10.png is 10x10, which does not match the 256x256 frame it is scoped against
exit: 3
(report.toml: not created)
```

Also reproduced with `--region badge` added (mask 10x10 against the cropped 70x60 frame): same result — both sizes named (10x10 and 70x60), exit 3, no report.

**This does not match the security file's "reopened" account** (which reports exit 1/no stderr without `--report`, and exit 2/no report with `--report`). In every variant I drove — with the mask sized against the full frame, against the cropped region, with and without `--report` — the control worked exactly as `evaluate.rs:120` and the original (pre-correction) security file described: both sizes are named, the message goes to stderr only (confirmed by redirecting stdout and stderr separately), exit code is 3, and no report is written on the error path (consistent with `run_compare`'s `?`-propagation happening before the report-writing block is reached).

**Measured fact:** I could not reproduce the failure the orchestrator recorded. The mask-size-mismatch control is present, reachable, and correct on the current HEAD, by direct, repeated, varied reproduction. The most plausible explanation for the discrepancy is that the orchestrator's earlier run used a binary built before `ca6cf94` (which added the `#![forbid(unsafe_code)]` guard and is the last commit touching `chrys-rule` before HEAD) or some other stale-artifact condition, since the source at HEAD does not admit the behavior described. I am not able to confirm that explanation directly (I cannot rerun their exact steps), so I record this as a discrepancy for the human to reconcile, not as a settled root cause.

**Verdict: T-03-11 does not block the phase**, on the strength of my own direct, repeated reproduction. `03-SECURITY.md`'s "reopened" correction should itself be corrected or re-verified by a human before being taken as ground truth, since it now disagrees with a live re-drive on the same commit.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chrys-rule/src/lib.rs`, `mask.rs`, `evaluate.rs` | Rule schema, mask loading, evaluation | ✓ VERIFIED | Present, substantive, wired; `#![forbid(unsafe_code)]` confirmed present. |
| `crates/chrys-baseline/src/lib.rs`, `golden.rs` | `BaselineStore` trait + `GoldenFileStore` impl | ✓ VERIFIED (with CR-02 caveat) | Trait has exactly `resolve`/`accept`; `GoldenFileStore` is sole caller; write path is not atomic (CR-02). |
| `crates/chrys-cli/src/main.rs`, `report.rs` | CLI wiring: compare, accept, rule gate, report | ⚠️ WIRED BUT INCORRECT UNDER `--region` | Present and wired for the non-`--region` path; demonstrably wrong when `--region` combines with `--rule`/`--report` (CR-01). |
| `examples/rules/example.toml` | Shipped scoped example | ✓ VERIFIED | Read directly; every rule scoped, none bare. |
| `chrys-baselines/MANIFEST.toml` | Committed baseline manifest | ✓ VERIFIED | Exists, not gitignored, matches the trait-based store. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `chrys-cli::run_compare` | `chrys_rule::evaluate` | direct call, per-frame | ⚠️ PARTIAL | Correct when `--region` absent; wrong `hints`/`frame_size` inputs when `--region` present (CR-01). |
| `chrys-cli::run_compare` | `chrys_baseline::GoldenFileStore::resolve`/`verify_baseline_digests` | direct call | ✓ WIRED | Digest re-verification confirmed by orchestrator's tamper test and by reading `verify_baseline_digests`. |
| `chrys-cli::run_accept` | `chrys_baseline::GoldenFileStore::accept` | sole call site | ✓ WIRED, ⚠️ NOT ATOMIC | Sole call site confirmed (T-03-22's guard); write itself is not crash-safe (CR-02). |
| `chrys-cli::report::frame_report` | report `source`/`region`/`kind`/`size` fields | direct construction | ⚠️ PARTIAL | `source` correct always; `region` wrong under `--region` (CR-01); `kind`/`size` correct. |

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|---|---|---|---|
| RULE-01 | 03-01 | ✓ SATISFIED | Tolerance scoped by `ChangeKind`; library-level, unaffected by CLI-layer CR-01. |
| RULE-02 | 03-01, 03-02 | ✓ SATISFIED | `Scope::Region`/`Scope::Mask` both implemented and tested at the `chrys-rule` level. |
| RULE-03 | 03-01, 03-02 | ✓ SATISFIED | No computable field in the rule schema (per 03-REVIEW.md, re-confirmed by reading `lib.rs`). |
| RULE-04 | 03-01, 03-02 | ✓ SATISFIED | Unscoped rule refused via `validate_row`; shipped example fully scoped. |
| RULE-05 | 03-01 | ✓ SATISFIED | `an_unknown_key_fails_naming_the_line` re-run by me, passes. |
| BASE-01 | 03-04 | ✓ SATISFIED | `MANIFEST.toml` committed, not gitignored. |
| BASE-02 | 03-04 | ✓ SATISFIED | Per 03-REVIEW.md's read of `tests/store.rs`'s byte-for-byte assertion; not independently re-run this session. |
| BASE-03 | 03-04 | ✓ SATISFIED | Trait has exactly `resolve`/`accept`; `GoldenFileStore` sole impl referenced by `chrys-cli`. |
| BASE-04 | 03-04 | ✓ SATISFIED, DURABILITY GAP ADJACENT | `resolve` on an unaccepted name errors (drive-tested by orchestrator); the *separate* durability defect is CR-02, tracked as its own gap rather than a BASE-04 failure, since BASE-04's own text is about `resolve`, not write-crash-safety. |
| CLI-01 | 03-01 | ✗ BLOCKED | "Exits non-zero when a rule fails" is falsified by the CR-01 live reproduction above: exit 0 on a real, untolerated change under `--region` + mask-scoped `--rule`. |
| CLI-02 | 03-03 | ⚠️ PARTIAL | Report correct without `--region`; `region` field wrong when `--region` is combined with `--report` (CR-01). |
| CLI-03 | 03-01, 03-04 | ✓ SATISFIED | Regression guard (`a_second_run_gives_byte_identical_stdout`) unaffected; `base_and_candidate` path count validation confirmed by reading. |
| CLI-04 | 03-02, 03-03 | ✓ SATISFIED | Static call-site guard, drilled red twice per 03-SECURITY.md; `#![forbid(unsafe_code)]` in 8/8 crates. |

**No orphaned requirements**: all 13 IDs (RULE-01..05, BASE-01..04, CLI-01..04) are claimed by one of the four plans' `requirements:` frontmatter, confirmed by grep.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/chrys-cli/src/main.rs` | ~238-321 | Variable shadowing hides a coordinate-space bug (CR-01) | 🛑 Blocker | Unsafe false-pass on CI gate, demonstrated live. |
| `crates/chrys-baseline/src/golden.rs` | 266-361 | Non-atomic multi-step write with a destructive first step | 🛑 Blocker | Data-loss path on `accept` failure (CR-02). |
| `crates/chrys-rule/src/mask.rs` | 162-193 | Check-then-use path resolution (WR-01) | ⚠️ Warning | Defense-in-depth gap only; no attacker-controlled concurrent process in this project's stated threat model. Not blocking. |
| `crates/chrys-rule/src/lib.rs` | 84-99, 405-420 | Doc comment overstates "type-level" refusal (IN-01) | ℹ️ Info | Documentation accuracy only. Not blocking. |

No unresolved `TBD`/`FIXME`/`XXX` markers found in the files this phase touched (checked via the same file list as `covered_files`).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Unknown-key rule fails naming the field and the line | `cargo test -p chrys-rule an_unknown_key_fails_naming_the_line -- --exact` | 1 passed | ✓ PASS |
| CR-01 exists and is live-exploitable | Manual `chrys compare` drive, both without and with `--region` | Exit 3 → Exit 0 on the identical real change | ✓ CONFIRMED (this is the failing behavior, not a passing check) |
| T-03-11 mask-size-mismatch control works | Manual `chrys compare` drive, with/without `--region`/`--report` | Both sizes named, stderr only, exit 3, no report, in every variant | ✓ PASS (contradicts 03-SECURITY.md's "reopened" note) |
| Full workspace suite still green | `cargo test --workspace` (one full run, per the single-full-run rule) | 0 failures across all crates | ✓ PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` files exist in this repository and none are referenced by this phase's plans/summaries. Skipped: no probes to run.

### Human Verification Required

None required to determine phase status — both blocking issues (CR-01, CR-02) are conclusively demonstrated by direct, reproducible command-line evidence, not matters of taste or visual judgment. One item is flagged for human reconciliation rather than technical re-verification:

1. **T-03-11's "reopened" account in 03-SECURITY.md vs. this report's direct reproduction**
   **Test:** Re-run the orchestrator's exact original steps (their mask file, their exact commands) if those artifacts still exist, to find why their run differed from mine.
   **Expected:** Either the discrepancy traces to a stale binary/build artifact on the orchestrator's side (in which case 03-SECURITY.md's "reopened" note should be corrected back to closed), or a real, narrower reproduction condition is found that I did not hit.
   **Why human:** I do not have access to the orchestrator's exact prior session state (build artifacts, exact mask file, exact working directory) to mechanically diff against my own reproduction; a human with access to both sessions' history can reconcile this faster than further guessing on my part.

## Gaps Summary

Two unfixed Critical findings from 03-REVIEW.md block this phase's goal, and both are now backed by live reproduction rather than by reading the review alone:

1. **CR-01** turns "gates CI on the verdict" into an unsafe gate under a realistic, untested flag combination (`--region` + a mask-scoped `--rule`): the same real, substantial change gives exit 3 without `--region` and exit 0 with it. This is a false-green path on the exact mechanism the phase exists to build.
2. **CR-02** is a genuine, reproducible-by-inspection (not independently re-triggered this session, since it requires injecting an I/O failure) data-loss path in `accept`, the phase's other headline operation: a failed accept can destroy the last-known-good baseline with no staging or atomic rename.

Both require code changes before this phase can be considered to have achieved its goal safely. T-03-11, by contrast, is **not** a gap: direct, repeated, varied reproduction on the current HEAD shows the mask-size-mismatch control working correctly, contradicting the "reopened" note in 03-SECURITY.md. That file's correction should itself be revisited by a human, but it does not block this phase.

---

_Verified: 2026-09-07_
_Verifier: Claude (gsd-verifier)_
