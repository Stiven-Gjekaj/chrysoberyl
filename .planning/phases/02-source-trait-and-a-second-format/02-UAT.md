---
status: complete
phase: 02-source-trait-and-a-second-format
source: 02-01-SUMMARY.md, 02-02-SUMMARY.md, 02-03-SUMMARY.md, 02-04-SUMMARY.md, 02-05-SUMMARY.md
started: 2026-09-07T01:00:00Z
updated: 2026-09-07T01:00:00Z
---

## Current Test

[testing complete]

Tests 1 to 11 were run by the orchestrator against the built binary. Every one is
mechanically observable. Test 12 is a judgment a measurement cannot settle.

## Tests

### 1. A numbered frame sequence pair compares frame by frame
expected: eleven frames, each reported, the changed one named
result: pass
observed: eleven frame blocks; index 4 (`frame5.png`) reports the recolour; exit 1

### 2. Natural sort orders the sequence correctly
expected: `frame2.png` before `frame10.png`
result: pass
observed: the recolour lands at index 4. Lexicographic order would have put
`frame10.png` and `frame11.png` before `frame2.png` and moved it.

### 3. A GIF pair compares frame by frame
expected: five frames, one change named
result: pass
observed: frame 3 reports `Recoloured region at x=43, y=43, width=50, height=40,
colour delta 77.99`; exit 1

### 4. An APNG pair compares frame by frame
result: pass
observed: exit 1, same five-frame shape

### 5. An animated WebP pair compares frame by frame
result: pass
observed: exit 1, same five-frame shape

### 6. A named region narrows the comparison
expected: a change outside the region is not reported when the region is named
result: pass
observed: whole frame exits 1; `--region logo` on the same pair exits 0 and reports
`identical`

### 7. Phase 1 still works unchanged
expected: the raster path and the four committed digests are untouched
result: pass
observed: exit 1 on the phase-1 pair, exit 0 on an identical pair, and
`tests/golden/pair-01/expected-digest.sha256` matches live output

### 8. The engine did not change
expected: no file under `crates/chrys-core/` changed across waves 3, 4 and 5
result: pass
observed: `git rev-parse HEAD:crates/chrys-core` reads
`63aad81ddee9939047b1436a33eed8f0896da409` at every one of those wave boundaries, and
`git diff --name-only` lists no engine file

### 9. Nothing entered the engine's dependency graph
expected: no `serde`, no `toml`, no format, graphics or GPU crate
result: pass
observed: `cargo tree -p chrys-core -e normal` matches zero of them

### 10. The determinism guards still pass
result: pass
observed: 6 of 6 guards green; 175 workspace tests pass; clippy clean under
`-D warnings`; `cargo fmt --check` clean

### 11. Six runners agree on all five fixture families
expected: identical digests across three operating systems and two architectures
result: pass
observed: GitHub Actions run on `f493fb0`, all six digest jobs plus `agree` plus
`guards` green, over `pair-01`, `sequence-01`, `gif`, `apng` and `webp-anim`

### 12. The per-frame output is readable at length
expected: a person scanning a hundred-frame sequence can find the changed frames
result: issue
reported: "Fail and record."
severity: major
observed: measured on a built hundred-frame pair in which five frames differ. The
tool prints 200 lines: 100 `frame N` headers, 95 bare `identical` lines, and 5 lines
that carry a verdict. 195 of 200 lines are noise. A person scanning the output reads
a wall of `identical`. The exit code is 1, which is correct.

The information is recoverable with a second tool. `grep -B1 -E
'^(Recoloured|Moved|Added|Removed|Resized)'` returns `frame 6, 22, 23, 60, 87`
cleanly. A tool that is only readable at length through another tool does not meet
the expectation as written, and a sequence is the input family this phase exists to
support.

Two further observations from the same run, recorded so they are not lost:

- The output numbers frames from zero while the files are named `frame001` to
  `frame100`, so the printed `frame 6` is the file `frame007.png`. The off-by-one
  between the report and the directory is a second reading cost.
- Frames 22 and 23 print the same verdict text twice with nothing marking them as
  one run of adjacent changes. A regression in a real animation usually spans
  several frames, so this is the common case, not an edge case.

## Summary

total: 12
passed: 11
issues: 1
pending: 0
skipped: 0
blocked: 0

## Gaps

- gap_id: G-02-12
  truth: "A person scanning a hundred-frame sequence can find the changed frames."
  status: failed
  reason: "User reported: Fail and record. Measured on a hundred-frame pair with five
    changed frames: 200 lines out, of which 5 carry a verdict and 195 do not. The
    changed frames are only findable by piping the output through grep."
  severity: major
  test: 12
  artifacts:
    - path: "crates/chrys-cli/src/main.rs"
      issue: "The multi-frame path prints a frame header and a verdict line for every
        frame, including every identical one, with no summary line and no quiet mode."
    - path: "crates/chrys-cli/src/main.rs"
      issue: "The printed frame number starts at zero while the files are named from
        one, so the report and the directory disagree by one."
  missing:
    - "Print only the frames that changed by default, and close with a count of the
      frames that did not."
    - "Keep the per-frame detail available behind a flag, so nothing that reads the
      current shape loses it."
    - "Make the printed frame number agree with the name of the file it describes."
    - "Decide whether a run of adjacent frames carrying the same verdict prints once
      or once per frame."
  deferred_to: "phase 3"
  deferred_reason: >
    This is a reporting change, not an engine change. Phase 3 owns CLI-01 through
    CLI-04, including the report artifact that names each change by kind, region and
    size. Fixing the shape here and again there would write it twice. Phase 3's
    success criteria carry it, and ROADMAP.md records it.

Two limits recorded so they are not mistaken for coverage:

- Lossy animated WebP determinism is unproven. Every animated WebP fixture in this
  phase is lossless, because `image-webp` composites an alpha blend imprecisely even
  at full opacity. The claim is not widened.
- The engine-boundary check proves no file changed. It does not prove the adapter
  could not have needed a change. That the animation wave never asked for one is
  evidence, not proof.
