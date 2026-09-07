---
phase: 03-baseline-rules-and-ci-gate
plan: 04
subsystem: baseline-store
tags: [rust, toml, baseline, static-guard, cli]

# Dependency graph
requires:
  - phase: 03-baseline-rules-and-ci-gate
    provides: "plan 03-03's crates/chrys-cli/src/main.rs (named_frames_for, run_compare) and chrys-source's Source::load_named, both already wired in before this plan started"
provides:
  - "The chrys-baseline crate: pub trait BaselineStore (resolve, accept, exactly two methods) and its committed-golden backend GoldenFileStore, in crates/chrys-baseline/src/golden.rs. Depends on neither chrys-core nor chrys-source (cargo tree -p chrys-baseline -e normal names only serde, thiserror, toml)."
  - "chrys accept <NAME> <PATH> [--store DIR]: the only call site of BaselineStore::accept in the workspace. Its --help text and its handler's doc comment both state, word for word, that an accept is a statement the candidate is now correct and that nothing in this tool can tell a correct accept from a mistaken one."
  - "chrys compare --baseline <NAME> [--store DIR] CANDIDATE: the base side is resolved from the store by name; a name nothing accepted is a loud, non-writing error naming the name and the store root (BASE-04). compare BASE CANDIDATE with no --baseline is unchanged (CLI-03)."
  - "The manifest is checked, not decorated: after resolving a baseline, compare recomputes chrys_core::hash::rgba8_digest over each decoded frame and compares it against chrys-baselines/MANIFEST.toml's own recorded digest for that name, failing the run and naming the baseline, the frame, and both digests on any disagreement."
  - "chrys-baselines/MANIFEST.toml and chrys-baselines/pair-01/base.png: BASE-01's committed hash manifest and its first accepted baseline, produced by a real run of accept and tracked by git."
  - "Two independently drilled BASE-04 guards: resolve_writes_nothing_to_the_store (behavioural, records the whole path set with length and modification time) and only_the_accept_path_writes_to_the_store (static, a third comment-and-string stripper over chrys-baseline's own src, since neither chrys-core's nor chrys-cli's existing copy is reachable from this crate's test target under D-03)."
affects: []

# Actuals (#2632)
actuals:
  tokens: 13498
  tasks: 3
  commits: 4
  plan_head_before: bf3a95e83cc4bab647fd47b31877d24e9dfa8dd2

# Tech tracking
tech-stack:
  added:
    - "chrys-baseline (new workspace crate): toml, serde (derive), thiserror, all from the workspace table; no new external crate version"
  patterns:
    - "A baseline store never learns a pixel format or a Verdict: chrys-baseline depends on neither chrys-core nor chrys-source, and its own trait, BaselineStore, takes a Path and pre-computed (source, digest) pairs, never a Frame. This is what makes BASE-03's 'a second backend needs no engine change' a structural fact rather than a hope."
    - "Every write inside chrys-baseline lives in one private function, write_baseline, called from exactly one place, accept's own body. This is proven twice: a behavioural test (resolve_writes_nothing_to_the_store) and a static guard (only_the_accept_path_writes_to_the_store), each drilled red in a disposable git worktree before being trusted, matching the discipline 02-LEARNINGS.md records under 'A guard whose silence is read as evidence gets a drill.'"
    - "resolve finds a baseline by listing its own directory, never by trusting MANIFEST.toml's own source field, so a person renaming the stored file with git mv keeps a working baseline. A directory holding exactly one file resolves to that file's own path; more than one resolves to the directory."
    - "The manifest is read-checked by the caller that holds the pixels (compare), not by the store itself: GoldenFileStore::manifest_digests only reads MANIFEST.toml and returns (source, digest) pairs; resolve never touches a digest. Keeping this split is what lets resolve's own read-only contract stay simple while the manifest still gets checked rather than decorated."
    - "A third copy of the project's comment-and-string stripper exists in crates/chrys-baseline/tests/store.rs, matching the pattern crates/chrys-cli/tests/decode_limits_guard.rs already established: crates/chrys-core/tests/determinism.rs's own copy is unreachable under D-03, and chrys-cli's own copy is private to a different crate's test binary."
  removed: []

key-files:
  created:
    - crates/chrys-baseline/Cargo.toml
    - crates/chrys-baseline/src/lib.rs
    - crates/chrys-baseline/src/golden.rs
    - crates/chrys-baseline/tests/store.rs
    - crates/chrys-cli/tests/baseline.rs
    - chrys-baselines/MANIFEST.toml
    - chrys-baselines/pair-01/base.png
  modified:
    - crates/chrys-cli/Cargo.toml
    - crates/chrys-cli/src/main.rs
    - Cargo.lock

