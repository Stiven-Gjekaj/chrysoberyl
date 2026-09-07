---
phase: "2"
slug: "source-trait-and-a-second-format"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-09-07"
---

# Phase 2 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

This file was written after the phase executed. The register is not new work:
all five plans carried a `<threat_model>` block, so every threat below was
authored before the code it covers. This audit reads the built tree and records
what it measured. Where a mitigation text no longer describes the code, that is
stated rather than corrected in silence.

Verification depth is ASVS level 1: a grep-level check that each named control
is present in the tree, plus a live run of the test suite and the drills. No
deeper trace was performed, and none is claimed.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| directory listing to frame list | An attacker-controlled directory decides how many files the process opens and in what order. | directory entries, file metadata |
| many files to one comparison | A sequence multiplies phase 1's per-file allowance by the file count. The bomb changes shape from one large file to a thousand small ones. | cumulative pixel count |
| container header to frame count | An animated container declares its own frame count. That claim is attacker-controlled. | GIF, APNG and WebP container fields |
| file content to decoder selection | The decoder is chosen from the content signature, never from the file name. | file signature |
| sidecar TOML to crop rectangle | An untrusted document names a rectangle the tool then crops to. | region name, origin, size |
| sidecar structs to engine graph | A `derive(Deserialize)` in the wrong crate would put `serde` inside the engine's own dependency graph. | crate manifest edges |
| commit range to boundary check | An empty result is the pass condition, and a wrong range also produces an empty result. | two commit ids |
| six runner reports to the agree job | The comparison logic was rewritten in this phase, so its earlier evidence does not carry over. | six digest reports |

---

## Threat Register

