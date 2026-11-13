# Changelog

This file records each notable change to Chrysoberyl. The format follows
Keep a Changelog (https://keepachangelog.com). The project aims to follow
semantic versioning. No version is released yet.

## Unreleased

### Added

- **PDF pages** (feature `pdf`). `hayro` rasterizes each page on the CPU at
  144 pixels per inch, at the scalar SIMD level, on one thread. Six CI
  runners agree on the digests.
- **The native window** (feature `view`). `chrys view` shows a pair side by
  side, as an overlay or as a difference map. A person pans, zooms, steps
  through the frames, and accepts or rejects. An accept writes the baseline.
- **The GPU mirror** (feature `view`). A wgpu compute pass finds the changed
  pixels for display only. `chrys gpu-check` compares its regions with the
  CPU verdict. It agrees on Apple M5. The proof on three GPU vendors is open.
- **AV1 video** (feature `video`). `re_rav1d` decodes AV1 in IVF in software.
  Two videos pair by presentation time, so a frame rate change alone is not
  a change. The colour conversion reads the matrix and the range tags, and a
  stream with no matrix tag is refused. The decoded planes equal libdav1d on
  each fixture.
- **3D meshes** (feature `mesh`). OBJ and STL render through a fixed rig of
  eight views, with a rasterizer written in this repository. The output names
  the blind spot: the triangles that no view sees.
- A boundary test for each optional feature. The engine cannot reach it, and
  the default binary does not hold it.
- A CI job that runs each test, with each feature on, on Linux, macOS and
  Windows.
- `docs/DESIGN.md`, `docs/DECISIONS.md`, the README and the standard
  repository files.

### Fixed

- A mask path with a root and no drive, such as `/etc/passwd`, or with a
  drive and no root, such as `C:mask.png`, is now refused on Windows. Before,
  `is_absolute` let it through.
- The AV1 decoder runs on a thread with a 64 MiB stack. A debug build on
  Windows overflowed the 1 MiB stack of the main thread.

### Removed

- The GSD planning files. `docs/DESIGN.md` keeps their content.

## Phases 1 to 4 (2026-09-06 to 2026-09-09)

- The comparison engine: registration, residual, labelled regions, the kind
  of each change, the antialiasing filter and the refusal.
- The determinism proof on six runners, three operating systems and two
  architectures.
- Raster images, numbered frame sequences, GIF, APNG and animated WebP.
- The baseline store and its manifest, the TOML rule engine, the report and
  the CI gate.
- SVG through `resvg`, with one font pinned in the repository.