key-decisions:
  - "write_baseline, accept's own single write function, was written to handle both a single-file candidate and a directory candidate from Task 1's own commit, rather than growing directory support only in Task 2 as the plan's own task split literally describes. Task 2's own commit therefore added only the tests that prove the directory case (byte-for-byte copy, rename survival) and the CLI's --baseline digest-check plan text; no further production code changed in Task 2's own commit. This is the same shape of task-boundary shift 03-03-SUMMARY.md already recorded once (Task 1's own restructuring delivering behaviour Task 2's own text separately describes). See Deviations."
  - "A prior acceptance under a name is removed in full (fs::remove_dir_all) before the new one is written, rather than copying new files over old ones in place, so a baseline whose frame count shrinks between two accepts cannot leave a stale file behind that resolve's own directory listing would then disagree with the manifest about."
  - "The manifest's [[baseline]] array is kept sorted by name after every accept, so accepting a second, unrelated baseline never reorders an already-committed one in the diff: BASE-01 wants a single accepted change to read as one changed digest line, not a reshuffled file."
  - "resolve's not-accepted case and its zero-files case are the same error, BaselineError::NotAccepted: a directory that exists but holds no file is exactly as unaccepted as a directory that was never created. This keeps BASE-04's 'no first silent accept' true regardless of which of the two states a corrupted or partially-cleaned store happens to be in."
  - "The write-call guard's failure message was extended, in a fourth commit, to name the enclosing function (via enclosing_function_name, scanning backward for the nearest fn keyword) rather than only a file and a byte offset. This was made before drilling, once it was clear the plan's own drill-one expectation ('confirm... its message names resolve') needed the guard to actually say so, not just make the offending offset locatable by hand. Tracked as a deviation below."

requirements-completed: [BASE-01, BASE-02, BASE-03, BASE-04, CLI-03]

