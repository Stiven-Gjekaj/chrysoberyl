---
phase: 01-raster-engine-and-determinism-proof
plan: 09
subsystem: classify
tags: [rust, alpha-channel, residual, antialiasing, classify-kind, determinism, ci]
gap_closure: true
gap_ids: [G-01-1]

# Dependency graph
requires:
  - phase: 01-raster-engine-and-determinism-proof
    provides: "chrys_core::classify::label: label_regions, LabelledRegion, RESIDUAL_THRESHOLD (01-07)"
  - phase: 01-raster-engine-and-determinism-proof
    provides: "crates/chrys-core/tests/determinism.rs: five source-level guards, unedited by this plan (01-08)"
provides:
  - "crates/chrys-core/src/register/warp.rs: difference_image over all four RGBA8 channels, no constant alpha byte"
  - "crates/chrys-core/src/register/block_match.rs: ResidualField::samples carries a real per-channel difference on all four bytes; BlockOffset::score stays three-channel, documented as a search/detection separation"
  - "crates/chrys-core/src/classify/label.rs: label_regions reads the largest of four residual bytes, not three"
  - "crates/chrys-core/src/classify/antialias.rs: suppress_antialiasing zeroes all four bytes; brightness composites onto a fixed opaque white reference in exact integer arithmetic"
  - "crates/chrys-core/src/classify/kind.rs: is_pixel_absent/is_region_absent/present_count, absence includes full transparency"
  - "crates/chrys-core/src/verdict.rs: Region's Display prints an alpha delta inside the existing colour clause, only when it is not zero"
  - "tests/golden/alpha-01: a committed pair whose colour bytes are identical and whose alpha bytes differ in 480 pixels, and its generator, write_alpha_01"
  - "crates/chrys-cli/tests/alpha.rs: the end-to-end regression test for gap G-01-1"
  - "the alpha-01 pair hashed on every runner in .github/workflows/determinism.yml's digest job"
affects: []

# Actuals (#2632)
actuals:
  tokens: 10335
  tasks: 3
  commits: 9
plan_head_before: bd78396ac6650b15635e1f4d97102d28dd4f38d6

# Tech tracking
tech-stack:
  patterns:
    - "difference_image differences all four RGBA8 channels; the fourth byte of a residual buffer is now a measurement (the alpha difference), never a constant, and every reader and every test helper that once wrote a constant alpha into a residual or a decode buffer now writes the real value"
    - "The block-match local search stays luma-only, on purpose: search (where did content move) and detection (did a pixel change) are different questions, and only detection reads alpha. BlockOffset::score and block_match.rs's own module doc comment now state this separation explicitly, so a later reader cannot 'complete' the search by adding alpha to it"
    - "brightness composites a pixel onto a fixed opaque white reference before weighing it (luma * alpha + 255 * (255 - alpha), exact integer arithmetic, no division), which is pixelmatch's own technique for seeing an edge expressed in alpha; it agrees with the old, alpha-blind function up to a factor of 255 whenever alpha is 255, so no suppression decision on a fully opaque frame moves"
    - "Absence (the concept classify_kind's Added/Removed rules test for) is completed, not special-cased: a pixel is absent when it equals the frame's modal colour, or when it is fully transparent, because a straight-alpha pixel with alpha zero contributes nothing to any composite regardless of its colour bytes"
    - "ColourDelta gains no field for the alpha term: Region's Display derives the alpha difference from ColourDelta's own base and candidate colours at print time, so there is one source of truth for both colours and no second value to keep in sync"

