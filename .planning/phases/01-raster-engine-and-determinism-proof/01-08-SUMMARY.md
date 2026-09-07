---
phase: 01-raster-engine-and-determinism-proof
plan: 08
subsystem: determinism
tags: [rust, connected-components, union-find, rustfft, cargo-tree, ci, determinism]

# Dependency graph
requires:
  - phase: 01-raster-engine-and-determinism-proof
    provides: "chrys_core::register::window: WORKING_RESOLUTION, cached_hann_table, plan_scalar_fft (01-03)"
  - phase: 01-raster-engine-and-determinism-proof
    provides: "chrys_core::classify::label: label_regions, LabelledRegion, RESIDUAL_THRESHOLD (01-07)"
provides:
  - "crates/chrys-core/src/classify/label.rs: a two-pass, union-find connected-component labeller this repository owns, replacing imageproc::region_labelling::connected_components"
  - "crates/chrys-core/tests/determinism.rs: five source-level guards (transcendental call, fused multiply-add, palette feature declaration, chrys-core dependency graph, auto-dispatching FFT planner)"
  - "scripts/cross-arch-hash.sh: the local, re-runnable two-architecture digest agreement check for DET-02"
  - "scripts/determinism-drill.sh: the re-runnable drill that plants a defect against each guard and confirms it goes red, naming what was planted"
  - "a guards job in .github/workflows/determinism.yml, running the determinism test target on every push"
affects: []

# Actuals (#2632)
actuals:
  tokens: 12098
  tasks: 4
  commits: 5
plan_head_before: 93171627e32d314f032f1b2f351483ab7e1ab004

# Tech tracking
tech-stack:
  removed: ["imageproc 0.27.0 (region_labelling::connected_components) and its transitive graph (rav1e, ravif, nalgebra, glam, rayon, rand, and about ninety other crates)"]
  patterns:
    - "chrys-core's own connected-component labeller assigns provisional labels in raster scan order (row by row, top to bottom, left to right), smallest label first on a tie through union-find, and the caller-facing Vec<LabelledRegion> is re-sorted by bounding box position before it is returned, so the raw scan-assigned label is never a caller-visible identity"
    - "A determinism guard that reads its own source tree strips comments, block comments (nested), and every string/byte-string/char literal before searching, so a mention of a forbidden method in prose or inside a string cannot trip the guard and a real call cannot hide inside one"
    - "A local, single-machine cross-architecture check (aarch64 native vs. x86_64 under Rosetta 2) is real evidence for an ISA-level divergence, but is not a reliable proxy for a CPU-generation-level SIMD dispatch divergence (AVX2 vs. no-AVX x86-64), because Rosetta's translated environment does not expose the same CPU feature surface a second, genuinely different x86-64 machine would; a rule stated as a hardware-dependent runtime check needs a source-level guard as its hardware-independent backup"
    - "A guard-failure drill mutates only a temporary git worktree created from HEAD, torn down by a trap on every exit path, so the drill is safe to re-run against a live working tree"

key-files:
  created:
    - crates/chrys-core/tests/determinism.rs
    - scripts/cross-arch-hash.sh
    - scripts/determinism-drill.sh
  modified:
    - crates/chrys-core/src/classify/label.rs
    - crates/chrys-core/Cargo.toml
    - Cargo.toml
    - Cargo.lock
    - .github/workflows/determinism.yml

