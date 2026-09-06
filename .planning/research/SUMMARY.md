# Project Research Summary

**Project:** Chrysoberyl
**Domain:** Deterministic, cross-platform structural diff tool for non-text media (raster, animation, video, PDF, SVG, 3D mesh), in Rust
**Researched:** 2026-09-06
**Confidence:** MEDIUM-HIGH

## Executive Summary

Chrysoberyl builds the same kind of tool as Chromatic, Percy, Applitools, odiff, and pixelmatch, but answers a question none of them answer: is the baseline itself portable, or only the diff algorithm? Every hosted competitor solves cross-machine drift by centralizing rendering on one controlled cloud fleet; every local library assumes the caller already has consistent captures. Chrysoberyl instead removes the GPU from the raster path entirely, so the same pair produces the same verdict on any machine. This is a checkable claim, and it is the project's whole competitive position — no other surveyed tool makes it or can make it, because none avoid GPU rasterization by design.

The recommended approach is corroborated three separate ways: STACK.md, ARCHITECTURE.md, and PITFALLS.md each independently land on `resvg` + `tiny-skia` as the one existing, working proof that CPU-only rasterization can be bit-identical across x86/ARM and Windows/macOS/Linux, because it bundles its own font and graphics stack rather than calling into the OS. Architecture research generalizes this into a strict five-stage pipeline (decode -> rasterize -> register -> classify -> score -> report) with one trait (`Source`) erasing format differences after rasterization, so a mesh, a video frame, and a PDF page all become the same `Frame` type. Features research confirms the field-wide gap this closes: no surveyed tool scopes tolerance by kind-of-change and named region in one declarative file, and no tool treats mesh diff as a cheap CI gate. Pitfalls research is blunt that the hard part is not the algorithm, it is proving determinism empirically rather than assuming it — font rasterization, FMA contraction, libm divergence, SIMD reduction order, GPU vendor bugs, and hardware video decode are five distinct, independently documented ways the "same verdict on any machine" claim can quietly fail, and each needs its own cross-platform test before the corresponding phase is called done.

The greatest risk is not any single pitfall but the accumulation: the core value proposition rests on determinism claims that are proven for SVG (resvg's own stated design goal) and for AV1 (rav1d's tested bit-exactness against dav1d), but are explicitly unproven for PDF (pdfium font/hinting variance), H.264 (OpenH264 not proven bit-exact), and hand-rolled mesh rasterization (no mature crate exists at all — must be built in-house). The project's own PROJECT.md open question about video decode is one instance of a pattern that recurs at every new format boundary. The roadmap should treat "prove determinism against a golden corpus, cross-OS, in CI" as a first-class, per-format exit criterion, not a documentation afterthought, and should sequence formats from the most-proven (SVG) toward the least-proven (mesh, video) so failures are attributable to the one new thing just added.

## Key Findings

### Recommended Stack

The stack is layered by media kind, and every layer states its dependency weight and license posture explicitly, per PROJECT.md's hard constraints (MIT core, no GPU pixels, feature-gated video/PDF). The clean, corroborated core: `resvg`/`tiny-skia` for vector, `image` (trimmed to pure-Rust codecs) for raster, `wgpu` for comparison/display compute only, `clap`/`toml`/`thiserror`/`anyhow` for CLI plumbing, and `cargo-insta` as the golden-review harness whose terminal-only workflow is the closest existing precedent for Chrysoberyl's own native-window review loop.

