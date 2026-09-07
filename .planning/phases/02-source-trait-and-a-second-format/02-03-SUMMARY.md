---
phase: 02-source-trait-and-a-second-format
plan: 03
subsystem: source-adapter
tags: [rust, image, gif, apng, webp, animation, riff-container, determinism]

# Dependency graph
requires:
  - phase: 02-source-trait-and-a-second-format
    provides: "compare_sequence, the one comparison pipeline; the Source trait and Frame shape unchanged since phase 1 (02-01, 02-02)"
provides:
  - "chrys-source-animation, the GIF/APNG/lossless-animated-WebP Source adapter"
  - "AnimationSource, AnimationLimits, AnimationError"
  - "sniff::is_animation, the one content-sniffed format decision the CLI asks"
  - "the committed tests/golden/formats/{gif,apng,webp-anim} fixture pairs, five frames each, one index changed"
  - "a hand-assembled RIFF/VP8X/ANIM/ANMF animated-WebP container generator, gated on its own in-memory round-trip decode"
  - "chrys compare dispatching a sniffed animation file to AnimationSource, with the still-raster path unchanged"
affects: [02-04, 02-05]

# Actuals (#2632)
actuals:
  tokens: 9750
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added:
    - "image's gif feature, newly enabled on chrys-source-animation only (png and webp features were already enabled elsewhere in the workspace)"
    - "png = \"=0.18.1\" (workspace dependency, taken as a dev-dependency of chrys-source-animation only, for APNG fixture encode)"
  patterns:
    - "Reshape, never recomposite: the adapter calls image::AnimationDecoder::into_frames() and copies the already-composited full-canvas RGBA8 buffer; it writes no disposal, blend or compositing logic of its own for any of the three formats."
    - "One sniff function, every format branch inside it: sniff::is_animation is the CLI's one question: content-guessed (never extension-guessed), returning true for a GIF, an is_apng() PNG, or a has_animation() WebP, and false for everything else."
    - "A hand-rolled fixture generator earns trust by measurement: the animated-WebP container assembler round-trips its own output through WebPDecoder in memory, before a single byte reaches disk, and aborts naming the frame index on any disagreement."

key-files:
  created:
    - crates/chrys-source-animation/Cargo.toml
    - crates/chrys-source-animation/src/lib.rs
    - crates/chrys-source-animation/src/sniff.rs
    - crates/chrys-source-animation/examples/make-fixtures.rs
    - crates/chrys-source-animation/tests/animation.rs
    - tests/golden/formats/gif/base.gif, candidate.gif
    - tests/golden/formats/apng/base.png, candidate.png
    - tests/golden/formats/webp-anim/base.webp, candidate.webp
  modified:
    - Cargo.toml
    - crates/chrys-cli/Cargo.toml
    - crates/chrys-cli/src/main.rs

key-decisions:
  - "AnimationError's Decode variant always carries a frame index; a decoder-construction-time failure (before any frame is pulled) reports index 0 rather than adding a second, index-less decode variant, keeping the error enum to exactly the five cases the plan named."
  - "The ANMF flags byte sets the 'do not blend' bit rather than 'use alpha blending'. The first assembled container failed its own round-trip gate on frame 0 because image-webp's alpha-blend compositing path uses an approximate fixed-point formula that is not bit-exact even at full opacity; 'do not blend' instead takes the exact full-canvas overwrite path, which is correct here since every frame already covers the whole canvas."
  - "The round-trip check runs in memory, against the assembled byte buffer, before std::fs::write is ever called, rather than reading the file back from disk after writing it. This is a stronger reading of the plan's own stated goal ('no wrong container field ever reaches tests/golden/') than 'write then check' would have been."
  - "chrys-source-raster is a dev-dependency only of chrys-source-animation, used solely by a unit test asserting AnimationLimits' width/height/alloc numbers equal DecodeLimits' own; the crate's production dependency graph never references it."

patterns-established:
  - "A new input family's Source adapter can enable a new image feature flag (gif) and a new dev-only fixture-generation dependency (png) without either one reaching chrys-core's own dependency graph, proving SRC-08's claim a second time over a harder case than the sequence adapter (02-01) was."

requirements-completed: [SRC-03, SRC-08]

