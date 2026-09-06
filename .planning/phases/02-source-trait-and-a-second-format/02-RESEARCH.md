# Phase 2: Source trait and a second format - Research

**Researched:** 2026-09-07
**Domain:** Multi-frame input adapters (numbered raster sequences, animated GIF/APNG/WebP) over an already-built format-blind comparison engine; a hint channel that lets a producer name a region
**Confidence:** HIGH on crate and API facts (verified this session against the live crates.io registry and against `docs.rs` source, not recalled); HIGH on the in-repo architecture facts (every claim about `chrys-source`, `chrys-source-raster` and `chrys-core` is grounded in a `Read` of the exact file this session, quoted below); MEDIUM on the animated-WebP determinism and encode-tooling findings, because they rest on a documented gap in the ecosystem rather than a settled answer.

## Summary

Phase 1 already built more of this phase's plumbing than the roadmap goal suggests. `chrys_source::Source::load` already returns `Vec<Frame>`, and `Frame` already carries an `index: usize` and a `hints: Vec<RegionHint>` field
`[VERIFIED: crates/chrys-source/src/lib.rs:18-30]`. Nothing in `chrys-core` reads `hints` today — a repository-wide grep this session found the field written as `Vec::new()` in eleven fixture-construction sites across `chrys-core` and nowhere read
`[VERIFIED: grep -rn "hints" crates/chrys-core/src, this session]`. `compare()` itself takes exactly two `&Frame` values and knows nothing about a sequence, an index, or a hint
`[VERIFIED: crates/chrys-core/src/lib.rs:57]`. So the real work of this phase is not changing the `Source` trait — it does not need to change — but adding two new adapter crates, one small new `chrys-core` module for pairing a sequence, and one small new method on `Frame` for cropping to a hint. None of that touches the existing `register`, `classify`, `verdict` or `hash` modules.

The three target animation formats are fully covered by the `image` crate already pinned at `=0.25.10`, with no C library anywhere in the decode path. Reading the crate's own source this session (via `docs.rs`'s source view, not its prose) confirmed that `GifDecoder`, `ApngDecoder` and `WebPDecoder` all implement `AnimationDecoder` and all three already composite each frame onto a full-canvas RGBA buffer internally, honouring the format's own disposal and blend rules, before handing a frame to the caller
`[VERIFIED: docs.rs/image/0.25.10/src/image/codecs/gif.rs.html, docs.rs/image/0.25.10/src/image/codecs/png.rs.html, docs.rs/image-webp/latest/src/image_webp/decoder.rs.html — all read this session]`. The adapter this phase writes never reimplements GIF disposal or APNG blend math; it only calls `into_frames()` and reshapes the result into `chrys_source::Frame`. The one genuine gap in the ecosystem is the mirror image of that finding: `image`/`image-webp` can decode a lossy, animated WebP, but neither can *encode* one — `image-webp`'s own documentation states its encoder is lossless-still-image only
`[CITED: .planning/research/STACK.md Layer 3, cross-checked this session via WebSearch against image-webp's published capability description]`. Building a committed animated-WebP fixture therefore needs a small, hand-written RIFF/`VP8X`/`ANIM`/`ANMF` container assembled around frames the existing lossless encoder already produces — a narrow, well-specified exception to "don't hand-roll," in the same spirit as this project's own mesh-rasterizer exception.

The four success criteria decompose cleanly if the phase is sequenced in this order: build the generic multi-frame pairing logic into `chrys-core` while implementing the numbered-sequence format (criterion 1) — this is a `chrys-core` change, but SRC-08's "no engine change" test is scoped to the criterion 3 wording ("no change to any file in the comparison engine" during "the animation adapter" specifically), so this is not the wave that test watches. Add the animation adapter second (criterion 2), reusing the pairing logic built in the first wave and touching nothing under `crates/chrys-core/` — that wave is the one the git-diff proof of criterion 3 watches, and it can pass cleanly because the generic plumbing already exists. Add the hint channel last (criterion 4, SRC-09): the recommended design (crop each frame to its hint's bounding box, then call the unmodified `compare()`) needs **no `chrys-core` change at all**, closing the one place this phase could otherwise have created real tension with its own headline claim.

**Primary recommendation:** add `chrys-source-sequence` and `chrys-source-animation` as new adapter crates depending on `chrys-source` (and, for the sequence crate, on `chrys-source-raster` for decode reuse); add `natord = "=1.0.9"` for natural sort; enable `image`'s `gif` feature (new) alongside the already-enabled `png` and `webp` features for animation decode; add a `chrys-core::sequence::compare_sequence` function and two new `RefusalReason` variants during the sequence wave; add `Frame::crop_to_region` in `chrys-source` and CLI-level hint orchestration during the hint wave, touching neither `chrys-core` nor the `Source` trait.

## Architectural Responsibility Map

