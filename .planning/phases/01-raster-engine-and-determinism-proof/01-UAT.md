---
status: partial
phase: 01-raster-engine-and-determinism-proof
source: 01-01-SUMMARY.md, 01-02-SUMMARY.md, 01-03-SUMMARY.md, 01-04-SUMMARY.md, 01-05-SUMMARY.md, 01-06-SUMMARY.md, 01-07-SUMMARY.md, 01-08-SUMMARY.md
started: 2026-09-07T00:00:00Z
updated: 2026-09-07T00:00:00Z
---

## Current Test

[testing paused — 2 items need a person]

Tests 1 to 11 were run by the orchestrator against the built binary, because every
one of them is mechanically observable. Tests 12 and 13 are judgment calls that a
measurement cannot settle, and they stay open for Stiven.

## Tests

### 1. A change is named by kind, never as a bare pixel count
expected: `chrys compare` on the golden pair prints a named kind, not a count
result: pass
observed: `Recoloured region at x=64, y=64, width=96, height=64, colour delta 111.83 (base [40, 90, 200, 255], candidate [200, 90, 40, 255])`

### 2. A colour change reports a difference value and both colours
expected: the verdict carries a numeric difference and the two colours
result: pass
observed: `colour delta 111.83 (base [40, 90, 200, 255], candidate [200, 90, 40, 255])`

### 3. A region carries a bounding box
expected: the verdict names a position and a size
result: pass
observed: `x=64, y=64, width=96, height=64`

### 4. A raster pair compares in all four formats
expected: PNG, JPEG, WebP and TIFF all produce a verdict
result: pass
observed: all four report the same region; JPEG's delta differs slightly, which is
correct for a lossy encoder

### 5. A translated region reports its offset in pixels
expected: a shifted pair reports how far it moved
result: pass
observed: `Moved region at x=40, y=40, width=50, height=40, moved by (-2, 1)` on
`refuse-01/should-register/pair-08`

### 6. A new region is named as added
expected: content present in one frame only is named, not described as a colour change
result: pass
observed: `Added region at x=224, y=224, width=6, height=6` on
`refuse-01/should-register/pair-07`

### 7. An unregisterable pair is refused, with a reason
expected: the tool declines and says why, rather than returning a verdict it cannot support
result: pass
observed: `refused: the pair is too different to register: peak confidence 976.82 is
below the threshold 2313.88; this engine compares near-identical pairs only`, exit 2

### 8. An identical pair is not refused
expected: the same file twice reports identical, not a refusal
result: pass
observed: `identical`, exit 0. This is the direction a zero-floor defect would have
broken, and plan 5 records finding and fixing exactly that.

### 9. Hostile and malformed input fails safely
expected: a corrupt, truncated, empty or missing file is reported, not crashed on
result: pass
observed: four cases, each with a specific message on stderr and exit 3. Invalid PNG
signature, unexpected end of file, unexpected end of file, and No such file or
directory.

### 10. The exit code carries the verdict
expected: a caller can branch on the exit code
result: pass
observed: 0 identical, 1 differs, 2 refused, 3 unreadable

### 11. The engine holds its boundaries
expected: no GPU crate, no format crate, and the determinism guards pass
result: pass
observed: zero GPU or format crates in `cargo tree -p chrys-core`; 5 of 5 determinism
guards pass; 21 of 21 classification tests pass; six CI runners plus the drill job
green on this commit

### 12. The verdict line is useful to a reviewer
expected: a person reading one line knows what changed and whether it matters
result: [pending]
reason: only a person can judge whether `Recoloured region at x=64, y=64, width=96,
height=64, colour delta 111.83` is the sentence they want, or whether it needs
different fields, a different order, or a different name for the kind.

### 13. The refusal message earns its trust
expected: a person who sees a refusal understands it and does not simply raise the threshold
result: [pending]
reason: the wording is the whole defence against the failure mode this feature exists
to prevent. A measurement cannot tell whether a reader accepts the refusal or works
around it.

## Summary

total: 13
passed: 11
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps

None. No test produced an issue.

Two observations that are not gaps, recorded so they are not lost:

- The colour difference is CIE76, a Euclidean distance in Lab space, not CIEDE2000.
  The orchestrator reported CIEDE2000 twice during execution and was wrong. CIEDE2000
  is a recorded future upgrade, deferred because it adds a sine, a cosine and a power
  function on top of the cube root Lab already needs.
- Four of the eight `should-register` calibration pairs report `identical`. That is
  correct: the corpus calibrates the refusal threshold, not change detection.