coverage:
  - id: D1
    description: "A GIF pair compares frame by frame through the CLI over a committed five-frame fixture, with exactly one index (frame 3) reporting a change."
    requirement: SRC-03
    verification:
      - kind: integration
        ref: "crates/chrys-source-animation/tests/animation.rs#gif"
        status: pass
      - kind: integration
        ref: "crates/chrys-source-animation/tests/animation.rs#a_frame_count_limit_below_the_fixture_size_refuses_and_names_the_limit"
        status: pass
      - kind: other
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/formats/gif/base.gif tests/golden/formats/gif/candidate.gif (5 frame blocks, frame 3 changed, exit 1)"
        status: pass
    human_judgment: false
  - id: D2
    description: "An APNG pair compares frame by frame through the CLI over the same five frames the GIF fixture uses; is_animation is true for the APNG fixture and false for the still PNG phase 1 committed; the still-PNG pair still routes through the raster path unchanged."
    requirement: SRC-03
    verification:
      - kind: integration
        ref: "crates/chrys-source-animation/tests/animation.rs#apng"
        status: pass
      - kind: integration
        ref: "crates/chrys-source-animation/tests/animation.rs#is_animation_is_true_for_the_apng_fixture_and_false_for_a_still_png"
        status: pass
      - kind: integration
        ref: "crates/chrys-source-animation/tests/animation.rs#frame_zero_of_the_gif_and_apng_fixtures_hold_the_same_pixels"
        status: pass
      - kind: other
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/formats/png/base.png tests/golden/formats/png/candidate.png (no frame block in output, unchanged raster path)"
        status: pass
    human_judgment: false
  - id: D3
    description: "A lossless animated WebP pair compares frame by frame through the CLI. The hand-assembled container round-trips its own output through WebPDecoder before it is ever written to disk, and a deliberate one-byte corruption drill confirms the gate refuses a malformed container rather than tolerating one."
    requirement: SRC-03
    verification:
      - kind: integration
        ref: "crates/chrys-source-animation/tests/animation.rs#webp_anim"
        status: pass
      - kind: integration
        ref: "crates/chrys-source-animation/tests/animation.rs#is_animation_is_true_for_the_animated_webp_fixture_and_false_for_a_still_webp"
        status: pass
      - kind: integration
        ref: "crates/chrys-source-animation/tests/animation.rs#frame_zero_of_the_gif_and_animated_webp_fixtures_hold_the_same_pixels"
        status: pass
      - kind: other
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/formats/webp-anim/base.webp tests/golden/formats/webp-anim/candidate.webp (5 frame blocks, frame 3 changed, exit 1); manual drill: corrupting byte 24 of the assembled container before the round-trip check raised 'Format error decoding WebP: Frame outside image' and wrote no file"
        status: pass
    human_judgment: false
  - id: D4
    description: "This plan's whole commit range changes no file under crates/chrys-core/, and cargo tree -p chrys-core -e normal names no format or GPU crate."
    requirement: SRC-08
    verification:
      - kind: other
        ref: "git diff --name-only d81381b93fd147c33c1df59557c39222696ca668^..d1c1e51e54d5b82c0dd2aa04262f0114f24e324f -- crates/chrys-core/ (empty output)"
        status: pass
      - kind: other
        ref: "git rev-parse HEAD:crates/chrys-core == 63aad81ddee9939047b1436a33eed8f0896da409, checked after every commit in this plan"
        status: pass
      - kind: other
        ref: "cargo tree -p chrys-core -e normal (names chrys-source, libm, palette, rustfft, sha2, thiserror and their own transitive deps only)"
        status: pass
    human_judgment: false

duration: unmeasured (PLAN_START_TIME not captured at launch; commit span covers only the three task commits, not the reading, dependency research and debugging before and between them)
completed: 2026-09-07
status: complete
---

# Phase 2 Plan 03: Add the animation input family — GIF, APNG and lossless WebP — with no engine change, Summary

**A new `chrys-source-animation` crate compares GIF, APNG and lossless animated WebP pairs frame by frame through the existing CLI, adding zero lines to `crates/chrys-core/` across all three tasks, and the one piece of new unreviewed code — a hand-assembled WebP container — is proven correct by its own in-memory round-trip decode before a byte reaches disk.**

## Performance

- **Duration:** unmeasured (see frontmatter note)
- **Completed:** 2026-09-07
- **Tasks:** 3 / 3
- **Files modified:** 15 (3 modified manifests/CLI, 5 new source files in the animation crate, 7 new committed fixture files)

## Accomplishments

