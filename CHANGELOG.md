# Changelog

This file records each notable change to Chrysoberyl. The format follows
Keep a Changelog (https://keepachangelog.com). The project aims to follow
semantic versioning.

## Unreleased

## 1.0.0 (2026-11-13)

The first release. Each input below gives the same verdict on Linux, macOS
and Windows, on x86-64 and aarch64. Six CI runners prove it on each push.

### Added

- **The comparison engine.** It names each change: moved, added, removed,
  recoloured or resized, with a bounding box, an offset and a colour
  difference. It does not report antialiasing. It refuses a pair that is
  not near-identical, and says why.
- **Raster images, frame sequences and animation.** PNG, JPEG, WebP, TIFF,
  a directory of numbered frames, GIF, APNG and animated WebP.
- **SVG** through `resvg`, with one font pinned in the repository.
- **Baselines, rules and a CI gate.** `chrys accept` writes a baseline and a
  manifest of digests. A TOML rule file tolerates a kind of change inside a
  named region or a mask. The exit code gates CI, and `--report` writes a
  TOML report.
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
- Release archives for five platforms, and the API documentation on GitHub
  Pages.

### Fixed

- A mask path with a root and no drive, such as `/etc/passwd`, or with a
  drive and no root, such as `C:mask.png`, is now refused on Windows. Before,
  `is_absolute` let it through.
- The AV1 decoder runs on a thread with a 64 MiB stack. A debug build on
  Windows overflowed the 1 MiB stack of the main thread.

### Removed

- The GSD planning files. `docs/DESIGN.md` keeps their content.
