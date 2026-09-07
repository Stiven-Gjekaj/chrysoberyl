//! A guard that `chrys-source` declares no dependency.
//!
//! This is checked by a hand line scan of the crate's own `Cargo.toml`,
//! not by a TOML crate: adding a TOML dependency to this crate to prove it
//! has none would be its own answer. The invariant it protects is not
//! obvious from the code alone: Task 1 deliberately kept the hints sidecar
//! parser out of `chrys-source` and out of a derive on `RegionHint`,
//! specifically so `serde` never enters `chrys-core`'s own dependency
//! graph through this crate.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn chrys_source_declares_no_dependency() {
    let manifest_path = repo_root().join("crates/chrys-source/Cargo.toml");
    let text = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", manifest_path.display()));

    let lines: Vec<&str> = text.lines().collect();

    let dependencies_header = lines
        .iter()
        .position(|line| line.trim() == "[dependencies]")
        .unwrap_or_else(|| {
            panic!(
                "{} holds no [dependencies] section to check",
                manifest_path.display()
            )
        });

    let mut offending: Vec<(usize, &str)> = Vec::new();
    for (offset, line) in lines[dependencies_header + 1..].iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            // The next table header: the [dependencies] section has ended.
            break;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // A 1-based line number: dependencies_header is 0-based and points
        // at the header line itself, so the first line of the section is
        // dependencies_header + 2 in 1-based terms.
        offending.push((dependencies_header + 2 + offset, line));
    }

    assert!(
        offending.is_empty(),
        "{} declares a dependency, but chrys-source is a dependency of chrys-core and must \
         depend on nothing outside the standard library. Offending line(s): {}",
        manifest_path.display(),
        offending
            .iter()
            .map(|(number, line)| format!("line {number}: {}", line.trim()))
            .collect::<Vec<_>>()
            .join("; ")
    );
}
