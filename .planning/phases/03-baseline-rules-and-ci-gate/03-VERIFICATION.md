---
phase: 03-baseline-rules-and-ci-gate
verified: 2026-09-08T01:30:00Z
status: passed
score: 6/6 must-haves verified
covered_files: [".planning/REQUIREMENTS.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-01-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-01-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-02-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-02-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-03-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-03-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-04-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-04-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-05-PLAN.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-05-SUMMARY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-REVIEW.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-SECURITY.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-UAT.md", ".planning/phases/03-baseline-rules-and-ci-gate/03-VALIDATION.md", "crates/chrys-baseline/src/golden.rs", "crates/chrys-baseline/src/lib.rs", "crates/chrys-baseline/tests/store.rs", "crates/chrys-cli/src/main.rs", "crates/chrys-cli/src/report.rs", "crates/chrys-cli/tests/region_rule.rs", "crates/chrys-core/src/classify/colour.rs", "crates/chrys-rule/src/evaluate.rs", "crates/chrys-rule/src/lib.rs", "crates/chrys-rule/src/mask.rs", "crates/chrys-source-raster/examples/make-fixtures.rs", "crates/chrys-source/src/lib.rs", "examples/rules/example.toml", "tests/golden/rule-01/mask-cropped-size.toml", "tests/golden/rule-01/masks/badge-cropped.png"]
covered_digest: "v1:sha256:2d1be32b25cd2c1e15c337f701c75286b37e181f8eb9f50d1a5bd039684fd53f"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 6/6
  gaps_closed: []
  gaps_remaining: []
  regressions: []
deferred: []
---

# Phase 3: Baseline, rules and CI gate — Re-Verification Report

**Phase Goal:** A person commits a baseline, scopes tolerance by rule, and gates CI on the verdict.
**Verified:** 2026-09-08T01:30:00Z
**Status:** passed
**Re-verification:** Yes — the prior `passed` report was flagged as stale and re-derived from scratch, not rubber-stamped.

## What changed since the prior verification

Exactly one non-planning file changed between the prior verification's HEAD
(`3adf9e6`) and this run's base (`bf3d749`): `examples/rules/example.toml`,
8 lines, comment-only (`git diff 3adf9e6 bf3d749 -- examples/rules/`
confirms this is the entire non-planning diff). The change corrects a
factual error the prior verification's own UAT test 10 missed: the shipped
example told a reader its threshold was a **CIEDE2000** delta E; the engine
measures a **CIE76** delta E (a Euclidean distance in Lab space). The two
formulas produce different numbers for the same colour pair, so a person
who copied a threshold from a CIEDE2000 tool would have set a limit that
did not mean what they thought.

**Could the comment fix have changed behaviour? No.** Verified three ways:

1. `git diff 3adf9e6 bf3d749 -- examples/rules/example.toml` shows only
   comment lines (`#`-prefixed) added/edited; every `[[rule]]` table,
   every key, and every value (`kind`, `region`, `mask`, `max_delta_e`,
   `max_alpha_delta`, `max_offset_px`) is byte-identical before and after.
2. `crates/chrys-rule/tests/rule_file.rs`'s
   `the_shipped_example_rule_file_parses_and_is_fully_scoped` test —
   which asserts only that the file parses and every rule carries a
   scope, deliberately blind to wording — re-run by me, passes (`cargo
   test -p chrys-rule the_shipped_example_rule_file_parses_and_is_fully_scoped
   -- --exact`).
3. No other file in the phase's `covered_files` set changed at all
   (confirmed by `git diff 3adf9e6 bf3d749 --stat`), so no code path the
   example exercises moved either.

