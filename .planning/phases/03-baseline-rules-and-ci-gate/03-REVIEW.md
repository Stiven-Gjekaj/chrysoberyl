---
phase: 03-baseline-rules-and-ci-gate
reviewed: 2026-09-07T00:00:00Z
depth: standard
files_reviewed: 23
files_reviewed_list:
  - crates/chrys-rule/src/lib.rs
  - crates/chrys-rule/src/mask.rs
  - crates/chrys-rule/src/evaluate.rs
  - crates/chrys-rule/tests/rule_file.rs
  - crates/chrys-rule/tests/mask.rs
  - crates/chrys-rule/Cargo.toml
  - crates/chrys-baseline/src/lib.rs
  - crates/chrys-baseline/src/golden.rs
  - crates/chrys-baseline/tests/store.rs
  - crates/chrys-baseline/Cargo.toml
  - crates/chrys-cli/src/main.rs
  - crates/chrys-cli/src/report.rs
  - crates/chrys-cli/tests/rule_gate.rs
  - crates/chrys-cli/tests/baseline.rs
  - crates/chrys-cli/tests/report.rs
  - crates/chrys-cli/tests/decode_limits_guard.rs
  - crates/chrys-cli/tests/unsafe_guard.rs
  - crates/chrys-cli/Cargo.toml
  - crates/chrys-source/src/lib.rs
  - crates/chrys-source-sequence/src/lib.rs
  - crates/chrys-source-raster/src/lib.rs
  - examples/rules/example.toml
  - chrys-baselines/MANIFEST.toml
findings:
  critical: 2
  warning: 1
  info: 1
  total: 4
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-09-07T00:00:00Z
**Depth:** standard
**Files Reviewed:** 23
**Status:** issues_found

## Summary

`chrys-rule`'s parsing, scope validation and mask logic are solid: the
byte-size ceiling runs before any read, `deny_unknown_fields` closes the
schema, `Scope` genuinely cannot represent an unscoped rule at the value
level, the mask path check runs a lexical pass before a canonical pass in
the right order, and the white-and-opaque membership test (`>= 250` on
every colour channel AND `== 255` alpha, never colour alone) is exactly
right, with tests that pin the boundary in both directions. All integer
arithmetic in `evaluate.rs` and `mask.rs` (area, overlap, majority-pixel
count) is `u64`, with no floating point outside the one already-computed
`f32` comparison `Tolerance::Colour` performs, matching D-04. `chrys-baseline`
correctly depends on neither `chrys-core` nor `chrys-source`, `resolve` is
provably read-only (both by inspection and by `tests/store.rs`'s own
before/after filesystem snapshot test), and `chrys-cli`'s
`verify_baseline_digests` recomputes and compares every frame's digest
against `MANIFEST.toml` before trusting a resolved baseline.

Two defects earned a Critical rating. First, `chrys-cli`'s `run_compare`
shadows `base_frames`/`candidate_frames` with the region-cropped versions
before computing rule outcomes and the report, which silently feeds the
rule engine and the report writer frames whose `hints` are always empty
and whose width/height are the cropped rectangle, not the frame's own
size — this is exactly the "`--region` combined with `--rule`" interaction
the phase's own review brief asks to be checked, and it was not caught by
any existing test. Second, `GoldenFileStore::write_baseline` deletes the
previous baseline directory before the new one is fully written and before
`MANIFEST.toml` is rewritten, so an I/O failure partway through (a full
disk, a permission change, a failed `fs::copy` on a later frame of a
multi-frame candidate) can leave the store holding neither the old,
complete baseline nor the new one. A smaller, defense-in-depth gap in the
mask path resolver and one overstated doc comment round out the findings.

## Critical Issues

### CR-01: `--region` silently breaks rule evaluation and reporting when combined with `--rule` or `--report`

**File:** `crates/chrys-cli/src/main.rs:238-321`

**Issue:**

```rust
let (base_names, base_frames) = named_frames_for(base_path)?;
let (_candidate_names, candidate_frames) = named_frames_for(candidate_path)?;
...
let (base_frames, candidate_frames) = match region {
    Some(name) => (
        crop_frames_to_region(&base_frames, name, base_path)?,
        crop_frames_to_region(&candidate_frames, name, candidate_path)?,
    ),
    None => (base_frames, candidate_frames),
};

let verdicts = chrys_core::compare_sequence(&base_frames, &candidate_frames)?;

let outcomes: Option<Vec<chrys_rule::RuleOutcome>> = rules.as_ref().map(|rules| {
    verdicts.iter().enumerate().map(|(index, verdict)| {
        let hints = base_frames.get(index).map(|frame| frame.hints.as_slice()).unwrap_or(&[]);
        let frame_size = base_frames.get(index).map(|frame| (frame.width, frame.height)).unwrap_or((0, 0));
        chrys_rule::evaluate(verdict, hints, rules, frame_size)
    }).collect::<Result<Vec<_>, _>>()
}).transpose()?;
```

