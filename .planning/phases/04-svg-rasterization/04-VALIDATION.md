---
phase: "4"
slug: "svg-rasterization"
status: validated
nyquist_compliant: true
wave_0_complete: false
created: "2026-09-08"
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

Seeded from the `## Validation Architecture` section of `04-RESEARCH.md`,
before the planner runs, and the body below is that section verbatim.

The order is the point. Phase 1 shipped with no validation contract, because
the researcher was never asked for the section and the blocking gate for the
missing file did not fire. Phase 3 needed a correction sent mid-run for the
same omission. Phase 4's brief asked for it from the start.

`nyquist_compliant` stays `false` until the plans exist and the checks run.

---

## The claim this phase must not assume

Success criterion 3 is a falsification test, not a formality.

The research rates cross-platform bit-identity MEDIUM confidence, and says
why: resvg's README states it, but this session did not execute the six-runner
matrix, and `tiny-skia` ships SIMD (SSE2, AVX2, NEON) enabled by default.
Phase 1 already recorded that run-time CPU dispatch is the exact shape of
defect this project treats as fatal, and it pinned `FftPlannerScalar` for that
reason.

So the phase carries one unproven claim, and the gate is the proof. If the
matrix disagrees, the documented fallback is to disable `tiny-skia`'s `simd`
feature and measure again. If it still disagrees, the honest outcome is to
withdraw the claim in writing rather than ship it, which is what this project
did for lossy animated WebP in phase 2.

A green matrix on one machine is not evidence here. Only the six runners are.

---


### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in), unchanged from every prior phase |
| Config file | none new |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo test --workspace --all-features` |

### Sampling Rate

- **Per task commit:** `cargo test --workspace` (or `-p chrys-source-svg` for the inner loop; the full workspace command is what a task's own verification step should record, per this project's established convention).
- **Per wave merge:** `cargo test --workspace --all-features`, plus `scripts/engine-boundary-drill.sh <wave-first-commit> <wave-last-commit>` — this phase's own headline architectural claim ("`chrys-core`'s tree object id does not move") is the identical shape of claim Phase 2 and Phase 3 already proved with this same, already-existing, already-generic script.
- **Phase gate:** full suite green, the engine-boundary drill green across the whole phase's commit range, **and** the six-runner determinism matrix green on the new SVG fixture specifically (not merely on the four pre-existing fixture families) — all three, before `/gsd-verify-work`. The SVG fixture's own matrix result is this phase's actual exit gate for the font/curve determinism claim (criterion 3), distinct from and in addition to the pre-existing fixtures' continued agreement.

### Phase Requirements → Test Map

| Req ID | Behaviour | Test Type | Automated Command | File Exists? |
|--------|-----------|-----------|---------------------|-------------|
| SRC-04 | An SVG pair with no differences compares `Identical` | unit | `cargo test -p chrys-source-svg identical_svgs_produce_identical_frames` | ❌ Wave 0 |
| SRC-04 | An SVG pair with a real visual difference (e.g. a recoloured shape) compares `Changed`, at the same quality (a named kind, a bounding box) as a raster pair | integration | `cargo test -p chrys-cli an_svg_pair_reports_a_recoloured_region` (via `Command::new(env!("CARGO_BIN_EXE_chrys"))`, the pattern already used in `crates/chrys-cli/tests/digest.rs`) | ❌ Wave 0 |
| SRC-04 | `named_frames_for` routes an `.svg`-content file (regardless of its extension) through `SvgSource`, not `RasterSource` | unit | `cargo test -p chrys-cli an_svg_file_with_no_extension_still_dispatches_to_svg_source` | ❌ Wave 0 |
| DET-05 | `resvg`'s `system-fonts` Cargo feature is disabled workspace-wide | static (manifest guard) | `cargo test -p chrys-cli only_the_text_feature_of_resvg_is_enabled` — a guard reading `Cargo.lock`/`cargo tree -e features` output, mirroring `02-LEARNINGS.md`'s own "A new format's feature flag stops at the adapter" verification style | ❌ Wave 0 |
| DET-05 | The workspace source tree names `load_system_fonts` nowhere | static (source guard) | `cargo test -p chrys-core --test determinism` (extend the existing determinism guard file, mirroring `DENIED_DEPENDENCY_CRATES`'s own string-search technique, verified this session by reading `crates/chrys-core/tests/determinism.rs`'s own pattern) — *or*, if this guard is judged to belong nearer the crate it protects, `cargo test -p chrys-source-svg no_source_file_calls_load_system_fonts` | ❌ Wave 0 |
| DET-05 | A rendered glyph's pixels come only from the pinned font: a `<text>` naming an unrelated family still renders (falls back to the pinned family, Pattern 2), rather than rendering empty | unit | `cargo test -p chrys-source-svg an_unmatched_font_family_falls_back_to_the_pinned_font` | ❌ Wave 0 |
| SRC-04 + DET-05 (criterion 3) | A text-bearing SVG fixture's raw RGBA8 hash is identical on Linux, macOS and Windows | CI-only (six-runner matrix, extended) | `cargo run -q --release -p chrys-cli -- compare tests/golden/formats/svg/base.svg tests/golden/formats/svg/candidate.svg --hash-only`, appended to `.github/workflows/determinism.yml`'s existing `digest` job, checked by the existing, unmodified `agree` job's whole-file `cmp -s` | ❌ Wave 0 (CI workflow extension) |
| (adapter-internal, not a numbered requirement) | An oversized SVG (declared width/height, or raw file size, above `SvgLimits`) is refused with a typed error, not a panic or unbounded allocation | unit | `cargo test -p chrys-source-svg an_oversized_declared_canvas_is_refused` and `cargo test -p chrys-source-svg an_oversized_file_is_refused_before_parsing` | ❌ Wave 0 |
| (adapter-internal) | `Frame.pixels` from an `SvgSource` is straight alpha, verified against a fixture with a known partially-transparent pixel | unit | `cargo test -p chrys-source-svg alpha_is_straight_not_premultiplied` | ❌ Wave 0 |

### Manual-Only Verifications

| Verification | Why it cannot be automated |
|---------------|------------------------------|
| The shipped `tests/golden/formats/svg/base.svg`/`candidate.svg` fixture actually contains legible text, rendered with the pinned font, when opened as a PNG by a person | A hash match proves consistency across runners, not that the rendering itself looks like the text a person authored (Pitfall 2); a person needs to look once, the same "reads clearly to a person" caveat `03-RESEARCH.md`'s own Manual-Only Verifications table already records for its shipped rule-file example |

### The risks this phase carries

1. **The cross-architecture SIMD question (A1) is the phase's own unproven claim, carried forward exactly as the roadmap's own "three phases carry an unproven claim forward" framing anticipates for Phase 4, 5 and 7.** Unlike Phase 1's `FftPlanner` (where a scalar-only alternative was known and adopted before any CI run), this phase's recommended default is to run the matrix with SIMD *on* first, because that is the configuration every real resvg/tiny-skia user actually runs and the one this project's own dependency pin will build by default; only fall back to a scalar-only tiny-skia build if the matrix falsifies the vendor's own claim. **Recommendation:** the plan should treat "the matrix disagreed and `simd` had to be disabled" as an equally acceptable, equally documented outcome, not a failure of the plan — the phase brief itself says withdrawing a claim in writing is preferable to shipping one that cannot be proven.
2. **A font pinned today is a font this repository owns forever, including its own licence's own obligations.** SIL OFL 1.1 permits bundling and redistribution freely, but it is not public domain: the licence text itself (`OFL.txt`) must be committed alongside the font file, verbatim, the same way this project already treats every other externally-sourced artifact's provenance as something to record, not merely to use. **Recommendation:** the plan's own font-pinning task should commit `OFL.txt` in the same commit as the font file, per this project's own "code and its tests in the same commit" discipline read one level more broadly (an asset and its licence, together).

---

## Validation Sign-Off

- [x] All tasks have an automated verify or a Wave 0 dependency. 8 of 8 tasks, 68 automated commands.
- [x] Sampling continuity: no 3 consecutive tasks without an automated verify. Maximum 2 per wave.
- [x] Wave 0 covers every missing reference.
- [x] No watch-mode flags. Zero matches.
- [ ] Feedback latency measured. Not done. It needs a green run of a suite that does not exist yet.
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-09-08, on the measured checks above.

## What the orchestrator measured before execution

Independently, against the plan text, not taken from the checker:

- 68 automated commands, 68 failure conditions, one to one.
- **0** of them use `--exact`.
- **14** carry a name filter, where a typo silently selects nothing and exits
  0. All 14 guard against `0 passed`. That is the defect phase 2 shipped,
  closed here by construction.
- **0** entries under `crates/chrys-core/` in any plan's `files_modified`.
- The engine tree object id is asserted 25 times across the three plans.
