# Pitfalls Research

**Domain:** Deterministic cross-platform media diff tool (Rust, CPU rasterization, GPU compute compare)
**Researched:** 2026-09-06
**Confidence:** MEDIUM (web sources, no curated Context7/docs access this session; several claims cross-checked across 2+ independent sources)

## Critical Pitfalls

### Pitfall 1: Font and vector rasterization is not deterministic across platforms by default

**What goes wrong:**
The same font file and the same SVG produce different pixels on Windows, macOS and Linux, even with the exact same rasterizer library, because glyph hinting and antialiasing are supplied by the platform text stack unless the rasterizer explicitly avoids it.

**Why it happens:**
Windows uses DirectWrite or GDI, macOS uses Core Text, Linux uses FreeType with a distro-configured hinting profile. Each interprets font hints and applies antialiasing differently, changing glyph edge pixels by a few levels even on identical input. This is confirmed independently: font-rendering guides document the three-stack split, and the Vercel Satori project shipped a regression (issue #708) where a specific font plus a specific text string rendered differently on macOS vs. Linux/Windows CI, traced to glyph-caching changes interacting with per-platform text shaping. The bug did not trigger for most fonts or most text, which is exactly the kind of failure that a small test corpus will not catch.

**How to avoid:**
Rasterize with a library that bundles its own font engine and does not call into the OS text stack (resvg is built this way specifically to avoid this class of bug). Pin the font-rendering crate version and its bundled dependencies (e.g. the shaping/hinting engine version) in the lockfile, because a minor version bump in the bundled engine is itself a determinism break. Build a cross-platform golden-image test early — one pair per phase that touches text — and run it in CI on all three target OSes before trusting any other verdict.

**Warning signs:**
A hash-manifest baseline captured on one OS fails verification on another OS for a pair that a human judges "identical." The failure is intermittent by font or by string, not by file, which points at glyph-level hinting, not a gross bug.

**Phase to address:**
The phase that builds CPU rasterization (vector/document input). Add a same-input, cross-OS parity test as an exit criterion for that phase, not as an afterthought.

---

### Pitfall 2: Floating-point contraction (FMA) and libm differences quietly break "deterministic"

**What goes wrong:**
Two builds of the same Rust code, or the same code on two CPU architectures, can produce different floating point results for the same input, because the compiler may fuse a multiply and an add into a single fused-multiply-add instruction depending on target and optimization flags, and because `sin`, `cos`, `pow`, and other transcendental functions in Rust's standard library call into the platform's C library (libm), whose implementations are not bit-identical across platforms.

**Why it happens:**
IEEE-754 guarantees bit-identical results for `+ - * /` given the same rounding mode, but does not mandate a single implementation for transcendental functions, and does not forbid a compiler from contracting `a*b+c` into one FMA rounding step instead of two. This is documented across NVIDIA developer forums (GPU vs CPU divergence), the Rust project's own issue tracker (the phrase "non-deterministic floating point" is explicitly flagged as misleading — the true cause is contraction and libm, not true nondeterminism), and gamedev lockstep-simulation literature. A concrete example found in research: the same `sin()` call producing results differing by over 10% between platforms — no exotic case is needed.

**How to avoid:**
Anywhere the comparison engine does its own floating-point math (block matching, phase correlation, colour-space conversion, perceptual metrics), audit for transcendental function calls and either replace them with a pure-Rust libm implementation (removes the platform C library from the dependency chain) or restrict comparisons to operations that only need `+ - * /` and comparisons, not trig or exponentials. Disable FMA contraction explicitly for the comparison-critical code path (a targeted `-C target-feature` or per-function attribute), rather than relying on default codegen being stable across Rust compiler versions. Treat any new floating-point function added to the comparison core as a determinism review item, not a normal code review item.

**Warning signs:**
A CI matrix job on a different CPU architecture (e.g. Apple Silicon vs. x86_64, or an ARM CI runner) produces a byte-different structural verdict for a baseline recorded on the other architecture, despite the CPU-only-rasterization rule being followed.

**Phase to address:**
The comparison-engine phase (block matching / phase correlation), before the cross-platform-parity claim is made publicly. This should be the first thing tested with a same-machine-vs-different-architecture CI job, since it is cheaper to catch before the rule file and baseline-manifest phases build on top of it.

---

### Pitfall 3: SIMD lane width and vectorized codegen change results between builds

**What goes wrong:**
A rasterizer or comparison routine that uses SIMD (either explicitly or via auto-vectorization) can produce different pixel or metric output depending on which SIMD width the compiler chose for that target (SSE vs AVX vs AVX2 vs NEON), because horizontal reduction order changes the rounding of intermediate sums.

**Why it happens:**
Vectorized code processes multiple lanes per instruction, and combining lanes (a horizontal sum, a min/max reduction) does not associate identically to the scalar equivalent unless the algorithm is written to force a fixed reduction order. This compounds with Pitfall 2: a vector CPU path with FMA enabled changes rounding relative to a scalar path without it. Confirmed generically in cross-platform SIMD engineering writeups; the concrete rasterizer-specific case is the Satori issue above, where glyph-cache-driven codegen differences (not literal SIMD width, but the same class of "same input, different low-level execution path, different pixels") caused the platform-dependent bug.

**How to avoid:**
For any hot path that must feed the deterministic verdict, either force a fixed, sequential (non-lane-parallel) reduction order in the reference/baseline-producing code path, or accept SIMD only where the operation is proven associative in the encoding actually used (e.g. integer min/max over exact pixel values, not floating accumulation). Do not let `-C target-cpu=native` or `-C target-feature=+avx2` leak into the build that produces a committed baseline; pin the target features used for baseline-producing builds explicitly in the build script, and document that value next to the baseline manifest format.

**Warning signs:**
A verdict changes when the crate is rebuilt with a different `RUSTFLAGS` or on a machine with a different microarchitecture, with no source change.

**Phase to address:**
The comparison-engine phase, same exit criterion as Pitfall 2: a fixed-features build for anything that writes or checks a baseline.

---

### Pitfall 4: Visual regression suites die from threshold inflation, not from bad algorithms

**What goes wrong:**
Teams adopt a pixel or perceptual diff tool, get burned by flaky failures unrelated to real changes (animation frames, font antialiasing noise, async content not fully loaded at capture time), and respond by raising the tolerance threshold until the noisy failures stop. This also silences real regressions of similar magnitude, and eventually someone disables the check because it "never catches anything real" or "everything is red."

**Why it happens:**
This is the single most consistently reported failure mode across current (2025-2026) visual-testing literature and is presented explicitly as the wrong fix, because threshold inflation treats the symptom (noisy diffs) instead of the cause (nondeterministic capture). It is exactly the problem Chrysoberyl's own competitive framing calls out: "a wall of red is not a review."

**How to avoid:**
This is the pitfall the whole project exists to solve, so treat it as a first-class acceptance criterion, not a side effect. The declarative TOML rule file with tolerance scoped by kind-of-change and by named region is the direct answer — it lets a team say "ignore antialiasing-class differences in this region" without opening the tolerance globally. Never expose a single global numeric threshold as the primary UX; make named, structural exclusions the primary mechanism and keep any numeric tolerance as the narrow fallback. Ship an example rule set that demonstrates scoping tolerance by structural class (added/removed vs. shifted vs. recoloured) from day one, so users do not reach for "just raise the number" out of habit.

**Warning signs:**
Any user-facing metric or example in the docs that recommends "increase the threshold if you get false positives" without also showing the scoped-exclusion alternative.

**Phase to address:**
The rule-file phase (TOML rule file, tolerance scoped by kind of change and region). This is a roadmap-shaping pitfall: it argues for building the rule file early enough that early adopters never learn the "raise the global threshold" habit from Chrysoberyl's own docs.

---

### Pitfall 5: wgpu compute pipelines stall or hang on the CPU/GPU readback boundary

**What goes wrong:**
A first wgpu compute pipeline reads results back to the CPU by mapping a buffer, and the map future never resolves, or resolves but the program has already serialized every dispatch behind a CPU wait, killing throughput; in the worst case the GPU driver reports a device-lost or a validation panic that differs between backends.

**Why it happens:**
`buffer.map_async()` requires the device to be polled (`device.poll(Maintain::Poll)` or the wgpu event loop) continuously while the future is pending, or it hangs forever — this is a recurring, independently filed issue across the wgpu tracker. A buffer still referenced by an in-flight command submission cannot be mapped and will panic or reject; the documented pattern is to copy the compute-output buffer into a separate `MAP_READ | COPY_DST` staging buffer and map only the staging buffer, after the copy command has completed. Mapping immediately after submit (rather than doing other work while the GPU finishes) serializes CPU and GPU and is a documented anti-pattern, not a bug — it "works" but is slow enough to look broken on large images. Buffer offsets and sizes used for mapping must also respect `wgpu::MAP_ALIGNMENT` and cannot have a non-multiple-of-4 length, which is an easy-to-miss constraint when slicing a buffer for partial readback.

**How to avoid:**
Always route GPU compute output through a dedicated staging buffer, never map the buffer the compute shader wrote directly. Drive the poll loop explicitly and treat "the future never resolves" as a wiring bug, not a timeout to paper over with a longer wait. Write one deliberately overlapped readback path (submit next dispatch while previous readback is in flight) before assuming the naive synchronous path will be fast enough for a 12000x8000 image tiled into many dispatches.

**Warning signs:**
GPU compute mode is dramatically slower than expected relative to CPU-only mode, or the program hangs specifically on larger images/more dispatches, not on the toy test case.

**Phase to address:**
The GPU-compute comparison phase. This should carry its own explicit spike/prototype before the phase plan, given the project context notes the author has not written GPU code before — budget for this pitfall specifically, do not assume the naive tutorial pattern scales.

---

### Pitfall 6: Backend and vendor differences in wgpu are real, not theoretical

**What goes wrong:**
The same wgpu compute shader can behave correctly on Vulkan and incorrectly on D3D12, or hang on one GPU vendor/driver combination and not another, because Naga's WGSL-to-SPIR-V/MSL/HLSL translation is a real code path with its own bugs, and because GPU drivers themselves have vendor-specific bugs.

**Why it happens:**
wgpu's own "Known Driver Issues" wiki (a first-party, authoritative source) documents concrete cases: an Intel Vulkan driver hang when the same semaphore is signaled and waited on twice (Mesa 21.2.3), an Intel D3D12 driver timeout/device-reset from `CreatePlacedResource` on HD Graphics 4600, an Nvidia Vulkan driver requiring 16-byte-aligned `write_buffer` calls or misbehaving, an Nvidia Vulkan device hang on specific render sequences on certain driver versions, an AMD D3D12 bug where partial texture updates read back as zero, and a Mesa Vulkan integer-overflow bug with very large buffers. Separately, the wgpu issue tracker has open reports of code that validates cleanly and runs correctly on Vulkan but panics on DX12 for the same input.

**How to avoid:**
Since Chrysoberyl's core promise is a portable baseline and the GPU path is explicitly the comparison stage (not the rasterization stage), design the GPU compute path so a wrong or crashing GPU result never becomes the reported verdict silently: validate GPU output against a CPU-computed checksum or a cheap invariant on a sample before trusting it, and provide a documented CPU-fallback compare mode. Test on at least one GPU per major vendor (Intel integrated, Nvidia, AMD) and on Apple Silicon (Metal) before calling the GPU compute path production-ready, not just "it runs on my machine's GPU." Treat any DX12-only or Vulkan-only bug report from a user as expected, not surprising, and keep the wgpu backend-selection override documented and easy for a user to force (e.g. force Vulkan on a Windows machine with a known-bad D3D12 driver).

**Warning signs:**
A GPU-mode comparison gives a different verdict than the CPU-mode fallback for the same pair on the same machine — the correct response is to trust neither until the discrepancy is root-caused, not to assume the GPU path is simply "more precise."

**Phase to address:**
The GPU-compute comparison phase, and the CLI/CI phase (exit non-zero on rule failure) should include a "GPU compute disagreed with CPU checksum" as its own distinct exit condition, separate from "structural change found."

---

### Pitfall 7: Large images blow up memory before decode even starts

**What goes wrong:**
A malicious or merely very large input (a claimed 12000x8000 image, or a crafted file with a small file size but a huge declared dimension or compressed tile) can cause unbounded memory allocation during decode, well before any comparison logic runs.

**Why it happens:**
This is a documented, repeated class of bug in the Rust image ecosystem specifically: a security audit found the `png` crate's decompression path (via the `inflate` crate) used an unsafe `set_len` call that could expose uninitialized memory contents in the decoded output; CVE-2023-29408 affected the `image` crate's TIFF decoder, which placed no limit on declared compressed-tile size, letting a crafted file force excessive memory and CPU consumption. In response, `image-rs` added a `Limits` API (`max_image_width`, `max_image_height`, `max_alloc`) — but this is opt-in per decoder call, not a default, so a caller that just calls the convenience `image::open()` path gets no protection.

**How to avoid:**
Explicitly configure `Limits` (or the equivalent for whichever decoder library is chosen) on every decode call in the ingestion path, sized to a documented maximum the tool supports, and reject anything larger with a clear error rather than attempting to decode it. Treat every input pair as untrusted, even in local/offline use — a corrupted or adversarially crafted file used as one half of a "baseline vs current" pair should fail cleanly, not hang the process or exhaust memory. Test explicitly with a decompression-bomb-style fixture (small file, huge declared dimensions) as part of the ingestion phase's test suite.

**Warning signs:**
Memory usage during decode is not bounded by expected file size; the process is killed by the OS OOM killer on a file that "looks" small on disk.

**Phase to address:**
Whichever phase first implements raster-frame ingestion (the earliest phase, "compare a pair of raster frames"). This is foundational and should not be deferred, since every later format (animation, video, PDF, mesh-view raster) sits on top of this ingestion path.

---

### Pitfall 8: EXIF orientation and premultiplied alpha silently change what "identical" means

**What goes wrong:**
Two files that are byte-for-byte the same image can be displayed rotated 90 degrees differently depending on whether the EXIF orientation tag is honored, and two files with the same visible colour can decode to different raw pixel values depending on whether alpha is stored premultiplied or straight, especially at partially-transparent edges.

**Why it happens:**
EXIF orientation handling is inconsistently implemented across image libraries and tools — some auto-rotate on load, some do not, some only honor it for JPEG and not PNG (a case explicitly reported for browsers not honoring EXIF-equivalent orientation metadata in PNG). Premultiplied-alpha bugs recur across tools (Unity's Recorder had a documented case for PNG and EXR) because un-premultiplying can produce out-of-range values when RGB exceeds alpha, causing colour clipping or halos at edges — and getting this wrong changes actual comparison-relevant pixel data, not just display.

**How to avoid:**
Pick one canonical internal representation early (a specific orientation-normalized, straight-alpha or premultiplied-alpha pixel buffer) and normalize every decoded input to it before any comparison logic runs, rather than comparing raw decoded buffers whose orientation or alpha convention may differ per-format or per-decoder default. Make the normalization step itself part of the deterministic pipeline under test — a golden test pair should include at least one image with a non-default EXIF orientation tag and one with meaningful alpha to catch a regression here.

**Warning signs:**
A pair that is visually identical to a human reports as "changed" (rotated) or reports a colour-region difference that only appears near transparent edges.

**Phase to address:**
The raster-frame ingestion phase, alongside Pitfall 7 — both are "normalize on decode" concerns and belong in the same pipeline stage.

---

### Pitfall 9: Video "bit-exact" decode is a claim to prove, not a property to assume

**What goes wrong:**
The open question already recorded in this project's own PROJECT.md — that video decode is specified bit-exact and so hardware and software decode should agree — is not automatically true in practice. Differences in GPU architecture, driver version, and floating-point/FMA ordering inside a hardware decode path can produce small (typically +/-1 to 3 level) pixel differences relative to a software decode of the same bitstream, even though the codec specification itself defines the decode process exactly.

**Why it happens:**
The codec spec (e.g. AV1, H.264) defines the bitstream-to-pixel mapping exactly, but hardware decoders are separate implementations of that spec, and any implementation detail not pinned by the spec (rounding order in a hardware IDCT, for instance) can diverge. This matches the general FMA/rounding-noise pattern from Pitfall 2, applied to a decode ASIC instead of a CPU. Reference software decoders explicitly target and test for bit-exactness against a conformance corpus (the dav1d/AV2 reference-decoder porting effort found byte-for-byte matches against every conformance clip when using pure software decode, with the single documented exception being a deliberately malformed conformance clip whose own reference decoder does not decode it deterministically across thread counts) — the guarantee that does exist is software-decoder-to-software-decoder, and only when threading does not introduce its own nondeterminism.

**How to avoid:**
Given the CPU-only-rasterization rule already adopted for images, apply the same rule to video: use only a software decode path for anything that feeds the deterministic verdict, and disable hardware-accelerated decode entirely for the comparison pipeline, even though it is slower. If threaded software decode is used for speed, verify explicitly that the chosen decoder is deterministic across thread-count settings (not all are, per the above) before trusting it as a baseline source. Do not let "the spec says bit-exact" substitute for a corpus test — build the conformance-style test the PROJECT.md itself calls for before shipping the video feature.

**Warning signs:**
A video comparison gives a different verdict for the same file pair between two machines that both use software decode but different thread-count settings, or between a machine with hardware decode available/enabled and one without.

**Phase to address:**
The video comparison phase (behind the off-by-default Cargo feature). The project's own open question already flags this — treat "prove bit-exactness against a corpus" as a phase exit criterion, not a stretch goal.

---

### Pitfall 10: Variable frame rate and colour-matrix mismatches make "the same frame" ambiguous

**What goes wrong:**
Two videos that are logically "the same content" can have frame N in one file map to a different presentation timestamp — or no frame at all — in the other file, because variable-frame-rate (VFR) containers give each frame its own timestamp rather than a fixed interval, so "frame number" does not correspond to a consistent point in time across two files. Separately, YUV-to-RGB conversion can silently apply the wrong colour matrix (e.g. BT.601 instead of BT.709/BT.2020) if the actual colourspace tag on the stream is not read, producing a real colour shift between two tools that decode the identical bitstream.

**Why it happens:**
VFR is a real, supported container feature, not an edge case — frame durations vary, so aligning frame N of one file against frame N of another (or against a fixed time grid) produces drift that grows across the video's length. `ffmpeg`'s own `swscale` component has a documented history of ignoring the stream's actual colourspace and unconditionally applying a BT.601 conversion matrix, which is simply wrong for BT.709/BT.2020 content — this is a real, filed bug pattern, not a hypothetical.

**How to avoid:**
Compare video by presentation timestamp, not by frame index, and make the alignment strategy for mismatched timestamps (nearest-neighbor, interpolation, or explicit "no matching frame" as a reported condition) an explicit, documented, and tested part of the video comparison design — do not assume constant frame rate. Read and honor the actual colourspace/transfer/matrix tags from the container and stream metadata when converting to RGB for comparison, rather than assuming a default matrix; treat a colourspace-tag mismatch between the baseline and current file as a reportable condition in its own right, since it can look exactly like a "colour changed" structural finding when the actual pixels never changed.

**Warning signs:**
A video comparison reports drifting small differences that grow over the length of the clip (VFR timestamp drift), or a uniform colour-cast difference across the entire frame that a human does not perceive as a color change (colour-matrix mismatch).

**Phase to address:**
The video comparison phase. Frame-alignment strategy and colourspace handling should both be named design decisions in that phase's plan, not implementation details discovered mid-phase.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|--------------------|-----------------|-----------------|
| Calling `image::open()` / default decode without setting `Limits` | Less boilerplate at ingestion | Unbounded memory on crafted or oversized input (documented CVE class) | Never in the shipped ingestion path; acceptable only in a throwaway prototype never exposed to untrusted input |
| Letting the default Rust `sin`/`cos`/`pow` (libm-backed) run in the comparison core | Simpler code, no extra dependency | Cross-platform verdict divergence that violates the project's core value claim | Never in the comparison core; acceptable in UI-only or display-only code that never affects the verdict |
| Using hardware video decode for speed | Much faster decode | Breaks the "same verdict on any machine" claim for the video feature | Never for the feature that produces the reported verdict; acceptable for a preview/scrub window in the native GUI, clearly separated from the comparison path |
| Mapping a wgpu buffer directly instead of staging it | Fewer lines of code in a first prototype | Serializes CPU/GPU, and can panic if the buffer is still in flight | Acceptable only in a disposable spike to learn the API, never in the shipped compute pipeline |
| Global numeric diff threshold instead of scoped rule-file exclusions | Ships the MVP diff faster | Directly reproduces the "everything is red, then everyone disables it" failure this project is meant to prevent | Never — this is the anti-pattern the whole product exists to avoid |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|-----------------|-------------------|
| wgpu (Metal/D3D12/Vulkan) | Assuming portable `wgpu::Limits` defaults match what the hardware actually supports | Query `adapter.limits()` at runtime and fail loudly (not silently clamp) if a needed limit is unavailable on a target backend |
| ffmpeg-based video decode (optional feature) | Relying on the "build" feature to compile FFmpeg from source, which is documented to fail on Windows configure scripts | Prefer linking a system-installed or vcpkg-provided FFmpeg via `link_system_ffmpeg`/`link_vcpkg_ffmpeg`, and document the exact supported FFmpeg build config per OS |
| ffmpeg licensing | Linking a prebuilt or distro-packaged FFmpeg without checking whether it was built with `--enable-gpl` | Pin and document the exact FFmpeg build flags used for any shipped/optional binary; treat GPL components as opt-in and keep them out of the default, MIT-licensed build path, consistent with the project's existing "core is MIT, video sits behind an off-by-default feature" decision |
| resvg / vector rasterization | Assuming the bundled font engine version is interchangeable across releases | Pin the exact resvg (and its bundled font-shaping dependency) version in the lockfile; treat a version bump as a determinism-relevant change requiring the cross-OS golden test to re-run |
| GitHub Actions macOS runner (code signing) | Trying to codesign/notarize a macOS binary from a Linux cross-compilation runner | Build and sign on an actual macOS runner in the CI matrix; budget the documented 2-5 minutes of Apple-side notarization scan time per release build |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|-----------------|
| Synchronous GPU readback per dispatch | GPU compute mode is slower than CPU-only mode | Overlap dispatch and readback via a staging-buffer pipeline; poll the device explicitly rather than blocking | Any image large enough to need more than one dispatch, e.g. a 12000x8000 pair |
| Decoding a full, un-clamped image before any size check | Process OOMs or hangs on a single large file | Read declared dimensions first and apply `Limits`/max_alloc before decoding pixel data | A 12000x8000 pair (768M subpixels at 8bpc RGBA is roughly 3GB unclamped for two images plus intermediates) |
| Frame-index alignment on VFR video | Reported differences grow steadily across a clip's length instead of staying localized | Align by presentation timestamp, with an explicit policy for unmatched frames | Any VFR-encoded input, which is common from screen recorders and some mobile cameras |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Treating local, offline input as trusted because there is no network/account | A crafted local file (decompression bomb, malformed TIFF tile) still crashes or exhausts memory on the user's own machine | Apply the same untrusted-input discipline (explicit `Limits`, dimension checks before decode) regardless of the tool being local-only |
| Assuming "no hosted service" removes all attack surface | CI usage (exit non-zero, report artifact) means Chrysoberyl processes files from a git history or PR branch that may not be fully trusted (e.g. in a monorepo with external contributors) | Treat CI-triggered comparisons as processing semi-trusted input; keep decode limits enforced there too |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-------------------|
| Reporting a single "pixels differ" verdict with no structural classification | User cannot tell antialiasing noise from a real change, leading straight to Pitfall 4's threshold-inflation spiral | Report structural classification (added/removed/shifted/recoloured) and let the rule file scope tolerance by that classification and by named region |
| Silent GPU/CPU disagreement | User trusts a GPU-produced verdict that happens to be wrong on their specific vendor/driver | Surface a distinct "GPU and CPU disagree" condition rather than only ever showing the GPU result |
| No visibility into which decode path (hardware/software) produced a video verdict | User cannot reproduce a video-comparison failure on a different machine | Report the decode path used (software-only, per project design) directly in the artifact, so a mismatch is diagnosable |

## "Looks Done But Isn't" Checklist

- [ ] **CPU rasterization determinism claim**: Often missing a cross-OS golden-image test in CI — verify the same baseline hash is produced on Linux, macOS and Windows runners for at least one text-bearing and one vector-only fixture, not just "it renders."
- [ ] **Decode-side input limits**: Often missing explicit `Limits`/`max_alloc` configuration on every decode call — verify with a deliberately oversized or malformed fixture, not just normal-sized test images.
- [ ] **GPU compute correctness**: Often missing a CPU-computed cross-check for the GPU result — verify the pipeline reports disagreement rather than silently trusting the GPU path, and that this has been tested on more than one GPU vendor.
- [ ] **Rule-file tolerance model**: Often missing named-region/kind-of-change scoping in the first shipped example, defaulting instead to a single numeric threshold — verify the example rule file demonstrates scoped exclusions, not just a global tolerance.
- [ ] **Video bit-exactness claim**: Often missing an actual conformance-style test against a corpus — verify the open question in PROJECT.md ("prove it against a corpus") has a corresponding test before the video feature ships, not just a decoder crate dependency.
- [ ] **macOS release signing**: Often missing a real macOS CI runner in the release pipeline — verify the release workflow actually invokes `codesign`/`notarytool` on macOS hardware, not a cross-compiled artifact assumed to be signable elsewhere.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|-----------------|-----------------|
| Cross-platform rasterization divergence discovered post-release | MEDIUM | Pin the exact rasterizer/font-engine version that produced existing baselines; add the golden cross-OS test retroactively; document the pinned version as part of the baseline manifest format so old baselines remain valid |
| FMA/libm divergence discovered in the comparison core | MEDIUM-HIGH | Identify every transcendental function call in the affected path, replace with pure-Rust libm or an integer/rational reformulation, and invalidate/regenerate any baseline produced before the fix, since the verdict itself may have silently varied by machine |
| GPU vendor-specific bug discovered in production | LOW-MEDIUM | Ship a documented backend-override flag and a CPU-fallback compare mode immediately; file the specific bug against wgpu's Known Driver Issues process rather than working around it silently in application code |
| Accidental GPL linkage discovered in a shipped binary | HIGH | Audit the exact FFmpeg build configuration used for every published binary; rebuild and re-release without `--enable-gpl` components, and notify anyone who received the affected binary, since this is a legal exposure, not just a technical one |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|--------------------|--------------|
| Font/vector rasterization divergence | CPU rasterization phase | Cross-OS golden-image test (Linux/macOS/Windows) passes for a text-bearing and a vector-only fixture |
| FMA/libm divergence | Comparison-engine phase | Same-input CI job across two CPU architectures produces byte-identical verdict |
| SIMD lane-width divergence | Comparison-engine phase | Baseline-producing build uses a pinned, documented target-feature set; verdict is stable when rebuilt without that pin |
| Threshold inflation / "everything is red" | Rule-file phase | Shipped example rule file demonstrates scoped, named-region tolerance, not a single global number |
| wgpu readback stalls | GPU-compute comparison phase | A large-image (12000x8000-class) benchmark completes without the naive synchronous-map anti-pattern, verified by profiling, not by inspection |
| wgpu backend/vendor divergence | GPU-compute comparison phase | GPU result is cross-checked against a CPU-computed value on at least Intel, Nvidia/AMD, and Apple Silicon before the phase is called done |
| Large-image memory blowup / decompression bombs | Raster-frame ingestion phase (earliest phase) | A deliberately oversized/malformed fixture is rejected cleanly with a bounded-memory error, not a crash or hang |
| EXIF orientation / premultiplied alpha | Raster-frame ingestion phase | Golden test pair includes a non-default-orientation image and a meaningful-alpha image |
| Video bit-exactness assumption | Video comparison phase (feature-gated) | Corpus-based conformance test exists and passes, per the project's own recorded open question |
| VFR timestamp alignment / colour-matrix mismatch | Video comparison phase (feature-gated) | Frame-alignment policy and colourspace-tag handling are named, tested design decisions with a VFR fixture and a non-BT.601 fixture |
| macOS code signing / native runner | Release/CI phase | Release workflow runs `codesign`/`notarytool` on an actual macOS runner and the resulting binary launches unblocked on a clean macOS install |
| Accidental GPL linkage via FFmpeg build flags | Video comparison phase (feature-gated) / release phase | Documented, pinned FFmpeg build configuration for every shipped binary; a license check step in the release process confirms no `--enable-gpl` component is present unless explicitly intended |

## Sources

- Vercel Satori issue #708 — platform-dependent non-deterministic SVG text rendering (github.com/vercel/satori/issues/708) — cross-checked via WebFetch, MEDIUM confidence
- gfx-rs/wgpu "Known Driver Issues" wiki (github.com/gfx-rs/wgpu/wiki/Known-Driver-Issues) — first-party project source, HIGH confidence
- FFmpeg official legal page (ffmpeg.org/legal.html) — first-party authoritative source on LGPL/GPL split, HIGH confidence
- Shnatsel (Sergey Davidoff), "Auditing popular Rust crates" and "How I've found vulnerability in a popular Rust crate" (Medium) — png/inflate uninitialized-memory disclosure, MEDIUM confidence
- image-rs/image GitHub issue #938, "Library wide memory limits"; CVE-2023-29408 (TIFF decoder) — MEDIUM-HIGH confidence, corroborated by CVE record
- Rust project issue tracker: rust-lang/rust #150323 ("Usage of non-deterministic floating point operations is misleading") and rust-lang/rfcs #2686 (FMA/extra-precision RFC discussion) — MEDIUM confidence
- Rust users forum threads on libm/platform floating-point divergence and on FFmpeg static-linking/cross-compilation failures — MEDIUM confidence, community-reported, cross-checked across multiple threads
- NVIDIA Developer Forums, "CPU and GPU floating point calculations Results are different" and related threads — MEDIUM confidence
- Chromium issue tracker and community reports on font-rendering flakiness (issues.chromium.org/issues/41058522 and related) — MEDIUM confidence
- General visual-regression-testing industry articles (2025-2026), on flaky baselines, threshold inflation, and Percy/Chromatic tradeoffs — MEDIUM confidence, multiple independent articles agree on the core failure mode
- gfx-rs/wgpu GitHub issues on buffer mapping, `map_async`/`Maintain::Poll`, and DX12-vs-Vulkan panics (#9, #2266, #3971, #2060, #6832) — MEDIUM confidence
- rusty_ffmpeg / rust-ffmpeg (zmwangx) GitHub issues on Windows build/static-linking failures — MEDIUM confidence
- FFmpeg swscale colourspace-conversion bug reports (Blender T21889, BabitMF/bmf #117, Shotcut forum thread) — MEDIUM confidence, multiple independent tool reports agree
- memorysafety/rav1d and community rav2d bit-exactness porting notes — MEDIUM confidence, illustrates both the achievability and the limits (thread-count nondeterminism on malformed streams) of "bit-exact" claims
- No evidence found for a specific, named public post-mortem of a team abandoning a visual-diff tool over a documented incident (only general industry accounts of the pattern) — flagged as a gap, not filled with speculation

---
*Pitfalls research for: deterministic cross-platform media diff tool (Rust, CPU rasterization + GPU compute compare)*
*Researched: 2026-09-06*
