//! The boundary of the PDF path, read from the resolved dependency graph.
//!
//! 1. No PDF crate is in the dependency graph of the engine.
//! 2. No PDF crate is in the default build of the binary.
//!
//! Each test reads `cargo tree`, not a manifest. A manifest says what was
//! asked for. The resolved graph says what was built.
//! Each test fails, and does not skip, when `cargo` cannot run.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The crates that only the PDF path brings in.
const PDF_CRATES: &[&str] = &[
    "hayro",
    "hayro-interpret",
    "hayro-syntax",
    "vello_cpu",
    "vello_common",
    "fearless_simd",
    "chrys-source-pdf",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Return the name of each crate in the normal dependency graph of
/// `package`, built with `extra` arguments.
fn graph(package: &str, extra: &[&str]) -> Vec<String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(&cargo)
        .args(["tree", "-p", package, "-e", "normal", "--prefix", "none"])
        .args(["--format", "{p}"])
        .args(extra)
        .current_dir(workspace_root())
        .output()
        .unwrap_or_else(|error| panic!("this guard needs `cargo tree` ({error})"));
    assert!(
        output.status.success(),
        "`cargo tree` failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let names: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_string)
        .collect();
    assert!(!names.is_empty(), "`cargo tree` printed no crate");
    names
}

fn pdf_crates_in(names: &[String]) -> Vec<&String> {
    names
        .iter()
        .filter(|name| PDF_CRATES.contains(&name.as_str()))
        .collect()
}

#[test]
fn the_engine_graph_holds_no_pdf_crate() {
    let names = graph("chrys-core", &[]);
    assert_eq!(pdf_crates_in(&names), Vec::<&String>::new());
}

#[test]
fn the_default_binary_holds_no_pdf_crate() {
    let names = graph("chrys-cli", &[]);
    assert_eq!(pdf_crates_in(&names), Vec::<&String>::new());
}

#[test]
fn the_pdf_feature_brings_in_the_pdf_crates() {
    // This test proves that the two tests above can fail. Without it, a
    // name list with a typing error passes for the wrong reason.
    let names = graph("chrys-cli", &["--features", "pdf"]);
    assert!(names.iter().any(|name| name == "hayro"), "{names:?}");
    assert!(
        names.iter().any(|name| name == "chrys-source-pdf"),
        "{names:?}"
    );
}
