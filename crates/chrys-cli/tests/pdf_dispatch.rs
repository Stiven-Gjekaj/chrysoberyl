//! The PDF dispatch tests.
//!
//! With the `pdf` feature, a PDF pair compares page by page.
//! Without it, the binary refuses a PDF file and names the feature.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/formats/pdf")
        .join(name)
}

fn compare(base: &std::path::Path, candidate: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(base)
        .arg(candidate)
        .output()
        .expect("the chrys binary runs")
}

#[cfg(feature = "pdf")]
#[test]
fn a_pdf_pair_reports_the_changed_page_only() {
    let output = compare(&fixture("base.pdf"), &fixture("candidate.pdf"));
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!(output.status.code(), Some(1), "stdout was:\n{stdout}");
    assert!(stdout.contains("frame 0"), "stdout was:\n{stdout}");
    assert!(!stdout.contains("frame 1"), "stdout was:\n{stdout}");
    assert!(
        stdout.contains("1 of 2 frames changed"),
        "stdout was:\n{stdout}"
    );
}

#[cfg(feature = "pdf")]
#[test]
fn the_dispatch_reads_the_content_not_the_extension() {
    let dir = std::env::temp_dir().join(format!("chrys-pdf-dispatch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("make the scratch directory");
    let base = dir.join("base.bin");
    let candidate = dir.join("candidate.bin");
    std::fs::copy(fixture("base.pdf"), &base).expect("copy base");
    std::fs::copy(fixture("candidate.pdf"), &candidate).expect("copy candidate");

    let output = compare(&base, &candidate);
    std::fs::remove_dir_all(&dir).ok();
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(
        stdout.contains("1 of 2 frames changed"),
        "stdout was:\n{stdout}"
    );
}

#[cfg(not(feature = "pdf"))]
#[test]
fn a_binary_without_the_feature_refuses_a_pdf_and_names_the_feature() {
    let output = compare(&fixture("base.pdf"), &fixture("candidate.pdf"));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert_eq!(output.status.code(), Some(3), "stderr was:\n{stderr}");
    assert!(stderr.contains("`pdf` feature"), "stderr was:\n{stderr}");
}