**Core technologies:**
- `resvg` 0.48.1 + `tiny-skia` 0.12.0 (MIT/Apache-2.0, BSD-3) — SVG/vector rasterization with a stated, designed-in bit-identical-across-platforms guarantee; no system font/graphics library dependency.
- `image` 0.25.10, trimmed via `default-features = false` (MIT/Apache-2.0) — raster decode/encode (PNG, JPEG, WebP incl. animated, TIFF, AVIF-encode) with pure-Rust decoders for every format except AVIF-decode, which should route through `rav1d` rather than `libdav1d`.
- `wgpu` 26.0.0 (MIT/Apache-2.0) — the one Rust API reaching Metal/D3D12/Vulkan from one codebase; compute-only, gated behind a `gpu-compute`/`ui` feature so the slim core needs no GPU driver at all.
- `pdfium-render` 0.9.3 wrapping the pdfium native binary (MIT/Apache-2.0 wrapper, predominantly BSD-3-family engine) — the cleanest-licensed full-fidelity PDF rasterizer available; feature-gated, off by default; determinism is "per pinned build and pinned fonts," not proven bit-exact, and must be validated with a golden-image test before being trusted.
- `rav1d` (BSD-2) for AV1 decode — the strongest determinism claim of any codec in the stack, a proven bit-exact port of dav1d; `openh264`/`symphonia`/`ffmpeg-next` fill in H.264 and other codecs behind a `video` feature, with FFmpeg's GPL-configuration risk flagged as the single biggest license trap in the whole stack.
- `image-compare` (MIT) for SSIM, not `dssim` (AGPL/commercial) — a direct license-driven substitution the roadmap must respect from day one.
- Hand-rolled scanline/half-space z-buffer rasterizer for the mesh layer, on top of `glam` — no mature pure-Rust crate exists for CPU triangle rasterization; this is a real gap in the ecosystem, not a research oversight, and the fixed eight-view rig's narrow scope makes writing ~200-400 lines the correct engineering choice.

**License hazards to actively avoid:** `mupdf-rs` (AGPL-3.0), Poppler bindings (GPL-2.0+), `dssim`/`dssim-core` (AGPL/commercial), and any FFmpeg build compiled with `--enable-gpl` — each would poison the binary the moment the feature is compiled in, even if off by default in source.

### Expected Features

