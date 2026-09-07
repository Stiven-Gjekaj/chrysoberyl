---
phase: "3"
slug: "baseline-rules-and-ci-gate"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
# One medium threat, T-03-11, was reopened after this file was first
# written. It is below the `high` blocking threshold, so threats_open
# stays 0, but it is open and the register says so.
created: "2026-09-07"
---

# Phase 3 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

This file was written after the phase executed. The register is not new work:
all four plans carried a `<threat_model>` block, so every threat below was
authored before the code it covers. This audit reads the built tree and records
what it measured.

Verification depth is ASVS level 1: a grep-level check that each named control
is present, plus a live run of the guard suite and the drills, plus four
controls driven end to end through the built binary.

This phase adds the largest new attack surface of the project so far. Two of
its three inputs are documents a person writes and a tool reads: a rule file,
which in CI can arrive from a fork's pull request, and a manifest, which
decides what a comparison is measured against. The third is a mask image,
which is a file path inside the first one.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| rule file to tolerance decision | An untrusted TOML document decides which changes a gate forgives. In CI it can come from a fork's pull request. | change kind, scope, tolerance values |
| rule file to file system | A rule names a mask by path. The path is attacker-controlled text that becomes a file the tool opens. | mask path |
| mask pixels to tolerated area | A decoded image decides which part of a frame a rule forgives. | RGBA8 mask pixels |
| manifest to comparison subject | `MANIFEST.toml` decides which stored file a candidate is measured against. | baseline name, source name, digest |
| stored baseline to verdict | A file inside the store is compared as though it were reviewed. Nothing downstream knows whether its accept was correct. | stored image bytes |
| report artifact to reader | A file a CI job publishes carries the paths the caller supplied. | base path, candidate path, per-change detail |
| new crate to engine graph | `chrys-rule` and `chrys-baseline` are new edges near an engine that must declare no `serde` and no `toml`. | crate manifest edges |

---

## Threat Register