key-decisions:
  - "Task 0 (orchestrator-added, not in the original plan): replaced imageproc::region_labelling::connected_components with a two-pass, union-find connected-component labeller this repository owns, and removed imageproc from the dependency graph. imageproc re-exported the image crate, so chrys-core reached a format crate transitively, defeating research/ARCHITECTURE.md's compile-time 'no format crate in the engine' rule; imageproc's own default features also pulled in about ninety unused transitive crates (rav1e, ravif, nalgebra, glam, rayon, rand among them), none of which the connected-component call used."
  - "The dependency guard's deny-list (DENIED_DEPENDENCY_CRATES) covers both a GPU/graphics category (wgpu, slint, vulkano, ash, gfx-hal, glow, metal, glutin, gl, skia-safe) and a format-decoding category (image, imageproc, image-webp, png, gif, jpeg-decoder, zune-jpeg, webp, tiff), reading the resolved cargo tree for chrys-core restricted to normal edges, not the direct manifest, so a transitive arrival is caught the same way the imageproc mistake would have been."
  - "The planner drill's first run found that scripts/cross-arch-hash.sh does not reliably detect a switch from FftPlannerScalar to rustfft's auto-dispatching FftPlanner on this machine: aarch64-apple-darwin and an x86_64-apple-darwin binary run under Rosetta 2 produced identical digests even with the auto-dispatching planner in place, confirmed independently by building both variants natively and comparing to the committed verdict digest. This is a finding about the guard, not the drill (Rosetta's translated environment does not expose the same CPU feature surface a second, genuinely different x86-64 machine would), so a fifth, hardware-independent static guard was added to determinism.rs, and the planner drill now treats that guard as its primary check, running cross-arch-hash.sh only as secondary, informational evidence."
  - "The dependency-guard drill (drill three) injects glow, an OpenGL-binding crate already on the deny-list and already present in the local cargo registry cache, so the drill resolves and builds offline with no crates.io network dependency."

patterns-established:
  - "A source-level determinism guard is defence in depth for a runtime/hardware-dependent check, not a replacement for it: the transcendental guard and the FFT-planner guard both exist because a check that depends on the transcendental/dispatch behaviour of whichever machine happens to run it can silently miss a real regression."
  - "A guard-failure drill runs entirely inside a disposable git worktree it creates and removes itself (via mktemp -d and a trap), so the same drill that proves a guard can fail can be re-run against a live, uncommitted working tree without risk."

requirements-completed: [DET-01, DET-02, DET-03, DET-04, DET-06]

