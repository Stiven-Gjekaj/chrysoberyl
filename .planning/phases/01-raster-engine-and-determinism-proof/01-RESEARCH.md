# Phase 1: Raster engine and determinism proof - Research

**Researched:** 2026-09-06
**Domain:** Deterministic CPU image comparison (phase correlation, block matching, connected-component labelling) and cross-platform floating-point reproducibility in Rust
**Confidence:** MEDIUM-HIGH

## Summary

Phase 1 has two halves that must both hold: a comparison algorithm (phase correlation, block
match, connected-component labelling, an antialiasing-aware verdict) and a determinism proof
(CI hashes raw RGBA8 output and gets the same hash on Linux, macOS, Windows, x86-64 and
aarch64). The algorithm half is standard and well documented; every sub-step named in
`research/ARCHITECTURE.md` has a citable reference implementation. The determinism half is
where this session's own verification changed the picture the project had been planning
around.

The single most load-bearing finding this session: Rust's standard library documents, in the
current stable docs, that `sqrt` and `mul_add` are guaranteed bit-reproducible ("guaranteed not
to change"), while `sin`, `cos`, `cbrt`, `powf`, `exp`, `ln` and `atan2` are documented as
"Unspecified precision... non-deterministic... varies by platform, Rust version, and can even
differ within the same execution from one invocation to the next." `[VERIFIED:
doc.rust-lang.org/std/primitive.f64.html]` This is stronger and more precise than the risk the
project's own prior research stated. It also means the FMA-contraction risk the project has
been planning around is smaller than feared (plain `a * b + c` is never silently fused in stable
Rust) but the transcendental-function risk is exactly as large as feared, and it lands directly
on CORE-03 (colour-difference reporting) the moment a Lab colour space is used, because Lab
conversion needs `cbrt` regardless of which colour-difference formula sits on top of it.

A second load-bearing finding: `rustfft`'s default `FftPlanner` auto-detects AVX/SSE/NEON at
runtime and switches to a different algorithm depending on what the running CPU supports.
`[VERIFIED: docs.rs/rustfft]` Two machines with the same OS and the same Rust build can silently
take different floating-point code paths inside the same phase-correlation call. The library
ships the fix already: `FftPlannerScalar` forces the non-SIMD path and must be the planner used
in the register stage that feeds a hash-checked verdict.

A third finding worth restating plainly for the CI plan: GitHub's free, unlimited, standard
public-repository runner matrix in 2026 covers all five of the project's target
(OS, architecture) pairs without workarounds. `[VERIFIED: docs.github.com/en/actions/reference/
runners/github-hosted-runners]` No QEMU, no self-hosted runner, and no cross-compilation
substitute is needed for DET-01/DET-02's CI proof.

**Primary recommendation:** Build the register stage (phase correlation + block match) against
`FftPlannerScalar` from `rustfft` 6.4.1, route every colour-space and colour-difference
computation through the `palette` crate's `libm` feature instead of `std`, classify residual
regions with `imageproc`'s `region_labelling` module, and prove determinism with a SHA-256 hash
of raw RGBA8 buffers on the six free GitHub-hosted runner labels identified below.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CORE-01 | Name each change by kind, never a bare pixel count | Classify stage: `imageproc::region_labelling::connected_components`, typed by residual pattern (see Architecture Patterns) |
| CORE-02 | Register a translated region and report offset in pixels | Phase correlation (Q2) gives the global offset; block match gives per-region offsets |
| CORE-03 | Report colour change as a difference value plus both colours | `palette` crate, Lab colour space, `libm` feature (Q6, Q8) |
| CORE-04 | Group changed pixels into labelled regions with bounding boxes | `imageproc` connected-component labelling (Q4) |
| CORE-05 | Do not report an antialiasing difference a person cannot see | pixelmatch's `antialiased()`/`hasManySiblings()` heuristic, adapted (Q5) |
| CORE-06 | Refuse a pair it cannot register, and say why | Phase-correlation peak-confidence threshold; low peak sharpness or high residual after warp is the refusal signal |
| CORE-07 | Compare only near-identical pairs, and say so when too different | Same signal as CORE-06, reported as a distinct verdict variant, not a crash |
| DET-01 | Identical verdict on Linux, macOS, Windows | CI matrix (Q7); FMA/libm audit (Q6) |
| DET-02 | Identical verdict on x86-64 and aarch64 | CI matrix (Q7); `FftPlannerScalar` (Q2); libm crate (Q6) |
| DET-03 | CI proves DET-01/DET-02 on every commit, hash of raw RGBA8, never re-encoded | SHA-256 via `sha2` 0.11.0 (Q6, Q10) |
| DET-04 | No pixel entering a comparison is GPU-produced | No GPU crate in this phase's dependency graph at all (hard constraint, self-enforcing by omission) |
| DET-06 | Comparison path calls no platform transcendental function; FMA contraction disabled | `libm` crate + `palette`'s `libm` feature; plain arithmetic is not auto-contracted (Q6) |
| SRC-01 | A raster image pair is compared (PNG, JPEG, WebP, TIFF) | `image` 0.25.10, `default-features = false` (Q8) |

## Architectural Responsibility Map

This project has no browser/server/CDN tiers. The equivalent boundary is the one
`research/ARCHITECTURE.md` already drew: adapter (format-aware) vs. engine (format-blind) vs.
CLI (orchestration only). Mapped against the generic tier model for the plan-checker:

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Raster decode (PNG/JPEG/WebP/TIFF) | Source adapter (`chrys-source-raster`) | — | Format knowledge is confined here per the "no per-format special case" rule; nothing past this point may branch on file type |
| Pixel normalization (orientation, alpha, bit depth) | Source adapter | — | Same boundary: the adapter's job is to hand the engine one canonical shape |
| Register (phase correlation + block match) | Engine (`chrys-core`) | — | Format-blind by construction; operates only on `Frame` buffers |
| Classify (connected-component labelling) | Engine (`chrys-core`) | — | Pure function of the residual field, no format or file-path access |
| Colour-difference scoring | Engine (`chrys-core`) | — | Table-free in Phase 1 (rule engine is Phase 3); computes the value, does not yet gate pass/fail |
| Verdict / report data model | Engine (`chrys-core`) | CLI (serialization only) | The struct is designed now; only its file-writing form is Phase 3 (CLI-02) |
| CI determinism harness | CLI / test harness | — | Drives the engine headlessly; owns hashing and cross-runner comparison, not the algorithm itself |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `image` | 0.25.10 | PNG/JPEG/WebP/TIFF decode, `Limits` API, EXIF orientation | De facto Rust standard, pure-Rust decoders for every SRC-01 format `[VERIFIED: crates.io, package-legitimacy OK]` |
| `rustfft` | 6.4.1 | FFT for phase correlation | Only mature, actively maintained pure-Rust FFT; `[VERIFIED: crates.io, package-legitimacy OK, published 2015, 580K downloads/week, repo github.com/ejmahler/RustFFT]` |
| `realfft` | 3.5.0 | Real-to-complex wrapper around `rustfft` | Halves the work for real-valued pixel data; `[VERIFIED: crates.io, package-legitimacy OK, 346K downloads/week, repo github.com/HEnquist/realfft]` |
| `imageproc` | 0.27.0 | Connected-component labelling (`region_labelling` module) | Image-rs org crate, `[VERIFIED: crates.io registry API, direct query this session: created 2016-01-10, 12.7M downloads, repo github.com/image-rs/imageproc]`. Note: the automated package-legitimacy check returned `SUS` for this package due to a registry-lookup failure inside that tool (`unknown-age`, `no-repository`); a direct `crates.io/api/v1/crates/imageproc` query this session returned full, clean metadata, so this is treated as `OK` on direct evidence, not on the tool's cached verdict. |
| `palette` | 0.7.7 | Lab colour space + colour-difference (`EuclideanDistance`, `Ciede2000`), with a `libm` feature | `[VERIFIED: crates.io, package-legitimacy OK, 810K downloads/week, repo github.com/Ogeon/palette]`. Chosen over `empfindung`/`delta_e` — see Alternatives Considered. |
| `libm` | 0.2.16 | Pure-Rust `cos`, `cbrt`, `atan2`, `powf` etc. for any transcendental the comparison path cannot avoid | `[VERIFIED: crates.io, package-legitimacy OK, repo github.com/rust-lang/compiler-builtins — an official rust-lang org crate]` |
| `sha2` | 0.11.0 | SHA-256 hash of raw RGBA8 buffers for the golden/CI proof | `[VERIFIED: crates.io, package-legitimacy OK, 18.8M downloads/week, repo github.com/RustCrypto/hashes]` |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `clap` | 4.6.6 | CLI argument parsing (derive feature) | The CLI binary crate only |
| `thiserror` | 2.0.20 | Typed, matchable errors | `chrys-core` and adapter crates (library-side) |
| `anyhow` | 1.0.104 | Convenient error propagation | CLI binary crate only, never in a library's public API |
| `insta` / `cargo-insta` | 1.48.0 | Snapshot testing for structural-diff reports | Golden-report regression tests |
| `criterion` | 0.8.2 | Benchmark harness | Register/classify hot-path benchmarks |
| `glam` | 0.33.6 | Vector/matrix math | Only if the register stage's transform estimation benefits from it; not required for pure translation |

All versions above were confirmed this session via `cargo search <pkg> --limit 1` against the
live crates.io registry, cross-checked against `gsd_run query package-legitimacy check`.
`[VERIFIED: crates.io registry, this session]`

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `palette` (Lab + colour-difference) | `empfindung` 0.2.6 | `research/STACK.md` already flagged `empfindung`'s exact license as unverified this session. This session's own `package-legitimacy check` independently flags it `SUS` (34 downloads/week). `delta_e` 0.2.6-equivalent scores the same way (49 downloads/week, `SUS`). `palette` has an `OK` verdict, 810K downloads/week, a clear MIT/Apache-2.0 license, and — decisively — a `libm` feature that routes every transcendental through pure Rust instead of `std`, which neither smaller crate is documented to offer. Use `palette`. |
| `rustfft`'s default `FftPlanner` | `FftPlannerScalar` (still `rustfft`, different entry point) | The default planner auto-detects AVX/SSE/NEON and silently changes the arithmetic path per machine (Q2, Q6). Use `FftPlannerScalar` for anything that feeds a hash-checked verdict. |
| Hand-rolled connected-component labelling | `imageproc::region_labelling` | `imageproc` implements the standard two-pass algorithm (Wu/Otoo/Suzuki-family, per `research/ARCHITECTURE.md`) already; hand-rolling duplicates well-tested code for no determinism benefit, since the algorithm is pure integer comparison with no transcendental exposure either way. |
| CIEDE2000 for CORE-03 in Phase 1 | `EuclideanDistance` (CIE76-style ΔE) in Phase 1, `Ciede2000` later | CIEDE2000 needs `atan2`, `sin`, `cos`, `powf` on top of the `cbrt` every Lab conversion already needs. `EuclideanDistance` needs only `sqrt`, which is the one transcendental-adjacent op the Rust standard library documents as bit-reproducible. Ship the simpler formula first; upgrade to `Ciede2000` (still via `palette`'s `libm` feature) once the golden-hash CI job is green and a second data point exists. |

**Installation:**
```bash
cargo new --lib crates/chrys-core
cargo new --lib crates/chrys-source
cargo new --lib crates/chrys-source-raster
cargo new --bin crates/chrys-cli

