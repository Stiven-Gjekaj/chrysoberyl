---
phase: 04-svg-rasterization
plan: 01
subsystem: svg-adapter
tags: [rust, svg, resvg, tiny-skia, fontdb, alpha-conversion, dispatch]

# Dependency graph
requires:
  - phase: 04-svg-rasterization
    provides: "04-VALIDATION.md's approved contract and the engine-boundary anchor c97a6fb778c4b1373e5c4dc563481cc18e4c98c0, read at commit 199477d"
provides:
  - "crates/chrys-source-svg: SvgSource, SvgLimits, SvgError, looks_like_svg, and fonts::pinned_fontdb/pinned_family_name — a Source impl that rasterizes one SVG document into one straight-alpha Frame, with every glyph resolved from a repository-committed font"
  - "named_frames_for's fourth branch: a content-sniffed SVG file reaches SvgSource before the animation sniff runs"
  - "the workspace resvg pin: resvg = { version = \"=0.48.1\", default-features = false, features = [\"text\"] } in the root Cargo.toml, read by plan 04-02's guard"
  - "the unsafe-code guard's crate floor, raised to nine"
affects: []

# Actuals (#2632)
actuals:
  tokens: 15038
  tasks: 3
  commits: 4
  plan_head_before: 681982b379663d854fe6b0ec6fe0de57ae8bbf93

# Tech tracking
tech-stack:
  added:
    - "resvg 0.48.1 (default-features = false, features = [\"text\"]), pulling in usvg, tiny-skia and fontdb transitively, with system-fonts, svgz, memmap-fonts and raster-images all off"
  patterns:
    - "A one-font fontdb::Database is not enough to make a text fallback DET-05-safe: usvg's own default font selector always appends a generic 'serif' query after every named family fails to resolve, so the pinned font must also be registered as every one of fontdb's five generic families (set_serif_family, set_sans_serif_family, set_cursive_family, set_fantasy_family, set_monospace_family), not only set as usvg::Options::font_family. Options::font_family only covers the case where no font-family attribute is present at all."
    - "A file-size cap is checked twice, at two different moments: once from fs::metadata (a declared length) and once from a Read::take-bounded read (a length actually read), because a size read at one moment is not a guarantee about what a later read call will return."
    - "A content sniff reads a bounded prefix (1024 bytes) through File::open + Read::take, never the whole file, so the dispatch decision itself cannot become a decompression-bomb surface."
  removed: []

key-files:
  created:
    - crates/chrys-source-svg/Cargo.toml
    - crates/chrys-source-svg/src/lib.rs
    - crates/chrys-source-svg/src/fonts.rs
    - crates/chrys-source-svg/src/sniff.rs
    - crates/chrys-source-svg/tests/svg.rs
    - crates/chrys-source-svg/examples/render-svg.rs
    - crates/chrys-source-svg/fonts/NotoSans-Regular.ttf
    - crates/chrys-source-svg/fonts/OFL.txt
    - crates/chrys-cli/tests/svg_dispatch.rs
    - tests/golden/formats/svg/base.svg
    - tests/golden/formats/svg/candidate.svg
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/chrys-cli/Cargo.toml
    - crates/chrys-cli/src/main.rs
    - crates/chrys-cli/tests/unsafe_guard.rs

key-decisions:
  - "The OFL licence text is committed from google/fonts' ofl/notosans/OFL.txt, not from notofonts.github.io directly. At the pinned tag (noto-monthly-release-2026.09.01) the notofonts.github.io tree carries no per-family OFL.txt anywhere under fonts/NotoSans/; it carries only a repository-root Apache-2.0 LICENSE covering that repo's own build tooling. The font's own embedded name-table copyright string (\"Copyright 2022 The Noto Project Authors (https://github.com/notofonts/latin-greek-cyrillic)\") matches google/fonts' OFL.txt for the same family verbatim, so that file is the licence committed beside the font. See Deviations."
  - "SvgError uses five variants (Io, FileTooLarge, Parse, CanvasTooLarge, AllocTooLarge, EmptyCanvas — six, not five) rather than folding the canvas-dimension and allocation-size refusals into one variant, so each of the three size-based refusals plan 04-02 tests names its own field set rather than a shared, less specific one."
  - "build_options() constructs a fresh usvg::Options (and a fresh pinned_fontdb()) on every SvgSource::load call rather than caching one Arc<Database> on SvgSource itself, matching the plan's own action text ('In src/lib.rs, build usvg::Options') and keeping SvgSource a small, cheaply-constructed value with no shared mutable state; the embedded font bytes are static, so the repeated Database::new() + load_font_data() cost is the only overhead, paid once per load."