coverage:
  - id: D1
    description: "A person accepts a baseline once by name (chrys accept pair-01 tests/golden/pair-01/base.png), and a later comparison finds it by that name alone (compare --baseline pair-01 CANDIDATE), giving the same verdict compare BASE CANDIDATE gave before the baseline existed (BASE-01, BASE-02)."
    requirement: BASE-01
    verification:
      - kind: integration
        ref: "manual run recorded verbatim below: cargo run -q -p chrys-cli -- compare --baseline pair-01 tests/golden/pair-01/candidate.png, byte-identical output and exit code to compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png"
        status: pass
      - kind: unit
        ref: "crates/chrys-baseline/tests/store.rs#accept_writes_a_manifest_toml_file"
        status: pass
    human_judgment: false
  - id: D2
    description: "chrys-baselines/MANIFEST.toml and chrys-baselines/pair-01/base.png are committed and tracked by git; the manifest's contents are recorded verbatim in this summary."
    requirement: BASE-01
    verification:
      - kind: other
        ref: "git ls-files --error-unmatch chrys-baselines/MANIFEST.toml chrys-baselines/pair-01/base.png: both paths printed, exit 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "A directory candidate's every file lands in the store byte for byte (not by size), and the manifest records one frame line per file, in frame order."
    requirement: BASE-02
    verification:
      - kind: unit
        ref: "crates/chrys-baseline/tests/store.rs#accept_copies_the_candidate_file_byte_for_byte"
        status: pass
    human_judgment: false
  - id: D4
    description: "BaselineStore declares exactly two methods (resolve, accept); chrys-baseline's own Cargo.toml names neither chrys-core nor chrys-source; cargo tree -p chrys-baseline -e normal confirms it."
    requirement: BASE-03
    verification:
      - kind: unit
        ref: "crates/chrys-baseline/tests/store.rs#the_trait_has_exactly_resolve_and_accept"
        status: pass
      - kind: other
        ref: "cargo tree -p chrys-baseline -e normal: serde, thiserror, toml and their own transitive dependencies only"
        status: pass
    human_judgment: false
  - id: D5
    description: "resolve on a name nothing accepted is a loud, non-writing error naming the name and the store root; resolve writes nothing on either a successful or a failed call; both the behavioural test and the static write-call guard are drilled red in a disposable worktree before being trusted, and both drills' output is recorded verbatim below."
    requirement: BASE-04
    verification:
      - kind: unit
        ref: "crates/chrys-baseline/tests/store.rs#resolving_an_unaccepted_name_is_an_error, resolve_writes_nothing_to_the_store, only_the_accept_path_writes_to_the_store"
        status: pass
      - kind: integration
        ref: "crates/chrys-cli/tests/baseline.rs#a_baseline_that_was_never_accepted_fails_loudly"
        status: pass
      - kind: other
        ref: "two planted-defect drills in disposable git worktrees, recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D6
    description: "compare BASE CANDIDATE with no --baseline still prints and exits exactly as it did before this plan; cargo test -p chrys-cli --test digest passes unchanged across the positional-argument change."
    requirement: CLI-03
    verification:
      - kind: integration
        ref: "cargo test -p chrys-cli --test digest: 6 passed, 0 failed"
        status: pass
    human_judgment: false
  - id: D7
    description: "A hand-edited manifest digest fails the comparison, naming the baseline, the frame, and both digests, so the manifest is evidence rather than decoration."
    requirement: (this plan's own must_haves)
    verification:
      - kind: integration
        ref: "crates/chrys-cli/tests/baseline.rs#a_manifest_digest_that_disagrees_with_its_file_fails_the_run; manual run recorded verbatim below"
        status: pass
    human_judgment: false
  - id: D8
    description: "No file under crates/chrys-core/ changes anywhere in this plan's commit range, and the whole-phase engine-boundary drill stays silent over the whole phase's own commit range."
    requirement: (D-03, project-wide invariant)
    verification:
      - kind: other
        ref: "git rev-parse HEAD:crates/chrys-core read c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 before this plan's first commit and after each of its four commits; scripts/engine-boundary-drill.sh 29f5add 6da87c4 (2 of 2 drills behaved as expected, recorded verbatim below)"
        status: pass
    human_judgment: false
  - id: D9
    description: "cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --check all pass at every commit boundary; the workspace test count only grows (243 to 253), never regresses."
    requirement: (project-wide invariant)
    verification:
      - kind: other
        ref: "cargo test --workspace: 253 passed, 0 failed, at HEAD (6da87c4); cargo clippy --workspace --all-targets -- -D warnings: clean; cargo fmt --check: clean; cargo test -p chrys-core --test determinism: 6 passed, 0 failed"
        status: pass
    human_judgment: false

duration: not captured (PLAN_START_TIME was not recorded at launch; see git log for the four task commits' own timestamps)
completed: 2026-09-07
status: complete
---

# Phase 3 Plan 04: Accept a baseline and refuse one nothing accepted Summary

**The `chrys-baseline` crate: a two-method `BaselineStore` trait and its committed-golden backend, reached only through `chrys accept NAME PATH` and `chrys compare --baseline NAME`, with a real manifest committed to this repository and two independently drilled guards proving nothing but an explicit accept ever writes to the store.**

## Performance

- **Duration:** not captured (see frontmatter note)
- **Completed:** 2026-09-07
- **Tasks:** 3 / 3 (plus one small fixup commit; see Deviations)
- **Files modified:** 10 (3 modified, 7 new)

## Accomplishments

- Created the `chrys-baseline` crate (`crates/chrys-baseline/Cargo.toml`, `src/lib.rs`, `src/golden.rs`). Its dependency table names only `toml`, `serde` (with `derive`), and `thiserror`, all from the workspace table; `cargo tree -p chrys-baseline -e normal` confirms it names neither `chrys-core` nor `chrys-source`.
- `pub trait BaselineStore` declares exactly two methods: `resolve(&self, name: &str) -> Result<PathBuf, Self::Error>`, documented as read-only by contract, and `accept(&self, name: &str, candidate_path: &Path, frame_digests: &[(String, String)]) -> Result<(), Self::Error>`. `crates/chrys-baseline/tests/store.rs#the_trait_has_exactly_resolve_and_accept` reads the trait's own source and counts its methods, rather than trusting a description of it.
- `GoldenFileStore { root: PathBuf }` is the committed-golden backend. `resolve` lists `<root>/<name>/` rather than trusting `MANIFEST.toml`'s own `source` field: a directory holding one file resolves to that file's own path, more than one resolves to the directory itself, and a person who renames the stored file with `git mv` keeps a working baseline (proven by `resolve_survives_a_rename_inside_the_baseline_directory`).
- Every write in the crate — `fs::create_dir_all`, `fs::remove_dir_all`, `fs::copy`, `fs::write` — lives inside one private function, `write_baseline`, called from exactly one place: `accept`'s own body. `write_baseline` removes any prior acceptance under a name in full before writing the new one, so a baseline whose frame count shrinks between two accepts leaves no stale file behind. The manifest's own `[[baseline]]` array is kept sorted by name after every accept, so an unrelated accept never reorders an already-committed baseline's own lines in a diff.
- Added `chrys accept <NAME> <PATH> [--store DIR]` to the CLI: it loads `PATH` through the same `named_frames_for` (and so `Source::load_named`) path `compare` already uses, computes `chrys_core::hash::rgba8_digest` over each decoded frame's own RGBA8 bytes, and is the only call site of `BaselineStore::accept` in the whole workspace. Its `--help` text and its handler's own doc comment carry the identical sentence, word for word: "An accept is a statement: this candidate is now correct. Every later comparison against NAME is measured against it, and nothing in this tool can tell a correct accept from a mistaken one." (Proven by `accept_help_states_that_an_accept_is_a_statement`.)
- Added `--baseline <NAME>` and `--store <DIR>` to `compare`. The two positional paths became one `Vec<PathBuf>` taking one or two values, validated explicitly in `base_and_candidate`: with no `--baseline`, exactly two paths (base, then candidate); with `--baseline`, exactly one (the candidate), because the base comes from the store via `BaselineStore::resolve`. `compare BASE CANDIDATE` with no `--baseline` is unchanged; `cargo test -p chrys-cli --test digest` (CLI-03's own regression guard) stays green throughout.
- The manifest is checked, not decorated: `verify_baseline_digests`, called from `run_compare` whenever `--baseline` was given, recomputes `chrys_core::hash::rgba8_digest` over each decoded base-side frame and compares it, and the recorded frame count, against `GoldenFileStore::manifest_digests`'s own read of `MANIFEST.toml`. A disagreement fails the run and names the baseline, the frame (by both index and source file name), and both digests. `GoldenFileStore::resolve` itself never reads a digest; the check lives with the caller that has the pixels.
- Ran `cargo run -q -p chrys-cli -- accept pair-01 tests/golden/pair-01/base.png` from the repository root and committed the result: `chrys-baselines/MANIFEST.toml` and `chrys-baselines/pair-01/base.png`, both tracked by git (`git ls-files --error-unmatch` exits 0 on both). The manifest's full contents are recorded verbatim below.
- Task 2 added `accept_copies_the_candidate_file_byte_for_byte` (asserting byte equality, never size equality, per `AGENTS.md`'s own "a size is not a state") and `resolve_survives_a_rename_inside_the_baseline_directory` to `crates/chrys-baseline/tests/store.rs`, and `crates/chrys-cli/tests/baseline.rs` (new) with `a_manifest_digest_that_disagrees_with_its_file_fails_the_run`. All three tests pass against code already delivered in Task 1's own commit; see Deviations.
- Task 3 added `a_baseline_that_was_never_accepted_fails_loudly` (also asserting the store directory is never created on a failed resolve) to `crates/chrys-cli/tests/baseline.rs`; `resolve_writes_nothing_to_the_store` (recording the whole path set under a store, with each entry's own length and modification time, around both a successful and a failed resolve) and `only_the_accept_path_writes_to_the_store` (a static guard, carrying a third comment-and-string stripper since neither `chrys-core`'s nor `chrys-cli`'s existing copy is reachable from this crate's own test target under D-03) to `crates/chrys-baseline/tests/store.rs`; and `the_trait_has_exactly_resolve_and_accept`.
- Both BASE-04 guards were drilled red in disposable git worktrees, created from HEAD and removed on completion, before being trusted. Both drills behaved exactly as the plan predicted (recorded verbatim below).
- `git rev-parse HEAD:crates/chrys-core` read `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` before this plan's first commit and after each of its four commits. `scripts/engine-boundary-drill.sh 29f5add 6da87c4` (this phase's own first commit through this plan's own last commit) ran both of its drills and both behaved as expected (recorded verbatim below): this is the phase gate.
- `cargo test --workspace` (253 passed, 0 failed, up from the 243 baseline this plan started from), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --check` all pass, at every commit boundary. `cargo test -p chrys-core --test determinism` passes with all six guards, and `cargo test -p chrys-cli --test digest` passes unchanged (CLI-03).

## Task Commits

Each task was committed atomically, plus one small fixup commit (see Deviations):

1. **Task 1: Accept a baseline and compare against the stored one** — `1353e98`
2. **Task 2: Copy a whole sequence into the store byte for byte** — `5e47a8a`
3. **Task 3: Refuse a baseline nothing has accepted** — `b801311`
4. **Fixup: Name the enclosing function in the write-call guard's failure message** — `6da87c4`

**Plan metadata:** pending (this SUMMARY and STATE.md/ROADMAP.md updates are committed by the orchestrator, per this plan's execution instructions).

## Files Created/Modified

- `crates/chrys-baseline/Cargo.toml` — new crate manifest: `toml`, `serde` (derive), `thiserror`, from the workspace table
- `crates/chrys-baseline/src/lib.rs` — `pub trait BaselineStore` (`resolve`, `accept`)
- `crates/chrys-baseline/src/golden.rs` — `GoldenFileStore`, `BaselineError`, `Manifest`/`BaselineEntry`/`FrameEntry`, `read_manifest`, `write_baseline` (the crate's one write function), `manifest_digests`
- `crates/chrys-baseline/tests/store.rs` — new: 7 tests (`accept_writes_a_manifest_toml_file`, `resolving_an_unaccepted_name_is_an_error`, `accept_copies_the_candidate_file_byte_for_byte`, `resolve_survives_a_rename_inside_the_baseline_directory`, `resolve_writes_nothing_to_the_store`, `only_the_accept_path_writes_to_the_store`, `the_trait_has_exactly_resolve_and_accept`), plus its own third comment-and-string stripper
- `crates/chrys-cli/Cargo.toml` — added `chrys-baseline` path dependency
- `crates/chrys-cli/src/main.rs` — `accept` subcommand, `--baseline`/`--store` flags on `compare`, `base_and_candidate`, `run_accept`, `verify_baseline_digests`, `DEFAULT_STORE_ROOT`
- `crates/chrys-cli/tests/baseline.rs` — new: 4 tests (`a_manifest_digest_that_disagrees_with_its_file_fails_the_run`, `a_baseline_that_was_never_accepted_fails_loudly`, `accept_help_states_that_an_accept_is_a_statement`)
- `chrys-baselines/MANIFEST.toml` — new, committed: BASE-01's hash manifest
- `chrys-baselines/pair-01/base.png` — new, committed: the first accepted baseline
- `Cargo.lock` — `chrys-baseline` added as a workspace member; no new external crate version

## Decisions Made

- `write_baseline` was written to handle a directory candidate from Task 1's own commit onward, rather than growing that support only in Task 2 as the plan's task split literally describes; Task 2's own commit added only the tests that prove it. See Deviations.
- A prior acceptance under a name is removed in full (`fs::remove_dir_all`) before the new one is written, so a shrinking frame count cannot leave a stale file the manifest and `resolve`'s own directory listing would then disagree about.
- `resolve`'s "never accepted" and "accepted but zero files" cases return the identical `BaselineError::NotAccepted`, so BASE-04's guarantee holds regardless of which of the two states a store happens to be in.
- The manifest's `[[baseline]]` array is sorted by name after every accept, so an accept of one baseline never reorders another's lines in a reviewer's diff.
- The write-call guard's failure message was extended to name the enclosing function, in a fourth, small commit made before drilling. See Deviations.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 2's own directory-copy behaviour was already delivered by Task 1's commit; Task 2 added only tests and the CLI's digest check**
- **Found during:** Task 1, while implementing `write_baseline` and the CLI's `accept` handler.
- **Issue:** The plan's own Task 1 action text requires `GoldenFileStore::accept` to accept "a file or a directory" from its very first commit (the trait's own doc comment, written in Task 1, already says so), and the shipped `accept` CLI subcommand must load `PATH` through the same `load_named` path `compare` uses "so a directory accepts as a sequence and a file accepts as one frame" — also Task 1 text. Implementing `write_baseline` to handle only the single-file case in Task 1, and adding directory support as a second, later change in Task 2, would have meant Task 1's own committed `accept` subcommand could not actually accept a directory candidate at all, contradicting Task 1's own stated behaviour. The natural single write function that satisfies Task 1's own requirements already walks a candidate directory's own top-level files.
- **Fix:** `write_baseline` (Task 1's commit, `1353e98`) copies both a single file and every regular top-level file of a directory into the store, and the CLI's `verify_baseline_digests` (also `1353e98`) was added in the same commit alongside `--baseline`, so `--baseline`'s own behaviour ("composes with every existing flag") is complete and checked from its first appearance rather than shipping unchecked for one commit and gaining a check the next. Task 2's own commit (`5e47a8a`) then added `accept_copies_the_candidate_file_byte_for_byte`, `resolve_survives_a_rename_inside_the_baseline_directory`, and `a_manifest_digest_that_disagrees_with_its_file_fails_the_run`, with no further production-code change.
- **Files modified:** `crates/chrys-baseline/src/golden.rs`, `crates/chrys-cli/src/main.rs` (both already in Task 1's own `files_modified` list).
- **Verification:** All three of Task 2's own named tests pass against the code as it stood after Task 1's own commit. `cargo test -p chrys-cli --test digest` (CLI-03) stayed green throughout both commits.
- **Committed in:** `1353e98` (Task 1 commit); proven by tests added in `5e47a8a` (Task 2 commit).
- **Precedent:** The identical shape of deviation `03-03-SUMMARY.md` already recorded once in this phase: a task's own literal behaviour requirement, read carefully, sometimes already implies production code the plan's own task split assigned to a later task's action text.

**2. [Rule 1 - Bug] The write-call guard's failure message did not name the enclosing function, which the plan's own drill-one expectation requires**
- **Found during:** Task 3, immediately before running the two required planted-defect drills.
- **Issue:** The plan's own action text for drill one says: "run the static guard, and confirm it fails and that its message names `resolve`." The guard as first written (`b801311`) reported only a file path and a byte offset for an offending write call, which does let a person locate the call by hand but does not itself say the word "resolve" anywhere in the failure output. Drilling a guard whose message cannot ever satisfy the plan's own stated success condition, then declaring the drill successful because the assertion happened to fail for the right reason, would be exactly the kind of guard-quality gap 02-LEARNINGS.md's "A guard whose silence is read as evidence gets a drill" warns against, one level removed: a guard that goes red is not the same as a guard whose message says what the plan requires it to say.
- **Fix:** Added `enclosing_function_name`, which scans backward from an offending call's own byte offset for the nearest `fn` keyword and reads off the following identifier, and used it to add `, inside fn {enclosing_fn}` to the guard's own failure message.
- **Files modified:** `crates/chrys-baseline/tests/store.rs`.
- **Verification:** `cargo test -p chrys-baseline --test store` (7 passed), `cargo clippy --workspace --all-targets -- -D warnings` (clean), `cargo fmt --check` (clean), `cargo test --workspace` (253 passed) all re-run and green before drilling. Drill one's own output (recorded verbatim below) now reads `..., inside fn resolve: \`fs::create_dir_all(\``, satisfying the plan's own stated check.
- **Committed in:** `6da87c4`, a fourth, small commit made after Task 3's own commit (`b801311`) and before either drill was run.
- **Precedent:** None; this is the first time this project's own drill process has required a guard's message text itself to be corrected before the drill it was written for could be run meaningfully.

---

**Total deviations:** 2 auto-fixed (1 blocking/Rule 3, 1 bug/Rule 1). No file outside the plan's own `files_modified` list was touched.

## Issues Encountered

None beyond the deviations recorded above.

## User Setup Required

None — no external service configuration required.

## `chrys-baselines/MANIFEST.toml`, Full Contents, Recorded Verbatim

```toml
[[baseline]]
name = "pair-01"

[[baseline.frame]]
source = "base.png"
digest = "5a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc"
```

## `compare --baseline pair-01 tests/golden/pair-01/candidate.png`, Recorded Verbatim

```
$ cargo run -q -p chrys-cli -- compare --baseline pair-01 tests/golden/pair-01/candidate.png
Recoloured region at x=64, y=64, width=96, height=64, colour delta 111.83 (base [40, 90, 200, 255], candidate [200, 90, 40, 255])
$ echo $?
1
```

This is byte-identical, in both stdout and exit code, to `compare tests/golden/pair-01/base.png tests/golden/pair-01/candidate.png` (BASE-01, BASE-02, CLI-03).

## The Disagreeing-Manifest-Digest Failure, Recorded Verbatim

Reproduced manually against a fresh temporary store (the same scenario `a_manifest_digest_that_disagrees_with_its_file_fails_the_run` automates): `accept` into an empty store, flip the first hexadecimal character of the recorded digest in `MANIFEST.toml`, then `compare --baseline`:

```
$ cargo run -q -p chrys-cli -- compare --baseline pair-01 --store .manual-drill-store tests/golden/pair-01/candidate.png
baseline "pair-01" frame 0 ("base.png"): the manifest records digest 0a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc, but the stored file now produces 5a5f59948cbf0cfca0c432dfc439f3c48e6bd9f7d0dad442212440081fe584cc
$ echo $?
3
```

The message names the baseline (`pair-01`), the frame (`0`, `"base.png"`), the digest the manifest records, and the digest the file actually produces.

## The Two Planted-Defect Drills for the BASE-04 Guards, Recorded Verbatim

Both drills ran inside a disposable `git worktree add --detach --quiet <path> HEAD`, created after this plan's own Task 3 commit and its follow-up fixup commit (`6da87c4`, so the guard's improved failure message already existed to be drilled), removed with `git worktree remove --force <path>` on completion. The real working tree held no uncommitted changes before or after either drill.

**Drill one:** planted a write call moved out of `write_baseline` into `resolve` (`let _ = fs::create_dir_all(&baseline_dir);`, added at the top of `resolve`'s own body):

```
running 1 test
test only_the_accept_path_writes_to_the_store ... FAILED

failures:

---- only_the_accept_path_writes_to_the_store stdout ----

thread 'only_the_accept_path_writes_to_the_store' panicked at crates/chrys-baseline/tests/store.rs:499:5:
the following write call(s) live outside write_baseline, the one function every write in this crate must go through: /.../.drills/drill1/crates/chrys-baseline/src/golden.rs@3477, inside fn resolve: `fs::create_dir_all(`

failures:
    only_the_accept_path_writes_to_the_store

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
```

The static guard went red and named `resolve` as the enclosing function of the planted call, exactly as the plan requires.

**Drill two:** planted `resolve` creating its own directory for a name that was never accepted (`let _ = fs::create_dir_all(&baseline_dir);`, added inside the `NotFound` arm, immediately before returning `BaselineError::NotAccepted`), in a fresh disposable worktree:

```
running 1 test
test resolve_writes_nothing_to_the_store ... FAILED

failures:

---- resolve_writes_nothing_to_the_store stdout ----

thread 'resolve_writes_nothing_to_the_store' panicked at crates/chrys-baseline/tests/store.rs:256:5:
assertion `left == right` failed: a failed resolve wrote to the store
  left: [(".../MANIFEST.toml", 90, SystemTime { .. }), (".../pair-01", 96, SystemTime { .. }), (".../pair-01/base.png", 22, SystemTime { .. })]
 right: [(".../MANIFEST.toml", 90, SystemTime { .. }), (".../never-accepted", 64, SystemTime { .. }), (".../pair-01", 96, SystemTime { .. }), (".../pair-01/base.png", 22, SystemTime { .. })]

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.01s
```

The behavioural test went red and its diff names the exact path that appeared (`.../never-accepted`), the one `resolve` planted, present on the right (after) side and absent on the left (before).

## `chrys accept --help`, Recorded Verbatim

```
Accept PATH as the new baseline named NAME.

An accept is a statement: this candidate is now correct. Every later comparison against NAME is measured against it, and nothing in this tool can tell a correct accept from a mistaken one.

Usage: chrys accept [OPTIONS] <NAME> <PATH>

Arguments:
  <NAME>
          The baseline's own name

  <PATH>
          The path to accept: a file or a directory

Options:
      --store <STORE>
          The baseline store root to accept into
          
          [default: chrys-baselines]

  -h, --help
          Print help (see a summary with '-h')
```

## `scripts/engine-boundary-drill.sh`, the Whole Phase's Own Commit Range

First commit of the phase (plan 03-01's own first commit): `29f5add`. Last commit of the phase (this plan's own last commit): `6da87c4`.

`scripts/engine-boundary-drill.sh 29f5add 6da87c4`, full output:

```
engine-boundary-drill: drill one: checked range 6da87c469eb1c4f711c7fa52183bda7ef5b9f6e5..e0a38ed7b6c7e938081d505b0d116a4bfa5c913d
engine-boundary-drill: drill one: check output:
crates/chrys-core/src/lib.rs
drill ok: planted-defect drill: the check went red and named crates/chrys-core/src/lib.rs
engine-boundary-drill: drill two: checked range 29f5add^..6da87c4
engine-boundary-drill: drill two: check output:
(empty)
drill ok: clean-range drill: the check stayed silent over plan 02-03's own range

engine-boundary-drill: 2 of 2 drills behaved as expected
```

Both drills behaved as expected: the planted-defect drill went red and named the exact file planted, and the clean-range drill stayed silent over the whole phase's own real commit range, `29f5add^..6da87c4` (all twelve of phase 3's own task commits, across plans 03-01 through 03-04, plus this plan's own fixup commit). This is the evidence for the phase's headline claim (D-03): the rule reader, the mask, the report, and the baseline store all arrived without the engine learning that any of them exist.

## `git diff --stat`, This Plan's Own Commit Range, `crates/chrys-core/`

`git diff --stat bf3a95e..6da87c4 -- crates/chrys-core/`: empty output. No file under `crates/chrys-core/` changed anywhere in this plan's own four commits.

## Full Verification, This Plan's End State

```
cargo test -p chrys-baseline --test store: 7 passed, 0 failed.
cargo test -p chrys-cli --test baseline: 3 passed, 0 failed.
cargo test -p chrys-cli --test digest: 6 passed, 0 failed (CLI-03 unmoved).
cargo test --workspace: 253 passed, 0 failed (up from the 243 baseline this plan started from; +10 new tests: 7 in chrys-baseline/tests/store.rs, 3 in chrys-cli/tests/baseline.rs).
cargo test -p chrys-core --test determinism: 6 passed, 0 failed (all six guards green).
cargo clippy --workspace --all-targets -- -D warnings: clean.
cargo fmt --check: clean.
cargo tree -p chrys-baseline -e normal: names only serde, thiserror, toml and their own transitive dependencies (no chrys-core, no chrys-source).
git rev-parse HEAD:crates/chrys-core: c97a6fb778c4b1373e5c4dc563481cc18e4c98c0 (unmoved, checked before the first commit and after each of the four commits).
git ls-files --error-unmatch chrys-baselines/MANIFEST.toml chrys-baselines/pair-01/base.png: both paths printed, exit 0.
scripts/engine-boundary-drill.sh 29f5add 6da87c4 (the whole phase's own range): 2 of 2 drills behaved as expected.
```

## Predictions That Turned Out Wrong

- None outright wrong, but one prediction needed correcting before it could be trusted: the plan's own drill-one instruction ("confirm it fails and that its message names `resolve`") assumed the static guard's failure message would already name the function an offending call sat inside. The guard as first written named only a file and a byte offset. This was fixed (see Deviations, item 2) before the drill was run, rather than treating the drill as satisfied by a red result that could not, as written, ever have named `resolve`.
- Every other prediction and instruction in this plan (the two-method trait shape, `resolve` finding a baseline by directory listing rather than by trusting the manifest, the one-write-function discipline, the exact wording required of `accept --help`, and the manifest's own `[[baseline]]`/`[[baseline.frame]]` table shape) matched exactly.

## Next Phase Readiness

- BASE-01 through BASE-04 and CLI-03's own regression guard are all delivered and observable: a real, committed baseline exists in this repository, a person accepts one only through `chrys accept`, and a comparison against a name nothing accepted is a loud, non-writing error.
- The manifest is evidence, not decoration: a hand-edited digest fails the run and names both digests, so a future reader trusting a green `compare --baseline` run is trusting a claim the tool itself re-checks on every run, not a claim it printed once and forgot.
- `chrys-core`'s tree object id (`c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`) and dependency graph are unmoved, checked at every commit boundary and by the engine-boundary drill over this plan's own commit range and, as this plan's own phase gate, over the whole of phase 3's commit range (`29f5add^..6da87c4`).
- This closes phase 3 (rule schema and evaluation, mask-scoped rules, the report and the decode guard, and now the baseline store). `crates/chrys-baseline/src/golden.rs`'s own `BaselineStore`/`GoldenFileStore` shapes are the reference implementation for any later plan that adds a second baseline backend; BASE-03's own promise is that doing so needs no change to `crates/chrys-core/`, `crates/chrys-rule/`, or the CLI's own comparison path, only a second type implementing the same two methods.

## Self-Check: PASSED

All created files verified present on disk (`crates/chrys-baseline/Cargo.toml`, `src/lib.rs`, `src/golden.rs`, `tests/store.rs`, `crates/chrys-cli/tests/baseline.rs`, `chrys-baselines/MANIFEST.toml`, `chrys-baselines/pair-01/base.png`); all four commit hashes (`1353e98`, `5e47a8a`, `b801311`, `6da87c4`) verified present in `git log --oneline`; `git rev-parse HEAD:crates/chrys-core` verified reading `c97a6fb778c4b1373e5c4dc563481cc18e4c98c0` after the final commit; working tree verified clean (`git status --short`) after both planted-defect drills' worktrees were removed and after the manual manifest-digest-disagreement reproduction's own temporary store was removed.

---
*Phase: 03-baseline-rules-and-ci-gate*
*Completed: 2026-09-07*
