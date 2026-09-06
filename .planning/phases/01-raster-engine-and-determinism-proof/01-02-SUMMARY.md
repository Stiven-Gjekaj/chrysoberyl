---
phase: 01-raster-engine-and-determinism-proof
plan: 02
subsystem: infra
tags: [rust, image, png, jpeg, webp, tiff, exif, decode-limits]

# Dependency graph
requires:
  - phase: 01-raster-engine-and-determinism-proof
    provides: "chrys-source-raster crate decoding PNG with guarded limits (01-01)"
provides:
  - "decode_guarded: the single guarded decode entry point for every raster format in chrys-source-raster"
  - "normalize_to_rgba8 and Orientation: EXIF orientation, straight alpha and bit-depth normalization to RGBA8"
  - "PNG, JPEG, WebP and TIFF decode support on the image dependency, default-features still false"
  - "tests/golden/formats/{png,jpeg,webp,tiff}: one committed pair per raster format, read from disk"
affects: [01-03, 01-04, 01-05, 01-06, 01-07, 01-08]

# Actuals (#2632)
actuals:
  tokens: 7743
  tasks: 2
  commits: 2
plan_head_before: 5b21ab820967099081de38af619a2765324c5d57

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "decode.rs is the only module in chrys-source-raster that opens an image::ImageReader or builds an image decoder; a grep gate in the plan's own verify proves lib.rs no longer does"
    - "The declared width and height are checked against DecodeLimits twice: once implicitly inside image::ImageReader::into_decoder, and once again explicitly via ImageDecoder::set_limits in decode.rs, before any pixel buffer is allocated"
    - "Orientation is read through ImageDecoder::orientation(), the decoder's own metadata accessor, and converted to this crate's own eight-variant Orientation enum; no format's byte layout is parsed by hand"
    - "normalize_to_rgba8 reuses image::DynamicImage::apply_orientation (an integer pixel move: rotate or flip, never a resample) instead of a hand-rolled transform"

key-files:
  created:
    - crates/chrys-source-raster/src/decode.rs
    - crates/chrys-source-raster/src/normalize.rs
    - crates/chrys-source-raster/tests/formats.rs
    - tests/golden/formats/png/base.png
    - tests/golden/formats/png/candidate.png
    - tests/golden/formats/jpeg/base.jpg
    - tests/golden/formats/jpeg/candidate.jpg
    - tests/golden/formats/webp/base.webp
    - tests/golden/formats/webp/candidate.webp
    - tests/golden/formats/tiff/base.tif
    - tests/golden/formats/tiff/candidate.tif
  modified:
    - crates/chrys-source-raster/src/lib.rs
    - crates/chrys-source-raster/Cargo.toml
    - crates/chrys-source-raster/examples/make-fixtures.rs
    - Cargo.lock

key-decisions:
  - "decode_guarded returns (image::DynamicImage, normalize::Orientation) instead of the DynamicImage-only signature the plan's action text sketched. The plan also requires Source::load to call decode_guarded then normalize_to_rgba8 separately, and normalize_to_rgba8 itself performs the orientation transform (its own behaviour tests exercise this directly). A DynamicImage-only return has nowhere to carry the orientation the decoder read to that second call, short of a second, unguarded file read purely to re-derive it. Returning the orientation alongside the image keeps decode.rs the single place that touches a decoder, at the cost of a small, documented signature deviation from the action text; no acceptance criterion pins the exact return type."
  - "The pinned image 0.25.10 exposes real Exif orientation for all four SRC-01 formats: PNG through ImageDecoder's default Exif-chunk-based implementation (PNG's own exif_metadata reads the eXIf chunk), and JPEG, WebP and TIFF each through their own orientation() override. No format defaulted to Orientation::Normal for lack of an accessor; from_exif_u16 defaults only on an out-of-range tag value (0, 9 and above), which is a data-quality fallback, not a capability gap."
  - "No [dev-dependencies] image entry was added for the fixture generator, though the plan's action text calls for one. In image 0.25.10 each of the four format features (png, jpeg, webp, tiff) already enables both its encoder and its decoder; there is no separate encode-only feature to add. Confirmed by reading the pinned crate's own source: PngEncoder, JpegEncoder, WebPEncoder and TiffEncoder all live behind the same feature flags already added to [dependencies]."
  - "The fixture generator wraps each buffer in image::DynamicImage before calling .save(), not the raw RgbaImage. Only DynamicImage::save runs the encoder-compatibility conversion; the JPEG encoder rejects straight RGBA8 with 'the encoder or decoder for Jpeg does not support the color type Rgba8' when saved as a bare ImageBuffer. This was caught by running the generator, not by reading the source alone."
  - "Task 1's 'a PNG header that declares dimensions above the limit' test uses a small, valid PNG decoded against a DecodeLimits with max_width/max_height set to 1, instead of hand-crafting a PNG with a large declared IHDR. Both exercise the identical code path (declared dimensions checked against the caller's limit before a pixel buffer is allocated); the chosen form needs no byte-level PNG surgery and builds its state entirely inside the test, per AGENTS.md."