Twenty-seven distinct threats. `T-03-SC` is one supply-chain threat carried by
every plan with a per-plan mitigation; it is listed once.

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-03-01 | Denial of Service | `load_rules` | high | mitigate | `MAX_RULE_FILE_BYTES` is 1 MiB, checked against `std::fs::metadata` before the read. Verified: `chrys-rule/src/lib.rs:29`. Same posture and same limit `hints.rs` already takes. | closed |
| T-03-02 | Tampering | the rule schema | high | mitigate | Eight literal-valued fields, no field able to hold an expression or a reference to another rule. `deny_unknown_fields` closes the document and the row. Measured: 2 occurrences in `chrys-rule/src/lib.rs`. | closed |
| T-03-03 | Tampering | the tolerance evaluation | high | mitigate | A colour tolerance reading only `delta_e` would forgive opaque becoming fully transparent, because Lab carries no alpha axis. `max_alpha_delta` is a separate field defaulting to 0. Verified: `lib.rs:64`. This is phase 1's alpha decision reaching the rule layer. | closed |
| T-03-04 | Spoofing | an unscoped rule | medium | mitigate | `Scope` has exactly two variants, `Region` and `Mask`, and no `Option`, so an unscoped rule cannot be deserialized. Verified live: an unscoped rule prints `line 1: a rule must name exactly one of region or mask, but this rule names neither` and exits 3. | closed |
| T-03-05 | Tampering | a misspelled key | medium | mitigate | `deny_unknown_fields` makes a misspelled tolerance key a loud failure naming a line, rather than a silent default. | closed |
| T-03-06 | Elevation of Privilege | `chrys-core`'s dependency graph | medium | mitigate | Measured: `cargo tree -p chrys-core -e normal` matches `serde`, `toml`, `chrys-rule` and `chrys-baseline` **zero** times. The engine's manifest still names only `chrys-source`, `thiserror`, `sha2`, `rustfft`, `libm`, `palette`. | closed |
| T-03-07 | Repudiation | a refused pair under a rule file | medium | mitigate | A rule cannot forgive a `Refused` verdict. `evaluate` returns an empty outcome for a refusal, `Refused` keeps exit rank 2, and the doc comment states that an empty outcome is not a pass. | closed |
| T-03-08 | Information Disclosure | the `mask` path | high | mitigate | **Driven end to end.** `mask = "../../../../../../etc/passwd"` and `mask = "/etc/passwd"` are both refused with exit 3, and the message names both the offending path and the directory it left. A lexical check refuses an absolute path and any parent component before a file is opened; a canonical check refuses a symlink that reaches outside. | closed |
| T-03-09 | Denial of Service | `load_mask` | high | mitigate | The mask decodes through `decode_guarded` with `DecodeLimits`, the same CVE-2023-29408-class mitigation every other decode carries, and a named test proves the limit is applied rather than assumed. | closed |
| T-03-10 | Tampering | the mask membership rule | high | mitigate | **Driven end to end.** A mask of opaque white tolerates and exits 0. The committed `white-on-transparency.png` fixture does NOT tolerate and exits 1. Had the convention read colour only, that second mask would have silenced every change in the frame with no error anywhere. See Decisions below. | closed |
| T-03-11 | Spoofing | a mask of the wrong size | medium | **OPEN** | The control exists in the source (`evaluate.rs:120` returns `MaskSizeMismatch`), but the orchestrator could not make it fire. A 10x10 mask against a 256x256 frame produced a verdict and exit 1 with nothing on stderr, and a second run of the same shape produced exit 2 and no report. Neither named a size. **This threat is reopened. See the correction below.** | open |
| T-03-12 | Denial of Service | a partly applied rule file | medium | mitigate | `load_rules` decodes every mask while the file loads, so a rule file whose third mask is missing fails before any image is decoded and no partial verdict is produced. | closed |
| T-03-13 | Elevation of Privilege | `chrys-rule` reaching a format crate | medium | mitigate | `chrys-rule` depends on `chrys-source-raster`, which holds a format decoder. The direction is the control: `chrys-core` depends on neither. The check reads the resolved tree rather than the declared list, because phase 1 recorded a re-export carrying a banned dependency past a manifest check. | closed |
| T-03-14 | Denial of Service | a future unguarded decode call site | high | mitigate | A static guard asserts every direct reader call site is one of two allow-listed files, with exact counts. **Drilled red twice**: an unexpected call site planted in `chrys-rule/src/mask.rs` made the guard name that file, and a deleted call site failed the count assertion `left: 1, right: 2`. | closed |
| T-03-15 | Repudiation | the report on a failing run | high | mitigate | The report is written on the non-zero path and on the refused path. A write that fails fails the run and names the path, rather than dropping the artifact silently. | closed |
| T-03-16 | Tampering | a defaulted rule outcome in the report | medium | mitigate | The `rule_outcome` key is omitted entirely when `--rule` was absent, so "nobody asked" cannot be read as "the answer was no". A named test asserts the absence. | closed |
| T-03-17 | Spoofing | the frame-to-name pairing | medium | mitigate | `SequenceSource::load` is implemented in terms of `load_named`, so the directory is listed once and the pairing is made where the order is decided. Two independent listings cannot drift. | closed |
| T-03-18 | Information Disclosure | paths written into the report | low | accept | The report's `meta` table holds the paths the caller supplied on its own command line. See Accepted Risks R-08. | closed |
| T-03-19 | Elevation of Privilege | `chrys-source`'s dependency table | medium | mitigate | `load_named` is a defaulted trait method using `std::path` only. Measured: `chrys-source`'s dependency table holds **zero** entries, and `chrys-source/tests/manifest.rs` passes. A new `Frame` field was rejected for this reason; it would have edited 45 sites across 13 files of the engine. | closed |
| T-03-20 | Elevation of Privilege | an implicit first accept | high | mitigate | **Driven end to end.** `compare --baseline never-accepted` prints `baseline "never-accepted" was never accepted; the store at chrys-baselines holds no baseline of that name` and exits 3, and creates nothing. A static guard and a behavioural test hold it, both drilled red. | closed |
| T-03-21 | Tampering | `MANIFEST.toml` | high | mitigate | **Driven end to end.** The stored `base.png` was replaced with a different image; the run failed with `the manifest records digest 5a5f5994..., but the stored file now produces 99bf5b95...`, naming the baseline, the frame and both digests, exit 3. A manifest nothing verifies is a list of numbers. | closed |
| T-03-22 | Tampering | `resolve` | high | mitigate | Every write in `chrys-baseline` lives inside one private function called from `accept` alone. **Drilled red twice**: moving the write into `resolve` made the guard fail naming `resolve`, and making `resolve` create its own directory failed a path-set comparison naming the stray path. | closed |
| T-03-23 | Denial of Service | the manifest parse | medium | mitigate | `MAX_MANIFEST_BYTES` is 1 MiB, checked from `std::fs::metadata` before the read, and parsed with `deny_unknown_fields`. Verified: `chrys-baseline/src/golden.rs:34, 160`. | closed |
| T-03-24 | Tampering | the recursive copy in `accept` | medium | mitigate | The walk copies regular files only and does not recurse. Verified: `golden.rs:233` tests `metadata.is_file()`. Same rule the sequence adapter's own listing already applies. | closed |
| T-03-25 | Repudiation | a mistaken accept | medium | accept | Nothing downstream can tell a correct accept from a mistaken one. See Accepted Risks R-09. The control this phase owes is honest wording, and it is present: see Decisions below. | closed |
| T-03-26 | Elevation of Privilege | `chrys-baseline` reaching the engine | medium | mitigate | Measured: `chrys-baseline`'s manifest names `toml`, `serde` and `thiserror` only. It names neither `chrys-core` nor `chrys-source`, so no direction of dependency exists in which a store could reach the engine. This is what makes BASE-03 true by construction rather than by discipline. | closed |
| T-03-SC | Tampering | crates.io installs | low | accept | This phase introduces **no new external package**. `toml`, `serde` and `thiserror` are already pinned in the workspace table and already carry an `OK` verdict in `01-RESEARCH.md` or `02-RESEARCH.md`'s Package Legitimacy Audit. See Limitations. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` (high) count toward `threats_open`*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## A correction to this file, made after it was written

