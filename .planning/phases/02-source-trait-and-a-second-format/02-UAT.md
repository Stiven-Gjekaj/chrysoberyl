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
result: pass
observed: this test failed first, on 2026-09-07, and the shape was changed. The
same hundred-frame pair with five changed frames printed 200 lines, of which 5
carried a verdict. It now prints 11 lines, and every one of them carries a change:

    frame 6
    Recoloured region at x=34, y=34, width=16, height=16, colour delta 104.80 (base [200, 120, 40, 255], candidate [40, 120, 200, 255])
    frame 22
    Recoloured region ...
    frame 23
    Recoloured region ...
    frame 60
    Recoloured region ...
    frame 87
    Recoloured region ...
    5 of 100 frames changed

Frames 22 and 23 read as one run, because no unchanged frame sits between them
any more. The exit code stays 1.

Three shapes are unchanged and are held by their own tests: `--all-frames`
prints the same 200 lines as before, `--hash-only` reports every frame because
the digest report is the determinism evidence, and a single pair prints neither
a header nor a tail.

One reading cost from the first run is not fixed here, and is not a defect in
this shape. The printed number is the frame's own index, which starts at zero,
while the fixture files are named from one. No fixed offset can be correct,
because the producer names the files: a sequence named from zero would then
disagree instead. The fix is for the report to name the file, which needs the
frame to carry its name, and phase 3 owns the report artifact. Carried as an
observation below, not as a gap.

## Summary

total: 12
passed: 12
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

None. Test 12 failed on the first run and the shape was changed in the same
phase, so no gap is carried forward. The commit that changed it names the
measurement.

Two limits recorded so they are not mistaken for coverage:

- Lossy animated WebP determinism is unproven. Every animated WebP fixture in this
  phase is lossless, because `image-webp` composites an alpha blend imprecisely even
  at full opacity. The claim is not widened.
- The engine-boundary check proves no file changed. It does not prove the adapter
  could not have needed a change. That the animation wave never asked for one is
  evidence, not proof.
- The printed frame number is a zero-based index, not the name of the file it
  describes. A reader of a sequence named from one has to subtract one. Phase 3
  owns the report artifact and is where the frame gains a name.