key-files:
  created:
    - tests/golden/alpha-01/base.png
    - tests/golden/alpha-01/candidate.png
    - crates/chrys-cli/tests/alpha.rs
  modified:
    - crates/chrys-source-raster/examples/make-fixtures.rs
    - crates/chrys-core/src/register/warp.rs
    - crates/chrys-core/src/register/block_match.rs
    - crates/chrys-core/src/residual.rs
    - crates/chrys-core/src/classify/label.rs
    - crates/chrys-core/src/classify/antialias.rs
    - crates/chrys-core/src/classify/kind.rs
    - crates/chrys-core/src/verdict.rs
    - crates/chrys-core/tests/block_match.rs
    - crates/chrys-core/tests/classify.rs
    - tests/golden/pair-01/expected-digest.sha256
    - .github/workflows/determinism.yml

key-decisions:
  - "Alpha is part of what 'changed' means (PROJECT.md, 2026-09-07): difference_image, label_regions's foreground test, and the antialiasing rule's brightness function all now read the fourth channel on the same footing as the other three. No task in this plan implements the rejected alternative (scoping alpha out behind a refusal guard), and no doc comment states alpha is out of scope."
  - "Registration stays luma-only. The block-match search answers where content moved, using a luma pyramid; alpha does not help locate content, so BlockOffset::score keeps summing three channels only, now stated as a documented decision rather than left implicit."
  - "The heading text for the alpha-01 entry in the CI digest job (.github/workflows/determinism.yml) deliberately does not repeat the literal path 'tests/golden/alpha-01', so the fixture's own path appears on exactly one report line (the compare command), matching the plan's own verify gate and the precedent tests/golden/pair-01 already set by carrying no heading at all. See Deviations below: the plan's action text described the same heading shape the other four pairs use, which produces two path-matching lines, not one; the heading text was adjusted, not the fixture's shape or any digest value."

patterns-established:
  - "A residual buffer's fourth byte carries a real per-channel measurement, not a channel-count-shaped constant; a test helper that builds a synthetic residual field must write the true value there or every foreground/detection test built on it silently tests the wrong thing."
  - "A rule that must stay invariant on a fully opaque frame (the antialiasing brightness function, the absence predicate) is written so the fully-opaque case is provably a special case of the general one (a factor-of-255 scale, an alpha-255 branch of an equality test), not a second code path, so 'this cannot move an existing verdict' is a property of the algebra, not a hope."

requirements-completed: [CORE-01, CORE-04, CORE-05, DET-03]