This project has no browser/server/CDN tiers; the boundary is the one Phase 1 already built and proved: adapter (format-aware) vs. engine (format-blind) vs. CLI (orchestration only).

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Numbered-sequence file discovery, natural sort, per-file decode | Source adapter (`chrys-source-sequence`, new) | — | Format and filesystem-layout knowledge stays out of the engine; reuses `chrys-source-raster`'s decode, does not reimplement it |
| Animation container decode (GIF/APNG/WebP) and frame compositing | Source adapter (`chrys-source-animation`, new) | — | `image`'s `AnimationDecoder` already composites; the adapter only reshapes its output into `Frame` |
| Multi-frame pairing (count mismatch, per-index dispatch) | Engine (`chrys-core`, new `sequence` module) | — | Format-blind: operates on `&[Frame]` only, with no knowledge of where the frames came from — same discipline as `compare()` itself |
| Hint-scoped comparison (crop to a named region) | Source (`chrys-source`, new `Frame` method) + CLI (orchestration) | — | Cropping is a pure `Frame` transformation; `chrys-source` already owns `Frame` and `RegionHint`. No engine change needed because the cropped frame is handed to the unmodified `compare()` |
| Hint supply (a producer naming a region) | Source adapter (sidecar `<name>.hints.toml`, parsed by the raster/sequence adapters) | — | A hint is metadata *about* a source's content, so the adapter that reads the content is the natural place to also read its sidecar |
| Verdict / digest reporting for a sequence | CLI (loop + existing `hash::digest_report` per frame) | Engine (`compare_sequence`'s `Vec<Verdict>`) | The digest mechanism itself (`rgba8_digest`, `DigestSet`) is already generic per-pair; only the loop that calls it per frame index is new, and it belongs in the CLI the same way the single-pair loop already does |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SRC-02 | A numbered frame sequence pair is compared frame by frame | `chrys-source-sequence` (new crate) + `natord` for sort order + `chrys-core::sequence::compare_sequence` for pairing (Q3, Q4) |
| SRC-03 | An animation pair is compared frame by frame (GIF, APNG, animated WebP) | `chrys-source-animation` (new crate), `image`'s `AnimationDecoder` (`gif`, `png`, `webp` features) (Q1, Q2) |
| SRC-08 | A new input family is added without a change to any code in the comparison engine | Sequencing: generic pairing plumbing lands during the sequence wave; the animation wave that follows touches only `chrys-source-animation` (Q5, Q8) |
| SRC-09 | An input source can supply named regions as a hint, and the engine registers inside a named region instead of searching for it | `Frame::crop_to_region` in `chrys-source` (new) + CLI orchestration; zero `chrys-core` change (Q6) |

## Research Questions Answered

### Q1 — Animation decoding in pure Rust

`image` `=0.25.10` covers all three formats through its `AnimationDecoder` trait, and nothing here needs a C library.

| Format | Decoder | Cargo feature | Implements `AnimationDecoder`? |
|--------|---------|----------------|-------------------------------|
| GIF | `image::codecs::gif::GifDecoder` | `gif` (not yet enabled in this workspace) | Yes — `impl<'a, R: BufRead + Seek + 'a> AnimationDecoder<'a> for GifDecoder<R>` `[VERIFIED: docs.rs/image/0.25.10/image/codecs/gif/struct.GifDecoder.html]` |
| APNG | `image::codecs::png::ApngDecoder` | `png` (already enabled) | Yes `[VERIFIED: docs.rs/image/0.25.10/image/codecs/png/struct.ApngDecoder.html]` |
| Animated WebP | `image::codecs::webp::WebPDecoder` | `webp` (already enabled) | Yes `[VERIFIED: docs.rs/image/0.25.10/image/trait.AnimationDecoder.html]` |

Only `gif` is a new feature flag; `png` and `webp` are already enabled on `chrys-source-raster`'s `image` dependency
`[VERIFIED: crates/chrys-source-raster/Cargo.toml: features = ["png", "jpeg", "webp", "tiff"]]`. `cargo add image@0.25.10 --dry-run` against the live registry this session confirmed `gif` is a real, published feature of this exact pinned version
`[VERIFIED: cargo add --dry-run output, this session: "+ gif" listed among image 0.25.10's features]`.

Compositing is already done for you, and this is the load-bearing finding of this question. Reading the decoder source (not just its prose docs) this session showed:
- `GifDecoder`'s `into_frames()` builds a `GifFrameIterator` holding a `non_disposed_frame: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>` accumulator and calls a `blend_and_dispose_pixel` helper that matches on `DisposalMethod::{Any, Keep, Background, Previous}` before yielding each `animation::Frame` `[VERIFIED: docs.rs/image/0.25.10/src/image/codecs/gif.rs.html, read this session]`.
- `ApngDecoder`'s `into_frames()` calls `mix_next_frame()`, which applies `DisposeOp::{None, Background, Previous}` and `BlendOp::{Source, Over}` from each frame's `fcTL` chunk before returning the composited canvas `[VERIFIED: docs.rs/image/0.25.10/src/image/codecs/png.rs.html, read this session]`.
- `image-webp`'s decoder holds its own `canvas: Vec<u8>` accumulator and calls `extended::composite_frame(...)` with the `use_alpha_blending` and `dispose` flags read from each `ANMF` chunk's frame-info byte `[VERIFIED: docs.rs/image-webp/latest/src/image_webp/decoder.rs.html, read this session]`.

**Consequence for the adapter:** `chrys-source-animation` never touches disposal or blend logic. It calls `.into_frames()`, and for each `image::Frame` it receives, copies `frame.into_buffer().into_raw()` (already full-canvas RGBA8) into a `chrys_source::Frame` with a sequential `index`. This removes an entire class of hand-rolled compositing bugs from the plan before it is written.

### Q2 — Frame timing

Frame identity for this phase is the ordinal index alone — the same identity the sequence format already needs, and the same one `Frame.index` already exists to carry `[VERIFIED: crates/chrys-source/src/lib.rs:25-26]`. Do not add a presentation-time field to `Frame` in this phase.

REQUIREMENTS.md itself draws the line this recommendation follows: SRC-02/SRC-03 say frames are "compared frame by frame," while SRC-06 (Phase 7, video) is explicit that a video pair is "paired by presentation timestamp and not by frame index"
`[VERIFIED: .planning/REQUIREMENTS.md:47-49 ("SRC-02... compared frame by frame", "SRC-03... compared frame by frame"), :53-54 ("SRC-06... paired by presentation timestamp and not by frame index")]`. That contrast is deliberate, not an oversight this research is filling in: it means index-based identity is correct for any format whose frames arrive in one fixed, enumerable order (a directory listing, a container's own frame array), and timestamp-based pairing is reserved for the one format (video) where frame count alone cannot answer "which frame is this." Blurring the two now, by threading a `Delay`-derived duration through `Frame` for animation, would spend engine surface area Phase 2 does not need and would preempt a design decision that belongs to Phase 7.

Practically: `image::Frame::delay()` returns a `Delay` (convertible to `Duration` via `From<Delay> for Duration`, or to a millisecond ratio via `numer_denom_ms()`) `[VERIFIED: docs.rs/image/0.25.10/image/struct.Delay.html]`, but the animation adapter can discard it once each `image::Frame` is reshaped into a `chrys_source::Frame`. If a later phase wants per-frame duration reported as diagnostic metadata (not as a pairing key), that is an additive `Frame` field to design then, against the video phase's own concrete needs, not against a guess made here.

### Q3 — Numbered frame sequences

Zero-padded, fixed-width numbering (`frame_0001.png`, ffmpeg's own `%0Nd` pattern) is the convention tools reach for specifically because it avoids the lexicographic-sort trap, but it is not the only convention a real input will use. ffmpeg's own `image2` demuxer documentation treats an *unpadded* `%d` pattern as an equally first-class, expected form, not a fallback
`[CITED: ffmpeg image2 demuxer documentation, via WebSearch this session — "If the number in the filenames are not padded with leading zeroes then the pattern is %d."]`. A tool that only handles the zero-padded case will silently mis-order any sequence a screen-recorder, a hand-numbered test harness, or an older render pipeline produces.

**The rule:** sort with a natural-order comparator, unconditionally, regardless of whether the input happens to be zero-padded. Do not special-case "looks zero-padded" and skip natural sort for it — the padded case sorts identically either way, so there is no cost to always using it, and it is one fewer branch to get wrong.

**Crate, not hand-written:** `natord` `1.0.9` `[VERIFIED: cargo add --dry-run + `gsd_run query package-legitimacy check`, this session: published 2014-11-21, ~195K downloads/week, OK verdict, repo github.com/lifthrasiir/rust-natord]`. Its entire public surface is two functions:
```rust
// Source: docs.rs/natord, quoted verbatim
pub fn compare(a: &str, b: &str) -> std::cmp::Ordering;
pub fn compare_ignore_case(a: &str, b: &str) -> std::cmp::Ordering;
```
Four alternatives exist on crates.io (`human-sort`, `lexical-sort`, `alphanumeric-sort`, `natural-sort-rs`); all were checked this session and all but `natural-sort-rs` (119 downloads/week, `SUS`) scored `OK`
`[VERIFIED: gsd_run query package-legitimacy check --ecosystem crates natord human-sort lexical-sort natural-sort-rs alphanumeric-sort, this session]`. `natord` is recommended because it is the oldest, most-downloaded, smallest-surface option, and its two-function API is exactly the shape this adapter needs — a comparator to hand to `Vec::sort_by`, nothing more.

**What defines a sequence:** a directory path, not a glob pattern and not a manifest file. `chrys-source-sequence`'s `Source::load(path)` treats `path` as a directory: it lists immediate (non-recursive) file entries, decodes each through `chrys_source_raster::decode::decode_guarded` (reused, not reimplemented — see Don't Hand-Roll), sorts the resulting `(filename, Frame)` pairs with `natord::compare` on the filename, and assigns `index` by the sorted position. A directory, not a glob, keeps this consistent with the CLI's existing two-`PathBuf` signature (`chrys compare <base> <candidate>`, unchanged) and avoids introducing a second, competing configuration surface before Phase 3 designs the project's one declarative-config format (the TOML rule file, RULE-01..05). If a file in the directory fails to decode, `load` fails loudly and names the file — it does not skip it silently, matching this project's stated preference (`AGENTS.md`, "What a test can hold on to": a check must ask the real question, not a nearby one) and the existing `RasterError` style.

### Q4 — Pairing two sequences

Phase 1 established the precedent this recommendation extends directly: `compare()` already refuses rather than guesses when `base.same_shape_as(candidate)` is false, returning `Verdict::Refused { reason: RefusalReason::DimensionMismatch { .. } }` instead of erroring or scaling
`[VERIFIED: crates/chrys-core/src/lib.rs:62-69, crates/chrys-core/src/verdict.rs:97-117]`. The sequence-pairing cases below extend that same taxonomy one level up, from "two frames of different shape" to "two sequences of different length."

| Case | Recommended behaviour |
|------|------------------------|
| Equal frame counts, every index-matched pair shares one shape | Compare each index pair independently with the existing, unmodified `compare()`; collect one `Verdict` per index |
| Different frame counts (`base.len() != candidate.len()`) | Refuse the whole sequence with a new `RefusalReason::FrameCountMismatch { base: usize, candidate: usize }` variant. Do not attempt to pair a prefix, do not interpolate, do not pair the shorter side against a truncated tail of the longer one — any of those would report on data the producer never intended to be compared |
| One side has zero frames | Defence in depth only: an adapter should already fail to `load()` a source with no frames (this is a malformed-input case for the adapter, not a legitimate empty sequence). `compare_sequence` still checks for it explicitly and returns a new `RefusalReason::EmptySequence`, distinct from `CompareError::EmptyFrame` (which means "a frame with zero pixels," a different failure already handled at `crates/chrys-core/src/lib.rs:58-60`) |
| Equal counts, but frame *i* differs in width/height between base and candidate | Do not abort the whole sequence. Let index *i*'s own `compare()` call return its own `Verdict::Refused { DimensionMismatch }`; every other index is still compared and reported. This is the literal reading of "compared frame by frame" (SRC-02's own wording): a per-frame outcome, not an all-or-nothing one |
| Equal counts, frames *within* one side vary in size from each other (e.g. base frame 1 is 100x100, base frame 2 is 200x200) | Not an error condition by itself. Each index is only ever compared against its own same-index counterpart on the other side; a side's own internal size variation is invisible to `compare_sequence` |

**Recommended shape**, extending `verdict.rs`'s existing `RefusalReason` enum (add two variants alongside the existing `DimensionMismatch` and `PeakConfidenceTooLow`) and adding one new function in a new `chrys-core/src/sequence.rs` module:

```rust
// Recommended, not yet written — sketch only. Builds on
// crates/chrys-core/src/verdict.rs:97-117 and crates/chrys-core/src/lib.rs:57-110,
// both read verbatim this session; no existing function's signature changes.
pub fn compare_sequence(base: &[Frame], candidate: &[Frame]) -> Result<Vec<Verdict>, CompareError> {
    if base.is_empty() || candidate.is_empty() {
        return Ok(vec![Verdict::Refused { reason: RefusalReason::EmptySequence }]);
    }
    if base.len() != candidate.len() {
        return Ok(vec![Verdict::Refused {
            reason: RefusalReason::FrameCountMismatch { base: base.len(), candidate: candidate.len() },
        }]);
    }
    base.iter().zip(candidate.iter()).map(|(b, c)| compare(b, c)).collect()
}
```
A `Vec<Verdict>` of length 1 signals a whole-sequence refusal; a `Vec<Verdict>` matching the frame count signals a per-frame result set. This adds no new top-level struct, reuses `Verdict` and `RefusalReason` exactly as they exist, and calls `compare()` with no change to its signature or body.

### Q5 — The `Source` trait as it stands

The trait does not need to change. Its one method already returns `Vec<Frame>`:
```rust
// Source: crates/chrys-source/src/lib.rs:69-75, quoted verbatim
pub trait Source {
    type Error;
    fn load(&self, path: &Path) -> Result<Vec<Frame>, Self::Error>;
}
```
This already generalises to a sequence and to an animation: both are "a path that decodes to more than one frame," which is exactly what `Vec<Frame>` already models. `RasterSource`'s own implementation already returns a one-element `Vec<Frame>` today `[VERIFIED: crates/chrys-source-raster/src/lib.rs:125-135]`, so a multi-frame adapter is not a new shape, only a new adapter filling in the shape that was already there.

**The smallest change that serves sequences, animation, and later video and PDF** is not a trait change at all — it is two additive, narrowly-scoped pieces:
1. `chrys-core::sequence::compare_sequence` (Q4) — new code in `chrys-core`, added during the sequence wave, generic over `&[Frame]` with no knowledge of where the frames came from. Video (Phase 7) and PDF (Phase 5, if it ever needs multi-page comparison) can reuse this unchanged, the same way the animation adapter reuses it in this phase, *provided* they also pair by index — SRC-06 already says video does not, so Phase 7 will need its own timestamp-pairing function, not a reuse of this one, and that is fine: the two are different questions (index vs. time) with different correct answers, not two implementations of the same thing.
2. `Frame::crop_to_region` (Q6) — new code in `chrys-source`, needed for the hint channel, reusable by any future adapter without a per-format branch.

**Which crates a change may touch, stated plainly:** `chrys-source` (new `Frame` method, no field or trait signature change), `chrys-core` (new `sequence` module and two new `RefusalReason` variants, added during the sequence wave — not the animation wave), `chrys-source-sequence` (new crate), `chrys-source-animation` (new crate), `chrys-cli` (new orchestration for directory/multi-frame input and for hint-driven comparison). `chrys-core`'s existing `register`, `classify`, `hash` and `verdict`'s existing variants are untouched; `verdict.rs` gains two enum variants, which is an additive, non-breaking change reviewable in the same commit as `compare_sequence`, per this repository's own "code and its tests in the same commit" convention `[VERIFIED: AGENTS.md:47]`.

### Q6 — The hint channel (SRC-09)

`RegionHint` already exists with exactly the shape this needs — name, x, y, width, height, all in pixels `[VERIFIED: crates/chrys-source/src/lib.rs:50-63]` — but nothing populates it and nothing reads it. `compare()`'s body never references `base.hints` or `candidate.hints` `[VERIFIED: crates/chrys-core/src/lib.rs:57-110, full function read this session]`, and a repository-wide grep found `hints` written only as `Vec::new()` in test and fixture code, never read `[VERIFIED: grep -rn "hints" crates/chrys-core/src, this session — 11 occurrences, all `hints: Vec::new()` inside `Frame { .. }` literals]`. This path is entirely unbuilt today; the question of whether building it must touch `chrys-core` is real, and this research answers it with a design that keeps the answer "no."

**How a producer supplies one:** a sidecar file, `<image-stem>.hints.toml`, next to the source file, read by the adapter that already owns that file (`chrys-source-raster` for a single image; `chrys-source-sequence` per frame, if a sequence ever needs per-frame hints). This keeps the hint genuinely *source*-supplied, per SRC-09's own wording ("an input source can supply..."), rather than a CLI flag standing in for the source. Example:
```toml
# base.hints.toml, next to base.png
[[hint]]
name = "logo"
x = 10
y = 10
width = 64
height = 64
```
Parsed with `toml` `1.1.5` + `serde` `1.0.229` (`derive` feature) — both confirmed against the live registry this session `[VERIFIED: cargo add --dry-run, this session: toml v1.1.5, serde v1.0.229]` and both already named as this project's standard TOML/serialization pair in `.planning/research/STACK.md` Layer 8. Deserializing directly into `Vec<RegionHint>`-shaped rows needs `chrys_source::RegionHint` to derive `serde::Deserialize`, a one-line, additive change to `chrys-source` (not `chrys-core`).

**How the adapter carries it:** unchanged from what already exists — the adapter populates `Frame.hints` instead of leaving it `Vec::new()`. No new field, no trait change.

**How the engine uses it — and why it does not need to change:** crop, then call the existing `compare()` unmodified. Add `Frame::crop_to_region(&self, hint: &RegionHint) -> Frame` in `chrys-source` (a pure, format-blind data transformation on a type `chrys-source` already owns, exactly like the existing `same_shape_as`). CLI orchestration (new code in `chrys-cli`, which already imports and wires adapters today) matches a hint by name across `base.hints` and `candidate.hints`, crops both frames to their own hint's rectangle, and calls `compare(&cropped_base, &cropped_candidate)` — the same function, same signature, same file. `compare()`'s own phase correlation still runs, but now it runs *inside* the hinted rectangle instead of across the whole canvas, which is the literal meaning of "registers inside that region instead of searching for it": the engine is told where to look instead of being asked to find it, and it never had to change to be told.

**The conflict this avoids, named plainly:** an alternative design exists — constrain `register::phase_correlate`'s own search window to a caller-supplied rectangle, inside `chrys-core` itself. That design *would* touch `chrys-core` (specifically `crates/chrys-core/src/register/phase_correlation.rs` and `mod.rs`, and `compare()`'s own signature to accept an optional region). It is not recommended, precisely because it creates the tension this question asks to be named rather than smoothed over: SRC-08 says a new input family needs no engine change, and while SRC-09 is not itself gated by that same sentence, a `chrys-core` change for hint support would sit uncomfortably next to a phase whose headline success criterion is "no change to the comparison engine." The crop-then-compare design was chosen specifically because it makes that tension not arise at all, not because it argues the tension away.