Twenty-three distinct threats. `T-02-SC` is one supply-chain threat carried by
every plan with a per-plan mitigation; it is listed once.

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-02-01 | Denial of Service | `SequenceSource::load` | high | mitigate | `max_frames` 512 and `max_total_pixels` 134,217,728, checked as frames accumulate. Verified: `chrys-source-sequence/src/lib.rs:28, 31, 37, 38`. | closed |
| T-02-02 | Denial of Service | per-file decode in the sequence adapter | high | mitigate | Every file decodes through `decode_guarded`. Measured: **zero** `ImageReader::open` sites in `chrys-source-sequence`, so the guard has no second call site to bypass. | closed |
| T-02-03 | Tampering | directory listing | medium | mitigate | Non-recursive `read_dir`, and only entries whose metadata reports a regular file are kept. Verified: `sequence.rs:24, 39`. A file that does not decode fails the whole load rather than being skipped. | closed |
| T-02-04 | Tampering | `compare_sequence` index pairing | medium | mitigate | Unequal counts refuse with `RefusalReason::FrameCountMismatch` naming both counts, rather than pairing a prefix. Verified: `chrys-core/src/sequence.rs:105`. | closed |
| T-02-05 | Tampering | `expected-digest.sha256` | medium | mitigate | The file carries a header stating that a change to any of its four lines is a behaviour change a commit message must name. Verified by reading the file. **See Deviations: the residual line was regenerated in phase 1's gap closure.** | closed |
| T-02-06 | Repudiation | the one-pipeline guard | medium | mitigate | The defect was planted by hand in a disposable worktree and the red run and green run recorded in `02-02-SUMMARY.md`. A guard whose failure has never been observed proves nothing. | closed |
| T-02-07 | Information Disclosure | `--hash-only` output | low | accept | The digest report covers raw RGBA8 bytes and the verdict text only. No path, timestamp or host. See Accepted Risks R-05. | closed |
| T-02-08 | Denial of Service | `AnimationSource::load` | high | mitigate | `max_frames` 512 and `max_total_pixels` 134,217,728 checked as `into_frames()` yields, not after the iterator drains, so allocation stops at the limit rather than at the container's claim. Verified: `chrys-source-animation/src/lib.rs:45, 48, 57, 58`. | closed |
| T-02-09 | Denial of Service | decoder construction | high | mitigate | `to_image_limits()` applied to every decoder before a frame is pulled. Verified at all three branches: GIF `set_limits` line 184, APNG `PngDecoder::with_limits` line 193, WebP `set_limits` line 235. This is the CVE-2023-29408 class mitigation carried onto the animation path. | closed |
| T-02-10 | Spoofing | `sniff::is_animation` | medium | mitigate | The format comes from `open_guessed`, which reads the content signature, never the extension. Verified: `sniff.rs:20-25`. A `.png` holding a GIF cannot pick a decoder the bytes do not call for. | closed |
| T-02-11 | Tampering | the hand-assembled ANMF container | high | mitigate | The generator decodes every file it writes back through `WebPDecoder` and aborts on disagreement, in memory before a byte reaches disk. Verified: `chrys-source-animation/examples/make-fixtures.rs:170-193`, `round_trip_check`. New unreviewed container code cannot write evidence a later phase trusts without passing its own decode. | closed |
| T-02-12 | Repudiation | the animated WebP determinism claim | medium | mitigate | Only lossless frames are committed, and the generator header records that the lossy path is untested. The proof is not silently widened past what was measured. | closed |
| T-02-13 | Repudiation | the engine-boundary check | high | mitigate | An empty result is both the pass condition and what a wrong range produces. `scripts/engine-boundary-drill.sh` plants a defect and asserts the check produced output naming the planted file. | closed |
| T-02-14 | Tampering | the commit range the check reads | medium | mitigate | The range is passed as two arguments rather than embedded, and the script prints the range it checked, so the summary records what was measured rather than what was intended. | closed |
| T-02-15 | Repudiation | the `agree` job comparison | high | mitigate | The comparison was rewritten in this phase to a whole-file byte comparison, so a fixture added later needs no edit. A local run over two files differing in one character proved it goes red, recorded in `02-04-SUMMARY.md`. | closed |
| T-02-16 | Elevation of Privilege | the CI workflow | medium | mitigate | Measured on the current tree: `pull_request_target` count **0**, `permissions: contents: read` present, and **0** `uses:` lines that are not pinned to a 40-character hash. Phase 1's posture is unchanged by this phase. | closed |
| T-02-17 | Denial of Service | `engine-boundary-drill.sh` | low | mitigate | Refuses a dirty tree (`git status --porcelain`, line 46), builds a disposable worktree with `mktemp -d` (line 55), removes it through `trap cleanup EXIT INT TERM` (line 63). | closed |
| T-02-18 | Denial of Service | `Frame::crop_to_region` | high | mitigate | Checked arithmetic, returning `RegionOutOfBounds` rather than panicking or clamping. Verified: `chrys-source/src/lib.rs:59-72`, two `checked_add` calls covering the origin-plus-size overflow. | closed |
| T-02-19 | Tampering | `read_hints_sidecar` | medium | mitigate | `deny_unknown_fields` on both structs (`hints.rs:20, 31`), so a misspelled key is a loud failure rather than a silent default that crops the wrong rectangle. A duplicate region name is refused (`hints.rs:86`). | closed |
| T-02-20 | Denial of Service | TOML parse of an untrusted document | medium | mitigate | Parsed with `toml =1.1.5`, pinned (`Cargo.toml:22`). A malformed document returns an error carrying a line position; it does not panic. A second hand-written parser would be a second parser to secure. | closed |
| T-02-21 | Elevation of Privilege | the engine's dependency graph | medium | mitigate | `chrys-source` declares no `serde` and no `toml` (measured: zero matches in its manifest). The sidecar structs live in the raster adapter. `chrys_source_declares_no_dependency` (`chrys-source/tests/manifest.rs:18`) makes that a red test rather than a convention, for a direct declaration in that one crate. Measured on the engine graph today: **0** image, serde, toml, gif, png, webp, tiff or wgpu crates across 48 crates. **The transitive guard does not cover serde or toml. See Deviations.** | closed |
| T-02-22 | Information Disclosure | the unknown-region error message | low | accept | The message lists the region names the sidecars declare. Those names are already in a file the same caller can read. See Accepted Risks R-06. | closed |
| T-02-SC | Tampering | crates.io installs | low | accept | `natord =1.0.9`, `png =0.18.1`, `toml =1.1.5`, `serde =1.0.229`, and the `gif` feature of the already-pinned `image =0.25.10`. All carry an `OK` verdict in `02-RESEARCH.md`'s Package Legitimacy Audit with a registry age, a download count and a source repository. Every version is pinned with `=` and `Cargo.lock` is committed. See Limitations. | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` (high) count toward `threats_open`*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Deviations from the register as written

One mitigation text no longer describes what happened. It is recorded rather
than corrected in silence.

