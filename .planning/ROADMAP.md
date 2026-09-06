# Roadmap: Chrysoberyl

## Overview

Chrysoberyl proves one claim in eight phases: the same pair gives the same
verdict on any machine. Phase 1 builds the comparison engine against raster
images and proves determinism on that engine in CI, so a bug in the core
math is found before any format sits on top of it. Phase 2 formalizes the
`Source` trait against a second, easy format, so the engine's format-blind
design is tested early. Phase 3 makes the tool usable: a baseline, a
declarative rule file, and a CI gate. Phases 4 through 8 then add one format
family at a time, ordered from most-proven (SVG) to least-proven (PDF, the
wgpu display mirror, video, mesh), so a failure is always attributable to the
one new kind of risk that phase just added. Each format phase after Phase 1
carries its own cross-OS, cross-architecture golden-hash test as an exit
criterion, because research/SUMMARY.md shows every threat to the Core Value
recurs at each new format boundary. Three phases carry an unproven claim
forward as a phase gate that must be settled, or withdrawn in writing, before
the phase counts as done.

## Derivation Notes

research/SUMMARY.md converged on a nine-phase order. This roadmap merges and
reassigns as follows, and lands at eight phases, inside the 5-to-8 range for
`standard` granularity.

- **Merged research Phase 1 (raster spine) and Phase 2 (determinism CI) into
  Phase 1.** The task guidance is explicit: the determinism proof is a phase
  exit criterion of the phase that adds the decoder or rasterizer, not a
  separate later phase. Raster decode is Phase 1's decoder, so its
  determinism proof belongs in Phase 1.
- **Moved VIEW-01 through VIEW-04 from research's Phase 4 to Phase 6.**
  PROJECT.md's own tech stack decision ties the native window to wgpu,
  through Slint's shared device and queue. A window cannot exist before that
  infrastructure does, so the window requirements move to the phase that
  builds it.
- **Kept Phase 5 (PDF), Phase 7 (video), and Phase 8 (mesh) as
  single-requirement phases**, despite standard granularity's usual
  preference against a thin phase. Each phase introduces exactly one new
  kind of risk: a new document decoder, a new codec-decode path, and a
  hand-rolled rasterizer with no existing crate. The project's own
  sequencing constraint states that risk isolation outranks packing phases
  full, so these stay separate rather than merge into a neighbour.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

- [ ] **Phase 1: Raster engine and determinism proof** - Compare raster images and prove the verdict is identical across OS and CPU architecture.
- [ ] **Phase 2: Source trait and a second format** - Add animation and frame-sequence input without a change to the comparison engine.
- [ ] **Phase 3: Baseline, rules and CI gate** - Commit a baseline, scope tolerance by rule, and gate CI on the verdict.
- [ ] **Phase 4: SVG rasterization** - Rasterize vector input on the CPU and prove the font and curve output is deterministic.
- [ ] **Phase 5: PDF page comparison** - Compare PDF pages behind a feature flag, and prove or withdraw the determinism claim.
- [ ] **Phase 6: wgpu mirror and native window** - Show a pair in a native window with GPU-accelerated pan and zoom, with the GPU never deciding a verdict.
- [ ] **Phase 7: Video comparison** - Compare video pairs behind a feature flag, decoded in software, paired by timestamp.
- [ ] **Phase 8: Mesh comparison via eight-view rig** - Compare 3D meshes through a fixed rig, with a hand-written rasterizer proven deterministic.

## Phase Details

### Phase 1: Raster engine and determinism proof

**Goal**: A person compares two raster images and gets a structural verdict that is byte-identical across OS and architecture.
**Mode:** mvp
**Depends on**: Nothing (first phase)
**Requirements**: CORE-01, CORE-02, CORE-03, CORE-04, CORE-05, CORE-06, CORE-07, DET-01, DET-02, DET-03, DET-04, DET-06, SRC-01
**Success Criteria** (what must be TRUE):

  1. The tool compares two raster images (PNG, JPEG, WebP, TIFF) and names each change by kind: moved, added, removed, recoloured, or resized. It never reports a bare pixel count.
  2. The tool reports a translated region with its offset in pixels, and a colour change with a colour-difference value and both colours.
  3. The tool does not report an antialiasing difference that a person cannot see.
  4. The tool refuses a pair it cannot register, and states why, instead of giving a verdict it cannot support.
  5. CI hashes the raw RGBA8 output of the same pair on Linux, macOS, Windows, x86-64, and aarch64, and the hash matches on every commit. This is the phase's determinism exit gate, and the mechanism every later format phase reuses.

