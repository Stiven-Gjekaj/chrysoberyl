---
phase: 01-raster-engine-and-determinism-proof
reviewed: 2026-09-07T09:25:16Z
depth: standard
files_reviewed: 39
files_reviewed_list:
  - crates/chrys-source/src/lib.rs
  - crates/chrys-source-raster/src/lib.rs
  - crates/chrys-source-raster/src/decode.rs
  - crates/chrys-source-raster/src/normalize.rs
  - crates/chrys-source-raster/examples/make-fixtures.rs
  - crates/chrys-source-raster/tests/formats.rs
  - crates/chrys-source-raster/Cargo.toml
  - crates/chrys-core/src/lib.rs
  - crates/chrys-core/src/hash.rs
  - crates/chrys-core/src/residual.rs
  - crates/chrys-core/src/verdict.rs
  - crates/chrys-core/src/register/mod.rs
  - crates/chrys-core/src/register/window.rs
  - crates/chrys-core/src/register/luma.rs
  - crates/chrys-core/src/register/phase_correlation.rs
  - crates/chrys-core/src/register/subpixel.rs
  - crates/chrys-core/src/register/confidence.rs
  - crates/chrys-core/src/register/warp.rs
  - crates/chrys-core/src/register/integral.rs
  - crates/chrys-core/src/register/block_match.rs
  - crates/chrys-core/src/classify/mod.rs
  - crates/chrys-core/src/classify/label.rs
  - crates/chrys-core/src/classify/antialias.rs
  - crates/chrys-core/src/classify/colour.rs
  - crates/chrys-core/src/classify/kind.rs
  - crates/chrys-core/tests/determinism.rs
  - crates/chrys-core/tests/refusal.rs
  - crates/chrys-core/tests/register.rs
  - crates/chrys-core/tests/block_match.rs
  - crates/chrys-core/tests/classify.rs
  - crates/chrys-core/Cargo.toml
  - crates/chrys-cli/src/main.rs
  - crates/chrys-cli/tests/digest.rs
  - crates/chrys-cli/tests/refusal.rs
  - Cargo.toml
  - rust-toolchain.toml
  - scripts/determinism-drill.sh
  - scripts/cross-arch-hash.sh
  - .github/workflows/determinism.yml
  - crates/chrys-core/src/sequence.rs (read for call-chain tracing only; not in the declared scope, not counted or scored below)
findings:
  critical: 1
  warning: 5
  info: 2
  total: 8
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-09-07T09:25:16Z
**Depth:** standard
**Files Reviewed:** 39
**Status:** issues_found

## Summary

This phase builds the raster decode adapter, the phase-correlation/block-match
registration stage, the classification stage, and the determinism-proof test
suite. The determinism engineering itself is careful and largely sound: every
transcendental on the comparison path is routed through `libm`, the FFT
planner is pinned to the non-dispatching scalar variant, ordering-sensitive
folds use `BTreeMap` instead of `HashMap`, and `tests/determinism.rs` encodes
several of these rules as executable guards backed by
`scripts/determinism-drill.sh`'s fault-injection drills. `cargo check` and
`cargo clippy --all-targets` are both clean with no warnings.

The one blocking defect is a correctness gap, not a determinism gap: the
comparison pipeline (`chrys-core/src/sequence.rs`'s `compare_pair`, wired
through `block_match` → `difference_image` → `label_regions`) never inspects
the alpha channel anywhere. A `Frame` is explicitly documented as "RGBA8,
straight alpha" everywhere in this codebase, but a pair that differs only in
alpha is reported `Verdict::Identical`. Everything else found is a
robustness or test-quality issue in the determinism guard's own coverage and
diagnostics, not a defect in a shipped verdict.

## Critical Issues

### CR-01: A pair that differs only in the alpha channel is reported `Identical`

**File:** `crates/chrys-core/src/register/warp.rs:69-83`, `crates/chrys-core/src/classify/label.rs:159-173`, `crates/chrys-core/src/sequence.rs:32-85`
**Issue:**
`compare_pair` (`sequence.rs`) has no raw-byte-equality shortcut; it always
runs registration, then `block_match`, then `suppress_antialiasing`, then
`label_regions`, and reports `Verdict::Identical` exactly when
`label_regions` returns no region (`sequence.rs:66-68`). But the residual
buffer `label_regions` reads is produced by `difference_image`
(`warp.rs:69-83`), which loops `for channel in 0..3` — red, green, blue only
— and unconditionally forces the residual's alpha byte to `u8::MAX`
(`warp.rs:80`). `label_regions`'s own foreground test then reads only
`residual.samples[idx]`, `[idx+1]`, `[idx+2]` (`label.rs:166-171`), i.e. R, G,
B. Alpha is never differenced and never contributes to "is this pixel
foreground."