# chrys-core/Cargo.toml
cargo add rustfft@6.4.1 realfft@3.5.0 imageproc@0.27.0 --manifest-path crates/chrys-core/Cargo.toml
cargo add palette@0.7.7 --no-default-features --features libm --manifest-path crates/chrys-core/Cargo.toml
cargo add libm@0.2.16 thiserror@2.0.20 --manifest-path crates/chrys-core/Cargo.toml

# chrys-source-raster/Cargo.toml
cargo add image@0.25.10 --no-default-features --features png,jpeg,webp,tiff --manifest-path crates/chrys-source-raster/Cargo.toml

# chrys-cli/Cargo.toml
cargo add clap@4.6.6 --features derive --manifest-path crates/chrys-cli/Cargo.toml
cargo add anyhow@1.0.104 sha2@0.11.0 --manifest-path crates/chrys-cli/Cargo.toml

# dev-dependencies, workspace-wide
cargo add insta@1.48.0 criterion@0.8.2 --dev --manifest-path crates/chrys-core/Cargo.toml
```

**Version verification:** every version above was checked with `cargo search <pkg> --limit 1`
against the live registry this session (2026-09-06), not recalled from training data. `image`,
`rustfft`, `realfft`, `imageproc`, `palette`, `libm`, `sha2`, `clap`, `toml`, `thiserror`,
`anyhow`, `insta`/`cargo-insta`, `criterion` all matched the versions `research/STACK.md`
recorded three months of research-cadence earlier, with two exceptions worth flagging: `toml`
now reports `1.1.5+spec-1.1.0` (a 1.x release; `research/STACK.md` had not pinned a `toml`
version), and `criterion` reports `0.8.2` (a patch ahead of `research/STACK.md`'s `0.8.x`).
Neither is a breaking surprise, but re-run `cargo search` at plan time rather than trusting this
document's numbers verbatim once implementation starts.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `image` | crates.io | 2014-11-20 (~12 yr) | 3.7M/wk | github.com/image-rs/image | OK | Approved |
| `rustfft` | crates.io | 2015-01-25 (~11 yr) | 580K/wk | github.com/ejmahler/RustFFT | OK | Approved |
| `realfft` | crates.io | 2020-06-17 (~6 yr) | 346K/wk | github.com/HEnquist/realfft | OK | Approved |
| `imageproc` | crates.io | 2016-01-10 (~10 yr, verified directly against crates.io API this session) | 12.7M | github.com/image-rs/imageproc | OK (direct verification; automated tool returned stale `SUS`) | Approved |
| `palette` | crates.io | 2016-01-12 (~10 yr) | 810K/wk | github.com/Ogeon/palette | OK | Approved |
| `libm` | crates.io | 2018-07-14 (~8 yr) | 8.6M/wk | github.com/rust-lang/compiler-builtins | OK | Approved |
| `sha2` | crates.io | 2016-05-06 (~10 yr) | 18.8M/wk | github.com/RustCrypto/hashes | OK | Approved |
| `clap`, `thiserror`, `anyhow`, `toml`, `insta`/`cargo-insta`, `criterion` | crates.io | all 6+ yr | all 2M/wk+ | dtolnay/clap-rs/toml-rs/mitsuhiko orgs | OK | Approved |
| `empfindung` | crates.io | 2021-06-30 | 34/wk | github.com/mina86/empfindung | SUS (low-downloads) | Not selected — see Alternatives Considered |
| `delta_e` | crates.io | 2017-05-26 | 49/wk | github.com/elliotekj/DeltaE | SUS (low-downloads) | Not selected — see Alternatives Considered |

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** `empfindung` and `delta_e` were probed and rejected
in favour of `palette`, not adopted, so no `checkpoint:human-verify` gate is needed for them.
`imageproc`'s stale `SUS` from the automated tool is superseded by a direct, verified crates.io
API query this session; no `checkpoint:human-verify` is needed for `imageproc` either, since the
disposition rests on direct evidence, not on trusting the tool's cache.

## Architecture Patterns

### System Architecture Diagram

```
   baseline.png, candidate.png
            |
            v
   +-------------------+
   |  Source adapter    |   decode (image crate, Limits set) -> normalize
   |  (chrys-source-    |   (EXIF orientation, straight alpha, 8/16-bit)
   |   raster)          |
   +-------------------+
            |
            v  Vec<Frame>  (RGBA8, same shape for baseline and candidate)
            |
   +-------------------+
   |  REGISTER          |   1. downsample luma to a fixed working resolution
   |  (chrys-core)      |   2. apply a precomputed Hann window
   |                    |   3. FftPlannerScalar: forward R2C FFT both frames
   |                    |   4. cross-power spectrum, phase-only normalize
   |                    |   5. inverse FFT -> correlation surface
   |                    |   6. coarse peak + subpixel refine (Guizar-Sicairos)
   |                    |   7. peak confidence check -> refuse if too low
   |                    |        (CORE-06, CORE-07)
   |                    |   8. warp candidate by coarse offset
   |                    |   9. coarse-to-fine block match on a pyramid,
   |                    |      residual = |diff| image, integral-image SAD
   +-------------------+
            |
            v  residual field + per-block offsets + colour deltas
            |
   +-------------------+
   |  CLASSIFY          |   imageproc::region_labelling::connected_components
   |  (chrys-core)       |   on the thresholded residual -> typed regions
   |                    |   antialiasing filter drops AA-only regions (CORE-05)
   +-------------------+
            |
            v  Vec<TypedRegion> { kind, bbox, offset_px, colour_delta }
            |
   +-------------------+
   |  VERDICT           |   in-memory struct (report artifact's Phase 3 shape
   |  (chrys-core)       |   is designed here; not yet serialized to a file)
   +-------------------+
            |
            v
   CLI prints verdict; CI hashes the raw RGBA8 buffers with SHA-256
   and compares the hash across Linux/macOS/Windows x x86-64/aarch64
