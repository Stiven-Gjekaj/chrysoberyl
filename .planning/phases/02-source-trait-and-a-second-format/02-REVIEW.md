---
phase: 02-source-trait-and-a-second-format
reviewed: 2026-09-07T12:29:09Z
depth: standard
files_reviewed: 21
files_reviewed_list:
  - crates/chrys-source/src/lib.rs
  - crates/chrys-source/tests/manifest.rs
  - crates/chrys-source-sequence/src/lib.rs
  - crates/chrys-source-sequence/src/sequence.rs
  - crates/chrys-source-sequence/tests/sequence.rs
  - crates/chrys-source-animation/src/lib.rs
  - crates/chrys-source-animation/src/sniff.rs
  - crates/chrys-source-animation/tests/animation.rs
  - crates/chrys-source-animation/examples/make-fixtures.rs
  - crates/chrys-source-raster/src/hints.rs
  - crates/chrys-source-raster/src/lib.rs
  - crates/chrys-source-raster/examples/make-fixtures.rs
  - crates/chrys-core/src/sequence.rs
  - crates/chrys-core/src/lib.rs
  - crates/chrys-core/src/verdict.rs
  - crates/chrys-core/tests/determinism.rs
  - crates/chrys-cli/src/main.rs
  - crates/chrys-cli/tests/digest.rs
  - crates/chrys-cli/tests/region_hint.rs
  - scripts/engine-boundary-drill.sh
  - .github/workflows/determinism.yml
findings:
  critical: 1
  warning: 4
  info: 2
  total: 7
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-09-07T12:29:09Z
**Depth:** standard
**Files Reviewed:** 21
**Status:** issues_found

## Summary

Phase 2's central claim — that a new input family (animation, sequence) enters
the system with zero change to `chrys-core` — holds up under inspection.
`chrys-core/src/sequence.rs`, `lib.rs` and `verdict.rs` name no format, no
GPU crate, and no per-format branch; `--region` cropping lives entirely in
`chrys-cli`, which is genuine orchestration, not a special case smuggled into
the engine. The hand-assembled RIFF/VP8X/ANIM/ANMF WebP container in
`chrys-source-animation/examples/make-fixtures.rs` is byte-accurate against
the WebP container spec (flag bits, 24-bit offset/size fields, chunk padding)
and is gated on its own round-trip decode before a byte reaches disk; no
defect was found in it. The incremental frame/pixel accounting in
`chrys-source-sequence` and `chrys-source-animation` correctly bounds
cumulative memory to a small constant regardless of file/frame count — the
"multiplies the per-file allowance by the file count" failure mode the task
asked me to hunt for does **not** materialize there.