requirements-completed: [SRC-04, DET-05]

coverage:
  - id: D1
    description: "chrys compare over the committed SVG pair exits 1 and names a Recoloured region and its rectangle, at the same quality a raster pair reports (SRC-04)."
    requirement: SRC-04
    verification:
      - kind: unit
        ref: "crates/chrys-cli/tests/svg_dispatch.rs#an_svg_pair_reports_a_recoloured_region"
        status: pass
      - kind: integration
        ref: "manual run over a freshly built release binary, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D2
    description: "The SVG dispatch decides on content, not extension: an extensionless copy of the pair still reaches SvgSource, and every already-committed still and animated fixture still sniffs false for looks_like_svg."
    requirement: SRC-04
    verification:
      - kind: unit
        ref: "crates/chrys-cli/tests/svg_dispatch.rs#an_svg_file_with_no_extension_still_dispatches_to_svg_source, #no_committed_fixture_is_taken_from_the_adapter_it_already_used"
        status: pass
    human_judgment: false
  - id: D3
    description: "SvgSource's frames carry straight alpha, proved on a pixel where premultiplied and straight alpha disagree, and drilled red against the premultiplied accessor."
    requirement: SRC-04
    verification:
      - kind: unit
        ref: "crates/chrys-source-svg/tests/svg.rs#alpha_is_straight_not_premultiplied"
        status: pass
      - kind: other
        ref: "planted-defect drill in a disposable worktree, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every glyph a text node renders comes from the one pinned font: an unmatched family and no family at all render byte-identical to the pinned family, and all three render a real, non-empty glyph area."
    requirement: DET-05
    verification:
      - kind: unit
        ref: "crates/chrys-source-svg/tests/svg.rs#an_unmatched_font_family_falls_back_to_the_pinned_font, #the_pinned_database_holds_one_face_whose_family_the_options_name"
        status: pass
      - kind: other
        ref: "planted-defect drill in a disposable worktree, recorded verbatim below"
        status: pass
    human_judgment: true
  - id: D5
    description: "The font and its licence are committed together, and the pin (release tag, URL, byte length, SHA-256) is reproducible from this summary."
    requirement: DET-05
    verification:
      - kind: other
        ref: "git ls-files --error-unmatch, recorded below; byte length and SHA-256 recorded below"
        status: pass
    human_judgment: false
  - id: D6
    description: "resvg is pinned with default-features off and only the text feature on; chrys-source-svg's own dependency tree names no image, gif, image-webp or zune-jpeg crate."
    requirement: DET-05
    verification:
      - kind: other
        ref: "cargo tree -p chrys-source-svg -e normal, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D7
    description: "No file under crates/chrys-core/ changes anywhere in this plan's commit range."
    requirement: (D-03, project-wide invariant)
    verification:
      - kind: other
        ref: "git rev-parse HEAD:crates/chrys-core read c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 before this plan's first commit and after each of its four commits"
        status: pass
    human_judgment: false
  - id: D8
    description: "cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --check all pass at every commit boundary; the digest and unsafe-code-guard surfaces stay green."
    requirement: (project-wide invariant)
    verification:
      - kind: other
        ref: "cargo test --workspace: 0 failed at HEAD; cargo clippy --workspace --all-targets -- -D warnings: clean; cargo fmt --check: clean; cargo test -p chrys-cli --test digest: 6 passed; cargo test -p chrys-cli --test unsafe_guard: 1 passed at floor nine; cargo test -p chrys-core --test determinism: 6 passed"
        status: pass
    human_judgment: false

duration: not captured (PLAN_START_TIME was not recorded at launch)
completed: 2026-09-08
status: complete
---