```

### Recommended Project Structure

```
chrysoberyl/
├── Cargo.toml               # [workspace] members, resolver = "3", [workspace.lints]
├── crates/
│   ├── chrys-core/          # register, classify, verdict struct — no format crate deps
│   │   ├── src/
│   │   │   ├── register/    # phase correlation, block match, pyramids
│   │   │   ├── classify/    # connected-component labelling, antialiasing filter
│   │   │   └── verdict.rs   # the Phase-3-ready report data model
│   │   └── Cargo.toml
│   ├── chrys-source/        # the Source trait + Frame type (thin, format-free)
│   ├── chrys-source-raster/ # image crate wrapper: decode, normalize, Limits
│   └── chrys-cli/           # binary: wires adapter to engine, prints verdict
└── tests/
    └── golden/              # committed baseline pairs + expected SHA-256 hashes
```

### Structure Rationale

- `chrys-core` has zero format-crate dependencies in its own `Cargo.toml`. This is the same
  compile-time enforcement of "no per-format special case" that `research/ARCHITECTURE.md`
  already specified; Phase 1 is where it is first laid down, not just designed.
- `chrys-source` exists as its own tiny crate even though Phase 1 has only one adapter, because
  Phase 2 (`SRC-02`, `SRC-08`) needs to add a second adapter without touching `chrys-core`, and
  the trait needs a stable home before that happens.

### Pattern 1: `FftPlannerScalar`, not `FftPlanner`, on the verdict path

**What:** `rustfft::FftPlanner` auto-detects AVX2/AVX/SSE4.1/NEON at runtime and dispatches to a
different, faster algorithm depending on what instruction sets the running CPU exposes.
`[VERIFIED: docs.rs/rustfft — "Simply plan a FFT using the FftPlanner on a machine that supports
the avx and fma CPU features, and RustFFT will automatically switch to faster AVX-accelerated
algorithms."]` This is precisely the SIMD-reduction-order risk `research/PITFALLS.md` Pitfall 3
already names in the abstract; here it is the literal, concrete mechanism inside the one FFT
crate this project will actually depend on.

**When to use:** Any FFT call whose output feeds a value that a golden-hash CI test checks.
Use `FftPlannerScalar` there, unconditionally, regardless of what CPU features are available at
build or run time.

**Example:**
```rust
// Source: docs.rs/rustfft (FftPlannerScalar), adapted for this project's register stage
use rustfft::{FftPlannerScalar, num_complex::Complex32};

fn plan_reference_fft(len: usize) -> std::sync::Arc<dyn rustfft::Fft<f32>> {
    // FftPlannerScalar never dispatches to AVX/SSE/NEON, so the exact same
    // arithmetic sequence runs on every target this project's CI matrix covers.
    let mut planner = FftPlannerScalar::new();
    planner.plan_fft_forward(len)
}
```

A GPU-mirror or interactive-only FFT path (Phase 6+) may use the auto-dispatching
`FftPlanner`, because that path never writes a verdict — see `research/ARCHITECTURE.md`
Pattern 2.

### Pattern 2: `palette`'s `libm` feature on every colour-space conversion

**What:** `palette::Lab` conversion needs `cbrt` for the XYZ-to-Lab nonlinearity regardless of
which colour-difference formula is layered on top. Rust's own `std` documents `cbrt` (and
`powf`, `atan2`, `sin`, `cos`, `exp`, `ln`) as having "Unspecified precision... non-deterministic
[output that] varies by platform, Rust version, and can even differ within the same execution
from one invocation to the next." `[VERIFIED: doc.rust-lang.org/std/primitive.f64.html]`
`palette` supports `#![no_std]` and states: "It uses `libm`, via the `libm` feature, to provide
the floating-point operations that are typically in `std`." `[CITED: docs.rs/palette]`

