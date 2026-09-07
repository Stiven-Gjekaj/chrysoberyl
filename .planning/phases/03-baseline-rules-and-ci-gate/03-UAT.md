---
status: complete
phase: 03-baseline-rules-and-ci-gate
source: 03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md, 03-04-SUMMARY.md, 03-05-SUMMARY.md
started: 2026-09-08T00:10:00Z
updated: 2026-09-08T00:10:00Z
---

## Current Test

[testing complete]

Tests 1 to 9 were run by the orchestrator against the release binary it built
in the same session, because every one of them is mechanically observable.
Test 10 was read rather than measured, and reading it found and fixed a
factual error in the shipped example.

## Tests

### 1. A rule scopes a tolerance by kind and by a named region
expected: a rule that names the region and allows the measured delta tolerates it; a tighter rule does not
result: pass
observed: `tolerate.toml` exits 0, `too-tight.toml` exits 1, on the same pair

### 2. An unscoped rule is refused, and the message names the line
expected: a rule naming neither a region nor a mask fails loudly
result: pass
observed: `line 1: a rule must name exactly one of `region` or `mask`, but this rule names neither`, exit 3

### 3. An unknown key is refused, and the message names the line
expected: a misspelled key fails rather than deserializing into a default
result: pass
observed: `cannot parse /tmp/k.toml: TOML parse error at line 4, column 1`, exit 3

### 4. A mask tolerates only where it is white and opaque
expected: an opaque white mask tolerates; a mask drawn white on transparency does not
result: pass
observed: `mask-tolerate.toml` exits 0, `mask-transparent.toml` exits 1

### 5. A mask path cannot leave the rule file's directory
expected: a path that climbs out is refused before it is opened
result: pass
observed: `../../etc/passwd leaves /tmp, the rule file's own directory; a mask path must resolve inside it`, exit 3

### 6. A baseline nothing accepted is refused
expected: the tool refuses rather than accepting the candidate as a first baseline
result: pass
observed: `baseline "never-accepted" was never accepted; the store at chrys-baselines holds no baseline of that name`, exit 3, and no directory is created

### 7. The accept command says what an accept means
expected: the help text states that an accept is a claim, not bookkeeping
result: pass
observed: "An accept is a statement: this candidate is now correct. Every later comparison against NAME is measured against it, and nothing in this tool can tell a correct accept from a mistaken one."

### 8. The report names each change by kind, region, size and source file
expected: the report artifact carries all four
result: pass
observed: `region = "badge"`, `source = "base.png"`, `kind = "recoloured"`, `x = 40`, `width = 50`, `size = 2000`. The `x = 40` is the frame's own coordinate, not the crop's, which is the gap-closure fix visible in the artifact.

### 9. The shipped example names a scope on every rule
expected: no rule in the example is a bare global threshold
result: pass
observed: 2 of 2 rules name a region or a mask

### 10. The shipped example rule file reads clearly to a stranger
expected: a person who has not seen the schema can read the example and write their own rule from it
result: pass
observed: read end to end. It carries what a reader needs: why a rule must name a scope and what that limit prevents, a worked `<stem>.hints.toml` sidecar showing how a region name comes to exist, and the mask convention stated with its own failure mode ("a mask drawn as white on a transparent background is white everywhere and opaque nowhere").

Reading it found one defect, now fixed. The file told a reader its threshold
was a CIEDE2000 delta E. The engine measures a CIE76 delta E, and
`crates/chrys-core/src/classify/colour.rs:19` says so. The two formulas give
different numbers for the same pair, so a person copying a threshold from a
tool that reports CIEDE2000 would have set a limit that did not mean what they
thought. This was the one error that would have made the file actively
misleading rather than merely terse.

What a measurement cannot settle is whether the prose reads well, as opposed
to whether it is complete and correct. That half is Stiven's to overturn.

## Summary

total: 10
passed: 10
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

None. The two gaps this phase carried, a false pass under a region flag and a
destructive baseline write, were closed by plan 03-05 and re-verified before
this session ended.