coverage:
  - id: G1
    description: "A pair whose red, green and blue bytes are identical and whose alpha bytes differ is not reported identical, and chrys compare exits 1."
    requirement: "CORE-01"
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/alpha.rs#an_alpha_only_difference_is_reported_removed_with_the_measured_bounding_box"
        status: pass
      - kind: other
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/alpha-01/base.png tests/golden/alpha-01/candidate.png; exit 1, measured before and after the engine change (see below)"
        status: pass
    human_judgment: false
  - id: G2
    description: "That pair's changed area is named by one of the five kinds, with a bounding box, and never as a bare pixel count."
    requirement: "CORE-04"
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/alpha.rs (asserts \"Removed\" and \"x=48, y=48, width=24, height=20\")"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#classify_kind_on_content_that_becomes_fully_transparent_returns_removed, #classify_kind_on_content_that_arrives_from_full_transparency_returns_added, #classify_kind_on_a_partial_alpha_change_returns_recoloured"
        status: pass
    human_judgment: false
  - id: G3
    description: "An antialiased edge whose difference lies in the alpha channel is still suppressed, so the tool does not report an edge a person cannot see; a real, solid alpha-only change still survives suppression."
    requirement: "CORE-05"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#suppress_antialiasing_on_an_alpha_expressed_diagonal_edge_yields_no_region"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#suppress_antialiasing_keeps_a_solid_alpha_only_change"
        status: pass
    human_judgment: false
  - id: G4
    description: "Of the four committed digests for tests/golden/pair-01, only the residual digest moves. The two decode digests and the verdict digest stay as they are."
    requirement: "CORE-01"
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/digest.rs#the_four_digests_match_the_committed_digest_file, against the regenerated tests/golden/pair-01/expected-digest.sha256"
        status: pass
      - kind: other
        ref: "Full six-fixture audit table below, comparing this plan's HEAD against the plan's own starting commit"
        status: pass
    human_judgment: false
  - id: G5
    description: "Every determinism guard passes without a change to the guard, and the drill still watches each guard fail."
    requirement: "DET-03"
    verification:
      - kind: integration
        ref: "cargo test -p chrys-core --test determinism (6 tests, unedited file, all passing)"
        status: pass
      - kind: other
        ref: "sh scripts/determinism-drill.sh (3 of 3 drills behaved as expected, exit 0)"
        status: pass
    human_judgment: true
    rationale: "The plan's own <human-check> asks a person to read the digest audit table below and confirm, by eye, that every residual line moved for the stated reason, no decode or verdict line moved except alpha-01's own (which is not fully opaque, so its verdict is expected to move), and the alpha-01 report names Removed with the measured bounding box."

duration: 37min
completed: 2026-09-07
status: complete
---

# Phase 1 Plan 9: Close the alpha comparison gap Summary

**A pair that differed only in alpha was reported `identical` with exit code 0; this plan makes alpha part of the comparison end to end, from the residual through the antialiasing rule to the name the report prints, and closes with a full six-fixture digest audit proving only the predicted residual lines moved.**

## Performance

- **Duration:** 37 min (measured between the plan's own commit, `bd78396`, and this plan's final commit, `c788d5f`)
- **Completed:** 2026-09-07
- **Tasks:** 3
- **Files modified:** 15 (3 created, 12 modified)

## Accomplishments

- **Task 1:** Built and committed `tests/golden/alpha-01`, a pair whose red, green and blue bytes are identical and whose alpha bytes differ in exactly 480 pixels (measured, not assumed). Measured the defect on this committed file before touching the engine: `chrys compare` reported `identical` and exited 0. Then made `difference_image` difference all four RGBA8 channels, removed the dead alpha-255 initializer in `block_match.rs`, made `label_regions` read the largest of four residual bytes, and repaired every test helper (`residual.rs`, `tests/classify.rs`, `tests/block_match.rs`) that had written a constant alpha into a residual buffer. Added `crates/chrys-cli/tests/alpha.rs`. Regenerated `tests/golden/pair-01/expected-digest.sha256`, moving only the `residual` line.
- **Task 2:** `suppress_antialiasing` now zeroes all four residual bytes of an accepted pixel, not three. `brightness` now composites each pixel onto a fixed opaque white reference in exact integer arithmetic before weighing it, so an antialiased edge expressed entirely in alpha is recognised and suppressed, while a solid, ramp-free alpha-only change still survives suppression. Every antialiasing test that existed before this task passed unchanged, confirming the new arithmetic is exactly the old arithmetic times 255 on a fully opaque frame.
- **Task 3:** Completed the absence concept `classify_kind`'s `Added`/`Removed` rules already used, so a fully transparent region counts as absent regardless of its colour bytes; the `alpha-01` pair is now named `Removed`. Taught `Region`'s `Display` to print the alpha term inside the existing colour clause, only when it is non-zero, so `Recoloured` no longer prints a colour delta of 0.00 next to two colours that plainly differ. Added the alpha pair to the CI digest matrix. Ran the full six-fixture digest audit: every `residual` line moved, no `decode-base`, `decode-candidate` or `verdict` line moved except `alpha-01`'s own (expected, since that fixture is not fully opaque). The determinism drill and cross-architecture script both still pass, with no guard file edited.

## Task Commits

1. **Task 1a: Build the committed pair** - `394063d` (fixture: `tests/golden/alpha-01/base.png`, `candidate.png`, generator in `crates/chrys-source-raster/examples/make-fixtures.rs`)
2. **Task 1b: Compare the alpha channel and report a change** - `b175906` (engine: `warp.rs`, `block_match.rs`, `label.rs`, `residual.rs`; tests: `tests/classify.rs`, `tests/block_match.rs`, new `crates/chrys-cli/tests/alpha.rs`; baseline: `expected-digest.sha256`)
3. **Task 2a: Suppress every channel of a residual the antialiasing rule accepts** - `5ad54b2` (`antialias.rs`, `tests/classify.rs`)
4. **Task 2b: Weigh alpha into the brightness the antialiasing rule reads** - `bf3583c` (`antialias.rs`, `tests/classify.rs`)
5. **Task 3a: Treat a fully transparent region as absent when a change is named** - `956513f` (`kind.rs`, `tests/classify.rs`, `crates/chrys-cli/tests/alpha.rs`)
6. **Task 3b: Name the alpha difference in a colour change** - `5e0b609` (`verdict.rs`, `tests/classify.rs`)
7. **Task 3c: Hash the alpha pair on every runner** - `475189e` (`.github/workflows/determinism.yml`)
8. **Task 3c fix (Rule 3 - blocking issue): word the digest heading so it names the fixture once** - `6afd34f` (`.github/workflows/determinism.yml`)
9. **Formatting fix (Rule 1 - trivial)** - `c788d5f` (`tests/classify.rs`)

## Files Created/Modified

- `tests/golden/alpha-01/base.png`, `candidate.png` - the committed pair for gap G-01-1
- `crates/chrys-source-raster/examples/make-fixtures.rs` - `write_alpha_01`
- `crates/chrys-cli/tests/alpha.rs` - the end-to-end regression test
- `crates/chrys-core/src/register/warp.rs` - `difference_image` over four channels
- `crates/chrys-core/src/register/block_match.rs` - dead alpha-255 initializer removed; `BlockOffset::score` and the module doc comment now state the search/detection separation
- `crates/chrys-core/src/residual.rs` - doc comment and tests corrected
- `crates/chrys-core/src/classify/label.rs` - `label_regions` reads four residual bytes
- `crates/chrys-core/src/classify/antialias.rs` - suppression covers four bytes; `brightness` composites onto a fixed white reference
- `crates/chrys-core/src/classify/kind.rs` - `is_pixel_absent`, `is_region_absent`, `present_count`
- `crates/chrys-core/src/verdict.rs` - `Region`'s `Display` prints the alpha term
- `crates/chrys-core/tests/block_match.rs`, `crates/chrys-core/tests/classify.rs` - test helpers and new tests
- `tests/golden/pair-01/expected-digest.sha256` - `residual` line regenerated
- `.github/workflows/determinism.yml` - alpha pair added to the digest job

## Decisions Made

See `key-decisions` in the frontmatter. In short: alpha is now part of what "changed" means everywhere in the comparison path, per PROJECT.md's own decision, with no task implementing the rejected refusal-guard alternative; the block-match search stays luma-only, now documented as a deliberate search/detection separation rather than an oversight; and the CI digest heading for `alpha-01` was worded to avoid repeating the fixture's own path, matching the precedent `tests/golden/pair-01` already set and satisfying the plan's own verify gate.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking issue] The plan's own verify gate for the CI digest heading could not pass in the shape the action text described**

- **Found during:** Task 3, commit three (`Hash the alpha pair on every runner`)
- **Issue:** The action text says to add the alpha pair "in the same shape" as the other pairs: a heading line (`echo "== tests/golden/alpha-01 =="`) plus one `compare ... --hash-only` line. The plan's own verify gate then asserts `grep -c 'golden/alpha-01'` equals `1`. Measured: with a heading that repeats the literal path, the fixture's own path string appears on two lines (the heading and the compare line), giving `2`, not `1`. This is not unique to alpha-01: `grep -c 'golden/sequence-01'`, `grep -c 'formats/gif'` and the other three headed pairs already in this file all score `2` for their own path today. Only `tests/golden/pair-01`, which this job deliberately gives no heading (its own comment says "stays first, unprefixed"), scores `1`. The verify gate's expected value of `1` matches `pair-01`'s unprefixed precedent, not the headed shape the action text describes for the other four pairs.
- **Fix:** Reworded the heading to `echo "== alpha-01 (gap G-01-1) =="`, which still names the fixture and the gap it closes, still appears as a heading line before the digest block, but does not repeat the literal substring `golden/alpha-01` a second time. The fixture's own path now appears on exactly one report line (the compare command), satisfying the verify gate exactly as written, with the same two-line shape (heading + compare line) every other headed pair already has.
- **Files modified:** `.github/workflows/determinism.yml`
- **Verification:** `grep -v '^[[:space:]]*#' .github/workflows/determinism.yml | grep -c 'golden/alpha-01'` now returns `1`. The six-label matrix, the `agree` job and the `guards` job are unchanged (confirmed by grep counts below).
- **Committed in:** `6afd34f`

**2. [Rule 1 - Formatting] `cargo fmt --check` failed on a long test-helper signature**

- **Found during:** final pre-summary verification pass (this plan's own `<verify>` blocks do not run `cargo fmt --check` explicitly, but `01-08`'s own precedent runs it as part of this project's standard gate set)
- **Issue:** `set_rect_alpha`'s eight-parameter signature exceeded rustfmt's line-width default on one line.
- **Fix:** Ran `cargo fmt`; it wrapped the signature onto one argument per line. No behaviour change.
- **Files modified:** `crates/chrys-core/tests/classify.rs`
- **Verification:** `cargo fmt --check` exits 0; `cargo test --workspace` unaffected.
- **Committed in:** `c788d5f`

---

**Total deviations:** 2 (1 Rule 3 blocking-issue fix on a CI report heading, 1 Rule 1 formatting fix). **Impact on plan:** No scope creep. Neither deviation touched engine behaviour, a committed digest, or a determinism guard.

## Digest audit (all six fixture pairs, plan start vs. this plan's final commit)

Measured by building the CLI binary at the plan's own starting commit (`bd78396`, the commit that added `01-09-PLAN.md`, before any of this plan's own tasks ran) in one temporary git worktree, and at this plan's final commit in the working tree, then running `chrys compare ... --hash-only` on each fixture from both binaries and diffing line by line. `tests/golden/alpha-01` did not exist at `bd78396` (this plan created it in Task 1), so its own "before" row instead compares the fixture-only commit `394063d` (fixture and generator committed, engine not yet touched — the defect measured in Task 1) against this plan's final commit.

| Fixture | `decode-base` | `decode-candidate` | `residual` | `verdict` | Reason |
|---|---|---|---|---|---|
| `tests/golden/pair-01` | held | held | **moved** | held | Fully opaque; every fourth residual byte was the constant 255, now 0 everywhere on this pair. |
| `tests/golden/sequence-01` (11 frames) | held (11/11) | held (11/11) | **moved** (11/11) | held (11/11) | Same reason, on every frame including the one recoloured frame (index 4/frame 5), whose colour-channel residual bytes are unaffected. |
| `tests/golden/formats/gif` | held | held | **moved** | held | Same reason. |
| `tests/golden/formats/apng` | held | held | **moved** | held | Same reason. |
| `tests/golden/formats/webp-anim` | held | held | **moved** | held | Same reason. |
| `tests/golden/alpha-01` | held (`b51b7852...` both) | held (`7f345cf6...` both) | **moved** (`b7997c60...` -> `cd844488...`) | **moved** (`1cc103f3...` "identical" -> `b88d0949...` "Removed ...") | This fixture is not fully opaque, by design: it is gap G-01-1's own test case. Its verdict is *supposed* to move, from `identical` to a `Removed` region; a verdict that had *not* moved here would mean the fix did not work. |

Every fixture behaved exactly as the plan's own prediction table said it would, except `alpha-01` itself, whose verdict moving is the entire point of this plan and not a finding.

## Guard-failure drill (unedited guards, re-run after this plan's changes)

```
drill ok: transcendental drill: the guard went red and named window.rs
determinism-drill: planner drill: also running cross-arch-hash.sh, as evidence only
cross-arch-hash: aarch64-apple-darwin and x86_64-apple-darwin agree on all four digests
decode-base 5a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc
decode-candidate 99bf5b9508b222f45fa5badf610e21cdb464401b26bc30dda345780dc7d2f59f
residual dd3587b3eff6dccf2718a79643ba4863414a8eb92c321e9926c8911086c8415b
verdict 33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823
determinism-drill: planner drill: cross-arch-hash.sh exited 0 (informational)
drill ok: planner drill: the static FFT-planner guard went red and named window.rs
drill ok: GPU drill: the dependency guard went red and named glow

determinism-drill: 3 of 3 drills behaved as expected
```

No file under `crates/chrys-core/tests/determinism.rs` was edited by this plan. This holds because every arithmetic operation this plan adds is exact integer arithmetic on `u8` and `i32` values (`abs_diff`, a plain multiply, a plain add, one arithmetic shift): the transcendental guard, the fused multiply-add audit, the feature guard, the dependency guard and the planner guard all keep the meaning they had before this plan, and `sh scripts/determinism-drill.sh` reproduces the same three-of-three result it did at the end of plan 01-08.

`sh scripts/cross-arch-hash.sh` independently exits 0, confirming `aarch64-apple-darwin` and `x86_64-apple-darwin` agree on all four digests of `tests/golden/pair-01`, including the new `residual` value.

## Measured defect, before and after (Task 1)

Before the engine change (fixture `tests/golden/alpha-01` committed, engine untouched, commit `394063d`):

```
$ chrys compare tests/golden/alpha-01/base.png tests/golden/alpha-01/candidate.png
identical
$ echo $?
0
```

480 pixels' alpha bytes differ between the two files; every red, green and blue byte is identical (measured, not read from source).

After the engine change (this plan's final commit):

```
$ chrys compare tests/golden/alpha-01/base.png tests/golden/alpha-01/candidate.png
Removed region at x=48, y=48, width=24, height=20
$ echo $?
1
```

## Issues Encountered

See "Deviations from Plan" above. Both issues were found and resolved within this plan's own scope; neither required an architectural decision or a change to a determinism guard.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Gap G-01-1 is closed: a pair that differs only in alpha is reported changed, named by the correct kind, with the correct bounding box, through the real binary.
- `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets --release` (no warnings), and `cargo fmt --check` all pass on the final commit.
- `crates/chrys-core/tests/determinism.rs` (5 guards) passes unedited; `scripts/determinism-drill.sh` still reports 3 of 3 drills behaved as expected.
- The six-fixture CI digest matrix now includes `tests/golden/alpha-01`; the six-label runner matrix, the `agree` job and the `guards` job are unchanged.
- Open, unchanged from plan 01-08: the six-runner CI matrix has not yet run on a real remote, because this repository has none yet.
- No blockers.

## Self-Check: PASSED

All files below were verified present on disk, and all commit hashes verified present in `git log --oneline` on this branch, before this line was written:
- `tests/golden/alpha-01/base.png`, `candidate.png` — FOUND
- `crates/chrys-cli/tests/alpha.rs` — FOUND
- `crates/chrys-core/src/classify/kind.rs` (rewritten absence predicate) — FOUND
- `.github/workflows/determinism.yml` (alpha pair in digest job) — FOUND
- Commits `394063d`, `b175906`, `5ad54b2`, `bf3583c`, `956513f`, `5e0b609`, `475189e`, `6afd34f`, `c788d5f` — FOUND in `git log --oneline`

---
*Phase: 01-raster-engine-and-determinism-proof*
*Completed: 2026-09-07*