The `let (base_frames, candidate_frames) = match region { ... }` binding
shadows the original, uncropped `base_frames`. `chrys_source::Frame::crop_to_region`
documents plainly that a cropped frame "carries an empty `hints` list,
because a cropped frame's coordinate space is not the one the source
frame's hints were written in," and its width/height become the cropped
rectangle's, not the source frame's.

When `--region NAME` is combined with `--rule RULE.toml` (both flags are
independent and nothing prevents combining them):

- Every `Scope::Region(name)` rule can never match any region again,
  because `hints` is always `&[]` after the shadowing —
  `overlapping_hint_name` immediately returns `None` for every candidate
  hint name. A rule an author wrote and expected to tolerate a change now
  always reports `Violation`, silently making `--region` + `--rule`
  stricter than the rule file says, with no error or warning at all.
- Every `Scope::Mask(path)` rule is checked against `frame_size` taken
  from the *cropped* frame instead of the frame the mask image was
  authored against. If the mask's own dimensions happen to equal the
  region's cropped dimensions (a realistic authoring choice — a mask
  drawn specifically for the region being checked), `MaskSizeMismatch` is
  never raised, and `mask.tolerates(&region.bbox)` is evaluated using
  `region.bbox` coordinates in the cropped-frame coordinate space against
  mask pixels indexed in a different coordinate space, silently
  tolerating or rejecting the wrong pixels. This is a rule-engine
  correctness defect with a plausible path to a false "tolerated" verdict
  on a CI gate whose entire purpose is deciding which changes are
  forgiven.

The same shadowed `base_frames` also feeds the `--report` writer
(`hints` passed into `report::frame_report` at line ~302), so a report
written together with `--region` always reports `region: None` for every
change, even though `chrys_rule::overlapping_hint_name` is the mechanism
the report explicitly documents using.

No test in `crates/chrys-cli/tests/` exercises `--region` together with
`--rule` or `--report`; `rule_gate.rs` and `report.rs` only exercise each
flag in isolation.

**Fix:** Keep the original, uncropped frames alive under their own names
for hint/size lookups, and use a separate binding for the frames that are
actually compared pixel-for-pixel:

```rust
let (compared_base_frames, compared_candidate_frames) = match region {
    Some(name) => (
        crop_frames_to_region(&base_frames, name, base_path)?,
        crop_frames_to_region(&candidate_frames, name, candidate_path)?,
    ),
    None => (base_frames.clone(), candidate_frames.clone()),
};

let verdicts = chrys_core::compare_sequence(&compared_base_frames, &compared_candidate_frames)?;

// hints / frame_size below read from the ORIGINAL `base_frames`, not
// `compared_base_frames`, so a region-scoped or mask-scoped rule keeps
// seeing the frame it was actually written against.
let hints = base_frames.get(index).map(|frame| frame.hints.as_slice()).unwrap_or(&[]);
let frame_size = base_frames.get(index).map(|frame| (frame.width, frame.height)).unwrap_or((0, 0));
```

At minimum, until the coordinate-space question is resolved, `run_compare`
should refuse the combination of `--region` with `--rule` or `--report`
with a loud error, rather than silently producing a wrong outcome.

## CR-02: `write_baseline` is not atomic; a failure mid-write can destroy the previous baseline

**File:** `crates/chrys-baseline/src/golden.rs:266-361`

**Issue:** `write_baseline` performs, in order: (1) `fs::remove_dir_all`
on the *existing* `baseline_dir` if a prior acceptance exists, (2)
`fs::create_dir_all` on a fresh `baseline_dir`, (3) one `fs::copy` per
candidate file, (4) `read_manifest` + `fs::write` of the rewritten
`MANIFEST.toml`. None of steps 2-4 are staged in a temporary location and
renamed into place; every step writes directly to the final path.

If any step after (1) fails — `create_dir_all` fails on a permission or
disk-space error, one `fs::copy` in a multi-frame directory candidate
fails partway through the loop (e.g. the second of three frames), or
`read_manifest`/`fs::write` on `MANIFEST.toml` fails after the files were
already copied — the function returns `Err`, but the previously accepted
baseline directory has already been deleted in step (1) and the new one
is only partially written. The store is left holding neither the old,
complete baseline (BASE-04's whole reason for existing: "nothing in this
tool can tell a correct accept from a mistaken one" presumes the *prior*
correct accept survives a failed attempt) nor a new, complete one. This is
a genuine data-loss path: a maintainer's last-known-good golden file for a
name can be destroyed by an `accept` that itself fails and reports an
error, with no way to recover it from the store itself (only from version
control, if the store is even committed at that revision).

