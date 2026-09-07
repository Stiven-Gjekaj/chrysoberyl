---
phase: 03-baseline-rules-and-ci-gate
plan: 01
subsystem: rule-engine
tags: [rust, toml, serde, cli, tolerance-rule, ci-gate]

# Dependency graph
requires:
  - phase: 03-baseline-rules-and-ci-gate
    provides: "the engine tree object id c97a6fb778c4b1373e5c4dc563481cc18e4c98c0, the anchor every task and every plan of this phase must not move"
provides:
  - "chrys_rule::load_rules, the two-phase TOML rule reader (permissive Spanned row, then a semantic validation pass) with a type-level Scope that cannot express an unscoped rule (D-01)"
  - "chrys_rule::evaluate and overlapping_hint_name, turning a Verdict plus a rule set into a per-region Tolerated/Violation outcome, with the region/hint overlap test in pure u64 integer arithmetic"
  - "--rule <PATH> on chrys compare, additive to the existing exit-code scheme: a Changed verdict whose every region is tolerated ranks 0, unchanged otherwise (CLI-03's regression guard held)"
  - "tests/golden/rule-01, a committed fixture whose one Recoloured region measures a colour delta of 115.65, with tolerate.toml (116.0) and too-tight.toml (115.0) bracketing that measured value"
  - "crates/chrys-cli/tests/rule_gate.rs, the three-way exit-code proof (1 / 0 / 1) over the committed fixture"
  - "crates/chrys-rule/tests/rule_file.rs, nine failure/boundary cases plus rule_row_has_no_computable_field, RULE-03's own schema-shape guard"
affects: []

# Actuals (#2632)
actuals:
  tokens: 15636
  tasks: 3
  commits: 3
  plan_head_before: f234acc9e2f5eef1cba1a2e18ac0e9c3d6b0c4a1

# Tech tracking
tech-stack:
  added:
    - "chrys-rule (new workspace crate): depends on chrys-core and chrys-source by path, and on toml, serde (derive), thiserror — all three already pinned in the workspace Cargo.toml, no new external package"
  patterns:
    - "Two-phase deserialize: a permissive private RuleRow (every tolerance field Option<Spanned<T>>), then an explicit validate_row pass that serde cannot express — the exact shape chrys-source-raster/src/hints.rs already established for its own sidecar."
    - "Scope has exactly two variants (Region(String), Mask(PathBuf)) and no Option, so an unscoped rule is refused at the type level, not by a lint (D-01)."
    - "Tolerance carries the per-kind comparison logic as a private method (Tolerance::tolerates) inside lib.rs, so evaluate.rs's own geometry/overlap code stays pure u64 integer arithmetic with the one unavoidable f32 comparison (a Recoloured region's already-computed colour_delta.delta_e) living in the type that already declares that field, not in the file the plan asked to hold no float calculation of its own."
    - "The row schema (all 8 fields: kind, region, mask, max_offset_px, max_delta_e, max_alpha_delta, max_area_px, allow) and the full per-kind field-validity table were written together in Task 1's commit, since a partially-declared schema field that no code path reads is a rustc dead_code error under -D warnings. Task 2 and Task 3 then added only tests against that already-complete implementation. See Deviations."
  removed: []

key-files:
  created:
    - crates/chrys-rule/Cargo.toml
    - crates/chrys-rule/src/lib.rs
    - crates/chrys-rule/src/evaluate.rs
    - crates/chrys-rule/tests/rule_file.rs
    - crates/chrys-cli/tests/rule_gate.rs
    - tests/golden/rule-01/base.png
    - tests/golden/rule-01/candidate.png
    - tests/golden/rule-01/base.hints.toml
    - tests/golden/rule-01/candidate.hints.toml
    - tests/golden/rule-01/tolerate.toml
    - tests/golden/rule-01/too-tight.toml
  modified:
    - Cargo.lock
    - crates/chrys-cli/Cargo.toml
    - crates/chrys-cli/src/main.rs
    - crates/chrys-source-raster/examples/make-fixtures.rs

