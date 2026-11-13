# Design

This file holds the design of Chrysoberyl: the core value, the invariants,
the requirements, the state of each determinism gate, and the lessons.
It replaces the planning files that the project used before 2026-11-13.
The history of `main` keeps those files.

## What Chrysoberyl is

Chrysoberyl is a structural diff for everything that is not text.
It compares images, animation, video, PDF pages, SVG and 3D meshes.
It reports what changed, not which pixels differ.
It runs as a library, a command line tool, and a native window on Linux,
macOS and Windows.
There is no account and no hosted service.

## Core value

A baseline is portable.
The same pair gives the same verdict on any machine, any GPU and any driver.
Everything else can fail. This cannot.

## Invariants

These five rules were set on 2026-09-06, before the first line of code.

1. No pixel that enters a comparison comes from a GPU.
   Rasterization is on the CPU and is deterministic.
   The GPU compares for display and shows the result only.
2. The engine assumes near-identical pairs.
   It never grows into general image registration.
   A pair outside that assumption is refused, with the reason.
3. One engine, one meaning of "changed".
   A format can add detail.
   It never contradicts or replaces the raster verdict.
4. A mesh renders through a fixed eight-view rig that Chrysoberyl owns.
   A person cannot change the rig in v1.
   The output names the blind spot.
5. Rules are declarative TOML.
   A rule file is data. No rule is computed.

## Out of scope

- General image registration on arbitrary pairs.
- GPU rasterization of any pixel that enters a comparison.
- A hosted service, an account, or an upload.
- A configurable mesh rig in v1.
- A scripting language for rules.
- Text and code diffs.

## Crates

| Crate | Job |
|-------|-----|
| `chrys-source` | The `Frame` and `Source` types. No file format. |
| `chrys-core` | The engine: registration, residual, labels, kinds, verdict. |
| `chrys-source-raster` | PNG, JPEG, WebP, TIFF. |
| `chrys-source-sequence` | A directory of numbered frames. |
| `chrys-source-animation` | GIF, APNG, animated WebP. |
| `chrys-source-svg` | SVG through `resvg`, with one pinned font. |
| `chrys-source-pdf` | PDF through `hayro`. Feature `pdf`. |
| `chrys-source-video` | AV1 in IVF through `re_rav1d`. Feature `video`. |
| `chrys-source-mesh` | OBJ and STL through the rig. Feature `mesh`. |
| `chrys-rule` | The TOML rule engine. |
| `chrys-baseline` | The baseline store and its manifest. |
| `chrys-gpu` | The wgpu difference mask, for display only. Feature `view`. |
| `chrys-view` | The native window. Feature `view`. |
| `chrys-cli` | The `chrys` binary. |

Each optional feature of `chrys-cli` is off by default.
A test in `chrys-cli/tests` reads the resolved dependency graph for each
optional path. It proves that the engine cannot reach the path, and that
the default binary does not hold it.

Each crate forbids `unsafe` code.
The engine never names a file format.
A new format adds one adapter crate and changes no engine file.

## Requirements

All 42 v1 requirements have code.
Two of them carry a claim that stays open. See "Open claims".

| Group | Requirements | State |
|-------|--------------|-------|
| Engine | CORE-01 to CORE-07 | Done, phase 1. |
| Determinism | DET-01 to DET-06 | Done, phase 1. Each later format repeats DET-01 and DET-02 in its own gate. |
| Input | SRC-01 to SRC-04, SRC-08, SRC-09 | Done, phases 1, 2 and 4. |
| Input | SRC-05, PDF | Done, phase 5. |
| Input | SRC-06, video | Done, phase 7, for the AV1 path only. |
| Input | SRC-07, mesh | Done, phase 8. |
| Rules | RULE-01 to RULE-05 | Done, phase 3. |
| Baseline | BASE-01 to BASE-04 | Done, phase 3. |
| CLI and CI | CLI-01 to CLI-04 | Done, phase 3. |
| Window | VIEW-01 to VIEW-04 | Done, phase 6. |
| GPU mirror | GPU-01, GPU-02 | Done, phase 6, with one change in writing. See below. |
| GPU mirror | GPU-03 | Open. See "Open claims". |

