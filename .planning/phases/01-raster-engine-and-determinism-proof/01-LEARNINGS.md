---
phase: 1
phase_name: "Raster engine and determinism proof"
project: "Chrysoberyl"
generated: "2026-09-07"
counts:
  decisions: 8
  lessons: 7
  patterns: 6
  surprises: 6
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
---

# Phase 1 Learnings: Raster engine and determinism proof

## Decisions

### Own the connected-component labelling rather than import it

`imageproc::region_labelling::connected_components` was replaced with a two-pass
union-find labeller this repository owns, and `imageproc` left the manifest with
about 90 transitive crates.

**Rationale:** `imageproc` re-exports the `image` crate as `imageproc::image`, so
`classify/label.rs` imported a format crate into the format-blind engine and
defeated the boundary the architecture rests on. The stronger reason is that the
label assignment order is that crate's implementation detail, and the verdict
depends on it, so a patch release elsewhere could change every verdict.
**Source:** 01-08-SUMMARY.md

### The FFT planner is chosen, never auto-detected

The register stage constructs `FftPlannerScalar` only. `FftPlanner::new` appears
nowhere outside a comment, and a static guard asserts it.

**Rationale:** the default planner detects AVX, SSE and NEON at run time and picks
a different algorithm per machine. A verdict built on it differs between two
machines while every local test stays green.
**Source:** 01-04-SUMMARY.md, 01-08-SUMMARY.md

### No platform transcendental on any comparison path

`sin`, `cos`, `cbrt`, `powf`, `exp`, `ln` and `atan2` go through `libm`, and colour
maths goes through `palette` with its `libm` feature.

**Rationale:** Rust's own precision documentation says those functions vary by
platform, by Rust version, and even inside one execution. `sqrt` and `mul_add` are
documented as stable and are allowed.
**Source:** 01-RESEARCH.md question 6, 01-04-SUMMARY.md

### `realfft` is not adopted

The register stage calls `rustfft::FftPlannerScalar` directly.

**Rationale:** 01-RESEARCH.md left open whether `realfft` 3.5.0 exposes a scalar
planner injection point. Reading 3.5.0's source at its pinned commit showed every
FFT construction path goes through the auto-dispatching planner, with no injection
point. The question was answered from source rather than assumed.
**Source:** 01-04-SUMMARY.md

### The refusal threshold is measured, not chosen

`REFUSAL_THRESHOLD` is 2313.88, the midpoint between the lowest registerable ratio
measured at 3041.24 and the highest unregisterable at 1586.52, over a committed
16-pair corpus.

**Rationale:** a threshold picked until tests pass is a threshold that proves
nothing. The corpus is committed so the measurement can be repeated.
**Source:** 01-05-SUMMARY.md

### The summed-area table accumulates in `u64`

`IntegralImage` holds `Vec<u64>` and its row accumulator is `u64`.

**Rationale:** floating-point summation is not associative, so a table built in a
different reduction order gives different values on a different machine. Integer
accumulation removes the problem rather than managing it.
**Source:** 01-06-SUMMARY.md

### The CI workflow ships live, and its trigger design does not change

`.github/workflows/determinism.yml` triggers on `push` and `pull_request`, holds
zero occurrences of `pull_request_target`, sets `permissions: contents: read`, and
pins every action to a 40-character commit hash.

**Rationale:** the plan was written while the repository had no remote and said the
matrix could not run. That condition was satisfied mid-phase. The remote question
changed; the threat mitigations did not.
**Source:** 01-03-SUMMARY.md

### The shipped colour difference is CIE76, not CIEDE2000

`classify/colour.rs` uses `palette::color_difference::EuclideanDistance` over Lab.

**Rationale:** CIEDE2000 adds a sine, a cosine and a power function on top of the
cube root every Lab conversion already needs, which widens the determinism surface.
CIEDE2000 is recorded as a future upgrade once the matrix is green a second time on
this module.
**Source:** 01-07-SUMMARY.md

---

## Lessons

### A guard that cannot go red reports green forever

The cross-architecture script was expected to catch a switch to the auto-dispatching
FFT planner. Its first drill run showed it does not: `aarch64-apple-darwin` and
`x86_64-apple-darwin` under Rosetta agree anyway, because Rosetta does not expose
real AVX or CPU-generation divergence. A fifth, hardware-independent static guard
now covers it.

**Context:** found by the drill's own first run, not by review. Both attempts are
recorded verbatim in the summary rather than only the working one.
**Source:** 01-08-SUMMARY.md

### Three quality gates can all check the wrong axis

`imageproc` was approved by the phase research, named by the plan, and passed by the
plan checker. All three assessed it for supply-chain trust, which it passes on 12.7
million downloads and a named source repository. None assessed it against the
architectural invariant, which it fails.

**Context:** a dependency needs two separate questions asked of it. Is it
trustworthy, and does it belong in this crate.
**Source:** 01-07-SUMMARY.md, 01-08-SUMMARY.md

### A defect can be correct at every sampled point

The subpixel twiddle table indexed by the raw unsigned FFT bin instead of the signed
frequency. That is exactly right at every integer sample, which is all the
whole-pixel tests exercise, and an order-of-magnitude error at a fractional shift.

**Context:** caught by this plan's own tests and confirmed against an independent
plain Python DFT rather than by reasoning about the algebra.
**Source:** 01-04-SUMMARY.md

### A safety check can fire on the safest possible input

`assess_peak`'s zero-floor case would have refused bit-identical pairs. An identical
pair drives the correlation floor to exact zero while the peak stays finite, so the
ratio was undefined in the direction that reads as "cannot register".

**Context:** the refusal gate's failure direction that matters is the one where a
working pair starts being refused, because the tool then looks careful while it has
stopped working.
**Source:** 01-05-SUMMARY.md