It does materialize, unguarded, in a different place: `chrys-source-raster`'s
`<stem>.hints.toml` sidecar reader has no size limit at all, in direct
contrast to every decode path in this project, all of which are deliberately
bounded against exactly this class of attack (the CVE-2023-29408
decompression-bomb mitigation the raster crate's own doc comment cites). That
is this review's one Critical finding. The remaining findings are latent
correctness/coverage gaps: an automated guard that is supposed to prove half
of invariant 5 (no `serde`, no `toml` in `chrys-core`) never actually checks
for either crate name; a verification script that can report a false "clean"
pass when handed a bad commit range; an unchecked `u32` multiply sitting
right next to a function that deliberately uses checked arithmetic for the
same class of overflow; and an inconsistency in which index `chrys-cli`
prints for a multi-frame sequence depending on which flag was passed.

## Critical Issues

### CR-01: The hints sidecar reader has no size limit, unlike every other decode path in the project

**File:** `crates/chrys-source-raster/src/hints.rs:62-80`
**Issue:** `read_hints_sidecar` calls `std::fs::read_to_string(&sidecar_path)`
with no cap on the sidecar's size, and then hands the full text to
`toml::from_str`, which will happily allocate a `Vec<HintRow>` sized by
however many `[[hint]]` tables the file declares. Every other file this
project reads from disk is explicitly bounded: `DecodeLimits::default()` caps
a raster image at 16384x16384 pixels and 512 MiB of allocation specifically
"so a crafted file cannot force an unbounded allocation before this crate has
read a single pixel" (`crates/chrys-source-raster/src/lib.rs:21-26`), and
`AnimationLimits`/`SequenceLimits` extend the same bound across a whole
animation or directory. `read_hints_sidecar` is called for *every* raster
file and for *every* file in a sequence directory
(`crates/chrys-source-sequence/src/sequence.rs:99`), and is on the exact same
trust boundary as the image bytes the rest of the crate already defends: a
directory an attacker can plant one file into, they can plant a second file
into. A `<stem>.hints.toml` of several gigabytes (or one legitimate-looking
file with millions of `[[hint]]` blocks) is read to completion, unbounded, before
parsing even begins.
**Fix:** Cap the sidecar's size the same way `DecodeLimits` caps an image, and
refuse before reading the full file into memory:
```rust
/// The largest `<stem>.hints.toml` sidecar this crate will read, in bytes.
/// A sidecar is a small, hand-written or generated metadata file; nothing
/// legitimate needs more than this to describe a handful of named regions.
const MAX_HINTS_SIDECAR_BYTES: u64 = 1_048_576; // 1 MiB

pub fn read_hints_sidecar(image_path: &Path) -> Result<Vec<RegionHint>, RasterError> {
    let sidecar_path = sidecar_path_for(image_path);

    let metadata = match std::fs::metadata(&sidecar_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(RasterError::Io {
                path: sidecar_path,
                source: error,
            });
        }
    };
    if metadata.len() > MAX_HINTS_SIDECAR_BYTES {
        return Err(RasterError::HintsTooLarge {
            path: sidecar_path,
            size: metadata.len(),
            limit: MAX_HINTS_SIDECAR_BYTES,
        });
    }

    let text = std::fs::read_to_string(&sidecar_path) /* ... existing NotFound/Io handling ... */;
    // ... rest unchanged
}
```
(add a matching `RasterError::HintsTooLarge { path, size, limit }` variant
next to `TooLarge`).

## Warnings

### WR-01: `engine-boundary-drill.sh` trusts an unchecked `git diff` exit status, so a bad argument can read as a clean pass

**File:** `scripts/engine-boundary-drill.sh:83-87, 131-145`
**Issue:** `boundary_check()` runs `git -C "$1" diff --name-only "$2" --
crates/chrys-core/` and its stdout is captured with no check of `git`'s own
exit status:
```sh
boundary_check() {
    git -C "$1" diff --name-only "$2" -- crates/chrys-core/
}
...
drill_two_output=$(boundary_check "$drill_worktree" "${wave_first_commit}^..${wave_last_commit}")
...
if [ -z "$drill_two_output" ]; then
    drill_ok "clean-range drill: the check stayed silent over plan 02-03's own range"
```
The script's own doc comment (lines 5-10) names exactly this failure mode —
"an empty result is also what a wrong commit range... or a silently failing
command produces" — as the reason the drill exists, but the drill's *own*
invocation of `git diff` for drill two is exactly as unchecked as the thing
it is supposed to be proving safe. `wave_first_commit` and
`wave_last_commit` are caller-supplied arguments (`$1`/`$2` of the script);
if `wave_first_commit` names a commit with no parent, `wave_first_commit^`
fails, `git diff` exits non-zero, stderr carries the error, and stdout — the
only thing captured — is empty. The script reports `drill ok: clean-range
drill: the check stayed silent`, a false pass for a command that never ran.
**Fix:** Capture and check the exit status explicitly rather than only the
stdout text:
```sh
boundary_check() {
    # boundary_check WORKTREE RANGE prints the path of every file the range
    # changed under crates/chrys-core/, or returns non-zero if the diff
    # itself could not be computed. Empty output with a zero exit is the
    # pass condition; non-zero exit is never silently read as a pass.
    git -C "$1" diff --name-only "$2" -- crates/chrys-core/
}

drill_two_output=$(boundary_check "$drill_worktree" "${wave_first_commit}^..${wave_last_commit}")
drill_two_status=$?
if [ "$drill_two_status" -ne 0 ]; then
    drill_failed "clean-range drill: git diff over ${wave_first_commit}^..${wave_last_commit} exited ${drill_two_status}, the check could not run"
elif [ -z "$drill_two_output" ]; then
    drill_ok "clean-range drill: the check stayed silent over plan 02-03's own range"
else
    drill_failed "clean-range drill: expected no output over plan 02-03's own range"
fi
```

### WR-02: The dependency guard's denylist omits `serde` and `toml`, so half of invariant 5 is unproven

**File:** `crates/chrys-core/tests/determinism.rs:104-148`
**Issue:** `DENIED_DEPENDENCY_CRATES` and its test
`no_gpu_or_format_crate_enters_chrys_cores_dependency_graph` are the
project's one automated proof that `chrys-core`'s dependency graph stays
clean. The project's own invariant states plainly: "`chrys-core` declares no
format crate, no GPU crate, no serde and no toml." The list this test
actually checks covers the GPU/graphics half and the format-decoding half
(`wgpu`, `slint`, `image`, `png`, `gif`, …) but contains no entry for
`"serde"` or `"toml"` at all. `chrys-core/Cargo.toml` currently declares
neither directly, so there is no live violation today — but the one
automated guard that is supposed to catch a future transitive `serde` or
`toml` creeping in through `rustfft`, `sha2`, `palette`, `libm`, `thiserror`
or `chrys-source` would stay green even if one did. This is the same class of
silent-pass risk WR-01 flags, in the guard that protects a named project
invariant rather than a CI convenience.
**Fix:**
```rust
const DENIED_DEPENDENCY_CRATES: &[&str] = &[
    // Graphics and GPU.
    "wgpu", "wgpu-core", "wgpu-hal", "slint", "vulkano", "ash", "gfx-hal",
    "glow", "metal", "glutin", "gl", "skia-safe",
    // Raster and other format decoding.
    "image", "imageproc", "image-webp", "png", "gif", "jpeg-decoder",
    "zune-jpeg", "webp", "tiff",
    // Serialization formats: chrys-core declares neither, and the hints
    // sidecar parser (serde + toml) must never enter the engine's own
    // dependency graph (see chrys-source-raster/src/hints.rs's own doc
    // comment for why).
    "serde", "toml",
];
```

### WR-03: `crop_to_region`'s pixel-copy loop repeats the overflow-prone multiply that `pixel_count` deliberately avoids

**File:** `crates/chrys-source/src/lib.rs:79-84` (compare with `pixel_count` at `crates/chrys-source/src/lib.rs:34-36`)
**Issue:** `crop_to_region`'s own doc comment takes care to say "the bounds
test uses checked arithmetic, so a hint... is refused rather than wrapping",
and the bounds test (`hint.x.checked_add(hint.width)...`) lives up to that.
But the actual pixel-copy loop right below it does not:
```rust
for row in hint.y..(hint.y + hint.height) {
    let row_start = ((row * self.width + hint.x) as usize) * 4;
```
`row` and `self.width` are both `u32`, and the multiply happens *before* the
cast to `usize`, so on a build without overflow checks this wraps silently
for a sufficiently large frame instead of refusing — producing a wrong
cropped region rather than a clean error, the opposite of what the function's
own doc comment promises. Contrast this with `pixel_count` two methods above
it, in the same file:
```rust
pub fn pixel_count(&self) -> usize {
    (self.width as usize) * (self.height as usize)
}
```
which casts to `usize` *before* multiplying, avoiding exactly this class of
overflow. `Frame` carries no enforced maximum dimension of its own — today's
`DecodeLimits` (16384x16384) keeps every adapter-produced `Frame` well under
the `u32::sqrt` threshold where this would actually wrap, so this is not
reachable through the current adapters, but it is a real inconsistency
between a function's stated guarantee and what it does, in a public type that
`chrys-core` depends on directly.
**Fix:**
```rust
for row in hint.y..(hint.y + hint.height) {
    let row_start = (row as usize * self.width as usize + hint.x as usize) * 4;
    let row_end = row_start + (hint.width as usize) * 4;
    pixels.extend_from_slice(&self.pixels[row_start..row_end]);
}
```

### WR-04: The multi-frame `frame N` header prints different things depending on `--hash-only`

**File:** `crates/chrys-cli/src/main.rs:110-125` vs `:132-139`
**Issue:** The `--hash-only` multi-frame branch deliberately prints the
frame's own `index` field, with a comment explaining why:
```rust
// The printed index is the frame's own `index` field, the same
// number the engine paired on, not the loop position.
println!("frame {}", base_frame.index);
```
The plain verdict-text multi-frame branch a few lines down prints the loop
position instead, with no such care:
```rust
for (index, verdict) in verdicts.iter().enumerate() {
    println!("frame {index}");
    print!("{verdict}");
}
```
Today these always agree, because every `Source` impl assigns `index` from
`enumerate()` in the same order frames are later zipped in
`compare_sequence`. But that agreement is an invariant of the current three
adapters, not something the `Frame` or `Source` contract enforces — `Frame`'s
own doc comment says only "the frame index inside a sequence", and nothing
stops a future `Source` from yielding a non-contiguous or reordered `index`.
If that ever happens, the two output modes for the same command would print
different frame headers for the same underlying comparison, and no existing
test would catch it (`a_sequence_reports_one_digest_block_per_frame_index`
only covers `--hash-only`).
**Fix:** Use the frame's own index in both places:
```rust
for (position, verdict) in verdicts.iter().enumerate() {
    let frame_index = base_frames
        .get(position)
        .map(|frame| frame.index)
        .unwrap_or(position);
    println!("frame {frame_index}");
    print!("{verdict}");
}
```

## Info

### IN-01: `AnimationSource` silently drops every region hint, so `--region` can never work on an animated file

**File:** `crates/chrys-source-animation/src/lib.rs:286-292`
**Issue:** `collect_frames` always pushes `hints: Vec::new()`; unlike
`chrys-source-raster` and `chrys-source-sequence`, this crate never imports or
calls `read_hints_sidecar`. That means `chrys-cli --region` against a GIF,
APNG or animated WebP will always fail with "has no region named ...; it
names: none" (`crates/chrys-cli/src/main.rs:163-192`), for every frame, on
every animated file, regardless of whether a `<stem>.hints.toml` sidecar
exists next to it. This is plausibly an intentional phase-2 scope line — the
animation crate's own doc comment never mentions hints — but nothing says so
explicitly, and no test asserts this limitation on purpose (every
`--region` test in `chrys-cli/tests/region_hint.rs` uses static PNGs).
**Fix:** Either note the limitation explicitly in `AnimationSource`'s own doc
comment ("this adapter attaches no region hint to any frame; `--region`
against an animated file always refuses"), or add a test asserting the
current refusal is expected, so a future change to wire hints through does
not silently change CLI behaviour unnoticed.

### IN-02: The animation/sequence limit-parity test checks hardcoded numbers, not the other crate's own values

**File:** `crates/chrys-source-animation/src/lib.rs:311-316`
**Issue:** `decode_limits_match_the_raster_crates_own_numbers` compares
against `chrys_source_raster::DecodeLimits::default()` directly, but its
sibling test,
```rust
fn frame_and_pixel_limits_match_the_sequence_crates_own_numbers() {
    let limits = AnimationLimits::default();
    assert_eq!(limits.max_frames, 512);
    assert_eq!(limits.max_total_pixels, 134_217_728);
}
```
checks against hardcoded literals rather than
`chrys_source_sequence::SequenceLimits::default()`. If `chrys-source-sequence`
ever changes its own defaults, this test stays green while the two crates'
limits silently drift apart — the exact drift the test's name claims to
guard against. (No cross-crate dependency currently exists to make a direct
comparison possible without adding one, so this may be an accepted
trade-off; noting it for visibility.)
**Fix:** If a `chrys-source-animation` dev-dependency on
`chrys-source-sequence` is acceptable, compare directly as the raster test
does; otherwise, leave a comment on the constant explaining why a literal is
used instead.

---

_Reviewed: 2026-09-07T12:29:09Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