**Plans**: 3/8 plans executed

Plans:
**Wave 1**

- [x] 01-01-PLAN.md — Walking skeleton: the workspace, four crates, and one verdict from two real PNG files

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 01-02-PLAN.md — Guard every decode and read a pair in PNG, JPEG, WebP and TIFF

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 01-03-PLAN.md — The digest report over raw RGBA8 and the six-runner determinism workflow

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 01-04-PLAN.md — Global registration on one arithmetic path, with subpixel refinement

**Wave 5** *(blocked on Wave 4 completion)*

- [ ] 01-05-PLAN.md — Refuse a pair the engine cannot register, on a threshold measured from a corpus

**Wave 6** *(blocked on Wave 5 completion)*

- [ ] 01-06-PLAN.md — Local registration: warp, summed-area table, block match and the residual field

**Wave 7** *(blocked on Wave 6 completion)*

- [ ] 01-07-PLAN.md — Classify: labelled regions, antialiasing suppression, change kind and colour

**Wave 8** *(blocked on Wave 7 completion)*

- [ ] 01-08-PLAN.md — Determinism guards, watched failing on purpose

**Note**: the six-runner matrix cannot run while this repository has no remote,
which PROJECT.md records as a decision. Plan 01-03 writes and validates the
workflow and records the green matrix as a human check. Plan 01-08 supplies the
cross-architecture evidence that can be measured on one machine today.

### Phase 2: Source trait and a second format

**Goal**: A new kind of input is added without a change to the comparison engine.
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: SRC-02, SRC-03, SRC-08, SRC-09
**Success Criteria** (what must be TRUE):

  1. A numbered frame sequence pair is compared frame by frame, at the same quality as a single raster pair.
  2. An animation pair (GIF, APNG, animated WebP) is compared frame by frame.
  3. The animation adapter is added with no change to any file in the comparison engine.
  4. An input source supplies a named region as a hint, and the engine registers inside that region instead of searching for it.

**Plans**: TBD

### Phase 3: Baseline, rules and CI gate

**Goal**: A person commits a baseline, scopes tolerance by rule, and gates CI on the verdict.
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: RULE-01, RULE-02, RULE-03, RULE-04, RULE-05, BASE-01, BASE-02, BASE-03, BASE-04, CLI-01, CLI-02, CLI-03, CLI-04
**Success Criteria** (what must be TRUE):

  1. A TOML rule file scopes tolerance by kind of change and by a named region or mask. The shipped example shows a scoped exclusion, never a bare global threshold.
  2. An unknown key or a malformed rule fails loudly and names the line.
  3. A baseline hash manifest is committed to the repository. Committed golden files are the default store, reached through a trait, so a second backend needs no change to the engine.
  4. The CLI compares two paths, prints the verdict, exits non-zero on a rule failure, and writes a report artifact that names each change by kind, region, and size.
  5. A person accepts a new baseline explicitly through the CLI. Nothing updates a baseline on its own, and every decode call sets an explicit memory limit.

**Plans**: TBD

### Phase 4: SVG rasterization

**Goal**: Vector input is rasterized deterministically on the CPU, proving the Core Value against real curves, gradients, and text.
**Mode:** mvp
**Depends on**: Phase 2, Phase 3
**Requirements**: SRC-04, DET-05
**Success Criteria** (what must be TRUE):

  1. An SVG pair is rasterized on the CPU with resvg and tiny-skia, and compared at the same quality as raster input.
  2. Text rasterization uses a font set pinned in the repository. The tool never reads the host font database.
  3. A cross-OS golden-hash test on a text-bearing SVG fixture passes on Linux, macOS, and Windows. This test is the phase's exit gate for the font and curve determinism claim, and it is the template every later format-specific gate reuses.

**Plans**: TBD

### Phase 5: PDF page comparison