Consequently, two frames whose RGB bytes are pixel-for-pixel identical but
whose alpha channel differs (a transparency edit, a mask change, a
compositing change — all realistic edits for the PNG/WebP/TIFF formats this
crate decodes) produce zero labelled regions and `Verdict::Identical`, even
though `Frame::pixels` differs and the frame's own `rgba8_digest` (used by
`--hash-only`) would differ between `decode-base` and `decode-candidate`.
That makes the digest report self-contradictory in this case: different
decode digests alongside a `verdict` digest of `identical`. This is exactly
the class of silent, wrong "no change" verdict CORE-06/CORE-07's own
refuse-rather-than-guess principle is built to prevent, except here nothing
even signals uncertainty — the tool actively asserts sameness for content
that changed.

No doc comment anywhere in `register/` or `classify/` states that alpha is
deliberately out of scope for phase 1; every doc comment instead describes
the pixel format as "RGBA8, straight alpha" without qualification
(`chrys-source/src/lib.rs:13`, `warp.rs`'s own module doc, etc.), so this
reads as an oversight rather than a documented, deliberate limitation.

**Fix:** Either (a) extend the residual and foreground test to include the
alpha channel (difference alpha in `difference_image`, and let
`label_regions`'s magnitude test consider it, e.g.
`r.max(g).max(b).max(a)`), or (b), if alpha is deliberately out of scope for
phase 1, make that an explicit, tested decision: document it prominently on
`Frame`, `difference_image` and `compare_pair`, and add a guard (e.g. in
`compare_pair` or in a `chrys-core` test) that fails loudly, or reports a
distinct verdict, when two frames' RGB channels are identical but their raw
bytes are not — rather than silently reporting `Identical`.

```rust
// warp.rs, option (a): include alpha in the residual
pub fn difference_image(base: &[u8], warped: &[u8]) -> Vec<u8> {
    debug_assert_eq!(base.len(), warped.len(), "...");
    let mut out = Vec::with_capacity(base.len());
    for chunk_start in (0..base.len()).step_by(4) {
        for channel in 0..4 {
            out.push(base[chunk_start + channel].abs_diff(warped[chunk_start + channel]));
        }
    }
    out
}
```

## Warnings

### WR-01: The forbidden-transcendental and `mul_add` guards miss path-qualified calls

**File:** `crates/chrys-core/tests/determinism.rs:337-346`
**Issue:** `find_method_calls` searches stripped source for the literal
substring `.{method}(` (e.g. `.sin(`). This only matches method-call syntax
(`angle.sin()`). A fully qualified call such as `f64::sin(angle)` or
`<f64>::cos(angle)` contains no `.sin(`/`.cos(` substring at all and slips
through both `the_comparison_path_calls_no_forbidden_transcendental` and
`a_fused_multiply_add_call_needs_an_allow_list_entry_and_a_subnormal_test`
undetected. No call site in the current tree uses this form, so nothing is
wrong today, but this is precisely the kind of blind spot this project's own
`DENIED_DEPENDENCY_CRATES`/`AUTO_DISPATCHING_FFT_PLANNER_CONSTRUCTOR` guards
were careful to avoid (those match a bare substring, with no assumed `.`
prefix). The transcendental guard is the one guard in this file still
narrower than it needs to be.
**Fix:** Reuse `find_substring_occurrences` (already used for the pipeline
and FFT-planner guards) with both a `.method(` and a `::method(` pattern, or
switch to a regex/word-boundary match on `method(` that also rejects a
preceding identifier character other than `.`/`:`.

### WR-02: A multi-line block comment shifts the line numbers this file reports

**File:** `crates/chrys-core/tests/determinism.rs:196-256`
**Issue:** `strip_comments_and_strings` consumes every character of a
`/* ... */` comment, including any newlines inside it, without emitting a
replacement for them. Only the newline that immediately follows the closing
`*/` survives (because it lies outside the comment). Every subsequent line
in the file is therefore shifted upward in the stripped string relative to
the original source by one line per newline the comment consumed. Since
`find_method_calls`/`find_substring_occurrences` derive their reported line
number by counting lines in the *stripped* string
(`stripped.lines().enumerate()`), a forbidden call reported after a
multi-line block comment in the same file will carry the wrong line number
in the test failure message — undermining the very "so a person can read
this list top to bottom" clarity this test suite otherwise cultivates.
**Fix:** When consuming a block comment, push one `\n` to `out` for each
newline consumed inside it, so line numbers downstream stay aligned with the
original source.

### WR-03: `RasterError::TooLarge`'s single `limit` field can misreport an asymmetric configuration

**File:** `crates/chrys-source-raster/src/decode.rs:108-115`, `crates/chrys-source-raster/src/lib.rs:104-114`
**Issue:** `too_large` always reports `limits.max_width.max(limits.max_height)`
as "the limit," and the error's `Display` impl prints it as a single,
undifferentiated "the limit of {limit} pixels per side." With the default,
symmetric `DecodeLimits` (16384×16384) this never misleads. But
`RasterSource::with_limits` accepts asymmetric limits, and in that case the
message can name a limit that was never actually violated on the offending
axis (e.g. `max_width: 5000, max_height: 3000`, an image `4000×4000`: the
message says "exceeds the limit of 5000 pixels per side," but the width
(4000) is within the stated 5000 limit — only height exceeded its own,
different, unstated limit).
**Fix:** Carry both `max_width` and `max_height` in `TooLarge` (or the
whole `DecodeLimits`), and name the axis or axes that were actually
exceeded in the message.

### WR-04: Public API safety invariants are enforced only by `debug_assert!`, which release builds strip

**File:** `crates/chrys-core/src/register/warp.rs:69-74`, `crates/chrys-core/src/register/integral.rs:64-68`
**Issue:** `difference_image` (re-exported at `chrys_core::register::difference_image`)
requires its two buffers to be equal length, and `IntegralImage::window_sum`
(re-exported at `chrys_core::register::IntegralImage`) requires its rectangle
to lie inside the table, but both contracts are checked only by
`debug_assert!`/`debug_assert_eq!`, which compile out in release builds.
Both functions are part of `chrys-core`'s public surface (`register` is a
`pub mod`), not private helpers reachable only from the two call sites the
comments describe. A release build handed a mismatched pair by any future or
external caller still cannot corrupt memory (Rust bounds-checks slice
indexing unconditionally), but it panics with a generic
"index out of bounds" message instead of the graceful `Result`-based refusal
this crate practices everywhere else its own contracts can be violated
(`CompareError::ShapeMismatch`, `RegionOutOfBounds`, etc.).
**Fix:** Either keep these two functions private (`pub(crate)`) so the
"only two call sites already guarantee this" comment is enforced by the
compiler, or validate the contract with a real `assert!`/`Result` at the
public boundary.