# Phase 4 Plan 01: Compare an SVG pair through a new SVG adapter Summary

**A new `chrys-source-svg` crate rasterizes an SVG pair on the CPU through `resvg` (system-fonts compiled out), converts `tiny_skia`'s premultiplied pixmap to the straight alpha `Frame` documents, and resolves every glyph — named, unmatched, or unnamed — to the one `NotoSans-Regular.ttf` this repository commits.**

```
first commit: e70c85bbb5d76db131b1ae1c03d73dbd219499cd
last commit: e4f3a246a1b48d88450802f04835dbfc8d7eebe5
```

## Performance

- **Duration:** not captured (see frontmatter note)
- **Completed:** 2026-09-08
- **Tasks:** 3 / 3 (four commits: Task 1 split into its own two commits, as its own action text specifies)
- **Files modified:** 16 (5 modified, 11 new)

## Accomplishments

### Task 1 — SRC-04, the crate, the pin, the dispatch, the fixture (two commits)

- Downloaded `NotoSans-Regular.ttf` from the dated tag `noto-monthly-release-2026.09.01` of `github.com/notofonts/notofonts.github.io` (not a moving branch), committed alongside `OFL.txt`. See Deviations for why `OFL.txt` was sourced from `google/fonts` rather than the notofonts tree directly.
- Added `resvg = { version = "=0.48.1", default-features = false, features = ["text"] }` to the workspace dependency table. No separate `usvg`, `tiny-skia` or `fontdb` line was declared; all three resolved transitively through `resvg`'s own manifest.
- Created `crates/chrys-source-svg`: `#![forbid(unsafe_code)]`, `[lints] workspace = true`, `[dependencies]` limited to `chrys-source`, `resvg`, `thiserror`, `[dev-dependencies]` limited to `chrys-source-raster` and `png`.
- `src/fonts.rs` holds the workspace's only `include_bytes!` call (`"../fonts/NotoSans-Regular.ttf"`, relative to `fonts.rs`'s own directory) and `pinned_fontdb()`, which loads exactly one face into an otherwise-empty `fontdb::Database`.
- `SvgSource::load` enforces `SvgLimits` (`max_width`/`max_height`/`max_alloc` matching `chrys_source_raster::DecodeLimits::default()`; `max_file_bytes` at 8 MiB) in order: `fs::metadata` cap, then a `Read::take`-bounded read cap, then `usvg::Tree::from_data`, then a canvas-dimension and allocation-size cap read from `tree.size().to_int_size()` before `tiny_skia::Pixmap::new`. `SvgError` carries six variants: `Io`, `FileTooLarge`, `Parse`, `CanvasTooLarge`, `AllocTooLarge`, `EmptyCanvas`.
- `looks_like_svg` reads at most 1024 bytes through a bounded read, skips a UTF-8 BOM, and answers true only when the first non-whitespace byte is `<` and the prefix contains `<svg`.
- `named_frames_for` (`crates/chrys-cli/src/main.rs`) gained a fourth branch, checked before the animation sniff: a content-sniffed SVG file loads through `SvgSource::new()`.
- The unsafe-code guard's crate floor was raised from eight to nine in the same commit that added the ninth crate.
- `tests/golden/formats/svg/base.svg`/`candidate.svg`: a 256x256 fixture with four saturated corner rectangles, a curved path, a linear-gradient band, one shape at 50% fill-opacity, and two text elements (one naming the pinned family explicitly, one with no `font-family` attribute at all). The candidate recolours exactly one rectangle (green `#00cc00` to magenta `#cc00cc`).
- `crates/chrys-source-svg/tests/svg.rs#identical_svgs_produce_identical_frames` and `crates/chrys-cli/tests/svg_dispatch.rs` (3 tests) all pass; see the recorded runs below.

### Task 2 — the alpha conversion, drilled, and the renderer

- `crates/chrys-source-svg/tests/svg.rs#alpha_is_straight_not_premultiplied`: builds a two-rectangle SVG in the test itself (one pure-red rectangle at 50% fill-opacity, one at full opacity), asserts the translucent sample's red channel exceeds its alpha by more than 100, its alpha lies between 120 and 140, and its green/blue are below 8; asserts the opaque sample's alpha is 255 and red is at least 250.
- `crates/chrys-source-svg/examples/render-svg.rs`: loads an SVG through `SvgSource` and writes the exact returned RGBA8 buffer as a PNG via the `png` crate (dev-dependency), no second rendering path.

