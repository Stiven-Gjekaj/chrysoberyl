---
phase: 01-raster-engine-and-determinism-proof
plan: 07
subsystem: core
tags: [rust, connected-components, antialiasing, lab-colour, palette, imageproc, determinism]

# Dependency graph
requires:
  - phase: 01-raster-engine-and-determinism-proof
    provides: "chrys_core::register::block_match: BlockOffset, ResidualField (01-06)"
provides:
  - "chrys_core::classify::label: label_regions, LabelledRegion, RESIDUAL_THRESHOLD"
  - "chrys_core::classify::antialias: is_antialiasing, suppress_antialiasing"
  - "chrys_core::classify::colour: colour_delta, routed through palette's Lab colour space with the libm feature"
  - "chrys_core::classify::kind: classify_kind, the five-step ordered decision list naming Added, Removed, Resized, Moved and Recoloured"
  - "compare's Verdict::Changed regions now carry a real kind, a real offset when moved, and a real Lab colour delta when recoloured, replacing plan 01-06's single-region, single-kind placeholder"
affects: [01-08]

# Actuals (#2632)
actuals:
  tokens: 12884
  tasks: 3
  commits: 3
plan_head_before: c6ae5ee0d757d4d2e7c5b10c191e199523c0a348

# Tech tracking
tech-stack:
  added: ["imageproc 0.27.0 (region_labelling::connected_components)", "palette 0.7.7 (Lab, EuclideanDistance, libm feature)"]
  patterns:
    - "label_regions sorts its output by bounding box position, not by imageproc's own internal label order, so a version bump in the labelling crate cannot reorder a verdict"
    - "A histogram that decides a tie (frame_background's modal-colour count, or a future majority vote) is built over a BTreeMap, never a HashMap, because a HashMap's iteration order is not fixed across platforms and this project's whole premise is a byte-identical verdict on every platform"
    - "is_antialiasing reads only the baseline's own brightness structure to decide whether an edge exists at all; a region with no edge in the baseline (a flat area that is later recoloured wholesale) can never trip the rule, which is exactly what keeps a real recolour from being suppressed as antialiasing"
    - "classify_kind's five rules are ordered and each guarded by a named, documented constant (BACKGROUND_FRACTION, RESIZE_FRACTION); a person can read the list top to bottom and predict the answer for any region"

key-files:
  created:
    - crates/chrys-core/src/classify/mod.rs
    - crates/chrys-core/src/classify/label.rs
    - crates/chrys-core/src/classify/antialias.rs
    - crates/chrys-core/src/classify/colour.rs
    - crates/chrys-core/src/classify/kind.rs
  modified:
    - crates/chrys-core/Cargo.toml
    - crates/chrys-core/src/lib.rs
    - crates/chrys-core/src/verdict.rs
    - crates/chrys-core/tests/classify.rs

key-decisions:
  - "The residual threshold (RESIDUAL_THRESHOLD = 4), the background fraction (BACKGROUND_FRACTION = 0.9), the resize fraction (RESIZE_FRACTION = 0.2) and the antialiasing sibling threshold (ANTIALIAS_SIBLING_THRESHOLD = 2, pixelmatch's own 'more than two') are each a named constant with a doc comment explaining what it decides and why; see 'Constants recorded' below for the full list with reasons."
  - "classify_kind computes the frame's background as the modal RGBA colour over the whole baseline, once, and reuses that single value to test both frames' pixels inside a region's bounding box, rather than computing a second, independent modal colour for the candidate; this makes 'background in base but not candidate' (Added) and its reverse (Removed) a single, consistent comparison."
  - "A region's mean colour, for the Recoloured fall-through, is computed over its bounding box, not its labelled pixel mask: LabelledRegion carries a bounding box and a pixel count, not the mask itself, so the bounding box is the closest data classify_kind has to 'the region.' This is a documented approximation, not a silent one."
  - "The Euclidean (CIE76-style) Lab distance ships now; CIEDE2000 is the named future upgrade once the six-runner determinism matrix is green a second time on this module, per 01-RESEARCH.md's own Alternatives Considered row."

patterns-established:
  - "A crate that must gain no format-crate dependency of its own can still consume one indirectly through a re-export: imageproc re-exports the image crate as imageproc::image, so classify/label.rs builds a GrayImage without chrys-core ever naming image in its own Cargo.toml."
  - "A antialiasing-suppression test built from an unbroken, whole-frame diagonal (value as a pure function of x - y) rather than a diagonal clipped into an interior box avoids an artificial seam at the box's own edge that would otherwise starve the rule's 'many siblings' check of the neighbourhood width it needs near the diagonal's own tips."

