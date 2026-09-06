# Architecture Research

**Domain:** Structural media diff engine (image, animation, video, PDF, SVG, 3D mesh comparison)
**Researched:** 2026-09-06
**Confidence:** MEDIUM-HIGH (algorithms and reference implementations are well documented; the exact GPU/CPU equivalence pattern for this project is novel, so that part is a recommendation, not a proven pattern)

## Standard Architecture

### System Overview

```
┌───────────────────────────────────────────────────────────────────────┐
│                          INPUT FAMILY ADAPTERS                        │
│  raster · frame-sequence · video (feature) · PDF (feature) · SVG      │
│  · 3D mesh (fixed 8-view rig)                                         │
│  each adapter implements one trait: Source → CanonicalFrames          │
├───────────────────────────────────────────────────────────────────────┤
│  DECODE            RASTERIZE (CPU only)                               │
│  bytes → native     native structure → RGBA8 pixel buffer(s)          │
│  structure          + per-frame metadata (index, timestamp, region    │
│                     hints from the optional hint channel)             │
├───────────────────────────────────────────────────────────────────────┤
│                        CANONICAL FRAME PAIR                           │
│         (baseline: Vec<Frame>, candidate: Vec<Frame>)                 │
│         Frame = { RGBA8 buffer, width, height, index, hints }         │
├───────────────────────────────────────────────────────────────────────┤
│  REGISTER (CPU reference, GPU-accelerated mirror)                     │
│  phase correlation → coarse-to-fine block match → residual field      │
├───────────────────────────────────────────────────────────────────────┤
│  CLASSIFY                                                             │
│  connected-component labelling of residual → typed regions            │
│  (translate / scale / colour-shift / added / removed)                 │
├───────────────────────────────────────────────────────────────────────┤
│  SCORE                                                                │
│  declarative TOML rule engine applies tolerance per region/kind       │
│  → pass/fail verdict, never computed truth, only threshold lookup     │
├───────────────────────────────────────────────────────────────────────┤
│  REPORT                                                               │
│  report artifact (JSON + rendered overlay) · exit code · baseline     │
│  hash manifest update · native window view (Slint + wgpu)             │
├───────────────────────────────────────────────────────────────────────┤
│                        BASELINE STORE (pluggable)                     │
│  committed goldens (default) · content-addressed cache · hash         │
│  manifest checked into the repository                                 │
└───────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| Source adapter | Turns one input family into a `Vec<Frame>` of RGBA8 buffers plus per-frame metadata. Owns every per-format decision. | One crate or module per family, all behind one trait, `resvg`/`tiny-skia` for SVG, a PDF crate for pages, `image`/`gif`/frame-index scan for raster and sequences, the fixed 8-view rig for meshes |
| Decode | Bytes to native structure (SVG tree, PDF page tree, mesh scene graph, container demux). Format-specific. Lives inside the adapter, never in the engine. | Per-format crate |
| Rasterize | Native structure to RGBA8, CPU only, deterministic. Format-specific, but the *output shape* is identical for every family. | `tiny-skia`, a software PDF rasterizer, a software mesh rasterizer for the 8-view rig |
| Register | Finds the geometric and photometric relationship between two same-shape RGBA8 buffers, under the near-identical-pair assumption. Format-agnostic. | Phase correlation (FFT) for global translation, coarse-to-fine block matching for local residual, integral images for local statistics |
| Classify | Labels the residual field into typed connected regions. Format-agnostic. | Two-pass union-find connected-component labelling |
| Score | Applies the TOML rule file's declared tolerances to the classified regions. Format-agnostic, deterministic table lookup only. | Rule engine, no per-format branch, no computed truth |
| Report | Serializes verdict, writes the artifact, updates the hash manifest, feeds the interactive window. | JSON + optional overlay PNG, Slint UI, wgpu canvas |
| Baseline store | Maps a test identity to a content-addressed baseline blob and a manifest hash. Pluggable backend, committed goldens by default. | Trait with a local filesystem implementation; optional S3/GCS-style implementation later |

## Recommended Project Structure

```
chrysoberyl/
├── crates/
│   ├── chrys-core/          # register, classify, score, report — the frozen spine
│   │   ├── register/        # phase correlation, block match, pyramids, integral images
│   │   ├── classify/        # connected-component labelling, region typing
│   │   ├── rules/           # TOML schema + declarative scorer
│   │   └── report/          # artifact writer, hash manifest
│   ├── chrys-source/        # the Source trait, and the CanonicalFrames type
│   ├── chrys-source-raster/ # raster image + animation/frame-sequence adapter
│   ├── chrys-source-vector/ # SVG adapter (tiny-skia backend)
│   ├── chrys-source-pdf/    # PDF adapter, Cargo feature, off by default
│   ├── chrys-source-video/  # video adapter, Cargo feature, off by default
│   ├── chrys-source-mesh/   # 3D mesh adapter, fixed 8-view rig
│   ├── chrys-gpu/           # wgpu compute mirror of register/classify, for interactive display
│   ├── chrys-store/         # baseline store trait + committed-goldens implementation
│   ├── chrys-cli/           # command line entry point
│   └── chrys-window/        # Slint + wgpu native window
└── tests/
    └── golden/              # committed baseline pairs and expected verdicts
