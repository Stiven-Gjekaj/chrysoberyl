---
phase: 2
phase_name: "Source trait and a second format"
project: "Chrysoberyl"
generated: "2026-09-07"
counts:
  decisions: 6
  lessons: 5
  patterns: 5
  surprises: 4
missing_artifacts:
  - "VERIFICATION.md"
---

# Phase 2 Learnings: Source trait and a second format

## Decisions

### The sequence is the only unit of comparison
`compare_sequence` carries the one pipeline. `compare` is a thin wrapper that calls
it with a slice of one.

**Rationale:** phases 5 to 8 add PDF pages, video frames and a mesh rig's eight
views, and every one is a sequence. Two entry points would force each later format
to choose between them, which is the per-format branch invariant 3 forbids. A
delegation test and a static guard hold it.
**Source:** 02-01-SUMMARY.md, 02-02-SUMMARY.md

### The hint channel crops, it does not constrain the search
`Frame::crop_to_region` lives in `chrys-source`. The command line crops both frames
to a named region before calling the unmodified engine.

**Rationale:** the alternative constrains the search window inside
`register::phase_correlate`, which touches the engine and would break success
criterion 3. Cropping delivers the same behaviour from outside.
**Source:** 02-05-SUMMARY.md, 02-RESEARCH.md question 6

### `RegionHint` derives nothing
`toml` and `serde` are dependencies of `chrys-source-raster`, never of
`chrys-source`. The sidecar's parse structs are private to the raster adapter.

**Rationale:** `chrys-source` is a dependency of `chrys-core`, so a derive on
`RegionHint` would put `serde` into the engine's graph for a parser the engine never
runs. A manifest guard reads the dependency table line by line and was drilled red
by planting `serde = "1"`.
**Source:** 02-05-SUMMARY.md

### The animated WebP container sets "do not blend"
The ANMF flags byte selects full-canvas overwrite rather than alpha blending.

**Rationale:** the first assembled container failed its own round-trip on frame 0.
`image-webp`'s alpha-blend compositor is imprecise even at full opacity. The
do-not-blend path is exact.
**Source:** 02-03-SUMMARY.md

### A sequence exit code is the worst verdict across every index
Refused outranks changed, which outranks identical.

**Rationale:** it keeps the exit-code contract identical for one pair and for a
thousand frames, so a caller branches the same way either way.
**Source:** 02-01-SUMMARY.md

### The frame header prints the frame's own index, not the loop position
**Rationale:** the label stays correct if a later adapter's frames are not stored in
index order.
**Source:** 02-02-SUMMARY.md

---

## Lessons

### An architecture claim is worth having only when it is falsifiable
Success criterion 3 said a new input family arrives with no engine change. That
became `git rev-parse HEAD:crates/chrys-core` compared against a recorded id, checked
after every commit in three consecutive waves. The id never moved.

**Context:** a tree object id covers content and file mode across the whole subtree,
so it catches an edit that a filename check would miss. It was recorded after the
last permitted engine change, which is the only point where the anchor is correct.
**Source:** 02-02-SUMMARY.md, 02-03-SUMMARY.md, 02-04-SUMMARY.md, 02-05-SUMMARY.md

### A generated expectation file agrees with a regression
The four phase-1 digests were typed into `expected-digest.sha256` from the plan text,
not produced by running the binary.

**Context:** a file generated from current output matches whatever the code does
today, including a fault. The file carries its own header saying that any edit to it
is a behaviour change a commit message must name.
**Source:** 02-02-SUMMARY.md

### A fixture generator must earn trust before its bytes are used
The hand-assembled animated WebP failed its own round-trip gate on the first attempt.

**Context:** no Rust crate encodes an animated WebP, so the generator is new
unreviewed code producing data other tests then trust. The gate runs in memory
against the assembled buffer before `fs::write` is ever called, so a bad container is
never written at all.
**Source:** 02-03-SUMMARY.md

### A guard checking four line numbers stops working when the file grows
The `agree` job read four fixed lines. Five fixture pairs produce twenty.

**Context:** rewritten to `cmp -s` over the whole file, with `diff` run only on a
runner that disagrees. Proven red by corrupting one character.
**Source:** 02-04-SUMMARY.md

### A validation contract can carry a command that runs nothing
The contract first held `cargo test -p chrys-core --lib sequence::tests -- --exact`,
lifted from the research and never run. `--exact` matches a whole test name, not a
module prefix, so it reports `0 passed; 54 filtered out` and exits zero.

**Context:** the planner caught it. A command written into a contract is not evidence
until somebody has watched it run.
**Source:** 02-VALIDATION.md, the planner's deviation 4

---

## Patterns

### Record a tree object id as the anchor a later wave checks itself against
**When to use:** any claim of the form "this change did not touch that subtree". The
anchor is taken after the last permitted change, and every later commit compares
against it.
**Source:** 02-02-SUMMARY.md

### A guard whose silence is read as evidence gets a drill
The drill confirms two things, not one: that the check goes red, and that its message
names what was planted.

**When to use:** every guard in this project. Phase 1 found a guard that could not go
red and had been reporting green for a whole phase.
**Source:** 02-04-SUMMARY.md, 02-05-SUMMARY.md

### An adapter never opens its own decoder
A new `Source` adapter depends on `chrys-source` and on the raster crate's guarded
`decode_guarded` and `normalize_to_rgba8`, never on `image::ImageReader`.

**When to use:** every new input family. It keeps the untrusted-input boundary in one
file rather than one per format.
**Source:** 02-01-SUMMARY.md

### A new format's feature flag stops at the adapter
`chrys-source-animation` enables `image`'s `gif` feature and takes `png` as a
dev-only fixture dependency. Neither reaches `chrys-core`.

**When to use:** every format phase from here. Verify with `cargo tree`, not with the
declared list; phase 1 lost this invariant to a re-export.
**Source:** 02-03-SUMMARY.md

### A fixture proves the thing it is built for and nothing else
The hint fixture uses a colour change outside the named region rather than a shift,
so no translation inside the block matcher's own search window could hide the result.

**When to use:** whenever a fixture's own structure could produce a pass for a reason
other than the one under test.
**Source:** 02-05-SUMMARY.md

---

## Surprises

### The `Source` trait needed no change at all
`load` already returned a vector of frames and `Frame` already carried `index` and
`hints`. Phase 1 built the shape that sequences, animation and later video need.

**Impact:** the phase named after the trait changed the trait not at all. The work was
in adapters and in the engine's entry point.
**Source:** 02-RESEARCH.md question 5

### The engine boundary held across three waves without a single violation
Waves 3, 4 and 5 all reported the same tree id, and `git diff` listed no engine file.

**Impact:** three new input families arrived without the engine learning they exist.
That is the first direct evidence the architecture delivers what it claims rather than
being asserted to.
**Source:** 02-03-SUMMARY.md through 02-05-SUMMARY.md

### An alpha blend at full opacity is not lossless
`image-webp` composites an ANMF frame imprecisely even when alpha is 255, so the
round-trip failed on a frame that should have been an exact copy.

**Impact:** the container sets the do-not-blend bit. Lossy animated WebP determinism
stays unproven and this phase uses lossless fixtures only.
**Source:** 02-03-SUMMARY.md

### The six-runner matrix absorbed four new input families in one push
Twenty digest lines across PNG, a numbered sequence, GIF, APNG and animated WebP, all
identical on three operating systems and two architectures, green on the first run.

**Impact:** the portability claim now covers five fixture families rather than one.
**Source:** GitHub Actions run 34071436487