- Built `chrys-source-animation`: `AnimationSource` implements `Source` for GIF (`GifDecoder`), APNG (`PngDecoder::apng()`) and animated WebP (`WebPDecoder`), calling each format's `AnimationDecoder::into_frames()` and reshaping the already-composited full-canvas RGBA8 buffer into a `chrys_source::Frame`. No disposal, blend or compositing arithmetic exists anywhere in this crate — every format's own decoder does that internally, confirmed by reading each decoder's source before writing a line of adapter code.
- `AnimationLimits` mirrors `DecodeLimits`' width, height and allocation numbers (a unit test asserts the three stay equal to `chrys_source_raster::DecodeLimits::default()`, taken as a dev-dependency only) and adds a 512-frame and 134,217,728-total-pixel budget, both checked as `into_frames()` yields rather than after the whole file decodes — a limit that works on a file declaring a million frames, since the iterator is lazy.
- `sniff::is_animation` holds every format branch in one function: true for any GIF, true for a PNG only when `PngDecoder::is_apng()` says so, true for a WebP only when `WebPDecoder::has_animation()` says so, false otherwise. The format is guessed from content, never extension, the same rule the raster crate's `decode_guarded` already follows. Tests confirm both the positive and the negative case for APNG and for animated WebP, each against phase 1's own still-format fixture.
- Extended the CLI's `frames_for`: a directory still routes to `SequenceSource`; a file sniffed as an animation now routes to `AnimationSource`; anything else keeps the unchanged `RasterSource` path. A still PNG's comparison output is byte-for-byte what it was before this plan (verified: no `frame` line appears).
- Built the fixture generator in the shape of the raster crate's own: five 256x256 frames per side, integer rectangle formulas only, each frame's rectangles shifted from the last so the animation moves, with frame 3 recoloured on the candidate side only. The same five frames feed all three formats, so a cross-format pixel-identity test (frame 0 of GIF equals frame 0 of APNG equals frame 0 of animated WebP) is the cheapest available proof that three independent compositors agree.
- The animated-WebP container — `RIFF`/`VP8X`/`ANIM`/`ANMF`, assembled by hand around VP8L payloads extracted from `image`'s existing lossless single-image encoder, since no crate in this workspace can encode an animated WebP — is gated on its own round-trip decode, run in memory against the assembled byte buffer before `std::fs::write` is ever called. The first assembled container failed this gate on frame 0: the ANMF flags byte selected alpha blending, and `image-webp`'s blend compositor uses an approximate fixed-point formula that is not bit-exact even at full opacity. Setting the "do not blend" bit instead routes decode through the exact full-canvas overwrite path, which is correct here since every frame already covers the whole canvas. Every frame is lossless (VP8L) on purpose — phase 1 proved a lossless WebP decode path bit-exact on six runners and never exercised a lossy (VP8) animated frame, so this fixture does not extend that proof past what was measured.
- Performed a corruption drill (not committed as code, recorded here): flipping one byte of the assembled container's frame-descriptor region before the round-trip check made the generator abort with `Format error decoding WebP: Frame outside image`, and no file was written — confirming the gate refuses a malformed container rather than silently tolerating one a lenient reader might still accept.

## Task Commits

Each task was committed atomically:

1. **Task 1: Compare a GIF pair through a new animation adapter** - `d81381b`
2. **Task 2: Compare an APNG pair through the animation adapter** - `522575d`
3. **Task 3: Assemble an animated WebP fixture and prove it round-trips** - `d1c1e51`

**Plan metadata:** pending (this SUMMARY, STATE.md and ROADMAP.md updates are committed by the orchestrator, per this plan's execution instructions — this plan's own instructions additionally direct committing this SUMMARY.md directly)

first commit: d81381b93fd147c33c1df59557c39222696ca668
last commit: d1c1e51e54d5b82c0dd2aa04262f0114f24e324f

Verbatim output of `git diff --name-only d81381b93fd147c33c1df59557c39222696ca668^..d1c1e51e54d5b82c0dd2aa04262f0114f24e324f -- crates/chrys-core/`:

```
(empty)
```