```

### Structure Rationale

- **chrys-core/ has no format knowledge.** It only ever sees `Vec<Frame>` of RGBA8. This is where the "no per-format special case" rule is enforced by construction: the crate cannot import a format crate, because it has no dependency on one.
- **Each `chrys-source-*` crate is a leaf.** It depends on `chrys-source` (for the trait) and on its own format library. It never depends on `chrys-core`. The CLI wires adapters to the core, not the other way around. This keeps the dependency graph a star, not a mesh, so adding a seventh family cannot touch an existing one.
- **chrys-gpu/ is a mirror, not a replacement.** It reimplements the register/classify math as compute shaders for the interactive window, and is validated against chrys-core by an equivalence test suite. It is never the source of a CI verdict.
- **chrys-store/ is a trait plus one implementation.** Committed goldens ship as the only backend in v1. A content-addressed or cloud backend can be added later without touching the CLI or the core.

## Architectural Patterns

### Pattern 1: One trait admits every input family

**What:** A single `Source` trait converts any input family into the same canonical shape. No stage past this trait may branch on format.

```rust
/// One canonical frame: what every downstream stage operates on.
pub struct Frame {
    pub pixels: RgbaBuffer,      // CPU-rasterized, deterministic
    pub width: u32,
    pub height: u32,
    pub index: usize,            // frame/page/view number
    pub hints: Vec<RegionHint>,  // optional producer-supplied region names
}

/// Implemented once per input family. Nothing outside this trait
/// knows what a "page" or a "mesh view" is.
pub trait Source {
    type Error;
    fn load(&self, path: &Path) -> Result<Vec<Frame>, Self::Error>;
}
```

A 3D mesh adapter's `load()` renders the fixed eight-view rig and returns eight `Frame`s. A PDF adapter's `load()` returns one `Frame` per page. A video adapter's `load()` returns one `Frame` per decoded frame. `chrys-core` never knows the difference. This is the direct mechanism for the "no per-format special case" hard constraint: the constraint is not a style rule to remember, it is a compile-time fact, because `chrys-core` does not link against any format crate.

**When to use:** Any time a new input family is added. The trait is the only extension point.
**Trade-offs:** The trait must be right early. Widening it later (for example, adding audio channels to `Frame`) is a breaking change to every adapter. This is why the spine is frozen before more formats are added (see Build Order).

### Pattern 2: CPU reference, GPU mirror, equivalence tests bind them

**What:** Every register/classify algorithm has two implementations: a CPU reference in `chrys-core` (authoritative, used for every verdict, CI, and the report artifact) and a wgpu compute mirror in `chrys-gpu` (used only to redraw the interactive comparison view at interactive speed for large frames). A test suite asserts the two *agree on the categorical verdict*, not on exact floating point values.

**When to use:** Any stage that both needs to run in CI (headless, must be portable) and needs to render responsively in the window (large frames, must be fast).

**Trade-offs:** Doubles the implementation cost for register and classify. This is deliberate: the hard constraint says the GPU may compare and display, but no pixel entering a comparison is GPU-produced, and the project's core value is "same baseline, same verdict, any GPU." The only way to keep both promises is to never let the GPU path be the one that decides. See "The GPU comparison layer" below for why categorical agreement, not numeric agreement, is the correct bar.

### Pattern 3: Rules are a lookup table, not a program

**What:** The TOML rule file declares tolerance thresholds keyed by region kind and, optionally, by named region (matched to a hint). The scorer reads the file and does a table lookup against the already-classified regions. It contains no expression evaluator, no scripting hook, and no computed thresholds.

**When to use:** Every score-stage decision.

**Trade-offs:** Some legitimate cases (a threshold that should scale with region area) need a slightly richer schema (for example, `tolerance_px` versus `tolerance_fraction_of_area`), but the schema stays declarative: a fixed set of named fields, never an executable expression. This is what keeps a rule file reviewable in a pull request, per the project's explicit rejection of a rule scripting language.

## Data Flow

### Comparison Flow

```
baseline path, candidate path
        ↓
   Source::load()           (one call per input, per adapter)
        ↓
   Vec<Frame>, Vec<Frame>    (RGBA8, same shape contract for every family)
        ↓
   pair frames by index      (frame-sequence/video/PDF/mesh: 1:1 by position;
                              raster: the single pair)
        ↓
   REGISTER  per frame pair
     1. phase correlation on downsampled luma → coarse global translation + scale estimate
     2. warp candidate into baseline's frame using the coarse estimate
     3. coarse-to-fine block match on a Gaussian pyramid → residual field
        ↓
   residual field (per-pixel signed delta, plus a local colour-shift map)
        ↓
   CLASSIFY
     connected-component labelling of the thresholded residual
     → typed regions: translated / scaled / colour-shifted / added / removed
        ↓
   SCORE
     rule engine looks up tolerance per region kind + hint match
     → pass/fail per region, pass/fail overall
        ↓
   REPORT
     JSON artifact + overlay image + exit code
     baseline store: on --update-baseline, write new golden + update hash manifest