coverage:
  - id: D0
    description: "chrys-core owns its own two-pass, union-find connected-component labelling algorithm (Wu, Otoo and Suzuki, 2009) instead of calling imageproc, and no longer reaches the image crate transitively; every existing classify.rs test passes unchanged and the tests/golden/pair-01 verdict digest is unchanged at 33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823."
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/classify.rs (21 tests, all passing unchanged after the algorithm swap)"
        status: pass
      - kind: other
        ref: "cargo run -q -p chrys-cli -- compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png --hash-only | head -2 | diff - tests/golden/pair-01/expected-decode.sha256"
        status: pass
      - kind: other
        ref: "cargo tree -p chrys-core -e normal (no image, no imageproc; only chrys-source, libm, palette, rustfft, sha2, thiserror)"
        status: pass
    human_judgment: false
  - id: D1
    description: "A test fails, naming the file and line, when the comparison path calls a platform transcendental with unspecified, platform-varying precision; sqrt is the one permitted exception, with its guarantee documented."
    requirement: "DET-06"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/determinism.rs#the_comparison_path_calls_no_forbidden_transcendental"
        status: pass
    human_judgment: false
  - id: D2
    description: "A test audits every explicit mul_add call and fails, naming the file and line, with a message describing the allow-list and subnormal-test requirement a call would need."
    requirement: "DET-06"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/determinism.rs#a_fused_multiply_add_call_needs_an_allow_list_entry_and_a_subnormal_test"
        status: pass
    human_judgment: false
  - id: D3
    description: "A test asserts the workspace manifest's palette entry declares default-features = false and the libm feature, and that no .cargo/config.toml enables an extra instruction set or a fast-maths flag."
    requirement: "DET-06"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/determinism.rs#the_colour_crate_declares_no_default_features_and_the_pure_rust_maths_feature"
        status: pass
    human_judgment: false
  - id: D4
    description: "A test reads chrys-core's own resolved dependency tree (normal edges only) and fails, naming the crate and printing the tree, when it names a GPU/graphics crate or a format-decoding crate."
    requirement: "DET-04"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/determinism.rs#no_gpu_or_format_crate_enters_chrys_cores_dependency_graph"
        status: pass
    human_judgment: false
  - id: D5
    description: "A test fails, naming the file and line, when the comparison path constructs rustfft's auto-dispatching FftPlanner instead of FftPlannerScalar. Added after the planner drill's first run showed a local, Rosetta-based digest comparison alone cannot be trusted to catch this."
    requirement: "DET-02"
    verification:
      - kind: integration
        ref: "crates/chrys-core/tests/determinism.rs#the_comparison_path_never_constructs_the_auto_dispatching_fft_planner"
        status: pass
    human_judgment: false
  - id: D6
    description: "scripts/cross-arch-hash.sh builds the chrys binary in release for the host's own architecture and for a second architecture, and exits 0 only when the two four-line digest reports are byte-identical; exits 2, not 0, when the drill is unavailable on the host."
    requirement: "DET-02"
    verification:
      - kind: other
        ref: "sh scripts/cross-arch-hash.sh (exit 0 on this host; aarch64-apple-darwin and x86_64-apple-darwin agree on all four digests, matching the committed verdict digest)"
        status: pass
    human_judgment: false
  - id: D7
    description: "A guards job in .github/workflows/determinism.yml runs the determinism test target on ubuntu-24.04 on every push, alongside (not replacing) the six-runner digest/agree matrix."
    requirement: "DET-03"
    verification:
      - kind: other
        ref: "grep -v '^[[:space:]]*#' .github/workflows/determinism.yml | grep -q -- '--test determinism'"
        status: pass
      - kind: other
        ref: "six-label matrix unchanged: grep -cE '^[[:space:]]*- +(ubuntu-24\\.04|ubuntu-24\\.04-arm|macos-15|macos-15-intel|windows-2022|windows-11-arm)[[:space:]]*$' equals 6"
        status: pass
    human_judgment: false
  - id: D8
    description: "scripts/determinism-drill.sh plants one defect per guard inside a disposable temporary git worktree and confirms each guard goes red and names the planted defect: the transcendental guard names window.rs after a std cos() substitution, the static FFT-planner guard names window.rs after an auto-dispatching-planner substitution, and the dependency guard names glow after it is added to chrys-core's manifest. The real working tree is unmodified afterward."
    verification:
      - kind: other
        ref: "sh scripts/determinism-drill.sh (prints three 'drill ok' lines, exits 0; see this SUMMARY's own transcript of both attempts)"
        status: pass
      - kind: other
        ref: "git status --porcelain (empty both before and after the drill)"
        status: pass
    human_judgment: true
    rationale: "The plan's own <human-check> asks a person to read the drill's full output and confirm, by eye, that each of the three guards went red for the reason claimed and not some other reason; this SUMMARY records the transcript for that review."

duration: 90min
completed: 2026-09-06
status: complete
---

# Phase 1 Plan 8: Five determinism guards, a cross-architecture digest check, and a guard-failure drill Summary

**An orchestrator-added task first replaced `imageproc`'s connected-component labelling with an in-house two-pass union-find implementation and removed `imageproc` (and its ~90-crate transitive graph) from `chrys-core`; the plan itself then added five source-level determinism guards, a local two-architecture digest script, and a self-cleaning drill script that plants a real defect against each guard and confirms it goes red — a run that itself found and closed a real gap (a Rosetta-based digest comparison cannot reliably catch an auto-dispatching FFT planner) before the phase could be called proven.**

## Performance

