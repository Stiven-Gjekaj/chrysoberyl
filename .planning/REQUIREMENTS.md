# Requirements: Chrysoberyl

**Defined:** 2026-09-06
**Core Value:** A baseline is portable. The same pair gives the same verdict on
any machine, any GPU and any driver.

## v1 Requirements

The format scope is wide by decision. The architecture freezes first and the
formats arrive in stages, so a wide requirement list does not mean a wide first
release. See ROADMAP.md for the order.

### Comparison Engine

- [x] **CORE-01**: The engine names each change by kind (moved, added, removed,
      recoloured, resized), never as a pixel count alone
- [x] **CORE-02**: The engine registers a translated region and reports the
      offset in pixels
- [x] **CORE-03**: The engine reports a colour change as a colour difference
      value and the two colours
- [x] **CORE-04**: The engine groups changed pixels into labelled regions, each
      with a bounding box
- [x] **CORE-05**: The engine does not report an antialiasing difference that a
      person cannot see
- [x] **CORE-06**: The engine refuses a pair it cannot register, and says why,
      instead of reporting a verdict it cannot support
- [x] **CORE-07**: The engine compares only near-identical pairs, and says so
      when a pair is too different to register

### Determinism

- [x] **DET-01**: The same pair gives an identical verdict on Linux, macOS and
      Windows
- [x] **DET-02**: The same pair gives an identical verdict on x86-64 and
      aarch64
- [x] **DET-03**: CI proves DET-01 and DET-02 on every commit, with a hash of
      raw RGBA8 output and never of a re-encoded file
- [x] **DET-04**: No pixel that enters a comparison is produced by a GPU
- [ ] **DET-05**: Text rasterization uses a font set pinned in the repository,
      never the host font database
- [x] **DET-06**: The comparison path calls no platform transcendental
      function, and the build disables floating point contraction on that path

### Input Sources

- [x] **SRC-01**: A raster image pair is compared (PNG, JPEG, WebP, TIFF)
- [ ] **SRC-02**: A numbered frame sequence pair is compared frame by frame
- [ ] **SRC-03**: An animation pair is compared frame by frame (GIF, APNG,
      animated WebP)
- [ ] **SRC-04**: An SVG pair is rasterized on the CPU and compared
- [ ] **SRC-05**: A PDF page pair is compared, behind a Cargo feature that is
      off by default
- [ ] **SRC-06**: A video pair is compared, paired by presentation timestamp
      and not by frame index, behind a Cargo feature that is off by default
- [ ] **SRC-07**: A 3D mesh pair is compared through a fixed eight-view rig,
      and the report names the blind spot the rig leaves
- [ ] **SRC-08**: A new input family is added without a change to any code in
      the comparison engine
- [ ] **SRC-09**: An input source can supply named regions as a hint, and the
      engine registers inside a named region instead of searching for it

### Rules and Tolerance

- [ ] **RULE-01**: A TOML rule file sets tolerance by kind of change
- [ ] **RULE-02**: A rule is scoped to a named region or to a mask
- [ ] **RULE-03**: A rule file is read as data. No rule is computed or executed
- [ ] **RULE-04**: The shipped example rule file shows a scoped exclusion, and
      never a bare global threshold
- [ ] **RULE-05**: An unknown key or a malformed rule fails loudly and names
      the line

### Baseline

- [ ] **BASE-01**: A baseline hash manifest is committed to the repository
- [ ] **BASE-02**: Committed golden files are the default store backend
- [ ] **BASE-03**: The store is a trait, so a second backend is added without a
      change to the engine
- [ ] **BASE-04**: A person accepts a new baseline explicitly. Nothing updates
      a baseline on its own

### Command Line and CI

- [ ] **CLI-01**: The tool exits non-zero when a rule fails
- [ ] **CLI-02**: The tool writes a report artifact that names each change by
      kind, region and size
- [ ] **CLI-03**: The tool compares two paths given on the command line and
      prints the verdict
- [ ] **CLI-04**: Every decode call sets an explicit memory limit, so a
      malformed input fails instead of exhausting memory

### Native Window

- [ ] **VIEW-01**: A window shows a pair side by side and as an overlay
- [ ] **VIEW-02**: A person pans and zooms a pair at full frame rate
- [ ] **VIEW-03**: A person moves through a sequence or a video pair frame by
      frame