patterns-established:
  - "Every new format feature this project enables on the image crate goes through the same single decode_guarded entry point; no per-format branch was added anywhere in this plan, in chrys-source-raster or elsewhere."

requirements-completed: [SRC-01]

coverage:
  - id: D1
    description: "A PNG, JPEG, WebP and TIFF pair each decode through RasterSource into one Frame with the same shape contract, and chrys compare reports a change on each."
    requirement: "SRC-01"
    verification:
      - kind: integration
        ref: "cargo test -p chrys-source-raster --test formats"
        status: pass
      - kind: e2e
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/formats/tiff/base.tif tests/golden/formats/tiff/candidate.tif; test $? -eq 1"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every decode call sets a memory limit before a pixel buffer is allocated, and a malformed or oversized file returns a typed error instead of exhausting memory."
    requirement: "SRC-01"
    verification:
      - kind: unit
        ref: "crates/chrys-source-raster/src/decode.rs#tests::decode_guarded_on_a_zero_byte_file_returns_a_decode_error"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-raster/src/decode.rs#tests::decode_guarded_on_a_truncated_png_returns_a_decode_error"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-raster/src/decode.rs#tests::decode_guarded_rejects_a_declared_size_above_the_configured_limit"
        status: pass
      - kind: other
        ref: "grep -q 'set_limits' crates/chrys-source-raster/src/decode.rs"
        status: pass
    human_judgment: false
  - id: D3
    description: "A rotated JPEG-style EXIF orientation decodes to the orientation a person sees, and two decodes of the same file are byte-identical."
    requirement: "SRC-01"
    verification:
      - kind: unit
        ref: "crates/chrys-source-raster/src/normalize.rs#tests::normalize_to_rgba8_with_exif_orientation_6_swaps_width_and_height"
        status: pass
      - kind: unit
        ref: "crates/chrys-source-raster/src/decode.rs#tests::decode_guarded_on_the_same_path_twice_returns_identical_pixels"
        status: pass
    human_judgment: false
  - id: D4
    description: "A file whose extension does not match its content still decodes, because the format is guessed from content."
    requirement: "SRC-01"
    verification:
      - kind: integration
        ref: "crates/chrys-source-raster/tests/formats.rs#a_file_decodes_by_content_even_with_a_mismatched_extension"
        status: pass
    human_judgment: false

duration: 19min
completed: 2026-09-06
status: complete
---

# Phase 1 Plan 2: Guard, normalize and widen the raster path to four formats Summary

**`decode_guarded` in `chrys-source-raster` is now the one place any raster file is opened, and PNG, JPEG, WebP and TIFF each have a committed pair, a decode test, and a working `chrys compare` run.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-09-06T18:05:01+02:00
- **Completed:** 2026-09-06T18:24:19+02:00
- **Tasks:** 2
- **Files modified:** 15 (11 created, 4 modified — see key-files)