- **Duration:** 90 min (measured between this plan's final commit, `52bfcae`, and 01-07's final commit, `9317162`)
- **Completed:** 2026-09-06
- **Tasks:** 4 (Task 0, orchestrator-added, plus the plan's own Tasks 1-3)
- **Files modified:** 8 (3 created, 5 modified, including `Cargo.lock`)

## Accomplishments

- **Task 0 (orchestrator addition):** `crates/chrys-core/src/classify/label.rs` now owns a two-pass, union-find connected-component labelling algorithm (Wu, Otoo and Suzuki, 2009) instead of calling `imageproc::region_labelling::connected_components`. `imageproc` re-exported the `image` crate, so `chrys-core` reached a format crate transitively, defeating `research/ARCHITECTURE.md`'s compile-time "no format crate in the engine" rule. Removing `imageproc` from the workspace manifest also dropped roughly ninety unused transitive crates (`rav1e`, `ravif`, `nalgebra`, `glam`, `rayon`, `rand`, and others) that the connected-component call never used. Every existing `classify.rs` test (21 tests) passes unchanged, and the committed `tests/golden/pair-01/` verdict digest is unchanged at `33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823`, confirming the algorithm swap is a pure internal replacement.
- **Task 1:** `crates/chrys-core/tests/determinism.rs` gained three guards: a transcendental-call guard (walks every file under `src/`, strips comments and string/char literals, and fails naming the file and line when it finds a call to a forbidden standard-library transcendental), a fused-multiply-add audit, and a `palette` feature-declaration guard.
- **Task 2:** A fourth guard was added — a dependency guard that reads `chrys-core`'s own resolved `cargo tree` (normal edges only) and fails, naming the crate, when it finds a GPU/graphics crate or a format-decoding crate. `scripts/cross-arch-hash.sh` was written and run for real: `aarch64-apple-darwin` and `x86_64-apple-darwin` (the latter under Rosetta 2) agree on all four digests. A `guards` job was added to `.github/workflows/determinism.yml`, running the determinism test target on every push, without touching the existing six-runner matrix.
- **Task 3:** `scripts/determinism-drill.sh` plants one defect per guard inside a disposable temporary git worktree and confirms each guard goes red and names the planted defect. Its first run found that the planner drill's assumption (a local cross-architecture digest comparison would catch an auto-dispatching FFT planner) does not hold on this machine; the fix — a fifth, hardware-independent static guard — is documented below and the drill now passes 3 of 3.

## Task Commits

0. **Task 0 (orchestrator addition): Own connected-component labelling and drop imageproc** - `0667c65` (code + test: `crates/chrys-core/src/classify/label.rs`, `crates/chrys-core/Cargo.toml`, `Cargo.toml`, `Cargo.lock`)
1. **Task 1: Guard the comparison path against platform maths** - `3ffe4b5` (test: `crates/chrys-core/tests/determinism.rs`)
2. **Task 2: Keep the GPU out, and compare two architectures on one machine** - `e592e20` (test + script + CI: `crates/chrys-core/tests/determinism.rs`, `scripts/cross-arch-hash.sh`, `.github/workflows/determinism.yml`)
3. **Task 3, part 1: Add the re-runnable drill** - `82e664e` (script: `scripts/determinism-drill.sh`)
3. **Task 3, part 2: Add a static guard for the auto-dispatching FFT planner (fix found by the drill's first run)** - `52bfcae` (test + script: `crates/chrys-core/tests/determinism.rs`, `scripts/determinism-drill.sh`)

**Plan metadata:** commit pending (this SUMMARY — orchestrator owns STATE.md/ROADMAP.md writes per the objective given to this executor)

## Files Created/Modified

- `crates/chrys-core/src/classify/label.rs` - two-pass, union-find connected-component labelling this repository owns; documents the label assignment order (raster scan, smallest label first) as this project's own decision
- `crates/chrys-core/tests/determinism.rs` - five guards: transcendental call, fused multiply-add audit, `palette` feature declaration, `chrys-core` dependency graph, auto-dispatching FFT planner
- `scripts/cross-arch-hash.sh` - local two-architecture digest agreement check; exit 0 agree, exit 1 disagree, exit 2 unavailable
- `scripts/determinism-drill.sh` - re-runnable drill; plants one defect per guard inside a disposable git worktree, prints `drill ok` or `DRILL FAILED` per drill
- `crates/chrys-core/Cargo.toml` - `imageproc` removed
- `Cargo.toml` - `imageproc` removed from the workspace dependency table
- `Cargo.lock` - `imageproc` and its transitive graph (~90 crates) removed
- `.github/workflows/determinism.yml` - `guards` job added; six-runner matrix unchanged

## Decisions Made

See `key-decisions` in the frontmatter for the full list with reasons. In short: Task 0 replaced `imageproc` with an owned algorithm because a re-exported format crate defeated the engine's own "no format crate" compile-time rule; the dependency guard's deny-list covers both GPU/graphics and format-decoding crates by category, not just the crates seen so far; and the planner drill's first run found a real gap (a Rosetta-based local cross-architecture check cannot reliably catch an auto-dispatching FFT planner), fixed by adding a fifth, hardware-independent static guard rather than treating the false negative as acceptable.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] The planner drill's cross-architecture check does not reliably catch the exact defect it was written to catch**

- **Found during:** Task 3, first run of `scripts/determinism-drill.sh`
- **Issue:** The plan's own action text for drill two specified: replace the scalar FFT planner with the auto-dispatching one, then assert `scripts/cross-arch-hash.sh` exits 1 and names a differing digest. Running the drill as specified found the two architectures (`aarch64-apple-darwin` native, `x86_64-apple-darwin` under Rosetta 2) produced byte-identical digests even with the auto-dispatching planner in place. A follow-up experiment (building the mutated planner for `aarch64-apple-darwin` alone and comparing to the committed verdict digest) confirmed this is not a drill bug: `FftPlanner` and `FftPlannerScalar` produce the identical digest on this machine's native architecture too, because Rosetta 2's translated x86-64 environment does not expose the same CPU feature surface a second, genuinely different x86-64 machine would, and this project's working resolution happens to dispatch to a numerically identical code path on this hardware regardless. 01-RESEARCH.md's own Pitfall 3 names the real risk as divergence between two different x86-64 *CPU generations* (AVX2 vs. no-AVX), which a single Apple Silicon Mac, even with Rosetta, cannot reproduce. This left DET-02's protection against this specific pitfall resting entirely on a check that can silently pass when the mutation it is meant to catch is actually present.
- **Fix:** Added a fifth test to `crates/chrys-core/tests/determinism.rs`, `the_comparison_path_never_constructs_the_auto_dispatching_fft_planner`, a static source guard that searches for the literal construction of `rustfft`'s auto-dispatching `FftPlanner` and fails, naming the file and line, independent of any machine's CPU features. Updated the planner drill in `scripts/determinism-drill.sh` to treat this new guard as its primary, decisive check; `cross-arch-hash.sh` still runs as part of the drill and its result is still printed, as evidence, but no longer decides the drill's outcome.
- **Files modified:** `crates/chrys-core/tests/determinism.rs`, `scripts/determinism-drill.sh`
- **Verification:** Re-ran the drill; the static guard test goes red and names `window.rs` when the planted mutation is present, and the drill now reports `drill ok` for the planner drill. Both attempts' transcripts are recorded below.
- **Committed in:** `52bfcae`

**2. [Rule 2 - Missing critical functionality / architectural, orchestrator-directed] Own connected-component labelling and drop `imageproc`**

- **Found during:** Assigned as Task 0 by the orchestrator, before any of the plan's own tasks began.
- **Issue:** `imageproc`, added in plan 01-07 for `region_labelling::connected_components`, re-exports the `image` crate as `imageproc::image`, so `chrys-core` reached a format crate transitively even though it never named `image` directly in its own manifest. `research/ARCHITECTURE.md` states the engine crate may not import a format crate, specifically so "no per-format special case" is a compile-time fact rather than a rule a person has to remember; the transitive import defeated that mechanism. Separately, `label.rs`'s own prior comment recorded that a version bump inside `imageproc` could reassign labels in a different internal order, meaning the verdict's region ordering depended on a third-party crate's own internal choice, not on this project's documented decision.
- **Fix:** Replaced the `imageproc` call with a two-pass, union-find connected-component labeller this repository owns (same algorithm family `imageproc` used, per `research/ARCHITECTURE.md`'s own citation of Wu, Otoo and Suzuki, 2009), keeping eight-way connectivity. The label assignment order (raster scan, smallest label first) is now documented in `label.rs`'s own module doc comment as this project's own decision. Removed `imageproc` from both the workspace manifest and `chrys-core`'s own manifest, which also dropped its unused ~90-crate transitive graph (`rav1e`, `ravif`, `nalgebra`, `glam`, `rayon`, `rand`, and others — none of which the connected-component call ever used).
- **Files modified:** `crates/chrys-core/src/classify/label.rs`, `crates/chrys-core/Cargo.toml`, `Cargo.toml`, `Cargo.lock`
- **Verification:** All 21 `classify.rs` tests pass unchanged; `cargo test --workspace` reports 132 tests passing (unchanged from 01-07's baseline, before this plan's own new `determinism.rs` tests); the `tests/golden/pair-01/` verdict digest is unchanged at `33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823`; `cargo tree -p chrys-core -e normal` shows exactly `chrys-source`, `libm`, `palette`, `rustfft`, `sha2`, `thiserror` and nothing else, matching this plan's own strengthened dependency guard.
- **Committed in:** `0667c65`

---

**Total deviations:** 2 (1 orchestrator-directed architectural fix, Task 0; 1 Rule 2 auto-fix found by the plan's own drill, Task 3). **Impact on plan:** No scope creep beyond what the orchestrator explicitly assigned and what the plan's own drill mechanism was designed to surface. Both fixes strengthen the phase's determinism guarantees rather than working around them.

## Guard-failure drill transcript (both attempts)

Per the plan's own instruction: "If a drill does not go red, that is a finding about the guard, not about the drill; fix the guard, re-run, and record both attempts." Both attempts follow, with `cargo`'s own `Compiling ...` noise trimmed for readability; nothing else is edited.

### Attempt 1 (before the fix) — drill exits 1, 2 of 3 drills behaved as expected

```
drill ok: transcendental drill: the guard went red and named window.rs
DRILL FAILED: planner drill: expected the cross-architecture script to exit 1 and name a differing digest (exit=0)
cross-arch-hash: ensuring x86_64-apple-darwin is installed
info: component rust-std for target x86_64-apple-darwin is up to date
cross-arch-hash: building for aarch64-apple-darwin
    Finished `release` profile [optimized] target(s) in 30.61s
cross-arch-hash: building for x86_64-apple-darwin
    Finished `release` profile [optimized] target(s) in 33.19s
cross-arch-hash: running the aarch64-apple-darwin binary
cross-arch-hash: running the x86_64-apple-darwin binary
cross-arch-hash: aarch64-apple-darwin and x86_64-apple-darwin agree on all four digests
decode-base 5a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc
decode-candidate 99bf5b9508b222f45fa5badf610e21cdb464401b26bc30dda345780dc7d2f59f
residual f812da6aa3625e87bba393bb9742f6f24da5ca2a69d10434f364bab3b2fcaea0
verdict 33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823
drill ok: GPU drill: the dependency guard went red and named glow

determinism-drill: 2 of 3 drills behaved as expected
```

(A fused multiply-add-style follow-up check, run manually outside the drill, confirmed this was not an artefact of the drill's own mechanics: a standalone build of the mutated planner for `aarch64-apple-darwin` alone reproduced the same, unchanged, committed verdict digest, `33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823` — the auto-dispatching planner is not diverging from the scalar one on this hardware at all, at this working resolution.)

### Attempt 2 (after adding the static FFT-planner guard) — drill exits 0, 3 of 3 drills behaved as expected

```
drill ok: transcendental drill: the guard went red and named window.rs
determinism-drill: planner drill: also running cross-arch-hash.sh, as evidence only
cross-arch-hash: ensuring x86_64-apple-darwin is installed
info: component rust-std for target x86_64-apple-darwin is up to date
cross-arch-hash: building for aarch64-apple-darwin
    Finished `release` profile [optimized] target(s) in 28.69s
cross-arch-hash: building for x86_64-apple-darwin
    Finished `release` profile [optimized] target(s) in 37.37s
cross-arch-hash: running the aarch64-apple-darwin binary
cross-arch-hash: running the x86_64-apple-darwin binary
cross-arch-hash: aarch64-apple-darwin and x86_64-apple-darwin agree on all four digests
decode-base 5a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc
decode-candidate 99bf5b9508b222f45fa5badf610e21cdb464401b26bc30dda345780dc7d2f59f
residual f812da6aa3625e87bba393bb9742f6f24da5ca2a69d10434f364bab3b2fcaea0
verdict 33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823
determinism-drill: planner drill: cross-arch-hash.sh exited 0 (informational)
drill ok: planner drill: the static FFT-planner guard went red and named window.rs
drill ok: GPU drill: the dependency guard went red and named glow

determinism-drill: 3 of 3 drills behaved as expected
```

The real working tree's `git status --porcelain` was empty before and after every run of the drill, in both attempts. A third, final confirmation run (after all commits) reproduced attempt 2's result exactly: exit 0, three `drill ok` lines.

## Cross-architecture digest evidence (DET-02)

`sh scripts/cross-arch-hash.sh`, run on this development machine (Apple Silicon macOS, Rosetta 2 available), `aarch64-apple-darwin` (native) vs. `x86_64-apple-darwin` (translated):

```
decode-base 5a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc
decode-candidate 99bf5b9508b222f45fa5badf610e21cdb464401b26bc30dda345780dc7d2f59f
residual f812da6aa3625e87bba393bb9742f6f24da5ca2a69d10434f364bab3b2fcaea0
verdict 33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823
```

All four digests match the digests this project has committed to since plan 01-06 (decode) and plan 01-07 (verdict). This is the first real cross-architecture measurement this project has; the six-runner matrix (`.github/workflows/determinism.yml`) remains the broader, multi-OS, multi-CPU-generation proof, open until the repository has a remote, per plan 01-03's own human check.

## Dependency guard deny-list

`DENIED_DEPENDENCY_CRATES` in `crates/chrys-core/tests/determinism.rs`:

- **Graphics and GPU:** `wgpu`, `wgpu-core`, `wgpu-hal`, `slint`, `vulkano`, `ash`, `gfx-hal`, `glow`, `metal`, `glutin`, `gl`, `skia-safe`
- **Raster and other format decoding:** `image`, `imageproc`, `image-webp`, `png`, `gif`, `jpeg-decoder`, `zune-jpeg`, `webp`, `tiff`

Confirmed today: `cargo tree -p chrys-core -e normal` names exactly `chrys-source`, `libm`, `palette`, `rustfft`, `sha2`, `thiserror` (plus their own transitive proc-macro and support crates) — none of the above.

## Issues Encountered

See "Deviations from Plan" above; both issues encountered were resolved within this plan's own scope.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Every determinism claim this phase makes is now held by a guard: `crates/chrys-core/tests/determinism.rs` holds five tests (transcendental call, fused multiply-add audit, `palette` feature declaration, dependency graph, auto-dispatching FFT planner), and every one of them has been watched failing, on purpose, with a message naming the planted defect (`scripts/determinism-drill.sh`, 3 of 3 drills passing).
- `cargo build --workspace`, `cargo test --workspace` (132 base tests + 5 determinism tests = 137, all passing), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --check` all pass on the final commit.
- All 16 calibration pairs in `tests/golden/refuse-01/` still classify correctly (verified via `crates/chrys-core/tests/refusal.rs`, 6 tests, unchanged).
- `chrys-core`'s own dependency graph is now exactly `chrys-source`, `thiserror`, `sha2`, `rustfft`, `libm`, `palette` — matching this plan's own project constraint.
- Open, by design, per plan 01-03's own human check: the six-runner CI matrix (`.github/workflows/determinism.yml`) has not yet run on a real remote, because this repository has none yet. The local guards job, `scripts/cross-arch-hash.sh`, and `scripts/determinism-drill.sh` are the evidence available today; the six-runner matrix remains this phase's final, broader confirmation once a remote exists.
- No blockers.

## Self-Check: PASSED

All files below were verified present on disk, and all five commit hashes verified present in `git log --oneline` on this branch, before this line was written:
- `crates/chrys-core/src/classify/label.rs` (rewritten) — FOUND
- `crates/chrys-core/tests/determinism.rs` — FOUND
- `scripts/cross-arch-hash.sh` (executable) — FOUND
- `scripts/determinism-drill.sh` (executable) — FOUND
- `.github/workflows/determinism.yml` (guards job present) — FOUND
- Commits `0667c65`, `3ffe4b5`, `e592e20`, `82e664e`, `52bfcae` — FOUND in `git log --oneline`

---
*Phase: 01-raster-engine-and-determinism-proof*
*Completed: 2026-09-06*