The manifest-digest check in `chrys-cli`'s `verify_baseline_digests` will
eventually surface a resulting inconsistency on the *next* `compare
--baseline`, but by then the original accepted content is already gone;
the check does not prevent the loss, it only detects it after the fact.

**Fix:** Stage the new baseline in a sibling temporary directory, write
the new `MANIFEST.toml` content to a temporary file, and only then
atomically replace the old state (`fs::rename` on the same filesystem for
both the directory and the manifest), so any failure before the final
rename leaves the previous, complete baseline exactly as it was:

```rust
let staging_dir = root.join(format!(".{name}.accept-tmp"));
fs::create_dir_all(&staging_dir)?;
// ...copy every candidate file into staging_dir...
let manifest_tmp = root.join("MANIFEST.toml.tmp");
fs::write(&manifest_tmp, serialized)?;

// Only now touch the paths a reader or a future accept will see.
if baseline_dir.is_dir() {
    fs::remove_dir_all(&baseline_dir)?;
}
fs::rename(&staging_dir, &baseline_dir)?;
fs::rename(&manifest_tmp, &manifest_path)?;
```

A failure at any point before the final two renames now leaves the store
byte-for-byte as it was before `accept` was called.

## Warnings

### WR-01: `resolve_mask_path` checks the canonical path but returns and later opens the non-canonical one

**File:** `crates/chrys-rule/src/mask.rs:162-193`

**Issue:** `resolve_mask_path` canonicalizes both `rule_dir` and the
joined mask path, checks `canonical_mask.starts_with(&canonical_dir)`,
and then returns `joined` — the pre-canonicalization path — which
`load_rules` immediately passes to `mask::load_mask`, which opens it
through `chrys_source_raster::decode::decode_guarded`. Between the
canonical check and the actual open, `joined` is re-resolved by the OS a
second time; if any path component of `joined` (most plausibly the mask
file itself) is replaced with a symlink pointing outside `rule_dir`
between those two points, the safety check that already ran is bypassed
by the open that follows it. The doc comment's own stated intent
("Neither check falls back to reading the file... or canonicalizes first
and asks afterwards") is undermined by returning the un-canonicalized
path rather than the one that was actually verified.

In this project's primary threat model (a rule file arriving from a
checked-out pull request in CI), there is no attacker-controlled process
running concurrently with `chrys` to perform the swap, so this is a
defense-in-depth gap rather than a directly exploitable one today. It is
still the kind of check-then-use gap that becomes exploitable the moment
this crate is used anywhere with less controlled concurrency (a long-lived
service, a shared build agent), and it costs nothing to close.

**Fix:** Return the already-verified canonical path, not the joined one:

```rust
if !canonical_mask.starts_with(&canonical_dir) {
    return Err(RuleError::MaskPathEscapesDirectory { ... });
}

Ok(canonical_mask)
```

## Info

### IN-01: Doc comment overstates "type-level" refusal of an unscoped rule

**File:** `crates/chrys-rule/src/lib.rs:84-99`, `crates/chrys-rule/src/lib.rs:405-420`

**Issue:** `Scope`'s doc comment states an unscoped rule "is refused at
parse time, at the type level, not by a lint (D-01)." In fact,
`RuleRow` (the `serde`-deserialized intermediate struct `validate_row`
consumes) has both `region: Option<Spanned<String>>` and
`mask: Option<Spanned<PathBuf>>`, and `toml::from_str` happily
deserializes a `RuleRow` with both `None` — deserialization does not fail.
The actual refusal is a semantic check performed by `validate_row` after
deserialization succeeds (`(None, None) => Err(RuleError::MissingScope
{...})`). The end result — an unscoped rule can never reach a `Rule`
value, and it fails before any comparison runs — is correct and well
tested, but the claim that this is enforced "at the type level" at parse
time is not accurate for the deserialization step itself; it is `Scope`
(the *post-validation* type) that cannot represent the missing case, not
`RuleRow`. A future maintainer relying on the literal claim might assume
no code path can ever observe a `RuleRow` with both fields absent, when
in fact `validate_row` must (and does) handle exactly that.

**Fix:** Reword to be precise about which type carries the guarantee,
e.g.: "An unscoped rule cannot be represented as a `Scope` value — the
type this crate hands to the evaluator. `validate_row` is the one place
that converts the permissive, `serde`-deserialized `RuleRow` (where both
`region` and `mask` may be absent) into that guarantee, refusing before
any comparison runs (D-01)."

---

_Reviewed: 2026-09-07T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