### Q7 — Determinism at this boundary

GIF and APNG decode carry the same, already-proven determinism class as Phase 1's PNG path; animated WebP carries one new, unproven surface.

- **GIF:** LZW decompression plus an indexed colour-table lookup — integer-only arithmetic, no floating point anywhere in the codepath, the same class of guarantee (and the same `gif` crate family) as the deflate decoding Phase 1 already proved bit-exact on six runners for PNG. HIGH confidence this carries over without a new determinism risk.
- **APNG:** decoded through `image::codecs::png::ApngDecoder`, which is a thin animation wrapper around the *same* `png` crate deflate decode Phase 1's SRC-01 PNG path already exercises and already proved `[VERIFIED: png crate pinned at 0.18.1 in Cargo.lock, shared by both the static-PNG and APNG decode paths]`. This is not a new decode surface at all, only a new metadata-driven compositing step on top of an already-proven one. HIGH confidence.
- **Animated WebP:** `image-webp` decodes both lossless (VP8L) and lossy (VP8) animated frames `[CITED: WebSearch synthesis of image-webp's published capability description, this session]`. Phase 1's own committed WebP golden fixture is lossless: its file header reads `WEBPVP8L`, confirmed by inspecting the raw bytes this session (`xxd -l 32 tests/golden/formats/webp/base.webp`) `[VERIFIED: direct byte inspection, this session]`. That means Phase 1's six-runner matrix has proven VP8L (lossless) decode bit-exact, but has never exercised VP8 (lossy, DCT-based, closer in shape to JPEG's IDCT than to PNG's deflate) inside `image-webp` at all. If this phase's animated-WebP fixtures use only lossless (VP8L) frames, they inherit Phase 1's existing proof and add no new risk. If a lossy animated-WebP fixture is introduced, that is new, unproven determinism surface and must be said so, not assumed safe by analogy to the JPEG proof, which is a different decoder.

