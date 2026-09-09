---
phase: 04-svg-rasterization
reviewed: 2026-09-09T12:14:39Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - crates/chrys-source-svg/src/lib.rs
  - crates/chrys-source-svg/src/fonts.rs
  - crates/chrys-source-svg/src/sniff.rs
  - crates/chrys-source-svg/tests/svg.rs
  - crates/chrys-source-svg/examples/render-svg.rs
  - crates/chrys-source-svg/Cargo.toml
  - crates/chrys-cli/src/main.rs
  - crates/chrys-cli/tests/svg_boundary_guard.rs
  - crates/chrys-cli/tests/system_font_guard.rs
  - crates/chrys-cli/tests/svg_dispatch.rs
  - crates/chrys-cli/tests/unsafe_guard.rs
  - crates/chrys-cli/Cargo.toml
  - Cargo.toml
  - .github/workflows/determinism.yml
  - crates/chrys-source-svg/fonts/NotoSans-Regular.ttf
  - crates/chrys-source-svg/fonts/OFL.txt
  - tests/golden/formats/svg/base.svg
  - tests/golden/formats/svg/candidate.svg
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 04: Code Review Report

**Reviewed:** 2026-09-09T12:14:39Z
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

This phase adds `chrys-source-svg` (an `resvg`/`tiny-skia`/`fontdb`-based CPU
SVG adapter), wires it into `chrys-cli`'s content-sniffed dispatch, pins one
font and its licence, adds guards proving `resvg`'s `system-fonts` feature
never compiles in and that no SVG crate reaches `chrys-core`'s dependency
graph, and extends the six-runner determinism matrix to a text-bearing SVG
fixture (which came back green on the real CI run per `04-03-SUMMARY.md`).

I read every plan/summary pair, the engine-boundary anchor, `AGENTS.md` and
`PROJECT.md`, then read every file in scope and ran the workspace's own test
suite, clippy, and `cargo tree` against the current tree, not just the diff.
`cargo test --workspace` (43 test binaries), `cargo clippy --workspace
--all-targets -- -D warnings`, and every guard named in the plans
(`unsafe_guard`, `svg_boundary_guard`, `system_font_guard`, `svg_dispatch`)
all pass locally, matching what the summaries report. I additionally built
two throwaway harnesses outside the repository (not committed, not part of
this review's artifacts) to directly exercise `looks_like_svg` and
`bounded_read`/the canvas-allocation check against adversarial input, rather
than trusting the doc comments' claims about their behaviour.

The DET-05 guarantees (no `system-fonts` feature, no SVG crate in
`chrys-core`'s graph, no `load_system_fonts` call anywhere in the workspace
source, straight-not-premultiplied alpha, the pinned-font fallback covering
both the no-attribute and the named-but-unavailable cases) are all real: I
did not find a guard that passes vacuously, and the three planted-defect
drills recorded in the summaries are consistent with what the code actually
does. No file under `crates/chrys-core/` changed (`git rev-parse
HEAD:crates/chrys-core` still reads `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`
against the current tree). The font pin (`NotoSans-Regular.ttf`, 621572
bytes, SHA-256 `478c558e...4f3823`) and `OFL.txt` are both committed and
match the summary's own recorded byte length and hash.

What I found is not in the DET-05/engine-boundary claims themselves, but in
two places at the edge of the adapter's own resource-bound logic and in the
content sniff's documented-vs-actual precision, both demonstrated by
running the code rather than inferred from reading it.

## Warnings

### WR-01: `looks_like_svg` answers true for non-SVG XML that merely contains the substring `<svg` anywhere in its first 1024 bytes, not only at the document root

**File:** `crates/chrys-source-svg/src/sniff.rs:54-58`

**Issue:** The function's own doc comment (lines 24-31) and the plan's
STRIDE mitigation for T-04-02 both claim the sniff requires "the prefix
contains the four characters that open an SVG root element." The
implementation does not check that the match is the root element, or even
that it is a standalone tag name: `bytes.windows(4).any(|window| window ==
b"<svg")` matches any 4-byte window equal to `<svg`, including as a prefix
of a longer, unrelated element or attribute name (e.g. `<svgProfile>`, or a
comment containing the literal text `<svg`).

I verified this by building a throwaway harness against the compiled crate
and feeding it a well-formed, non-SVG XML document:

```xml
<mydoc xmlns="urn:example">
  <note>this is not an svg document but mentions svg-icon and <svgProfile>data</svgProfile></note>
</mydoc>
```

`looks_like_svg` returned `Ok(true)` for this file. `starts_with_angle_bracket`
is satisfied because the document begins with `<`, and `carries_svg_root` is
satisfied because `<svgProfile` contains the four-byte window `<svg`.

**Why it matters here:** `named_frames_for` checks `looks_like_svg` before
the animation sniff and before falling back to the raster decoder
(`crates/chrys-cli/src/main.rs:663`), so a false positive here steals a file
from the adapter it should have reached. The committed test
(`no_committed_fixture_is_taken_from_the_adapter_it_already_used`) only
exercises this in the direction of proving binary raster/animation fixtures
are *not* misdetected; it does not (and, given the fixture set, cannot)
prove that a non-SVG file starting with `<` and containing `<svg` as a
substring is correctly rejected. In this codebase's own words (AGENTS.md,
"What a test can hold on to"): the code's own claim is stronger than what it
actually checks, which is the same shape of gap AGENTS.md warns a test can
fall into, just here in the production code the test doc-comment describes.
The practical blast radius is bounded today (a misrouted file fails to parse
as SVG and the CLI reports a decode error rather than silently comparing
wrong data), but it is a real content-sniffing precision bug at a
security-relevant dispatch boundary, and it will misroute a legitimate,
larger input format later if one is ever added whose bytes happen to embed
that four-byte window.