**Goal**: A PDF page pair is compared with the same verdict discipline as SVG, without assuming pdfium's cross-platform determinism.
**Mode:** mvp
**Depends on**: Phase 4
**Requirements**: SRC-05
**Success Criteria** (what must be TRUE):

  1. A PDF page pair is compared behind a Cargo feature that is off by default.
  2. The pdfium binary build and its bundled fonts are pinned in the repository, not resolved at build time.
  3. A cross-OS golden-hash test proves the same PDF pair gives the same verdict on Linux, macOS, and Windows. Until this test passes, or a written note withdraws the claim, the PDF feature stays experimental. This settles Unproven Claim 1 (and Unproven Claim 6) from research/SUMMARY.md.

**Plans**: TBD

### Phase 6: wgpu mirror and native window

**Goal**: A person reviews a pair in a native window with GPU-accelerated pan, zoom, and scrubbing, and the GPU never decides a verdict.
**Mode:** mvp
**Depends on**: Phase 3, Phase 5
**Requirements**: VIEW-01, VIEW-02, VIEW-03, VIEW-04, GPU-01, GPU-02, GPU-03
**Success Criteria** (what must be TRUE):

  1. A window shows a pair side by side and as an overlay. A person pans and zooms it at full frame rate.
  2. A person moves through a sequence or a video pair frame by frame.
  3. A person accepts or rejects a change in the window, and an accept writes the new baseline.
  4. The wgpu compute path never writes a verdict. It agrees with the CPU reference by category: the same regions, the same kinds, and bounding boxes inside a stated tolerance, on at least three GPU vendors. This settles Unproven Claim 4 from research/SUMMARY.md.

**Plans**: TBD
**Note**: Budget a wgpu spike, on the buffer-staging and cross-vendor equivalence test pattern, before writing this phase's plan. The author has no prior GPU work.
**UI hint**: yes

### Phase 7: Video comparison

**Goal**: A video pair is compared by presentation timestamp, decoded in software only, on a codec path proven bit-exact rather than assumed bit-exact.
**Mode:** mvp
**Depends on**: Phase 2, Phase 3
**Requirements**: SRC-06
**Success Criteria** (what must be TRUE):

  1. A video pair is compared behind a Cargo feature that is off by default, decoded in software only.
  2. Frames are paired by presentation timestamp, not by frame index, so variable-frame-rate drift never reports a change that did not happen.
  3. Colourspace and colour-matrix tags are read and honoured, never assumed.
  4. A conformance-corpus test proves bit-exact decode for every codec path the feature ships, on Linux, macOS, and Windows. Until this test passes, or a written note withdraws a codec path, that path stays out of v1. This settles Unproven Claim 2 and Unproven Claim 8, the master gate, from research/SUMMARY.md, and answers PROJECT.md's own Open Question.

**Plans**: TBD

### Phase 8: Mesh comparison via eight-view rig

**Goal**: A 3D mesh pair is compared through a fixed rig, with a hand-rolled rasterizer whose determinism is proven, not assumed.
**Mode:** mvp
**Depends on**: Phase 4
**Requirements**: SRC-07
**Success Criteria** (what must be TRUE):

  1. A 3D mesh pair is compared through the fixed eight-view rig.
  2. The report names the blind spot the rig leaves.
  3. The rasterizer calls no platform transcendental function in its hot path. Every view and projection matrix is precomputed, never computed per frame.
  4. A cross-OS, cross-architecture golden-hash test on the rig's rendered views passes on Linux, macOS, Windows, x86-64, and aarch64. This settles Unproven Claim 3 from research/SUMMARY.md.

**Plans**: TBD
**Note**: Budget a rasterizer spike, on fixed-point versus IEEE-754 determinism strategy, before writing this phase's plan. No crate exists for this rasterizer. It is written by hand.

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Raster engine and determinism proof | 3/8 | In Progress|  |
| 2. Source trait and a second format | 0/TBD | Not started | - |
| 3. Baseline, rules and CI gate | 0/TBD | Not started | - |
| 4. SVG rasterization | 0/TBD | Not started | - |
| 5. PDF page comparison | 0/TBD | Not started | - |
| 6. wgpu mirror and native window | 0/TBD | Not started | - |
| 7. Video comparison | 0/TBD | Not started | - |
| 8. Mesh comparison via eight-view rig | 0/TBD | Not started | - |

---
*Roadmap created: 2026-09-06*