### Task 3 — DET-05's rendering half, and a real correction to the plan's own assumption

- `crates/chrys-source-svg/tests/svg.rs#the_pinned_database_holds_one_face_whose_family_the_options_name`: asserts `pinned_fontdb()` holds exactly one face, and asserts that face's own declared family equals `pinned_family_name`'s return, never a string literal typed twice.
- `crates/chrys-source-svg/tests/svg.rs#an_unmatched_font_family_falls_back_to_the_pinned_font`: renders three documents (pinned family, `"Arial"`, no family attribute) and asserts all three produce byte-identical, non-empty buffers.
- **The first run of this test failed and surfaced a real gap**, not a test bug: see "Predictions That Turned Out Wrong" and Deviations.
- Fixed `fonts.rs`: `pinned_fontdb()` now also calls `set_serif_family`, `set_sans_serif_family`, `set_cursive_family`, `set_fantasy_family`, `set_monospace_family`, all with the pinned font's own name, so `usvg`'s own always-appended generic-family fallback resolves to the pinned font in every case, not only the no-`font-family`-attribute case `usvg::Options::font_family` covers.

## Task Commits

1. **Task 1a: Pin one font and its licence in the repository** — `e70c85b`
2. **Task 1b: Compare an SVG pair through a new SVG adapter** — `9b6a5db`
3. **Task 2: Convert a rasterized pixmap to straight alpha** — `f4d3a59`
4. **Task 3: Resolve every text node to the one pinned font** — `e4f3a24`

**Plan metadata:** pending (this SUMMARY and STATE.md/ROADMAP.md updates are committed by the orchestrator).

## Files Created/Modified

See `key-files` in frontmatter.

## Decisions Made

See `key-decisions` in frontmatter.

## Font Pin, Reproducible

- **Release tag:** `noto-monthly-release-2026.09.01` (`github.com/notofonts/notofonts.github.io`)
- **Download URL:** `https://raw.githubusercontent.com/notofonts/notofonts.github.io/noto-monthly-release-2026.09.01/fonts/NotoSans/hinted/ttf/NotoSans-Regular.ttf`
- **Byte length:** 621572
- **SHA-256:** `478c558ea716033cd60c03438f628dfa75694dcf6b5f6d505a2f05fd2b4f3823`
- **Family name, read from the font itself (not typed from memory):** `Noto Sans`
- **Licence:** `crates/chrys-source-svg/fonts/OFL.txt`, fetched from `https://raw.githubusercontent.com/google/fonts/main/ofl/notosans/OFL.txt` — see Deviations for why this source was used instead of the notofonts.github.io tree.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] `pinned_fontdb` needed every generic-family mapping, not only the one-font database itself, or an unmatched `font-family` renders nothing**
- **Found during:** Task 3, the first run of `an_unmatched_font_family_falls_back_to_the_pinned_font`.
- **Issue:** The document naming `font-family="Arial"` rendered 0 non-transparent pixels, while the pinned-family and no-family documents each rendered 571. Reading `usvg`'s own parser (`usvg-0.48.1/src/parser/text.rs`) showed why: `usvg::Options::font_family` is only pushed onto the family candidate list when the `font-family` attribute is absent or fails to parse. A `font-family` attribute that parses but names a family the database does not carry is left as `FontFamily::Named("Arial")`, and `usvg`'s own `default_font_selector` (`usvg-0.48.1/src/text/mod.rs`) appends only a generic `fontdb::Family::Serif` query as its last-resort fallback — a fallback a `fontdb::Database` with no generic-family mapping of its own resolves to nothing.
- **Fix:** `fonts::pinned_fontdb()` now calls `set_serif_family`, `set_sans_serif_family`, `set_cursive_family`, `set_fantasy_family` and `set_monospace_family`, all with the pinned font's own name, so `usvg`'s always-appended generic-family fallback lands on the pinned font regardless of which named family a document requested or omitted.
- **Files modified:** `crates/chrys-source-svg/src/fonts.rs`.
- **Verification:** all three of `an_unmatched_font_family_falls_back_to_the_pinned_font`'s documents now render byte-identical, 571-non-transparent-pixel buffers; the fix was also drilled (see below).
- **Committed in:** `e4f3a24`.

