//! Every crate in this workspace forbids unsafe code, and this guard is
//! what keeps that true as crates are added.
//!
//! Phase 3 added two crates. One of them, `chrys-rule`, shipped without
//! `#![forbid(unsafe_code)]` while the other seven crates carried it. No
//! test failed, no lint fired, and the gap was found only by counting the
//! crates by hand during the phase's own security review. A count that a
//! person has to remember to run is not a guard, so this file runs it.
//!
//! This lives in `chrys-cli` rather than in `chrys-core` for the same
//! reason the decode-site guard beside it does: `chrys-cli` is the crate
//! that already depends on every other one, and phase 3 forbids a change
//! to any file under `crates/chrys-core/`.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The crate root a crate declares its lints in: `src/lib.rs` for a
/// library, `src/main.rs` for a binary. A crate with both is a library
/// whose binary is a thin wrapper, and the library root is the one that
/// carries the attribute for the code that matters.
fn crate_root(crate_dir: &Path) -> Option<PathBuf> {
    let lib = crate_dir.join("src/lib.rs");
    if lib.is_file() {
        return Some(lib);
    }
    let main = crate_dir.join("src/main.rs");
    if main.is_file() {
        return Some(main);
    }
    None
}

#[test]
fn every_crate_in_the_workspace_forbids_unsafe_code() {
    let crates_dir = repo_root().join("crates");
    let entries = std::fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", crates_dir.display()));

    let mut checked = 0usize;
    let mut missing: Vec<String> = Vec::new();

    for entry in entries {
        let entry = entry.expect("read a directory entry");
        if !entry.file_type().expect("read a file type").is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(root) = crate_root(&entry.path()) else {
            // A directory under `crates/` with no crate root is not a
            // crate. Failing here would make an unrelated stray directory
            // fail this guard for the wrong reason.
            continue;
        };
        checked += 1;
        let source = std::fs::read_to_string(&root)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", root.display()));
        if !source.contains("#![forbid(unsafe_code)]") {
            missing.push(format!("{name} ({})", root.display()));
        }
    }

    // A guard that checks nothing passes. The workspace has eight crates;
    // fewer than that means this walk stopped finding them, which is its
    // own defect and must not read as a pass.
    assert!(
        checked >= 8,
        "the guard found only {checked} crate roots under {}, so it is not checking what it claims",
        crates_dir.display()
    );

    assert!(
        missing.is_empty(),
        "these crates do not forbid unsafe code: {}",
        missing.join(", ")
    );
}