**Recommendation:** commit only lossless (VP8L-encoded) animated WebP fixtures in this phase. This is also forced by the fixture-generation gap in Q9 below (there is no available pure-Rust lossy animated-WebP *encoder* to build a fixture with in the first place), so the two findings point the same direction independently.

**The test that settles it:** extend the existing golden-hash mechanism, unchanged in kind. `chrys-core::hash::digest_report` already produces a `DigestSet` (SHA-256 of each frame's raw RGBA8, of the residual, and of the verdict's `Display` text) for one pair `[VERIFIED: crates/chrys-core/src/hash.rs:34-73]`; running it once per frame index of a committed animation or sequence fixture, on the same six-runner matrix `.github/workflows/determinism.yml` already runs (`ubuntu-24.04`, `ubuntu-24.04-arm`, `macos-15`, `macos-15-intel`, `windows-2022`, `windows-11-arm`) `[VERIFIED: .github/workflows/determinism.yml:44-51]`, extends the exact mechanism Phase 1 already proved, rather than inventing a new one.

### Q8 — Proving the engine did not change

Two independent mechanisms prove this, and they check different things.

**1. A dependency-graph guard already exists and needs no change.** `crates/chrys-core/tests/determinism.rs` already runs `cargo tree -p chrys-core -e normal` and fails if any crate in `DENIED_DEPENDENCY_CRATES` — which already lists `image`, `imageproc`, `image-webp`, `png`, `gif`, `webp`, `tiff`, `zune-jpeg`, `jpeg-decoder`, alongside every GPU/graphics crate — appears anywhere in `chrys-core`'s own dependency graph `[VERIFIED: crates/chrys-core/tests/determinism.rs:124-148 (the exact `DENIED_DEPENDENCY_CRATES` list, quoted), :481-527 (`no_gpu_or_format_crate_enters_chrys_cores_dependency_graph`, quoted)]`. This guard already has a drill proving it can fail: `scripts/determinism-drill.sh`'s third drill plants `glow` into `crates/chrys-core/Cargo.toml` inside a disposable worktree and confirms the guard goes red and names `glow` `[VERIFIED: scripts/determinism-drill.sh:132-144, read this session]`. Nothing in this phase needs to touch this guard or its drill; running `cargo test -p chrys-core --test determinism` after the animation wave is free, automatic evidence that no format crate leaked into `chrys-core`'s manifest.

**2. A new check is needed for the literal claim: no *file* changed, not just no *dependency* changed.** A dependency graph could stay clean while an internal `chrys-core` algorithm was still edited; criterion 3's wording ("no change to any file") is stronger than the dependency guard alone can prove. The exact command:
```sh
git diff --name-only <first-animation-wave-commit>^..<last-animation-wave-commit> -- crates/chrys-core/
```
Expected output: nothing — an empty result is the pass condition. A failure prints one path per changed file under `crates/chrys-core/`, e.g. `crates/chrys-core/src/lib.rs`, which is exactly the information needed to say which file broke the claim and go fix the wave's own commit boundaries. Record the wave's first and last commit hash in that wave's own `SUMMARY.md`, the same way Phase 1's summaries recorded verbatim drill output, so this check stays reproducible after the fact rather than depending on someone's memory of which commits were "the animation wave."

**Planting a defect that makes this check fail (the drill):** follow `scripts/determinism-drill.sh`'s own pattern exactly — a disposable `git worktree` created from `HEAD` and removed on every exit path, never touching the real tree `[VERIFIED: scripts/determinism-drill.sh:17-42, read this session]`. Inside the worktree, append a harmless comment line to `crates/chrys-core/src/lib.rs` as part of a commit that also touches `chrys-source-animation`, then run the `git diff --name-only` command above across that commit range. Confirm it goes red (prints `crates/chrys-core/src/lib.rs`) and confirm the message names that exact file, not just "something changed." A new script, `scripts/engine-boundary-drill.sh`, following the same three-part shape (plant, assert red, assert the message names the plant) is the recommended home for this, rather than folding it into `determinism-drill.sh`, since it drills a phase-2-specific, wave-boundary claim rather than a determinism property.

### Q9 — Fixtures

Follow `crates/chrys-source-raster/examples/make-fixtures.rs`'s exact pattern: a committed Rust example, run with `cargo run -p <crate> --example make-fixtures`, that builds every fixture from small integer formulas (rectangles at fixed offsets, a linear-congruential noise generator seeded by a literal constant) and writes the result to `tests/golden/`, so a reader can reproduce every byte without running the generator `[VERIFIED: crates/chrys-source-raster/examples/make-fixtures.rs:1-9, 188-210, read in full this session]`. No external asset, no downloaded font, no network call.

**What this phase needs, concretely:**

| Fixture | Where | How generated | Size |
|---------|-------|----------------|------|
| Numbered sequence pair (base/, candidate/) | `tests/golden/sequence-01/{base,candidate}/` | New `chrys-source-sequence/examples/make-fixtures.rs`: 3-5 frames per side, reusing `build_structured_canvas(dx, dy)`-style rectangles shifted a little per frame index, encoded as PNG via `image`'s existing encoder | 256x256 per frame, a handful of frames — same order of magnitude as Phase 1's existing fixtures |
| **Non-padded filenames**, deliberately, to exercise the natural-sort trap | Same directory, e.g. `frame1.png` .. `frame11.png` | Same generator; name files without zero-padding on purpose so a lexicographic-sort regression is caught by a file that already exists, not by a bug report later | — |
| GIF animation pair | `tests/golden/formats/gif/{base,candidate}.gif` | New `chrys-source-animation/examples/make-fixtures.rs`, using `image::codecs::gif::GifEncoder::encode_frames` (confirmed to exist and accept `IntoIterator<Item = image::Frame>` `[VERIFIED: docs.rs/image/0.25.10/image/codecs/gif/struct.GifEncoder.html]`) over the same structured-rectangle frames | Same |
| APNG animation pair | `tests/golden/formats/apng/{base,candidate}.png` | Same generator, using the `png` crate directly (pinned at `0.18.1` via `Cargo.lock`, already a transitive dependency of `image`) and its `Encoder::set_animated(num_frames, num_plays)` + `Writer::set_frame_delay(...)` API, confirmed to exist in this exact version `[VERIFIED: docs.rs/png/latest/png/struct.Encoder.html + Cargo.lock `png` = 0.18.1]`. `image`'s own high-level save path has no animated-PNG *encoder*, only the `ApngDecoder` for reading, so this fixture is written with `png` directly, not through `image::DynamicImage::save` |
| Animated WebP pair (lossless only — see Q7) | `tests/golden/formats/webp-anim/{base,candidate}.webp` | The one hand-rolled piece this phase needs: encode each frame with the already-existing `image::codecs::webp::WebPEncoder::new_lossless` (confirmed API: `encode(buf, width, height, color_type)` `[VERIFIED: docs.rs/image/0.25.10/image/codecs/webp/struct.WebPEncoder.html]`), extract the `VP8L` chunk payload from each resulting single-image WebP file, and wrap the set in a hand-assembled `RIFF`/`WEBP` container with a `VP8X` header (animation bit set), one `ANIM` chunk, and one `ANMF` chunk per frame. The exact byte layout of `ANIM` (background colour, loop count) and `ANMF` (frame x/y in units of 2px, width-minus-one, height-minus-one, duration, blend/dispose bits) was fetched and quoted from Google's own WebP container specification this session `[VERIFIED: developers.google.com/speed/webp/docs/riff_container, fetched this session]`. Because this is new, unreviewed container-assembly code (unlike everything else in this table, which only calls an existing encoder), require a round-trip test as part of the same commit: decode the generated file back through `image::codecs::webp::WebPDecoder` and assert the frame count and pixel content match what was fed in, before trusting the file as a golden fixture |