**When to use:** Every call into `palette`'s colour types and `color_difference` module,
anywhere in the register/classify path.

**Example:**
```rust
// Source: palette crate docs (docs.rs/palette), libm feature enabled in Cargo.toml
// Cargo.toml: palette = { version = "0.7.7", default-features = false, features = ["libm"] }
use palette::{Lab, Srgb, IntoColor};
use palette::color_difference::EuclideanDistance;

fn colour_delta(a: Srgb<f32>, b: Srgb<f32>) -> f32 {
    let lab_a: Lab = a.into_color();
    let lab_b: Lab = b.into_color();
    lab_a.distance(lab_b) // routed through libm's cbrt internally, not std's
}
```

### Pattern 3: refuse before you classify (CORE-06, CORE-07)

**What:** The phase-correlation peak carries a confidence signal (peak height relative to the
surrounding correlation-surface noise floor). A near-identical pair produces a sharp, high peak;
an unrelated pair produces a flat, ambiguous surface. Check this before ever reaching the block
match or classify stages.

**When to use:** Immediately after the coarse phase-correlation step, before any warp or block
match is attempted. This is the concrete implementation seam for CORE-06/CORE-07's "refuse a
pair it cannot register, and say why."

**Trade-off:** The exact confidence threshold is a tuning decision, not a determinism concern
(it operates on already-computed correlation values, and only feeds a boolean decision, not the
verdict's floating-point payload). Treat the specific threshold value as `[ASSUMED]` pending a
golden-corpus calibration pass; do not hardcode a threshold without a test fixture that exercises
both a pair that should register and one that should not.

### Anti-Patterns to Avoid

- **Computing SAD/SSD per block, per candidate, from scratch:** For a given candidate global
  translation, build one full-resolution difference image once (O(pixels)), then use an integral
  image over that single difference image to answer every block's windowed sum in O(1). Looping
  pixel-by-pixel inside each block for each candidate is the naive version `research/
  ARCHITECTURE.md` already names as the thing integral images exist to avoid.
- **Calling `.mul_add()` for a speed win without a cross-arch check:** `f64::mul_add` is
  documented, as of the Rust version in this environment, to be "guaranteed to be the rounded
  infinite-precision result" — but that guarantee is only as good as the software fallback used
  on hardware without a native FMA instruction, and that fallback has had real, documented
  subnormal-rounding bugs in Rust's own standard library and in musl libc. `[CITED: shnatsel.
  github.io/implementing-fma-finding-bugs-in-std]` Do not introduce `mul_add` into the register
  or classify hot path without a same-input-different-architecture CI check exercising it
  directly.
- **Using `image::DynamicImage::blur()` (or any filter) on a straight-alpha `Frame` buffer
  without converting first:** at least one `image` crate operation assumes premultiplied alpha
  input for images with non-constant alpha. `[CITED: community/deepwiki synthesis of image-rs
  source, not independently re-verified against the exact function this session — treat as
  MEDIUM confidence]` Normalize to one documented convention (straight alpha, per `research/
  PITFALLS.md` Pitfall 8) on decode, and never call an `image`-crate filter directly on the
  canonical `Frame` buffer without converting first.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| FFT for phase correlation | A custom radix-2 FFT | `rustfft` + `realfft` (`FftPlannerScalar`) | Mature, tested, handles arbitrary lengths via mixed-radix, not just powers of two `[CITED: docs.rs/rustfft]` |
| Connected-component labelling | A custom two-pass union-find | `imageproc::region_labelling` | Already implements the standard algorithm `research/ARCHITECTURE.md` names (Wu/Otoo/Suzuki-family); no determinism upside to reimplementing pure-integer union-find |
| Lab colour space + colour difference | A hand-rolled XYZ/Lab/CIEDE2000 pipeline | `palette` with the `libm` feature | Gets the `libm`-routed transcendental-safety for free; a hand-rolled version would need to independently re-derive and test that same libm-routing discipline |
| SHA-256 for the golden hash | A hand-rolled hash function | `sha2` | Correctness of a hash function is exactly the kind of code where "the popular library" and "the tested library" are the same library; RustCrypto's `sha2` is the ecosystem standard |
| Image decode with memory limits | A custom PNG/JPEG/WebP/TIFF parser | `image` crate's `Limits` API | CVE-2023-29408 exists precisely because a hand-rolled or unguarded decode path is a documented, exploitable class of bug, per `research/PITFALLS.md` Pitfall 7 |

**Key insight:** every "don't hand-roll" item above is also, independently, a determinism
concern: a hand-rolled version has to separately earn the same cross-platform bit-identity
guarantee an established crate already states and, in `palette`'s and `libm`'s case, has
already engineered a feature specifically to provide.

## Common Pitfalls

### Pitfall 1: Assuming rustc auto-fuses `a * b + c` into FMA

**What goes wrong:** A plan defends against a risk that does not exist in the form it was
described: silent, compiler-inserted FMA contraction of ordinary arithmetic changing rounding
between platforms.

**Why it happens:** Other languages' compilers (C/C++ with `-ffp-contract=fast`, which is GCC's
and Clang's default for C++/GNU C) do contract by default. Rust does not carry that default.
`[VERIFIED: doc.rust-lang.org/nightly/core/intrinsics/fn.fmuladdf64.html — the fused-or-not
choice is only reachable through an explicit, unstable, nightly-only intrinsic, never through
plain `a * b + c` in stable Rust]`

**How to avoid:** Do not spend plan effort disabling FMA contraction for code that never opts
into it. Spend that effort instead on the two risks that are real: (1) any explicit call to
`.mul_add()` (Pitfall 2 below), and (2) any dependency whose own `Cargo.toml`/build script
opts into `-C target-feature=+fma` or an LLVM fast-math-equivalent flag for its own compiled
code — audit third-party crates for this, not first-party arithmetic.

**Warning signs:** A plan task reads "disable FMA contraction" as if it were a compiler flag to
flip. There is no such default to flip off in stable Rust for ordinary arithmetic; there is only
a discipline around explicit `mul_add()` calls to enforce.

**Phase to address:** Phase 1, in the plan-writing step — reframe the DET-06 task from "disable
contraction" to "audit for explicit `mul_add()` calls and third-party fast-math opt-ins."

### Pitfall 2: Trusting `mul_add()`'s "guaranteed" wording without a cross-arch test

**What goes wrong:** `f64::mul_add` is documented, in the Rust version installed in this
environment, as "guaranteed to be the rounded infinite-precision result... guaranteed not to
change." A plan reads this and concludes the function is safe to use freely across DET-02's
x86-64/aarch64 matrix with no further check.

**Why it happens:** The guarantee describes the specified behaviour, not the state of every
implementation that provides it. A named investigation into Rust's and musl's `fma`/`mul_add`
software-fallback code (used on hardware without a native FMA instruction — some pre-AVX2 x86,
32-bit ARM, some embedded targets) found real, historical subnormal-rounding bugs traced to
FreeBSD-derived code reused across toolchains. `[CITED: shnatsel.github.io/implementing-fma-
finding-bugs-in-std]` 64-bit ARM (this project's aarch64 target) was reported unaffected by the
specific bug found, but the pattern — "guaranteed" wording outrunning every implementation's
actual behaviour — is the general shape of every determinism claim this project has to verify
empirically rather than accept from documentation.

**How to avoid:** If `mul_add()` is used anywhere in the register/classify hot path, add it to
the same golden-hash CI matrix that proves DET-01/DET-02, specifically exercising values near
subnormal range, not just typical pixel-value magnitudes.

**Warning signs:** A hash mismatch that only appears on one architecture, isolated to a code
path that recently added a `mul_add()` call.

**Phase to address:** Phase 1's determinism CI job (DET-03) — treat `mul_add` sites as flagged
locations requiring an explicit test case, not as "already safe because the docs say so."

### Pitfall 3: Letting `FftPlanner`'s runtime CPU-feature dispatch reach the verdict path

**What goes wrong:** Two CI runners with the same OS but different CPU generations (both
"x86-64", but one has AVX2 and one doesn't) silently execute different FFT algorithms for the
same phase-correlation call, and DET-02's cross-architecture hash check fails for a reason that
has nothing to do with x86-64 vs aarch64.

**Why it happens:** `rustfft::FftPlanner`'s documented behaviour is to auto-detect and switch
algorithms based on the running CPU's feature flags. `[VERIFIED: docs.rs/rustfft]`

**How to avoid:** Use `FftPlannerScalar` (see Pattern 1) for every FFT call in the register
stage. Reserve the auto-dispatching `FftPlanner` for a future GPU-mirror or interactive-only
path that never feeds a hash-checked verdict.

**Warning signs:** A golden-hash CI job passes reliably on `ubuntu-24.04` but fails
intermittently depending on which specific GitHub-hosted VM instance the job happened to land
on — that pattern points at CPU-feature-dependent dispatch, not at OS or architecture.

**Phase to address:** Phase 1, at the moment the register stage's FFT calls are written, not
discovered later via a flaky CI job.

### Pitfall 4: Building the Hann window with `std::f64::cos()` per-pixel

**What goes wrong:** The window function applied before the FFT (to reduce edge-effect
"streaking" in the correlation peak, per the phase-correlation literature) is typically a raised
cosine: `w(n) = 0.5 - 0.5 * cos(2*pi*n / (N-1))`. `[CITED: general phase-correlation windowing
literature; Wikipedia "Phase correlation" and related apodization sources, MEDIUM confidence]`
Computed with `std`'s `cos`, this hits exactly the "Unspecified precision... non-deterministic"
function this research flags as the sharpest DET-06 risk.

**How to avoid:** Two independent mitigations, either sufficient alone: (1) downsample every
frame pair to one fixed working resolution before applying the window (this project's near-
identical-pair assumption makes this a reasonable, documented design choice, not a hack), which
turns the window into a single fixed-size coefficient table computed once; (2) compute that
table with `libm::cos` instead of `std::f64::cos`, so the coefficients are bit-identical across
every OS and architecture in the CI matrix regardless of which downsample resolution is chosen.
Do both: fix the resolution for speed and cache-friendliness, and use `libm` for the one-time
table computation for determinism.

**Warning signs:** A golden hash that matches on every OS at one working resolution but breaks
when the working resolution is later made configurable per-image-size.

**Phase to address:** Phase 1's register-stage implementation, before any golden fixture is
committed — the working resolution is a determinism-relevant design decision, not a performance
tuning knob to defer.

### Pitfall 5: Believing the CI-runner gap forces a workaround

**What goes wrong:** A plan budgets time for QEMU emulation, cross-compilation, or a self-hosted
runner to cover a `(OS, architecture)` pair, on the assumption that GitHub does not offer it for
free on a public repository.

**Why it happens:** This was true as recently as 2024–2025: Linux ARM64 runners for public
repos only reached general availability in 2025, and Windows ARM64 standard runners only
extended to private repos as of a January 2026 changelog entry. `[CITED: github.blog/
changelog/2025-08-07-arm64-hosted-runners-for-public-repositories-are-now-generally-available,
github.blog/changelog/2026-01-29-arm64-standard-runners-are-now-available-in-private-
repositories]` A plan drafted from stale knowledge of this landscape will overbuild the CI
matrix.

**How to avoid:** See Environment Availability and Q7 below — as of this research date, all six
target labels are free, standard, unlimited-minute runners on public repositories. Use them
directly.

**Warning signs:** A plan task that says "set up cross-compilation for aarch64 Linux" or
"configure a self-hosted ARM runner" — both are unnecessary work for this phase.

**Phase to address:** Phase 1's CI setup task — write the matrix directly against the six free
labels, do not add an emulation layer.

## Code Examples

### Real-to-complex FFT setup (register stage)

```rust
// Source: docs.rs/realfft, adapted to this project's scalar-planner requirement.
// realfft wraps rustfft; force the scalar path by constructing rustfft's
// FftPlannerScalar and handing it to realfft's planner construction, OR,
// if realfft's public API does not expose planner injection, use rustfft's
// FftPlannerScalar directly for the R2C transform via the manual packing
// realfft documents internally. Verify realfft's planner-injection API at
// implementation time; this is the one integration point flagged [ASSUMED]
// pending a docs.rs check against the exact 3.5.0 API surface.
use realfft::RealFftPlanner;