requirements-completed: [CORE-01, CORE-03, CORE-04, CORE-05]

coverage:
  - id: D1
    description: "Changed pixels are grouped into labelled, eight-way-connected regions, each with the smallest bounding box holding its pixels, sorted by position rather than by the labelling library's own internal order."
    requirement: "CORE-04"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#label_regions_on_two_separated_blobs_returns_two_regions"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#label_regions_on_blobs_touching_only_at_a_corner_returns_one_region"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#each_bounding_box_is_the_smallest_rectangle_holding_its_region"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#the_region_list_is_sorted_by_bounding_box_position_top_to_bottom_then_left_to_right"
        status: pass
    human_judgment: false
  - id: D2
    description: "A difference that is only antialiasing is not reported, using pixelmatch's own heuristic; a real recoloured block survives suppression because the baseline has no edge there to mistake for a gradient."
    requirement: "CORE-05"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#is_antialiasing_returns_true_for_a_pixel_on_a_smooth_ramp_both_frames_share"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#suppress_antialiasing_on_a_diagonal_edge_only_difference_yields_no_region"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#suppress_antialiasing_keeps_a_real_recoloured_blocks_region"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#is_antialiasing_returns_false_at_the_centre_of_a_solid_block_that_changed_colour"
        status: pass
    human_judgment: false
  - id: D3
    description: "A colour change reports a difference value computed through palette's Lab colour space (the libm feature, not std), plus both RGBA colours, never a value alone."
    requirement: "CORE-03"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#colour_delta_on_two_identical_colours_returns_a_zero_difference_and_both_colours_unchanged"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#colour_delta_on_two_different_colours_returns_the_same_number_on_two_runs"
        status: pass
      - kind: other
        ref: "grep -q 'EuclideanDistance' crates/chrys-core/src/classify/colour.rs"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every region is named by kind through an ordered, reviewable five-rule decision list: Added, Removed, Resized, Moved (with its offset in pixels) and the Recoloured fall-through (always carrying a colour delta)."
    requirement: "CORE-01"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#classify_kind_on_a_region_background_in_the_base_and_content_in_the_candidate_returns_added"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#classify_kind_on_a_region_background_in_the_candidate_and_content_in_the_base_returns_removed"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#classify_kind_on_a_region_whose_majority_block_offset_is_non_zero_returns_moved_with_the_offset"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#classify_kind_on_a_region_whose_content_count_changed_by_more_than_the_fraction_returns_resized"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs#classify_kind_falls_through_to_recoloured_and_always_carries_a_colour_delta"
        status: pass
    human_judgment: false
  - id: D5
    description: "The printed verdict for the committed pair names a kind, a bounding box, a colour difference and both colours; nothing in the output is a bare pixel count; the wave-4 determinism grep gates and all 16 calibration pairs still hold."
    verification:
      - kind: other
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png | grep -qiE 'recoloured.*(#|rgba|[0-9]+, *[0-9]+, *[0-9]+)'"
        status: pass
      - kind: integration
        ref: "crates/chrys-core/tests/refusal.rs (all 6 tests, covering all 16 calibration pairs)"
        status: pass
      - kind: other
        ref: "grep -q 'FftPlannerScalar' crates/chrys-core/src/register/*.rs && ! grep -rq 'FftPlanner::new' crates/chrys-core/src/"
        status: pass
    human_judgment: false

duration: 53min
completed: 2026-09-06
status: complete
---

# Phase 1 Plan 7: Connected-component labelling, antialiasing suppression, and named regions Summary

**`chrys_core::classify` turns the residual field into a verdict a person can read: eight-way-connected regions with bounding boxes, an antialiasing filter that reads only the baseline's own edges, a Lab colour distance routed through `palette`'s pure-Rust `libm` feature, and a five-rule ordered decision list that names every region `Added`, `Removed`, `Resized`, `Moved` or `Recoloured`.**

## Performance

- **Duration:** 53 min (measured between this plan's final commit and 01-06's final commit)
- **Completed:** 2026-09-06T22:09:48+02:00
- **Tasks:** 3
- **Files modified:** 9 (4 created, 5 modified, Cargo.lock excluded)

## Accomplishments

