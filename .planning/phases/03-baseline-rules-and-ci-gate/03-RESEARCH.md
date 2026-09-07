# Phase 3: Baseline, rules and CI gate - Research

**Researched:** 2026-09-07
**Domain:** A declarative TOML tolerance-rule engine and a trait-based committed-golden-file baseline store, layered entirely outside the existing format-blind comparison engine
**Confidence:** HIGH on in-repo architecture facts (every claim about `Verdict`, `Region`, `ChangeKind`, `Frame`, `RegionHint`, `hash::rgba8_digest`, `hints.rs` and the CLI is grounded in a `Read` of the exact file this session, quoted below, or in a command actually run this session); HIGH on the `toml` 1.1.5 error-reporting behaviour (verified by compiling and running real code against the pinned version, not by reading prose documentation); MEDIUM on the exact rule-file field names and CLI exit-code scheme, which are this session's own design recommendation, not a fact fetched from an external source.

## Summary

Phase 3 adds three capabilities — a TOML tolerance-rule reader, a committed-golden-file baseline store, and a CI-gating report — to a comparison engine (`chrys-core`) that Phase 1 and Phase 2 already built and that this project's own invariants forbid touching a second way here. The header constraint is explicit: `chrys-core` declares no `serde` and no `toml`, so the rule reader and the baseline store cannot live there. This session's own reading of `chrys-core`'s public surface shows that constraint costs nothing: `Verdict`, `Region`, `ChangeKind`, `RefusalReason` and `chrys_core::hash::rgba8_digest` are already `pub` `[VERIFIED: crates/chrys-core/src/lib.rs:16-18, verdict.rs:59-71, 112-145, hash.rs:22]`, so a rule evaluator and a baseline store can each be built as a new crate that *depends on* `chrys-core` and reads its already-public types, never as a crate `chrys-core` depends on and never as an edit inside `crates/chrys-core/`. The strongest, most testable finding of this research is that **this phase can be planned to touch zero files under `crates/chrys-core/`** — the same falsifiable, drillable claim Phase 2 proved for SRC-08, and `scripts/engine-boundary-drill.sh` already exists, is already generic over any commit range, and needs no modification to drill Phase 3's version of that same claim `[VERIFIED: scripts/engine-boundary-drill.sh:1-154, read in full this session]`.