**2. [Rule 1 - Bug] `field_reassign_with_default` clippy lint against `build_options`' first draft**
- **Found during:** Task 1, the first `cargo clippy --workspace --all-targets -- -D warnings` run.
- **Issue:** `build_options()`'s first draft built `usvg::Options::default()` then reassigned `.fontdb` and `.font_family` as two separate statements, which `clippy::field_reassign_with_default` (part of `-D warnings`) refuses.
- **Fix:** Rewrote as one struct-literal expression with `..resvg::usvg::Options::default()`, setting `fontdb` and `font_family` inline.
- **Files modified:** `crates/chrys-source-svg/src/lib.rs`.
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` clean afterward.
- **Committed in:** `9b6a5db`.

**3. [Documentation accuracy] The OFL licence text was sourced from `google/fonts`, not from `notofonts.github.io`, because the pinned tag carries no per-family `OFL.txt`**
- **Found during:** Task 1, immediately after downloading the font.
- **Issue:** The plan's action text says to commit `OFL.txt` "as that release ships it." Walking the `notofonts.github.io` tree at the pinned tag (`fonts/NotoSans/{hinted,full,googlefonts,unhinted}/ttf/`) found only `.ttf` files at every path checked; the repository root carries a single Apache-2.0 `LICENSE` that covers that repo's own build/scripts, not the font's own SIL OFL terms.
- **Fix:** Fetched `OFL.txt` from `google/fonts`' `ofl/notosans/OFL.txt` instead. Its stated copyright line ("Copyright 2022 The Noto Project Authors (https://github.com/notofonts/latin-greek-cyrillic)") is byte-identical to the copyright string embedded in `NotoSans-Regular.ttf`'s own `name` table, confirming this is the correct licence text for this exact font, not a different Noto family's terms.
- **Files modified:** none beyond the originally planned `crates/chrys-source-svg/fonts/OFL.txt`.
- **Verification:** the two copyright strings compared by hand; both name the same authors and the same upstream repository.
- **Committed in:** `e70c85b`.

---

**Total deviations:** 3 (1 missing-functionality/Rule 2, 1 bug/Rule 1, 1 documentation-accuracy note). No architectural change; no scope beyond this plan's own stated files, except `fonts.rs`'s generic-family additions, which live inside the one file Task 1 already created and Task 3's own `<files>` list already names.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None — no external service configuration required. The font was downloaded once, by this executor, from a public, dated release tag, and is now committed.

## `chrys compare` Over The Committed SVG Pair, Recorded Verbatim

```
$ cargo build --release
    Finished `release` profile [optimized] target(s) in 19.67s