- `crates/chrys-core/src/classify/label.rs`: `LabelledRegion` and `label_regions(residual) -> Vec<LabelledRegion>`. Thresholds the residual against the named `RESIDUAL_THRESHOLD`, hands a `GrayImage` (via `imageproc::image`, so `chrys-core` never names the `image` crate directly) to `imageproc::region_labelling::connected_components` with `Connectivity::Eight`, and returns the region list sorted by bounding box position rather than the library's own label order.
- `crates/chrys-core/src/classify/antialias.rs`: `is_antialiasing(base, candidate, x, y)` and `suppress_antialiasing(residual, base, candidate)`, adapting pixelmatch's own heuristic (not Yee 2004, which has no explicit pixel rule). The module's own doc comment records, in four sentences, pixelmatch's provenance and its two accepted blind spots: a thin, single-pixel feature (a hairline) can fail the neighbour-count test even when genuinely antialiased, and a real, small structural change that happens to produce a smooth local gradient in both frames will be suppressed. Both are accepted for this phase; a future perceptual metric is the named fix, not a wider threshold.
- `crates/chrys-core/src/classify/colour.rs`: `colour_delta(base, candidate) -> ColourDelta`, converting each RGBA colour to `palette::Srgb<f32>` then to `Lab` and taking the `EuclideanDistance`. Documents why the simpler CIE76-style formula ships now rather than CIEDE2000: CIEDE2000 needs an arc tangent, a sine, a cosine and a power function on top of the cube root every Lab conversion already needs, while the Euclidean distance needs only a square root, which Rust's own precision documentation guarantees will not change.
- `crates/chrys-core/src/classify/kind.rs`: `classify_kind(region, base, candidate, blocks) -> (ChangeKind, Option<(i32, i32)>, Option<ColourDelta>)`, the five-step ordered decision list (`Added`, `Removed`, `Resized`, `Moved`, `Recoloured`), with `BACKGROUND_FRACTION` and `RESIZE_FRACTION` as named, documented constants. The frame's own background colour is computed once, as the modal RGBA value over the baseline via a `BTreeMap` histogram (never a `HashMap`, to keep a tie deterministic across platforms).
- `crates/chrys-core/src/lib.rs`: `compare` now runs `suppress_antialiasing` before `label_regions`, and fills every `Region`'s kind, offset and colour delta from `classify_kind` and the block match's own `field.blocks`, replacing the single-region, always-`Recoloured` placeholder plan 01-06 left in place.
- `crates/chrys-core/src/verdict.rs`: `Region`'s `Display` now prints a `Moved` region's offset alongside its bounding box; `ColourDelta`'s own doc comment records that `delta_e` is now the real Lab distance, not a stand-in.
- `crates/chrys-core/tests/classify.rs`: 21 integration tests across all three tasks' behaviour lists, built entirely from residual fields and frame pairs constructed inline, never from a fixture file.

## Task Commits

1. **Task 1: Group changed pixels into labelled regions with bounding boxes** - `d148ee5` (code + test: `chrys-core/src/classify/{mod,label}.rs`, `chrys-core/src/lib.rs`, `chrys-core/Cargo.toml`, `chrys-core/tests/classify.rs`)
2. **Task 2: Drop a difference that is only antialiasing** - `6c153c4` (code + test: `chrys-core/src/classify/{mod,antialias}.rs`, `chrys-core/src/lib.rs`, `chrys-core/tests/classify.rs`)
3. **Task 3: Name every region by kind, and report a colour change with both colours** - `5180864` (code + test: `chrys-core/src/classify/{mod,colour,kind}.rs`, `chrys-core/src/{lib,verdict}.rs`, `chrys-core/Cargo.toml`, `chrys-core/tests/classify.rs`)

**Plan metadata:** commit pending (this SUMMARY — orchestrator owns STATE.md/ROADMAP.md writes per the objective given to this executor)

## Files Created/Modified

- `crates/chrys-core/src/classify/mod.rs` - declares and re-exports `label`, `antialias`, `colour`, `kind`
- `crates/chrys-core/src/classify/label.rs` - `LabelledRegion`, `label_regions`, `RESIDUAL_THRESHOLD`
- `crates/chrys-core/src/classify/antialias.rs` - `is_antialiasing`, `suppress_antialiasing`, `ANTIALIAS_SIBLING_THRESHOLD`
- `crates/chrys-core/src/classify/colour.rs` - `colour_delta`
- `crates/chrys-core/src/classify/kind.rs` - `classify_kind`, `BACKGROUND_FRACTION`, `RESIZE_FRACTION`
- `crates/chrys-core/src/lib.rs` - `compare` wires suppression, labelling and classification together; the pre-existing recoloured-rectangle test fixture was corrected (see Deviations)
- `crates/chrys-core/src/verdict.rs` - `Region`'s `Display` prints a moved offset; `ColourDelta`'s doc comment updated
- `crates/chrys-core/Cargo.toml` - adds `imageproc` (Task 1) and `palette` (Task 3), each with a comment recording the package-legitimacy disposition
- `crates/chrys-core/tests/classify.rs` - 21 integration tests

