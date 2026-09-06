# Stack Research

**Domain:** Deterministic, cross-platform structural diff tool for non-text media, written in Rust (core crate + CLI + later Slint window)
**Researched:** 2026-09-06
**Confidence:** MEDIUM-HIGH (crate identities, licenses and the determinism claim for SVG rasterization are HIGH confidence and cross-checked against crates.io/lib.rs/docs.rs; version numbers pulled the same day carry HIGH confidence; the CPU 3D mesh rasterization layer has no mature crate and is LOW confidence by nature of the gap, not by weak research)

## Hard Constraints Applied

- Core crate: MIT, pure Rust. Video and PDF sit behind Cargo features, off by default.
- No pixel that enters a comparison is produced by a GPU. wgpu touches only already-rasterized frames.
- Direct dependency count is a stated value (README badge). Every recommendation below notes its direct dependency weight and flags anything that pulls a large transitive tree.
- Native on Linux, macOS, Windows from one codebase.

---

## Layer 1: Deterministic CPU Rasterization of SVG and Vector Content

### Recommended: `resvg` 0.48.1 + `tiny-skia` 0.12.0

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| resvg | 0.48.1 | MIT OR Apache-2.0 | SVG parse + layout + render orchestration |
| tiny-skia | 0.12.0 | BSD-3-Clause | The actual CPU rasterizer (a Rust port of a subset of Google Skia's software backend) |
| usvg | (bundled with resvg) | MIT OR Apache-2.0 | SVG normalization/simplification pass resvg runs before rasterizing |

**Why:** resvg's own documentation makes the determinism claim explicit: it does not call into any system library (no Cairo, no system FreeType, no platform font-rendering backend), so rendering an SVG on x86 Windows and on ARM macOS produces bit-identical pixels. This is the only Rust SVG stack that states and is built around that guarantee rather than treating it as an accident of the platform. tiny-skia itself has no floating-point paths that depend on platform libm transcendental functions for the geometry it rasterizes (Bezier flattening and scanline fill use fixed algorithms, not platform sin/cos), which is exactly the determinism failure mode that sinks naive "use whatever's on the system" approaches.

Both crates are dual MIT/Apache-2.0 or BSD-3-Clause: fully compatible with an MIT core, no copyleft to report.

**Dependency weight:** resvg pulls a small, deliberate dependency set (bytemuck, gif, image-webp, zune-jpeg, log, pico-args, rgb, svgtypes, tiny-skia, usvg — roughly 9-10 direct deps, ~10 crates total in the tree including font handling). This is lean by graphics-stack standards.

**Confidence:** HIGH. Determinism claim verified against resvg's own stated design goal, not inferred.

### Rejected alternatives

| Rejected | Why not |
|----------|---------|
| `raqote` | Unmaintained since ~2021, thinner feature set, no SVG parser bundled — would need pairing with a separate SVG frontend with no track record of determinism testing. |
| `femtovg` / `vello` | Both are GPU-first (WebGPU/wgpu-backed). Vello in particular is explicitly a GPU compute-shader rasterizer — this is disqualified outright by the hard constraint that no comparison pixel may come from a GPU. |
| System rendering via `resvg`'s Skia backend, Cairo, or platform WebView/CoreGraphics | Any system-library or platform-native rendering path reintroduces exactly the vendor/driver/font-hinting variance this project exists to eliminate. Do not shell out to `rsvg-convert`, Chromium headless, or Inkscape for rasterization; each renders slightly differently per OS and per installed version. |

---

## Layer 2: Deterministic CPU Rasterization of PDF Pages

This is the layer where every candidate has a license or determinism caveat. State them precisely.

### Recommended: `pdfium-render` 0.9.3, feature-gated, off by default

| Crate | Version | License | Notes |
|-------|---------|---------|-------|
| pdfium-render | 0.9.3 | MIT OR Apache-2.0 | Idiomatic Rust wrapper (~35 dependencies at most feature-complete, includes `image`, `chrono`, `log`; can be trimmed) |
| pdfium (the C++ engine, via Google's prebuilt binaries or `bblanchon/pdfium-binaries`) | rolling Chromium release | Predominantly BSD-3-Clause, with a bundle of third-party licenses (FreeType — FreeType License or GPLv2 dual, libjpeg — custom permissive BSD-like, libopenjpeg, zlib, etc.) shipped in `LICENSE` and `BUILD_LICENSES/` | Not a crate. A native library, downloaded as a prebuilt dynamic library (`libpdfium.so`/`.dylib`/`.dll`) or linked via `pdfium-render`'s `pdfium-latest` / static-linking feature |

**Precise license accounting:** pdfium-render itself (the Rust binding crate) is dual MIT/Apache-2.0 — clean. Pdfium (the engine) is Google's own BSD-3-Clause code plus a set of bundled third-party libraries under their own permissive licenses (FreeType's FTL, a BSD-like libjpeg license, zlib). None of these are copyleft. This is the cleanest license story of any full-fidelity PDF renderer available to Rust. Because pdfium ships as a prebuilt native binary rather than source pulled into the Cargo build, license text must still be redistributed alongside the binary — document this in the PDF feature's README section, but it does not touch the MIT badge on the core crate since the feature is off by default and the native library is not a Rust dependency.

**Determinism caveat, stated plainly:** Pdfium is not proven bit-exact across OS and CPU architecture the way resvg's software path is. It uses its own software rasterizer for most content (no GPU), but font substitution, hinting, and some anti-aliasing choices can differ by platform build and by embedded-font-availability. Treat PDF rasterization as "deterministic per pinned pdfium build and pinned fonts," not "deterministic by construction" the way SVG is. Pin the exact pdfium binary version (via `bblanchon/pdfium-binaries` release tag) in the lockfile-equivalent for this feature, and prove bit-exactness across the three target OSes with a golden-image test before trusting it, exactly as PROJECT.md's open question already flags for video.

**Confidence:** HIGH on the license facts (verified against pdfium's own LICENSE and third-party bundle). MEDIUM on the determinism claim — no independent bit-exactness study was found; this must be validated empirically per PROJECT.md's stated open-question pattern.

### Rejected alternatives

| Rejected | License | Why not |
|----------|---------|---------|
| `mupdf-rs` / `mupdf-sys` | **AGPL-3.0** | MuPDF's license is AGPL unless you buy Artifex's commercial license. Linking this into any binary, even behind a Cargo feature, makes the resulting binary AGPL-encumbered for anyone who builds with that feature on. This directly violates the project's "core is MIT, report license terms exactly" constraint the moment the feature is compiled in, and AGPL's network-use clause is the strongest copyleft available — do not use even as an optional feature without an explicit, loud warning and ideally a build-time refusal unless the user opts in with eyes open. |
| `poppler` bindings (`poppler-rs`, etc.) | Poppler is GPL-2.0-or-later | Same category of problem as MuPDF, GPL rather than AGPL, still copyleft, still wrong for a feature that should be droppable into any downstream build. |
| Pure-Rust PDF renderers (`pdf-writer`, `pdf` crate, `lopdf`) | Various permissive | None of these are page rasterizers — `pdf-writer` only writes PDFs, `lopdf`/`pdf` parse the object graph but do not paint pixels. No mature pure-Rust PDF rasterizer exists as of this research. This is a real gap; pdfium via FFI is the only practical full-fidelity option today. |

---

## Layer 3: Raster Image Decode/Encode (PNG, JPEG, WebP, AVIF, TIFF, animated WebP, APNG)

### Recommended: `image` 0.25.10, with `default-features = false` and explicit format features

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| image | 0.25.10 | MIT OR Apache-2.0 | Umbrella decode/encode API, format dispatch |
| png | (image's dependency, also usable standalone) | MIT OR Apache-2.0 | PNG and APNG, pure Rust, decode + encode |
| image-webp | (image's dependency, also usable standalone) | MIT OR Apache-2.0 | Pure-Rust WebP: decodes lossless, lossy, alpha, **and animated** WebP; encodes lossless WebP only (no lossy or animated encode yet) |
| zune-jpeg | (image's dependency) | MIT OR Apache-2.0 OR Zlib | Pure-Rust JPEG decode, fast, replaces libjpeg-turbo without the C dependency |
| tiff (image-rs org) | 0.8+ | MIT OR Apache-2.0 | TIFF decode/encode, pure Rust |
| ravif | 0.13.0 | BSD-3-Clause | AVIF **encode**, wraps rav1e |
| rav1e | (ravif's dependency) | BSD-2-Clause | AV1 encoder used for AVIF encode path |

**Why `image` and not a rebuild from parts:** the `image` crate is the de facto standard, with the widest format coverage and the most-audited decode paths in the Rust ecosystem (80M+ downloads). Critically for the dependency-count goal, it is feature-gated per format — turning off `default-features` and enabling only `png`, `jpeg`, `webp`, `tiff`, `avif` pulls in only the pure-Rust decoders listed above, not the full matrix. AVIF **decode** in `image` defaults to a `libdav1d`-backed path (`avif-decoder` feature); prefer wiring this to `rav1d` (see Layer 4) for a pure-Rust decode path where possible, falling back to the C dav1d only if `rav1d` proves too slow for large corpora.

**Animated WebP and APNG:** both are covered without a native library. `image-webp`'s decoder explicitly supports animation frames; the `png` crate's APNG support handles the `acTL`/`fcTL`/`fdAT` chunks in pure Rust. This is the one area where the pure-Rust stack fully satisfies the "no C library needed" preference without compromise.

**Dependency weight:** with formats trimmed to exactly what's needed (PNG, JPEG, WebP, TIFF, AVIF-encode), the `image` tree stays in the range of a dozen or so transitive crates, not the 40+ you get with every optional decoder turned on. Audit with `cargo tree` per feature set before locking the README badge number.

**Confidence:** HIGH. Version, license and per-format pure-Rust status verified directly.

### Rejected alternatives

| Rejected | Why not |
|----------|---------|
| `libwebp-sys` / `webp` (libwebp bindings) | Pulls a C library (BSD-licensed, so not a legal problem, but it is a native build dependency on all three OSes, meaning a C compiler and cross-compilation setup on every target — exactly the friction `image-webp`'s pure-Rust path avoids). Only reach for it if `image-webp`'s lossy/animated encode gap becomes a blocker. |
| `zune-image` (the umbrella crate around zune-jpeg etc.) | A reasonable alternative umbrella (MIT/Apache-2.0/Zlib, fast, fewer deps than `image` in some configurations), but has less mature TIFF and AVIF coverage and a smaller install base/audit history than `image`. Worth revisiting if `image`'s dependency count becomes the binding constraint later; not the first choice today. |
| `avif-decode` / `libavif` bindings | Wraps `libaom`/`dav1d` C libraries directly; redundant once `rav1d` (Layer 4) is already a project dependency for video, and reintroduces a C toolchain requirement for a format `image`+`rav1d` can already cover. |

---

## Layer 4: Video Demux and Decode

State plainly: **there is no pure-Rust, C-free path that covers every codec.** Pick per-codec, feature-gate the whole layer, and document exactly which sub-path is pure Rust and which needs a system library.

### Recommended stack (all behind a single `video` Cargo feature, off by default)

| Crate | Version | License | Pure Rust? | Role |
|-------|---------|---------|------------|------|
| symphonia | 0.5.x | **MPL-2.0** | Yes | Container demux (MP4/ISOBMFF, MKV/WebM, etc.) and audio decode. Does not decode video frames itself. |
| rav1d | current (memorysafety/rav1d) | BSD-2-Clause | Yes | AV1 video decode. A line-by-line safe-Rust port of VideoLAN's dav1d, with an explicit, tested claim of **bit-exact** output versus dav1d C, frame by frame, for both 8-bit and 10-bit streams, filters on and off. This is the strongest determinism claim of any codec option in this whole stack. |
| openh264 (+ openh264-sys2) | 0.9.x / current | BSD-2-Clause (Cisco's OpenH264, source vendored, compiled via `cc` in `build.rs`) | No — needs a C compiler at build time, but no external runtime dependency to install (source is vendored in the crate) | H.264 decode, for the enormous existing corpus of H.264 video the project will be asked to diff |
| ffmpeg-next (+ ffmpeg-sys-next) | current | MIT (the Rust binding) wrapping **FFmpeg itself, which is LGPL-2.1+ or GPL-2+/3+ depending on build configuration** | No — links a system or vendored FFmpeg build | Fallback demux/decode for codecs and containers the above don't cover (H.265, VP9, ProRes, legacy formats) |

**License accounting, precisely:**
- symphonia is MPL-2.0. File-level copyleft: modifying symphonia's own source and distributing that modification requires releasing those file changes under MPL-2.0. Merely depending on it and linking it does not force the rest of the binary under MPL. This is compatible with shipping an MIT-licensed video feature, but it must be disclosed (MPL requires a copy of the license and attribution), and the project may not relicense symphonia's own files.
- rav1d is BSD-2-Clause. Clean, permissive, no disclosure burden beyond attribution.
- openh264's binding crate is BSD-2-Clause; OpenH264 itself is BSD-2-Clause with a Cisco patent grant covering H.264 patents specifically for users of Cisco's binary — read that patent grant's exact scope before assuming it extends to a self-compiled build; the source-vendored `openh264-sys2` path is the safer of the two.
- ffmpeg-next's own code is MIT. **FFmpeg the C library is LGPL-2.1-or-later by default, and GPL-2-or-later if built with certain encoders (e.g. x264) enabled.** If this project ever links a GPL-configured FFmpeg build, the resulting binary is GPL-encumbered as a whole. This is the single biggest license trap in the entire stack: keep any FFmpeg build used here strictly LGPL-configured (no `--enable-gpl`), document the exact `ffmpeg -version`/configure flags of the binary shipped or required, and treat this as a hard build-time check, not a comment.

**Bit-exactness across implementations:** rav1d/dav1d is the only pairing in this layer with a proven, stated bit-exact guarantee (see above). H.264 via OpenH264 is not proven bit-exact against, say, an OS's hardware decoder or against libavcodec's own H.264 decoder — do not assume it. PROJECT.md's own open question is correct to flag this: prove decode agreement empirically against a conformance corpus before the video feature's baseline depends on cross-decoder agreement. Where a choice exists, prefer software decode paths (rav1d, OpenH264) over any OS hardware-accelerated decode path, since hardware decoders are exactly as unreproducible across vendors as GPU rasterization is, for the same reason (fixed-function silicon rounds differently per vendor).

**Confidence:** HIGH on crate identities and license terms (each verified independently). HIGH on rav1d's bit-exactness claim (it is the project's own stated, tested guarantee). LOW/unverified on H.264 or FFmpeg-path cross-decoder bit-exactness — treat as an open question requiring the corpus test PROJECT.md already calls for.

### Rejected alternatives

| Rejected | Why not |
|----------|---------|
| GStreamer (`gstreamer-rs`) | The Rust bindings are dual MIT/Apache-2.0, but GStreamer core is LGPL-2.1+ and many commonly-used plugins (`gst-plugins-bad`, `gst-plugins-ugly`) are GPL. Pulling in "the codec you need" from GStreamer's plugin ecosystem risks a GPL plugin sneaking into the build. Also a much heavier system dependency (a whole plugin-discovery runtime) than a targeted decoder crate, working against the "count the dependencies" ethos. |
| Hardware-accelerated decode (VideoToolbox, D3D11VA/DXVA, VAAPI/NVDEC) | Explicitly the same determinism failure as GPU rasterization: different vendor, different silicon, different rounding. Do not use for any frame that enters the comparison pipeline, even though it is tempting for playback speed in the eventual Slint viewer — display-only decode paths are fine, comparison-path decode is not. |
| `rav2d` (AV2 decoder) | AV2 is not yet a deployed, widely-encountered codec for the media this tool will realistically be asked to diff in 2026. Interesting but premature; revisit if AV2 content becomes common in the corpus. |

---

## Layer 5: Software (CPU) Rasterization of 3D Meshes, and Mesh File Loading

This is the weakest part of the current Rust ecosystem for this project's needs, and that should be stated plainly rather than papered over.

### Mesh file loading: recommended, per format

| Format | Crate | Version | License | Notes |
|--------|-------|---------|---------|-------|
| glTF | gltf | 1.4.1 | MIT OR Apache-2.0 | The reference glTF 2.0 loader. ~7 direct deps (byteorder, serde_json, gltf-json, lazy_static; base64/image/urlencoding optional for embedded data URIs). Mature, widely used. |
| OBJ | tobj | 4.0.4 | MIT | Small, focused, matches tinyobjloader's behavior; minimal deps. |
| PLY | ply-rs-bw | 2.0.0 | MIT | The original `ply-rs` is unmaintained; `ply-rs-bw` is the maintained fork (forked specifically to clear a transitive CVE in `linked-hash-map`). Use the fork, not the original crate name. |
| STL | stl_io | 0.10.0 | MIT | Reads binary and ASCII STL safely; writes binary STL only (no ASCII STL writer, rarely needed). |

All four are permissive-licensed and individually small. Pull in only the format(s) actually needed per Cargo feature, consistent with the project's dependency-counting posture.

### CPU triangle rasterization: no mature crate exists — recommend hand-rolled

**Finding, stated directly:** there is no maintained, widely-used, production-grade pure-Rust crate for CPU (software) triangle rasterization of 3D meshes with a z-buffer, comparable to what `tiny-skia` is for 2D vector paths. What exists are hobbyist/tutorial-grade single-purpose projects (e.g. small "software rasterizer" learning repos), not crates published to crates.io with any maintenance track record, feature completeness, or determinism testing.

**Recommendation:** given the project's fixed eight-view rig requirement (PROJECT.md: no configurable camera, a small fixed set of views) and its "count the dependencies" ethos, this is a case where **writing a small, in-house scanline/half-space z-buffer rasterizer is the correct engineering choice**, not a gap to be filled by a dependency. The scope is narrow and known in advance: fixed orthographic or fixed-FOV perspective views, flat or simple Lambertian shading (no need for a general-purpose shader pipeline), a single z-buffer per view, output as a raster frame that flows into the same comparison pipeline as every other input kind. This keeps the determinism guarantee fully in the project's own hands — the exact arithmetic (fixed-point or IEEE-754 with a documented, invariant operation order) is visible and testable rather than inherited from an opaque dependency, and it adds zero third-party dependencies to the badge count for this layer.

Build it on top of:

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| glam | 0.29.x | MIT OR Apache-2.0 | Vector/matrix math (SIMD-friendly, but avoid enabling its SIMD feature-gated fast-math paths for the rasterizer's core arithmetic if bit-for-bit cross-arch determinism is required — verify scalar fallback path behavior explicitly) |

**Determinism caveat for the hand-rolled path:** IEEE-754 arithmetic is deterministic given an identical sequence of identical operations, but two failure modes must be actively guarded against: (1) compiler auto-vectorization or FMA contraction changing operation order/rounding between `-C target-cpu` settings on different CI runners — pin `-C target-feature` explicitly or structure the inner loop to be FMA-contraction-safe; (2) any call into a transcendental function (`sin`, `cos`, `sqrt` is IEEE-754-specified and fine, but `sin`/`cos`/`atan2` are not standardized to the same ULP across libm implementations) — the fixed eight-view rig should use precomputed view/projection matrices rather than computing trig per-frame, sidestepping this entirely.

**Confidence:** HIGH that no mature crate exists (searched crates.io and lib.rs specifically for this). HIGH that hand-rolling fits the project's stated constraints better than any available crate. MEDIUM on the specific determinism guidance above — sound general Rust/IEEE-754 knowledge, not verified against a published case study for this exact use.

### Rejected alternatives

| Rejected | Why not |
|----------|---------|
| `three-d`, `bevy` (rendering via its software fallback), `wgpu` itself for mesh rasterization | All are GPU-first (wgpu-backed) rendering stacks. Disqualified outright by the no-GPU-pixels constraint. `bevy` in particular pulls a huge transitive dependency tree, the opposite of this project's stated priority. |
| `rasterizer`/`black`/`minirender`-style hobby crates found during research | Unmaintained or single-author demo projects with no version discipline, no test suite verifying cross-platform output, and no crates.io presence in most cases. Adopting one as a dependency trades a small amount of code-writing now for an unowned, unaudited dependency later — worse trade than writing the ~200-400 lines this rig actually needs. |
| `meshopt` (Rust port of `meshoptimizer`) | Solves a different problem (mesh simplification/optimization for GPU rendering throughput), not rasterization. Might be useful later if mesh complexity becomes a performance problem, not part of the core rasterization path. |

---

## Layer 6: GPU Compute with wgpu for Image Comparison Work

### Recommended: `wgpu` 26.0.0

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| wgpu | 26.0.0 | MIT OR Apache-2.0 | Cross-platform GPU access (Vulkan, Metal, D3D12, and OpenGL/GLES as fallback) for compute passes over already-rasterized frames |
| naga | 26.0.0 (locked to the same version as wgpu) | MIT OR Apache-2.0 | WGSL-to-native-shading-language translation; ships as wgpu's internal shader compiler, rarely a direct dependency you invoke yourself |

**Why wgpu, restated from PROJECT.md's own constraint:** it is the only Rust graphics API that reaches Metal, D3D12 and Vulkan from one codebase, and it is what Slint's own renderer can share a device/queue with (`set_rendering_notifier()`, `slint::Image::try_from<wgpu::Texture>()`), so the same GPU context serves both comparison compute and the eventual desktop window without standing up two separate graphics stacks.

**What the compute-shader story looks like for this project in 2026:** wgpu's compute pipeline API (`ComputePipeline`, `ComputePass`, storage buffers/textures, workgroup-shared memory) is mature and stable at this version; write comparison kernels (block matching via SSD/SAD reduction, phase correlation via FFT-adjacent butterfly passes or a simpler cross-correlation approach, tree reductions for aggregate error scores) directly in WGSL. wgpu versions have moved on a roughly year-cadence major-version bump scheme since dropping the pre-1.0 `0.x` numbering; expect a compatible-but-not-identical API surface at the next major bump, so pin the exact version and re-verify before upgrading rather than tracking latest continuously.

**Critical determinism reminder specific to this layer:** the hard constraint is that no pixel entering a comparison may come from a GPU — this layer is compute-only, over CPU-rasterized input, and it must never write back a GPU-computed pixel value into what gets called "the baseline." GPU floating-point reduction order (workgroup-size-dependent, driver-dependent) is not guaranteed bit-identical across vendors even for the same WGSL source — that's fine for computing a similarity score or a match offset (a scalar or small vector result, tolerant of ULP-level noise), but do not use the GPU compute path to produce or modify any frame that later serves as a stored baseline artifact.

**Dependency weight:** wgpu itself is a substantial crate with a real transitive tree (its own backend bindings for Vulkan/Metal/D3D12), but it is the single unavoidable cost of the "reach three graphics APIs from one codebase" requirement PROJECT.md already committed to — there is no lighter alternative that meets that requirement, so it is accepted as the one deliberately heavy dependency, distinct from every other layer's lean-by-default posture. Since the CLI/core doesn't need a GPU at all for its primary job (CPU rasterization + comparison), gate wgpu behind a `gpu-compute` or `ui` feature too, so the MIT/pure-Rust slim core build genuinely doesn't need a GPU driver present to run.

**Confidence:** HIGH on version and API maturity (wgpu's compute pass API has been stable since well before this version). MEDIUM on the "workgroup-size-dependent reduction is non-deterministic across vendors" claim — this is well-established general GPU-compute knowledge, not a wgpu-specific citation, but it is not something a specific 2026 source was checked against.

### Rejected alternatives

| Rejected | Why not |
|----------|---------|
| `ash` (raw Vulkan bindings) | Reaches only Vulkan, not Metal or D3D12 — fails the "one codebase, three systems" constraint outright without a second backend for macOS. |
| `metal`/`d3d12` crates directly | Same problem in reverse: single-platform, would require three separate compute backends maintained in parallel. wgpu exists specifically to avoid this. |
| CUDA / `cudarc` | NVIDIA-only, fails cross-platform and cross-vendor requirements entirely; also pulls a proprietary runtime dependency the project has no reason to require. |
| CPU-only comparison (skip GPU entirely) | Viable for small images but block matching and phase correlation over large frames/video is exactly the workload GPU compute exists to accelerate; PROJECT.md already commits to wgpu for this reason. Worth keeping a CPU fallback path for portability/testing, but not as the primary path. |

---

## Layer 7: Perceptual Metrics (SSIM, Colour Difference, Butteraugli-like)

### Recommended: `image-compare` 0.5.0 for SSIM, `empfindung` for CIEDE2000, `ssimulacra2` for a Butteraugli-adjacent metric

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| image-compare | 0.5.0 | MIT | SSIM and RMS comparison for grayscale/RGB images, plus histogram-distance metrics; built directly on the `image` crate; small, focused, permissively licensed |
| empfindung | current | MIT (verify at adoption time — not independently re-checked this session, but stated MIT by its author's parallel `DeltaE` project) | Pure-Rust CIEDE2000, CIE94, CIE76, CMC l:c colour-difference implementations |
| ssimulacra2 | 0.5.1 | BSD-2-Clause | Rust implementation of SSIMULACRA2, a modern perceptual quality metric considered strong for medium/low-fidelity comparisons; complements SSIM |

**Why not `dssim` for the primary SSIM path:** `dssim`/`dssim-core` is **dual-licensed AGPL or commercial** — a direct conflict with the "core is MIT" constraint the moment it's linked into anything shipped, even as a feature. `image-compare` gives the same core capability (multiscale SSIM-family comparison) under a plain MIT license with no dual-licensing trap, at the cost of being a smaller, less battle-tested project than `dssim`. Given the license constraint is non-negotiable per PROJECT.md, this is not a close call.

**Butteraugli specifically:** no direct, maintained Rust port of Google's Butteraugli was found as an independent crate (the canonical Butteraugli lives in `libjxl`, C++, BSD-3-Clause-ish, not ported to Rust). `ssimulacra2` is the closest available Rust-native equivalent in spirit — a modern, multi-artifact-aware perceptual metric — and multiple sources note it's commonly used alongside Butteraugli-style metrics precisely because they catch different artifact classes (SSIMULACRA2 for medium/low fidelity, Butteraugli-style for very high fidelity). If true Butteraugli-equivalent behavior becomes a requirement, the only path today is FFI to `libjxl`'s Butteraugli implementation (BSD-3-Clause-ish, but a C++ dependency), not a pure-Rust crate — flag this as a gap for a future phase rather than solving it now.

**Confidence:** HIGH on `image-compare` and `ssimulacra2` license/version facts (independently verified). MEDIUM on `empfindung`'s exact license (recalled from its GitHub description in-session, not independently fetched from its Cargo.toml/crates.io license field this session — verify before depending on it). HIGH on the absence of a mature Rust Butteraugli port (searched directly, found none).

### Rejected alternatives

| Rejected | Why not |
|----------|---------|
| `dssim` / `dssim-core` | AGPL-or-commercial dual license — incompatible with shipping under a plain MIT core without a commercial license purchase. Reconsider only if the project ever accepts a paid Kornel Lesiński commercial license explicitly, and even then keep it opt-in and clearly labeled. |
| `delta_e` crate (the other CIEDE2000 implementation found) | Functionally overlapping with `empfindung`; either is fine, listed as an explicit alternative rather than a rejection — pick one, don't depend on both. |
| Rolling a custom SSIM/CIEDE2000 implementation from the papers | Possible, and consistent with the "hand-roll layer 5" posture, but these are well-defined, numerically fiddly algorithms (SSIM's windowing and Gaussian weighting, CIEDE2000's several correction terms) where an existing, tested, permissively-licensed crate is lower-risk than a fresh implementation. Unlike the 3D rasterizer gap, mature options exist here, so use them. |

---

## Layer 8: CLI, TOML, Error Handling, Test Harness

### Recommended

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| clap | 4.6.6 (derive feature) | MIT OR Apache-2.0 | CLI argument parsing. The standard choice; derive macros keep the CLI crate's own code minimal even though `clap` itself has a moderate dependency tree (largely from `clap_builder`/`anstyle`/`strsim` for help-text formatting) — worth `cargo tree`-auditing if the badge counts transitively. |
| toml_edit | 0.25.x (or plain `toml` if round-trip/comment-preservation isn't needed) | MIT OR Apache-2.0 | TOML parsing for the declarative rule file. Prefer `toml_edit` only if the tool ever needs to *write back* a rule file preserving comments/formatting (e.g. an interactive rule-authoring mode); for pure read-only rule parsing, the plain `toml` crate (thin wrapper over `toml_edit` internally, via `serde`) is the smaller, simpler dependency. |
| serde + serde_derive | 1.x | MIT OR Apache-2.0 | Underlies both TOML parsing and any manifest/baseline serialization; effectively unavoidable in the Rust ecosystem at this project's scope, and worth treating as a "free" foundational dependency rather than counting it as a discretionary choice. |
| thiserror | 2.0.20 | MIT OR Apache-2.0 | Library-side error types (core crate) — derives `std::error::Error` without runtime cost, keeps error variants explicit and matchable, which a comparison tool's callers (CI systems parsing exit codes/report artifacts) will want. |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | CLI-side error handling only — use `thiserror` in the core library crate (typed, matchable errors) and `anyhow` only in the CLI binary crate (convenient `?`-propagation, context strings, doesn't need to be a stable public API). Do not use `anyhow` in the core library's public API — it erases error types that library consumers may need to match on. |

### Test harness, specifically for a project whose core deliverable is "the same verdict on any machine"

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| insta | (cargo-insta 1.48.0) | Apache-2.0 | Snapshot testing — exactly the right shape for "given this pair, assert this exact structural-diff report," and for locking in golden baseline hashes per PROJECT.md's "committed goldens as the default backend" requirement. This is the test harness this project's core value proposition needs, more than a generic assertion library. |
| trycmd | current | MIT OR Apache-2.0 | Blunt, high-volume CLI integration testing — run the compiled binary against a directory of `.toml` cases and expected stdout/exit codes, good fit for "does the CLI exit non-zero correctly across many rule/pair combinations." |
| assert_cmd | current | MIT OR Apache-2.0 | Finer-grained, individually-asserted CLI test cases (specific flag combinations, specific error messages) — use alongside `trycmd`, not instead of it; `trycmd` for the herd, `assert_cmd` for the pets, per the project's own test-case shape. |
| criterion | 0.8.x (`criterion2` fork also available, 3.0.4, if upstream `criterion` maintenance concerns become a problem) | Apache-2.0 OR MIT | Benchmark harness for the block-matching/phase-correlation/CPU-rasterization hot paths — the statistically rigorous, historically-trending option, appropriate given this project will care about regressions in comparison speed over time. `divan` is a lighter-weight, newer alternative worth a second look if criterion's dependency weight becomes an issue, but criterion's maturity and trend-reporting fit a project that will accumulate a performance history. |

**Confidence:** HIGH across this entire layer — these are among the most stable, widely-verified facts in the Rust ecosystem, and version numbers were checked directly against docs.rs listings this session.

### Rejected alternatives

| Rejected | Why not |
|----------|---------|
| `structopt` | Merged into `clap` (as the `derive` feature) years ago; using it today means depending on an archived, pre-merger crate. Use `clap`'s own derive feature. |
| `config` crate for TOML | General-purpose layered-config crate (env vars + files + defaults merged) — more machinery than a single declarative rule file needs, and works against the "reviewable in a pull request" simplicity PROJECT.md wants for rule files. Plain `toml`/`toml_edit` + `serde` is the right amount of tool. |
| `failure` crate | Deprecated years ago in favor of `thiserror`/`anyhow`; do not use. |
| `snapbox` in place of `insta` | `snapbox` is `trycmd`'s and `assert_cmd`'s underlying snapshot engine and is a reasonable alternative to `insta` for text-snapshot needs, but `insta`'s review workflow (`cargo insta review`) is the more mature, more widely adopted tool for the specific "review and accept a changed golden" loop this project's baseline-hash workflow needs. |

---

## Summary Table: What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `mupdf-rs` for PDF rendering | AGPL-3.0 — copyleft-poisons any binary that links it | `pdfium-render` (MIT/Apache-2.0 wrapper around BSD-3-Clause-family pdfium) |
| `poppler-rs`/Poppler bindings for PDF | GPL-2.0-or-later | `pdfium-render` |
| `dssim`/`dssim-core` for SSIM | Dual AGPL/commercial license | `image-compare` (MIT) |
| GPU-backed rendering (`vello`, `three-d`, `bevy`) for any pixel entering a comparison | Violates the no-GPU-rasterization hard constraint; vendor/driver rounding differs | CPU-only: `resvg`/`tiny-skia` for vector, `pdfium-render` for PDF, hand-rolled scanline rasterizer for meshes |
| Hardware video/image decode (VideoToolbox, DXVA, VAAPI, NVDEC) for comparison-path frames | Same non-determinism as GPU rasterization, different silicon per vendor | Software decode: `rav1d` for AV1 (proven bit-exact), `openh264` for H.264 |
| FFmpeg built with `--enable-gpl` (or any GPL-configured native FFmpeg) | Makes the resulting binary GPL as a whole | LGPL-configured FFmpeg only, and prefer `rav1d`/`openh264`/`symphonia` where they cover the needed codec instead |
| `structopt`, `failure` | Both deprecated/merged years ago | `clap` derive, `thiserror`/`anyhow` |
| Rolling a full 3D engine (`bevy`, `three-d`) just to get a mesh rasterizer | Enormous transitive dependency tree, entirely GPU-oriented, wrong tool for a fixed eight-view CPU raster job | Hand-rolled scanline/half-space z-buffer rasterizer over `gltf`/`tobj`/`ply-rs-bw`/`stl_io`-loaded meshes |

---

## Version Compatibility Notes

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| resvg 0.48.1 | tiny-skia 0.12.0 | resvg vendors its exact tiny-skia version internally; do not pin tiny-skia independently, let resvg's Cargo.toml drive it. |
| wgpu 26.0.0 | naga 26.0.0 | wgpu and naga are versioned in lockstep by the wgpu project; never mix versions. |
| image 0.25.10 | tiff 0.8+ | image 0.25's TIFF path requires tiff >=0.8; older tiff versions are incompatible with image 0.25's decoder trait surface. |
| pdfium-render 0.9.3 | pdfium binary (pinned release tag via `bblanchon/pdfium-binaries`) | The crate and the native binary version drift independently — pin the exact pdfium binary build/tag used for golden-image tests, not just the crate version, since the determinism claim depends on the native binary build, not the Rust wrapper. |
| ffmpeg-next / ffmpeg-sys-next | System or vendored FFmpeg build | Must verify the linked FFmpeg's `configure` flags at build time (no `--enable-gpl`) — this is a build-environment fact, not something `Cargo.lock` captures, and needs an explicit CI check. |

## Sources

- crates.io and docs.rs pages for: resvg, tiny-skia, pdfium-render, pdfium (via Google's pdfium LICENSE), mupdf/mupdf-sys, image, image-webp, zune-jpeg, tiff, ravif, rav1e, symphonia, rav1d, openh264/openh264-sys2, ffmpeg-next/ffmpeg-sys-next, gstreamer-rs, gltf, tobj, ply-rs/ply-rs-bw, stl_io, wgpu, naga, dssim/dssim-core, image-compare, empfindung, delta_e, ssimulacra2, clap, toml/toml_edit, thiserror, anyhow, cargo-insta, trycmd, assert_cmd, criterion/criterion2 — HIGH confidence, checked this session.
- lib.rs crate summary pages, used as a secondary cross-check for license strings and dependency graphs on resvg, pdfium-render, tiny-skia — HIGH confidence.
- Google's pdfium LICENSE and third-party bundle (FreeType, libjpeg) — HIGH confidence, checked against the project's own repository documentation via search.
- rav1d/rav2d project documentation for the bit-exactness claim against dav1d/dav2d — HIGH confidence, this is the project's own stated and tested guarantee.
- General Rust/IEEE-754 determinism reasoning (Layer 5 and Layer 6 GPU-reduction-order caveats) — MEDIUM confidence, sound domain knowledge applied to this project's specifics, not tied to a single cited source.
- No mature Rust crate found for CPU 3D mesh rasterization or a direct Butteraugli port — HIGH confidence in the negative finding (searched directly, multiple query angles, found only unmaintained hobby projects and no crates.io presence).

---
*Stack research for: deterministic cross-platform structural media diff (Chrysoberyl)*
*Researched: 2026-09-06*
