# Decisions

This file records each choice that the author makes about the scope of
the project.
Each entry gives the date, the choice, and the reason.

## 2026-11-13: The project starts again after phase 4

- Phase 5 uses `hayro`, a pure-Rust PDF rasterizer.
  It does not use `pdfium`.
  Reason: `hayro` keeps binary files out of git.
  It comes from the same author as `resvg`, and `resvg` already passes the
  determinism gate.
- Phase 6 builds the native window and the wgpu path.
  The proof on three GPU vendors stays open.
  This machine has an Apple GPU only.
  The CPU decides each verdict, so the open proof cannot change a verdict.
- The determinism gates run on GitHub Actions.
  The work goes to `main`, and `main` is pushed after each phase.
- The GSD planning files go out of the repository.
  The invariants, the requirements, and the open claims move into
  `docs/DESIGN.md` first.