$ ./target/release/chrys compare tests/golden/formats/svg/base.svg tests/golden/formats/svg/candidate.svg
Recoloured region at x=192, y=16, width=48, height=48, colour delta 199.12 (base [0, 204, 0, 255], candidate [204, 0, 204, 255])
$ echo $?
1
```

Exactly the recoloured rectangle: kind `Recoloured`, rectangle `x=192, y=16, width=48, height=48`. This is success criterion 1's full claim — a named kind and a bounding box, the same quality a raster pair reports — not a bare pixel count.

## The Rendered Fixture, Inspected Once

`cargo run -q -p chrys-source-svg --example render-svg -- tests/golden/formats/svg/base.svg target/svg-base.png`, then viewed. Both text elements read as authored: **"Chrysoberyl"** (naming the pinned family explicitly) and **"Rasterized"** (naming no family at all), every glyph in both drew rather than coming out blank or boxed. The curved path and the linear-gradient band are both visible. The 50%-opacity magenta rectangle is visibly translucent against the white background and the four saturated corner rectangles (red, green, blue, yellow). The PNG was inspected and then deleted; it was never committed.

## The Two Planted-Defect Drills, Recorded Verbatim

Both drills ran inside a disposable `git worktree add <path> HEAD`, with the currently-uncommitted test/source changes copied in by hand (each drill ran before its own task's commit landed), removed with `git worktree remove --force <path>` on completion. The real working tree held no uncommitted changes before or after either drill.

**Drill 1 (Task 2): `Pixmap::take_demultiplied()` replaced with `Pixmap::take()`** (the plain, still-premultiplied accessor), at `/tmp/chrys-alpha-drill`, made from commit `9b6a5db`:

```
thread 'alpha_is_straight_not_premultiplied' panicked at crates/chrys-source-svg/tests/svg.rs:107:5:
translucent pixel is red=128, green=0, blue=0, alpha=128: straight alpha must report a red channel that exceeds alpha by more than 100
test alpha_is_straight_not_premultiplied ... FAILED
```

The test went red and its message names the observed red (128) and alpha (128) — the same value, exactly what premultiplied storage at 50% opacity produces, and exactly what the test's own assertion is built to catch.

**Drill 2 (Task 3): the `font_family: family` line deleted from `build_options`'s `Options` literal**, leaving `usvg`'s own default (`"Times New Roman"`) in place, at `/tmp/chrys-fallback-drill`, made from commit `f4d3a59`:

```
test an_unmatched_font_family_falls_back_to_the_pinned_font ... ok
```

**The test stayed GREEN.** This is the second of the two informative outcomes the plan's own action text names, and it is the one that occurred: the `font_family: family` line is not what makes the fallback work. The load-bearing mechanism, discovered by this same task's fix (see Deviations), is the generic-family registration added to `fonts.rs`: `usvg`'s own font selector always appends a generic `Serif` query after every named family fails, and that query — not `Options::font_family` — is what resolves to the pinned font in every case this test exercises. This drill's own result is recorded here rather than left as an unstated assumption, per the plan's own instruction.

## Assumptions Log A3: Corrected, Not Confirmed

`04-RESEARCH.md`'s Assumptions Log A3 recommended setting `usvg::Options::font_family` to the pinned family's own name as the mechanism that makes an unmatched or absent `font-family` fall back to the pinned font. Task 3's first test run and Drill 2 above together show this recommendation was **incomplete**: `Options::font_family` only ever reaches `usvg`'s font-candidate list when a `font-family` attribute is entirely absent (or fails to parse). A `font-family` attribute that parses but names an unavailable family bypasses `Options::font_family` altogether and instead falls through `usvg`'s own always-appended generic `Serif` query, which only resolves to the pinned font because this plan additionally registered the pinned font as every one of `fontdb`'s five generic families. Both mechanisms are now in place (`Options::font_family` for the no-attribute case, the generic-family registration for the named-but-unavailable case), and both are proven by the same test.

## `cargo tree -p chrys-source-svg -e normal`, Recorded Verbatim (Relevant Lines)

```
chrys-source-svg v0.1.0
├── chrys-source v0.1.0
├── resvg v0.48.1
│   ├── ... tiny-skia v0.12.0, usvg v0.48.1 (which pulls in fontdb v0.24.0, harfrust, skrifa, roxmltree, svgtypes, ...)
└── thiserror v2.0.20
```

No `image`, `gif`, `image-webp` or `zune-jpeg` anywhere in the tree; `resvg v0.48.1` present. The `raster-images` feature is off, confirmed by the absence of every one of those four crate names.

## Full Verification, This Plan's End State

```
cargo test -p chrys-source-svg identical_svgs_produce_identical_frames: 1 passed.
cargo test -p chrys-source-svg --test svg (all four): alpha_is_straight_not_premultiplied,
  identical_svgs_produce_identical_frames, an_unmatched_font_family_falls_back_to_the_pinned_font,
  the_pinned_database_holds_one_face_whose_family_the_options_name: 4 passed, 0 failed.