let mut real_planner = RealFftPlanner::<f32>::new();
let r2c = real_planner.plan_fft_forward(working_resolution);
let mut indata = r2c.make_input_vec();   // length = working_resolution
let mut spectrum = r2c.make_output_vec(); // length = working_resolution / 2 + 1
r2c.process(&mut indata, &mut spectrum).unwrap();
```

### Connected-component labelling (classify stage)

```rust
// Source: docs.rs/imageproc, region_labelling module
use imageproc::region_labelling::{connected_components, Connectivity};
use image::GrayImage;

fn label_residual(thresholded: &GrayImage) -> image::ImageBuffer<image::Luma<u32>, Vec<u32>> {
    // background pixel = Luma([0]); foreground (changed) pixels get 0 for
    // background and an incrementing label per connected region otherwise.
    connected_components(thresholded, Connectivity::Eight, image::Luma([0u8]))
}
```

### Antialiasing-aware pixel classification (CORE-05)

```javascript
// Source: github.com/mapbox/pixelmatch, index.js, antialiased() and
// hasManySiblings() — quoted verbatim this session, for adaptation to Rust.
// NOTE: this is pixelmatch's OWN heuristic, not a literal implementation of
// Yee 2004. Yee's paper (per pdiff's own metric.html) handles antialiasing
// implicitly via a Laplacian-pyramid spatial-frequency + CSF-weighted
// comparison, with no explicit AA-pixel-detection rule. pixelmatch's rule
// below is a separate, later invention commonly (and loosely) described as
// "Yee-inspired."
function antialiased(img, x1, y1, width, height, a32, b32) {
    // ... samples the 3x3 neighborhood, tracks the min and max brightness
    // delta among the 8 neighbors, and returns false immediately if more
    // than 2 neighbors have zero delta (a solid-color region, not an edge).
    // If a genuine min/max delta pair exists, it checks whether the pixel
    // at that extreme position has more than 2 identical-valued siblings in
    // ITS OWN 3x3 neighborhood, in BOTH the baseline and candidate images.
    // Only if both endpoints of the local brightness ramp look like part of
    // a smooth gradient (many matching siblings) in both images is the
    // pixel classified as antialiasing, not a structural change.
}
```

**What this rule misses:** thin, single-pixel-wide features (a hairline or a small dot) can fail
the `hasManySiblings` (`> 2` matching neighbors) test even when they are genuinely just
antialiased, because a thin feature's neighborhood has too few pixels sharing its exact value.
It also cannot distinguish "coincidentally looks like an AA ramp" from "is actually one" — a
real, small structural change that happens to produce a smooth 3x3 gradient in both images will
be classified as antialiasing and suppressed. `[CITED: github.com/mapbox/pixelmatch, verbatim
source read this session]`

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Assume GitHub Actions has no free ARM64 runner for public repos | `ubuntu-24.04-arm`, `windows-11-arm` are free, standard, unlimited on public repos; `macos-14`/`macos-15`/`macos-latest` are ARM64 by default | GA Aug 2025 (Linux), private-repo parity Jan 2026 (Windows) | The DET-01/DET-02 CI matrix needs no emulation layer |
| Assume `macos-13` (Intel) is the standard macOS x86-64 label | `macos-13` retires Dec 2025; `macos-15-intel` is the current, and last-ever, x86-64 macOS label, sunsetting entirely by Aug 2027 | Changelog Sept 2025 | If macOS x86-64 parity matters past 2027, the plan needs a documented sunset note now |
| `f64::mul_add`'s behaviour was historically platform/library-dependent | Current stable Rust docs state it is "guaranteed... guaranteed not to change," though the software-fallback implementation has had real historical bugs | Ongoing (const since 1.94.0 per this session's docs fetch) | Treat the guarantee as the specification, and the historical bug report as the reason to still test it, not as a reason to avoid it |

**Deprecated/outdated:** the mental model "the compiler might silently fuse my arithmetic" for
plain stable-Rust code — verified this session to not apply outside an explicit, unstable,
nightly-only intrinsic.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The Hann window formula `w(n) = 0.5 - 0.5*cos(2*pi*n/(N-1))` is the right window for this project's phase-correlation step | Pitfall 4, Q2 | Wrong window shape reduces registration accuracy on translated content but does not break determinism, since the mitigation (fixed table, `libm`-computed) applies to any raised-cosine-family window |
| A2 | `realfft`'s planner API allows injecting `rustfft`'s scalar (non-SIMD) path in version 3.5.0 | Code Examples | If `realfft` always delegates internally to the auto-dispatching planner with no scalar override, the register stage must call `rustfft::FftPlannerScalar` directly instead of going through `realfft`'s convenience wrapper — verify at implementation time, before committing to `realfft`'s API in a plan |
| A3 | `image::DynamicImage`'s filter operations (e.g. `.blur()`) assume premultiplied alpha while the base decoded buffer is straight alpha | Anti-Patterns to Avoid | Sourced from a community/deepwiki synthesis of the `image` crate's source, not independently re-read from the exact source file this session; if wrong, the recommended "convert before filtering" step is unnecessary work, not a correctness bug, so the risk of acting on this assumption is low |
| A4 | The phase-correlation peak-confidence threshold for CORE-06/CORE-07's refusal decision is a tunable value, not a fixed constant derivable from theory | Pattern 3 | If a plan hardcodes a specific threshold number without a calibration fixture, the refusal behaviour may be too strict (rejecting valid near-identical pairs) or too loose (registering pairs that should be refused) |
| A5 | GitHub's free-tier runner specifications and label availability, as fetched this session, remain stable through this phase's execution | Q7, Environment Availability | GitHub has changed this landscape twice in the past 18 months (Linux ARM64 GA in 2025, Windows ARM64 private-repo parity in Jan 2026); re-verify the exact labels against `docs.github.com/en/actions/reference/runners/github-hosted-runners` immediately before writing the CI workflow file, not from this document alone |

## Open Questions

1. **What confidence metric and threshold decide CORE-06/CORE-07's refusal?**
   - What we know: the phase-correlation peak's sharpness relative to the correlation surface's
     noise floor is the standard signal (Kuglin & Hines; Reddy & Chatterji).
   - What's unclear: no specific numeric threshold was verified this session; this needs a
     calibration pass against a small corpus of pairs a human judges as "should register" and
     "should refuse."
   - Recommendation: treat threshold calibration as its own task in the plan, with a fixture-
     backed test, not a hardcoded constant chosen by inspection.

2. **Does `realfft` 3.5.0 expose scalar-planner injection, or must the register stage bypass it
   for the R2C transform?**
   - What we know: `rustfft::FftPlannerScalar` exists and forces non-SIMD dispatch.
   - What's unclear: whether `realfft`'s convenience API (`RealFftPlanner`) lets a caller supply
     a specific `rustfft` planner instance, or always constructs its own internally.
   - Recommendation: check `docs.rs/realfft/3.5.0` at implementation time before writing the
     plan's register-stage task in detail; if injection is not exposed, fall back to `rustfft`'s
     own real-to-complex helpers directly, still via the scalar planner.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo`/`rustc` (local dev) | All of Phase 1 | Yes | rustc 1.97.1, cargo 1.97.1 (`[VERIFIED: rustc --version, this session]`) | — |