## Decisions Made

See `key-decisions` in the frontmatter for the full list with reasons. In short: the frame's background is one modal colour, computed once over the baseline and reused for both frames' region tests; a region's mean colour for the `Recoloured` case is taken over its bounding box, since `LabelledRegion` does not carry a pixel mask; every histogram this plan adds that could face a tie (`frame_background`'s colour histogram, `majority_block_offset`'s vote) is built over a `BTreeMap`, not a `HashMap`, so a tie resolves identically on every platform; and the CIE76-style Euclidean Lab distance ships now, with CIEDE2000 named as the next upgrade once the six-runner matrix is green a second time.

### Constants recorded

| Constant | Value | Reason |
|---|---|---|
| `RESIDUAL_THRESHOLD` | `4` | The smallest residual byte value this engine treats as a change; a value at or below it is background noise, not a region. |
| `ANTIALIAS_SIBLING_THRESHOLD` | `2` | pixelmatch's own "more than two" threshold: more than this many zero-delta neighbours means a solid-colour area, not an edge; more than this many identical-value neighbours at an edge's own extreme means that extreme looks like part of a smooth gradient. |
| `BACKGROUND_FRACTION` | `0.9` | The fraction of a region's own bounding box that must equal the frame's modal colour for that region to count as background in that frame. |
| `RESIZE_FRACTION` | `0.2` | The fraction by which a region's non-background pixel count may differ between frames before the rule calls it `Resized` rather than `Recoloured`. |

### What the antialiasing rule cannot catch