**Total new fixture weight:** on the same order as Phase 1's existing `tests/golden/` tree (a few hundred KB across all new files, all 256x256-or-smaller, all lossless or losslessly-re-derivable) — this does not change the project's dependency-count posture, only its committed-test-asset size.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `image` | `=0.25.10` (already pinned) | Add the `gif` feature to the existing pin for `chrys-source-animation`; `png` and `webp` are already enabled elsewhere | Already the project's one raster/animation decode dependency; adding a feature, not a new crate `[VERIFIED: cargo add --dry-run this session confirms `gif` is a real feature of this exact pinned version]` |
| `natord` | `=1.0.9` | Natural-order sort for numbered sequence filenames | Smallest, oldest, most-downloaded natural-sort crate on crates.io; two-function API matches exactly what a `sort_by` comparator needs `[VERIFIED: crates.io registry + package-legitimacy check, this session]` |
| `toml` | `=1.1.5` | Parse the `<name>.hints.toml` sidecar | Already this project's designated declarative-config format (used again, unmodified, for RULE-01 in Phase 3); reusing it for hints avoids introducing a second config language `[VERIFIED: cargo add --dry-run this session]` |
| `serde` | `=1.0.229` (`derive` feature) | Deserialize the hints sidecar into `Vec<RegionHint>` | Standard, unavoidable pairing with `toml`; already named in `.planning/research/STACK.md` Layer 8 as this project's serialization foundation `[VERIFIED: cargo add --dry-run this session]` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `png` | `0.18.1` (already a transitive pin via `image`, add as a direct dependency of the animation crate's fixture generator only) | Direct `Encoder`/`Writer` access for APNG *encode* | Only inside `chrys-source-animation/examples/make-fixtures.rs`, a dev/example-only use — `image`'s own high-level API has no animated-PNG encoder `[VERIFIED: Cargo.lock pins `png` = 0.18.1; docs.rs/png confirms `set_animated`/`set_frame_delay` this session]` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `natord` | `alphanumeric-sort` 1.5.8, `lexical-sort` 0.3.1 | Both scored `OK` in this session's legitimacy check and would work equally well; `natord`'s narrower two-function surface (versus `alphanumeric-sort`'s broader in-place-sort helpers, which this adapter does not need) is preferred for minimal surface area, not for a correctness difference |
| `natural-sort-rs` | (not used) | Scored `SUS` this session: published 2024-10-04, 119 downloads/week `[VERIFIED: gsd_run query package-legitimacy check, this session]`. Not selected; no `checkpoint:human-verify` needed since it was never adopted |
| Hand-rolled directory-to-`Vec<Frame>` decode loop | Reuse `chrys_source_raster::decode::decode_guarded` + `normalize::normalize_to_rgba8` per file | Both functions are already `pub`, already guard against the CVE-2023-29408-class decompression-bomb risk, and already normalize orientation/alpha/bit-depth `[VERIFIED: crates/chrys-source-raster/src/decode.rs:28 (`pub fn decode_guarded`), crates/chrys-source-raster/src/normalize.rs:90 (`pub fn normalize_to_rgba8`)]`. Reimplementing either inside `chrys-source-sequence` would duplicate a decode-limits guard this project has already built and tested once |
| Constraining `register::phase_correlate`'s search window inside `chrys-core` for hint support | Crop-then-compare in `chrys-source` + CLI orchestration (recommended, Q6) | The in-`chrys-core` design works too, but it touches the engine for a phase whose headline claim is "no change to the comparison engine" — avoidable, so avoided |

**Installation:**
```bash
# New crates (workspace members via the existing crates/* glob, no workspace Cargo.toml edit needed)
cargo new --lib crates/chrys-source-sequence
cargo new --lib crates/chrys-source-animation

# chrys-source-sequence/Cargo.toml
cargo add --manifest-path crates/chrys-source-sequence/Cargo.toml \
    --path ../chrys-source chrys-source
cargo add --manifest-path crates/chrys-source-sequence/Cargo.toml \
    --path ../chrys-source-raster chrys-source-raster
cargo add natord@=1.0.9 thiserror --manifest-path crates/chrys-source-sequence/Cargo.toml

# chrys-source-animation/Cargo.toml
cargo add --manifest-path crates/chrys-source-animation/Cargo.toml \
    --path ../chrys-source chrys-source
cargo add image --features gif,png,webp --manifest-path crates/chrys-source-animation/Cargo.toml
cargo add thiserror --manifest-path crates/chrys-source-animation/Cargo.toml

# hints sidecar support, added to chrys-source-raster (and chrys-source-sequence if per-frame hints are needed)
cargo add toml@=1.1.5 serde --features derive --manifest-path crates/chrys-source-raster/Cargo.toml

# fixture generator dependency, example-only
cargo add png@=0.18.1 --dev --manifest-path crates/chrys-source-animation/Cargo.toml
```