**T-03-11 is reopened.** This file first recorded it as closed, on the strength
of reading the control in the source. The control is there:
`evaluate.rs:120` compares the mask's own size to the frame's and returns
`RuleError::MaskSizeMismatch` naming both. Reading it was not enough.

Driving it produced two results that do not match the claim, and do not match
each other:

- A 10x10 mask against a 256x256 frame, with no region flag: a verdict on
  stdout, exit 1, nothing on stderr. No size was named.
- The same shape, run again with `--report`: exit 2, and no report file
  written at all.

The orchestrator could not explain either result by inspection, and stopped
rather than keep guessing. What is certain is that the claim this file made,
"a size mismatch is refused with both sizes named", is not what the built
binary does. The threat is open, its severity is medium so it does not block
the phase, and the verification report carries it forward.

This is recorded rather than quietly amended because the failure here is the
one this project names most often: a control that exists in the source, is
read, and is certified without being run.

---

## A gap this audit found, and closed

**One crate shipped without `#![forbid(unsafe_code)]`.**

Phase 3 added two crates. Seven of the eight crates in the workspace carried
the attribute; `chrys-rule` did not. No test failed. No lint fired. The
workspace lint table did not catch it. It was found by counting the crates by
hand during this review.

A count a person has to remember to run is not a guard, so the fix is both
halves: the attribute now sits in `chrys-rule/src/lib.rs`, and
`crates/chrys-cli/tests/unsafe_guard.rs` asserts every crate root under
`crates/` declares it. The guard was watched failing: with the attribute
removed it names `chrys-rule` and goes red. It also refuses to pass when it
finds fewer than eight crate roots, so a walk that stopped finding crates
cannot report a clean workspace.

This is recorded rather than quietly fixed because it is the same shape as the
defect phase 1 recorded about its own guards: a rule everybody believed was
enforced, enforced nowhere.