key-decisions:
  - "The full RuleRow schema (8 fields) and the full Tolerance enum (Colour, Allow, MaxOffsetPx, MaxAreaPx) plus the per-kind field-validity table were all written in Task 1's own commit, rather than split so that Task 1 built only Colour/Allow and Task 2 added the rest as the plan's prose describes. Reason: the plan's own Task 1 action text requires the row schema to declare all 8 fields from Task 1 onward, and a struct field or enum variant that no code path ever reads or constructs is a rustc dead_code lint, promoted to a hard error by cargo clippy --all-targets -- -D warnings, which Task 1's own <verify> block runs. Writing the complete validation and evaluation logic in Task 1, then adding Task 2's and Task 3's own tests against that already-complete code in their own commits, is the same precedent 02-05-SUMMARY.md recorded for deny_unknown_fields and the duplicate-region-name check (written in Task 1's commit, tested in Task 2's)."
  - "The one floating-point comparison a colour tolerance needs (delta.delta_e <= max_delta_e) lives in a private Tolerance::tolerates method inside lib.rs, not in evaluate.rs. The plan's Task 1 acceptance criterion states 'evaluate.rs holds no f32 and no f64; every overlap calculation is integer arithmetic,' and 03-VALIDATION.md's own note clarifies the concern is a floating-point calculation inside the evaluator's geometry, not a threshold comparison of an already-computed field the engine itself declares as f32. Keeping the comparison in lib.rs, next to the Colour variant that already declares the f32 field, satisfies the letter of the acceptance criterion (evaluate.rs itself contains zero f32/f64 tokens) without inventing a new integer encoding for a value the engine already publishes as a float."
  - "Tolerance::Allow carries the rule file's own bool value (Allow(bool)), not a value-less variant, so a rule file that writes allow = false is read literally (never tolerates) rather than being indistinguishable from allow being absent. Not exercised by any fixture in this plan; documented here since it is a design choice the plan's own prose ('a rule of any kind with allow = true tolerates...') did not settle for the false case."
  - "RULE_01_REGION margin is ten pixels on every side of STRUCTURED_RECTS[0]'s own rectangle (40, 40, 50, 40), giving a badge region of (30, 30, 70, 60): the recoloured bounding box's entire area (2000 px) falls inside the named region's area (4200 px), well over the required half."

patterns-established:
  - "A crate-boundary constraint stated as 'file X holds no f32/f64' is read as 'no floating-point calculation is introduced in file X', not as 'no f32-typed value may ever appear in a match arm inside file X' — an unavoidable comparison against an already-computed engine value is placed in the type that owns that value's declaration instead."

requirements-completed: [RULE-01, RULE-02, RULE-03, RULE-04, RULE-05, CLI-01, CLI-03]

coverage:
  - id: D1
    description: "A TOML rule file names a kind of change, a named region, and a tolerance, and the tool reads it as data. A rule table with neither region nor mask fails to deserialize, at the type level (D-01)."
    requirement: RULE-01, RULE-02, RULE-03, RULE-04
    verification:
      - kind: unit
        ref: "crates/chrys-rule/src/lib.rs#tests::a_scoped_recoloured_rule_loads_into_one_rule"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/src/lib.rs#tests::an_unscoped_rule_is_refused"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/tests/rule_file.rs#an_unscoped_rule_is_refused"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/tests/rule_file.rs#rule_row_has_no_computable_field"
        status: pass
    human_judgment: false
  - id: D2
    description: "The same pair exits 1 with no rule file, 0 with a rule file that tolerates its one measured change, and 1 with a rule file whose tolerance is one step too tight. The tolerance is measured from a real run, not chosen until the test passed."
    requirement: CLI-01
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/rule_gate.rs#exits_zero_when_every_change_is_tolerated"
        status: pass
      - kind: integration
        ref: "crates/chrys-cli/tests/rule_gate.rs#exits_nonzero_when_a_change_has_no_tolerating_rule"
        status: pass
      - kind: other
        ref: "manual runs recorded verbatim below: exit 1 (no --rule), exit 0 (--rule tolerate.toml), exit 1 (--rule too-tight.toml), all over tests/golden/rule-01"
        status: pass
    human_judgment: false
  - id: D3
    description: "A colour tolerance never tolerates an alpha-only change, because ColourDelta's delta_e carries no alpha axis; the alpha difference is read from the two colours' own fourth bytes."
    requirement: RULE-01
    verification:
      - kind: unit
        ref: "crates/chrys-rule/src/evaluate.rs#tests::a_recoloured_rule_with_a_generous_colour_limit_does_not_tolerate_an_alpha_only_change"
        status: pass
    human_judgment: false
  - id: D4
    description: "An unknown key, a wrong type, an invalid kind, and a tolerance field wrong for its own row's kind each fail with a message that names a line."
    requirement: RULE-05
    verification:
      - kind: unit
        ref: "crates/chrys-rule/tests/rule_file.rs#an_unknown_key_fails_naming_the_line"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/tests/rule_file.rs#a_wrong_typed_value_fails_naming_the_line"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/tests/rule_file.rs#an_invalid_kind_fails_naming_at_least_two_of_the_five_valid_kinds"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/src/lib.rs#tests::a_tolerance_field_wrong_for_its_kind_fails_naming_the_line"
        status: pass
    human_judgment: false
  - id: D5
    description: "A tolerance is scoped by the kind of change it names, for all five ChangeKind variants, and a rule never tolerates a region of a kind it does not name."
    requirement: RULE-01
    verification:
      - kind: unit
        ref: "crates/chrys-rule/src/evaluate.rs#tests::tolerance_is_scoped_by_change_kind"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/src/evaluate.rs#tests::a_moved_region_with_no_offset_is_a_violation_not_a_pass"
        status: pass
      - kind: unit
        ref: "crates/chrys-rule/src/evaluate.rs#tests::a_recoloured_region_with_no_colour_delta_is_a_violation_not_a_pass"
        status: pass
    human_judgment: false
  - id: D6
    description: "No file under crates/chrys-core/ changes anywhere in this plan's commit range, and cargo tree -p chrys-core -e normal names neither serde, toml, nor chrys-rule."
    requirement: (D-03, project-wide invariant)
    verification:
      - kind: other
        ref: "git diff --name-only 29f5add^..49c66fd -- crates/chrys-core/ (empty output); cargo tree -p chrys-core -e normal (no serde, no toml, no chrys-rule, no format crate); git rev-parse HEAD:crates/chrys-core read c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 before the plan's first commit and after each of its three commits"
        status: pass
    human_judgment: false
  - id: D7
    description: "The flag-less compare path is unchanged (CLI-03's regression guard)."
    requirement: CLI-03
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/digest.rs (all 6 tests, unmodified, still pass)"
        status: pass
    human_judgment: false

