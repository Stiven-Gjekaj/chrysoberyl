---
phase: "3"
slug: "baseline-rules-and-ci-gate"
status: validated
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-07"
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

This file is seeded from the `## Validation Architecture` section of
`03-RESEARCH.md`, before the planner runs, and its body below is that section
verbatim rather than a paraphrase of it.

The ordering matters and is the point. Phase 1 shipped with no validation
contract, because the researcher was never asked for the section and the
plan-checker's own blocking gate for the missing file did not fire. Phase 2
fixed that by ordering. Phase 3 keeps the same order, and the orchestrator
had to send the researcher a correction mid-run to get the section at all,
which is recorded here so the next phase's brief includes it from the start.

`nyquist_compliant` stays `false` until the plans exist and the five checks
run against them.

---

## Two decisions taken before planning

The research left two questions open with recommendations. Both are settled
here so the planner does not reopen them.

**An unscoped rule is refused at parse time, not linted.** Success criterion 1
says the shipped example shows a scoped exclusion and never a bare global
threshold. A type-level refusal makes that true of every rule file, not only
the shipped one. The stronger form is easier to loosen later than the reverse.

**A mask pixel is tolerated when it is white AND opaque.** The research
recommended white on all three colour channels at 250 or above, ignoring
alpha. Ignoring alpha is the wrong half of that. Phase 1 settled that alpha is
part of what "changed" means, and there is a concrete failure: a mask authored
as white on transparency is (255, 255, 255, 0) everywhere outside the drawn
area, so a convention that ignores alpha tolerates the whole canvas without
saying so. Requiring the pixel to be opaque costs nothing and removes that.

---


### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in), same as Phase 1 and Phase 2 — no new framework is introduced |
| Config file | none new |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo test --workspace --all-features` |
| Typical runtime | Not measured this session. No baseline number exists for this workspace's full suite yet; do not plan a timing budget around an invented figure. Measure once Wave 0's crates exist, and record the real number in that wave's own summary. |

### Sampling Rate

- **Per task commit:** `cargo test --workspace` (scoped to `-p <touched-crate>` is acceptable for a fast inner loop, but the full workspace command is what a task's own verification step should record, per this project's existing convention in `01-RESEARCH.md`/`02-RESEARCH.md`).
- **Per wave merge:** `cargo test --workspace --all-features`, **plus this phase's own engine-boundary gate**: `scripts/engine-boundary-drill.sh <wave-first-commit> <wave-last-commit>`. This script already exists, is already generic over any commit range, and needs no modification `[VERIFIED: scripts/engine-boundary-drill.sh:1-154, read in full this session]` — it is the exact mechanism Phase 2 built for its own SRC-08 claim, and this phase's headline architectural claim ("zero files under `crates/chrys-core/` change") is the identical shape of claim, checked the identical way. Every wave of this phase should record its own first/last commit hash and run this drill before the wave is considered merged, the same discipline `02-LEARNINGS.md` records for Phase 2's three format waves.
- **Phase gate:** full suite green, `scripts/engine-boundary-drill.sh` green across the *whole phase's* commit range (not only per-wave), and the six-runner determinism matrix still green (this phase adds no new fixture to that matrix unless a plan later decides the rule/baseline machinery needs its own cross-OS proof — see the note below) — all three, before `/gsd-verify-work`.

**Note on the determinism matrix:** this phase's own new code (`chrys-rule`, `chrys-baseline`, the CLI's new flags) does no floating-point comparison work of its own — it reads already-computed `Verdict`/`Region` values and does integer/rectangle arithmetic (Pattern 2) or file I/O (Pattern 3). It therefore is not expected to need a new cross-OS golden-hash fixture the way Phase 1's decode path or Phase 4's SVG rasterizer will. If a plan discovers a floating-point comparison inside the rule evaluator (e.g. the majority-overlap area calculation, which this research specifies in integer arithmetic precisely to avoid this), that discovery is itself a signal to route it through the same `libm`/integer discipline `01-RESEARCH.md` established, not to add a new determinism proof for it.

### Phase Requirements → Test Map

Every command below either names one real, specific test function as a `cargo test` substring filter with no `--exact` (confirmed this session to select exactly that one test and nothing else — see Pitfall 3), or names a full `module::tests::function` path *with* `--exact`. All test files are new (Wave 0); none exist today.

| Req ID | Behaviour | Test Type | Automated Command | File Exists? |
|--------|-----------|-----------|---------------------|-------------|
| RULE-01 | A rule sets tolerance scoped by `kind` (one of the five `ChangeKind` variants) | unit | `cargo test -p chrys-rule tolerance_is_scoped_by_change_kind` | ❌ Wave 0 |
| RULE-02 | A rule scoped to a named region matches a region overlapping that hint; a rule scoped to a mask matches a region overlapping the mask's tolerated pixels | unit | `cargo test -p chrys-rule region_scoped_rule_matches_overlapping_region` and `cargo test -p chrys-rule mask_scoped_rule_matches_overlapping_region` | ❌ Wave 0 |
| RULE-03 | The rule schema has no field that can hold a computed value; deserializing a rule row never executes anything (a static claim, tested by asserting the deserialized type is plain data — no `Fn`, no closure, no expression string) | unit | `cargo test -p chrys-rule rule_row_has_no_computable_field` | ❌ Wave 0 |
| RULE-04 | A rule table with neither `region` nor `mask` is refused at parse/validation time | unit | `cargo test -p chrys-rule an_unscoped_rule_is_refused` | ❌ Wave 0 |
| RULE-04 | The shipped `examples/rules/example.toml` file parses successfully and contains no unscoped rule | integration | `cargo test -p chrys-rule the_shipped_example_rule_file_parses_and_is_fully_scoped` | ❌ Wave 0 |
| RULE-05 | An unknown key in a rule file fails, and the message names the line | unit | `cargo test -p chrys-rule an_unknown_key_fails_naming_the_line` | ❌ Wave 0 |
| RULE-05 | A syntactically valid but semantically wrong field (e.g. `max_delta_e` on a `moved` rule) fails, naming the exact field's own line | unit | `cargo test -p chrys-rule a_tolerance_field_wrong_for_its_kind_fails_naming_the_line` | ❌ Wave 0 |
| BASE-01 | `accept` writes `MANIFEST.toml`, and it is not excluded by `.gitignore` | integration | `cargo test -p chrys-baseline accept_writes_a_manifest_toml_file` (the `.gitignore` claim itself was verified this session by reading the file directly, not by a test — see Sources) | ❌ Wave 0 |
| BASE-02 | `accept` copies the candidate file (or directory) verbatim into the store, byte for byte | integration | `cargo test -p chrys-baseline accept_copies_the_candidate_file_byte_for_byte` | ❌ Wave 0 |
| BASE-03 | `GoldenFileStore` is the only concrete type implementing `BaselineStore` referenced by `chrys-cli`, and the trait itself has exactly two methods | unit | `cargo test -p chrys-baseline the_trait_has_exactly_resolve_and_accept` | ❌ Wave 0 |
| BASE-04 | `resolve` on a name that was never accepted returns an error, never a silent accept | unit | `cargo test -p chrys-baseline resolving_an_unaccepted_name_is_an_error` | ❌ Wave 0 |
| CLI-01 | `compare --rule <path>` exits non-zero when a changed region has no tolerating rule, and exits zero when every changed region is tolerated | integration | `cargo test -p chrys-cli exits_nonzero_when_a_change_has_no_tolerating_rule` and `cargo test -p chrys-cli exits_zero_when_every_change_is_tolerated` (both via `Command::new(env!("CARGO_BIN_EXE_chrys"))`, the pattern already used in `crates/chrys-cli/tests/digest.rs:8-24`, verified this session) | ❌ Wave 0 |
| CLI-02 | `compare --report <path>` writes a TOML file naming every changed region's kind, region (if any) and size | integration | `cargo test -p chrys-cli the_report_names_kind_region_and_size_per_change` | ❌ Wave 0 |
| CLI-03 | `compare <base> <candidate>` with no new flags prints the verdict and exits as it did before this phase (regression guard on already-existing behaviour) | integration | `cargo test -p chrys-cli a_second_run_gives_byte_identical_stdout` — **already exists today**, at `crates/chrys-cli/tests/digest.rs:82` `[VERIFIED: crates/chrys-cli/tests/digest.rs, `grep -n "fn "` output this session]` | ✅ exists |
| CLI-04 | Every `image::ImageReader::open(` call site in the workspace is one of the already-known, allow-listed sites | unit (static guard, mirroring `determinism.rs`'s own technique) | `cargo test -p chrys-cli only_allow_listed_call_sites_open_an_image_reader_directly` | ❌ Wave 0 |

### Manual-Only Verifications

| Verification | Why it cannot be automated |
|---------------|------------------------------|
| The shipped `examples/rules/example.toml` reads clearly to a person unfamiliar with the schema | RULE-04's own criterion is about a human reading a PR, not a parseable property; `the_shipped_example_rule_file_parses_and_is_fully_scoped` (above) proves the file is *valid and scoped*, not that its comments and structure are *legible* |
| A `MANIFEST.toml` diff, in an actual pull request view (e.g. GitHub's diff renderer), reads as "this baseline's hash changed" without opening the binary image | This is a claim about a PR review UI's rendering, not about this repository's own code; the closest automated proxy is asserting the diff is a small, single-line change (already covered by BASE-01/BASE-02's tests above), but "reads clearly to a person" itself needs a human to look |

### The risks this phase carries

A contract that lists only what passes is not a contract. Two risks are structural to this phase, not incidental:

1. **A rule file is new, structured input read as data, but it is still attacker-reachable input if a repository's rule file is ever supplied by an untrusted contributor (e.g. a rule file changed in a pull request from a fork, then read by a CI job with write access to the baseline).** RULE-03's "no rule is computed or executed" is the mitigation in principle; the risk this phase must not silently reintroduce is a rule field that, in a future revision, gains an "execute this" escape hatch (a shell-out, a glob pattern reaching outside the repository, a mask path that traverses `../` outside the project root). The mask-path field in particular should be checked against path traversal (`mask` resolves relative to the rule file's own directory and must not escape it) — this is not yet a requirement ID, but it is the concrete form RULE-03's promise takes once a real file-path field exists in the schema. **Recommendation:** the plan should add an explicit test asserting a `mask = "../../etc/passwd"`-shaped path is refused, not silently canonicalized and read.

2. **A baseline that a person accepts is a place where a wrong acceptance becomes the new truth, and nothing downstream can tell the difference between a correct accept and a mistaken one.** BASE-04 ensures no *automatic* update happens, but it does not, by itself, protect against a person running `accept` against a candidate that itself already contains the regression it was meant to catch (e.g. accepting a broken render as the new golden by habit, without looking). This is a process risk, not a code defect, and this research does not recommend building an approval-gate feature for it in Phase 3 (out of scope; the native window's own accept/reject flow in Phase 6 is the actual UI-level mitigation for this, per the roadmap). **Recommendation:** the `accept` subcommand's own CLI help text and the shipped example documentation should say, plainly, that `accept` is a statement "this is now correct," not a neutral bookkeeping operation — the same discipline `AGENTS.md`'s own "What to measure" section asks for elsewhere in this project (measure and name the thing you actually mean, not something adjacent to it).

---

## Validation Sign-Off

- [ ] All tasks have an automated verify or a Wave 0 dependency.
- [ ] Sampling continuity: no 3 consecutive tasks without an automated verify.
- [ ] Wave 0 covers every missing reference.
- [ ] No watch-mode flags.
- [ ] Feedback latency measured.
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending. The boxes above are checked when the plans exist and
the checks run against them, not before.