Recorded in `antialias.rs`'s own module doc comment, and repeated here per this plan's own instruction: this is pixelmatch's own heuristic, not Yee 2004 (which has no explicit per-pixel antialiasing rule at all, handling it implicitly through a Laplacian pyramid and a contrast sensitivity function). Two blind spots are accepted for this phase. First, a thin, single-pixel-wide feature such as a hairline or a small dot can fail the neighbour-count test even when it is genuinely just antialiased, because its own neighbourhood has too few pixels sharing its exact value — this is the "too aggressive" direction the plan's own risk framing says is lower-priority, since a false positive there is visible and a person corrects it. Second, and weighted more heavily per the plan's own guidance: a real, small structural change that happens to produce a smooth local gradient in both frames will be classified as antialiasing and suppressed — the permissive direction this plan tested directly (`suppress_antialiasing_keeps_a_real_recoloured_blocks_region`, and the diagonal-edge test's own construction, which relies on the baseline having no edge at all around a genuine recolour for the rule to correctly decline suppression there). Both limits are known and accepted; a future perceptual metric (closer to Yee's own approach) is the named fix, not a wider threshold.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The pre-existing recoloured-rectangle test's fixture was ambiguous under the new background rule**
- **Found during:** Task 3, first `cargo test -p chrys-core --lib` run after wiring `classify_kind` into `compare`
- **Issue:** `a_recoloured_rectangle_returns_one_recoloured_region` (inherited from plan 01-06) painted its base rectangle in the frame's own dominant background colour (`[240, 240, 240, 255]`) before recolouring it to green in the candidate. Under `classify_kind`'s now-implemented rule ("the region is background in the baseline and not in the candidate, so it is `Added`"), this fixture is, by the letter of the rule, an `Added` scenario, not a `Recoloured` one: the base pixels there genuinely matched the frame's modal colour. The test asserted `ChangeKind::Recoloured`, which was true only because Task 1's placeholder always reported `Recoloured` and had not yet been replaced by real classification logic.
- **Fix:** Repainted the base's inner rectangle to a distinct mid-grey (`[180, 180, 180, 255]`), not equal to the frame's background, the anchor rectangles, or the moat, so the fixture is unambiguously "existing content that changed colour" rather than "content added on background." Added a comment at the paint call explaining why this specific colour choice now matters.
- **Files modified:** `crates/chrys-core/src/lib.rs`
- **Verification:** `cargo test -p chrys-core --lib tests::a_recoloured_rectangle_returns_one_recoloured_region` passes with the same bounding box the test has always asserted, now with `delta.base == [180, 180, 180, 255]`; `cargo test --workspace` reports 132 tests passing, 0 failing.
- **Committed in:** `5180864` (found and fixed before Task 3's own commit)

**2. [Rule 3 - Blocking issue] A rejected-crate name appeared inside this plan's own dependency comment, tripping its own grep gate**
- **Found during:** Task 3, running the plan's own verify command for the rejected-crate count
- **Issue:** The doc comment added beside `palette.workspace = true` in `crates/chrys-core/Cargo.toml` named `empfindung` and `delta_e` directly, to explain why they were rejected. `test "$(grep -c 'empfindung\|delta_e' crates/chrys-core/Cargo.toml)" = "0"` therefore failed, even though neither crate was declared as a dependency: the gate greps the whole file, including comments.
- **Fix:** Reworded the comment to describe the rejection without naming either crate, pointing to `01-RESEARCH.md`'s own Alternatives Considered table for the names, and to explain that naming them in the manifest would defeat the gate's own purpose.
- **Files modified:** `crates/chrys-core/Cargo.toml`
- **Verification:** `test "$(grep -c 'empfindung\|delta_e' crates/chrys-core/Cargo.toml)" = "0"` passes.
- **Committed in:** `5180864` (found and fixed before Task 3's own commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 1 bug in a pre-existing test fixture, 1 Rule 3 blocking self-inflicted grep-gate failure)
**Impact on plan:** No scope creep. Both fixes are corrections within files this plan already owns, made necessary by wiring the real classification logic into `compare` for the first time.

## Issues Encountered

The diagonal-edge antialiasing test fixture (`suppress_antialiasing_on_a_diagonal_edge_only_difference_yields_no_region`) initially used a bg/mid/fg pattern clipped to an interior box, with a uniform forced background outside it. Two of the ten diagonal pixels near the box's own corners failed to classify as antialiasing, because the box's artificial edge starved the rule's "many siblings" check of the neighbourhood width it needs right at the diagonal's own tips — a real instance of this rule's own documented "thin feature" blind spot, self-inflicted by the test's own geometry rather than the diagonal itself. Rebuilding the pattern as a single, unbroken function of `x - y` across the whole frame, differing between base and candidate only at transition pixels comfortably clear of every frame edge, removed the artificial seam and all ten pixels now classify correctly. This was resolved before Task 2's own commit; no code in `antialias.rs` itself needed to change, only the test's own fixture geometry.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `chrys_core::classify` (`label_regions`, `is_antialiasing`, `suppress_antialiasing`, `colour_delta`, `classify_kind`) is the complete, stable classification surface: every requirement CORE-01, CORE-03, CORE-04 and CORE-05 names is implemented and tested. `compare` now produces a genuinely differentiated verdict — a person reads a kind, a bounding box, an offset when moved, and a colour difference with both colours when recoloured, never a bare pixel count.
- The four named constants (`RESIDUAL_THRESHOLD`, `ANTIALIAS_SIBLING_THRESHOLD`, `BACKGROUND_FRACTION`, `RESIZE_FRACTION`) are now committed determinism and behaviour inputs; changing any of them changes which regions the engine reports and how it names them, and is a `reversibility="costly"` decision matching this plan's own doc comments.
- The committed pair's digests, via `cargo run -q -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png --hash-only`: `decode-base 5a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc` and `decode-candidate 99bf5b9508b222f45fa5badf610e21cdb464401b26bc30dda345780dc7d2f59f` are unchanged, matching the committed `tests/golden/pair-01/expected-decode.sha256` exactly. `residual f812da6aa3625e87bba393bb9742f6f24da5ca2a69d10434f364bab3b2fcaea0` is also unchanged from plan 01-06, since neither the labelling, antialiasing nor colour stage touches the residual bytes themselves. `verdict 33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823` has changed from plan 01-06's `083c2c50a92ffaa47776c6a1037b4fb310962957328ff4dcb767c57b96032b0f`, exactly as this plan's own environment note expects: the verdict now says something different, a real `Recoloured` kind with a Lab colour delta instead of the earlier RGB-Euclidean stand-in on a single always-`Recoloured` placeholder region. Neither the residual nor the verdict digest is committed to a file; the six-runner CI matrix is what proves them identical across every runner, matching the pattern plan 01-06 already established.
- All 16 calibration pairs in `tests/golden/refuse-01/` still classify correctly (8 refused, 8 registered), and the wave-4 determinism grep gates (`FftPlannerScalar` present, `FftPlanner::new` absent, no `std` transcendental beyond `sqrt` in `register/`) still hold.
- No blockers.

## Self-Check: PASSED

All files below were verified present on disk and all three commit hashes verified present in `git log --oneline` on this branch before this line was written:
- `crates/chrys-core/src/classify/mod.rs`, `label.rs`, `antialias.rs`, `colour.rs`, `kind.rs` — FOUND
- `crates/chrys-core/tests/classify.rs` — FOUND
- Commits `d148ee5`, `6c153c4`, `5180864` — FOUND in `git log --oneline`

---
*Phase: 01-raster-engine-and-determinism-proof*
*Completed: 2026-09-06*