```

### Interactive Flow (native window)

```
report artifact + Vec<Frame> pair
        ↓
   chrys-window (Slint) requests a redraw
        ↓
   chrys-gpu compute mirror runs register/classify on the GPU, on the same
   RGBA8 buffers the CPU already rasterized
        ↓
   wgpu texture (diff overlay) → Slint::Image::try_from<wgpu::Texture>()
        ↓
   pan / zoom / frame scrub re-triggers the same GPU path, never the CPU path
```

The window never recomputes the CI verdict. It reads the already-written report artifact for the pass/fail state and uses the GPU path only to paint.

### Key Data Flows

1. **Format erasure happens once, at the adapter boundary.** Every stage after `Source::load()` operates on the same `Frame` shape regardless of family. This is the single most important boundary in the system: it is where the "no per-format special case" rule lives or dies.
2. **The verdict is single-sourced.** Only `chrys-core`'s CPU path writes the report artifact and the baseline manifest. The GPU path and any format-native additive pass (see below) both read from or annotate that same verdict; neither can overrule it.
3. **A format-native pass is additive only.** If a format adds a native comparison detail (for example, PDF text-layer diffing, or SVG path-id diffing), it runs as a *second, clearly labelled* result attached to the same report, never replacing the raster verdict. This is a direct architectural encoding of "one engine, one meaning of changed."

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| Small raster pairs (thumbnails, icons, most CI runs) | CPU-only path is fast enough; GPU mirror is not exercised. |
| Large frame pairs (12000x8000, PDF pages at print DPI, video frames) | Tile the register/classify stages; use the wgpu compute mirror for interactive redraw; CPU path stays single-pass but should tile too, to bound peak memory. |
| Long sequences (video, many-frame animations) | Pipeline frame pairs through a bounded channel so decode of frame N+1 overlaps register of frame N; do not hold the whole sequence in memory. |

### Scaling Priorities

1. **First bottleneck: peak memory on large single frames.** A 12000x8000 RGBA8 buffer is 384 MB per frame; baseline and candidate together, plus a residual field and a pyramid, exceed a gigabyte quickly. Tile the register stage (see the GPU section) and free each pyramid level after use.
2. **Second bottleneck: sequence length.** A 10-minute video at 30 fps is 18000 frame pairs. Stream frame pairs through the pipeline instead of collecting `Vec<Frame>` for the whole sequence; keep only the current window needed for temporal hints.

## Anti-Patterns

### Anti-Pattern 1: A format branch anywhere past the adapter

**What people do:** Add `if format == Pdf { ... }` inside the register or classify stage "just this once," because a format has some special property.
**Why it's wrong:** This is the exact failure mode the project's architecture rule exists to prevent. One exception becomes the template for the next five, and the general engine stops being general.
**Do this instead:** If a format genuinely needs different behavior, it belongs in that format's `Source::load()` (for example, choosing a different rasterization DPI for PDF vs SVG), or as an additive native pass that never touches the shared verdict path.

### Anti-Pattern 2: Letting the GPU path decide the verdict, even "just this once"

**What people do:** Compute the diff on the GPU because it is faster, and treat the CPU path as a fallback for headless CI only, quietly drifting the two implementations apart.
**Why it's wrong:** GPU floating point differs by vendor and driver (different FMA fusion, different transcendental approximations, different reduction order in parallel reductions). The moment the GPU result is authoritative even once, the baseline is no longer portable, which breaks the project's core value.
**Do this instead:** GPU always mirrors, CPU always decides. Bind them with categorical equivalence tests, not bit-equivalence tests.

### Anti-Pattern 3: General image registration

**What people do:** Reach for full affine/homography estimation, RANSAC feature matching (SIFT/ORB), or optical flow, because "it's more robust."
**Why it's wrong:** These are the right tools for unconstrained registration between unrelated images. They are slower, they have failure modes (mismatched features, degenerate homographies) that a near-identical-pair engine never needs to handle, and they reintroduce the research problem the project deliberately opted out of.
**Do this instead:** Phase correlation plus local block matching is the correct scope for a near-identical pair. If a pair is *not* near-identical, the correct answer is "this pair is out of scope for this tool," not a heavier algorithm.

## Integration Points

### External Libraries (per stage)

| Stage | Library | Notes |
|-------|---------|-------|
| SVG rasterize | `resvg` + `tiny-skia` (Rust, pure, no system dependency) | `resvg`'s own documentation claims bit-identical output across x86/ARM and Linux/macOS/Windows, because it avoids system font/graphics libraries. This is the closest existing proof of the project's own determinism claim, and the reference to match behavior against. |
| PDF rasterize | A CPU-only PDF rasterizer (feature-gated) | Must be audited for the same claim `resvg` makes; PDF rasterizers that shell out to system libraries (e.g., poppler via cairo) will not be reproducible. |
| Register (FFT) | A pure-Rust FFT crate, single fixed algorithm, no runtime CPU-feature dispatch that changes numerics | FFT implementations that switch between a naive and a SIMD radix path based on detected CPU features can produce different rounding per machine. Pin one code path for the reference implementation. |
| Baseline store | Filesystem, content-addressed by hash | Modeled on `reg-suit`'s plugin split between a key-generator (what identifies a test) and a publisher (where the blob lives), and on git's own object model (content hash as the identity, not the file path). |
| GPU compute | `wgpu` | Compute pipeline only for register/classify mirror and for painting the diff overlay in the window; never for rasterization. |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| Source adapter to core | `Vec<Frame>` value, no shared state | One-way. The adapter never receives anything back from the core. |
| Core to CLI | Report artifact (owned struct, then serialized) | CLI decides exit code from the artifact's verdict field, never recomputes it. |
| Core to GPU mirror | Same `Frame` buffers, read-only upload to a `wgpu::Texture` | GPU mirror never writes back into a `Frame`; it produces a display-only texture. |
| Core to baseline store | Content hash + blob, through the store trait | Store implementation is swappable without touching the core. |
| Rule file to score stage | Parsed TOML struct, validated once at load | No re-parsing per region; no evaluation, only field lookup. |

---

## 1. The structural comparison algorithm

The near-identical-pair assumption is what turns this from an open registration problem into a short, well-understood pipeline. Each sub-problem below has a named reference implementation to build against, not to reinvent.

### Global geometric alignment: phase correlation

Phase correlation estimates translation between two images from the phase of their cross-power spectrum, and is the standard first step whenever two inputs are expected to be nearly aligned already.

- **Reference:** Kuglin & Hines, "The Phase Correlation Image Alignment Method" (1975), the original method. Reddy & Chatterji, "An FFT-Based Technique for Translation, Rotation, and Scale-Invariant Image Registration" (IEEE Trans. Image Processing, 1996) extends it to rotation and scale via a log-polar transform of the magnitude spectrum, which is exactly the "translation, scale" part of this project's requirement.
- **Subpixel accuracy:** Guizar-Sicairos, Thurman & Fienup, "Efficient subpixel image registration algorithms" (Optics Letters, 2008), gives subpixel translation via an upsampled DFT around the coarse peak, without computing a full upsampled FFT. This is what `scikit-image`'s `phase_cross_correlation` implements, and is the concrete target to match, algorithmically, even though the implementation here is in Rust, not Python.
- **What it produces:** one global translation vector, and (via the log-polar extension) one global scale and rotation estimate, per frame pair. It does not localize *where* something changed; it only answers "how is the whole frame shifted."

### Local residual: coarse-to-fine block matching

After the global transform is removed, remaining local differences (a moved element, a resized region) are found by block matching on a Gaussian/mip pyramid, coarse to fine.

- **Reference:** Bouguet, "Pyramidal Implementation of the Lucas Kanade Feature Tracker" (Intel Corporation technical report, 2000) is the standard coarse-to-fine pattern: estimate motion at the coarsest pyramid level, propagate the estimate down, refine at each finer level. This project needs the pyramid-and-propagate structure, not the Lucas-Kanade optical flow math itself, because the near-identical-pair assumption means block displacement, not per-pixel flow, is the right granularity.
- **Block search pattern:** Zhu & Ma, "A New Diamond Search Algorithm for Fast Block-Matching Motion Estimation" (IEEE Trans. Image Processing, 2000), and Tourapis, "Enhanced Predictive Zonal Search for Single and Multiple Frame Motion Estimation" (VCIP, 2002, EPZS, the fast-search algorithm used in the H.264/HEVC reference software). These are motion-estimation algorithms from video coding, and they apply directly here: given a near-identical pair, most blocks have zero or near-zero displacement, so a small search pattern around the predicted position (diamond, or EPZS's zonal prediction from neighboring blocks) finds the true displacement in a handful of candidate positions instead of an exhaustive search.
- **What it produces:** a per-block displacement and a per-block match residual (typically sum of absolute or squared differences, SAD/SSD). Blocks with near-zero residual after the best match are unchanged. Blocks with high residual even after the best local match are candidates for "added," "removed," or "colour-shifted," not "moved."

### Fast local statistics: integral images

Both the block-match residual computation and any local-window statistic (local mean, local variance, or an SSIM-style windowed comparison used to separate "colour shift" from "structural change") benefit from an integral image (summed-area table), which turns any rectangular-window sum into four array lookups regardless of window size.

- **Reference:** Crow, "Summed-Area Tables for Texture Mapping" (SIGGRAPH, 1984) is the original construction. Viola & Jones, "Rapid Object Detection using a Boosted Cascade of Simple Features" (CVPR, 2001) popularized the same structure, under the name "integral image," for fast rectangular-feature evaluation, which is the terminology most computer-vision code uses today.
- **What it produces:** O(1) windowed sum and sum-of-squares queries, which makes local mean and local variance (needed to separate a colour shift from a structural change) cheap at every block size the coarse-to-fine search visits, instead of recomputing a windowed sum from scratch at each pyramid level.

### Residual segmentation: connected-component labelling

Once the residual field is thresholded into "changed" and "unchanged" pixels, changed pixels are grouped into typed regions (an added region, a removed region, a moved region) by connected-component labelling.

- **Reference:** Wu, Otoo & Suzuki, "Two Strategies to Speed Up Connected Component Labeling Algorithms" (LBNL / Pattern Analysis and Applications, 2009), is the standard two-pass union-find-with-path-compression algorithm most CPU implementations use (this is what `OpenCV`'s `connectedComponents` and `scikit-image`'s `label` implement in spirit). For a GPU-parallel version, Playne & Hawick, "A New Algorithm for Parallel Connected-Component Labelling on GPUs" (IEEE Transactions on Parallel and Distributed Systems, 2018), gives a block-based union-find that resolves cross-block merges with limited atomic contention.
- **What it produces:** a set of typed regions with bounding boxes and pixel counts, which is exactly the shape the classify stage needs to hand to the score stage.

### GPU-friendly versus CPU-only parts

| Sub-step | GPU-friendly? | Why |
|----------|---------------|-----|
| Phase correlation FFT | Yes | FFT is a textbook data-parallel workload; GPU FFT libraries are standard. But the *reference* implementation for the verdict stays CPU, per the hard constraint. The GPU mirror can use a GPU FFT for the interactive redraw. |
| Block match SAD/SSD scoring at candidate positions | Yes | Each candidate position's residual is an independent reduction over a block; embarrassingly parallel across blocks and candidates, maps cleanly to one workgroup per block. |
| Integral image construction | Yes, with care | A summed-area table is a 2D prefix sum, which is a well-known parallel-scan (log-step) GPU pattern, though it needs two passes (row-scan, then column-scan) and careful workgroup-boundary handling. |
| Diamond search / EPZS's *sequential candidate selection* | No, not directly | These algorithms choose the next search center based on the previous step's winner (a data-dependent, sequential decision). The candidate *evaluation* at a fixed set of positions is parallel; the *choice of where to look next* is not. A GPU implementation should replace the sequential-refinement outer loop with a fixed small number of parallel evaluation passes (a small fixed search pattern evaluated in parallel, several times) rather than porting the sequential search loop directly. |
| Connected-component labelling | Partially | The Playne-Hawick style algorithm makes it feasible, but union-find has contention on shared roots, and label propagation across large flat regions needs multiple passes. This is the hardest stage to make both GPU-fast and simple; budget real time for it. |
| Coarse-to-fine pyramid control flow (deciding when to stop refining, whether a block should be reported as "moved" versus "added") | No | This is a small number of sequential, data-dependent decisions over already-reduced values (a handful of floats per block), not a bulk numeric workload. It belongs on the CPU (or as a trivial scalar step after GPU reductions come back), regardless of where the heavy numeric work ran. |

## 2. The pipeline shape

The five named stages (decode, rasterize, register, classify, score, report) are staged as a strict pipeline, each stage owning one transformation and one output shape:

| Stage | Input | Output | Boundary rule |
|-------|-------|--------|---------------|
| Decode | Raw bytes | A format-native structure (SVG DOM, PDF page tree, mesh scene graph, container demux) | Lives entirely inside a `Source` adapter. Never exposed outside the adapter. |
| Rasterize | Format-native structure | RGBA8 pixel buffer(s), one per `Frame` | Also lives inside the adapter. This is the last format-specific step. Its *output type* is fixed for every family; its *implementation* is free to differ. |
| Register | Two `Vec<Frame>` (baseline, candidate) | A residual field plus a coarse global transform, per frame pair | First stage in `chrys-core`. Takes no format knowledge, only pixel buffers. |
| Classify | Residual field | Typed, bounded regions (added/removed/translated/scaled/colour-shifted) | Pure function of the residual; no access to original pixel semantics beyond what the residual already encodes. |
| Score | Typed regions, TOML rules | Pass/fail per region, pass/fail overall | Table lookup only, described in Pattern 3 above. |
| Report | Verdict, regions, frames | Artifact (JSON, overlay image), exit code, baseline manifest update | Terminal stage; nothing reads from this except the CLI, the store, and the window. |

The handoff at each arrow is a concrete, format-erased Rust type. This is deliberate: it means a unit test for the register stage never needs an SVG file or a video file, only two `Frame` values, which keeps the spine testable in isolation before any format adapter exists (see Build Order).

## 3. The plugin boundary for input families

See Pattern 1 above for the trait shape. Three points matter for the roadmap:

1. **The trait must return owned data, not a stream tied to a decoder's lifetime.** `Vec<Frame>` (or a bounded iterator of `Frame` for long sequences, to bound memory per the Scaling section) is simple and format-erased. A trait that instead returned "a handle you can query for pixels" would leak format-specific behavior into how the core calls it, reopening the door to per-format branches.
2. **The hint channel is part of the trait's output, not a side input.** A producer's named-region hints attach to a `Frame`, not to the file path or a separate parameter, so every stage after `Source::load()` sees hints the same way regardless of which adapter produced them.
3. **The 3D mesh adapter is the hardest test of the boundary, and should be built after the boundary is proven, not before.** Rendering eight fixed views and returning them as eight `Frame`s exercises the trait exactly like a video adapter returning many frames does; if the trait needs to change to fit meshes, it is cheaper to discover that after raster, sequences, and one rasterized-vector family have already validated the common case.

## 4. Determinism

Cross-platform, cross-architecture bit-identical CPU rasterization has one existing, working proof: `resvg` (with `tiny-skia`) claims and is designed to produce pixel-identical output on x86 Windows and ARM macOS, specifically because it avoids system font and graphics libraries. This is the direct precedent to match, not a novel claim to invent from scratch.

What makes it work, and what a rasterizer in this project needs:

- **No system library dependency for anything that touches a pixel.** System font rasterizers (FreeType versions differ), system codecs, and system graphics libraries (Core Graphics, GDI+, Cairo-via-system-Cairo) all differ by OS and by installed version. A pure, vendored implementation is the only way to guarantee the same code runs everywhere.
- **Disable FMA contraction.** A fused multiply-add computes `a*b+c` with a single rounding step, while separate multiply and add round twice. Compilers may silently fuse these (`-ffp-contract=fast` is the default in many toolchains) unless told not to. Rust's `f32`/`f64` arithmetic maps to LLVM, so this must be controlled at the LLVM/codegen level: build with `-C target-feature` choices that do not implicitly enable FMA-generating instruction sets for the rasterizer's hot path, or force strict IEEE 754 semantics with the equivalent of `-ffp-contract=off`.
- **No `-ffast-math`-equivalent optimizations.** Rust does not enable fast-math by default, which already helps; the risk is a dependency that does, or a build profile that turns on aggressive vectorization passes that reorder floating point sums.
- **Avoid platform `libm` for any transcendental function in the rasterization path** (`sin`, `cos`, `sqrt` for curve flattening, gradients, or anti-aliasing coverage). Platform `libm` implementations (glibc vs musl vs the Windows CRT vs Apple's) are not required to return bit-identical results for the same input. Use a portable, vendored implementation (Rust's `libm` crate, a pure-Rust reimplementation of fdlibm) so the same code, not just the same algorithm, runs on every OS.
- **Fix SIMD lane width and reduction order, or avoid data-parallel reductions in the rasterizer's numeric core entirely.** Auto-vectorization can choose a different lane width (SSE2 vs AVX2 vs NEON) depending on the target, and summing floats in a different order changes the rounding of the result. Either pin the reduction to a fixed scalar order (simplest, and what a raster fill and blend loop can afford, since these are not the performance-critical bulk-numeric stage in this project, that role is played by the GPU compute mirror), or use a fixed-width SIMD implementation with an explicit, order-preserving reduction, never rely on the compiler's auto-vectorizer to pick a lane width.
- **Pin every dependency version, including transitive ones.** A minor version bump in a font-shaping or curve-flattening dependency can change output by a subpixel. Lockfile discipline is a determinism requirement here, not just a supply-chain one.

### Proving it in CI

The standard method is a CI matrix across OS and architecture that renders the same golden inputs and compares output hashes, not output pixels by eye:

- Run the same rasterization test on `ubuntu-latest` (x86-64), `macos-latest` (aarch64), `windows-latest` (x86-64), and an aarch64 Linux runner, from the same commit.
- Hash the raw RGBA8 output (not a re-encoded PNG, which reintroduces an encoder as a variable) with a fixed algorithm (SHA-256 is sufficient; this is a correctness check, not a security boundary).
- Assert every runner produces the same hash for the same golden input. This is the same mechanism the baseline hash manifest already needs for regression detection (see Baseline Storage below), so the CI determinism proof and the product's own baseline-comparison feature share one implementation.
- Treat a hash mismatch across platforms as a release blocker, and bisect it the same way any other reproducibility bug is bisected: which dependency, which flag, which instruction set changed.

## 5. The GPU comparison layer

wgpu's compute pipeline is a natural fit for the register/classify mirror, structured as follows:

- **Tiling for large pairs (12000x8000).** Do not dispatch one workgroup invocation per pixel over the whole image at once; tile the image into fixed-size chunks (for example, 256x256) that fit comfortably in a workgroup's shared memory budget, and dispatch one workgroup per tile per pyramid level. This is the same tiled-reduction pattern used for tiled matrix multiplication on GPUs: load a tile into shared/workgroup memory once, compute everything that tile's data supports, then move to the next tile, instead of re-reading global memory per output element.
- **Buffer reuse.** Allocate the pyramid levels' storage buffers once, sized for the largest expected pair, and reuse them across frame pairs in a sequence (video, animation) rather than allocating per frame. Chain the phase-correlation FFT pass, the block-match pass, and the labelling pass in a single command encoder submission so intermediate results never leave the GPU between passes. Only the final small result, the typed region list, its bounding boxes, and pixel counts, needs a readback; the full-resolution residual buffer stays GPU-resident and is only read back if the interactive window is actually displaying an overlay at that moment.
- **Readback discipline.** A round trip to the CPU is the expensive operation, not the compute itself. Read back the classify stage's output structure (typically a few hundred bytes to a few kilobytes: region count, per-region bounding box, per-region kind), never the full per-pixel residual, unless the window is actively rendering a zoomed overlay, in which case that overlay texture stays on the GPU and is displayed via `wgpu::Texture`, not read back to the CPU at all.

### Keeping the GPU result "identical enough" to the CPU reference

GPU floating point differs from CPU floating point for reasons that cannot be fully eliminated: different FMA fusion decisions by different vendor compilers, different transcendental function approximations (`sin`/`cos`/`sqrt` in a shader are not required to match a CPU `libm`), and different reduction order in parallel sums. This project's hard constraint, that no pixel entering a comparison is GPU-produced, already prevents the worst case (a GPU-rasterized pixel silently becoming baseline truth). The remaining question is how the GPU-computed *comparison* result relates to the CPU-computed one.

The recommended answer: **do not require numeric equality; require categorical equality.**

- The CPU path in `chrys-core` is the only path that ever writes a verdict to the report artifact or the baseline manifest. This is non-negotiable and follows directly from the core value claim ("same baseline, same verdict, any GPU").
- The GPU path in `chrys-gpu` exists only to paint the interactive window responsively for large frames. It runs the same conceptual algorithm (phase correlation, block match, labelling) but is allowed to disagree with the CPU path on the *exact* residual value at a pixel.
- What must hold, and what should be tested with an equivalence test suite over the golden corpus, is that the GPU path and the CPU path agree on the *classification*: the same regions are found, typed the same way, with bounding boxes that agree within a small pixel tolerance. This is a far weaker and far more achievable bar than bit-for-bit numeric agreement, and it is also the only thing a user actually looks at in the window.
- If the two paths ever disagree categorically (GPU finds a region the CPU does not, or types it differently), that is a bug to fix in the GPU shader, found by the equivalence test suite, not a tolerance to configure around. The report artifact's verdict is unaffected either way, because it never came from the GPU path.

## 6. Baseline storage

Existing visual-regression tools converge on the same three-part shape this project needs:

- **A content-addressed blob store.** Git's own object model is the clearest precedent: an object's identity is the hash of its content, not its file path, so two identical baselines (even across differently named tests) are stored once. `reg-suit` (a visual regression CLI) formalizes this as a plugin split between a *key-generator* plugin (decides what identifies "this version of this test," typically a git commit hash) and a *publisher* plugin (decides where the blob physically lives: local disk, S3, GCS). Percy and Chromatic, the commercial competitors named in this project's own context, both sell the *storage and review workflow* around this same idea, not a better diff algorithm, which is exactly what this project's `PROJECT.md` already observes.
- **A hash manifest checked into the repository.** A small, human-diffable file (test identifier to content hash) is committed alongside the code, so a pull request that intentionally changes a baseline shows a one-line manifest diff, not a binary blob diff, in the review. The actual golden image, in the default backend, is also committed (git-lfs-style, or plain committed binary if the repository's size tolerates it), so the default experience needs no external service at all, matching this project's "no hosted service" constraint.
- **A pluggable backend behind a trait, with committed goldens as the only v1 implementation.** This defers the "content-addressed cache" and "remote store" ambitions to a later milestone without blocking v1, and matches this project's own stated plan ("a pluggable store behind it and committed goldens as the default backend").

## 7. Suggested build order

The dependency reasoning follows directly from the component boundaries above, and from the project's own stated preference to freeze the architecture before staging formats, and to build core-then-CLI before the window (an already-paid-for lesson from a sibling project, `mandible`).

1. **The frozen spine: `chrys-core` against a single trivial family (raster images).** Build register, classify, score, and report against `Vec<Frame>` directly, using the raster family as the only adapter, before the `Source` trait is even formalized as a public boundary. This is the cheapest way to validate the algorithms in isolation (Sections 1 and 2 above) without any format complexity in the way. Nothing here can be parallelized against later work, because every later stage depends on this one being correct and stable.

2. **The determinism proof, as CI infrastructure, immediately after the spine works.** Stand up the multi-OS/multi-arch CI matrix and the hash-equality check (Section 4) while the surface area is still small (one rasterizer, raster images only). Finding a determinism bug against one simple family is tractable; finding one after six families and a GPU mirror both exist is not. This also produces the hash-manifest mechanism the baseline store needs, so it is not wasted infrastructure.

3. **Extract the `Source` trait, and prove it with a second, low-risk family (animation and frame sequences).** This family reuses the raster rasterizer entirely; it only adds "many frames, paired by index" to the pipeline, which is the minimum change needed to validate that the trait boundary (Section 3) and the pipeline's per-frame-pair looping (Section 2) are shaped correctly, without also introducing a new rasterizer at the same time.

4. **The baseline store (committed goldens, hash manifest, TOML rule engine, CLI, report artifact).** These can now be built against the two families already working, and are needed before any more formats are added, because every subsequent format's own tests depend on being able to commit a golden and get a pass/fail exit code. This is also the point at which the project becomes usable end to end for the simplest case, which is valuable to prove before investing in the harder families.

5. **SVG, as the first family with a real CPU rasterizer.** This is the first place the determinism claim is tested against non-trivial rendering (curves, gradients, anti-aliasing), using `resvg`/`tiny-skia` as the direct precedent (Section 4). It validates the decode-then-rasterize boundary inside an adapter for the first time.

6. **PDF, behind its Cargo feature.** Structurally identical to SVG (decode a document, rasterize pages as frames), so it is next while the "rasterize inside the adapter" pattern is fresh, but it is feature-gated from the start per the project's own dependency constraint, and its rasterizer must be independently audited for the same determinism claim, since many PDF rasterizers shell out to non-reproducible system libraries.

7. **The wgpu compute mirror and the Slint window.** This depends on `chrys-core` already being stable (Pattern 2 requires a CPU reference to test against) and depends on the CLI already existing headless (the project's own precedent from `mandible`: a face that needs a tty cannot be tested in CI or by an agent). It is deliberately sequenced after several formats exist, so the interactive view has real, varied content to exercise, and so GPU work, new to this author, is not blocking any format work in the meantime.

8. **Video, behind its Cargo feature.** Reuses the frame-sequence pairing logic from step 3 entirely; its new risk is decode (the project's own open question: whether hardware and software decode agree bit-exact), which should be proven against a corpus before the feature is trusted, exactly as `PROJECT.md` already flags.

9. **3D mesh, through the fixed eight-view rig.** Sequenced last because it is the hardest test of the `Source` trait (Section 3) and depends on a working CPU rasterizer pattern (steps 5 to 6) plus a stable pipeline (steps 1 to 4) to render each of the eight views against. It is also, along with the GPU work in step 7, one of the two areas genuinely new to this author, so it benefits most from every other part of the system already being proven stable underneath it.

**Ordering rationale, summarized:** algorithm correctness (1) before determinism proof (2) before extension mechanism (3) before product usability (4) before harder rasterizers (5, 6) before GPU/UI (7) before the two riskiest, newest-to-the-author families (8, 9). Each step only adds one new kind of risk at a time, which keeps a failure easy to attribute to the thing that just changed.

## Sources

- Kuglin & Hines, "The Phase Correlation Image Alignment Method" (1975)
- Reddy & Chatterji, "An FFT-Based Technique for Translation, Rotation, and Scale-Invariant Image Registration," IEEE Trans. Image Processing (1996)
- Guizar-Sicairos, Thurman & Fienup, "Efficient subpixel image registration algorithms," Optics Letters (2008) — implemented as `scikit-image`'s `phase_cross_correlation`
- Bouguet, "Pyramidal Implementation of the Lucas Kanade Feature Tracker," Intel Corporation (2000)
- Zhu & Ma, "A New Diamond Search Algorithm for Fast Block-Matching Motion Estimation," IEEE Trans. Image Processing (2000)
- Tourapis, "Enhanced Predictive Zonal Search for Single and Multiple Frame Motion Estimation" (EPZS), VCIP (2002)
- Crow, "Summed-Area Tables for Texture Mapping," SIGGRAPH (1984)
- Viola & Jones, "Rapid Object Detection using a Boosted Cascade of Simple Features," CVPR (2001)
- Wu, Otoo & Suzuki, "Two Strategies to Speed Up Connected Component Labeling Algorithms," LBNL / Pattern Analysis and Applications (2009)
- Playne & Hawick, "A New Algorithm for Parallel Connected-Component Labelling on GPUs," IEEE TPDS (2018)
- [resvg — reproducible rasterization claim and design](https://github.com/linebender/resvg)
- [tiny-skia](https://github.com/linebender/tiny-skia)
- [reg-suit — plugin architecture for baseline storage](https://github.com/reg-viz/reg-suit)
- [Floating-Point Determinism, Random ASCII (Bruce Dawson)](https://randomascii.wordpress.com/2013/07/16/floating-point-determinism/)
- [Floating Point Determinism, Gaffer On Games](https://gafferongames.com/post/floating_point_determinism/)
- [odiff — SIMD image comparison, YIQ color distance](https://github.com/dmtrKovalenko/odiff)
- [Rust wgpu compute: buffer readback and performance patterns](https://tillcode.com/rust-wgpu-compute-minimal-example-buffer-readback-and-performance-tips/)
- [WebGPU compute shaders and tiled workgroup patterns, toji.dev](https://toji.dev/webgpu-best-practices/compute-vertex-data.html)

---
*Architecture research for: Chrysoberyl (structural media diff engine, Rust)*
*Researched: 2026-09-06*
