---
phase: 03-baseline-rules-and-ci-gate
verified: 2026-09-07T23:02:51Z
status: passed
score: 6/6 must-haves verified
covered_files: [".planning/REQUIREMENTS.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-01-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-01-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-02-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-02-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-03-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-03-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-04-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-04-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-05-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-05-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-REVIEW.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-SECURITY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-VALIDATION.md", "crates/chrys-baseline/src/golden.rs", "crates/chrys-baseline/src/lib.rs", "crates/chrys-baseline/tests/store.rs", "crates/chrys-cli/src/main.rs", "crates/chrys-cli/src/report.rs", "crates/chrys-cli/tests/region_rule.rs", "crates/chrys-rule/src/evaluate.rs", "crates/chrys-rule/src/lib.rs", "crates/chrys-rule/src/mask.rs", "crates/chrys-source-raster/examples/make-fixtures.rs", "crates/chrys-source/src/lib.rs", "examples/rules/example.toml", "tests/golden/rule-01/mask-cropped-size.toml", "tests/golden/rule-01/masks/badge-cropped.png"]
covered_digest: "v1:sha256:38d87a54268c9fd26117dd2dbe1e2aff7adc12524058f857bbd6e04bd36c2570"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/6
  gaps_closed:
    - "The CLI compares two paths, prints the verdict, exits non-zero on a rule failure, and writes a report artifact that names each change by kind, region, and size. (CR-01, CR-01's false pass under `--region` plus a mask-scoped `--rule`)"
    - "A person accepts a new baseline explicitly through the CLI; the baseline store is durable and does not silently lose the last-known-good state. (CR-02, the destructive-delete-before-write path in `write_baseline`)"
  gaps_remaining: []
  regressions: []
deferred: []
---

# Phase 3: Baseline, rules and CI gate — Verification Report

**Phase Goal:** A person commits a baseline, scopes tolerance by rule, and gates CI on the verdict.
**Verified:** 2026-09-07T23:02:51Z
**Status:** passed
**Re-verification:** Yes — after gap closure (plan 03-05)

## Goal Achievement