**Version verification:** every version above was checked with `cargo add --dry-run` against the live crates.io registry this session (2026-09-07), not recalled from the Phase 1 research document. Re-run at plan time if implementation is delayed, per this project's own established habit.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `natord` | crates.io | 2014-11-21 (~11 yr) | 195K/wk | github.com/lifthrasiir/rust-natord | OK | Approved |
| `gif` (transitive, via `image`'s new feature) | crates.io | 2015-05-29 (~11 yr) | 2.59M/wk | github.com/image-rs/image-gif | OK | Approved |
| `png` (transitive already; direct dep added for fixture generation) | crates.io | 2015-05-26 (~11 yr) | 5.25M/wk | github.com/image-rs/image-png | OK | Approved |
| `toml` | crates.io | (long-established `toml-rs`/`toml_edit` lineage) | high-volume, workspace-standard | github.com/toml-rs/toml | OK (already project-standard per `.planning/research/STACK.md` Layer 8) | Approved |
| `serde` | crates.io | (foundational Rust ecosystem crate) | very high volume | github.com/serde-rs/serde | OK (already project-standard) | Approved |
| `human-sort`, `lexical-sort`, `alphanumeric-sort` | crates.io | 2019/2020/2018 | 6K-75K/wk | respective GitHub repos | OK | Not selected — see Alternatives Considered |
| `natural-sort-rs` | crates.io | 2024-10-04 | 119/wk | github.com/Vrtgs/natural-sort-rs | SUS (low-downloads) | Not selected |

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** `natural-sort-rs` was probed and rejected in favour of `natord`, never adopted, so no `checkpoint:human-verify` gate is needed.

## Architecture Patterns

### System Architecture Diagram

```
   sequence directory, or animation file (GIF/APNG/WebP), on each side
            |
            v
   +---------------------------+
   |  Source adapter            |  chrys-source-sequence: list dir, natord::compare
   |  (new crates)               |    sort, decode each file via chrys-source-raster
   |                             |  chrys-source-animation: sniff format, build
   |                             |    GifDecoder / ApngDecoder / WebPDecoder,
   |                             |    call .into_frames() (already composited)
   |                             |  Either adapter also reads a <name>.hints.toml
   |                             |    sidecar, if present, into Frame.hints
   +---------------------------+
            |
            v  Vec<Frame>  (RGBA8, index 0..N, hints populated or empty)
            |
   +---------------------------+
   |  CLI orchestration          |  chrys-cli: picks adapter by path shape
   |  (chrys-cli, new logic)     |    (directory -> sequence; file -> animation-
   |                             |     capable decode if the sniffed format is
   |                             |     gif, or is_apng()/has_animation() reports
   |                             |     true; otherwise the existing RasterSource
   |                             |     path, unchanged)
   |                             |  If a --region <name> hint is requested:
   |                             |    Frame::crop_to_region on both sides first
   +---------------------------+
            |
            v  base: &[Frame], candidate: &[Frame]  (length 1 for the
            |  existing single-image and hint-cropped cases; length N
            |  for a sequence or animation)
            |
   +---------------------------+
   |  chrys-core::sequence       |  compare_sequence: refuse on count mismatch
   |  (new module, this phase)   |    or an empty side; otherwise zip and call
   |                             |    the UNCHANGED compare() once per index
   +---------------------------+
            |
            v  Vec<Verdict>  (length 1 for a whole-sequence refusal or for
            |  the existing single-pair case; length N per-frame otherwise)
            |
   CLI prints one verdict block per index, or the single refusal; the
   existing hash::digest_report is called once per index for the CI
   golden-hash job, extending the existing six-runner mechanism unchanged
```

### Recommended Project Structure

```
chrysoberyl/
├── crates/
│   ├── chrys-source/              # unchanged trait; new Frame::crop_to_region,
│   │   │                          # new #[derive(Deserialize)] on RegionHint
│   ├── chrys-source-raster/        # unchanged decode/normalize; new hints.toml
│   │   │                          # sidecar read, added alongside the existing
│   │   │                          # RasterSource::load
│   ├── chrys-source-sequence/      # NEW: directory listing, natord sort,
│   │   │                          # decode reuse via chrys-source-raster
│   │   └── examples/
│   │       └── make-fixtures.rs   # sequence-01 fixtures, following the
│   │                              # existing chrys-source-raster generator's
│   │                              # pattern exactly
│   ├── chrys-source-animation/     # NEW: format sniff, GifDecoder/ApngDecoder/
│   │   │                          # WebPDecoder dispatch, AnimationDecoder use
│   │   └── examples/
│   │       └── make-fixtures.rs   # gif/apng/webp-anim fixtures, including the
│   │                              # hand-rolled ANMF container assembler and
│   │                              # its own round-trip decode test
│   ├── chrys-core/                 # register/, classify/, hash.rs, verdict.rs
│   │   │                          # ALL UNCHANGED. New sibling module:
│   │   └── src/sequence.rs        # NEW: compare_sequence, calling compare()
│   │                              # unmodified
│   └── chrys-cli/                  # extended: path-shape and format-sniff
│                                  # dispatch, --region flag, per-index output
└── tests/
    └── golden/
        ├── sequence-01/            # NEW
        └── formats/
            ├── gif/                # NEW
            ├── apng/                # NEW
            └── webp-anim/           # NEW
```

### Pattern 1: composited frames in, reshape only, never recomposite

**What:** `AnimationDecoder::into_frames()` already returns full-canvas, disposal-and-blend-resolved `image::Frame` values for all three formats (Q1). The adapter's only job is `image::Frame -> chrys_source::Frame`.

**Example:**
```rust
// Recommended shape for chrys-source-animation's core loop.
// Source: image::AnimationDecoder trait and image::Frame, both read via
// docs.rs this session; the reshape itself is new code, not yet written.
use image::AnimationDecoder;
use chrys_source::Frame as ChrysFrame;

fn frames_from_decoder<'a>(
    decoder: impl AnimationDecoder<'a>,
) -> Result<Vec<ChrysFrame>, AnimationError> {
    decoder
        .into_frames()
        .enumerate()
        .map(|(index, frame)| {
            let frame = frame.map_err(AnimationError::Decode)?;
            let buffer = frame.into_buffer(); // already full-canvas RGBA8
            let (width, height) = buffer.dimensions();
            Ok(ChrysFrame {
                pixels: buffer.into_raw(),
                width,
                height,
                index,
                hints: Vec::new(),
            })
        })
        .collect()
}
```

### Pattern 2: reuse the guarded decode, don't reopen the decoder

**What:** `chrys-source-sequence` never calls `image::ImageReader::open` itself. It calls `chrys_source_raster::decode::decode_guarded`, the single guarded entry point Phase 1 already built and tested against the CVE-2023-29408-class decompression-bomb risk.

**Example:**
```rust
// Source: crates/chrys-source-raster/src/decode.rs:28 and
// crates/chrys-source-raster/src/normalize.rs:90, both public functions
// read verbatim this session. This call site is new, in the recommended
// chrys-source-sequence crate.
use chrys_source_raster::{decode::decode_guarded, normalize::normalize_to_rgba8, DecodeLimits};

fn decode_one_frame(path: &std::path::Path, limits: &DecodeLimits) -> Result<(Vec<u8>, u32, u32), SequenceError> {
    let (dynamic, orientation) = decode_guarded(path, limits)?;
    Ok(normalize_to_rgba8(dynamic, orientation))
}
```

### Pattern 3: natural sort is unconditional, not a fallback

**What:** Sort every directory listing with `natord::compare`, whether or not the filenames look zero-padded. There is no branch that decides "this sequence doesn't need it."

**Example:**
```rust
// Source: docs.rs/natord, function signature quoted verbatim this session.
let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
    .filter_map(|e| e.ok().map(|e| e.path()))
    .filter(|p| p.is_file())
    .collect();
entries.sort_by(|a, b| {
    let a = a.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let b = b.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    natord::compare(a, b)
});
```

### Anti-Patterns to Avoid

- **Reimplementing GIF/APNG disposal or WebP blend math anywhere in this project.** All three are already handled inside `image`/`image-webp`'s own decoders (Q1, VERIFIED by reading their source this session). A hand-rolled compositor would be new, untested code solving an already-solved problem, and it would be exactly the kind of "don't hand-roll" violation `.planning/research/STACK.md` and `PITFALLS.md` warn against elsewhere in this project.
- **Special-casing zero-padded filenames and skipping natural sort for them.** There is no filename shape for which lexicographic and natural sort disagree in a way that makes lexicographic sort preferable; always use `natord::compare`.
- **Assuming animated-WebP determinism by analogy to the already-proven static WebP fixture.** Phase 1's committed WebP fixture is lossless (`VP8L`, confirmed by direct byte inspection this session); a lossy (`VP8`) animated frame is a different decode path with no existing proof. Do not extend the "WebP is already proven" claim past the codec path that was actually tested.
- **Letting the animation adapter's format-sniffing logic leak into `chrys-cli` as a per-format `match`.** Dispatch on `is_apng()`/`has_animation()`/file-extension belongs entirely inside the adapter selection step, and should resolve to "which `Source` implementation to construct," not to a chain of format-specific branches sprinkled through the CLI's comparison logic.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| GIF/APNG/WebP frame compositing (disposal, blend) | A hand-rolled canvas accumulator | `image`'s `AnimationDecoder::into_frames()` | Already implemented, already correct per each format's own disposal/blend rules, verified by reading the decoder source this session (Q1) |
| Natural-order filename sort | A hand-written numeric-substring comparator | `natord::compare` | A two-function crate, 11 years old, 195K downloads/week; a hand-rolled version would need to independently get right the same edge cases (mixed digit runs, leading zeros within a run) `natord` already handles |
| Per-file raster decode with memory limits | A second `image::ImageReader` call site inside `chrys-source-sequence` | `chrys_source_raster::decode::decode_guarded` (reused) | Already guards the CVE-2023-29408-class risk; a second call site is a second place that guard can be forgotten |
| Hints sidecar parsing | A hand-rolled key=value or line-based format | `toml` + `serde` | Already this project's designated declarative-config format; a second, bespoke format for hints would be a second parser to secure and document |

**Key insight:** every item in this table is already solved, correctly, by a dependency this project either already has pinned (`image`) or is about to standardize on for an unrelated reason (`toml`/`serde`, needed again in Phase 3 for the rule file). The one place this phase must hand-roll something — the animated-WebP container assembler for fixture generation (Q9) — is the exception precisely because no crate exists to do it, the same shape of justified exception `.planning/research/STACK.md` already accepted for the mesh rasterizer in a later phase.

## Common Pitfalls

### Pitfall 1: Trusting the WebP "already proven deterministic" claim past its actual scope

**What goes wrong:** A plan reads "Phase 1 already proved WebP decode is bit-exact on six runners" and assumes that covers any WebP content, including a lossy animated frame.

**Why it happens:** Phase 1's WebP fixture is lossless (`VP8L`); the claim that was actually proven is narrower than "WebP is deterministic."

**How to avoid:** Commit only lossless (VP8L) animated WebP fixtures this phase (also forced by the fixture-generation gap, Q9); if a lossy animated-WebP path is ever needed, treat it as new, unproven determinism surface requiring its own golden-hash fixture, not an extension of an existing proof.

**Warning signs:** A plan task that says "add a WebP animation fixture" with no mention of which VP8 variant it uses.

**Phase to address:** This phase, at fixture-generation time (Q7, Q9).

### Pitfall 2: Building the natural-sort comparator by hand "because it's simple"

**What goes wrong:** A hand-written comparator gets the common case right (`frame2` before `frame10`) and mishandles a less-common one (multiple digit runs in one filename, e.g. `v2_frame10` vs `v10_frame2`), silently mis-ordering a sequence that a person would not notice was wrong until a later frame's verdict looked strange.

**Why it happens:** The trap this question named explicitly (`frame10.png` before `frame2.png` under lexicographic sort) looks small enough to fix with a five-line regex, and a five-line regex usually does fix that one case while missing others `natord`'s own test suite already covers.

**How to avoid:** Use `natord::compare` unconditionally (Pattern 3); do not write a bespoke comparator.

**Warning signs:** A code review comment reading "isn't `natural_sort_key()` just splitting on digits?" — yes, and that split is exactly where the edge cases live.

**Phase to address:** This phase, in the sequence adapter's own implementation task.

### Pitfall 3: Building the ANMF container assembler with no round-trip test

**What goes wrong:** A hand-assembled RIFF/`VP8X`/`ANIM`/`ANMF` container has a subtly wrong field (the Google spec's `Frame X`/`Frame Y` fields are stored as the actual offset divided by 2, and `Frame Width Minus One`/`Frame Height Minus One` are stored as `value - 1`, both easy to get backwards) and produces a file that some WebP readers tolerate and others reject, or that decodes to the wrong frame positions silently.

**Why it happens:** This is new, unreviewed code with no existing test coverage to inherit, unlike every other fixture-generation path in this phase, which only calls an already-tested encoder.

**How to avoid:** Require a round-trip test in the same commit: decode the generated file back through `image::codecs::webp::WebPDecoder` and assert frame count and pixel content match the input, before the file is trusted as a golden fixture (Q9).

**Warning signs:** A generated `.webp` file that some tools render correctly and others show garbled or blank — a classic signature of a container-layout bug that happens to survive one specific reader's leniency.

**Phase to address:** This phase, in the animation fixture-generation task, as its own explicit sub-task with its own test.

### Pitfall 4: Threading a per-frame timestamp into `Frame` "to be ready for video"

**What goes wrong:** Adding a `presented_at: Duration` (or similar) field to `chrys_source::Frame` now, ahead of any phase that actually needs it, forces every existing `Frame { .. }` literal across `chrys-core`'s test and fixture code (eleven sites, confirmed by this session's grep) to be updated for a capability nothing in this phase consumes, and — worse — invites `compare_sequence` to start pairing by time instead of by index, quietly contradicting REQUIREMENTS.md's own explicit index/timestamp distinction (Q2) before Phase 7 has designed what timestamp pairing should actually look like for video's genuinely different failure modes (VFR drift, colour-matrix mismatch, per `.planning/research/PITFALLS.md` Pitfall 10).

**How to avoid:** Do not add a timing field to `Frame` in this phase. Let the animation adapter discard `image::Frame::delay()` after use, if it reads it at all.

**Warning signs:** A plan task titled "add frame timing support" with no corresponding requirement ID from this phase's list (SRC-02, SRC-03, SRC-08, SRC-09) driving it.

**Phase to address:** Flagged here so it is not accidentally picked up as "obviously the right thing to do" during planning; the actual design belongs to Phase 7.

## Code Examples

### Adapter selection in the CLI (recommended, not yet written)

```rust
// Recommended shape for chrys-cli's path-to-source dispatch. Builds on
// the existing RasterSource wiring at crates/chrys-cli/src/main.rs:65-67,
// read verbatim this session, extended rather than replaced.
fn frames_for(path: &Path) -> anyhow::Result<Vec<chrys_source::Frame>> {
    if path.is_dir() {
        return Ok(chrys_source_sequence::SequenceSource::new().load(path)?);
    }
    // Format sniff, reusing the same guessed-format entry point
    // chrys-source-raster already uses internally.
    if is_gif(path) || is_animated_png(path)? || is_animated_webp(path)? {
        return Ok(chrys_source_animation::AnimationSource::new().load(path)?);
    }
    Ok(chrys_source_raster::RasterSource::new().load(path)?)
}
```

### `is_apng` / `has_animation` detection (verified API surface)

```rust
// Source: docs.rs/image/0.25.10 — PngDecoder::is_apng (ImageResult<bool>)
// and WebPDecoder::has_animation (bool), both quoted verbatim this
// session.
let is_apng: bool = png_decoder.is_apng()?;
let is_animated: bool = webp_decoder.has_animation();
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Assume a GIF/APNG adapter must implement its own disposal/blend compositor | `image` 0.25's `AnimationDecoder` already composites internally | Confirmed by reading `image` 0.25.10's own decoder source this session, not a recent change per se, but a fact this phase's plan must not re-derive from first principles | Removes an entire implementation task and its associated determinism risk from the plan |
| Assume WebP's "no animated encode" gap (`.planning/research/STACK.md` Layer 3) is stale | Confirmed still true this session via WebSearch cross-check | This session | Fixture generation for animated WebP needs the hand-rolled container step (Q9); this is not a research gap that closed since Phase 1 |

**Deprecated/outdated:** none specific to this phase; the two "state of the art" rows above are confirmations of Phase 1-era findings, not changes to them.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A directory (not a glob pattern, not a manifest file) is the right unit for "what defines a sequence" | Q3 | If wrong, the CLI's two-`PathBuf` signature would need a new flag or syntax instead of reusing the existing one; low risk, since a directory is the more restrictive and simpler choice and can be loosened later without breaking anything already built |
| A2 | A `<name>.hints.toml` sidecar file is the right shape for "how a producer supplies a hint," rather than a CLI flag or an embedded-in-the-image-format mechanism | Q6 | If wrong, the hint-carrying mechanism changes but the crop-then-compare design (which is what actually keeps `chrys-core` unchanged) does not; low risk to the phase's headline claim, moderate risk to the exact UX shape |
| A3 | ffmpeg's `image2` demuxer documentation, read via WebSearch synthesis rather than a direct fetch of the primary ffmpeg.org page, accurately describes the unpadded `%d` convention | Q3 | If the summarized quote is imprecise, the underlying recommendation (always use natural sort, never assume padding) is unaffected — it is the more conservative choice either way |
| A4 | `image-webp`'s animated decode path supports lossy (VP8) frames, based on a WebSearch synthesis of the crate's published capability description rather than a direct read of its VP8-frame decode source | Q7 | If wrong (i.e., animated WebP is actually lossless-only end to end), Pitfall 1's caution is unnecessary but harmless — the recommendation to commit only lossless fixtures still holds and loses nothing by being followed anyway |

## Open Questions

1. **Exact `ANMF`/`VP8X` byte layout for the fixture generator's container assembler.**
   - What we know: the field names, sizes and units for `ANIM` and `ANMF` chunks, fetched and quoted from Google's own WebP container specification this session (Q9).
   - What's unclear: this was not implemented and round-trip-tested this session, only specified from documentation.
   - Recommendation: treat the round-trip decode test (Pitfall 3) as a hard gate before this fixture is trusted, not as an optional nicety.

2. **Whether `RegionHint` gaining `serde::Deserialize` needs a corresponding `Serialize` for symmetry, or read-only is sufficient for this phase.**
   - What we know: SRC-09 only requires the engine to *consume* a hint; nothing in this phase's four success criteria asks the tool to *write* a hints file.
   - What's unclear: whether a future phase (e.g. the native window's accept/reject flow, Phase 6) will want to write hints back out.
   - Recommendation: derive `Deserialize` only in this phase; add `Serialize` later if and when a concrete need appears, rather than speculatively.

## Environment Availability

No new external tool, service, or runtime dependency is introduced by this phase. Every new crate (`natord`, `toml`, `serde`, the `gif` feature of `image`, and `png` as a direct dev-dependency) is pure Rust, already resolvable from the same crates.io registry Phase 1 already depends on, and needs no system library, no network service at runtime, and no additional CI runner beyond the six labels `.github/workflows/determinism.yml` already uses. Fixture generation is a `cargo run --example` invocation, the same as Phase 1's `make-fixtures.rs`, needing nothing beyond the Rust toolchain already pinned for this repository.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in), same as Phase 1 — no new framework introduced |
| Config file | none new — this phase adds crates and test files under the existing workspace layout |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo test --workspace --all-features` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SRC-02 | A numbered frame sequence pair is compared frame by frame | integration | `cargo test -p chrys-source-sequence` | ❌ Wave 0 |
| SRC-02 | Natural sort orders `frame2.png` before `frame10.png` | unit | `cargo test -p chrys-source-sequence sequence::tests::sorts_by_natural_order -- --exact` | ❌ Wave 0 |
| SRC-03 | A GIF pair decodes to a `Vec<Frame>` matching its frame count | integration | `cargo test -p chrys-source-animation gif -- --exact` | ❌ Wave 0 |
| SRC-03 | An APNG pair decodes with disposal/blend already resolved | integration | `cargo test -p chrys-source-animation apng -- --exact` | ❌ Wave 0 |
| SRC-03 | An animated WebP pair decodes; round-trip test on the hand-assembled fixture | integration | `cargo test -p chrys-source-animation webp_anim -- --exact` | ❌ Wave 0 |
| SRC-08 | `compare_sequence` refuses on frame-count mismatch, refuses on an empty side, else pairs by index | unit | `cargo test -p chrys-core --lib sequence::tests -- --exact` | ❌ Wave 0 |
| SRC-08 | `git diff` proves no file under `crates/chrys-core/` changed across the animation wave's commits | CI/manual, per-wave | `git diff --name-only <wave-start>^..<wave-end> -- crates/chrys-core/` (expect empty) | ❌ Wave 0 (new drill script `scripts/engine-boundary-drill.sh`) |
| SRC-08 | The existing dependency guard stays green with no change needed | unit (already exists) | `cargo test -p chrys-core --test determinism no_gpu_or_format_crate_enters_chrys_cores_dependency_graph` | ✅ exists (`crates/chrys-core/tests/determinism.rs:481`) |
| SRC-09 | `Frame::crop_to_region` returns a frame of exactly the hint's width and height, at the hint's content | unit | `cargo test -p chrys-source crop_to_region -- --exact` | ❌ Wave 0 |
| SRC-09 | A CLI comparison with `--region <name>` registers inside the region, not the whole frame | integration | `cargo test -p chrys-cli region_hint -- --exact` | ❌ Wave 0 |
| (cross-cutting) | Cross-OS/cross-arch identical SHA-256 digest for the new sequence and animation fixtures | CI-only (six-runner matrix) | extend `.github/workflows/determinism.yml`'s digest job with one `compare --hash-only` invocation per new fixture pair | ❌ Wave 0 (workflow edit) |

### Sampling Rate

- **Per task commit:** `cargo test --workspace`
- **Per wave merge:** `cargo test --workspace --all-features`, plus (for the animation wave specifically) the `git diff --name-only -- crates/chrys-core/` check from Q8
- **Phase gate:** the extended six-runner matrix green, plus a clean run of `scripts/engine-boundary-drill.sh`, before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/chrys-source-sequence/` and `crates/chrys-source-animation/` — neither crate exists yet
- [ ] `crates/chrys-core/src/sequence.rs` and its `#[cfg(test)]` module — does not exist yet
- [ ] `Frame::crop_to_region` in `crates/chrys-source/src/lib.rs`, plus its unit tests — does not exist yet
- [ ] `<name>.hints.toml` parsing in `chrys-source-raster`, plus `#[derive(serde::Deserialize)]` on `RegionHint` — does not exist yet
- [ ] `tests/golden/sequence-01/`, `tests/golden/formats/{gif,apng,webp-anim}/` and their generator examples — do not exist yet
- [ ] `scripts/engine-boundary-drill.sh` — does not exist yet
- [ ] `.github/workflows/determinism.yml`'s digest job extended for the new fixtures — not yet edited
- [ ] Framework install: `cargo add natord@=1.0.9 toml@=1.1.5 serde --features derive` at the relevant crate level once the crates exist

## Security Domain

`security_enforcement` is on (ASVS level 1, block on high) per `.planning/config.json`. This phase adds two more local decode paths and one more local config-file (TOML sidecar) parse path; it introduces no network surface, no authentication, and no session state, consistent with Phase 1's own framing.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture | Yes (narrowly) | The same format-blind trait boundary Phase 1 established, extended: the two new adapter crates are the only place a format or a filesystem-layout convention is known, matching `.planning/research/ARCHITECTURE.md`'s rule this project already treats as a compile-time fact |
| V5 Input Validation | Yes | Every new decode call in `chrys-source-sequence` goes through the existing `decode_guarded` limits (Q3, Pattern 2), so the CVE-2023-29408-class risk is not reopened by a second call site. `chrys-source-animation`'s own `image::ImageReader`-based sniff-and-decode must apply the same `DecodeLimits`/`image::Limits` discipline Phase 1 established, not a fresh, unguarded decode path |
| V5 Input Validation | Yes | The new `<name>.hints.toml` parse path is a new untrusted-input surface (a local file, but still not authored by this project's own code) and must fail loudly on malformed TOML or an out-of-bounds region (e.g. a hint rectangle larger than the frame), rather than panicking or silently clamping |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A crafted GIF/APNG/animated-WebP file with a huge declared frame count or canvas size, forcing unbounded allocation before any pixel is read | Denial of Service | Apply `DecodeLimits`/`image::Limits` to the animation adapter's decode path exactly as Phase 1 applied it to the raster path; this is the same CVE-2023-29408-class mitigation, extended to a second decode entry point rather than assumed to already cover it |
| A crafted `<name>.hints.toml` naming a region larger than the frame, or with a negative/overflowing offset | Denial of Service / Tampering | `Frame::crop_to_region` must validate the hint's rectangle against the frame's own `width`/`height` and fail loudly (a `Result`, not a panic or a silent clamp) on an out-of-bounds hint, consistent with this project's "refuse rather than guess" precedent |
| A malformed hand-assembled `ANMF` container (Pitfall 3) being trusted as a golden fixture without a round-trip check | Tampering (of test evidence, not of a live system) | The round-trip decode test required in Q9/Pitfall 3 is itself the mitigation: a fixture that cannot be decoded back correctly is caught before it is committed, not after a later phase depends on it |

## Sources

### Primary (HIGH confidence)

- `crates/chrys-source/src/lib.rs`, `crates/chrys-source-raster/src/lib.rs`, `decode.rs`, `normalize.rs`, `crates/chrys-core/src/lib.rs`, `verdict.rs`, `hash.rs`, `crates/chrys-cli/src/main.rs`, `crates/chrys-core/tests/determinism.rs`, `scripts/determinism-drill.sh`, `crates/chrys-source-raster/examples/make-fixtures.rs`, `Cargo.toml`, `Cargo.lock`, `.github/workflows/determinism.yml` — all read in full or in relevant part this session, with line ranges quoted above
- `cargo add <pkg> --dry-run` against the live crates.io registry, this session, for every version cited in Standard Stack
- `gsd_run query package-legitimacy check` against crates.io metadata, this session, for `natord`, `human-sort`, `lexical-sort`, `natural-sort-rs`, `alphanumeric-sort`, `gif`, `png`
- Direct byte inspection (`xxd`) of `tests/golden/formats/webp/base.webp`, this session, confirming its `VP8L` (lossless) chunk fourcc
- [docs.rs/image/0.25.10/src/image/codecs/gif.rs.html](https://docs.rs/image/0.25.10/src/image/codecs/gif.rs.html), [docs.rs/image/0.25.10/src/image/codecs/png.rs.html](https://docs.rs/image/0.25.10/src/image/codecs/png.rs.html), [docs.rs/image-webp/latest/src/image_webp/decoder.rs.html](https://docs.rs/image-webp/latest/src/image_webp/decoder.rs.html) — decoder source read directly this session, not their prose docs
- [docs.rs/image/0.25.10/image/trait.AnimationDecoder.html](https://docs.rs/image/0.25.10/image/trait.AnimationDecoder.html), [.../codecs/gif/struct.GifDecoder.html](https://docs.rs/image/0.25.10/image/codecs/gif/struct.GifDecoder.html), [.../codecs/png/struct.ApngDecoder.html](https://docs.rs/image/0.25.10/image/codecs/png/struct.ApngDecoder.html), [.../codecs/webp/struct.WebPEncoder.html](https://docs.rs/image/0.25.10/image/codecs/webp/struct.WebPEncoder.html), [.../codecs/png/struct.PngDecoder.html](https://docs.rs/image/0.25.10/image/codecs/png/struct.PngDecoder.html), [.../codecs/webp/struct.WebPDecoder.html](https://docs.rs/image/0.25.10/image/codecs/webp/struct.WebPDecoder.html), [.../struct.Frame.html](https://docs.rs/image/0.25.10/image/struct.Frame.html), [.../struct.Delay.html](https://docs.rs/image/0.25.10/image/struct.Delay.html) — fetched and quoted this session
- [docs.rs/natord/latest/natord](https://docs.rs/natord/latest/natord/) — API quoted verbatim this session
- [docs.rs/png/latest/png/struct.Encoder.html](https://docs.rs/png/latest/png/struct.Encoder.html) — `set_animated`/`set_frame_delay` quoted this session
- [developers.google.com/speed/webp/docs/riff_container](https://developers.google.com/speed/webp/docs/riff_container) — `ANIM`/`ANMF` chunk layout and `VP8X` animation flag, fetched and quoted this session

### Secondary (MEDIUM confidence)

- WebSearch synthesis of `image-webp`'s published lossy/animated encode and decode capability description (Q1, Q7) — cross-checked against `.planning/research/STACK.md` Layer 3's prior finding, not independently re-read from `image-webp`'s own VP8 decode source this session
- WebSearch synthesis of ffmpeg's `image2` demuxer documentation on padded vs. unpadded numbering (Q3)
- `.planning/research/STACK.md`, `.planning/research/PITFALLS.md`, `.planning/research/ARCHITECTURE.md`, `01-RESEARCH.md`, `01-LEARNINGS.md` — this project's own prior research and phase record, incorporated directly where this session did not independently re-verify a claim

### Tertiary (LOW confidence)

- None specific to this phase beyond what is already flagged in the Assumptions Log above.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every crate version checked against the live crates.io registry this session via `cargo add --dry-run`, not recalled
- Architecture: HIGH — every claim about the existing `Source` trait, `Frame`, `compare()`, `RegionHint`, and the existing determinism guards is grounded in a direct `Read` of the exact file, this session, with line numbers and verbatim quotes
- Animation decode/compositing: HIGH — confirmed by reading decoder source (not prose docs) for all three formats this session
- Animation encode (fixture generation): MEDIUM — the GIF and APNG encode paths are confirmed APIs; the animated-WebP container assembly is new, unreviewed code by necessity (no crate exists), mitigated by requiring a round-trip test
- Pitfalls: HIGH — grounded in this session's own direct verification (the lossless-WebP byte inspection, the decoder source reads) rather than inherited assumption

**Research date:** 2026-09-07
**Valid until:** 30 days for crate versions (re-verify at plan time, per this project's own established habit); 90 days for the architecture and pitfalls material (grounded in source code read this session, stable until the code itself changes)