## Accomplishments
- `crates/chrys-source-raster/src/decode.rs`: `decode_guarded`, the single guarded decode entry point. It opens the file, guesses the format from content, and checks the declared width and height against `DecodeLimits` twice — once inside `image::ImageReader::into_decoder`, once again explicitly via `ImageDecoder::set_limits` — before allocating a pixel buffer. This is the CVE-2023-29408 mitigation, now proved by a grep gate: `crates/chrys-source-raster/src/lib.rs` contains zero references to `image::ImageReader`.
- `crates/chrys-source-raster/src/normalize.rs`: an eight-variant `Orientation` enum with `from_exif_u16`, and `normalize_to_rgba8`, which applies the orientation as an integer pixel move (reusing `image::DynamicImage::apply_orientation`, never a hand-rolled transform) and then converts to 8-bit RGBA with straight alpha.
- `image` now decodes PNG, JPEG, WebP and TIFF (`default-features = false` unchanged), and `crates/chrys-source-raster/examples/make-fixtures.rs` writes a committed pair per format into `tests/golden/formats/{png,jpeg,webp,tiff}/`, from the same in-memory buffers that build the existing `pair-01` PNG pair (unchanged, confirmed byte-identical after regenerating).
- `crates/chrys-source-raster/tests/formats.rs`: one test per format (PNG, WebP and TIFF assert byte equality against a buffer rebuilt inside the test; JPEG asserts shape and repeat-decode stability only, since it is lossy) plus a test proving a renamed file still decodes by content.
- `chrys-core/Cargo.toml` is untouched: still only `chrys-source` and `thiserror`.

## Task Commits

1. **Task 1: Guard and normalize every decode** - `74ba7c4` (feat + test)
2. **Task 2: Decode a pair in each of the four raster formats** - `8eb655a` (feat + test)

**Plan metadata:** commit pending (this SUMMARY — orchestrator owns STATE.md/ROADMAP.md writes per the objective given to this executor)

## Files Created/Modified
- `crates/chrys-source-raster/src/decode.rs` - `decode_guarded`, the single guarded decode entry point
- `crates/chrys-source-raster/src/normalize.rs` - `Orientation`, `normalize_to_rgba8`
- `crates/chrys-source-raster/src/lib.rs` - `Source::load` now calls `decode_guarded` then `normalize_to_rgba8`; no longer opens `image::ImageReader` directly
- `crates/chrys-source-raster/Cargo.toml` - `image` features now `["png", "jpeg", "webp", "tiff"]`
- `crates/chrys-source-raster/examples/make-fixtures.rs` - writes `tests/golden/formats/` in addition to the unchanged `pair-01`
- `crates/chrys-source-raster/tests/formats.rs` - one decode test per format, plus the mismatched-extension test
- `tests/golden/formats/{png,jpeg,webp,tiff}/{base,candidate}.*` - the eight committed fixture files
- `Cargo.lock` - picked up the jpeg/webp/tiff decode+encode dependency tree