This is a full re-verification, not a diff against the prior report. Every
truth below was re-tested against a freshly built release binary at
`3adf9e6` (the phase's current HEAD, one commit past 03-05's own last task
commit `3ca8ddc`, which only adds the plan's own SUMMARY.md), using my own
independently constructed fixtures where the prior report's own defect was
found, not the repository's committed ones alone.

### Observable Truths (Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A TOML rule file scopes tolerance by kind of change and by a named region or mask; the shipped example shows a scoped exclusion, never a bare global threshold. | ✓ VERIFIED | `Scope` (`crates/chrys-rule/src/lib.rs`) still has exactly two variants, `Region` and `Mask`, no `Option`, no unscoped variant possible. `examples/rules/example.toml` read directly: both `[[rule]]` tables name `region` or `mask`; neither is bare. Unaffected by plan 03-05. |
| 2 | An unknown key or a malformed rule fails loudly and names the line. | ✓ VERIFIED | Ran `cargo test -p chrys-rule an_unknown_key_fails_naming_the_line -- --exact` myself: 1 passed. `deny_unknown_fields` confirmed present on the rule row type by direct read. Unaffected by plan 03-05. |
| 3 | A baseline hash manifest is committed to the repository. Committed golden files are the default store, reached through a trait, so a second backend needs no change to the engine. | ✓ VERIFIED | `chrys-baselines/MANIFEST.toml` and `chrys-baselines/pair-01/base.png` are tracked by git (`git ls-files chrys-baselines`). `pub trait BaselineStore` (`crates/chrys-baseline/src/lib.rs:22`) still declares `resolve`/`accept` with an associated `Error` type; `GoldenFileStore` is the sole implementor. The write path behind this store changed shape (see truth 5) but the trait boundary and the committed-store default did not. |
| 4 | The CLI compares two paths, prints the verdict, exits non-zero on a rule failure, and writes a report artifact that names each change by kind, region, and size. | ✓ VERIFIED | **CR-01 re-tested myself with my own, independently generated adversarial fixture** (a fresh 70x60 white-opaque PNG I hand-encoded with a standalone Python script, at `/tmp/verify-cr01/my-mask.png`, distinct from the repository's own committed `masks/badge-cropped.png`, plus my own rule TOML pointing at it): both `chrys compare base candidate --rule r70.toml` and the same command with `--region badge` added now exit **3** and both name "70x60 ... does not match the 256x256 frame". The false green (exit 0 under `--region`) that the prior verification demonstrated is gone. I also independently confirmed the coordinate-space fix: `--region badge` (no rule) reports `x=40, y=40`, byte-identical to the whole-frame run's own rectangle, not the crop-local `x=10, y=10`. I re-ran all four of plan 03-05's own `region_rule.rs` tests myself (4 passed) and the full `cargo test --workspace` (all green, no `FAILED` line). See "CR-01 Re-Reproduction" below. |
| 5 | A person accepts a new baseline explicitly through the CLI. Nothing updates a baseline on its own, and every decode call sets an explicit memory limit. | ✓ VERIFIED | **CR-02 re-tested myself at the CLI level** (not merely by re-running the unit test): I accepted a first baseline, then obstructed the exact write step CR-02 named (pre-created a directory at `MANIFEST.toml.tmp`'s own path so the manifest write fails after every candidate file is already staged), and ran `chrys accept` again with a different candidate. The obstructed accept exited 3, and the previous baseline directory's own file and `MANIFEST.toml` were byte-for-byte unchanged (`sha1sum` before/after identical) with no leftover staging directory; `chrys compare --baseline` against the original candidate still resolved and reported `identical`. See "CR-02 Re-Reproduction" below. The explicit-only, no-auto-update property and the decode-limit guard are unaffected by this plan and remain intact (unchanged code paths, re-confirmed by reading). |
| 6 | The report names the file a frame came from, not only its index. | ✓ VERIFIED | `report::frame_report` still receives `source` from `base_names`, captured before the region-crop block runs; my own `--region badge --report` run wrote `source = "base.png"` alongside `index = 0`. Unaffected by plan 03-05's fix, and the report additionally now carries a correct `region = "badge"` value (see truth 4's evidence and CLI-02 below), which the prior report found wrong in this exact combination. |

**Score:** 6/6 truths verified.

### CR-01 Re-Reproduction (my own fixture, not the repository's committed one)

Built independently with a standalone Python PNG encoder (no PIL, no reuse
of `make-fixtures.rs`), at `/tmp/verify-cr01/my-mask.png`: 70x60, every
pixel `(255,255,255,255)` — sized to the `badge` hint's own cropped
dimensions, exactly the property that produced the false green:

```
$ ./target/release/chrys compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png --rule /tmp/verify-cr01/r70.toml
mask my-mask.png is 70x60, which does not match the 256x256 frame it is scoped against
exit: 3

$ ./target/release/chrys compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png --rule /tmp/verify-cr01/r70.toml --region badge
mask my-mask.png is 70x60, which does not match the 256x256 frame it is scoped against
exit: 3
```

Both exit 3. Both name 70x60 against 256x256. The `--region` run no longer
silently passes. This was built and run against a release binary I compiled
myself in this session (`cargo build --release`), not a pre-existing one.

Coordinate-space check, same fixtures, no rule:

```
$ ./target/release/chrys compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png --region badge
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 ...
exit: 1
$ ./target/release/chrys compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 ...
exit: 1
```

Identical rectangle reported with and without `--region`. The report
written with `--region badge --report` names `region = "badge"` on both
`[meta]` and the one `[[frame.change]]` table (I read the written file
directly).

### CR-02 Re-Reproduction (CLI-level, interrupted write)

```
$ ./target/release/chrys accept pair-01 tests/golden/rule-01/base.png --store /tmp/verify-cr02b/store
(exit 0)
$ mkdir -p /tmp/verify-cr02b/store/MANIFEST.toml.tmp
$ ./target/release/chrys accept pair-01 tests/golden/rule-01/candidate.png --store /tmp/verify-cr02b/store
/tmp/verify-cr02b/store/MANIFEST.toml.tmp: Is a directory (os error 21)
exit: 3
```

`sha1sum` of `pair-01/base.png` and `MANIFEST.toml` before and after the
obstructed accept are identical (`diff` of the two sha1sum captures
produced no output). No leftover `.pair-01.accept-tmp` staging directory
remained. `chrys compare --baseline pair-01 --store /tmp/verify-cr02b/store
tests/golden/rule-01/base.png` still resolved and printed `identical`,
exit 0. The previous, complete baseline survives a write interrupted
exactly where CR-02 named the destructive risk (after files are staged,
during the manifest write).

**Note on residual scope, not a gap:** two `fs::rename` calls
(`write_baseline`'s final swap of the staged directory and the temporary
manifest into place) are not one atomic act. A failure of the second
rename after the first succeeds is a narrower, lower-probability window
than the one CR-02 named (both old and new state remain on disk under
non-canonical names, recoverable by hand, not destroyed) and is explicitly
acknowledged in `write_baseline`'s own doc comment as the window
`verify_baseline_digests` fails closed against. This matches the phase's
own T-03-30/T-03-31 threat register entries and is not a new finding
against this phase's stated scope.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chrys-rule/src/lib.rs`, `mask.rs`, `evaluate.rs` | Rule schema, mask loading, evaluation | ✓ VERIFIED | Present, substantive, wired; `#![forbid(unsafe_code)]` confirmed present. `resolve_mask_path` now returns `canonical_mask` (read directly at `crates/chrys-rule/src/mask.rs:201`), closing WR-01. |
| `crates/chrys-baseline/src/lib.rs`, `golden.rs` | `BaselineStore` trait + `GoldenFileStore` impl | ✓ VERIFIED | Trait unchanged (`resolve`/`accept`); `write_baseline` rewritten as a staged write (stage in `.{name}.accept-tmp`, manifest in `MANIFEST.toml.tmp`, swap by rename), confirmed by direct read and by my own interrupted-write drill above. |
| `crates/chrys-cli/src/main.rs`, `report.rs` | CLI wiring: compare, accept, rule gate, report | ✓ VERIFIED | `run_compare` keeps `base_frames`/`candidate_frames` uncropped for the whole function (confirmed by direct read); `compared_base_frames`/`compared_candidate_frames` hold the cropped pixels; `translate_verdicts` adds the crop origin back onto every `Region.bbox` unconditionally. `report::Meta.region` present and populated. |
| `examples/rules/example.toml` | Shipped scoped example | ✓ VERIFIED | Unaffected; read directly, every rule scoped. |
| `chrys-baselines/MANIFEST.toml` | Committed baseline manifest | ✓ VERIFIED | Tracked by git; unaffected by the write-path rewrite. |
| `crates/chrys-cli/tests/region_rule.rs` | The test of `--region` combined with `--rule` and `--report` that did not exist before | ✓ VERIFIED | Exists, 4 tests, all pass on my own run: `a_rule_gives_the_same_exit_code_with_and_without_a_region`, `a_region_run_reports_the_rectangle_in_frame_coordinates`, `a_region_report_names_the_region_each_change_falls_in`, `the_mask_scoped_rule_refuses_the_untolerated_change_under_a_region`. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `chrys-cli::run_compare` | `chrys_rule::evaluate` | direct call, per-frame, on the uncropped base frame's hints and size | ✓ WIRED | Confirmed correct with and without `--region` by my own fixture above; no longer PARTIAL. |
| `chrys-cli::run_compare` | `chrys_baseline::GoldenFileStore::resolve`/`verify_baseline_digests` | direct call | ✓ WIRED | Unaffected by this plan; re-confirmed by reading. |
| `chrys-cli::run_accept` | `chrys_baseline::GoldenFileStore::accept` | sole call site | ✓ WIRED, ATOMIC ENOUGH | Sole call site unaffected; write itself is now staged and swapped by rename, confirmed by my own interrupted-write drill. |
| `chrys-cli::report::frame_report` | report `source`/`region`/`kind`/`size` fields | direct construction | ✓ WIRED | `region` now correct under `--region` (confirmed by my own run above); no longer PARTIAL. |

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|---|---|---|---|
| RULE-01 | 03-01 | ✓ SATISFIED | Tolerance scoped by `ChangeKind`; unaffected by this round. |
| RULE-02 | 03-01, 03-02, 03-05 | ✓ SATISFIED | `Scope::Region`/`Scope::Mask` both implemented and tested; `resolve_mask_path` now returns the verified canonical path (WR-01 closed), confirmed by direct read and by re-running `cargo test -p chrys-rule --test mask`. |
| RULE-03 | 03-01, 03-02 | ✓ SATISFIED | No computable field in the rule schema; unaffected. |
| RULE-04 | 03-01, 03-02 | ✓ SATISFIED | Unscoped rule refused via `validate_row`; `Scope`'s doc comment now correctly names `validate_row`/`RuleRow` (IN-01 closed). |
| RULE-05 | 03-01 | ✓ SATISFIED | `an_unknown_key_fails_naming_the_line` re-run by me, passes. |
| BASE-01 | 03-04 | ✓ SATISFIED | `MANIFEST.toml` committed, tracked. |
| BASE-02 | 03-04 | ✓ SATISFIED | Byte-for-byte copy behaviour unchanged by the staged-write rewrite (same copy rules, confirmed by reading). |
| BASE-03 | 03-04 | ✓ SATISFIED | Trait unchanged; `GoldenFileStore` sole impl. |
| BASE-04 | 03-04, 03-05 | ✓ SATISFIED | **CR-02 closed.** An accept whose manifest write fails leaves the previous baseline byte-for-byte intact and `resolve` still finds it, confirmed by my own CLI-level interrupted-write drill above, not merely by re-running the shipped unit test (which also passes: `a_failed_accept_leaves_the_previous_baseline_intact`, `a_stale_staging_directory_contributes_no_file_to_the_next_accept`, `only_the_accept_path_writes_to_the_store`, all re-run by me). |
| CLI-01 | 03-01, 03-05 | ✓ SATISFIED | **CR-01 closed.** "Exits non-zero when a rule fails" now holds with and without `--region`, confirmed against my own independently built adversarial mask fixture: exit 3 both ways, no false green. |
| CLI-02 | 03-03, 03-05 | ✓ SATISFIED | Report's `region` field now correct when `--region` is combined with `--report`, confirmed by my own run above. |
| CLI-03 | 03-01, 03-04, 03-05 | ✓ SATISFIED | `cargo test -p chrys-cli --test digest` re-run by me: passes; the flag-less path is byte-identical, confirmed by the translation being a no-op by value with origin `(0,0)`. |
| CLI-04 | 03-02, 03-03 | ✓ SATISFIED | Static call-site guard and `#![forbid(unsafe_code)]` in 8/8 crates; unaffected by this round. |

**No orphaned requirements**: all 13 IDs (RULE-01..05, BASE-01..04, CLI-01..04) are claimed by one of the five plans' `requirements:` frontmatter (03-05 additionally claims CLI-01, CLI-02, BASE-04, RULE-02 as the gap-closure plan), confirmed by grep.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | No `TBD`/`FIXME`/`XXX` marker found in any file this phase (including plan 03-05) touched | — | Scanned directly across all files in `covered_files`. |
| `crates/chrys-baseline/src/golden.rs` | 283-294 | Documented, acknowledged non-atomicity between the two final swap-renames | ℹ️ Info | Explicitly documented and mitigated by `verify_baseline_digests`'s fail-closed behavior (T-03-30/T-03-31 in the phase's own threat register); not a new finding, not blocking. |

The two prior 🛑 Blockers (CR-01's shadowing bug, CR-02's destructive first step) are both gone from the current source: confirmed by direct read of `run_compare` (no rebinding of `base_frames`/`candidate_frames` to a cropped value found anywhere in the function) and of `write_baseline` (no `fs::remove_dir_all` on the live baseline directory before the new state is staged).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| CR-01 is closed against my own, independently built adversarial mask | manual `chrys compare` drive, my own fixture, both without and with `--region` | Exit 3 both ways, both name 70x60 vs 256x256 | ✓ PASS |
| Coordinate space is one space, with and without `--region` | manual `chrys compare` drive | Identical `x=40, y=40` rectangle both ways | ✓ PASS |
| Report names the compared region under `--region` | manual `chrys compare --region badge --report` | `meta.region` and the change's own `region` both read `"badge"` | ✓ PASS |
| CR-02 is closed against a CLI-level interrupted write | manual `chrys accept` drive, obstruct `MANIFEST.toml.tmp`, retry | Previous baseline byte-for-byte unchanged, `resolve` still works | ✓ PASS |
| Plan 03-05's own new tests pass | `cargo test -p chrys-cli --test region_rule` | 4 passed, 0 failed | ✓ PASS |
| Plan 03-05's own new durability/guard tests pass | `cargo test -p chrys-baseline --test store` (three named tests) | 3 passed, 0 failed | ✓ PASS |
| Mask-path fix unbroken | `cargo test -p chrys-rule --test mask` and unknown-key test | passed | ✓ PASS |
| Full workspace suite still green | `cargo test --workspace` (one full run) | 0 failures across all crates, all `test result: ok` | ✓ PASS |
| Lints clean | `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` | both clean | ✓ PASS |
| Engine boundary unmoved, whole phase range, resolved hashes | `bash scripts/engine-boundary-drill.sh 29f5add $(git rev-parse 3adf9e6)` | 2 of 2 drills behaved as expected | ✓ PASS |

**On `scripts/engine-boundary-drill.sh`'s own `HEAD`-literal argument:** confirmed the script gives a false FAILED when the literal string `HEAD` is passed as `<last-commit>`, because the script re-resolves `$2` inside its own disposable worktree, whose `HEAD` has by then advanced past the planted-defect commit drill one made. This is the script's own argument-handling gotcha, already documented in 03-05-SUMMARY.md's "Issues Encountered", and is not recorded here as a defect in phase 3's own code. The drill was re-run with a resolved hash and passed 2 of 2.

### Probe Execution

No `scripts/*/tests/probe-*.sh` files exist in this repository and none are referenced by this phase's plans/summaries. Skipped: no probes to run.

### Human Verification Required

None. Both previously blocking findings (CR-01, CR-02) are conclusively demonstrated closed by direct, reproducible command-line evidence gathered independently in this session, against a release binary built in this session, using fixtures I constructed myself where the original defect was found (CR-01's mask) and a CLI-level (not merely unit-test-level) interruption for CR-02. All six roadmap success criteria and all 13 requirement IDs are satisfied. The prior report's own T-03-11 discrepancy (already resolved as "does not block the phase" in that same report, on the strength of its own direct reproduction) required no further action this round; it was not reopened by anything found in this verification.

## Gaps Summary

None. Both Critical findings from 03-REVIEW.md (CR-01, CR-02) are closed,
independently confirmed against fixtures and interruption points I built
myself rather than reusing only the executor's own committed test
artifacts. The two smaller findings from the same review (WR-01, IN-01)
are also closed. The full workspace test suite passes (0 failures),
clippy and fmt are clean, and the engine boundary (`crates/chrys-core`)
is confirmed unmoved across the whole of phase 3's commit range using a
resolved-hash invocation of the phase-gate drill script. Phase 3's goal —
a person commits a baseline, scopes tolerance by rule, and gates CI on the
verdict — is achieved.

---

_Verified: 2026-09-07T23:02:51Z_
_Verifier: Claude (gsd-verifier)_
