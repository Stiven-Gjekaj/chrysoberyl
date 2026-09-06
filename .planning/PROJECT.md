# Chrysoberyl

## What This Is

Chrysoberyl is a structural diff for everything that is not text. It compares
images, animation, video, PDF pages, SVG and 3D meshes, and it reports what
changed instead of which pixels differ. It runs as a library, a command line
tool, and a native window on Linux, macOS and Windows. There is no account and
no hosted service.

The name says what the tool does. A cat's eye chrysoberyl shows one bright line
across the stone. Alexandrite, the same mineral, shows a different colour under
a different light. The same frame shows a different face under a different
decoder.

## Core Value

A baseline is portable. The same pair gives the same verdict on any machine,
any GPU and any driver. Everything else can fail. This cannot.

This is the claim the project says out loud, because a person can check it.
"A better diff" is a claim that every competitor makes and that nobody can
check.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Compare a pair of raster frames and report structural change, not a
      pixel count
- [ ] Give the same verdict on Linux, macOS and Windows for the same input
- [ ] Rasterize vector and document input on the CPU, deterministically
- [ ] Read a declarative TOML rule file that scopes tolerance by kind of
      change and by named region
- [ ] Exit non-zero in CI when a rule fails, and write a report artifact
- [ ] Keep a baseline hash manifest in the repository, with a pluggable store
      behind it and committed goldens as the default backend
- [ ] Show a pair in a native window with pan, zoom and frame scrubbing
- [ ] Accept an optional hint channel so a producer can name its regions
- [ ] Compare animation and numbered frame sequences
- [ ] Compare video, behind a Cargo feature that is off by default
- [ ] Compare PDF pages, behind a Cargo feature that is off by default
- [ ] Compare 3D meshes through a fixed eight-view rig, and name the blind
      spot in the output

### Out of Scope

- General image registration on arbitrary pairs — the engine assumes
  near-identical pairs. That assumption is what makes this engineering and not
  research. Remove it and the guarantee goes with it.
- GPU rasterization of any pixel that enters a comparison — vendors and
  drivers round differently, so the baseline becomes machine-dependent and the
  core value is gone.
- A hosted service, an account, or an upload — the tool runs on the user's
  machine, like stenos and BitSmith.
- A configurable mesh camera rig in v1 — a rig that lives in a file the author
  edits is a baseline that moves when someone edits a config file.
- A scripting language for rules — a rule file that can compute is a rule file
  that can lie. Embedding MiruScriptX would be a better story and the wrong
  tool.
- Text and code diffs — git already does this well.

## Context

The author has not worked on the GPU before. That is deliberate. This project
is chosen to open a door that the existing portfolio does not: language
implementation (MiruScriptX), text parsing and TUI (mandible), industrial
control and native GUI (BetonPlusium), client-side web (BitSmith), offline
audio and ML (OpenBook, stenos), and game engine work (soulbound) are all
covered. The GPU, and deterministic rasterization next to it, are not.

The competitor is not `odiff` or `pixelmatch`. Both are free, fast and good at
pixels, and teams still pay Chromatic. What Chromatic sells is a stable
rendering environment and a review loop, not an algorithm. Chrysoberyl attacks
the environment first, then the diff, then the review.

The architecture rule that governs this repository comes from
`mandible-fork/AGENTS.md`: no per-thing special case, ever. The wide format
scope was chosen with that in mind. Six kinds of input make a per-format branch
impossible to write, so the general engine is the only engine that can exist.

## Constraints

- **Tech stack**: Rust, native on Linux, macOS and Windows — one codebase, three
  systems, no runtime.
- **Tech stack**: wgpu for comparison and display — the only Rust option that
  reaches Metal, D3D12 and Vulkan from one codebase, and Slint renders on it.
- **Tech stack**: Slint for the window, hosting a wgpu canvas through a shared
  device and queue — verified available through `set_rendering_notifier()` and
  `slint::Image::try_from<wgpu::Texture>()`.
- **Compatibility**: rasterization is CPU only — no pixel that enters a
  comparison is produced by a GPU, because that is what makes a baseline
  portable.
- **Dependencies**: the core is MIT and pure Rust — video and PDF sit behind
  Cargo features that are off by default, and the README badge counts the slim
  build.
- **Process**: `AGENTS.md` in this repository governs — ASD-STE100 Simplified
  Technical English, one change per commit, code and its tests in one commit,
  documentation in its own.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Structural diff, built on a perceptual layer | A wall of red is not a review. Pixel and perceptual comparison are solved and free. | — Pending |
| No pixel entering a comparison comes from a GPU | GPU rasterization differs by vendor and driver, which makes a baseline machine-dependent. | — Pending |
| The engine assumes near-identical pairs | Turns image registration from a research problem into block matching and phase correlation. | — Pending |
| One engine, one meaning of "changed" | A format may add a native pass that adds detail. It may never contradict or replace the raster verdict. | — Pending |
| A mesh renders through a fixed eight-view rig | A configurable rig puts the truth in a file the author edits. | — Pending |
| Rules are declarative TOML | A rule file must be reviewable in a pull request. A rule file that can compute can lie. | — Pending |
| Freeze the architecture, stage the formats | MiruScriptX shipped a tree walker first and replaced it in v0.5 once golden tests froze the behaviour. | — Pending |
| Core crate and CLI first, window second | A face that needs a tty cannot be tested in CI or by an agent. mandible §3.6 already paid for this. | — Pending |
| Repository is private until asked otherwise | Nothing goes to a remote until the author asks. | — Pending |

## Open Question

Video decode is specified bit-exact, unlike rasterization, so hardware and
software decode should agree. That is a claim, not a result. Prove it against a
corpus before the video feature depends on it.

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? Move to Out of Scope with the reason.
2. Requirements validated? Move to Validated with the phase reference.
3. New requirements emerged? Add to Active.
4. Decisions to log? Add to Key Decisions.
5. "What This Is" still accurate? Update if it drifted.

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections.
2. Core Value check. Is it still the right priority?
3. Audit Out of Scope. Are the reasons still valid?
4. Update Context with the current state.

---
*Last updated: 2026-09-06 after initialization*