v2 requirements: a second baseline store (STORE-01), a reusable CI
workflow (CI-01), more codecs (SRC-10), a browser DOM hint adapter
(SRC-11), and a Butteraugli-class metric (METRIC-01).

## Determinism gates

The `determinism` workflow runs the digest report on six runners:
`ubuntu-24.04`, `ubuntu-24.04-arm`, `macos-15`, `macos-15-intel`,
`windows-2022` and `windows-11-arm`.
The `agree` job compares the six reports byte for byte.
Each format adds its fixture to the same report, so one gate covers all.

| Format | Gate | Evidence |
|--------|------|----------|
| Raster, sequence, animation | Six runners agree | Run 34046421348, phase 1. |
| SVG | Six runners agree | Phase 4. |
| PDF | Six runners agree | Run 43499530340, 2026-11-13. |
| AV1 video | Six runners agree, and the decoded planes equal libdav1d | Run 43502294746, and `chrys-source-video/tests/video.rs`. |
| Mesh | Six runners agree | Run 43504557653, 2026-11-13. |

## How each new path stays deterministic

- PDF: `hayro` runs at the scalar SIMD level, on one thread, with the
  integer pipeline. Its fonts are embedded. It reads no host font.
- Video: the AV1 decoder runs without assembly, on one thread. AV1 decode
  is exact by its specification, and a test compares each decoded plane
  with libdav1d. The colour conversion reads the matrix and range tags and
  uses integer arithmetic. A stream with no matrix tag is refused.
  Two videos pair by presentation time. A frame rate change alone is not a
  change.
- Mesh: the rig matrices are constants. The float stage uses only
  operations that IEEE 754 makes exact. Coverage, depth, the cell pattern
  and the grain use integers. A test forbids each platform transcendental
  in the crate.
- GPU: the GPU mask is integer arithmetic on bytes. No verdict comes from
  it. A test proves that it equals the CPU mask byte for byte.

## Changes from the first plan, in writing

- PDF uses `hayro`, not `pdfium`. No binary goes into git.
- The window uses `winit` and `wgpu` directly, not Slint. The window is one
  canvas with keys and a title bar, so a UI toolkit adds weight and no
  function.
- GPU-02 asks for "the same kinds". The GPU path finds regions only. It
  classifies no kind, because a kind is part of a verdict, and GPU-01
  forbids a verdict from the GPU. The check compares the count of regions
  and each bounding box, inside 2 pixels.
- Video ships one codec path: AV1, 8 bits, in IVF. H.264, HEVC, VP9, MP4
  and WebM are out of v1. A path is added only with its own conformance
  test.
- The mesh views carry depth tone, a model-space cell pattern, a fixed
  backdrop and a fixed screen-space grain. A view of flat grey faces on
  white gave the phase correlation too little detail: the engine refused
  the pair or missed a small step. The engine did not change.

## Open claims

1. GPU-03: the GPU equivalence test passes on three vendors.
   It passes on Apple M5 (Metal) only.
   Run `cargo test -p chrys-gpu -- --ignored` and `chrys gpu-check` on an
   Intel, an Nvidia and an AMD machine to settle it.
   The CPU decides each verdict, so this claim cannot change a verdict.
2. PDF images: `hayro` scales an embedded image at a SIMD level that it
   finds itself. The PDF fixture holds no image. A PDF with an image is not
   yet proven across machines.
3. Lossy animated WebP determinism is not proven. Each animated fixture is
   lossless.
4. The window was opened on macOS only.

## Lessons

- A guard that cannot go red reports green forever. Prove each guard by
  planting its defect.
- Measure the thing that you claim. A digest of raw RGBA8 is evidence. A
  digest of a re-encoded file is not.
- The default FFT planner in `rustfft` picks an algorithm per machine. The
  engine uses the scalar planner.
- Platform transcendentals differ between machines. Colour and window
  maths goes through `libm` or `palette`.
- Stable Rust never fuses a multiplication and an addition, so that risk
  does not exist here.
- A flat region gives a correlation search nothing to hold. This was true
  for the phase 1 fixtures, and again for the mesh views.
- A periodic pattern gives the phase correlation many equal peaks. Use an
  aperiodic one.
- A fixture proves the thing that it is built for and nothing else.
- SVT-AV1 ignores `-g 1`. Use `-svtav1-params keyint=1` for intra-only
  fixtures.