**T-02-05 says the four digest values are "typed from this plan, not generated
from the binary under test", so the fixture cannot be regenerated into
agreement with a regression.** Phase 1's gap-closure plan `01-09` regenerated
the `residual` line of that file from the binary. The control was not bypassed:
`01-09` predicted in advance which line would move and which three would not,
gated each prediction on the value measured before the change, and stopped the
task on any move outside that table. The three lines the plan said would hold
did hold, including the `verdict` line at
`33142d19a0b218aea7528f4e6312e7eef7655bd7fbdb4c68799734e3dd2ed823`. The intent
of T-02-05 is that a digest must never be quietly re-baselined into agreement
with a regression, and that intent held. The literal wording no longer does,
because one line was regenerated under a plan that named it first.

**T-02-20's mitigation does not cover the size of the document it parses.**
The threat says a malformed TOML document returns an error rather than
panicking, and that holds. It says nothing about how large the document may be,
and `read_hints_sidecar` reads it with `std::fs::read_to_string`
(`hints.rs:65`), which has no cap. Measured on 2026-09-07: a 314,572,802-byte
sidecar drives peak resident set size to 318,324,736 bytes, because the whole
file is read before the parser is called. Every other decode path in this
project bounds its allocation explicitly. This one does not. The code review
records this as CR-01 and it is carried into `02-VERIFICATION.md` as a gap, not
resolved here.

**T-02-21's mitigation is narrower than the invariant it serves.** The threat
is that `derive(Deserialize)` on `RegionHint` would put `serde` into
`chrys-source` and therefore into the engine's graph. That specific threat is
guarded: `chrys-source/tests/manifest.rs` line-scans that crate's own
`Cargo.toml`, and it is proven able to go red on a planted dependency. But the
project invariant is wider, and reads "chrys-core declares no format crate, no
GPU crate, no serde and no toml". The transitive guard that reads the resolved
tree, `DENIED_DEPENDENCY_CRATES` in `chrys-core/tests/determinism.rs`, lists
twenty-one crates, all of them GPU or format. Measured: it names neither
`serde` nor `toml`. So `serde` arriving in `chrys-core` through some path other
than `chrys-source`'s own manifest is caught by nothing. The threat is closed;
the invariant is half guarded. The code review records this as WR-02.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-05 | T-02-07 | The digest report names no path, no timestamp and no host. It reports raw pixel bytes and verdict text, which is the contract `hash.rs` already states. Narrowing it further would remove the evidence the report exists to carry. | Stiven Gjekaj | 2026-09-07 |
| R-06 | T-02-22 | The unknown-region message lists the declared region names so a person who mistyped one learns the available names. Those names sit in a sidecar file the same caller already reads, so nothing is disclosed that the caller does not have. | Stiven Gjekaj | 2026-09-07 |
| R-07 | T-02-SC | Five package additions, each with a registry age, a download count and a named source repository recorded in `02-RESEARCH.md`. All pinned with `=`. Accepted on that evidence, not on an advisory scan, because none was run. See Limitations. | Stiven Gjekaj | 2026-09-07 |

---

## Limitations of this audit

Stated so no reader credits this file with more than it did.

- **Depth is ASVS level 1.** Each control was confirmed present by reading the
  tree and running the suite. No end-to-end trace and no boundary-placement
  analysis was performed.
- **No CVE database was consulted.** `cargo-audit` and `cargo-deny` are not
  installed on this machine, measured directly. T-02-SC rests on the registry
  evidence in `02-RESEARCH.md` and on pinned `=` versions with a committed
  `Cargo.lock`, not on an advisory scan.
- **The lossy animated WebP path is untested**, by design. T-02-12 records
  that the determinism claim covers lossless frames only.
- **The 48 crates in the engine graph were not each re-audited here.** The
  count and the category absence are measured; the per-crate verdicts come
  from `01-RESEARCH.md` and `02-RESEARCH.md`.
- **Two findings from the code review are open, not closed here.** The
  unbounded sidecar read and the transitive guard's missing `serde` and `toml`
  entries are recorded in Deviations above and carried to the verification
  report. This audit measured them; it did not fix them.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-07 | 23 | 23 (18 mitigated, 5 accepted) | 0 | orchestrator, ASVS L1 |

Live evidence collected on 2026-09-07 against commit `27f34ff`:

- 184 of 184 tests pass workspace-wide, zero failures.
- `cargo tree -p chrys-core -e normal`: 48 crates, **0** matching image,
  serde, toml, gif, png, webp, tiff or wgpu.
- `#![forbid(unsafe_code)]` present in all 6 crates.
- `pull_request_target` occurrences: 0. Unpinned `uses:` lines: 0.
- `ImageReader::open` sites in `chrys-source-sequence`: 0.
- `serde` or `toml` in `chrys-source`'s manifest: 0.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-07