The second load-bearing finding is empirical, not documentary: this session compiled and ran real code against the pinned `toml = "=1.1.5"` and `serde = "=1.0.229"` to answer RULE-05 directly, rather than trusting a documentation summary (an earlier `WebFetch` against `docs.rs` gave an imprecise answer for `toml::Spanned<T>` that this session's own compiled test then contradicted). Plain `toml::from_str` with `#[serde(deny_unknown_fields)]` already reports an unknown key, an invalid enum variant, and a wrong-typed value with a caret-annotated, line-and-column-numbered message, with no extra work `[VERIFIED: cargo run against /tmp/toml-span-test, this session — real compiler and runtime output quoted in Common Pitfalls and Code Examples]`. This is not new information this repository lacks: `crates/chrys-source-raster/src/hints.rs`'s own test suite already proves the same three failure modes for its hints sidecar, today, in the committed codebase `[VERIFIED: crates/chrys-source-raster/src/hints.rs:191-258]`. The one case that plain `deny_unknown_fields` cannot catch — a syntactically valid key holding a semantically nonsense value for its own rule's `kind` (a `max_delta_e` tolerance on a `Moved` rule, say) — is answered by wrapping each field in `toml::Spanned<T>`, which this session confirmed, by compiling and running it, reports the exact source line of the offending field through plain `toml::from_str`, with no special deserializer entry point required, contradicting the first-pass web-sourced answer.

The third finding reframes RULE-02's two scoping mechanisms. "A named region" is not new: `chrys_source::RegionHint` and the sidecar `<stem>.hints.toml` reader Phase 2 built already give every frame a `Vec<RegionHint>` `[VERIFIED: crates/chrys-source/src/lib.rs:125-138, crates/chrys-source-raster/src/hints.rs:1-63]`. A rule scoped "by named region" only has to test whether a detected `Region`'s `BoundingBox` overlaps a hint already present on the frame — no new hint-supply mechanism, no CLI flag, and critically, **no relationship to the existing `--region NAME` CLI flag**, which crops the whole comparison to one rectangle for registration purposes (Phase 2's Q6). Confusing the two is this phase's sharpest naming pitfall (see Common Pitfalls). "A mask" is the one genuinely new input this phase reads: an image file naming, by its own pixels, an irregular area to scope a tolerance to, decoded through the exact same guarded, memory-limited entry point (`chrys_source_raster::decode::decode_guarded`) every other raster decode in this project already uses, so CLI-04's discipline extends to it for free rather than by new code.

**Primary recommendation:** add two new crates, `chrys-rule` (TOML rule parsing plus tolerance evaluation, depending on `chrys-core`, `chrys-source`, `chrys-source-raster`, `toml`, `serde`, `thiserror`) and `chrys-baseline` (the `BaselineStore` trait plus a `GoldenFileStore` backend, depending only on `toml`, `serde`, `thiserror` — no pixel-format or comparison-engine dependency at all); extend the existing `compare` subcommand with `--baseline <NAME>`, `--rule <PATH>` and `--report <PATH>` flags, and add one new `accept <NAME> <PATH>` subcommand; add every new dependency at versions already pinned and already audited in `01-RESEARCH.md` and `02-RESEARCH.md`, since this phase introduces **no new external package** to the workspace.

## Architectural Responsibility Map

This project has no browser/server/CDN tiers; the boundary is the one Phase 1 drew and Phase 2 extended: adapter (format-aware) vs. engine (format-blind) vs. CLI (orchestration, serialization, policy).

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Rule-file parsing (TOML, `deny_unknown_fields`, `Spanned` line reporting) | New adapter-shaped crate (`chrys-rule`) | — | Needs `toml`/`serde`, which `chrys-core` may never depend on (invariant 1) |
| Tolerance evaluation (does a detected `Region` fall inside a rule's scope and within its tolerance?) | `chrys-rule` | — | Reads only `chrys-core`'s already-public `Verdict`/`Region`/`ChangeKind` types; the evaluation is a pure function of already-computed engine output, so it can live outside the engine without duplicating any engine logic |
| Mask decode (an image naming an irregular tolerance area) | `chrys-rule`, via `chrys-source-raster`'s existing `decode_guarded` | — | Reuses the one guarded decode entry point (CLI-04); `chrys-rule` never opens its own decoder |
| Baseline resolution and acceptance (name → stored path, hash manifest read/write) | New crate (`chrys-baseline`) | — | A pure storage concern; must not know a pixel format, a `Verdict`, or a `ChangeKind` (research question 4) |
| Report artifact serialization (kind, region, size, per Region) | CLI (`chrys-cli`) | — | Phase 1's own research already assigned "the file-writing form" of the verdict/report to the CLI, deferred to Phase 3 `[VERIFIED: .planning/phases/01-raster-engine-and-determinism-proof/01-RESEARCH.md, Architectural Responsibility Map row "Verdict / report data model"]` |
| Rule-gated exit code (CLI-01) | CLI (`chrys-cli`) | — | The exit code is a policy decision over an already-computed `Vec<Verdict>` plus `chrys-rule`'s outcome; it changes no engine behaviour |
| Comparison engine itself (register, classify, hash, verdict) | Engine (`chrys-core`) | — | **Unchanged this phase.** Every capability above is additive and external; see Summary and the Common Pitfalls entry "A rule that reaches back into the engine" |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RULE-01 | A TOML rule file sets tolerance by kind of change | `chrys-rule`'s `RuleRow.kind: Spanned<ChangeKindName>`, one of the five `ChangeKind` variants (Architecture Patterns, Pattern 1) |
| RULE-02 | A rule is scoped to a named region or to a mask | `Scope::Region(String)` matched against `Frame.hints`, or `Scope::Mask(PathBuf)` decoded via `decode_guarded` and tested by majority pixel overlap (Architecture Patterns, Pattern 2) |
| RULE-03 | A rule file is read as data. No rule is computed or executed | The rule schema has no expression, no operator and no scripting field; `serde(deny_unknown_fields)` closes the schema so no such field can be added silently (Don't Hand-Roll) |
| RULE-04 | The shipped example rule file shows a scoped exclusion, and never a bare global threshold | `Scope` has no "no scope" variant at the type level — every `[[rule]]` table requires `region` or `mask` — so an unscoped rule is a parse-time refusal, not merely an example convention (Architecture Patterns, Pattern 1; Open Questions 1) |
| RULE-05 | An unknown key or a malformed rule fails loudly and names the line | `deny_unknown_fields` + `toml::from_str` already does this for a malformed or unknown-keyed file, verified this session by compiling and running it; `toml::Spanned<T>` on each field does the same for a syntactically valid but semantically wrong value (Common Pitfalls, Code Examples) |
| BASE-01 | A baseline hash manifest is committed to the repository | `chrys-baseline/MANIFEST.toml`, written only by `accept`, never gitignored (`.gitignore` read this session, no matching exclusion) (Architecture Patterns, Pattern 3) |
| BASE-02 | Committed golden files are the default store backend | `GoldenFileStore` copies the accepted file or directory verbatim into `chrys-baseline/<name>/`, alongside the manifest (Architecture Patterns, Pattern 3) |
| BASE-03 | The store is a trait, so a second backend is added without a change to the engine | `BaselineStore` trait, two methods, defined in `chrys-baseline`, never in `chrys-core`; depends on neither `chrys-source` nor `chrys-core` (research question 4, Architecture Patterns Pattern 3) |
| BASE-04 | A person accepts a new baseline explicitly. Nothing updates a baseline on its own | The new `accept` subcommand is the only code path that calls `BaselineStore::accept`; `compare`/`--baseline` only ever calls `resolve` (Common Pitfalls, "An implicit accept") |
| CLI-01 | The tool exits non-zero when a rule fails | New exit-code scheme in `chrys-cli`, additive when `--rule` is given, unchanged otherwise (Architecture Patterns, Pattern 4) |
| CLI-02 | The tool writes a report artifact that names each change by kind, region and size | New `--report PATH` flag, TOML-serialized report structs living in `chrys-cli` only (Architecture Patterns, Pattern 5; addresses criterion 6 via the new `Frame.source_name` field) |
| CLI-03 | The tool compares two paths given on the command line and prints the verdict | **Already built and tested** by Phase 1/2's `compare <base> <candidate>`; Phase 3's job is regression-proofing this default path while adding new, opt-in flags (Common Pitfalls, "Breaking the flag-less path") |
| CLI-04 | Every decode call sets an explicit memory limit, so a malformed input fails instead of exhausting memory | Already true for every existing decode call site (`chrys-source-raster`, `chrys-source-sequence`, `chrys-source-animation`, all verified this session); Phase 3 adds a mask decode call site through the same guarded function, and adds a static guard test proving the invariant holds workspace-wide (Common Pitfalls, "An unaudited decode call site") |

## Standard Stack

### Core

No new external package is introduced by this phase. Every dependency Phase 3 needs is already pinned in the workspace `Cargo.toml` and already carries a Package Legitimacy Audit from `01-RESEARCH.md` or `02-RESEARCH.md`.

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `toml` | `=1.1.5` (already pinned) | Parse the rule file and the baseline manifest | Already this project's one declarative-config format; re-verified current this session `[VERIFIED: cargo search toml --limit 1, this session: "toml = \"1.1.5+spec-1.1.0\""]` |
| `serde` | `=1.0.229`, `derive` feature (already pinned) | Deserialize rule rows and manifest entries; serialize the manifest and the report | Already the project's serialization foundation; re-verified current this session `[VERIFIED: cargo search serde --limit 1, this session: "serde = \"1.0.229\""]` |
| `thiserror` | `=2.0.20` (already pinned) | Typed errors in `chrys-rule` and `chrys-baseline` | Already this project's error-type standard across every crate read this session |
| `chrys-core` (workspace) | — | `Verdict`, `Region`, `ChangeKind`, `RefusalReason`, `hash::rgba8_digest` — all already `pub` | `chrys-rule` and the CLI depend on it; it depends on neither in return |
| `chrys-source` (workspace) | — | `Frame`, `RegionHint` | Needed by `chrys-rule` for region-hint overlap and by the CLI's new `Frame.source_name` field |
| `chrys-source-raster` (workspace) | — | `decode::decode_guarded`, `DecodeLimits` | Reused, not reimplemented, for mask decode (CLI-04) |

### Supporting

None. This phase's two new crates (`chrys-rule`, `chrys-baseline`) need no dependency beyond the ones already listed above.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| A hand-rolled recursive directory copy for `GoldenFileStore::accept` on a multi-file (sequence) baseline | The `fs_extra` crate | `fs_extra` exists on crates.io but was not probed this session, because a recursive copy of a handful of files is not a "don't hand-roll" class problem — it is roughly the same ten lines as `crates/chrys-core/tests/determinism.rs`'s own `rust_files_under` stack-based walk `[VERIFIED: crates/chrys-core/tests/determinism.rs:190-208]` — and adding a dependency for it would cost more review than it saves |
| TOML for the report artifact | `serde_json` | JSON is the more common machine-report format, but it is a new dependency this phase does not otherwise need; `toml`+`serde` are already pinned, and Phase 1's own Architectural Responsibility Map already assigned "the file-writing form" of the verdict to the CLI without naming a format, so nothing external constrains the choice. TOML also keeps the report git-diff-readable, consistent with this project's existing plain-text digest files. Revisit at the point CI-01 (v2, a PR-comment format) is actually planned. |
| A `Spanned`-per-field rule schema | Reporting only the enclosing `[[rule]]` table's own span (its start line) for every error, syntactic or semantic | Table-level spanning was this session's first, simpler design and does satisfy RULE-05's "names the line" literally (it names the line the offending rule table starts on). Field-level `Spanned<T>` was adopted instead because it was verified, by compiling and running it, to report the *exact* line of the bad field, which is a strictly better answer to "malformed rule" and cost nothing extra to implement. |

**Installation:**
```bash
cargo new --lib crates/chrys-rule
cargo new --lib crates/chrys-baseline

# chrys-rule/Cargo.toml
cargo add --manifest-path crates/chrys-rule/Cargo.toml --path ../chrys-core chrys-core
cargo add --manifest-path crates/chrys-rule/Cargo.toml --path ../chrys-source chrys-source
cargo add --manifest-path crates/chrys-rule/Cargo.toml --path ../chrys-source-raster chrys-source-raster
cargo add toml serde --features derive --manifest-path crates/chrys-rule/Cargo.toml
cargo add thiserror --manifest-path crates/chrys-rule/Cargo.toml

# chrys-baseline/Cargo.toml (no chrys-core, no chrys-source: see research question 4)
cargo add toml serde --features derive --manifest-path crates/chrys-baseline/Cargo.toml
cargo add thiserror --manifest-path crates/chrys-baseline/Cargo.toml

# chrys-cli/Cargo.toml
cargo add --manifest-path crates/chrys-cli/Cargo.toml --path ../chrys-rule chrys-rule
cargo add --manifest-path crates/chrys-cli/Cargo.toml --path ../chrys-baseline chrys-baseline
cargo add toml serde --features derive --manifest-path crates/chrys-cli/Cargo.toml
```

**Version verification:** `toml` and `serde` were re-checked against the live registry this session with `cargo search`, the same command `01-RESEARCH.md` and `02-RESEARCH.md` used, and both match the versions already pinned in the workspace `Cargo.toml` exactly `[VERIFIED: this session]`.

## Package Legitimacy Audit

**No new external package is introduced by this phase.** Every dependency named above is already pinned in the workspace `Cargo.toml` `[VERIFIED: Cargo.toml:10-23, read this session]` and was already audited:

| Package | Registry | Previously audited in | Verdict | Disposition |
|---------|----------|------------------------|---------|-------------|
| `toml` | crates.io | `02-RESEARCH.md` Package Legitimacy Audit | OK | Re-used, no new audit needed |
| `serde` | crates.io | `02-RESEARCH.md` Package Legitimacy Audit | OK | Re-used, no new audit needed |
| `thiserror` | crates.io | `01-RESEARCH.md` Package Legitimacy Audit | OK | Re-used, no new audit needed |

**Packages removed due to `[SLOP]` verdict:** none — none were considered.
**Packages flagged as suspicious `[SUS]`:** none.

## Architecture Patterns

### System Architecture Diagram

```
   rule file (TOML)     mask image (PNG)      base/candidate paths, or
        |                     |                --baseline <NAME>
        v                     v                        |
  +-----------+       +----------------+                v
  | chrys-rule |<------| decode_guarded |    +----------------------+
  | parse+     |       | (reused,       |    | chrys-baseline        |
  | validate   |       |  chrys-source- |    | resolve(name) -> path |
  +-----------+       |  raster)       |    | (read MANIFEST.toml) |
        |              +----------------+    +----------------------+
        |  Vec<Rule>                                   |
        |                                    resolved base path
        v                                               |
  +----------------------------------------------------------------+
  |  chrys-cli: frames_for() (UNCHANGED, Phase 1/2) decodes both    |
  |  sides into Vec<Frame>, then chrys_core::compare_sequence()      |
  |  (UNCHANGED) produces Vec<Verdict>                               |
  +----------------------------------------------------------------+
        |
        v  Vec<Verdict>, Vec<Frame> (with .hints, .source_name)
        |
  +-----------+
  | chrys-rule |  evaluate(): for each Changed region, test its
  | evaluate() |  bbox against each rule's Scope (region-hint overlap,
  |           |  or mask-pixel overlap) and its own tolerance kind
  +-----------+
        |
        v  RuleOutcome { tolerated, violations }
        |
  +----------------------------------------------------------------+
  |  chrys-cli: exit-code policy (CLI-01) + --report writer (CLI-02) |
  |  named by kind, region, size, per Region -- see Pattern 5        |
  +----------------------------------------------------------------+
        |
        v
  process exit code; optional report.toml written to disk

  +----------------------------------------------------------------+
  |  chrys-cli: accept <NAME> <PATH> -- the ONLY path that calls    |
  |  chrys_baseline::GoldenFileStore::accept(). Computes             |
  |  chrys_core::hash::rgba8_digest per decoded frame, copies PATH   |
  |  verbatim into chrys-baseline/<NAME>/, updates MANIFEST.toml     |
  +----------------------------------------------------------------+
```

### Recommended Project Structure

```
chrysoberyl/
├── crates/
│   ├── chrys-core/              # UNCHANGED this phase — see Summary
│   ├── chrys-source/            # UNCHANGED this phase
│   ├── chrys-source-raster/     # UNCHANGED this phase (its decode_guarded is reused, not edited)
│   ├── chrys-rule/              # NEW: rule-file parsing + tolerance evaluation
│   │   ├── src/
│   │   │   ├── lib.rs           # Rule, Scope, ChangeKindName, RuleError
│   │   │   └── evaluate.rs      # evaluate(): region/mask overlap + tolerance check
│   │   └── tests/
│   ├── chrys-baseline/          # NEW: BaselineStore trait + GoldenFileStore
│   │   ├── src/
│   │   │   ├── lib.rs           # BaselineStore trait
│   │   │   └── golden.rs        # GoldenFileStore: MANIFEST.toml read/write, accept/resolve
│   │   └── tests/
│   └── chrys-cli/                # EXTENDED: --baseline/--rule/--report flags, accept subcommand,
│                                  # report.rs (TOML report structs), Frame.source_name plumbing
├── examples/
│   └── rules/
│       └── example.toml          # RULE-04's shipped example: a scoped exclusion, never global
└── tests/
    └── golden/
        ├── rule-01/               # NEW: fixtures exercising region- and mask-scoped tolerance
        └── baseline-01/           # NEW: fixtures exercising accept/resolve/manifest survival
```

### Pattern 1: The rule schema has no unscoped shape (RULE-01, RULE-02, RULE-04)

**What:** Every `[[rule]]` table requires exactly one of `region` or `mask`, enforced at the type level, not by convention. This is a stronger reading of RULE-04 than "write a good example": the schema itself cannot express a bare global threshold.

**Recommended shape** (not yet written; a design sketch grounded in `chrys_core::ChangeKind`, read this session at `crates/chrys-core/src/verdict.rs:59-71`):
```rust
// crates/chrys-rule/src/lib.rs — recommended, not yet written.
use serde::Deserialize;
use toml::Spanned;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum KindName {
    Moved,
    Added,
    Removed,
    Recoloured,
    Resized,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleRow {
    pub kind: Spanned<KindName>,
    pub region: Option<Spanned<String>>,
    pub mask: Option<Spanned<PathBuf>>,
    pub max_delta_e: Option<Spanned<f32>>,
    pub max_offset_px: Option<Spanned<u32>>,
    pub max_size_delta_fraction: Option<Spanned<f32>>,
    pub allow: Option<Spanned<bool>>,
}

#[derive(Debug, Deserialize)]
pub struct RuleDocument {
    #[serde(rename = "rule", default)]
    pub rule: Vec<RuleRow>,
}
```
`RuleRow` deserializes permissively (every tolerance field is `Option`); a second, explicit validation pass — not `serde` — rejects a row with neither `region` nor `mask`, both `region` and `mask`, or a tolerance field that does not match its own `kind` (see Code Examples for the verified line-reporting behind that rejection). This mirrors `chrys-source-raster/src/hints.rs`'s own two-phase shape exactly: deserialize into a permissive row type, then apply a semantic check `HintRow`/`RegionHint` conversion cannot express through `serde` alone `[VERIFIED: crates/chrys-source-raster/src/hints.rs:119-130 — the duplicate-name check runs after deserialization, not during it]`.

**Shipped example** (`examples/rules/example.toml`, RULE-04):
```toml
# example.toml — every rule below is scoped. There is no rule shape that
# applies to a whole frame; a rule that is not named to something specific
# is the "wall of green" this schema is built to prevent.

[[rule]]
kind = "recoloured"
region = "clock"
max_delta_e = 12.0
# The on-screen clock repaints every render. A small colour drift here is
# expected, and it is not a regression.

[[rule]]
kind = "moved"
mask = "masks/scrollbar-track.png"
max_offset_px = 4
# The scrollbar thumb's resting pixel depends on font metrics that vary by
# a pixel or two by design. The mask covers only its track, not the page
# content around it.
```

### Pattern 2: Two scoping mechanisms, one already built, one newly decoded

**What:** `region = "name"` is resolved by testing a detected `Region.bbox` against every `RegionHint` already present on the compared `Frame`s (Phase 2's sidecar mechanism, unchanged). `mask = "path"` is resolved by decoding the named image through the existing guarded entry point and testing the same bounding box against the mask's own pixels.

**When to use:** `region` when the tolerance area is a fixed, named rectangle a producer already declares (reuses an existing hint with zero new decode work). `mask` when the tolerance area is irregular, or when no hint sidecar names it.

**Example (region overlap, majority rule, matching this codebase's own tie-break idiom):**
```rust
// Recommended, not yet written. The "more than half the bbox area
// overlaps" rule mirrors classify::kind::majority_block_offset's own
// majority-wins idiom, verified this session at
// crates/chrys-core/src/classify/kind.rs:191-221.
fn bbox_overlaps_hint(bbox: &chrys_core::BoundingBox, hint: &chrys_source::RegionHint) -> bool {
    let overlap_x0 = bbox.x.max(hint.x);
    let overlap_y0 = bbox.y.max(hint.y);
    let overlap_x1 = (bbox.x + bbox.width).min(hint.x + hint.width);
    let overlap_y1 = (bbox.y + bbox.height).min(hint.y + hint.height);
    if overlap_x1 <= overlap_x0 || overlap_y1 <= overlap_y0 {
        return false;
    }
    let overlap_area = (overlap_x1 - overlap_x0) as u64 * (overlap_y1 - overlap_y0) as u64;
    let bbox_area = bbox.width as u64 * bbox.height as u64;
    overlap_area * 2 >= bbox_area.max(1) // at least half the bbox
}
```
**Example (mask decode, reusing the guarded entry point, satisfying CLI-04 for free):**
```rust
// Recommended, not yet written. mask_path is relative to the rule file's
// own directory, mirroring how hints.rs resolves a sidecar next to its
// image, not next to the process's current directory.
use chrys_source_raster::{DecodeLimits, decode::decode_guarded, normalize::normalize_to_rgba8};

fn load_mask(mask_path: &std::path::Path) -> Result<chrys_source::Frame, RuleError> {
    let (dynamic, orientation) = decode_guarded(mask_path, &DecodeLimits::default())?;
    let (pixels, width, height) = normalize_to_rgba8(dynamic, orientation);
    Ok(chrys_source::Frame { pixels, width, height, index: 0, hints: Vec::new(), source_name: mask_path.display().to_string() })
}
```

### Pattern 3: The baseline store is a two-method trait that never sees a pixel

**What:** Answering research question 4 directly. `BaselineStore` must not know a pixel format, a `Frame`, a `Verdict` or a `ChangeKind` — only a name, a path, and a list of already-computed digest strings.

```rust
// crates/chrys-baseline/src/lib.rs — recommended, not yet written.
use std::path::{Path, PathBuf};

pub trait BaselineStore {
    type Error;

    /// Return the path of the currently accepted baseline named `name`.
    fn resolve(&self, name: &str) -> Result<PathBuf, Self::Error>;

    /// Record `frame_digests` (one lowercase-hex SHA-256 per frame, in
    /// frame order) as the new baseline for `name`, and copy
    /// `candidate_path` (a file or a directory) into the store verbatim.
    /// The caller has already decoded `candidate_path` and computed each
    /// digest with `chrys_core::hash::rgba8_digest`; this trait never
    /// decodes anything itself.
    fn accept(
        &self,
        name: &str,
        candidate_path: &Path,
        frame_digests: &[(String, String)], // (source file name, hex digest)
    ) -> Result<(), Self::Error>;
}
```
`GoldenFileStore { root: PathBuf }` implements this over `<root>/<name>/` (the copied file(s), under their own original names) and `<root>/MANIFEST.toml`:
```toml
# chrys-baseline/MANIFEST.toml — written only by `accept`. A hand edit
# will not be caught by anything and will silently be trusted.

[[baseline]]
name = "logo-header"

[[baseline.frame]]
source = "base.png"
digest = "5a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc"
```
`resolve("logo-header")` returns `<root>/logo-header/` (or, when it holds exactly one file, that file's own path) by **listing the directory**, not by trusting the manifest's `source` field — so a hand-`git mv` rename of the stored file, while it stays inside its own `<name>/` directory, does not break resolution (research question 3's "survive a rename"). The manifest's `source` field is informational only, refreshed the next time `accept` runs; the identity a person actually tracks is the `name` key, not a file path.

**Why a diff of this file is readable in a pull request:** a single accepted change to one frame of one baseline changes exactly one `digest = "..."` line. A reviewer reads "this baseline's hash changed from X to Y" without opening the binary image at all, and the accompanying binary file diff (which GitHub renders as an image diff for common formats) supplies the "why."

### Pattern 4: The rule-gated exit code is additive, not a replacement

**What:** `chrys-cli`'s existing `verdict_rank` (`Identical` = 0, `Changed` = 1, `Refused` = 2) is read verbatim, unchanged, at `crates/chrys-cli/src/main.rs:184-193` this session. Without `--rule`, this stays the exit code exactly as Phase 1/2 built it (CLI-03's regression guard). With `--rule`, a `Changed` verdict whose every region is tolerated by a matching rule ranks 0 (pass); a `Changed` verdict with at least one untolerated region keeps ranking 1; `Refused` stays 2, because a rule cannot tolerate "this pair could not be registered" — refusal is a different question than tolerance.

```rust
// Recommended, not yet written. Extends crates/chrys-cli/src/main.rs's
// existing verdict_rank (read verbatim this session at lines 184-193)
// rather than replacing it.
fn verdict_rank_with_rules(verdict: &chrys_core::Verdict, outcome: Option<&chrys_rule::RuleOutcome>) -> u8 {
    match (verdict, outcome) {
        (chrys_core::Verdict::Changed { .. }, Some(outcome)) if outcome.violations.is_empty() => 0,
        (chrys_core::Verdict::Identical, _) => 0,
        (chrys_core::Verdict::Changed { .. }, _) => 1,
        (chrys_core::Verdict::Refused { .. }, _) => 2,
    }
}
```

### Pattern 5: The report reuses one new `Frame` field for two requirements at once

**What:** Criterion 6 ("the report names the file a frame came from, not only its index") and CLI-02 ("names each change by kind, region, and size") both need a frame-level label beyond the numeric `index` Phase 2 already carries. Add exactly one new field, `source_name: String`, to `chrys_source::Frame`, populated by every adapter: `RasterSource` sets it to the file's own name, `SequenceSource` sets it to each frame file's own name (the exact thing `02-RESEARCH.md`'s Q9 fixture generator already names on disk), `AnimationSource` sets it to the container file's own name for every frame (honest: a composited animation frame has no file of its own, only the container it came from).

This is additive to a type `chrys-core` does not own (`Frame` lives in `chrys-source`), so it is not a `chrys-core` edit; it does require updating every `Frame { .. }` literal in `chrys-core`'s own tests and fixtures with one more field, exactly the same mechanical, non-logic-changing update Phase 2 already made once for `hints` `[VERIFIED: 02-LEARNINGS.md's own "Surprises" section: "The Source trait needed no change at all... Frame already carried index and hints"]`. The same field feeds `chrys-baseline`'s manifest `source` column (Pattern 3), so this one addition serves both the report and the baseline manifest.

```toml
# report.toml — written by --report PATH. TOML, reusing the already-
# pinned toml/serde pair (see Alternatives Considered).

[meta]
base = "tests/golden/rule-01/base.png"
candidate = "tests/golden/rule-01/candidate.png"

[[frame]]
index = 0
source = "base.png"
verdict = "changed"

[[frame.region]]
kind = "recoloured"
region = "clock"          # the named hint this region overlapped, or absent
x = 96
y = 96
width = 64
height = 64
size = 4096                # width * height, in pixels
delta_e = 42.7
base_colour = [180, 180, 180, 255]
candidate_colour = [10, 200, 10, 255]
rule_outcome = "tolerated" # "tolerated" | "violation" | absent when --rule was not given
```

### Anti-Patterns to Avoid

- **A rule field that names a computation instead of a value.** RULE-03 forbids this outright: no field may hold an expression, a comparison operator string, or a reference to another rule. Every field in `RuleRow` above is a literal (a string, a number, a path, a bool); the schema's own shape is the enforcement, not a runtime check.
- **Trusting `serde(deny_unknown_fields)` to catch a semantically wrong value.** It catches an unknown *key* and a wrongly *typed* value (confirmed this session — see Code Examples), but a `max_delta_e` on a `Moved` rule is a well-typed float in a field that exists; only the post-deserialize validation pass (Pattern 1) catches that it does not belong on this row's own `kind`.
- **Letting `BaselineStore::resolve` write anything.** `resolve` is read-only by contract; only `accept` writes. A guard test asserting this (grep for `fs::write`/`fs::copy`/`fs::create_dir` occurring only inside `accept`'s own function body, mirroring `crates/chrys-core/tests/determinism.rs`'s own comment-and-string-stripping search technique) is cheap insurance against BASE-04 regressing silently.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML parsing with line-numbered errors | A hand-written key=value parser, or a hand-rolled line-counter over raw text | `toml::from_str` + `serde(deny_unknown_fields)` + `toml::Spanned<T>` | Verified this session, by compiling and running it, to already produce a caret-annotated, line-numbered message for an unknown key, a wrong type, an invalid enum variant, and (via `Spanned`) a semantically misplaced field — with zero custom parsing code |
| SHA-256 digest for the baseline manifest | A hand-rolled hash function, or a second hashing crate | `chrys_core::hash::rgba8_digest`, already `pub` `[VERIFIED: crates/chrys-core/src/hash.rs:22]` | The exact function Phase 1 built for the same "raw RGBA8, never a re-encoded file" contract this manifest also needs; a second implementation is a second place the contract can drift |
| Reading a mask image's pixels | A second `image::ImageReader` call site inside `chrys-rule` | `chrys_source_raster::decode::decode_guarded` + `normalize::normalize_to_rgba8` (reused) | Already guards the CVE-2023-29408-class decompression-bomb risk (CLI-04); a second unguarded call site is a second place that guard can be forgotten, the exact lesson `02-RESEARCH.md`'s own "Don't Hand-Roll" table already recorded for the sequence adapter |
| Rule-region overlap heuristic | A new, bespoke "is this region inside that rectangle" rule | The majority-overlap idiom already used by `classify::kind::majority_block_offset` `[VERIFIED: crates/chrys-core/src/classify/kind.rs:185-221]` | Reusing an idiom already reviewed and tested in this codebase, rather than inventing a second convention for "majority wins" |

**Key insight:** every item above is already solved by a dependency or a function this project already has, either pinned (`toml`, `serde`) or already `pub` (`chrys_core::hash::rgba8_digest`, `chrys_source_raster::decode::decode_guarded`). Phase 3's actual new code is small: two thin crates gluing already-existing, already-tested primitives together under a new, narrowly-scoped schema.

## Common Pitfalls

### Pitfall 1: Trusting a documentation summary over running the code

**What goes wrong:** A first-pass `WebFetch` against `docs.rs/toml/1.1.5/toml/struct.Spanned.html` stated that `toml::from_str` does not populate `Spanned<T>` and that "a specialized entry point" is required. This is wrong for the pinned version.

**Why it happens:** A documentation-summarizing tool answers from the page's prose, and `Spanned<T>`'s own populate mechanism (a magic field name the `Deserializer` recognizes) is an implementation detail the prose does not spell out plainly.

**How to avoid:** For a load-bearing claim about a specific pinned version's behaviour, compile and run real code against that exact version, in a throwaway crate, and read the actual output. This session did exactly that:
```
$ cargo run -q   # crate depending on toml = "=1.1.5", serde = "=1.0.229"
CASE1 error:
TOML parse error at line 1, column 8
  |
1 | kind = "teleported"
  |        ^^^^^^^^^^^^
unknown variant `teleported`, expected one of `moved`, `added`, `removed`, `recoloured`, `resized`

---span=Some(7..19)
CASE2 error:
TOML parse error at line 3, column 1
  |
3 | bogus = 1
  | ^^^^^
unknown field `bogus`, expected `kind` or `tolerance`
```
`[VERIFIED: this session's own compiled run against the pinned `toml`/`serde` versions]`

**Warning signs:** A plan task that cites a `docs.rs` prose summary for a specific version's exact runtime behaviour, with no command output to back it.

**Phase to address:** This phase, before the rule reader's design is finalized — it already has been, here.

### Pitfall 2: Conflating the two "region" concepts

**What goes wrong:** Phase 2's `--region NAME` CLI flag crops the *entire comparison* to one named rectangle, changing what the engine registers against. Phase 3's rule-file `region = "name"` field only *tags which already-detected regions get a tolerance*; it changes nothing about registration and requires no CLI flag. A plan or an implementer that treats these as the same mechanism will either crop comparisons the rule file never asked to crop, or silently expect `--region` to be present for rule scoping to work.

**Why it happens:** Both reuse `chrys_source::RegionHint.name`, and both are called "region" in prose (this research included).

**How to avoid:** State the distinction explicitly in the plan's task descriptions: `--region` is a CLI-only, whole-comparison narrowing flag (Phase 2, unchanged); a rule's `region =` is a TOML-only, post-hoc tolerance tag (Phase 3, new), and the two compose but never require each other.

**Warning signs:** A test that only exercises a rule's `region` scoping with `--region` also passed on the command line, hiding the fact that the rule alone (with no CLI flag) does not work.

**Phase to address:** This phase, in the rule-evaluation task's own test fixtures — include at least one fixture exercising `region =` scoping with **no** `--region` flag on the command line.

### Pitfall 3: Using `--exact` against a module path, silently testing nothing

**What goes wrong:** `cargo test -p <crate> <module>::tests -- --exact` reports `0 passed; 0 failed; ... N filtered out` and **exits 0**, because `--exact` matches a whole, fully-qualified test name, not a module prefix. Phase 2's own validation contract shipped exactly this mistake and the planner caught it only by inspection `[VERIFIED: .planning/phases/02-source-trait-and-a-second-format/02-LEARNINGS.md, "A validation contract can carry a command that runs nothing"]`. This session reproduced the identical failure mode against a throwaway crate to confirm the mechanism, not just the anecdote:
```
$ cargo test --lib rule::tests -- --exact
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out
```
`[VERIFIED: this session's own reproduction]`

**How to avoid:** Every command in this document's Validation Architecture below either (a) gives a substring naming one real, specific test function with no `--exact`, confirmed this session to select exactly that test, or (b) gives the full `module::tests::function_name` path *with* `--exact`. Neither form is copied from a source that was not itself run.

**Phase to address:** Every task's own verification step in the plan, not only the phase-level contract.

### Pitfall 4: An implicit accept (BASE-04)

**What goes wrong:** A convenience shortcut — "if no baseline exists yet, `compare --baseline NAME` just accepts the candidate as the new baseline" — silently violates BASE-04's second sentence. A missing baseline must be a loud error naming the missing name, never a silent first accept.

**How to avoid:** `BaselineStore::resolve` returns an error (not a fallback) when `name` has never been accepted; only the explicit `accept` subcommand ever calls `accept`.

**Warning signs:** A test that calls `compare --baseline` against a name that was never `accept`-ed and expects it to succeed.

**Phase to address:** This phase, in the `chrys-baseline` crate's own unit tests.

### Pitfall 5: An unaudited decode call site

**What goes wrong:** A future call site (the mask loader, or a later feature) opens `image::ImageReader::open` directly instead of going through `decode_guarded`, quietly reopening the CVE-2023-29408-class risk CLI-04 exists to close.

**How to avoid:** A static guard test, in the same style as `crates/chrys-core/tests/determinism.rs`'s `DENIED_DEPENDENCY_CRATES`/`FORBIDDEN_TRANSCENDENTALS` search (comment-and-string-stripped source, then a substring search), that walks every `.rs` file under `crates/*/src/` and asserts `image::ImageReader::open(` appears only inside the already-known, allow-listed call sites (`chrys-source-raster/src/decode.rs`, `chrys-source-animation/src/lib.rs`). `chrys-rule`'s mask loader calls the already-guarded `decode_guarded` function directly and therefore never trips this guard, but the guard is what makes that fact checked rather than assumed.

**Phase to address:** This phase, as its own dedicated test file (recommended: `crates/chrys-cli/tests/decode_limits_guard.rs`, since `chrys-cli` already depends on every adapter crate and mirrors `determinism.rs`'s own file-walking technique).

## Code Examples

### A malformed rule fails loudly and names the line (RULE-05, verified this session)

```
$ cargo run -q   # toml = "=1.1.5", serde = "=1.0.229", against a struct
                 # with #[serde(deny_unknown_fields)] and a rename_all-
                 # lowercase enum, matching Pattern 1's RuleRow shape
CASE3 error:
TOML parse error at line 2, column 13
  |
2 | tolerance = "high"
  |             ^^^^^^
invalid type: string "high", expected f64

---span=Some(27..33)
```
`[VERIFIED: this session's own compiled run]`

### A semantically wrong value on an otherwise valid key, pinpointed by field-level `Spanned<T>`

```rust
// Source: this session's own compiled and run test against toml = "=1.1.5".
use serde::Deserialize;
use toml::Spanned;

#[derive(Debug, Deserialize)]
struct Rule {
    kind: Spanned<String>,
    #[serde(default)]
    max_delta_e: Option<Spanned<f64>>,
}

#[derive(Debug, Deserialize)]
struct Doc { rule: Vec<Rule> }

fn line_of(text: &str, offset: usize) -> usize {
    text[..offset].matches('\n').count() + 1
}
```
Run against a two-rule document whose second rule sets `kind = "moved"` and (wrongly) `max_delta_e = 5.0` on line 7:
```
rule #0: kind="recoloured" at line 2
rule #1: kind="moved" at line 6
  SEMANTIC ERROR at line 7: max_delta_e does not apply to kind "moved" (value 5)
```
`[VERIFIED: this session's own compiled run — plain `toml::from_str`, no special deserializer entry point]`

### The existing hints sidecar already proves this contract, today

```rust
// Source: crates/chrys-source-raster/src/hints.rs:191-204, quoted verbatim
// this session. This is not a new pattern; it is the one this phase's
// rule reader repeats.
#[test]
fn invalid_toml_fails_with_a_message_naming_the_line() {
    let error = read_temp_sidecar("invalid-toml", "[[hint]\nname = \"logo\"\n")
        .expect_err("malformed TOML is refused");
    match error {
        RasterError::MalformedHints { message, .. } => {
            assert!(message.contains("line"), "message does not name a line: {message}");
        }
        other => panic!("expected RasterError::MalformedHints, got {other:?}"),
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Assume `toml::Spanned<T>` needs a dedicated deserializer entry point (as a first-pass documentation summary suggested) | Plain `toml::from_str` already populates `Spanned<T>` fields, verified this session by compiling and running it | Confirmed this session against the pinned `toml = "=1.1.5"` | The rule reader needs no special deserializer; `toml::from_str` alone, with `Spanned<T>` fields, gives exact-line semantic error reporting |

**Deprecated/outdated:** none specific to this phase beyond the corrected `Spanned` claim above.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Every `[[rule]]` table must require exactly one of `region` or `mask`, with no unscoped shape permitted at the type level | Pattern 1, RULE-04 | This is this session's own design recommendation, not a requirement quoted verbatim from REQUIREMENTS.md's exact wording (which describes the *shipped example*, not a hard schema constraint). If the planner or user prefers an optional global fallback for convenience, the type-level enforcement should be relaxed, but the shipped example must still never use it (RULE-04's literal text) |
| A2 | The majority-overlap rule (at least half a region's bbox area inside a hint or mask) is the right region/mask membership test | Pattern 2 | If wrong, a region could be misclassified as tolerated or as a violation near a scope boundary; low risk, since this is a policy knob easy to change later and does not affect the engine or the verdict itself |
| A3 | The report artifact should be TOML, not JSON | Pattern 5, Alternatives Considered | If a future consumer (e.g. a GitHub Action reading this report for a PR comment, v2's CI-01) strongly prefers JSON, the report module can be re-serialized without touching `chrys-rule`, `chrys-baseline`, or the engine — this is an isolated, low-risk choice |
| A4 | `AnimationSource` should set every frame's `source_name` to its own container file's name, not to a synthesized per-frame label | Pattern 5 | If wrong, the report's animation-frame labels are less specific than they could be; does not affect correctness, only report readability |
| A5 | The exit-code scheme in Pattern 4 (0/1/2/3) is the right mapping when `--rule` is given | Pattern 4, CLI-01 | This is a design recommendation, not sourced from an external convention; if the user wants a different code for "rule violation" specifically (distinct from the existing "changed" code 1), this is a one-line change isolated to `chrys-cli` |

## Open Questions

1. **Should the rule schema's type-level "no unscoped rule" enforcement (A1) be a hard parse-time refusal, or a softer lint that only the shipped example must honour?**
   - What we know: RULE-04's literal text constrains the *shipped example* file, not necessarily every rule file a user could write.
   - What's unclear: whether a future user legitimately wants a kind-only, unscoped tolerance (e.g. "tolerate any recolour anywhere, up to Δe 2, as a global anti-flicker allowance") without naming a region or mask.
   - Recommendation: enforce the stronger, type-level rule (no unscoped shape at all) unless the user, during planning or discuss-phase, explicitly wants the softer form. The stronger form is easier to loosen later than the reverse.

2. **Exact mask-membership semantics: does a mask pixel count by luminance threshold, by alpha, or by exact black/white match?**
   - What we know: the mask is decoded through the same guarded RGBA8 pipeline as any other raster input; nothing in this session established a convention for "which channel means tolerated."
   - What's unclear: whether the shipped example should ship a real mask fixture using a specific convention (e.g. white = tolerated, black = not, ignoring colour and alpha).
   - Recommendation: pick white (all three colour channels ≥ 250, ignoring alpha) as tolerated, matching the common convention in visual-regression tooling generally; document it plainly in the shipped example's own comment, since this is exactly the kind of convention this project's `AGENTS.md` says to write in plain, testable language rather than leave implicit.

## Environment Availability

No new external tool, service, or runtime dependency is introduced by this phase. Every new crate (`chrys-rule`, `chrys-baseline`) is pure Rust, depends only on already-pinned workspace crates, and needs no system library, no network service, and no additional CI runner beyond the six labels `.github/workflows/determinism.yml` already uses `[VERIFIED: .github/workflows/determinism.yml, read this session]`.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in), same as Phase 1 and Phase 2 — no new framework is introduced |
| Config file | none new |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo test --workspace --all-features` |
| Typical runtime | Not measured this session. No baseline number exists for this workspace's full suite yet; do not plan a timing budget around an invented figure. Measure once Wave 0's crates exist, and record the real number in that wave's own summary. |

### Sampling Rate

- **Per task commit:** `cargo test --workspace` (scoped to `-p <touched-crate>` is acceptable for a fast inner loop, but the full workspace command is what a task's own verification step should record, per this project's existing convention in `01-RESEARCH.md`/`02-RESEARCH.md`).
- **Per wave merge:** `cargo test --workspace --all-features`, **plus this phase's own engine-boundary gate**: `scripts/engine-boundary-drill.sh <wave-first-commit> <wave-last-commit>`. This script already exists, is already generic over any commit range, and needs no modification `[VERIFIED: scripts/engine-boundary-drill.sh:1-154, read in full this session]` — it is the exact mechanism Phase 2 built for its own SRC-08 claim, and this phase's headline architectural claim ("zero files under `crates/chrys-core/` change") is the identical shape of claim, checked the identical way. Every wave of this phase should record its own first/last commit hash and run this drill before the wave is considered merged, the same discipline `02-LEARNINGS.md` records for Phase 2's three format waves.
- **Phase gate:** full suite green, `scripts/engine-boundary-drill.sh` green across the *whole phase's* commit range (not only per-wave), and the six-runner determinism matrix still green (this phase adds no new fixture to that matrix unless a plan later decides the rule/baseline machinery needs its own cross-OS proof — see the note below) — all three, before `/gsd-verify-work`.

**Note on the determinism matrix:** this phase's own new code (`chrys-rule`, `chrys-baseline`, the CLI's new flags) does no floating-point comparison work of its own — it reads already-computed `Verdict`/`Region` values and does integer/rectangle arithmetic (Pattern 2) or file I/O (Pattern 3). It therefore is not expected to need a new cross-OS golden-hash fixture the way Phase 1's decode path or Phase 4's SVG rasterizer will. If a plan discovers a floating-point comparison inside the rule evaluator (e.g. the majority-overlap area calculation, which this research specifies in integer arithmetic precisely to avoid this), that discovery is itself a signal to route it through the same `libm`/integer discipline `01-RESEARCH.md` established, not to add a new determinism proof for it.

### Phase Requirements → Test Map

Every command below either names one real, specific test function as a `cargo test` substring filter with no `--exact` (confirmed this session to select exactly that one test and nothing else — see Pitfall 3), or names a full `module::tests::function` path *with* `--exact`. All test files are new (Wave 0); none exist today.

| Req ID | Behaviour | Test Type | Automated Command | File Exists? |
|--------|-----------|-----------|---------------------|-------------|
| RULE-01 | A rule sets tolerance scoped by `kind` (one of the five `ChangeKind` variants) | unit | `cargo test -p chrys-rule tolerance_is_scoped_by_change_kind` | ❌ Wave 0 |
| RULE-02 | A rule scoped to a named region matches a region overlapping that hint; a rule scoped to a mask matches a region overlapping the mask's tolerated pixels | unit | `cargo test -p chrys-rule region_scoped_rule_matches_overlapping_region` and `cargo test -p chrys-rule mask_scoped_rule_matches_overlapping_region` | ❌ Wave 0 |
| RULE-03 | The rule schema has no field that can hold a computed value; deserializing a rule row never executes anything (a static claim, tested by asserting the deserialized type is plain data — no `Fn`, no closure, no expression string) | unit | `cargo test -p chrys-rule rule_row_has_no_computable_field` | ❌ Wave 0 |
| RULE-04 | A rule table with neither `region` nor `mask` is refused at parse/validation time | unit | `cargo test -p chrys-rule an_unscoped_rule_is_refused` | ❌ Wave 0 |
| RULE-04 | The shipped `examples/rules/example.toml` file parses successfully and contains no unscoped rule | integration | `cargo test -p chrys-rule the_shipped_example_rule_file_parses_and_is_fully_scoped` | ❌ Wave 0 |
| RULE-05 | An unknown key in a rule file fails, and the message names the line | unit | `cargo test -p chrys-rule an_unknown_key_fails_naming_the_line` | ❌ Wave 0 |
| RULE-05 | A syntactically valid but semantically wrong field (e.g. `max_delta_e` on a `moved` rule) fails, naming the exact field's own line | unit | `cargo test -p chrys-rule a_tolerance_field_wrong_for_its_kind_fails_naming_the_line` | ❌ Wave 0 |
| BASE-01 | `accept` writes `MANIFEST.toml`, and it is not excluded by `.gitignore` | integration | `cargo test -p chrys-baseline accept_writes_a_manifest_toml_file` (the `.gitignore` claim itself was verified this session by reading the file directly, not by a test — see Sources) | ❌ Wave 0 |
| BASE-02 | `accept` copies the candidate file (or directory) verbatim into the store, byte for byte | integration | `cargo test -p chrys-baseline accept_copies_the_candidate_file_byte_for_byte` | ❌ Wave 0 |
| BASE-03 | `GoldenFileStore` is the only concrete type implementing `BaselineStore` referenced by `chrys-cli`, and the trait itself has exactly two methods | unit | `cargo test -p chrys-baseline the_trait_has_exactly_resolve_and_accept` | ❌ Wave 0 |
| BASE-04 | `resolve` on a name that was never accepted returns an error, never a silent accept | unit | `cargo test -p chrys-baseline resolving_an_unaccepted_name_is_an_error` | ❌ Wave 0 |
| CLI-01 | `compare --rule <path>` exits non-zero when a changed region has no tolerating rule, and exits zero when every changed region is tolerated | integration | `cargo test -p chrys-cli exits_nonzero_when_a_change_has_no_tolerating_rule` and `cargo test -p chrys-cli exits_zero_when_every_change_is_tolerated` (both via `Command::new(env!("CARGO_BIN_EXE_chrys"))`, the pattern already used in `crates/chrys-cli/tests/digest.rs:8-24`, verified this session) | ❌ Wave 0 |
| CLI-02 | `compare --report <path>` writes a TOML file naming every changed region's kind, region (if any) and size | integration | `cargo test -p chrys-cli the_report_names_kind_region_and_size_per_change` | ❌ Wave 0 |
| CLI-03 | `compare <base> <candidate>` with no new flags prints the verdict and exits as it did before this phase (regression guard on already-existing behaviour) | integration | `cargo test -p chrys-cli a_second_run_gives_byte_identical_stdout` — **already exists today**, at `crates/chrys-cli/tests/digest.rs:82` `[VERIFIED: crates/chrys-cli/tests/digest.rs, `grep -n "fn "` output this session]` | ✅ exists |
| CLI-04 | Every `image::ImageReader::open(` call site in the workspace is one of the already-known, allow-listed sites | unit (static guard, mirroring `determinism.rs`'s own technique) | `cargo test -p chrys-cli only_allow_listed_call_sites_open_an_image_reader_directly` | ❌ Wave 0 |

### Manual-Only Verifications

| Verification | Why it cannot be automated |
|---------------|------------------------------|
| The shipped `examples/rules/example.toml` reads clearly to a person unfamiliar with the schema | RULE-04's own criterion is about a human reading a PR, not a parseable property; `the_shipped_example_rule_file_parses_and_is_fully_scoped` (above) proves the file is *valid and scoped*, not that its comments and structure are *legible* |
| A `MANIFEST.toml` diff, in an actual pull request view (e.g. GitHub's diff renderer), reads as "this baseline's hash changed" without opening the binary image | This is a claim about a PR review UI's rendering, not about this repository's own code; the closest automated proxy is asserting the diff is a small, single-line change (already covered by BASE-01/BASE-02's tests above), but "reads clearly to a person" itself needs a human to look |

### The risks this phase carries

A contract that lists only what passes is not a contract. Two risks are structural to this phase, not incidental:

1. **A rule file is new, structured input read as data, but it is still attacker-reachable input if a repository's rule file is ever supplied by an untrusted contributor (e.g. a rule file changed in a pull request from a fork, then read by a CI job with write access to the baseline).** RULE-03's "no rule is computed or executed" is the mitigation in principle; the risk this phase must not silently reintroduce is a rule field that, in a future revision, gains an "execute this" escape hatch (a shell-out, a glob pattern reaching outside the repository, a mask path that traverses `../` outside the project root). The mask-path field in particular should be checked against path traversal (`mask` resolves relative to the rule file's own directory and must not escape it) — this is not yet a requirement ID, but it is the concrete form RULE-03's promise takes once a real file-path field exists in the schema. **Recommendation:** the plan should add an explicit test asserting a `mask = "../../etc/passwd"`-shaped path is refused, not silently canonicalized and read.

2. **A baseline that a person accepts is a place where a wrong acceptance becomes the new truth, and nothing downstream can tell the difference between a correct accept and a mistaken one.** BASE-04 ensures no *automatic* update happens, but it does not, by itself, protect against a person running `accept` against a candidate that itself already contains the regression it was meant to catch (e.g. accepting a broken render as the new golden by habit, without looking). This is a process risk, not a code defect, and this research does not recommend building an approval-gate feature for it in Phase 3 (out of scope; the native window's own accept/reject flow in Phase 6 is the actual UI-level mitigation for this, per the roadmap). **Recommendation:** the `accept` subcommand's own CLI help text and the shipped example documentation should say, plainly, that `accept` is a statement "this is now correct," not a neutral bookkeeping operation — the same discipline `AGENTS.md`'s own "What to measure" section asks for elsewhere in this project (measure and name the thing you actually mean, not something adjacent to it).

## Security Domain

`security_enforcement` is on (ASVS level 1, block on high) per `.planning/config.json` `[VERIFIED: .planning/config.json, read this session: "security_enforcement": true, "security_asvs_level": 1, "security_block_on": "high"]`. This phase adds a new local file-read surface (the rule file, the mask image, the baseline manifest) and a new local file-write surface (the baseline store's `accept`), all still local-file-only: no network surface, no authentication, no session state, consistent with Phase 1 and Phase 2's own framing.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture | Yes (narrowly) | The rule reader and the baseline store are new crates outside `chrys-core`, preserving the same trait-boundary discipline Phase 1/2 already established as a compile-time fact, not a convention |
| V5 Input Validation | Yes | The rule file's mask path must not escape the rule file's own directory (see "The risks this phase carries," item 1); the mask decode reuses `DecodeLimits` (CLI-04); the baseline manifest parse uses the same `deny_unknown_fields` discipline as the rule reader and the existing hints sidecar |
| V5 Input Validation | Yes | A malformed rule file or a malformed manifest fails loudly (RULE-05) rather than being silently ignored or partially applied |
| V6 Cryptography | Yes (narrowly) | The baseline manifest's digests reuse `chrys_core::hash::rgba8_digest` (SHA-256 via the already-audited `sha2` crate); no new hashing code is written |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A rule file's `mask` field pointing outside the project directory (`../../etc/passwd`, or an absolute path to a sensitive file) | Tampering / Information Disclosure | Resolve `mask` relative to the rule file's own directory and refuse a path that canonicalizes outside it, mirroring the refuse-rather-than-guess precedent already used for `RegionOutOfBounds` `[VERIFIED: crates/chrys-source/src/lib.rs:96-121]` |
| A crafted mask image with a huge declared size, forcing unbounded allocation before any pixel is read | Denial of Service | The same `DecodeLimits`/`decode_guarded` mitigation already applied to every other raster decode in this project, reused unchanged (CLI-04) |
| A hand-edited `MANIFEST.toml` claiming a digest that does not match the stored file | Tampering | Out of scope for `resolve()` itself (which trusts the directory listing over the manifest, per Pattern 3) to fix in this phase, but the plan should add a test proving `resolve()` never silently trusts a manifest digest that disagrees with the file it stored — worth a task-level check, not a blocking gate |

## Sources

### Primary (HIGH confidence)

- `crates/chrys-core/src/verdict.rs`, `hash.rs`, `lib.rs`, `classify/kind.rs`, `crates/chrys-source/src/lib.rs`, `crates/chrys-source-raster/src/lib.rs`, `decode.rs`, `normalize.rs`, `hints.rs`, `crates/chrys-cli/src/main.rs`, `crates/chrys-source-animation/src/lib.rs`, `crates/chrys-source-sequence/src/lib.rs`, `crates/chrys-core/tests/determinism.rs`, `crates/chrys-core/src/sequence.rs`, `scripts/engine-boundary-drill.sh`, `Cargo.toml`, `.gitignore`, `.github/workflows/determinism.yml`, `.planning/config.json` — all read in full or in relevant part this session, with line ranges quoted above
- `cargo search toml --limit 1` and `cargo search serde --limit 1` against the live crates.io registry, this session
- A throwaway crate at a temporary path, compiled and run this session with `toml = "=1.1.5"` and `serde = { version = "=1.0.229", features = ["derive"] }`, to verify: (a) `deny_unknown_fields` + a `rename_all = "lowercase"` enum produces a line-and-column-numbered, caret-annotated error for an unknown key, an invalid enum variant, and a wrong type; (b) `toml::Spanned<T>` on individual struct fields populates correctly through plain `toml::from_str`, with no special deserializer, pinpointing the exact line of a semantically wrong field; (c) `cargo test <module>::tests -- --exact` selects zero tests and exits 0, while a substring naming the real test function selects exactly that test — all output quoted verbatim above
- `.planning/phases/01-raster-engine-and-determinism-proof/01-RESEARCH.md`, `01-LEARNINGS.md`, `.planning/phases/02-source-trait-and-a-second-format/02-RESEARCH.md`, `02-LEARNINGS.md` — this project's own prior research and phase record, read in full this session

### Secondary (MEDIUM confidence)

- [docs.rs/toml/1.1.5/toml/de/struct.Error.html](https://docs.rs/toml/1.1.5/toml/de/struct.Error.html) — `message()`/`span()`/`set_input()` method list and the documented `Display` example, corroborated (and, for `Spanned`, corrected) by this session's own compiled run
- [docs.rs/clap/4.6.6/clap/struct.Arg.html](https://docs.rs/clap/4.6.6/clap/struct.Arg.html) — `required_unless_present`, fetched this session, used for the recommended `compare --baseline` positional-vs-flag design; not independently compiled and run this session

### Tertiary (LOW confidence)

- [docs.rs/toml/1.1.5/toml/struct.Spanned.html](https://docs.rs/toml/1.1.5/toml/struct.Spanned.html) — this session's own `WebFetch` summary of this page was wrong about whether plain `toml::from_str` populates spans; retained here only as a record of what was corrected, not as a source to trust going forward
- The exact mask-membership convention (white = tolerated) named in Open Questions 2 — a reasoned recommendation, not fetched from any visual-regression tool's own documentation this session

## Metadata

**Confidence breakdown:**
- Rule-file error reporting (RULE-05): HIGH — verified by compiling and running real code against the exact pinned versions, not by reading documentation
- In-repo architecture facts (what `chrys-core` already exposes, what `Frame`/`RegionHint` already carry, what the CLI already does): HIGH — every claim is grounded in a direct `Read` of the exact file this session, with line numbers
- Crate placement and the "zero `chrys-core` changes" claim: HIGH — directly derived from invariant 1's own wording plus a verified inventory of `chrys-core`'s already-public surface; independently testable via the already-existing `scripts/engine-boundary-drill.sh`
- Rule-file schema field names, baseline manifest shape, CLI flag names, exit-code scheme: MEDIUM — this session's own design recommendation, clearly flagged as such in the Assumptions Log, not fetched from an external convention or a locked user decision (no `CONTEXT.md` exists for this phase at research time)
- Mask-membership semantics and the exact overlap threshold: MEDIUM-LOW — a reasoned default, flagged in Open Questions for confirmation

**Research date:** 2026-09-07
**Valid until:** 30 days for crate versions (re-verify at plan time, per this project's own established habit); 90 days for the architecture, schema-design and pitfalls material (grounded in source code read this session and in this session's own compiled verification, stable until the code itself changes)