**Is the corrected claim itself true?** Read `crates/chrys-core/src/classify/colour.rs`
directly: `colour_delta` imports `palette::color_difference::EuclideanDistance`
and calls `base_lab.distance(candidate_lab)`. I then read palette 0.7.7's
own `color_difference.rs` (the exact vendored source at
`~/.cargo/registry/.../palette-0.7.7/src/color_difference.rs`, matching
`Cargo.lock`'s pinned version): `EuclideanDistance` and `Ciede2000` are
two distinct, separately-implemented traits in that module, with the
module's own doc table describing `EuclideanDistance` as "Low" complexity
and `Ciede2000` as "High" complexity, "the de-facto standard". `colour.rs`
uses `EuclideanDistance`, never `Ciede2000`. The engine genuinely computes
a CIE76-style delta E, not CIEDE2000, exactly as the corrected comment
now states, and exactly as `colour.rs`'s own doc comment (line 19) already
said before this UAT round touched anything.

## Goal Achievement

Every truth below was re-tested against a release binary I built myself
in this session (`cargo build --release`, confirmed fresh by touching
`crates/chrys-cli/src/main.rs` and observing a recompile before the
build), at HEAD `bf3d749`, using fixtures I constructed independently
rather than trusting the repository's own committed ones or the prior
verification's narrative.

### Observable Truths (Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A TOML rule file scopes tolerance by kind of change and by a named region or mask; the shipped example shows a scoped exclusion, never a bare global threshold. | ✓ VERIFIED | Read `examples/rules/example.toml` directly: two `[[rule]]` tables, one `region = "badge"`, one `mask = "masks/scrollbar-track.png"`; neither is bare. `Scope` (`crates/chrys-rule/src/lib.rs`) still has exactly two variants, `Region` and `Mask`, no `Option`, no unscoped variant representable. |
| 2 | An unknown key or a malformed rule fails loudly and names the line. | ✓ VERIFIED | Ran `cargo test -p chrys-rule an_unknown_key_fails_naming_the_line -- --exact` myself: 1 passed. Also drove it live with my own malformed rule file (`nonsense_field = 5` at line 5): `chrys compare ... --rule /tmp/verify-p3/bad.toml` printed `TOML parse error at line 5, column 1 ... unknown field \`nonsense_field\`, expected one of \`kind\`, \`region\`, \`mask\`, ...` and exited 3. |
| 3 | A baseline hash manifest is committed. Committed golden files are the default store, reached through a trait, so a second backend needs no engine change. | ✓ VERIFIED | `chrys-baselines/MANIFEST.toml` and `chrys-baselines/pair-01/base.png` tracked by git (`git ls-files chrys-baselines`). `pub trait BaselineStore` (`crates/chrys-baseline/src/lib.rs:22`) declares `resolve`/`accept` with an associated `Error` type; `GoldenFileStore` is the sole implementor, confirmed by direct read. Unaffected by the comment-only change. |
| 4 | The CLI compares two paths, prints the verdict, exits non-zero on a rule failure, and writes a report artifact naming each change by kind, region, and size. | ✓ VERIFIED | See "CR-01 re-drive" below: a real, real-world-shaped adversarial test (not just the size-mismatch error path) built by me from scratch confirms `--region` combined with a correctly-sized mask-scoped `--rule` cannot report a false pass on a real untolerated change, and correctly reports success when the mask genuinely covers the change. My own `--region badge --report` run wrote a report naming `kind = "recoloured"`, `region = "badge"`, `size = 2000`. |
| 5 | A person accepts a new baseline explicitly. Nothing updates a baseline on its own, and every decode call sets an explicit memory limit. | ✓ VERIFIED | See "CR-02 re-drive" below: I obstructed the exact write step CR-02 named at the CLI level (not the unit-test level) and confirmed the previous baseline survives byte-for-byte and `chrys compare --baseline` still resolves it. Decode-limit guard (`crates/chrys-cli/tests/decode_limits_guard.rs`, `crates/chrys-rule/src/mask.rs`'s `load_mask` routing through `decode_guarded`) unaffected by this round's only diff; re-run, 2 passed. |
| 6 | The report names the file a frame came from, not only its index. | ✓ VERIFIED | My own `--region badge --report` run wrote `source = "base.png"` alongside `index = 0` in the same `[[frame]]` table. |

**Score:** 6/6 truths verified.

### CR-01 re-drive: not just the error path, a real positive/negative pair

The prior verification's CR-01 fixture (a 70x60 mask, deliberately
sized to trigger the "mask does not match the frame" refusal) only
proves the size-mismatch guard still fires. This round goes further and
builds an adversarial pair that exercises the actual coordinate-consistency
fix (`translate_verdicts` adding the crop origin back onto every
`Region.bbox`), using a mask correctly sized to the **full, uncropped**
256x256 frame (the size `frame_size` in `run_compare` always reads from
`base_frames`, never from the crop):

1. `exact-cover-mask.png` — 256x256, solid opaque white exactly over
   `x=40..90, y=40..80` (the known changed rectangle in global frame
   coordinates), solid transparent black elsewhere. Built with a
   standalone Python PNG encoder I wrote for this session (no reuse of
   `make-fixtures.rs`).
2. `off-target-mask.png` — same dimensions, solid opaque white over an
   unrelated rectangle (`x=200..230, y=200..230`), transparent elsewhere.

```
$ chrys compare base.png candidate.png --rule r_exact.toml               # no --region
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 ...
exit: 0

$ chrys compare base.png candidate.png --rule r_exact.toml --region badge
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 ...
exit: 0

$ chrys compare base.png candidate.png --rule r_offtarget.toml           # no --region
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 ...
exit: 1

$ chrys compare base.png candidate.png --rule r_offtarget.toml --region badge
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 ...
exit: 1
```

The mask that genuinely covers the change tolerates it (exit 0) with and
without `--region`; the mask that does not cover it correctly fails
(exit 1) with and without `--region`. `--region` cannot flip a real,
untolerated change into a false pass, and cannot flip a genuinely
tolerated one into a false fail. This is the property CR-01 named,
confirmed against a fixture pair this session built for that specific
purpose, not the repository's own committed size-mismatch fixture.

I also re-ran the size-mismatch regression (my own 70x60 mask, placed
inside the rule file's own directory to clear the path-traversal guard):
both with and without `--region`, `chrys compare` printed `mask
my-mask-70x60.png is 70x60, which does not match the 256x256 frame it is
scoped against` and exited 3 both times — no false green.

### CR-02 re-drive: obstructed write at the CLI level, fresh store

```
$ chrys accept pair-01 base.png --store /tmp/verify-p3/store2
exit: 0
$ shasum store2/MANIFEST.toml store2/pair-01/base.png   # captured as "before"
$ mkdir -p /tmp/verify-p3/store2/MANIFEST.toml.tmp      # obstruct the exact write step CR-02 named
$ chrys accept pair-01 candidate.png --store /tmp/verify-p3/store2
/tmp/verify-p3/store2/MANIFEST.toml.tmp: Is a directory (os error 21)
exit: 3
$ find store2 | sort                                    # no leftover staging directory
store2 / store2/MANIFEST.toml / store2/MANIFEST.toml.tmp / store2/pair-01 / store2/pair-01/base.png
$ shasum store2/MANIFEST.toml store2/pair-01/base.png   # "after" — diff against "before" is empty
$ chrys compare --baseline pair-01 --store /tmp/verify-p3/store2 base.png
identical
exit: 0
```

The obstructed accept exits non-zero, leaves the previous baseline
byte-for-byte unchanged (before/after shasum diff is empty), leaves no
stray `.pair-01.accept-tmp` staging directory, and `chrys compare
--baseline` still resolves the untouched baseline afterward. Rebuilt
independently in a fresh store directory this session, not reused from
the prior verification's own store path.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/chrys-rule/src/lib.rs`, `mask.rs`, `evaluate.rs` | Rule schema, mask loading, evaluation | ✓ VERIFIED | Present, substantive, wired; `#![forbid(unsafe_code)]` confirmed present in all 8 crates (`grep -rln "forbid(unsafe_code)"`). Unaffected by this round's only diff. |
| `crates/chrys-baseline/src/lib.rs`, `golden.rs` | `BaselineStore` trait + `GoldenFileStore` impl | ✓ VERIFIED | Trait unchanged; staged-write (`accept-tmp` / `MANIFEST.toml.tmp` / rename-swap) re-confirmed by my own interrupted-write drill above. |
| `crates/chrys-cli/src/main.rs`, `report.rs` | CLI wiring: compare, accept, rule gate, report | ✓ VERIFIED | `run_compare`'s uncropped `base_frames`/`candidate_frames` binding confirmed by direct read; `frame_size` for `chrys_rule::evaluate` reads from `base_frames` (always the full, uncropped size) both with and without `--region`, confirmed by direct read and by my own exact-cover/off-target mask drive above. |
| `examples/rules/example.toml` | Shipped scoped example | ✓ VERIFIED | Comment-only diff since the prior verification; both rules still scoped; the corrected CIE76 wording matches `colour.rs`'s own implementation, confirmed by reading `palette` 0.7.7's own vendored source. |
| `chrys-baselines/MANIFEST.toml` | Committed baseline manifest | ✓ VERIFIED | Tracked by git; unaffected. |
| `crates/chrys-cli/tests/region_rule.rs` | The `--region` + `--rule` + `--report` regression suite | ✓ VERIFIED | 4 tests, all re-run by me, all pass. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `chrys-cli::run_compare` | `chrys_rule::evaluate` | direct call, per-frame, against the uncropped base frame's hints and size | ✓ WIRED | Re-confirmed with a genuinely positive and negative mask pair this session built, both with and without `--region`; see CR-01 re-drive above. |
| `chrys-cli::run_compare` | `chrys_baseline::GoldenFileStore::resolve`/`verify_baseline_digests` | direct call | ✓ WIRED | Unaffected; re-confirmed by reading and by the CR-02 re-drive's own `compare --baseline` step above. |
| `chrys-cli::run_accept` | `chrys_baseline::GoldenFileStore::accept` | sole call site | ✓ WIRED, staged write confirmed durable | Re-confirmed by my own obstructed-write drive above, in a fresh store directory. |
| `chrys-cli::report::frame_report` | report `source`/`region`/`kind`/`size` fields | direct construction | ✓ WIRED | My own `--region badge --report` run wrote all four fields correctly in one `[[frame.change]]` table. |

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|---|---|---|---|
| RULE-01 | 03-01 | ✓ SATISFIED | Tolerance scoped by `ChangeKind`; unaffected by this round. |
| RULE-02 | 03-01, 03-02, 03-05 | ✓ SATISFIED | `Scope::Region`/`Scope::Mask` both implemented and tested; re-confirmed by the exact-cover/off-target mask drive above (mask scoping is correct with and without `--region`). |
| RULE-03 | 03-01, 03-02 | ✓ SATISFIED | No computable field in the rule schema; unaffected. |
| RULE-04 | 03-01, 03-02 | ✓ SATISFIED | Unscoped rule refused via `validate_row`; unaffected. |
| RULE-05 | 03-01 | ✓ SATISFIED | `an_unknown_key_fails_naming_the_line` re-run by me, passes; also drove live with a fresh malformed rule naming line 5. |
| BASE-01 | 03-04 | ✓ SATISFIED | `MANIFEST.toml` committed, tracked. |
| BASE-02 | 03-04 | ✓ SATISFIED | Byte-for-byte copy behaviour unaffected; my own accept/obstruct drive shows an untouched copy after a failed write. |
| BASE-03 | 03-04 | ✓ SATISFIED | Trait unchanged; `GoldenFileStore` sole impl. |
| BASE-04 | 03-04, 03-05 | ✓ SATISFIED | My own CLI-level interrupted-write drive (fresh store, not reused) confirms the previous baseline survives byte-for-byte and still resolves. |
| CLI-01 | 03-01, 03-05 | ✓ SATISFIED | My own positive/negative mask pair confirms exit code correctness with and without `--region`; no false pass, no false fail. |
| CLI-02 | 03-03, 03-05 | ✓ SATISFIED | Report's `region` field correct under `--region --report`, confirmed by my own run. |
| CLI-03 | 03-01, 03-04, 03-05 | ✓ SATISFIED | The flag-less path (no `--region`) is byte-identical to the region path's translation being a no-op at origin `(0,0)`, confirmed by my own exact-cover mask run producing the identical rectangle and exit code with and without `--region`. |
| CLI-04 | 03-02, 03-03 | ✓ SATISFIED | Static call-site guard and `#![forbid(unsafe_code)]` in 8/8 crates; unaffected by this round. |

**No orphaned requirements**: all 13 IDs (RULE-01..05, BASE-01..04, CLI-01..04) are claimed by one of the five plans' `requirements:` frontmatter, confirmed by grep.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` marker found in `examples/rules/example.toml` (the only file this round touched) | — | Scanned directly. |
| `crates/chrys-baseline/src/golden.rs` | ~283-294 | Documented, acknowledged non-atomicity between the two final swap-renames | ℹ️ Info | Carried forward from the prior verification, unaffected by this round's only diff; mitigated by `verify_baseline_digests`'s fail-closed behavior, matches the phase's own T-03-30/T-03-31 threat register entries. Not a new finding, not blocking. |

Neither of the two prior Critical review findings (CR-01, CR-02) has
regressed: confirmed by direct read of the current `run_compare` (no
rebinding of `base_frames`/`candidate_frames` to a cropped value) and of
`write_baseline` (no `fs::remove_dir_all` on the live baseline directory
before the new state is staged), plus my own independent behavioural
drives of both above.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| CR-01: a mask that genuinely covers the real change tolerates it, with and without `--region` | manual `chrys compare` drive, my own exact-cover mask | Exit 0 both ways | ✓ PASS |
| CR-01: a mask that does not cover the real change still fails, with and without `--region` (no false pass) | manual `chrys compare` drive, my own off-target mask | Exit 1 both ways | ✓ PASS |
| Size-mismatch guard still fires, with and without `--region` | manual `chrys compare` drive, my own 70x60 mask | Exit 3 both ways, names 70x60 vs 256x256 | ✓ PASS |
| Malformed rule fails loudly, names the line | manual `chrys compare --rule bad.toml` drive, my own file | Names line 5, `nonsense_field`, exit 3 | ✓ PASS |
| Report names file, kind, region, size under `--region --report` | manual drive | `source = "base.png"`, `kind = "recoloured"`, `region = "badge"`, `size = 2000` | ✓ PASS |
| CR-02: obstructed manifest write leaves the previous baseline byte-for-byte intact and still resolvable | manual `chrys accept` drive, fresh store, obstruct `MANIFEST.toml.tmp`, retry | Exit 3 on the obstructed accept; before/after shasum diff empty; `compare --baseline` still resolves | ✓ PASS |
| The example's own structural test is unaffected by the comment fix | `cargo test -p chrys-rule the_shipped_example_rule_file_parses_and_is_fully_scoped -- --exact` | 1 passed | ✓ PASS |
| Unknown-key regression test | `cargo test -p chrys-rule an_unknown_key_fails_naming_the_line -- --exact` | 1 passed | ✓ PASS |
| Full workspace test suite | `cargo test --workspace` (one full run) | 0 failures across every crate, all `test result: ok` | ✓ PASS |
| Lints clean | `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` | both clean, exit 0 | ✓ PASS |
| Engine boundary unmoved, whole phase range, resolved hashes | `bash scripts/engine-boundary-drill.sh $(git rev-parse 29f5add) $(git rev-parse bf3d749)` | 2 of 2 drills behaved as expected | ✓ PASS |
| Engine tree object id unmoved | `git ls-tree bf3d749 crates/chrys-core` | `tree c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 crates/chrys-core` | ✓ PASS — matches the recorded id exactly |

**On `scripts/engine-boundary-drill.sh`'s own `HEAD`-literal argument:**
confirmed independently (not merely trusted from the prior report's
narrative or 03-05-SUMMARY.md) that passing the literal string `HEAD` as
`<last-commit>` gives a false result, because the script re-resolves `$2`
inside its own disposable worktree, whose `HEAD` has by then advanced.
Resolved hashes were used for both arguments in the run recorded above,
and the drill passed 2 of 2.

### Probe Execution

No `scripts/*/tests/probe-*.sh` files exist in this repository and none
are referenced by this phase's plans/summaries. Skipped: no probes to run.

### Human Verification Required

None. All six roadmap success criteria and all 13 requirement IDs are
satisfied, each re-confirmed against fixtures and interruption points I
built fresh this session — including a genuinely new positive/negative
mask pair for CR-01 that goes beyond the prior verification's own
size-mismatch-only drive. The example.toml change since the prior
verification is confirmed comment-only by diff, by an unaffected
structural test, and by a direct read of `colour.rs` and palette 0.7.7's
own vendored source confirming the corrected CIE76 claim is accurate.
The full workspace test suite is green, lints are clean, and the engine
boundary (`crates/chrys-core`) tree object id (`c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`)
is confirmed unmoved by direct `git ls-tree` read and by a resolved-hash
run of the phase-gate drill script.

## Gaps Summary

None. This re-verification found no regression and no new gap. The one
non-planning change since the prior `passed` report — a corrected code
comment in `examples/rules/example.toml` — is confirmed comment-only and
factually accurate against the engine's own implementation, and could not
have changed behaviour. Phase 3's goal — a person commits a baseline,
scopes tolerance by rule, and gates CI on the verdict — remains achieved.

---

_Verified: 2026-09-08T01:30:00Z_
_Verifier: Claude (gsd-verifier)_