- [ ] **VIEW-04**: A person accepts or rejects a change in the window, and an
      accept writes the new baseline

### GPU Mirror

- [ ] **GPU-01**: The wgpu compute path serves display only, and never writes a
      verdict
- [ ] **GPU-02**: The GPU path agrees with the CPU reference by category: the
      same regions, the same kinds, and bounding boxes inside a stated
      tolerance
- [ ] **GPU-03**: The equivalence test passes on at least three GPU vendors
      before the GPU path is called ready

## v2 Requirements

- **STORE-01**: A baseline store backend that is not committed goldens
- **CI-01**: A reusable GitHub Actions workflow and a pull request comment
  format
- **SRC-10**: Codecs beyond the v1 set
- **SRC-11**: A hint-channel adapter for a browser DOM
- **METRIC-01**: A Butteraugli-class metric. No mature Rust port exists today,
  so this waits for one or for a decision to bind to libjxl

## Out of Scope

| Feature | Reason |
|---------|--------|
| General image registration on arbitrary pairs | The near-identical assumption turns research into engineering. Remove it and the guarantee goes with it. |
| GPU rasterization of any comparison pixel | Vendors and drivers round differently, so a baseline becomes machine-dependent and the Core Value is gone. |
| A hosted service, an account, or an upload | The tool runs on the user's machine. Metered snapshot billing is the field's most common complaint. |
| A configurable mesh camera rig in v1 | A rig in a file the author edits is a baseline that moves when someone edits a config file. |
| A scriptable rule file | A rule file that can compute is a rule file that can lie. It must be reviewable in a pull request. |
| A black-box classifier that decides a verdict | Applitools publishes no decision boundary for its Visual AI. A CI gate a person cannot audit is a gate a person cannot trust. |
| Automatic baseline drift with no sign-off | A baseline that updates itself stops being a baseline. |
| Text and code diffs | git already does this well. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CORE-01 | Phase 1 | Complete |
| CORE-02 | Phase 1 | Complete |
| CORE-03 | Phase 1 | Complete |
| CORE-04 | Phase 1 | Complete |
| CORE-05 | Phase 1 | Complete |
| CORE-06 | Phase 1 | Complete |
| CORE-07 | Phase 1 | Complete |
| DET-01 | Phase 1 | Complete |
| DET-02 | Phase 1 | Complete |
| DET-03 | Phase 1 | Complete |
| DET-04 | Phase 1 | Complete |
| DET-06 | Phase 1 | Complete |
| SRC-01 | Phase 1 | Complete |
| SRC-02 | Phase 2 | Pending |
| SRC-03 | Phase 2 | Pending |
| SRC-08 | Phase 2 | Pending |
| SRC-09 | Phase 2 | Pending |
| RULE-01 | Phase 3 | Pending |
| RULE-02 | Phase 3 | Pending |
| RULE-03 | Phase 3 | Pending |
| RULE-04 | Phase 3 | Pending |
| RULE-05 | Phase 3 | Pending |
| BASE-01 | Phase 3 | Pending |
| BASE-02 | Phase 3 | Pending |
| BASE-03 | Phase 3 | Pending |
| BASE-04 | Phase 3 | Pending |
| CLI-01 | Phase 3 | Pending |
| CLI-02 | Phase 3 | Pending |
| CLI-03 | Phase 3 | Pending |
| CLI-04 | Phase 3 | Pending |
| SRC-04 | Phase 4 | Pending |
| DET-05 | Phase 4 | Pending |
| SRC-05 | Phase 5 | Pending |
| VIEW-01 | Phase 6 | Pending |
| VIEW-02 | Phase 6 | Pending |
| VIEW-03 | Phase 6 | Pending |
| VIEW-04 | Phase 6 | Pending |
| GPU-01 | Phase 6 | Pending |
| GPU-02 | Phase 6 | Pending |
| GPU-03 | Phase 6 | Pending |
| SRC-06 | Phase 7 | Pending |
| SRC-07 | Phase 8 | Pending |

**Coverage:**

- v1 requirements: 42 total
- Mapped to phases: 42
- Unmapped: 0

---
*Requirements defined: 2026-09-06*
*Last updated: 2026-09-06 after roadmap creation*