## Decisions Made
- `decode_guarded` returns `(image::DynamicImage, normalize::Orientation)` rather than the `DynamicImage`-only signature the plan's action text sketched, so the orientation the decoder read can reach `Source::load`'s second call to `normalize_to_rgba8` without a second, unguarded file read. See `key-decisions` in the frontmatter for the full reasoning; no acceptance criterion in the plan pins the exact return type, only that `decode_guarded` exists and calls `set_limits`.
- No format defaulted to `Orientation::Normal` for lack of an Exif accessor: PNG, JPEG, WebP and TIFF all expose real orientation reading in the pinned `image` 0.25.10, confirmed by reading each codec's `orientation()`/`exif_metadata()` implementation in the vendored crate source. `from_exif_u16` still defaults out-of-range tag values (0, 9+) to `Normal`, which is a data-quality fallback, not a missing-capability gap.
- No `[dev-dependencies]` entry for `image` was added in `Cargo.toml`, though the plan's action text calls for one: in `image` 0.25.10 each format feature already enables both its encoder and its decoder, so the existing `[dependencies]` features cover fixture generation too.
- The fixture generator saves through `image::DynamicImage`, not the raw `RgbaImage`, because only `DynamicImage::save` runs the encoder-compatibility conversion; the JPEG encoder otherwise rejects straight RGBA8 input outright.
- The "over-large declared dimension" test in `decode.rs` uses a small valid PNG against a `DecodeLimits` with `max_width`/`max_height` set to 1, rather than hand-crafting a PNG with a large declared header — both exercise the same rejection path, and this form needs no byte-level PNG construction.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `RgbaImage::save` cannot write a JPEG fixture**
- **Found during:** Task 2, first run of the fixture generator
- **Issue:** `base.save(&base_path)` on a raw `RgbaImage` panicked with "the encoder or decoder for Jpeg does not support the color type Rgba8" when writing the `.jpg` fixture. Only `DynamicImage::save`/`write_to` runs the automatic colour-type conversion (`make_compatible_img`) that drops the alpha channel for JPEG.
- **Fix:** Wrapped `base` and `candidate` in `image::DynamicImage::ImageRgba8(...)` before saving in `write_format_fixtures`.
- **Files modified:** `crates/chrys-source-raster/examples/make-fixtures.rs`
- **Verification:** `cargo run -p chrys-source-raster --example make-fixtures` writes all eight fixture files; `cargo test -p chrys-source-raster --test formats` passes, including the `jpeg` test.
- **Committed in:** `8eb655a` (Task 2 commit)

**2. [Rule 3 - Blocking] `cargo fmt` reformatted two new files after tests already passed**
- **Found during:** post-task self-check, before each commit
- **Issue:** `cargo fmt --check` reported a diff in `crates/chrys-source-raster/src/decode.rs` (a long `.write_to(...)` call) and `crates/chrys-source-raster/tests/formats.rs` (a long `.expect(...)` call), both exceeding the project's line width.
- **Fix:** Ran `cargo fmt` before each commit; the reformatted lines are part of the same task commit, not a separate one, since the code had not yet been committed.
- **Files modified:** the two files above
- **Verification:** `cargo fmt --check` reports no diff after each commit; all tests still pass.
- **Committed in:** `74ba7c4`, `8eb655a`

---

**Total deviations:** 2 auto-fixed (both Rule 3, blocking issues found while running the plan's own verification steps)
**Impact on plan:** No scope creep. Both fixes are necessary to complete the stated tasks; neither changes a public type, an error variant, or a file path named in the plan.

## Issues Encountered
None beyond the two auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `decode_guarded` and `normalize_to_rgba8` are the stable seam plan 01-03's digest guard and plan 01-04 onward's register stage can build on: every raster input, in every format this project names, now passes through the same two functions before becoming a `Frame`.
- `tests/golden/formats/` follows the same committed-generator-plus-output pattern as `tests/golden/pair-01/`, ready to extend if a later plan needs more format fixtures.
- `tests/golden/pair-01/{base,candidate}.png` are unchanged (confirmed via `git status` showing no modification after regenerating them), so plan 01-03's digest guard reads the same bytes it always would have.
- No blockers.

## Self-Check: PASSED

All files below were verified present on disk and all commit hashes verified present in `git log --oneline` on this branch before this line was written:
- `crates/chrys-source-raster/src/decode.rs`, `crates/chrys-source-raster/src/normalize.rs` — FOUND
- `crates/chrys-source-raster/tests/formats.rs` — FOUND
- `tests/golden/formats/png/base.png`, `tests/golden/formats/png/candidate.png` — FOUND
- `tests/golden/formats/jpeg/base.jpg`, `tests/golden/formats/jpeg/candidate.jpg` — FOUND
- `tests/golden/formats/webp/base.webp`, `tests/golden/formats/webp/candidate.webp` — FOUND
- `tests/golden/formats/tiff/base.tif`, `tests/golden/formats/tiff/candidate.tif` — FOUND
- Commits `74ba7c4`, `8eb655a` — FOUND in `git log --oneline`

---
*Phase: 01-raster-engine-and-determinism-proof*
*Completed: 2026-09-06*