Message the WebP generator printed when a container field was deliberately corrupted (one byte inside the first `ANMF` chunk's frame-descriptor region, flipped before the round-trip check ran):

```
thread 'main' panicked at crates/chrys-source-animation/examples/make-fixtures.rs:345:29:
round-trip decode .../tests/golden/formats/webp-anim/base.webp: Format error decoding WebP: Frame outside image
```

## Files Created/Modified

- `Cargo.toml` - added `png = "=0.18.1"` to the workspace dependency table (Task 2)
- `crates/chrys-source-animation/Cargo.toml` - new crate manifest: `chrys-source`, `image` (`gif`, `png`, `webp` features), `thiserror` as production deps; `chrys-source-raster` and `png` as dev-dependencies only
- `crates/chrys-source-animation/src/lib.rs` - `AnimationLimits`, `AnimationSource`, `AnimationError`, the `Source` impl dispatching to `GifDecoder`/`PngDecoder::apng()`/`WebPDecoder`, and `collect_frames`, the shared limit-checked frame-pulling loop
- `crates/chrys-source-animation/src/sniff.rs` - `is_animation`, the one content-sniffed format decision
- `crates/chrys-source-animation/examples/make-fixtures.rs` - the shared five-frame builder plus the GIF, APNG and hand-assembled animated-WebP writers, the RIFF chunk helpers, and the in-memory round-trip check
- `crates/chrys-source-animation/tests/animation.rs` - `gif`, `apng`, `webp_anim`, the frame-count limit test, two sniff tests and two cross-format pixel-identity tests
- `crates/chrys-cli/Cargo.toml` - added `chrys-source-animation` as a path dependency
- `crates/chrys-cli/src/main.rs` - `frames_for` now sniffs for an animation between the directory check and the raster fallback
- `tests/golden/formats/gif/{base,candidate}.gif` - committed GIF fixture
- `tests/golden/formats/apng/{base,candidate}.png` - committed APNG fixture
- `tests/golden/formats/webp-anim/{base,candidate}.webp` - committed lossless animated WebP fixture

## Decisions Made

- `AnimationError::Decode` always carries a frame index; a construction-time failure (before any frame is pulled) reports index 0, keeping the error enum to the five variants the plan named rather than adding a sixth, index-less case.
- The ANMF flags byte sets "do not blend," not "use alpha blending" — the latter is spec-legal but not bit-exact even at full opacity in `image-webp`'s own compositor, confirmed by this generator's own round-trip failure before the fix.
- The round-trip check runs against the in-memory byte buffer, before the file is written, not after reading it back from disk — a stronger reading of "no wrong container field ever reaches `tests/golden/`" than write-then-check would have been.
- `chrys-source-raster` is a dev-dependency only of the new crate, used solely by one unit test; it never appears under `[dependencies]`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Animated-WebP round-trip failed on frame 0 due to an incorrect ANMF blend flag**
- **Found during:** Task 3, first run of the fixture generator
- **Issue:** The ANMF flags byte was written as `0x00` (alpha-blend mode). `image-webp`'s alpha-blend compositor uses an approximate fixed-point formula (`do_alpha_blending`, using a truncated reciprocal scale) that is not bit-exact even when the source frame is fully opaque, so the round-trip check's own gate correctly refused the container: decoded frame 0 did not match the frame it was built from.
- **Fix:** Set the "do not blend" bit (`0b0000_0010`) in the ANMF flags byte, routing the decoder through its exact full-canvas-overwrite fast path instead of the approximate blend path. Confirmed correct: the round-trip check then passed for all five frames of both the base and candidate files.
- **Files modified:** `crates/chrys-source-animation/examples/make-fixtures.rs`
- **Verification:** `cargo run -p chrys-source-animation --example make-fixtures` completes with no panic; `cargo test -p chrys-source-animation webp_anim -- --exact` passes; the cross-format pixel-identity test (`frame_zero_of_the_gif_and_animated_webp_fixtures_hold_the_same_pixels`) passes.
- **Committed in:** `d1c1e51` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 bug, caught and fixed by the round-trip gate itself before any file reached disk)
**Impact on plan:** The deviation is exactly the failure mode Pitfall 3 (02-RESEARCH.md) predicted and the plan's own round-trip gate exists to catch. No scope creep; the fix is a one-byte flag correction with no change to the container's overall structure.

## Issues Encountered

None beyond the auto-fixed round-trip failure documented above, which the plan's own required gate caught as designed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `chrys-source-animation` is a complete, tested Source adapter for GIF, APNG and lossless animated WebP, wired into the CLI, with no file under `crates/chrys-core/` touched across any of this plan's three tasks — SRC-08's claim now holds over a second, harder input family (an adapter dispatching across three formats and a hand-rolled container) than the sequence adapter (02-01) exercised.
- The engine tree object id `63aad81ddee9939047b1436a33eed8f0896da409`, recorded by plan 02-02, is unchanged after this plan's last commit. Plan 02-04's cross-wave drill can check this commit range (`d81381b93fd147c33c1df59557c39222696ca668^..d1c1e51e54d5b82c0dd2aa04262f0114f24e324f`) directly.
- `cargo tree -p chrys-core -e normal` still names no format or GPU crate; the new `gif` feature and the new `png` dev-dependency both terminate inside `chrys-source-animation`'s own graph.
- The four phase-1 digests of `tests/golden/pair-01` and the six determinism guards in `crates/chrys-core/tests/determinism.rs` were verified passing after the final commit.
- The animated-WebP container assembler and its round-trip check are reusable groundwork: any later phase needing another hand-assembled RIFF-family fixture has a worked, gated example to follow.

## Self-Check: PASSED

All created files verified present on disk (`crates/chrys-source-animation/{Cargo.toml,src/lib.rs,src/sniff.rs,examples/make-fixtures.rs,tests/animation.rs}`, `tests/golden/formats/{gif,apng,webp-anim}/{base,candidate}.*`); all three task commit hashes (`d81381b`, `522575d`, `d1c1e51`) verified present in `git log --oneline`.

---
*Phase: 02-source-trait-and-a-second-format*
*Completed: 2026-09-07*
