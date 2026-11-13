//! The mesh dispatch tests.
//!
//! With the `mesh` feature, a mesh pair compares through the eight-view
//! rig, and the output names the blind spot. Without it, the binary
//! refuses a text mesh and names the feature.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/formats/mesh")
        .join(name)
}

fn compare(base: &str, candidate: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(fixture(base))
        .arg(fixture(candidate))
        .args(extra)
        .output()
        .expect("the chrys binary runs")
}

#[cfg(feature = "mesh")]
#[test]
fn a_mesh_pair_reports_views_and_names_the_blind_spot() {
    let output = compare("base.obj", "candidate.obj", &[]);
    let stdout = String::from_utf8(output.stdout).expect("UTF-8");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("of 8 frames changed"), "{stdout}");
    assert!(!stdout.contains("refused"), "{stdout}");
    assert!(
        stdout.contains("base.obj blind spot: 14 of 36 triangles are hidden"),
        "{stdout}"
    );
}

#[cfg(feature = "mesh")]
#[test]
fn the_digest_report_keeps_its_shape_for_a_mesh() {
    let output = compare("base.obj", "candidate.obj", &["--hash-only"]);
    let stdout = String::from_utf8(output.stdout).expect("UTF-8");
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert_eq!(stdout.matches("frame ").count(), 8, "{stdout}");
    assert!(!stdout.contains("blind spot"), "{stdout}");
}

#[cfg(not(feature = "mesh"))]
#[test]
fn a_binary_without_the_feature_refuses_a_mesh_and_names_the_feature() {
    let output = compare("base.obj", "candidate.obj", &[]);
    let stderr = String::from_utf8(output.stderr).expect("UTF-8");
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("`mesh` feature"), "{stderr}");
}
