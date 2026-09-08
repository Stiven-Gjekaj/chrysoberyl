# Phase 4: SVG rasterization - Research

**Researched:** 2026-09-08
**Domain:** CPU-only SVG rasterization (resvg/usvg/tiny-skia), a pinned-font text pipeline, and the cross-OS golden-hash gate that settles or withdraws the phase's central determinism claim
**Confidence:** MEDIUM-HIGH on the resvg/usvg/tiny-skia API surface (every function signature and doc comment below was fetched from `docs.rs` or the crate's own `master` branch source this session, not recalled); MEDIUM on the cross-platform bit-identity claim itself, because that claim rests on the vendor's own documentation and this session could not run the actual six-runner matrix — the phase's own success criterion 3 is the only thing that can move this to HIGH; HIGH on every in-repo architecture fact, each grounded in a `Read` of the exact file this session.

## Summary

Phase 4 adds one new source adapter (`chrys-source-svg`) that rasterizes an SVG document into the same `chrys_source::Frame` shape every other adapter already produces, then feeds a font this project's own repository carries, never the host's, into that rasterization. The stack the roadmap already names is confirmed on the live registry this session: `resvg` 0.48.1 (`[VERIFIED: cargo info resvg, this session]`), which re-exports both `usvg` 0.48.1 and `tiny-skia` 0.12.0 at its own crate root (`pub use usvg; pub use tiny_skia;`, read from `crates/resvg/src/lib.rs` on the project's `master` branch this session), and `usvg` in turn re-exports `fontdb` 0.24.0 (`pub use fontdb;`). One dependency, `resvg`, is therefore sufficient for the new crate; no separate `usvg`, `tiny-skia` or `fontdb` line is needed in `Cargo.toml`, because the workspace never names a type the re-export does not already carry.

The single most load-bearing finding this session, and the one that answers DET-05 directly: `fontdb::Database::new()` constructs an **empty** database — no system font is loaded until something calls `load_system_fonts()` — and `usvg::Options` holds that database as a plain field the caller populates itself, through `Options::fontdb_mut()` or by building a `fontdb::Database` and assigning it before parsing `[VERIFIED: docs.rs/fontdb/0.24.0, "Create a new, empty Database"; docs.rs/usvg/0.48.1, `fontdb_mut()` and the `fontdb: Arc<Database>` field doc comment, both fetched this session]`. resvg never reaches for the host font database on its own initiative; a caller has to ask for that, by name, through one specific method call. The stronger finding is that this discipline does not have to rest on a person remembering not to call that method: `resvg`'s own Cargo feature list gates it. `resvg = "0.48.1"`'s **default features are `["svgz", "text", "system-fonts", "memmap-fonts", "raster-images"]`** `[VERIFIED: cargo info resvg, this session — printed feature table quoted verbatim below]`, so a plain `cargo add resvg` pulls system-font scanning in by default. Disabling `system-fonts` at the Cargo-feature level, the same compile-time-fact-over-runtime-rule move `01-LEARNINGS.md` already names as this project's own established pattern ("An invariant survives better as a compile-time fact than as a rule"), removes the capability from the dependency graph entirely rather than trusting a code review to catch a stray `load_system_fonts()` call.

The second load-bearing finding answers research question 4 (raster dimensions) with no ambiguity: `usvg::Tree::size()` returns the SVG's own declared `width`/`height` — "Image size. Size of an image that should be created to fit the SVG" `[VERIFIED: docs.rs/usvg/0.48.1, Tree::size doc comment, fetched this session]` — falling back to `usvg::Options::default_size` (100×100) only when the document declares neither a size nor a `viewBox`. The raster dimensions are therefore a pure, deterministic function of the SVG document's own bytes and one documented, fixed fallback constant; nothing in this pipeline measures a rendered result to decide how large to make the canvas.

The third finding is a straight repeat of a lesson Phase 1 already paid for once, in a different crate: `tiny_skia::Pixmap` stores **premultiplied** alpha, not the straight alpha `chrys_source::Frame`'s own doc comment requires `[VERIFIED: docs.rs/tiny-skia/0.12.0, "A container that owns premultiplied RGBA pixels", fetched this session]`. The conversion is one call, not a hand-rolled divide-by-alpha loop: `Pixmap::take_demultiplied(self) -> Vec<u8>`, "Consumes the pixmap and returns the internal data as demultiplied RGBA bytes... Byteorder: RGBA" `[VERIFIED: docs.rs/tiny-skia source, pixmap.rs, fetched this session — signature and doc comment quoted verbatim]`. This is the exact byte order and exact alpha convention `Frame` already documents; no channel swap and no extra pass is needed once this one call is made, and skipping it silently produces a frame whose colour channels read too dark wherever alpha is not 255 — the same class of defect PROJECT.md's own Key Decisions table already recorded once for a different reason (the alpha decision of 2026-09-07).

The fourth, and the one this research cannot resolve from documentation alone, is the central risk the phase brief itself names: resvg's own README states "if you render an SVG file on x86 Windows and then render it on ARM macOS - the produced image will be identical. Each pixel would have the same value" `[CITED: github.com/linebender/resvg README, fetched this session]`, and tiny-skia's own stated design goal is to match Skia's output pixel-for-pixel, "absent bugs" `[CITED: search-engine synthesis of tiny-skia's own stated goals, not independently re-read from a single primary source this session]`. Both claims are the vendor's own, unverified by this session's own execution, and tiny-skia ships target-gated SIMD (SSE2/AVX2 on x86, NEON on AArch64, SIMD128 on wasm) behind a `simd` feature that is **on by default** `[VERIFIED: cargo info tiny-skia, this session]`. Unlike Phase 1's `rustfft::FftPlanner`, this dispatch is resolved at compile time per target architecture, not per running CPU at runtime, so it cannot introduce a mismatch between two machines of the *same* architecture the way `FftPlanner`'s runtime AVX detection could. It can still, in principle, produce a different rounding result between x86-64's SSE2/AVX2 path and AArch64's NEON path for the same input — exactly the cross-architecture question success criterion 3 exists to settle empirically. This research recommends building the fixture and running the six-runner matrix first, with `simd` left on (the tested, documented, default configuration every resvg user actually runs); disabling the `simd` feature (`tiny-skia = { version = "=0.12.0", default-features = false, features = ["std", "png-format"] }`) is the documented fallback if, and only if, that matrix goes red, exactly the same order of operations Phase 1 used for `mul_add()` (Pitfall 2, `01-RESEARCH.md`): trust the "guaranteed" wording as the starting hypothesis, then let the golden-hash CI job either confirm or falsify it, and act on the result rather than the marketing claim.

**Primary recommendation:** add one new crate, `chrys-source-svg`, depending only on `resvg` (`default-features = false, features = ["text"]` — `system-fonts`, `svgz`, `memmap-fonts` and `raster-images` all dropped) and `chrys-source`; build a `fontdb::Database` from `include_bytes!` on one pinned, repository-committed font (Noto Sans Regular, SIL Open Font License 1.1, sourced from the `notofonts/notofonts.github.io` project's own monthly release tag — the same family resvg's own reftest suite already uses); enforce an explicit file-size cap and a post-parse canvas-dimension cap, since `usvg::Options` carries no size-limit field of its own; and extend `.github/workflows/determinism.yml`'s `digest` job with one more `echo`/`cargo run` block, touching no other line, the same append-only pattern the `agree` job's whole-file `cmp -s` comparison was rewritten in Phase 2 specifically to accommodate.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SRC-04 | An SVG pair is rasterized on the CPU and compared | `chrys-source-svg`'s `SvgSource: Source`, built on `resvg`/`usvg`/`tiny-skia` (Architecture Patterns, Patterns 1-3); CLI dispatch extension in `named_frames_for` (Architecture Patterns, Pattern 5) |
| DET-05 | Text rasterization uses a font set pinned in the repository, never the host font database | `resvg`'s `system-fonts` Cargo feature disabled at the dependency level (compile-time enforcement, Pattern 1); `fontdb::Database::new()` plus `load_font_data` on an `include_bytes!`-embedded, repository-committed font (Pattern 2); a static guard test for `load_system_fonts` (Common Pitfalls) |

## Architectural Responsibility Map

This project has no browser/server/CDN tiers; the boundary is the one Phase 1 drew and every phase since has kept: adapter (format-aware) vs. engine (format-blind) vs. CLI (orchestration, dispatch).

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| SVG parsing and simplification (`usvg::Tree::from_data`) | New adapter crate (`chrys-source-svg`) | — | Format knowledge stays confined to the adapter; `chrys-core` declares no format crate today and this phase does not change that |
| Font database construction (empty `fontdb::Database`, one `load_font_data` call) | `chrys-source-svg` | — | DET-05 is an adapter-level guarantee: the engine never sees a font, only the `Frame` the adapter already rasterized |
| CPU rasterization (`resvg::render` into a `tiny_skia::Pixmap`) | `chrys-source-svg` | — | The one CPU-only rasterization boundary invariant 1 already states; no GPU crate enters this or any other crate's dependency graph |
| Premultiplied-to-straight-alpha conversion (`Pixmap::take_demultiplied`) | `chrys-source-svg` | — | The adapter's job, same as every other adapter, is to hand the engine one canonical shape; the engine has never once had to know an alpha convention existed to fix |
| File-content sniff (is this file an SVG?) | CLI (`chrys-cli`, `named_frames_for`) | — | The exact same tier Phase 2/3 already put the animation-vs-still sniff in; a new format's dispatch decision is a CLI concern, never an engine one |
| Comparison engine itself (register, classify, hash, verdict) | Engine (`chrys-core`) | — | **Unchanged this phase.** An SVG decodes to exactly one `Frame`, same shape a `RasterSource` already produces; the engine cannot tell the two apart and was never meant to |
| Cross-OS determinism gate (the new fixture, the new digest lines) | CI / test harness (`.github/workflows/determinism.yml`) | — | Extends the existing `digest`/`agree` jobs Phase 1 built and Phase 2 already proved append-safe; this phase adds no new job |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `resvg` | `=0.48.1` | SVG parse + CPU rasterize; re-exports `usvg` and `tiny_skia` at its own crate root | `[VERIFIED: cargo info resvg, this session — "version: 0.48.1", repository github.com/linebender/resvg, license Apache-2.0 OR MIT]`. The only actively maintained pure-Rust, CPU-only SVG rasterizer; the roadmap already names it |
| (re-exported) `usvg` | `0.48.1` | SVG-to-tree simplification, text-to-glyph resolution, `Tree::size()` | Same crate family; `resvg`'s own `Cargo.toml` pins `usvg = { path = "../usvg", version = "0.48.1" }` `[VERIFIED: crates/resvg/Cargo.toml on github.com/linebender/resvg master, fetched this session]` |
| (re-exported) `tiny_skia` | `0.12.0` | The CPU rasterizer proper: `Pixmap`, `render()`'s pixel-fill path | `[VERIFIED: cargo info tiny-skia, this session — "version: 0.12.0", repository github.com/linebender/tiny-skia, license BSD-3-Clause]` |
| (re-exported) `fontdb` | `0.24.0` | In-memory font database with CSS-like queries; the empty-by-default `Database` this phase pins fonts into | `[VERIFIED: cargo info fontdb, this session — "version: 0.24.0", repository github.com/RazrFalcon/fontdb]`. Note: still under the pre-transfer maintainer's own repository; `resvg`/`usvg`/`tiny-skia` moved to `github.com/linebender` in the 0.45.0 release, `fontdb` has not (see State of the Art) |
| `chrys-source` (workspace) | — | `Frame`, `Source` trait, `load_named` default body | `chrys-source-svg` depends on it and returns exactly one `Frame` per `load`, the same shape `chrys-source-raster` already returns |

### Supporting

None. `chrys-source-svg` needs no dependency beyond `resvg` and `chrys-source`; `thiserror` (already pinned workspace-wide) covers its error type, the same pattern every prior adapter crate already uses.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `resvg`/`tiny-skia` (CPU) | `librsvg` (Cairo/GTK, C bindings) | Cairo can use a GPU backend and pulls a C toolchain and system library dependency into a pure-Rust workspace; invariant 4 (CPU-only) and the project's own "the core is pure Rust" constraint both rule this out without further research |
| `resvg`/`tiny-skia` (CPU) | Headless Chromium/`resvg`'s own comparison target, a browser SVG renderer | A browser engine is orders of magnitude larger, is not CPU-only by construction (many browsers accelerate SVG/canvas paths on the GPU), and is not embeddable as a Rust library; never a serious candidate for this project's own constraints |
| Noto Sans (this research's pick) | DejaVu Sans (Bitstream Vera license) | Both are freely redistributable and both would satisfy DET-05. DejaVu's own license page states no current version number or release date `[CITED: dejavu-fonts.github.io/License.html, fetched this session — the page states redistribution terms but carries no version/date]`, and its own upstream project has been in low-activity maintenance for years; Noto Sans is resvg's own test-suite font family (`NotoSans-Regular.ttf`, `NotoSans-Bold.ttf` and siblings already live in `crates/resvg/tests/fonts/` on resvg's own repository `[VERIFIED: github.com/linebender/resvg, directory listing fetched this session]`), ships monthly-dated, tagged releases (e.g. `noto-monthly-release-2026.09.01`) from `notofonts/notofonts.github.io`, and is licensed SIL OFL 1.1, a licence written specifically for bundling and redistributing fonts. Pin one dated release tag; do not track a moving branch |
| `include_bytes!` (this research's pick) | `fontdb::Database::load_font_file` against a path resolved at runtime | `load_font_file` needs a working-directory-relative or `CARGO_MANIFEST_DIR`-relative path resolved correctly on every OS's own path-separator convention; `include_bytes!` bakes the font into the compiled binary at build time, so a wrong working directory in CI, in a downstream consumer, or in a future packaged binary can never produce "font not found" instead of a comparison — it can only ever produce a bigger binary. `Database::load_font_data(Vec<u8>)` takes the resulting owned buffer directly |

**Installation:**
```bash
cargo new --lib crates/chrys-source-svg
cargo add --manifest-path crates/chrys-source-svg/Cargo.toml --path ../chrys-source chrys-source
cargo add resvg@=0.48.1 --no-default-features --features text --manifest-path crates/chrys-source-svg/Cargo.toml
cargo add thiserror --manifest-path crates/chrys-source-svg/Cargo.toml
```

**Version verification:** `resvg`, `tiny-skia` and `fontdb` were checked against the live crates.io registry this session with `cargo info`, matching the pattern `01-RESEARCH.md` and `03-RESEARCH.md` both used. Re-run `cargo info resvg` at plan time; `resvg` released twice in the four weeks before this research (`0.48.0` on 2026-07-31, `0.48.1` on 2026-08-02), so this is an actively moving target, more so than any crate `01-RESEARCH.md`/`03-RESEARCH.md` pinned.

## Package Legitimacy Audit

| Package | Registry | Age (first published) | Weekly Downloads | Source Repo | Verdict | Disposition |
|---------|----------|------------------------|-------------------|--------------|---------|-------------|
| `resvg` | crates.io | 2017-12-18 (~9 yr) | 672,848/wk | github.com/linebender/resvg | OK | Approved |
| `usvg` (transitive, re-exported) | crates.io | 2018-04-24 (~8 yr) | 719,113/wk | github.com/linebender/resvg | OK | Approved |
| `tiny-skia` (transitive, re-exported) | crates.io | 2020-07-04 (~6 yr) | 1,128,608/wk | github.com/linebender/tiny-skia | OK | Approved |
| `fontdb` (transitive, re-exported) | crates.io | 2020-07-02 (~6 yr) | 841,500/wk | github.com/RazrFalcon/fontdb | OK | Approved |
| `rustybuzz` (transitive, text shaping) | crates.io | 2020-07-04 (~6 yr) | 752,543/wk | github.com/harfbuzz/rustybuzz | OK | Approved (not a direct dependency; audited because it sits on the text-shaping path DET-05 depends on) |
| `ttf-parser` (transitive) | crates.io | 2019-06-18 (~7 yr) | 2,161,168/wk | github.com/harfbuzz/ttf-parser | OK | Approved |
| `roxmltree` (transitive, XML parse) | crates.io | 2018-08-31 (~8 yr) | 1,625,839/wk | github.com/RazrFalcon/roxmltree | OK | Approved |

`[VERIFIED: gsd_run query package-legitimacy check --ecosystem crates, this session — every row's `verdict: "OK"`, `reasons: []`, cross-checked against `cargo info` for the primary three, this session]`

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** none.

**A note the audit table cannot carry on its own:** `resvg`, `usvg` and `tiny-skia` all transferred stewardship from their original author (RazrFalcon) to the Linebender organization as of the `0.45.0` release, 2025-02-26, with an accompanying licence change from MPL-2.0 to Apache-2.0 OR MIT `[VERIFIED: CHANGELOG.md on github.com/linebender/resvg master, fetched this session — "This is the first release under the stewardship of Linebender... Many thanks to Yevhenii Reizner for the years of hard work" and "the license of this project changed from MPL-2.0 to Apache-2.0 OR MIT"]`. `fontdb` and `roxmltree` have not made the same move and remain under `github.com/RazrFalcon`. This is not a legitimacy concern by itself (Linebender is the same organization that stewards `tiny-skia`, `parley`, `vello` and other Rust 2D-graphics infrastructure, and the transfer is documented, not silent), but it means this project's own dependency tree now spans two organizations for what reads, from the crate names alone, as one project. The workspace's own `Cargo.lock` will pin exact commits regardless; no action beyond noting it is needed.

## Architecture Patterns

### System Architecture Diagram

```
   base.svg, candidate.svg
            |
            v
   +----------------------------+
   |  chrys-source-svg          |   1. file-size guard (own limit,
   |  SvgSource::load()         |      before any parse; no
   |                            |      equivalent exists upstream)
   |                            |   2. build fontdb::Database::new()
   |                            |      (empty), load_font_data() on
   |                            |      one include_bytes!-embedded,
   |                            |      repository-committed font
   |                            |   3. usvg::Tree::from_data(bytes, opt)
   |                            |      -- opt.fontdb holds ONLY the
   |                            |      font loaded in step 2
   |                            |   4. tree.size() -> canvas dimensions
   |                            |      (own dimension guard here,
   |                            |      before Pixmap allocation)
   |                            |   5. tiny_skia::Pixmap::new(w, h)
   |                            |   6. resvg::render(&tree, identity,
   |                            |      &mut pixmap.as_mut())
   |                            |   7. pixmap.take_demultiplied()
   |                            |      -> straight-alpha RGBA8 bytes
   +----------------------------+
            |
            v  Vec<Frame>  (RGBA8, straight alpha -- same shape
            |               every other adapter already produces)
            |
   +----------------------------------------------------------------+
   |  chrys-cli: named_frames_for() -- EXTENDED, not rewritten.      |
   |  A new content sniff (is this an SVG?) runs before the existing |
   |  animation sniff and before the raster fallback, matching the   |
   |  same "content, never extension" rule every adapter follows.   |
   +----------------------------------------------------------------+
            |
            v
   +----------------------------------------------------------------+
   |  chrys-core: compare_sequence() (UNCHANGED). Never learns an    |
   |  SVG existed; it sees two Vec<Frame>, exactly as before.        |
   +----------------------------------------------------------------+
            |
            v
   Verdict, printed or hashed exactly as any other pair's is.
   CI: the six-runner matrix hashes a text-bearing SVG fixture's raw
   RGBA8 output the same way it already hashes PNG/GIF/APNG/WebP.
```

### Recommended Project Structure

```
chrysoberyl/
├── crates/
│   ├── chrys-core/              # UNCHANGED this phase
│   ├── chrys-source/            # UNCHANGED this phase
│   ├── chrys-source-raster/     # UNCHANGED this phase
│   ├── chrys-source-animation/  # UNCHANGED this phase
│   ├── chrys-source-sequence/   # UNCHANGED this phase
│   ├── chrys-rule/              # UNCHANGED this phase
│   ├── chrys-baseline/          # UNCHANGED this phase
│   ├── chrys-source-svg/        # NEW: the SVG adapter
│   │   ├── fonts/
│   │   │   ├── NotoSans-Regular.ttf   # pinned, committed, one file
│   │   │   └── OFL.txt                # SIL Open Font License 1.1, verbatim
│   │   ├── src/
│   │   │   ├── lib.rs            # SvgSource, SvgError, SvgLimits
│   │   │   └── fonts.rs          # the ONE place include_bytes! runs
│   │   └── tests/
│   └── chrys-cli/                # EXTENDED: sniff dispatch, unchanged flags
└── tests/
    └── golden/
        └── formats/
            └── svg/               # NEW: base.svg, candidate.svg -- the
                                    # text-bearing fixture criterion 3 names
```

### Pattern 1: The `system-fonts` Cargo feature is disabled, not merely unused (DET-05)

**What:** `resvg`'s own default features are `["svgz", "text", "system-fonts", "memmap-fonts", "raster-images"]` `[VERIFIED: cargo info resvg, this session — feature table quoted verbatim: "+default = [svgz, text, system-fonts, memmap-fonts, raster-images]"]`. `system-fonts` is the feature that compiles `fontdb::Database::load_system_fonts()`'s call path into `usvg`. Depending on `resvg` with default features therefore builds a binary *capable* of reading the host font database, whether or not any code in this project ever calls the method — a capability DET-05 says must not exist, not merely must not be exercised.

**When to use:** Every `Cargo.toml` entry for `resvg` in this workspace, without exception.

**Example:**
```toml
# Cargo.toml, chrys-source-svg's own dependency line.
# system-fonts, svgz and memmap-fonts are all dropped. raster-images
# (decoding a <image> element's own embedded raster data) is dropped
# too, since SRC-04's fixture is vector content; add it back only if a
# later fixture needs an embedded raster image inside the SVG itself.
resvg = { version = "=0.48.1", default-features = false, features = ["text"] }
```

**Why this is the stronger form of the guarantee:** the exact discipline `01-LEARNINGS.md` already names for `FftPlannerScalar` ("the FFT planner is chosen, never auto-detected... a static guard asserts it") applies here at one layer higher: instead of a static guard asserting a *function* is never called, the Cargo feature graph asserts a *capability* was never compiled in. `cargo tree -e features -p chrys-source-svg` becomes the check, the same verification style `02-LEARNINGS.md`'s own "A new format's feature flag stops at the adapter" pattern already established for `chrys-source-animation`'s `image`/`gif` feature.

### Pattern 2: The font database is built once, from bytes the compiler embedded (DET-05)

**What:** `fontdb::Database::new()` is documented as "Create a new, empty `Database`" `[VERIFIED: docs.rs/fontdb/0.24.0, fetched this session]` — no font, system or otherwise, is present until something loads one. `load_font_data(&mut self, data: Vec<u8>)` "Loads a font data into the `Database`" `[VERIFIED: docs.rs/fontdb/0.24.0, fetched this session]`, accepting bytes already in memory rather than a file path.

**When to use:** The one place `chrys-source-svg` constructs a `usvg::Options` value, before any `Tree::from_data` call.

**Example:**
```rust
// crates/chrys-source-svg/src/fonts.rs — recommended, not yet written.
// The font's bytes are compiled into this crate's own binary, so no
// working-directory assumption, no CARGO_MANIFEST_DIR resolution, and
// no missing-file error path exist at all: the font is either present
// (it always is, once this crate compiles) or the build itself failed.
const NOTO_SANS_REGULAR: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");

/// Build a font database holding exactly one font: the one this crate
/// carries in its own source tree. No system font, and no font this
/// crate did not commit, is ever reachable through the returned
/// database.
pub(crate) fn pinned_fontdb() -> resvg::usvg::fontdb::Database {
    let mut db = resvg::usvg::fontdb::Database::new();
    db.load_font_data(NOTO_SANS_REGULAR.to_vec());
    db
}
```
```rust
// crates/chrys-source-svg/src/lib.rs — recommended, not yet written.
use resvg::usvg;

fn build_options() -> usvg::Options<'static> {
    let mut options = usvg::Options {
        fontdb: std::sync::Arc::new(crate::fonts::pinned_fontdb()),
        ..usvg::Options::default()
    };
    // font_family is usvg's own fallback when an SVG names a family
    // this database does not carry (e.g. a <text> with no font-family
    // at all, or one naming a family this crate never pinned). Pointing
    // it at the one font this crate loaded means every text node
    // resolves to a font this repository committed, never to
    // usvg::Options::default()'s own "Times New Roman" fallback name,
    // which this database cannot supply and which would otherwise
    // resolve to nothing rather than to a deterministic glyph.
    options.font_family = "Noto Sans".to_string();
    options
}
```

### Pattern 3: The canvas size is read from the document, then capped before allocation

**What:** `usvg::Tree::size()` returns "Image size. Size of an image that should be created to fit the SVG. `width` and `height` in SVG" `[VERIFIED: docs.rs/usvg/0.48.1, fetched this session]`. `usvg::Options` carries no field bounding how large that size may be — this session read the full `Options` struct definition from the crate's own `master` source and found `resources_dir`, `dpi`, `font_family`, `font_size`, `languages`, `shape_rendering`, `text_rendering`, `image_rendering`, `default_size`, `image_href_resolver`, `font_resolver`, `fontdb`, `style_sheet`, and no size-limit field among them `[VERIFIED: crates/usvg/src/parser/options.rs on github.com/linebender/resvg master, fetched this session]`. An SVG the size of a postage stamp can declare `width="1000000" height="1000000"` and nothing upstream refuses it before a `Pixmap` of that size is allocated.

**When to use:** Immediately after `Tree::from_data` succeeds and before `Pixmap::new` is called. This is this project's own equivalent of `chrys-source-raster::DecodeLimits` and `chrys-source-animation::AnimationLimits`; every prior adapter in this workspace sets an explicit bound before allocating a pixel buffer, and `usvg` supplying no upstream bound is exactly the situation those two crates' own `max_width`/`max_height` fields already exist to cover for a different format.

**Example:**
```rust
// crates/chrys-source-svg/src/lib.rs — recommended, not yet written.
// Matches chrys_source_raster::DecodeLimits and
// chrys_source_animation::AnimationLimits in shape and in the exact
// numbers, so a crafted SVG is refused at the same size this
// workspace already refuses a crafted raster image or animation at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvgLimits {
    pub max_width: u32,
    pub max_height: u32,
    /// The largest SVG source file, in bytes, this crate will parse at
    /// all. A rule file's own sidecar carries the same kind of guard
    /// (`RasterError::HintsTooLarge`); an SVG document is source text,
    /// not a binary format, and a crafted multi-gigabyte file should
    /// never reach `usvg::Tree::from_data` in the first place.
    pub max_file_bytes: u64,
}

impl Default for SvgLimits {
    fn default() -> Self {
        SvgLimits {
            max_width: 16384,
            max_height: 16384,
            max_file_bytes: 8 * 1024 * 1024,
        }
    }
}
```

### Pattern 4: Straight alpha is one method call, not a per-pixel loop

**What:** `Pixmap::take_demultiplied(self) -> Vec<u8>` — "Consumes the pixmap and returns the internal data as demultiplied RGBA bytes... Byteorder: RGBA" `[VERIFIED: docs.rs/tiny-skia source, pixmap.rs, fetched this session — exact signature and doc comment]`. This is the RGBA byte order and the straight-alpha convention `chrys_source::Frame`'s own doc comment already requires, verbatim.

**When to use:** The one place `SvgSource::load` finishes building a `Frame`, after `resvg::render` has written into the `Pixmap`.

**Example:**
```rust
// crates/chrys-source-svg/src/lib.rs — recommended, not yet written.
use chrys_source::Frame;
use resvg::tiny_skia;

fn render_to_frame(tree: &resvg::usvg::Tree, width: u32, height: u32) -> Frame {
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .expect("width and height were already checked against SvgLimits and are non-zero");
    resvg::render(tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    Frame {
        pixels: pixmap.take_demultiplied(), // straight alpha, RGBA byte order
        width,
        height,
        index: 0,
        hints: Vec::new(),
    }
}
```

### Pattern 5: SVG dispatch is a content sniff, ahead of the existing animation sniff

**What:** `chrys-cli`'s `named_frames_for` currently dispatches on: is `path` a directory (sequence), else is it an animation by content-sniff (`chrys_source_animation::sniff::is_animation`, itself built on `image::ImageReader::open(path).with_guessed_format()`), else raster `[VERIFIED: crates/chrys-cli/src/main.rs:657-666, read this session]`. `image`'s own format guesser recognizes PNG/JPEG/GIF/WebP/TIFF by magic bytes; it has no SVG signature, so an SVG file today falls through to `RasterSource`, which will fail to decode it. A fourth branch is needed, and it must run on content, never on a `.svg` extension, the same rule every decode in this project already follows (`decode_guarded`'s own doc comment: "The format is guessed from the file's content, not from its extension").

**When to use:** `named_frames_for`, ahead of the animation-sniff branch (an SVG is never mistaken for a GIF/APNG/WebP by any sniff either one runs, so order between these two branches is not itself safety-critical, but placing the cheaper, purely-textual SVG check first avoids opening every non-directory file through `image::ImageReader` unless the SVG check has already said no).

**Example:**
```rust
// crates/chrys-cli/src/main.rs — recommended, not yet written.
// A real SVG sniff reads past a UTF-8 BOM and past a leading XML
// declaration/comment/DOCTYPE to find the first element tag, since
// none of those precede a "<svg" the way a bare "<svg ...>" at byte 0
// would. This sketch is deliberately conservative (checks only the
// simple, common case) — the plan's own task should decide how much
// of the SVG/XML grammar this sniff needs to tolerate, informed by
// what the shipped fixture actually looks like.
fn looks_like_svg(path: &Path) -> std::io::Result<bool> {
    let bytes = std::fs::read(path)?;
    let text_prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
    Ok(text_prefix.contains("<svg"))
}
```

### Anti-Patterns to Avoid

- **Calling `Database::load_system_fonts()` "just for local testing" and forgetting to remove it.** With the `system-fonts` Cargo feature disabled (Pattern 1), this call does not compile at all, which is the point: the anti-pattern is guarded against by the dependency graph, not by a comment asking a future editor to remember.
- **Skipping `take_demultiplied()` because the SVG fixture happens to have no partial transparency.** A fixture with `alpha == 255` everywhere hides this defect completely (premultiplied and straight alpha are byte-identical at full opacity), the same shape of trap `02-LEARNINGS.md` already recorded for the animated WebP compositor ("An alpha blend at full opacity is not lossless" — the reverse failure mode, but the same lesson: a fixture that never exercises partial alpha proves nothing about the alpha handling). The shipped fixture should include at least one partially transparent region.
- **Trusting `usvg::Options::default()`'s `font_family` ("Times New Roman") as a safety net.** This database was never loaded with a font by that name; a text node that falls through to this default resolves to nothing this repository controls. Pattern 2 points the default at the one pinned family instead.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SVG parsing, CSS resolution, `viewBox`/transform math | A custom SVG parser | `usvg::Tree::from_data` | `usvg`'s own stated purpose is exactly this: "a layer between an XML library and a potential SVG rendering library" that resolves "all the elements, attributes, references and other SVG features" `[CITED: usvg crate-root doc comment, github.com/linebender/resvg master, fetched this session]`; re-deriving this is a multi-year project on its own |
| Glyph shaping and outline extraction | Hand-rolled TrueType/OpenType parsing | `rustybuzz` + `ttf-parser` (both transitive through `resvg`'s `text` feature) | Already audited OK (Package Legitimacy Audit); a hand-rolled shaper has to separately earn every guarantee this pair already provides, and gets none of resvg's own test-suite coverage for free |
| Premultiplied-to-straight-alpha conversion | A per-pixel divide-by-alpha loop | `tiny_skia::Pixmap::take_demultiplied()` | One call, already correct at alpha == 0 (a hand-rolled divide needs its own zero-alpha special case or it divides by zero) |
| A font-embedding mechanism | A build script that copies a font file next to the binary at build time | `include_bytes!` | Zero runtime path resolution, zero missing-file failure mode, and the binary itself becomes the unit that either does or does not carry the pinned font — nothing about the deploy environment can change which font is used |

**Key insight:** every "don't hand-roll" item here is, like Phase 1's and Phase 3's own equivalent tables, also a determinism concern independently: a hand-rolled SVG parser, shaper or alpha conversion would each have to separately re-derive the exact bit-for-bit behaviour this phase is trying to prove, rather than inheriting a behaviour one upstream project already tests across the same architectures this project's own CI matrix covers.

## Common Pitfalls

### Pitfall 1: Trusting the "reproducible on all platforms" claim without the empirical test

**What goes wrong:** A plan treats resvg's own README claim — "if you render an SVG file on x86 Windows and then render it on ARM macOS - the produced image will be identical" `[CITED: github.com/linebender/resvg README, fetched this session]` — as settled fact, and skips or shortcuts the cross-OS golden-hash fixture success criterion 3 already requires.

**Why it happens:** The claim is stated plainly, by the project's own maintainers, with no hedge. Phase 1's own research found the identical trap in `rustfft`'s and `f64::mul_add`'s documentation: a "guaranteed" or "identical" claim describes the specification the authors intend, not an independent, adversarial test of every target this project's own CI matrix covers.

**How to avoid:** Build the cross-OS fixture and run the six-runner matrix before treating DET-05/SRC-04 as proven. This is not extra work invented by this research; it is success criterion 3 as already written in `ROADMAP.md`. Tiny-skia's `simd` feature (on by default, Summary) is the one variable this research could not independently rule out; if the matrix disagrees, disabling `simd` (Alternatives Considered) is the first thing to try, in the same "trust the guarantee, then let CI falsify it" order Phase 1 used for `mul_add()`.

**Warning signs:** A plan task that marks SRC-04/DET-05 complete before the six-runner matrix has actually run once against the new SVG fixture.

**Phase to address:** This phase's own final wave, as the phase gate — matching the pattern `03-RESEARCH.md`'s own Validation Architecture already established for a per-phase gate distinct from a per-wave one.

### Pitfall 2: A font-family name in the SVG that this database cannot answer

**What goes wrong:** An SVG's `<text font-family="Arial">` (or any family name other than the one pinned font's own name) resolves through `fontdb`'s CSS-like family matching against a database holding exactly one font. Depending on how that lookup fails, text can silently render with no glyphs, or with a substituted style this crate never chose, rather than failing loudly.

**Why it happens:** An SVG file is often authored assuming a rich, multi-family system font environment; a database intentionally restricted to one font is a much smaller world than most SVG authors assume.

**How to avoid:** Set `Options.font_family` to the one pinned family's own name (Pattern 2), so an SVG with no `font-family` at all, or one naming a family this database does not have, both resolve to the one committed font rather than to `usvg`'s own "Times New Roman" default (which this database cannot supply). The shipped fixture's own `<text>` elements should either omit `font-family` or name the pinned family explicitly, so the test proves the intended path, not an accidental fallback.

**Warning signs:** A rendered glyph area that is empty, or a rendered glyph that looks like a generic fallback box, in a fixture that was meant to render real Latin text.

**Phase to address:** This phase's fixture-authoring task, verified by inspecting the rendered PNG once, not only by a hash matching across runners (a hash proves consistency, not correctness — six runners can agree on a wrong picture).

### Pitfall 3: A malformed or oversized SVG reaching `Tree::from_data` unguarded

**What goes wrong:** `usvg::Options` carries no size-limit field (Pattern 3); a crafted SVG declaring an enormous `width`/`height`, or simply a very large file, is parsed and then a correspondingly enormous `Pixmap` is allocated, unbounded, before this crate's own code gets a chance to refuse it.

**Why it happens:** `usvg` is a general-purpose SVG library with no opinion on this project's own resource-limit conventions; every other adapter in this workspace (`DecodeLimits`, `AnimationLimits`) had to add this discipline itself too, because the underlying `image` crate's own `Limits` API does not extend to every crate this project depends on.

**How to avoid:** `SvgLimits` (Pattern 3), checked in two places: the raw file's byte length before `Tree::from_data` is ever called, and `Tree::size()`'s own `width`/`height` before `Pixmap::new` is called.

**Warning signs:** A fuzzed or adversarial-SVG test case that runs the process out of memory rather than returning a typed refusal error.

**Phase to address:** This phase, as its own dedicated test (a crafted SVG with `width="999999999"`, asserting a typed error, not a panic or an OOM).

### Pitfall 4: `include_bytes!`'s path is relative to the source file, not the crate root

**What goes wrong:** `include_bytes!("fonts/NotoSans-Regular.ttf")` (no `../`) fails to compile, or worse, silently resolves to the wrong file, if the macro is invoked from a module file that is not directly at `src/lib.rs`'s own directory level, because `include_bytes!`'s path is relative to the *current source file*, not to the crate root or `Cargo.toml`'s own directory.

**Why it happens:** This is `include_bytes!`'s documented behaviour, but it does not match `Cargo.toml`'s own path-resolution convention (relative to the manifest), which is the convention most of this workspace's own path handling already uses (dependency `path = "../chrys-core"` entries, for instance).

**How to avoid:** Put the `include_bytes!` call in `src/fonts.rs` (Pattern 2's own file), and write the path relative to that file's own location (`"../fonts/NotoSans-Regular.ttf"` when `fonts/` sits at the crate root, next to `src/`). Verify by compiling once; a wrong path is a compile-time error, not a silent runtime one, which is the main safety property `include_bytes!` already provides over a runtime file read.

**Warning signs:** A compile error naming a file that does not exist at the exact path given, immediately on the first `cargo build` after this crate is scaffolded.

**Phase to address:** This phase's first task, the moment the crate skeleton is created — a five-minute check, not a design decision.

## Code Examples

### The minimal parse-and-render pipeline, adapted from resvg's own example

```rust
// Source: crates/resvg/examples/minimal.rs, github.com/linebender/resvg
// master, fetched this session, adapted to this project's own
// pinned-font, no-system-fonts, size-guarded requirements. The
// upstream example calls opt.fontdb_mut().load_system_fonts() and
// sets no size limit; both of those calls are exactly what this
// phase's own adapter must NOT do (DET-05, Pattern 3).
use resvg::{tiny_skia, usvg};

fn rasterize(svg_bytes: &[u8], limits: &crate::SvgLimits) -> Result<chrys_source::Frame, crate::SvgError> {
    let options = crate::build_options(); // Pattern 2: one pinned font, no system fonts
    let tree = usvg::Tree::from_data(svg_bytes, &options)
        .map_err(|source| crate::SvgError::Parse { message: source.to_string() })?;

    let size = tree.size().to_int_size();
    if size.width() > limits.max_width || size.height() > limits.max_height {
        return Err(crate::SvgError::TooLarge {
            width: size.width(),
            height: size.height(),
            limit: limits.max_width.max(limits.max_height),
        });
    }

    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or(crate::SvgError::EmptyCanvas)?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    Ok(chrys_source::Frame {
        pixels: pixmap.take_demultiplied(), // Pattern 4: straight alpha, RGBA order
        width: size.width(),
        height: size.height(),
        index: 0,
        hints: Vec::new(),
    })
}
```

### Confirming the empty-by-default database, quoted from fontdb's own docs

```rust
// Source: docs.rs/fontdb/0.24.0, fetched this session — quoted for the
// exact wording, not as a code sample to compile.
// Database::new() doc comment: "Create a new, empty `Database`."
// load_system_fonts() doc comment: "Attempts to load system fonts.
// Supports Windows, Linux and macOS." — a database this crate never
// calls this method on never gains a system font, on any platform.
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `resvg`/`usvg`/`tiny-skia` maintained solely by RazrFalcon, under MPL-2.0 | Stewarded by the Linebender organization, under Apache-2.0 OR MIT | `0.45.0`, 2025-02-26 `[VERIFIED: CHANGELOG.md, github.com/linebender/resvg master, fetched this session]` | This project's own MIT-licensed core (`PROJECT.md`'s own constraint) is compatible with the new dual licence without further review; the repository URL this research cites (`github.com/linebender/resvg`) is the current one, not the historical `github.com/RazrFalcon/resvg` a training-data-only answer would likely still name |
| `resvg` versioned in lockstep as one large crate | `resvg`, `usvg` and `tiny-skia` versioned and released independently, though `resvg` 0.48.1 currently pins `usvg = "0.48.1"` (matching numbers, separate crates) | Ongoing, pre-dates this session | Pin `resvg` alone; do not add separate `usvg`/`tiny-skia` version lines that could drift from what `resvg`'s own `Cargo.toml` already requires |
| `fontdb`/`roxmltree` under the same maintainer as `resvg` | `fontdb` and `roxmltree` remain under `github.com/RazrFalcon`; only `resvg`/`usvg`/`tiny-skia` moved to Linebender | As of this session | No action needed; noted in the Package Legitimacy Audit so a future reader is not confused by two organizations appearing in one dependency tree |

**Deprecated/outdated:** any research or training-data reference to `github.com/RazrFalcon/resvg` as the current upstream — that URL now redirects to, or is stale relative to, `github.com/linebender/resvg`, verified this session.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | tiny-skia's SIMD paths (SSE2/AVX2 on x86-64, NEON on AArch64) produce bit-identical output to each other for this phase's own fixture | Summary, Pitfall 1 | This is the phase's own central, explicitly-flagged risk, not a side assumption; if the six-runner matrix disagrees, the documented fallback is disabling tiny-skia's `simd` feature (Alternatives Considered), re-running the matrix, and if it still disagrees, writing the withdrawal note the phase brief itself anticipates ("this project would rather withdraw a claim in writing than ship one it cannot prove") |
| A2 | Noto Sans Regular, sourced from `notofonts/notofonts.github.io`'s own monthly release tag, is the right single font to pin for this phase's fixture | Standard Stack, Alternatives Considered | Low risk: DejaVu Sans is a documented, equally viable fallback under the same OFL-adjacent (Bitstream Vera) permissive terms; switching fonts later touches only `chrys-source-svg/fonts/` and `fonts.rs`, never the engine or the CLI |
| A3 | `usvg::Options::font_family` set to the pinned family's own name is the right way to make an SVG with no `font-family`, or an unmatched one, resolve to the committed font rather than to a name this database cannot supply | Pattern 2, Pitfall 2 | This is this session's own design recommendation, not independently compiled and run against the exact pinned versions this session (unlike `03-RESEARCH.md`'s own `toml::Spanned` finding, which was compiled and run); verify at implementation time with a fixture whose `<text>` carries no `font-family` attribute at all |
| A4 | The exact release-asset path for `NotoSans-Regular.ttf` inside a `notofonts/notofonts.github.io` monthly release tag (e.g. under a `fonts/NotoSans/` subdirectory) was not directly browsed this session as a file listing | Standard Stack | Low risk, purely mechanical: the release exists and the family exists in that repository (confirmed this session); the exact subdirectory path is a five-minute lookup at implementation time, not a design decision |
| A5 | roxmltree's lack of documented DTD/custom-entity support means a billion-laughs-style SVG cannot reach an exponential expansion through this parser | Pitfall 3 | This session found roxmltree's own README states it lacks "complete DTD support" but found no explicit statement that custom internal general entities are refused outright, nor any independent falsification attempt run this session. Per this project's own evidence discipline, this absence of a stated protection is not itself a verified mitigation — `SvgLimits.max_file_bytes` (Pattern 3) is recommended as a defense that does not depend on this claim being true at all |

## Open Questions

1. **Does the six-runner matrix agree on the new SVG fixture with tiny-skia's default (SIMD-enabled) build?**
   - What we know: resvg's and tiny-skia's own documentation states cross-platform bit-identity is a design goal; this session could not run the matrix.
   - What's unclear: whether that goal holds in practice for a real, text-bearing fixture, on this project's own pinned toolchain and this project's own six runner labels.
   - Recommendation: run it first with `simd` on (the documented default every resvg user runs); only disable it if the matrix disagrees. Either outcome is itself the phase's headline finding, and either way, it belongs in `04-LEARNINGS.md`, not left as an open question past this phase.

2. **What is the exact `notofonts/notofonts.github.io` release-asset path for a single static `NotoSans-Regular.ttf` file?**
   - What we know: the repository exists, is the current canonical home for Noto Sans, tags monthly releases, and is SIL OFL 1.1 licensed.
   - What's unclear: the precise subdirectory/asset-name pattern inside one release's own download bundle, not browsed as a file listing this session.
   - Recommendation: resolved at implementation time, by fetching one release's own asset list directly; this does not change the licence, the family choice, or any code in this research.

3. **Should the SVG content-sniff (Pattern 5) tolerate an XML comment or a DOCTYPE preceding the first `<svg` tag, or only the simple "`<svg` appears in the first 512 bytes" case this research sketched?**
   - What we know: a real-world SVG frequently opens with an XML declaration (`<?xml version="1.0"?>`) before the root element; this research's own sketch already tolerates that, since it scans the first 512 bytes rather than checking only byte 0.
   - What's unclear: whether the shipped fixture, or any fixture a later task adds, needs a DOCTYPE or a leading comment tolerated too.
   - Recommendation: keep the sniff as simple as the shipped fixtures allow; widen it only when a real fixture needs it, rather than pre-building tolerance for SVG shapes this phase does not ship.

## Environment Availability

No new external tool, service, or system library is introduced by this phase. `resvg`, `usvg`, `tiny-skia` and `fontdb` are all pure Rust with `unsafe`-forbidden or `unsafe`-minimal implementations reachable through `cargo` alone; the font is committed to the repository, not fetched at build or run time. Every GitHub-hosted runner label `.github/workflows/determinism.yml` already uses (Phase 1's own Environment Availability table) needs no new dependency to run this phase's own fixture.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo`/`rustc` (local dev, pinned by `rust-toolchain.toml`) | All of Phase 4 | Yes | 1.97.1, well above `resvg`'s own `rust-version = "1.85.0"` floor `[VERIFIED: rust-toolchain.toml, read this session; cargo info resvg, this session]` | — |
| The six GitHub-hosted runner labels `.github/workflows/determinism.yml` already targets | The extended `digest`/`agree` jobs, criterion 3 | Yes, unchanged from Phase 1's own verified table | — | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in), unchanged from every prior phase |
| Config file | none new |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo test --workspace --all-features` |

### Sampling Rate

- **Per task commit:** `cargo test --workspace` (or `-p chrys-source-svg` for the inner loop; the full workspace command is what a task's own verification step should record, per this project's established convention).
- **Per wave merge:** `cargo test --workspace --all-features`, plus `scripts/engine-boundary-drill.sh <wave-first-commit> <wave-last-commit>` — this phase's own headline architectural claim ("`chrys-core`'s tree object id does not move") is the identical shape of claim Phase 2 and Phase 3 already proved with this same, already-existing, already-generic script.
- **Phase gate:** full suite green, the engine-boundary drill green across the whole phase's commit range, **and** the six-runner determinism matrix green on the new SVG fixture specifically (not merely on the four pre-existing fixture families) — all three, before `/gsd-verify-work`. The SVG fixture's own matrix result is this phase's actual exit gate for the font/curve determinism claim (criterion 3), distinct from and in addition to the pre-existing fixtures' continued agreement.

### Phase Requirements → Test Map

| Req ID | Behaviour | Test Type | Automated Command | File Exists? |
|--------|-----------|-----------|---------------------|-------------|
| SRC-04 | An SVG pair with no differences compares `Identical` | unit | `cargo test -p chrys-source-svg identical_svgs_produce_identical_frames` | ❌ Wave 0 |
| SRC-04 | An SVG pair with a real visual difference (e.g. a recoloured shape) compares `Changed`, at the same quality (a named kind, a bounding box) as a raster pair | integration | `cargo test -p chrys-cli an_svg_pair_reports_a_recoloured_region` (via `Command::new(env!("CARGO_BIN_EXE_chrys"))`, the pattern already used in `crates/chrys-cli/tests/digest.rs`) | ❌ Wave 0 |
| SRC-04 | `named_frames_for` routes an `.svg`-content file (regardless of its extension) through `SvgSource`, not `RasterSource` | unit | `cargo test -p chrys-cli an_svg_file_with_no_extension_still_dispatches_to_svg_source` | ❌ Wave 0 |
| DET-05 | `resvg`'s `system-fonts` Cargo feature is disabled workspace-wide | static (manifest guard) | `cargo test -p chrys-cli only_the_text_feature_of_resvg_is_enabled` — a guard reading `Cargo.lock`/`cargo tree -e features` output, mirroring `02-LEARNINGS.md`'s own "A new format's feature flag stops at the adapter" verification style | ❌ Wave 0 |
| DET-05 | The workspace source tree names `load_system_fonts` nowhere | static (source guard) | `cargo test -p chrys-core --test determinism` (extend the existing determinism guard file, mirroring `DENIED_DEPENDENCY_CRATES`'s own string-search technique, verified this session by reading `crates/chrys-core/tests/determinism.rs`'s own pattern) — *or*, if this guard is judged to belong nearer the crate it protects, `cargo test -p chrys-source-svg no_source_file_calls_load_system_fonts` | ❌ Wave 0 |
| DET-05 | A rendered glyph's pixels come only from the pinned font: a `<text>` naming an unrelated family still renders (falls back to the pinned family, Pattern 2), rather than rendering empty | unit | `cargo test -p chrys-source-svg an_unmatched_font_family_falls_back_to_the_pinned_font` | ❌ Wave 0 |
| SRC-04 + DET-05 (criterion 3) | A text-bearing SVG fixture's raw RGBA8 hash is identical on Linux, macOS and Windows | CI-only (six-runner matrix, extended) | `cargo run -q --release -p chrys-cli -- compare tests/golden/formats/svg/base.svg tests/golden/formats/svg/candidate.svg --hash-only`, appended to `.github/workflows/determinism.yml`'s existing `digest` job, checked by the existing, unmodified `agree` job's whole-file `cmp -s` | ❌ Wave 0 (CI workflow extension) |
| (adapter-internal, not a numbered requirement) | An oversized SVG (declared width/height, or raw file size, above `SvgLimits`) is refused with a typed error, not a panic or unbounded allocation | unit | `cargo test -p chrys-source-svg an_oversized_declared_canvas_is_refused` and `cargo test -p chrys-source-svg an_oversized_file_is_refused_before_parsing` | ❌ Wave 0 |
| (adapter-internal) | `Frame.pixels` from an `SvgSource` is straight alpha, verified against a fixture with a known partially-transparent pixel | unit | `cargo test -p chrys-source-svg alpha_is_straight_not_premultiplied` | ❌ Wave 0 |

### Manual-Only Verifications

| Verification | Why it cannot be automated |
|---------------|------------------------------|
| The shipped `tests/golden/formats/svg/base.svg`/`candidate.svg` fixture actually contains legible text, rendered with the pinned font, when opened as a PNG by a person | A hash match proves consistency across runners, not that the rendering itself looks like the text a person authored (Pitfall 2); a person needs to look once, the same "reads clearly to a person" caveat `03-RESEARCH.md`'s own Manual-Only Verifications table already records for its shipped rule-file example |

### The risks this phase carries

1. **The cross-architecture SIMD question (A1) is the phase's own unproven claim, carried forward exactly as the roadmap's own "three phases carry an unproven claim forward" framing anticipates for Phase 4, 5 and 7.** Unlike Phase 1's `FftPlanner` (where a scalar-only alternative was known and adopted before any CI run), this phase's recommended default is to run the matrix with SIMD *on* first, because that is the configuration every real resvg/tiny-skia user actually runs and the one this project's own dependency pin will build by default; only fall back to a scalar-only tiny-skia build if the matrix falsifies the vendor's own claim. **Recommendation:** the plan should treat "the matrix disagreed and `simd` had to be disabled" as an equally acceptable, equally documented outcome, not a failure of the plan — the phase brief itself says withdrawing a claim in writing is preferable to shipping one that cannot be proven.
2. **A font pinned today is a font this repository owns forever, including its own licence's own obligations.** SIL OFL 1.1 permits bundling and redistribution freely, but it is not public domain: the licence text itself (`OFL.txt`) must be committed alongside the font file, verbatim, the same way this project already treats every other externally-sourced artifact's provenance as something to record, not merely to use. **Recommendation:** the plan's own font-pinning task should commit `OFL.txt` in the same commit as the font file, per this project's own "code and its tests in the same commit" discipline read one level more broadly (an asset and its licence, together).

## Security Domain

`security_enforcement` is on (ASVS level 1, block on high) per `.planning/config.json` `[VERIFIED: .planning/config.json, read this session]`. This phase adds a new local file-read surface (the SVG document itself, an XML-based format) and no new file-write surface; still local-file-only, no network, no authentication, no session state, consistent with every prior phase's own framing.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture | Yes (narrowly) | `chrys-source-svg` is a new adapter crate outside `chrys-core`, preserving the same compile-time trait-boundary discipline every prior phase already established |
| V5 Input Validation | Yes | `SvgLimits` (file-size cap, canvas-dimension cap) is this phase's own equivalent of `DecodeLimits`/`AnimationLimits`, needed because `usvg::Options` supplies no size-limit field of its own (Pattern 3, Pitfall 3) |
| V5 Input Validation | Yes | An XML-based format (SVG) carries a documented denial-of-service class (entity expansion / "billion laughs") this session could not fully rule out from `roxmltree`'s own documentation alone (Assumptions Log A5); the file-size cap is the defense that does not depend on that claim |
| V6 Cryptography | No | No hashing code is newly written this phase; the existing `chrys_core::hash::rgba8_digest` (SHA-256, already audited) is reused unchanged for the new fixture, the same way every prior phase's own digest lines were produced |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A crafted SVG declaring an enormous `width`/`height`, forcing an unbounded `Pixmap` allocation | Denial of Service | `SvgLimits.max_width`/`max_height`, checked against `Tree::size()` before `Pixmap::new` (Pattern 3) |
| A crafted, deeply nested or entity-heavy SVG source file | Denial of Service | `SvgLimits.max_file_bytes`, checked before `Tree::from_data` is ever called; this defense does not rely on `roxmltree`'s own undocumented entity-expansion behaviour (Assumptions Log A5) |
| A font-loading code path that reaches for the host font database, reintroducing a machine-dependent input into a comparison meant to be portable | Tampering (of the determinism guarantee, not of data) | `system-fonts` Cargo feature disabled workspace-wide (Pattern 1); a static source guard denying `load_system_fonts` (Validation Architecture, DET-05 rows) |

## Sources

### Primary (HIGH confidence)

- `cargo info resvg`, `cargo info usvg`, `cargo info tiny-skia`, `cargo info fontdb`, `cargo info rustybuzz`, `cargo info ttf-parser`, `cargo info roxmltree` — against the live crates.io registry, this session
- `gsd_run query package-legitimacy check --ecosystem crates` — against crates.io metadata, this session, for the same seven packages
- `crates/chrys-source/src/lib.rs`, `crates/chrys-source-raster/src/lib.rs`, `crates/chrys-source-raster/src/decode.rs`, `crates/chrys-source-animation/src/lib.rs`, `crates/chrys-source-animation/src/sniff.rs`, `crates/chrys-core/src/lib.rs`, `crates/chrys-core/src/verdict.rs`, `crates/chrys-cli/src/main.rs`, `Cargo.toml`, `.planning/config.json`, `.github/workflows/determinism.yml`, `rust-toolchain.toml` — all read in full or in relevant part this session, with line ranges quoted above
- [docs.rs/fontdb/0.24.0/fontdb/struct.Database.html](https://docs.rs/fontdb/0.24.0/fontdb/struct.Database.html) — `new()`, `load_font_file`, `load_font_data`, `load_fonts_dir`, `load_system_fonts` signatures and doc comments, fetched verbatim this session
- [docs.rs/usvg/0.48.1/usvg/struct.Tree.html](https://docs.rs/usvg/0.48.1/usvg/struct.Tree.html) — `Tree::size()`, `Tree::from_data`/`from_str` signatures, fetched this session
- [docs.rs/usvg/0.48.1/usvg/struct.Options.html](https://docs.rs/usvg/0.48.1/usvg/struct.Options.html) — `fontdb_mut()`, the `fontdb` field's own doc comment, fetched this session
- [docs.rs/tiny-skia/0.12.0/tiny_skia/struct.Pixmap.html](https://docs.rs/tiny-skia/0.12.0/tiny_skia/struct.Pixmap.html) and its own `src/pixmap.rs.html` source page — `take_demultiplied()` signature and doc comment, premultiplied-alpha struct-level doc comment, fetched this session
- `crates/resvg/src/lib.rs`, `crates/resvg/Cargo.toml`, `crates/resvg/examples/minimal.rs`, `crates/usvg/src/lib.rs`, `crates/usvg/src/parser/options.rs`, `CHANGELOG.md` — all fetched from `github.com/linebender/resvg`'s `master` branch this session, quoted verbatim above
- `crates/resvg/tests/fonts/` directory listing on `github.com/linebender/resvg` — fetched this session, confirming Noto Sans is resvg's own test-suite font family
- [dejavu-fonts.github.io/License.html](https://dejavu-fonts.github.io/License.html) — DejaVu's own licence text, fetched this session, for the Alternatives Considered comparison

### Secondary (MEDIUM confidence)

- WebSearch synthesis on tiny-skia's stated design goal ("must produce exactly the same results as Skia... absent bugs") and its SIMD feature set (SSE2/AVX2/NEON/SIMD128) — corroborated across multiple independent search results this session, not read from one single primary tiny-skia source document directly
- `github.com/notofonts/notofonts.github.io`'s own monthly release tag naming and `github.com/notofonts/NotoSans`'s (archived) `LICENSE` file confirming SIL OFL 1.1 — fetched this session; the exact release-asset internal path was not browsed as a file listing (Assumptions Log A4)
- `roxmltree`'s own README, fetched this session, stating it lacks "complete DTD support" without an explicit statement on custom-entity expansion specifically — treated per this project's own absent-evidence discipline as not itself a verified mitigation (Assumptions Log A5)

### Tertiary (LOW confidence)

- General search-engine synthesis of "resvg glyph outlines rustybuzz ttf-parser no native hinting" — supportive but not independently confirmed against resvg's own glyph-rasterization source file this session (the specific file fetched, `crates/usvg/src/parser/text.rs`, covers text *parsing and styling*, not the outline-to-path conversion itself); treat the "no OS-hinting-engine dependency" framing as directionally correct, architecturally consistent with "resvg doesn't rely on any system libraries," but not itself independently verified this session at the source-code level

## Metadata

**Confidence breakdown:**
- Crate versions and the resvg/usvg/tiny-skia API surface (fontdb construction, `Options.fontdb`, `Tree::size()`, `Pixmap::take_demultiplied()`): HIGH — every signature and doc comment was fetched from `docs.rs` or the crate's own `master`-branch source this session, not recalled from training data
- The `system-fonts` Cargo feature as the compile-time enforcement mechanism for DET-05: HIGH — directly read from `resvg`'s own `Cargo.toml` and `cargo info`'s own feature table this session
- The cross-platform bit-identity claim (tiny-skia SIMD, resvg's own README claim): MEDIUM — vendor-stated, not independently executed against this project's own six-runner matrix this session; this is precisely the gap success criterion 3 exists to close
- Font choice and licence (Noto Sans, SIL OFL 1.1): MEDIUM-HIGH — the licence and the family's use in resvg's own test suite are directly verified; the exact release-asset path is not (Assumptions Log A4)
- SVG-specific denial-of-service protection (entity expansion): MEDIUM-LOW — this session found no explicit statement either confirming or refuting `roxmltree`'s handling of custom entity expansion, so `SvgLimits`'s own file-size cap is recommended as a defense that does not depend on the answer

**Research date:** 2026-09-08
**Valid until:** 14 days for the `resvg`/`usvg`/`tiny-skia` version pins specifically — this crate family released twice in the four weeks before this research and is under active, ongoing stewardship-transition churn; re-run `cargo info resvg` immediately before writing the plan's dependency-installation task. 90 days for the architecture, API-shape, and font-licensing material (grounded in source code and licence text read this session, stable until the code or the licence itself changes).
