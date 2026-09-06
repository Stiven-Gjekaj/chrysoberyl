# Walking Skeleton, Chrysoberyl

**Phase:** 1, Raster engine and determinism proof
**Generated:** 2026-09-06

This project is a Rust library and a command line tool. It has no web
framework, no database, no browser and no hosted environment. The standard
skeleton shape does not map onto it, so the equivalent shape is written out
here. The tracer is plan 01-01. The rest of the phase expands from it.

## Capability proven end to end

A person clones the repository, runs one command against two PNG files on disk,
and reads a verdict that names the kind of change, with a non-zero exit code
when the pair differs.

## The skeleton, item by item

The web-application template asks for scaffolding, routing, one real database
read and write, one real interface interaction, and a dev deployment. The
equivalents for this project are these five.

| Template item | This project's equivalent | Plan |
|---|---|---|
| Project scaffold | A Cargo workspace with four crates, a pinned toolchain and a committed lock file | 01-01 |
| One real read | Two real PNG files read from disk, not a buffer built in memory | 01-01 |
| One real write and read cycle | Decode, register, classify and produce a verdict, on one real pair | 01-01 |
| One real interaction | `chrys compare BASE CANDIDATE` prints a verdict and sets an exit code | 01-01 |
| Dev deployment | A workflow that runs the same command on six runner labels and compares a digest of the result | 01-03 |

The last row is this project's deployment equivalent. The skeleton is not done
until a person can clone the repository, run one command, and see a verdict, and
until the mechanism exists that will prove that verdict is the same on every
machine.

## Architectural decisions

These are the decisions later phases build on. They come from PROJECT.md, from
`.planning/research/ARCHITECTURE.md` and from
`.planning/phases/01-raster-engine-and-determinism-proof/01-RESEARCH.md`.

| Decision | Choice | Rationale |
|---|---|---|
| Language and runtime | Rust, native, no runtime | One codebase reaches Linux, macOS and Windows. PROJECT.md constraint. |
| Toolchain | Pinned in `rust-toolchain.toml` | The standard library documents transcendental precision as varying by Rust version, so the compiler version is a determinism input, not a convenience. |
| Dependency graph | Exact version pins and a committed `Cargo.lock` | A transitive bump can move a rounded value. Lock discipline is a determinism requirement here, not only a supply-chain one. |
| Workspace layout | `crates/chrys-source`, `crates/chrys-source-raster`, `crates/chrys-core`, `crates/chrys-cli` | The engine cannot import a format crate because it does not depend on one. The "no per-thing special case" rule becomes a compile-time fact. |
| Format boundary | The `Source` trait, returning `Vec<Frame>` of RGBA8 | Format knowledge is erased once. Every stage after it sees one shape, whether the input is an image, a page, a video frame or a mesh view. |
| Canonical pixel format | RGBA8, straight alpha, row major, no row padding | One documented convention, so no stage has to ask which one it received. At least one image crate filter assumes the other convention, so the choice is written on the type. |
| Rasterization | CPU only, always | PROJECT.md invariant 1. A GPU-produced pixel makes a baseline machine-dependent, which removes the product. |
| Registration | Phase correlation for the global shift, coarse-to-fine block matching for the local one | PROJECT.md invariant 2. The near-identical assumption turns registration from research into engineering. Feature matching, homography and optical flow are out of scope by decision. |
| Transform library | `rustfft`, always through its scalar planner | The default planner detects AVX, SSE and NEON at run time and switches algorithm, so two machines can take different arithmetic through one call. |
| Transcendental policy | The `libm` crate, and `palette` with its `libm` feature | The standard library documents its sine, cosine, cube root, power, exponential, logarithm and arc tangent as varying by platform and by Rust version. Its square root and its fused multiply-add are documented as guaranteed not to change. |
| Contraction policy | No build flag, an audit instead | Stable Rust never fuses an ordinary multiply and add. There is no default to switch off. The real risks are an explicit fused call and a dependency that opts into a fast-maths flag for its own code. |
| Local stage arithmetic | Integer only | An exact integer sum has one answer, so no lane width and no summation order can move a block score. |
| Colour difference | Lab through `palette`, Euclidean distance in Phase 1 | The richer 2000 formula adds an arc tangent, a sine, a cosine and a power on top of the cube root Lab already needs. The Euclidean form needs only a square root, which is the one operation the standard library guarantees. |
| Labelling | `imageproc`'s connected components, eight-way, sorted by this project | The crate already implements the standard algorithm. The sort is this project's, so a version bump cannot reorder a verdict. |
| Verdict shape | `Verdict` is `Identical`, `Changed { regions }` or `Refused { reason }` | PROJECT.md invariant 3. One engine, one meaning of changed. A refusal is a variant, so no caller can read it as a pass. |
| Change kinds | `Moved`, `Added`, `Removed`, `Recoloured`, `Resized`, decided by an ordered list of named rules | A reviewable decision list, not a black box. A CI gate a person cannot audit is a gate a person cannot trust. |
| Digest contract | SHA-256 over raw RGBA8 bytes and canonical verdict text, never a re-encoded file | A re-encoder is a second variable. The digest input is assembled in one function so the contract has one home. |
| Exit codes | 0 identical, 1 changed, 2 refused, 3 error | A CI gate needs a distinct code for "I cannot answer" and "the answer is a change". |
| Proof mechanism | Six GitHub-hosted runner labels, compared by digest, plus a local two-architecture check | All six labels are free standard runners today, so no emulation and no self-hosted machine is needed. |
| Repository state | Private, no remote, until the author asks | PROJECT.md key decision. The six-runner proof waits on that; the local proof does not. |

## Out of scope for the skeleton

These are deliberately absent from Phase 1. Later phases add them without
changing a decision above.

- Any window, any GUI, any GPU code. `chrys-window` and `chrys-gpu` are Phase 6.
- The `chrys-store` baseline store trait and the committed hash manifest. Phase 3.
- The declarative TOML rule file and the score stage. Phase 3.
- The report artifact written to a file. Phase 3, CLI-02.
- Animation, frame sequences and the hint channel. Phase 2.
- SVG, PDF, video and mesh adapters. Phases 4, 5, 7 and 8.
- The richer colour-difference formula. It is the named upgrade once the six-runner matrix is green.
- Subpixel warping of the candidate. Phase 1 refines the peak but warps by whole pixels.

## The one thing later phases must not break

Every later phase reuses the digest mechanism from plan 01-03 and the guard
pattern from plan 01-08. The project research is explicit that every threat to
the portable-baseline claim recurs at each new format boundary, so determinism is
proven per format and never once. When Phase 4 adds SVG, it adds its own
cross-system golden hash test in the same shape. When Phase 8 adds the mesh rig,
it does the same. The template for all of them is written in Phase 1.

## Subsequent slices

| Phase | The next capability a person gains |
|---|---|
| 2 | Compare an animation or a numbered frame sequence, with no change to the engine |
| 3 | Commit a baseline, scope tolerance in a TOML rule file, and gate CI on the verdict |
| 4 | Compare an SVG pair, rasterized on the CPU with a repository-pinned font set |
| 5 | Compare a PDF page pair, behind a feature that is off by default |
| 6 | Review a pair in a native window, with pan, zoom and scrubbing |
| 7 | Compare a video pair, paired by presentation timestamp |
| 8 | Compare a 3D mesh pair through a fixed eight-view rig |
</content>