duration: not captured (PLAN_START_TIME was not recorded at launch; the three task commits span the session, see git log for exact timestamps)
completed: 2026-09-07
status: complete
---

# Phase 3 Plan 01: Read a scoped rule file and let it set the exit code Summary

**A `.toml` rule file names a kind of change, a scope (a named region or a mask path, never neither or both, enforced at the type level), and a tolerance; `chrys compare --rule <PATH>` reads it as data and the same measured pair gives three different exit codes — 1, 0, 1 — depending only on the number in that file, with `chrys-core`'s dependency graph and tree object id unmoved.**

## Performance

- **Duration:** not captured (see frontmatter note)
- **Completed:** 2026-09-07
- **Tasks:** 3 / 3
- **Files modified:** 15 (4 modified, 11 new)

## Accomplishments

- Created the `chrys-rule` crate. `crates/chrys-rule/src/lib.rs` deserializes a `[[rule]]` array into a private, permissive `RuleRow` (every field wrapped in `toml::Spanned<T>`, `deny_unknown_fields` on both the row and the document), then a `validate_row` pass converts each row into a public `Rule { kind, scope, tolerance }`. `Scope` is a two-variant enum (`Region(String)`, `Mask(PathBuf)`) with no `Option`, so an unscoped rule cannot be represented at all (D-01). `Tolerance` is a four-variant enum (`Colour { max_delta_e, max_alpha_delta }`, `Allow(bool)`, `MaxOffsetPx(u32)`, `MaxAreaPx(u64)`), with a per-kind field-validity table enforced by `reject_if_present`, which names the offending field's own `Spanned` line, not the enclosing table's first line.
- `pub const MAX_RULE_FILE_BYTES: u64 = 1024 * 1024` is checked against `std::fs::metadata(rule_path).len()` before any byte of the rule file is read, the same posture and limit `crates/chrys-source-raster/src/hints.rs` already takes for its own sidecar.
- `crates/chrys-rule/src/evaluate.rs` holds `overlapping_hint_name` (majority-overlap test, at least half a region's own bounding-box area must lie inside a named hint, pure `u64` integer arithmetic, no `f32`/`f64` anywhere in that file) and `evaluate(verdict, hints, rules) -> RuleOutcome`. `RuleOutcome::is_clean()` is true when it holds no `Violation`; an `Identical` or `Refused` verdict produces an empty outcome, and `evaluate`'s own doc comment states a caller must not read a `Refused` verdict's empty outcome as a pass.
- Added `--rule <PATH>` to `chrys compare`. When present, `load_rules` runs once, before either side is decoded, so a bad rule file fails before any image decode work happens. `verdict_rank` was extended (not replaced): a `Changed` verdict whose outcome is clean now ranks 0; every other case (including `Refused`, always 2) is unchanged from before this flag existed, satisfying CLI-03's regression guard.
- Extended `crates/chrys-source-raster/examples/make-fixtures.rs` with `write_rule_01()`. The base is `build_structured_canvas(0, 0)`; the candidate recolours `STRUCTURED_RECTS[0]` (the same construction the committed `should-register` corpus already uses for `recoloured_small`). A `badge` hints sidecar on both sides names `(30, 30, 70, 60)`, a ten-pixel margin around the recoloured rectangle's own `(40, 40, 50, 40)`.
- Ran `cargo run -q -p chrys-cli -- compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png` against the fixture and read the actual verdict (recorded verbatim below): kind `Recoloured`, measured colour delta **115.65**. Wrote `tolerate.toml` with `max_delta_e = 116.0` (just above) and `too-tight.toml` with `max_delta_e = 115.0` (just below), both scoped to `region = "badge"`.
- Wrote `crates/chrys-cli/tests/rule_gate.rs`: `exits_zero_when_every_change_is_tolerated` and `exits_nonzero_when_a_change_has_no_tolerating_rule`, both running the built binary via `Command::new(env!("CARGO_BIN_EXE_chrys"))` (the `chrys-cli`-only pattern `digest.rs` already established), asserting on both the exit code and that stdout still prints the verdict text.
- Wrote `crates/chrys-rule/tests/rule_file.rs`: 9 failure/boundary cases (unknown key, invalid kind, wrong type, no scope, both scopes, no tolerance, oversize, exactly-at-limit, no `[[rule]]` table at all) plus `rule_row_has_no_computable_field`, which reads `RuleRow`'s own source and fails when a ninth field appears (RULE-03).
- `git rev-parse HEAD:crates/chrys-core` read `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` before this plan's first commit and after each of its three commits. `cargo tree -p chrys-core -e normal` names neither `serde`, `toml`, `chrys-rule`, nor any format crate. `git diff --name-only 29f5add^..49c66fd -- crates/chrys-core/` printed nothing.
- `cargo test --workspace` (29 test binaries, all `ok`, 0 failed across the run), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --check` all pass, at every commit boundary.

## Task Commits

Each task was committed atomically:

1. **Task 1: Read a scoped rule file and let it set the exit code** — `29f5add`
2. **Task 2: Scope a tolerance by the kind of change it names** — `5b96a80`
3. **Task 3: Refuse a rule file that is unscoped or malformed** — `49c66fd`

**Plan metadata:** pending (this SUMMARY and STATE.md/ROADMAP.md updates are committed by the orchestrator, per this plan's execution instructions)

## Files Created/Modified

- `crates/chrys-rule/Cargo.toml` — new: `chrys-core`, `chrys-source` (path), `toml`, `serde`, `thiserror` (workspace)
- `crates/chrys-rule/src/lib.rs` — new: `load_rules`, `Rule`, `Scope`, `Tolerance` (and its private `tolerates` method), `RuleError`, `MAX_RULE_FILE_BYTES`, `LoadedRules`, and 15 unit tests
- `crates/chrys-rule/src/evaluate.rs` — new: `evaluate`, `overlapping_hint_name`, `RuleOutcome`, `RegionOutcome`, and 10 unit tests
- `crates/chrys-rule/tests/rule_file.rs` — new: 10 integration tests over `load_rules`'s failure and boundary surface
- `crates/chrys-cli/Cargo.toml` — added `chrys-rule` path dependency
- `crates/chrys-cli/src/main.rs` — `--rule <PATH>` flag, rule loading before decode, `verdict_rank` extended to read an optional `RuleOutcome`
- `crates/chrys-cli/tests/rule_gate.rs` — new: the three-way exit-code proof
- `crates/chrys-source-raster/examples/make-fixtures.rs` — `write_rule_01()`, the `rule-01` fixture generator
- `Cargo.lock` — `chrys-rule` added to the workspace lockfile; no new external crate versions
- `tests/golden/rule-01/{base,candidate}.png`, `{base,candidate}.hints.toml`, `tolerate.toml`, `too-tight.toml` — new committed fixture

## Decisions Made

- The full `RuleRow` schema (8 fields) and the full `Tolerance` enum (all 4 variants) plus the per-kind field-validity table were written together in Task 1's own commit; Task 2 and Task 3 then added only tests against that already-complete implementation. See Deviations below for why.
- The one floating-point comparison a colour tolerance needs lives in a private `Tolerance::tolerates` method inside `lib.rs`, not in `evaluate.rs`, so `evaluate.rs` itself contains zero `f32`/`f64` tokens, per Task 1's own acceptance criterion, while the comparison against the engine's own already-`f32` `ColourDelta::delta_e` field still happens exactly once.
- `Tolerance::Allow` carries the rule file's own literal `bool` (`Allow(bool)`), not a unit variant, so `allow = false` reads as "never tolerates" rather than being indistinguishable from `allow` being absent.
- `RULE_01_REGION` is `(30, 30, 70, 60)`: a ten-pixel margin on every side of `STRUCTURED_RECTS[0]`'s own `(40, 40, 50, 40)`, so the detected region's entire area falls inside the named hint, well over the required half.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The full rule schema and Tolerance enum were written in Task 1's commit, not split across Task 1 and Task 2 as the plan's prose describes**
- **Found during:** Task 1, while implementing the `RuleRow` struct the plan's own action text requires ("The row's field names are `kind`, `region`, `mask`, `max_offset_px`, `max_delta_e`, `max_alpha_delta`, `max_area_px` and `allow`" — stated as part of Task 1).
- **Issue:** The plan's prose says Task 1 "builds its `Colour` variant and its `Allow` variant; task 2 adds the rest" (`MaxOffsetPx`, `MaxAreaPx`). But Task 1's own action text also requires the row schema to declare all 8 fields from Task 1 onward. If the row declares `max_offset_px`/`max_area_px` fields that Task 1's own validation logic never reads or converts into a `Tolerance` variant, `cargo clippy --all-targets -- -D warnings` (which Task 1's own `<verify>` block runs) fails on rustc's `dead_code` lint for an unread private struct field. Building only `Colour`/`Allow` in Task 1 while declaring all 8 row fields was not achievable without either leaving fields unread (a hard clippy error) or inventing a placeholder error path Task 2 would need to delete, which is worse than building the complete, correct logic once.
- **Fix:** Wrote the complete `Tolerance` enum (all 4 variants), the complete per-kind field-validity table (`reject_if_present` calls for every field against every kind), and the complete `validate_row` conversion logic in Task 1's own commit. Task 2's commit then added only tests (`tolerance_is_scoped_by_change_kind`, `a_tolerance_field_wrong_for_its_kind_fails_naming_the_line`, the alpha-only test, and two "no measurement is a violation" tests) proving behaviour that was already correct and already covered by Task 1's own clippy-clean commit.
- **Files modified:** `crates/chrys-rule/src/lib.rs`, `crates/chrys-rule/src/evaluate.rs` (both committed in Task 1; Task 2 touched the same two files to add tests only).
- **Verification:** `cargo clippy -p chrys-rule --all-targets -- -D warnings` was run and confirmed clean at the end of Task 1's own commit, before Task 2 began. All of Task 2's own named verify commands (`tolerance_is_scoped_by_change_kind`, `a_tolerance_field_wrong_for_its_kind_fails_naming_the_line`, `alpha`) pass, each reporting exactly 1 test matched and passed, not `0 passed`.
- **Committed in:** `29f5add` (Task 1 commit; production code), `5b96a80` (Task 2 commit; tests only).
- **Precedent:** `02-05-SUMMARY.md`'s own Decisions section records the identical shape of choice for `deny_unknown_fields` and the duplicate-region-name check in `hints.rs`: written in Task 1's commit "since the parser could not have been correctly designed without them," with Task 2 adding only the tests that prove both.

---

**Total deviations:** 1 auto-fixed (blocking, Rule 3), matching a pattern this repository's own prior plan (02-05) already established and documented for an analogous case.
**Impact on plan:** No scope creep, no behaviour beyond what the plan specifies. The only change is which task's commit the already-required production code landed in; every test the plan names for Task 1, Task 2, and Task 3 exists, is named exactly as the plan specifies, and passes.

## Issues Encountered

None beyond the deviation recorded above.

## User Setup Required

None — no external service configuration required.

## The Three Runs Over `tests/golden/rule-01`, Recorded Verbatim

`cargo run -q -p chrys-cli -- compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png` (no `--rule`):

```
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 (base [200, 40, 40, 255], candidate [40, 200, 200, 255])
```

Exit code: `1`.

`cargo run -q -p chrys-cli -- compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png --rule tests/golden/rule-01/tolerate.toml`:

```
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 (base [200, 40, 40, 255], candidate [40, 200, 200, 255])
```

Exit code: `0`.

`cargo run -q -p chrys-cli -- compare tests/golden/rule-01/base.png tests/golden/rule-01/candidate.png --rule tests/golden/rule-01/too-tight.toml`:

```
Recoloured region at x=40, y=40, width=50, height=40, colour delta 115.65 (base [200, 40, 40, 255], candidate [40, 200, 200, 255])
```

Exit code: `1`.

The kind the engine actually reported is `Recoloured`, the measured colour delta is `115.65`, `tolerate.toml` uses `max_delta_e = 116.0` (just above), and `too-tight.toml` uses `max_delta_e = 115.0` (just below). Neither number was chosen until a test passed; both were read from the fixture's own real run, exactly as the plan requires.

## Engine Boundary, This Plan's Own Commit Range

`git diff --name-only 29f5add^..49c66fd -- crates/chrys-core/`:

```
(empty)
```

`git rev-parse HEAD:crates/chrys-core` read `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` before this plan's first commit (baseline, at `f234acc`) and after each of the three commits above — four readings, all identical, matching the anchor this plan's own frontmatter states.

`cargo tree -p chrys-core -e normal` names: `chrys-source`, `libm`, `palette` (and its own sub-dependencies), `rustfft`, `sha2`, `thiserror`. No `serde`, no `toml`, no `chrys-rule`, no format or GPU crate.

## Full Verification, This Plan's End State

```
cargo test -p chrys-rule: 15 (lib) + 10 (rule_file.rs) = 25 passed, 0 failed.
cargo test -p chrys-cli --test rule_gate: 2 passed.
cargo test -p chrys-cli --test digest: 6 passed (CLI-03 regression guard, unmodified).
cargo test -p chrys-core --test determinism: 6 passed (all six guards green).
cargo test --workspace: 29 test binaries, all "ok", 0 failed anywhere.
cargo clippy --workspace --all-targets -- -D warnings: clean.
cargo fmt --check: clean.
cargo tree -p chrys-core -e normal: no serde, no toml, no chrys-rule, no format/GPU crate.
git rev-parse HEAD:crates/chrys-core: c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 (unmoved).
```

## Predictions That Turned Out Wrong

- The plan's Task 2 narrative predicted the `Tolerance` enum and its per-kind field table would be built incrementally, with Task 1 covering only `Colour`/`Allow`. As recorded in Deviations, this could not be done without a `dead_code` clippy failure inside Task 1's own commit, so the complete implementation landed in Task 1 instead, with Task 2 and Task 3 contributing only tests. No other prediction in the plan (the measured colour delta's rough magnitude, the fixture's region-containment margin, the exit-code scheme, the `toml::Spanned` line-reporting behaviour) turned out wrong; all matched the plan's own `03-RESEARCH.md` verification.

## Next Phase Readiness

- RULE-01, RULE-02 (its named-region half), RULE-03, RULE-04 (its type-level half, D-01), RULE-05, CLI-01, and CLI-03's regression guard are all delivered and observable: a rule file a person can read in a pull request decides whether a real, measured comparison passes, and the same pair gives three different exit codes depending only on the number in that file.
- `LoadedRules` already carries the shape (`rules: Vec<Rule>`, with `Scope::Mask(PathBuf)` already a valid, if unresolved, variant) that plan 03-02 is expected to fill with decoded mask pixels, without changing `evaluate`'s own signature, exactly as this plan's action text anticipated.
- `chrys-core`'s tree object id (`c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`) and dependency graph (no `serde`, no `toml`) are unmoved and are the anchor every remaining task and plan of phase 3 must keep checking against.

## Self-Check: PASSED

All created files verified present on disk (`crates/chrys-rule/Cargo.toml`, `crates/chrys-rule/src/lib.rs`, `crates/chrys-rule/src/evaluate.rs`, `crates/chrys-rule/tests/rule_file.rs`, `crates/chrys-cli/tests/rule_gate.rs`, `tests/golden/rule-01/base.png`, `tests/golden/rule-01/candidate.png`, `tests/golden/rule-01/base.hints.toml`, `tests/golden/rule-01/candidate.hints.toml`, `tests/golden/rule-01/tolerate.toml`, `tests/golden/rule-01/too-tight.toml`); all three task commit hashes (`29f5add`, `5b96a80`, `49c66fd`) verified present in `git log --oneline`; `git rev-parse HEAD:crates/chrys-core` verified reading `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` after the final commit.

---
*Phase: 03-baseline-rules-and-ci-gate*
*Completed: 2026-09-07*