### A flat region gives a correlation search nothing to hold

A large uniformly coloured block can alias against an identically flat background and
score a false perfect match. Bounded by keeping the pyramid's largest reachable
offset (24 px) below `BLOCK_SIDE` (32 px).

**Context:** the same class of problem appeared twice, first whole-frame in plan 5
and then per-block in plan 6.
**Source:** 01-05-SUMMARY.md, 01-06-SUMMARY.md

### An invariant survives better as a compile-time fact than as a rule

`chrys-core` builds its test frames in memory rather than depending on
`chrys-source-raster`, even as a dev-dependency, so the engine's dependency graph
stays exactly what it declares.

**Context:** stated in plan 1 and breached in plan 7 through a re-export, which shows
the discipline needs a guard rather than an intention.
**Source:** 01-01-SUMMARY.md, 01-08-SUMMARY.md

### A tool's own verdict on a dependency can be stale

The automated package-legitimacy check returned a suspicious verdict for `imageproc`
because of a registry-lookup failure inside that tool. A direct crates.io API query
in the same session returned clean metadata.

**Context:** the disposition rested on the direct evidence, and the discrepancy was
recorded rather than hidden. `empfindung` and `delta_e` were probed the same way,
scored suspicious, and rejected in favour of `palette`.
**Source:** 01-RESEARCH.md, 01-07-SUMMARY.md

---

## Patterns

### Prove a guard by planting its defect

`scripts/determinism-drill.sh` plants a real defect against each guard inside a
disposable git worktree it creates and removes itself, confirms the run goes red, and
confirms the failure message names what was planted.

**When to use:** any guard whose green result will later be read as evidence. The
drill is re-runnable against a live tree because it never touches it.
**Source:** 01-08-SUMMARY.md

### A source-level guard is defence in depth for a hardware-dependent one

The transcendental guard and the FFT-planner guard both exist because a check that
depends on hardware behaviour can be blind on the hardware you own.

**When to use:** whenever a property is checked by observing runtime behaviour. Add a
static check of the same property that does not depend on the machine.
**Source:** 01-08-SUMMARY.md

### Hold test helpers to the production determinism standard

Two `std` sin and cos calls first appeared in a `subpixel.rs` test helper. The grep
gate covers the whole module, not only its production paths.

**When to use:** any module under a determinism or dependency ban. A helper that
violates the rule teaches the next reader that the rule is optional.
**Source:** 01-04-SUMMARY.md

### Grep-gate the accumulator type, not just the API

`integral.rs` is checked for zero occurrences of `f32` and `f64`, the same way the
register stage is checked for its planner and its trigonometric calls.

**When to use:** where the numeric type itself carries the determinism property.
**Source:** 01-06-SUMMARY.md

### A fixture for a correlation test needs real structure

A frame built from a flat colour plus one changed patch is unsafe input once phase
correlation gates every comparison. A fixture needs several anchor edges.

**When to use:** any test that reaches `compare()` after plan 5.
**Source:** 01-05-SUMMARY.md, 01-06-SUMMARY.md

### One commit per change, with the tests inside it

The digest surface landed as one commit covering the engine change, the CLI flag, the
committed digest file and its guard test. The CI workflow landed as a second.

**When to use:** always in this repository, per AGENTS.md. It is what makes a later
reader able to remove one step and keep the others.
**Source:** 01-03-SUMMARY.md

---

## Surprises

### The FMA risk did not exist in this language

Three research documents carried floating-point contraction as a live threat. Rust's
own precision documentation shows `sqrt` and `mul_add` are guaranteed not to change,
and stable Rust never contracts `a * b + c` on its own; contraction is a nightly
intrinsic.

**Impact:** half of DET-06 guarded a fault Rust does not have. The other half, the
platform transcendentals, is real and is where the work went.
**Source:** 01-RESEARCH.md question 6

### The portability claim held on the first attempt, six times

Six runners across three operating systems and two architectures agreed with the
development machine on four SHA-256 digests, on every run from the first, including
the run where the verdict's meaning changed and all six moved to the new value
together.

**Impact:** the project's core value stopped being a claim during phase 1 rather than
at the end of the roadmap.
**Source:** 01-03-SUMMARY.md, 01-08-SUMMARY.md

### Owning the labelling changed nothing observable

Replacing `imageproc` with a two-pass union-find left the verdict digest at
`33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823` and every one of
the 21 classification tests passing unchanged.

**Impact:** the strongest available evidence that the replacement is faithful, since
no expectation had to be edited to accommodate it.
**Source:** 01-08-SUMMARY.md

### A re-export can carry a banned dependency past a manifest check

`imageproc` re-exports `image` as `imageproc::image`. A crate that must gain no
format dependency of its own acquired one through a name that does not look like one.

**Impact:** the dependency guard now denies a format category as well as a GPU
category, and the drill plants a defect against it.
**Source:** 01-07-SUMMARY.md, 01-08-SUMMARY.md

### Rosetta is not a substitute for a second architecture

Two `x86_64` and `aarch64` builds agreeing under Rosetta on one Mac does not prove
what a real second machine proves, because Rosetta does not expose AVX or
CPU-generation divergence.

**Impact:** the local cross-architecture script stays as supporting evidence and the
six-runner matrix remains the real proof.
**Source:** 01-08-SUMMARY.md

### Cargo scopes a binary path variable to its own package

`CARGO_BIN_EXE_chrys` is only defined for tests inside the binary's own package, so a
CLI exit-code test had to move from `chrys-core` to `chrys-cli`.

**Impact:** a small relocation, recorded so the next CLI behaviour test is written in
the right crate the first time.
**Source:** 01-05-SUMMARY.md