| GitHub-hosted runner: `ubuntu-24.04` (Linux x86-64) | DET-01, DET-02, DET-03 | Yes, free, unlimited, public repos | — | — |
| GitHub-hosted runner: `ubuntu-24.04-arm` (Linux aarch64) | DET-02, DET-03 | Yes, free, unlimited, public repos (GA Aug 2025) | — | — |
| GitHub-hosted runner: `macos-14`/`macos-15`/`macos-latest` (macOS aarch64) | DET-01, DET-02, DET-03 | Yes, free, unlimited, public repos | — | — |
| GitHub-hosted runner: `macos-15-intel` (macOS x86-64) | DET-01 (if macOS x86-64 parity is desired) | Yes, free, unlimited, public repos, but this is the last x86-64 macOS label GitHub will offer (sunsets Aug 2027) | — | Document the sunset date in the CI workflow's comments so a future maintainer is not surprised |
| GitHub-hosted runner: `windows-2022`/`windows-latest` (Windows x86-64) | DET-01, DET-02, DET-03 | Yes, free, unlimited, public repos | — | — |
| GitHub-hosted runner: `windows-11-arm` (Windows aarch64) | DET-02, DET-03 | Yes, free, unlimited, public repos (extended Jan 2026) | — | — |

`[VERIFIED: docs.github.com/en/actions/reference/runners/github-hosted-runners — direct fetch
this session of GitHub's own runner-images reference table]`