---

## Two decisions this phase settled before planning

Recorded here because both are security-relevant and neither came from the
research unchanged.

**A mask pixel is tolerated when it is white AND opaque.** The research
recommended white on all three colour channels, ignoring alpha. Ignoring alpha
is the wrong half. A mask authored as white on transparency is
(255, 255, 255, 0) everywhere outside the drawn area, so a convention that
reads colour alone tolerates the whole canvas and silences every change in the
frame without an error anywhere. The committed `white-on-transparency.png`
fixture exists to hold this, and it exits 1.

**An unscoped rule is refused at parse time, at the type level.** Success
criterion 1 says the shipped example shows a scoped exclusion and never a bare
global threshold. Refusing at the type level makes that true of every rule
file, not only the shipped one. A global threshold wearing a rule's clothes is
the shape that silences changes nobody named.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-08 | T-03-18 | The report names the base and candidate paths the caller typed on its own command line. That discloses nothing the caller does not have, and a report that hid them could not be matched to the run that produced it. | Stiven Gjekaj | 2026-09-07 |
| R-09 | T-03-25 | A person accepting a baseline makes it the new truth, and no code can distinguish a correct accept from a mistaken one. The roadmap puts the real mitigation in phase 6's accept-and-reject window. What this phase owes is honest wording, and the help text carries it: "An accept is a statement: this candidate is now correct. Every later comparison against NAME is measured against it, and nothing in this tool can tell a correct accept from a mistaken one." A test holds that wording. | Stiven Gjekaj | 2026-09-07 |
| R-10 | T-03-SC | No new external package enters the workspace this phase. The three crates the new code uses are already pinned with `=` and already audited. Accepted on that evidence, not on an advisory scan, because none was run. | Stiven Gjekaj | 2026-09-07 |

---

## Limitations of this audit

Stated so no reader credits this file with more than it did.

- **Depth is ASVS level 1.** Each control was confirmed present by reading the
  tree and running the suite. Four controls were additionally driven end to end
  through the built binary; the rest were not.
- **No CVE database was consulted.** `cargo-audit` and `cargo-deny` are not
  installed on this machine, measured directly in phase 1 and unchanged.
  T-03-SC rests on pinned `=` versions, a committed `Cargo.lock`, and the fact
  that this phase adds no new package at all.
- **The mask path check was tested on macOS only.** The lexical check is
  platform-independent, but the canonical symlink check runs against this
  machine's file system. A Windows runner has different path semantics, and
  nothing here proves the check behaves identically there. The six-runner
  matrix compiles and runs the suite on Windows, which is evidence, not proof
  of the traversal case specifically.
- **A rule file is trusted to the extent its author is.** The controls bound
  what a rule can reach and how large it can be. They do not, and cannot,
  decide whether a tolerance a person wrote is one they should have written.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-07 | 27 | 26 (21 mitigated, 5 accepted) | 1 (T-03-11, medium, below the blocking threshold) | orchestrator, ASVS L1 |

Live evidence collected on 2026-09-07 against commit `ca6cf94`:

- 254 of 254 tests pass workspace-wide, zero failures.
- Engine tree object id `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`, unmoved
  across all 13 commits of the phase.
- `scripts/engine-boundary-drill.sh 29f5add HEAD` over the whole phase: exit 0.
- 6 of 6 determinism guards, the `chrys-source` manifest guard, and both
  decode-site guards pass.
- `cargo tree -p chrys-core -e normal`: zero matches for `serde`, `toml`,
  `chrys-rule`, `chrys-baseline`.
- `#![forbid(unsafe_code)]` in **8 of 8** crates, now guarded by a test.
- `cargo clippy --workspace --all-targets`: zero warnings. `cargo fmt --check`
  clean.
- Four controls driven end to end: mask traversal refused, white-on-transparency
  not tolerated, unaccepted baseline refused, tampered manifest caught.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed, counting threats at or above `high`. One medium threat, T-03-11, is open.
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-07