**Must have (table stakes), matching Chrysoberyl's own Active requirements:**
- Perceptual, antialiasing-aware raster diff core (every surveyed tool has this; raw RGB delta is known to over-flag)
- CLI with non-zero exit code + a report artifact (universal CI-gate contract)
- Named-region/masked-region ignore, and a side-by-side/overlay diff view (universal review-trust pattern)
- Baseline approve/reject tied atomically to the next baseline (Percy/Chromatic/cargo-insta all do this; Chrysoberyl's committed-goldens model is the same mechanism without a hosted middle-man)

**Should have (differentiators), each independently confirmed as a field-wide gap:**
- CPU-only deterministic rasterization — the load-bearing differentiator; everything else is secondary to proving this claim
- Structural change classification (named kind of change, not a score) — no surveyed competitor does this
- Tolerance scoped by kind-of-change AND named region in one declarative TOML file — confirmed gap across Playwright, Resemble.js, BackstopJS, jest-image-snapshot
- One engine across six media kinds — every competitor is siloed by format
- No account, no hosted service — a direct, verified response to the field's most common complaint (metered-snapshot billing)

**Defer (v2+):**
- 3D mesh comparison — already Active but highest-complexity; sequence after video/PDF prove the format-staging pattern
- Additional baseline store backends beyond committed goldens
- Any AI-assisted "likely false positive" hint layer, and only ever as a suggestion beside the deterministic verdict, never replacing it (this is exactly the Applitools trust gap the project's design already rejects)

**Anti-features to actively resist:** cloud/GPU rendering grids, black-box "Visual AI" classification, metered billing, automatic baseline drift without human sign-off, scriptable tolerance/rules, general image registration, and a configurable mesh camera rig — all explicitly named as conflicting with the Core Value or the declarative-rule-file design.

### Architecture Approach

A strict pipeline (decode -> rasterize -> register -> classify -> score -> report) where format knowledge is erased at the `Source::load()` boundary and never crosses it again. `chrys-core` has no format dependency by construction — it cannot import a format crate — which is the concrete mechanism for AGENTS.md's "no per-thing special case" rule. Register/classify get a CPU reference (authoritative, decides every verdict) and a wgpu compute mirror (paints the interactive window only); the two are bound by categorical equivalence tests (same regions, same types), never bit-equivalence, because GPU floating point cannot be made bit-identical to CPU across vendors.

**Major components:**
1. `chrys-source-*` adapters (one per media kind) — decode + rasterize, the only place format-specific code may live
2. `chrys-core` — register (phase correlation + coarse-to-fine block match), classify (connected-component labelling), score (TOML rule lookup), report — the frozen, format-blind spine
3. `chrys-gpu` — wgpu compute mirror for interactive display only, validated against `chrys-core` by equivalence tests, never authoritative
4. `chrys-store` — pluggable baseline-store trait, committed goldens as the only v1 backend
5. `chrys-cli` / `chrys-window` — headless-first CLI, Slint+wgpu window built last, after the core and CLI are already stable

### Critical Pitfalls

1. **Font/vector rasterization is not deterministic across platforms by default** — mitigated by choosing a rasterizer (resvg) that bundles its own font engine and never calls the OS text stack; verify with a cross-OS golden-image test on a text-bearing fixture, in the CPU-rasterization phase.
2. **FMA contraction and libm divergence quietly break "deterministic"** — a `sin()` call can differ by over 10% between platforms; audit every transcendental call in the comparison core, disable FMA contraction explicitly, and test across CPU architectures before making the portability claim public.
3. **Threshold inflation is the industry's most common failure mode**, not a bad algorithm — this is literally the pitfall the whole project exists to solve; ship the scoped TOML rule file early enough that users never learn the "raise the global number" habit from Chrysoberyl's own docs or examples.
4. **wgpu readback stalls and vendor/driver bugs are real, not theoretical** — first-party wgpu "Known Driver Issues" documents concrete Intel/Nvidia/AMD hangs and mismatches; always stage GPU output through a dedicated buffer, cross-check GPU results against a CPU checksum, and never let a GPU disagreement silently become the reported verdict.
5. **Large/malformed inputs blow up memory before comparison even starts** — a documented CVE class in the `image` crate ecosystem (TIFF, PNG); configure explicit `Limits`/`max_alloc` on every decode call from the earliest ingestion phase, treating even local input as untrusted.

## Threats to the Core Value ("a baseline is portable")

The Core Value claim -- same pair, same verdict, any machine, any GPU, any driver -- is the one thing PROJECT.md says cannot fail. Collecting every finding across all four documents that threatens it:

| Finding | Source(s) | Mitigation proposed |
|---|---|---|
| Font hinting/AA differs by OS text stack even with the same rasterizer, unless the rasterizer avoids the OS entirely | STACK, ARCHITECTURE, PITFALLS (independently, all three) | Use resvg/tiny-skia, which is designed and documented to avoid system font/graphics libraries; pin the exact version; cross-OS golden test on a text-bearing fixture as a phase exit criterion |
| FMA contraction (compiler fuses multiply-add differently per target) changes rounding | STACK (Layer 5), ARCHITECTURE (Sec 4 Determinism), PITFALLS (#2, #3) | Disable FMA contraction explicitly for the comparison-critical path; do not rely on default codegen; pin target-feature flags for any baseline-producing build |
| Platform libm (`sin`/`cos`/`atan2`) is not bit-identical across glibc/musl/CRT/Apple libm | STACK, ARCHITECTURE, PITFALLS (all three, same finding) | Avoid transcendental calls in the rasterization/comparison hot path; precompute view/projection matrices for the fixed mesh rig instead of computing trig per-frame; use a pure-Rust libm if a transcendental is unavoidable |
| SIMD lane width / auto-vectorization changes reduction order and thus rounding | ARCHITECTURE, PITFALLS (#3) | Force a fixed, sequential reduction order in the baseline-producing code path, or restrict SIMD to operations proven associative in the encoding used; pin target-feature set for any build that writes/checks a baseline |
| GPU floating point differs by vendor/driver (FMA fusion, transcendental approximation, reduction order) | STACK (Layer 6), ARCHITECTURE (Sec 5), PITFALLS (#5, #6) -- corroborated across all four docs | Hard rule: no pixel entering a comparison is GPU-produced; GPU compute mirror is judged by categorical, not numeric, agreement with the CPU reference; cross-vendor equivalence tests (Intel/Nvidia/AMD/Apple) before calling the GPU path production-ready |
| PDF rasterization (pdfium) is not proven bit-exact across OS/architecture the way SVG is -- font substitution and hinting can differ by platform build | STACK (Layer 2) | Treat as "deterministic per pinned pdfium build and pinned fonts," not deterministic by construction; pin the exact pdfium binary tag; prove bit-exactness with a golden-image test before trusting the feature |
| Video hardware decode is not proven bit-exact against software decode, even though the codec spec is exact | STACK (Layer 4), PITFALLS (#9), and PROJECT.md's own Open Question | Software-only decode for anything feeding the verdict; rav1d is proven bit-exact for AV1, H.264/OpenH264 and FFmpeg paths are not proven and need a conformance-corpus test before the video feature ships |
| No mature Rust crate exists for CPU 3D mesh rasterization -- must hand-roll | STACK (Layer 5) | Build a small, scoped scanline/half-space z-buffer rasterizer with a documented, invariant floating-point operation order, since the exact arithmetic must be visible and testable, not inherited from an opaque dependency |
| VFR video timestamp drift and colour-matrix mismatches can masquerade as structural changes that never happened | PITFALLS (#10) | Align by presentation timestamp, not frame index; read and honor actual colourspace/matrix tags rather than assuming a default |

**The pattern:** every threat above reduces to one of two root causes -- (a) something in the pipeline calls into an OS-supplied or vendor-supplied numeric implementation instead of a vendored, pinned one, or (b) a determinism claim is asserted by specification rather than proven empirically against a cross-platform/cross-architecture golden corpus. Both root causes recur at every new format boundary; the roadmap should budget a corpus-based, CI-matrix proof for each format as it is added, not just once at the end.

## Unproven Claims Carried Forward as Phase Gates

Collected from every researcher's own stated uncertainty. Each becomes a gate: the phase that introduces the risk must not be called done until the claim is settled.

1. **PDF rasterization bit-exactness across OS/architecture** (STACK, MEDIUM confidence) -- settle with a golden-image test across the pdfium-pinned build, on all three target OSes, before the PDF feature is trusted.
2. **H.264/OpenH264 and FFmpeg-path cross-decoder bit-exactness** (STACK, LOW/unverified; PITFALLS #9 independently flags the same gap) -- settle with a conformance-corpus test, per PROJECT.md's own Open Question, before the video feature depends on it.
3. **Hand-rolled mesh rasterizer's determinism guidance** (STACK, MEDIUM confidence -- sound general IEEE-754 reasoning, not verified against a published case study for this exact use) -- settle by building the golden cross-OS/cross-arch hash test for the mesh rig as part of that phase's exit criteria, the same mechanism used for SVG.
4. **GPU-compute workgroup-size-dependent reduction non-determinism across vendors** (STACK Layer 6, MEDIUM -- well-established general knowledge, not a wgpu-specific citation this session) -- settle with the categorical-equivalence test suite across at least Intel/Nvidia/AMD/Apple GPUs, per Pitfall 6.
5. **`empfindung`'s exact license** (STACK, not independently re-verified this session) -- settle by checking its Cargo.toml/crates.io license field directly before depending on it.
6. **Whether any independent bit-exactness study exists for pdfium at all** (STACK) -- none was found; treat the absence itself as the finding, not a gap to explain away.
7. **No mature Rust Butteraugli port exists** (STACK, HIGH confidence in the negative finding) -- this is a real, permanent gap, not a phase gate to resolve; flag as an accepted limitation unless FFI to libjxl is later adopted.
8. **Video bit-exactness generally, as PROJECT.md's own stated Open Question** -- the single most explicitly named unproven claim in the whole research set, echoed independently by STACK and PITFALLS; this is the master gate for the entire video feature.

## Implications for Roadmap

Reconciling the three documents that each imply a build order (STACK implies dependency order per layer; FEATURES implies a dependency graph from table-stakes through differentiators; ARCHITECTURE gives an explicit nine-step suggested build order): all three agree on the same shape -- prove the spine on the simplest format first, prove determinism as infrastructure immediately after, then extend the format boundary with a low-risk second family before building anything user-facing, then stage harder formats from most-proven (SVG) to least-proven (video, mesh), with GPU/UI work sequenced after several formats already exist. ARCHITECTURE's explicit numbered order is the most detailed and is adopted directly below; FEATURES' dependency graph (classification before rule-file scoping, rule-file before GitHub Actions integration) nests inside it; STACK's per-layer license/determinism caveats attach as gates to the phase that first uses each layer. No document disagreed with another on ordering -- the only tension is that FEATURES treats mesh as P3/last-priority while ARCHITECTURE also sequences it last for a different reason (hardest test of the `Source` trait); both conclusions agree in outcome.

### Phase 1: Frozen spine -- comparison engine against raster images only
**Rationale:** ARCHITECTURE step 1 and FEATURES' dependency graph both start here: register/classify/score/report against `Vec<Frame>` directly, using raster as the only adapter, before the `Source` trait is even formalized. Cheapest way to validate phase correlation, block matching, and connected-component classification in isolation.
**Delivers:** Perceptual, antialiasing-aware structural diff core; CLI exit code + report artifact.
**Addresses:** Perceptual pixel-diff core, antialiasing-aware comparison, structural change classification (FEATURES table-stakes/differentiators).
**Avoids:** Pitfall 7 (memory blowup on decode -- configure `Limits` from day one) and Pitfall 8 (EXIF orientation/premultiplied alpha -- normalize on decode).

### Phase 2: Determinism proof as CI infrastructure
**Rationale:** ARCHITECTURE step 2 -- stand up the multi-OS/multi-arch CI matrix and hash-equality check while the surface area is still one rasterizer, one format. Finding a determinism bug here is tractable; finding one after six formats and a GPU mirror exist is not.
**Delivers:** Cross-OS/cross-arch golden-hash CI job; the hash-manifest mechanism the baseline store will reuse.
**Uses:** SHA-256 hashing of raw RGBA8 output (never a re-encoded format).
**Avoids:** Pitfall 2 (FMA/libm divergence) and Pitfall 3 (SIMD lane-width divergence) -- both must be caught here, before more formats build on top.

### Phase 3: Source trait + second family (animation/frame sequences)
**Rationale:** ARCHITECTURE step 3 -- reuses the raster rasterizer entirely, validates the trait boundary and per-frame-pair pairing without introducing a new rasterizer at the same time.
**Delivers:** Formalized `Source` trait; animation/frame-sequence comparison.
**Implements:** The `Source` trait pattern (ARCHITECTURE Pattern 1); the hint-channel-as-trait-output design.

### Phase 4: Baseline store, TOML rule engine, CLI, report artifact
**Rationale:** ARCHITECTURE step 4 and FEATURES' dependency graph agree this must land before any more formats, since every subsequent format's tests depend on committing a golden and getting a pass/fail exit code.
**Delivers:** Committed-goldens baseline store with hash manifest; declarative TOML rule file scoped by kind-of-change and named region; native-window review (pan/zoom/scrub, side-by-side + diff overlay).
**Addresses:** Baseline approve/reject workflow, named-region ignore, tolerance scoped by kind + region -- closes the confirmed field-wide gap.
**Avoids:** Pitfall 4 (threshold inflation) -- ship a scoped-exclusion example from day one, never a bare global threshold.

### Phase 5: SVG -- first real CPU rasterizer
**Rationale:** ARCHITECTURE step 5, STACK Layer 1 -- first place the determinism claim is tested against non-trivial rendering (curves, gradients, AA), using resvg/tiny-skia as the direct, already-proven precedent.
**Delivers:** CPU-only deterministic vector/document rasterization -- the core value proposition, must ship and be provable here.
**Uses:** `resvg` 0.48.1 + `tiny-skia` 0.12.0.
**Avoids:** Pitfall 1 (font/vector rasterization divergence) -- cross-OS golden test on a text-bearing fixture is this phase's exit criterion, not an afterthought.

### Phase 6: PDF (Cargo feature, off by default)
**Rationale:** ARCHITECTURE step 6, STACK Layer 2 -- structurally identical to SVG (decode document, rasterize pages as frames), so sequenced next while the pattern is fresh, but its determinism claim is unproven and must be independently audited.
**Delivers:** Feature-gated PDF page comparison via `pdfium-render`.
**Uses:** `pdfium-render` 0.9.3, pinned pdfium binary tag.
**Research flag:** PDF bit-exactness across OS is MEDIUM confidence and unverified independently -- needs its own golden-image proof before trust, per phase gate #1 above.

### Phase 7: wgpu compute mirror + Slint window
**Rationale:** ARCHITECTURE step 7 -- depends on `chrys-core` already being stable (a CPU reference to test against) and the CLI already existing headless. Deliberately sequenced after several formats exist so the interactive view has real content, and because GPU work is new to the author and should not block format work.
**Delivers:** Interactive native window with GPU-accelerated redraw, validated by categorical equivalence tests against the CPU reference; never authoritative for the verdict.
**Avoids:** Pitfall 5 (wgpu readback stalls) and Pitfall 6 (backend/vendor divergence) -- budget an explicit spike given the author has no prior GPU experience; test on Intel/Nvidia/AMD/Apple before calling this phase done.

### Phase 8: Video (Cargo feature, off by default)
**Rationale:** ARCHITECTURE step 8 -- reuses frame-sequence pairing from Phase 3 entirely; its new risk is decode, which is PROJECT.md's own stated Open Question.
**Delivers:** Feature-gated video comparison, software-decode-only for the comparison path.
**Uses:** `rav1d` (proven bit-exact for AV1), `openh264`/`symphonia`/`ffmpeg-next` for broader codec coverage, LGPL-only FFmpeg builds enforced at build time.
**Research flag:** This is the master phase gate -- video bit-exactness must be proven against a conformance corpus (Pitfall 9, Pitfall 10) before this feature ships; VFR timestamp alignment and colourspace-matrix handling are named design decisions, not implementation details.

### Phase 9: 3D mesh via fixed eight-view rig
**Rationale:** ARCHITECTURE step 9 and FEATURES both sequence this last -- hardest test of the `Source` trait, depends on a working CPU rasterizer pattern and a stable pipeline, and is (along with GPU work) one of the two areas genuinely new to the author.
**Delivers:** Fixed eight-view mesh comparison rig with named blind spot in the output.
**Uses:** `gltf`/`tobj`/`ply-rs-bw`/`stl_io` for mesh loading; a hand-rolled scanline/half-space z-buffer rasterizer on `glam`.
**Research flag:** No mature crate exists for the rasterizer itself -- this phase carries the highest implementation-cost/highest-novelty combination in the whole roadmap; budget accordingly and prove determinism with the same golden cross-OS/cross-arch hash mechanism used for SVG.

### Phase Ordering Rationale

- Algorithm correctness (1) precedes the determinism proof (2), which precedes the extension mechanism (3), which precedes product usability (4), which precedes harder rasterizers (5, 6), which precede GPU/UI (7), which precede the two riskiest, newest-to-the-author families (8, 9).
- Each phase adds exactly one new kind of risk, so a failure is attributable to the thing that just changed -- this is ARCHITECTURE's explicit design intent and all three ordering-implying documents converge on it.
- FEATURES' dependency graph nests inside this order without contradiction: structural classification (Phase 1) before rule-file scoping (Phase 4) before GitHub Actions integration (deferred to v1.x, attaches naturally after Phase 4/6).
- GitHub Actions reusable workflow + PR-comment format is deferred to "v1.x" per FEATURES' own MVP definition -- it depends on the report artifact format being stable, so it should land any time after Phase 4, not as its own numbered phase.

### Research Flags

Needs deeper research during planning:
- **Phase 6 (PDF):** pdfium's determinism claim is unproven; needs its own investigation into golden-image testing methodology and exact pinning strategy for `bblanchon/pdfium-binaries`.
- **Phase 7 (GPU/wgpu):** author has no prior GPU experience; needs a dedicated spike/prototype before the phase plan, specifically on the buffer-staging and cross-vendor equivalence testing patterns.
- **Phase 8 (Video):** decode bit-exactness is PROJECT.md's own named Open Question; needs research into building or sourcing a conformance-style test corpus, and into VFR/colourspace handling design.
- **Phase 9 (Mesh):** no existing crate or close precedent for the rasterizer; needs research into fixed-point vs. IEEE-754 determinism strategy for the hand-rolled implementation, informed by Phase 5's SVG determinism proof.

Phases with standard, well-documented patterns (research-phase can likely be skipped):
- **Phase 1 (raster diff core):** phase correlation, block matching, and connected-component labelling all have named, decades-old reference algorithms (Kuglin & Hines; Bouguet; Wu/Otoo/Suzuki) with existing implementations to study.
- **Phase 3 (animation/frame sequences):** reuses Phase 1's rasterizer entirely; only adds frame pairing.
- **Phase 5 (SVG):** `resvg`/`tiny-skia` is a mature, documented, drop-in dependency with a stated determinism design.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM-HIGH | Crate identities, licenses, and the SVG determinism claim are HIGH and cross-checked against crates.io/docs.rs directly this session. The CPU 3D mesh rasterization layer has no mature crate -- this is a genuine ecosystem gap, not weak research, and is correctly flagged LOW by nature. |
| Features | MEDIUM | Cross-checked against official docs, READMEs, and vendor pricing pages; some vendor marketing claims (Applitools "99.9% false positive elimination," Vizzly's Honeydiff benchmarks) are explicitly marked LOW/unverifiable rather than accepted at face value. |
| Architecture | MEDIUM-HIGH | The algorithms (phase correlation, block matching, connected-component labelling) are well-documented, decades-old, with named academic references. The exact GPU/CPU equivalence pattern for this specific project is a recommendation synthesized for this use case, not a proven existing pattern -- correctly flagged as such. |
| Pitfalls | MEDIUM | Web sources, no curated Context7/docs access this session; several claims (font rendering variance, wgpu driver bugs, FFmpeg licensing, image-crate CVEs) are cross-checked across 2+ independent sources and carry MEDIUM-HIGH confidence individually. |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- **PDF and H.264/video bit-exactness are unproven claims, not established facts** -- both must be settled with golden-corpus CI tests before their respective features (Phases 6, 8) are considered done; do not let "the spec says bit-exact" substitute for the actual test.
- **No mature Rust crate exists for CPU 3D mesh rasterization** -- Phase 9 must budget real engineering time to hand-roll this, and the determinism guidance for it (FMA-safety, precomputed matrices) is sound reasoning but not independently validated against a published case study; validate empirically as part of that phase.
- **GPU-compute cross-vendor equivalence is asserted from general GPU-computing knowledge, not a Chrysoberyl-specific citation** -- Phase 7 needs real hardware from at least three vendors (Intel, Nvidia/AMD, Apple) before the wgpu mirror is trusted, per Pitfall 6.
- **`empfindung`'s license was not independently re-verified this session** -- verify its Cargo.toml/crates.io license field directly before depending on it in the perceptual-metrics layer.
- **No independent public post-mortem of a team abandoning a visual-diff tool was found** -- the threshold-inflation failure mode (Pitfall 4) is corroborated by multiple independent industry articles describing the general pattern, but no single named incident exists to cite; treat the pattern as well-established, not singularly documented.

## Sources

### Primary (HIGH confidence)
- crates.io / docs.rs / lib.rs pages for resvg, tiny-skia, pdfium-render, image, symphonia, rav1d, wgpu, naga, clap, thiserror, anyhow, cargo-insta -- checked directly this session
- Google's pdfium LICENSE and third-party bundle (FreeType, libjpeg)
- gfx-rs/wgpu "Known Driver Issues" wiki -- first-party, authoritative
- FFmpeg official legal page (ffmpeg.org/legal.html)
- rav1d/rav2d project documentation for the bit-exactness claim against dav1d

### Secondary (MEDIUM confidence)
- Vercel Satori issue #708 (platform-dependent SVG text rendering)
- Rust project issue tracker (rust-lang/rust, rust-lang/rfcs) on FMA/float determinism
- NVIDIA Developer Forums on CPU/GPU floating-point divergence
- Chromatic, Percy, Applitools, Lost Pixel, Argos CI, Vizzly, odiff, pixelmatch, BackstopJS, jest-image-snapshot, diff-pdf, Skia GM tests -- vendor docs and pricing pages
- image-rs/image GitHub issue #938 and CVE-2023-29408 (TIFF decoder memory limits)

### Tertiary (LOW confidence)
- `empfindung`'s exact license (recalled, not independently re-fetched this session -- flagged for verification)
- Vendor marketing claims (Applitools "99.9% false positive elimination," Vizzly's Honeydiff benchmark comparisons) -- explicitly not independently verified

---
*Research completed: 2026-09-06*
*Ready for roadmap: yes*