**Missing dependencies with no fallback:** none. All six `(OS, architecture)` labels this phase's
determinism proof needs exist today as free, standard, public-repository GitHub-hosted runners.

**Missing dependencies with fallback:** none.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in) + `insta` 1.48.0 for snapshot/golden tests + `criterion` 0.8.2 for benchmarks |
| Config file | none yet — this phase creates `Cargo.toml`, `crates/*/Cargo.toml`, and `tests/golden/` |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo test --workspace --all-features && cargo bench --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CORE-01 | Named change kind, never a bare count | unit | `cargo test -p chrys-core classify::tests::names_change_kinds -- --exact` | ❌ Wave 0 |
| CORE-02 | Translated region offset reported in pixels | unit | `cargo test -p chrys-core register::tests::reports_translation_offset -- --exact` | ❌ Wave 0 |
| CORE-03 | Colour difference value + both colours | unit | `cargo test -p chrys-core classify::tests::reports_colour_delta -- --exact` | ❌ Wave 0 |
| CORE-04 | Regions grouped with bounding boxes | unit | `cargo test -p chrys-core classify::tests::labels_bounding_boxes -- --exact` | ❌ Wave 0 |
| CORE-05 | AA-only diffs suppressed | unit | `cargo test -p chrys-core classify::tests::suppresses_antialiasing -- --exact` | ❌ Wave 0 |
| CORE-06/07 | Refuse an unregisterable pair, and say why | integration | `cargo test -p chrys-core register::tests::refuses_dissimilar_pair -- --exact` | ❌ Wave 0 |
| SRC-01 | PNG/JPEG/WebP/TIFF pair decodes and compares | integration | `cargo test -p chrys-source-raster -- --exact` | ❌ Wave 0 |
| DET-01/02/03 | Cross-OS/cross-arch identical SHA-256 of raw RGBA8 | CI-only (six-runner matrix) | `cargo run -p chrys-cli -- compare tests/golden/pair-01/a.png tests/golden/pair-01/b.png --hash-only` compared across runners | ❌ Wave 0 (CI workflow file) |
| DET-06 | No transcendental in the comparison path outside `libm`/`palette` | static + unit | `cargo test -p chrys-core determinism::tests::no_std_transcendentals` (a lint or grep-based check) | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test --workspace`
- **Per wave merge:** `cargo test --workspace --all-features`
- **Phase gate:** the six-runner GitHub Actions matrix green (DET-03) before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `Cargo.toml` workspace root with `resolver = "3"` and `[workspace.lints]` — nothing exists yet
- [ ] `crates/chrys-core`, `crates/chrys-source`, `crates/chrys-source-raster`, `crates/chrys-cli` — none exist yet
- [ ] `tests/golden/` fixture directory with at least one text-free and one colour-gradient PNG pair
- [ ] `.github/workflows/determinism.yml` — the six-runner matrix job
- [ ] Framework install: `cargo add insta criterion --dev` at the workspace level once crates exist

## Security Domain

`security_enforcement` is on (ASVS level 1, block on high) per `.planning/config.json`. This
phase is a local CLI/library with no network surface, no authentication, and no session state,
so most ASVS categories do not apply. State that plainly rather than forcing a fit.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture | Yes (narrowly) | The "no per-format special case" trait boundary is itself a secure-design control: it bounds where untrusted file-format parsing code can live |
| V2 Authentication | No | No accounts, no network service (project's own hard constraint) |
| V3 Session Management | No | No sessions |
| V4 Access Control | No | Local file access only, governed by the OS, not the application |
| V5 Input Validation | Yes | `image::Limits` (`max_image_width`, `max_image_height`, `max_alloc`) on every decode call — this is CVE-2023-29408's exact mitigation, and CLI-04 (Phase 3) formalizes it, but the decode call site is built in Phase 1 |
| V6 Cryptography | Yes (narrowly) | SHA-256 via `sha2` for the golden hash is a correctness check, not a security boundary (per `research/ARCHITECTURE.md`'s own framing) — no key management, no secrets, so most of V6 does not apply beyond "use a vetted hash implementation, not a hand-rolled one" |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Decompression-bomb / malformed-dimension image (CVE-2023-29408-class) | Denial of Service | `image::Limits` configured on every `ImageReader`/decoder call, from the first decode call this phase writes — treat local input as untrusted per `research/PITFALLS.md` Pitfall 7 |
| Crafted file causing unbounded memory before any size check | Denial of Service | Read declared dimensions and call `Limits::check_dimensions` before allocating pixel buffers |

## Sources

### Primary (HIGH confidence)

- `cargo search` against the live crates.io registry, this session, for every crate version cited above
- `gsd_run query package-legitimacy check` against crates.io metadata, this session
- Direct `crates.io/api/v1/crates/imageproc` query, this session (bypassing a stale automated-tool result)
- [doc.rust-lang.org/std/primitive.f64.html](https://doc.rust-lang.org/std/primitive.f64.html) — `sqrt`, `mul_add`, `sin`, `cos`, `exp`, `ln`, `cbrt`, `powf`, `atan2` precision documentation, fetched verbatim this session
- [doc.rust-lang.org/nightly/core/intrinsics/fn.fmuladdf64.html](https://doc.rust-lang.org/nightly/core/intrinsics/fn.fmuladdf64.html) — contraction is unstable-intrinsic-only, not a stable-Rust default
- [docs.github.com/en/actions/reference/runners/github-hosted-runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) — current free/unlimited runner label table, fetched this session
- [github.blog/changelog/2025-08-07-arm64-hosted-runners-for-public-repositories-are-now-generally-available](https://github.blog/changelog/2025-08-07-arm64-hosted-runners-for-public-repositories-are-now-generally-available/)
- [github.blog/changelog/2026-01-29-arm64-standard-runners-are-now-available-in-private-repositories](https://github.blog/changelog/2026-01-29-arm64-standard-runners-are-now-available-in-private-repositories/)
- [github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down](https://github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/)
- [docs.rs/rustfft](https://docs.rs/rustfft/latest/rustfft/) — `FftPlanner` auto-dispatch and `FftPlannerScalar`, fetched this session
- [docs.rs/palette](https://docs.rs/palette/latest/palette/) — Lab, `color_difference` module, `libm` feature, fetched this session
- [github.com/mapbox/pixelmatch/blob/master/index.js](https://raw.githubusercontent.com/mapbox/pixelmatch/master/index.js) — `antialiased()` and `hasManySiblings()` quoted verbatim this session
- [pdiff.sourceforge.net/metric.html](https://pdiff.sourceforge.net/metric.html) — Yee 2004's own metric description, confirming it has no explicit AA-pixel rule
- [doc.rust-lang.org/cargo/reference/workspaces.html](https://doc.rust-lang.org/cargo/reference/workspaces.html) — `[workspace.lints]`, `resolver` field syntax, fetched this session
- [doc.rust-lang.org/cargo/reference/features.html](https://doc.rust-lang.org/cargo/reference/features.html) — optional-dependency `dep:` syntax, fetched this session

### Secondary (MEDIUM confidence)

- [shnatsel.github.io/implementing-fma-finding-bugs-in-std](https://shnatsel.github.io/implementing-fma-finding-bugs-in-std/) — historical FMA subnormal-rounding bugs in Rust std and musl
- [scikit-image phase_cross_correlation docs](https://scikit-image.org/docs/stable/api/skimage.registration.html) — subpixel upsampled-DFT algorithm, the named reference target for Phase 1's subpixel refinement
- Wikipedia "Phase correlation" and general apodization-window literature (Hann/Hamming windowing for FFT edge effects) — corroborated across multiple independent search results, MEDIUM confidence per this session's own sourcing standard
- x264 motion-estimation literature (median predictor, diamond/hexagon search, early termination) — corroborated across multiple independent sources
- `research/SUMMARY.md`, `research/ARCHITECTURE.md`, `research/STACK.md`, `research/PITFALLS.md` — this project's own prior research, HIGH-MEDIUM confidence per their own metadata, incorporated directly where this session did not independently re-verify a claim

### Tertiary (LOW confidence)

- `image` crate premultiplied-alpha-in-filters claim (Anti-Patterns, A3) — community/deepwiki synthesis, not independently re-read from the exact `image` source file this session
- Exact FFT wall-clock cost for a 4000x3000 pair — no benchmark run this session against this project's own working-resolution choice; only single-dimension `rustfft` microbenchmarks were found (~7.7 microseconds for a 1024-point 1D FFT on Apple M1). Treat any specific millisecond figure for the full 2D register stage as unverified until `criterion` benchmarks exist.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every crate version and license checked against the live crates.io registry this session, not recalled
- Architecture: MEDIUM-HIGH — algorithms are decades-old and well-cited (inherited from `research/ARCHITECTURE.md`); the specific `rustfft`/`palette`/`imageproc` integration choices are this session's own verified additions
- Determinism (Q6): HIGH on the `std` precision-documentation findings (directly quoted from doc.rust-lang.org this session); MEDIUM on the historical FMA-bug detail (single secondary source, not cross-checked against a second independent report)
- CI matrix (Q7): HIGH — directly fetched from GitHub's own current reference documentation and changelog posts this session
- Pitfalls: MEDIUM-HIGH — most inherited from `research/PITFALLS.md`'s own MEDIUM rating, with three (FMA-reality, FftPlanner dispatch, GitHub runner matrix) independently upgraded to HIGH this session via direct verification

**Research date:** 2026-09-06
**Valid until:** 30 days for crate versions and CI runner labels (both move quickly — GitHub
changed this landscape twice in 18 months); 90 days for the algorithm and pitfalls material
(stable, decades-old references)