**Fix:** Tighten `carries_svg_root` to require the match starts a tag
(preceded only by whitespace, `<`, or start-of-prefix, and followed by
whitespace, `>`, or `/`), or, more simply, restrict the search to the first
element actually opened after `starts_with_angle_bracket` — e.g. parse just
the first tag name out of the prefix and compare it against `svg` (allowing
a namespace prefix like `svg:svg` if that is a real concern for the fixture
corpus) rather than searching the whole 1024-byte window for a raw
substring:

```rust
let after_angle = &bytes[first_lt_index + 1..];
let tag_name: &[u8] = after_angle
    .iter()
    .take_while(|b| b.is_ascii_alphanumeric() || **b == b':')
    .cloned()... // collect until non-name byte
let carries_svg_root = tag_name == b"svg" || tag_name.ends_with(b":svg");
```

Add a test built in the test itself (per AGENTS.md, "build the state a test
needs inside the test") asserting `looks_like_svg` returns `false` for a
well-formed, non-SVG XML document that contains `<svg` as a substring of a
longer tag name, so this property is proven rather than merely documented.

### WR-02: `bounded_read`'s cap arithmetic and the canvas-allocation check both overflow on caller-supplied `SvgLimits`, panicking in a debug build and silently returning wrong data in release

**File:** `crates/chrys-source-svg/src/lib.rs:177` (`max_file_bytes + 1`) and
`crates/chrys-source-svg/src/lib.rs:259` (`(width as u64) * (height as u64)
* 4`)

**Issue:** `SvgLimits` and `SvgSource::with_limits` are public API, not
internal-only; nothing constrains the fields a caller passes. I verified by
direct call:

- `bounded_read(path, u64::MAX)` panics in a debug build
  (`attempt to add with overflow` at `lib.rs:177`) and, in a release build,
  silently wraps `max_file_bytes + 1` to `0`, so `Read::take(0)` reads zero
  bytes and the function returns `Ok(vec![])` for a real, non-empty file —
  the opposite of "no limit," and silently wrong rather than refused.
- With `max_width`/`max_height` set to `u32::MAX` (a caller's plausible way
  to say "effectively unbounded") and an SVG declaring a canvas near
  `u32::MAX` on a side, `(width as u64) * (height as u64) * 4` overflows a
  `u64` in a debug build (`attempt to multiply with overflow` panic at
  `lib.rs:259`); in the release build I tested it happened to still exceed a
  1 MB `max_alloc` after wrapping, but that is not guaranteed for every
  width/height pair near the boundary, and the code offers no guarantee
  either way once the multiplication has wrapped.

**Why it matters here:** the phase's own resource-bounds concern is that a
refusal "must not be defeatable by a lying file header or a declared-length
mismatch." Today the shipped CLI path is safe because `named_frames_for`
always constructs `SvgSource::new()` with `SvgLimits::default()`, whose
fields are far from any overflow boundary. But `chrys-source-svg` is a
library crate with a public `with_limits` constructor, and the panic path
is a real denial-of-service (a caller who reasonably interprets `u32::MAX`
or `u64::MAX` as "effectively unbounded" crashes the process instead of
getting a graceful refusal), while the release-mode wrap is a correctness
bug that is only accidentally safe at the specific values I tried.

**Fix:** Use checked or saturating arithmetic at both sites, refusing with
the existing typed errors instead of overflowing:

```rust
let cap = max_file_bytes.checked_add(1).unwrap_or(u64::MAX);
file.by_ref().take(cap).read_to_end(&mut buffer)...
```

```rust
let alloc = (width as u64)
    .checked_mul(height as u64)
    .and_then(|pixels| pixels.checked_mul(4))
    .unwrap_or(u64::MAX);
```

Add a unit test constructing `SvgLimits` with an extreme (but not
necessarily `u64::MAX`, just near-boundary) value and asserting a typed
refusal rather than a panic or a silently truncated read.

### WR-03: `EmptyCanvas` and `Io` (from `SvgSource::load`, not `bounded_read`) are declared and enforced but never exercised by a test

**File:** `crates/chrys-source-svg/src/lib.rs:149-155` (`EmptyCanvas`),
`crates/chrys-source-svg/src/lib.rs:218-221` and `:171-174`/`:179-182`
(`Io`, both the metadata-read site and the two `bounded_read` I/O sites)

**Issue:** `SvgError` declares six variants; the plan's own acceptance
criteria (04-01) require the error type to carry "at least: the path could
not be read; the file is larger than `max_file_bytes` ...; the document did
not parse ...; the declared canvas is larger than the limits ...; and the
canvas is empty," and both `FileTooLarge` and `CanvasTooLarge`/`AllocTooLarge`
are proven by tests in `crates/chrys-source-svg/tests/svg.rs`. `EmptyCanvas`
(reachable via a `width="0"` or `height="0"` declared SVG, which
`tiny_skia::Pixmap::new` refuses) and `Io` (reachable via a missing or
unreadable path) have no test anywhere in the reviewed files.

**Why it matters here:** this is a minor coverage gap, not a defect in the
enforcement itself — I traced the `EmptyCanvas` path by hand and confirmed
it is reachable (a `width="0"` canvas passes both the `CanvasTooLarge` and
`AllocTooLarge` checks since `0 * height * 4 == 0`, then `Pixmap::new(0, h)`
returns `None`). Two of the six declared error variants are unproven by any
committed test, which is a real gap relative to this phase's own stated
practice of proving every refusal it declares.

**Fix:** Add `a_zero_width_canvas_is_refused_as_empty` (an SVG declaring
`width="0"`) and `a_missing_path_reports_the_io_variant` (a path that does
not exist) to `crates/chrys-source-svg/tests/svg.rs`, matching the shape the
other refusal tests already use.

## Info

### IN-01: `pinned_family_name` is public and panics on a caller-supplied empty database

**File:** `crates/chrys-source-svg/src/fonts.rs:57-67`

**Issue:** `pub fn pinned_family_name(db: &fontdb::Database) -> String`
`.expect()`s that `db` holds at least one face. This is documented ("Panics
when `db` holds no face at all"), and every internal call site passes a
database `pinned_fontdb()` itself built, so it is not reachable from any
code in this workspace. It is nonetheless a public function on a public
module of a library crate, so an external caller passing an arbitrary
`fontdb::Database` (e.g. one built directly rather than through
`pinned_fontdb()`) gets a panic rather than a `Result`.

**Fix:** Either keep it as documented-panic API (acceptable, since this is
an internal-shaped helper exposed mainly for the test suite to read the
fallback name without a second literal) or change the signature to `Option<
String>`/`Result` if this function is meant to be part of the crate's public
contract rather than a test-support export. No change is required if the
intent is "this is effectively `pub(crate)` plus test visibility," but that
intent is not stated anywhere the signature itself is visible.

### IN-02: the OFL licence text is sourced from a different upstream repository than the font binary

**File:** `crates/chrys-source-svg/fonts/OFL.txt`

**Issue:** Not a defect — flagged only because the required reading
(04-01-SUMMARY.md, Deviations) records that `OFL.txt` was fetched from
`google/fonts`' `ofl/notosans/OFL.txt` rather than from
`notofonts.github.io`, because the pinned tag carries no per-family
`OFL.txt` of its own. The summary verifies the two sources agree by
comparing the embedded font's own copyright string against the licence
text's copyright string, which is a reasonable verification but is a
by-hand, one-time check rather than something a test or CI step re-proves.

**Fix:** No action required for this phase. If this crate's font is ever
re-pinned to a different release or family, re-verify (and ideally script)
the same copyright-string comparison rather than assuming the two sources
stay in agreement indefinitely.

---

_Reviewed: 2026-09-09T12:14:39Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