cargo test -p chrys-cli --test svg_dispatch: 3 passed, 0 failed.
cargo test -p chrys-cli an_svg_pair_reports_a_recoloured_region: 1 passed.
cargo test -p chrys-cli every_crate_in_the_workspace_forbids_unsafe_code: 1 passed (floor nine).
cargo test -p chrys-cli --test digest: 6 passed, 0 failed (CLI digest surface unmoved).
cargo test -p chrys-cli --test unsafe_guard: 1 passed, 0 failed.
cargo test -p chrys-core --test determinism: 6 passed, 0 failed (all six guards green).
cargo test --workspace: 0 failed at HEAD (e4f3a24); test count grew from the 261-passing baseline
  by the tests this plan added (4 in chrys-source-svg's unit tests, 4 in its integration tests, 3 in
  chrys-cli's svg_dispatch.rs).
cargo clippy --workspace --all-targets -- -D warnings: clean.
cargo fmt --check: clean.
cargo tree -p chrys-source-svg -e normal: resvg v0.48.1 present; no image, gif, image-webp or
  zune-jpeg anywhere in the tree.
git rev-parse HEAD:crates/chrys-core: c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 (unmoved, checked
  before this plan's first commit and after each of its four commits).
git ls-files --error-unmatch crates/chrys-source-svg/fonts/NotoSans-Regular.ttf
  crates/chrys-source-svg/fonts/OFL.txt tests/golden/formats/svg/base.svg
  tests/golden/formats/svg/candidate.svg: all four tracked.
Every already-committed format fixture (png, jpeg, webp, tiff, gif, apng, webp-anim) still exits 1
  through the adapter it used before this plan, confirmed by a manual re-run of chrys compare over
  each committed pair.
```

## Predictions That Turned Out Wrong

- **The plan's own recommendation for Assumptions Log A3 (setting only `usvg::Options::font_family`) was incomplete.** Task 3's first test run showed a document naming an unmatched-but-present `font-family` attribute rendered zero glyphs while the pinned-family and no-attribute documents both rendered correctly. The actual fix required registering the pinned font as every one of `fontdb`'s generic families, because `usvg`'s own font selector only reaches `Options::font_family` on the no-attribute path, and falls through a generic `Serif` query — which a bare one-font database cannot answer — on the named-but-unavailable path. See "Assumptions Log A3: Corrected, Not Confirmed" above.
- **Drill 2 (Task 3) confirmed the plan's own predicted "stays green" branch, not the "goes red" branch.** The plan named both outcomes as informative and required recording whichever occurred; the `font_family: family` line turned out not to be load-bearing for this test once the generic-family fix (found by the same task) was in place.
- Everything else the plan predicted held: the `take_demultiplied()` drill went red exactly as described, the engine tree id never moved, the digest and unsafe-code-guard surfaces stayed green, and the fixture's own arithmetic (one rectangle recoloured, one delta_e of 199.12, one 48x48 rectangle at (192, 16)) matched on the first run.

## Next Phase Readiness

- SRC-04 and DET-05 are both observable exactly as the plan's success criteria state: an SVG pair compares through the CLI at the same quality a raster pair does, and three documents naming a present, an absent, and no font family all render byte-identical, non-empty glyph buffers.
- `crates/chrys-core`'s tree object id is unmoved (`c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`), so plan 04-02's own guard work and plan 04-03's own phase gate both start from the same anchor this plan started from.
- The workspace `resvg` pin (`default-features = false, features = ["text"]`) is in place for plan 04-02's guard to read.
- The corrected understanding of `usvg`'s font-fallback chain (recorded above) should be read by whoever plans or reviews any later SVG-adapter work that touches font resolution again.

## Self-Check: PASSED

All created files verified present on disk (`crates/chrys-source-svg/Cargo.toml`, `src/lib.rs`, `src/fonts.rs`, `src/sniff.rs`, `tests/svg.rs`, `examples/render-svg.rs`, `fonts/NotoSans-Regular.ttf`, `fonts/OFL.txt`, `crates/chrys-cli/tests/svg_dispatch.rs`, `tests/golden/formats/svg/base.svg`, `tests/golden/formats/svg/candidate.svg`); all four commit hashes (`e70c85b`, `9b6a5db`, `f4d3a59`, `e4f3a24`) verified present in `git log --oneline`; `git rev-parse HEAD:crates/chrys-core` verified reading `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` after the final commit; working tree verified clean (`git status --short`) after both disposable drill worktrees were removed.

---
*Phase: 04-svg-rasterization*
*Completed: 2026-09-08*