### WR-05: `frame_background`'s modal-colour histogram scans every distinct RGBA value with a `BTreeMap`

**File:** `crates/chrys-core/src/classify/kind.rs:92-103`
**Issue:** Not a determinism bug (the `BTreeMap` choice is correct and
deliberate for tie-break stability, and this is explicitly out of scope as a
performance concern per the review brief), but worth flagging as a
robustness note: `frame_background` is recomputed once per call to
`classify_kind`, and `classify_kind` is called once per labelled region
(`sequence.rs:70-82`). For a frame with many labelled regions, this repeats
the whole-frame modal-colour scan once per region rather than once per
compare. This is a latent cost concern rather than a correctness one, noted
here only because a future contributor optimizing this function should not
reach for a `HashMap` to speed it up without re-deriving the same
determinism argument the doc comment already states.

## Info

### IN-01: `search_block`'s pre-loop `best_offset` initializer is dead

**File:** `crates/chrys-core/src/register/block_match.rs:321-322`
**Issue:** `let mut best_offset = candidates[0];` is always overwritten on
the loop's first iteration (`best_key` starts `None`, so `better` is `true`
unconditionally at `index == 0`, setting `best_offset` to the same value
again). Harmless, but it reads as if it does something the loop doesn't
already guarantee.
**Fix:** Drop the initializer and use `let mut best_offset: Option<(i32, i32)> = None;`, unwrapping after the loop, or leave a short comment noting the first iteration always overwrites it.

### IN-02: A doc comment overstates the equivalence of two tie-break rules

**File:** `crates/chrys-core/src/classify/kind.rs:169-174`
**Issue:** The doc comment for `majority_block_offset` says a remaining tie
"favours the lexicographically smaller offset, since `BTreeMap` iterates its
keys in a fixed order," and separately claims this "match[es] `block_match`'s
own tie-break convention." `block_match::search_block` actually breaks ties
by `(squared magnitude, dy, dx)` ascending (`block_match.rs:312-318`), not by
raw `(dx, dy)` tuple order. Both rules happen to agree on the *primary*
key (smaller magnitude wins) but can disagree on the secondary tie-break
between two offsets of equal magnitude (e.g. `(-1, 0)` vs. `(0, -1)`: tuple
order picks `(-1, 0)`, while `block_match`'s `dy`-before-`dx` order also
picks `(-1, 0)` here, but the two rules are not the same rule in general).
Functionally harmless (this is a rare secondary tie between rectangles at
different offsets of otherwise-equal vote count), but the comment claims
more than the code delivers.
**Fix:** Either align the two tie-break orderings, or reword the comment to
say the two rules agree on the primary criterion only.

---

_Reviewed: 2026-09-07T09:25:16Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
