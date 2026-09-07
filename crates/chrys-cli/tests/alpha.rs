//! The end-to-end regression test for gap G-01-1: a pair whose colour
//! bytes are identical and whose alpha bytes are not must not be reported
//! `identical`.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn an_alpha_only_difference_is_not_reported_identical() {
    let pair_dir = repo_root().join("tests/golden/alpha-01");
    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .output()
        .expect("the chrys binary runs");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 (changed)"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(
        !stdout.to_lowercase().contains("identical"),
        "an alpha-only difference must not be reported identical: {stdout}"
    );
    assert!(
        stdout.contains("x=48, y=48, width=24, height=20"),
        "the bounding box must match the fixture's own hole: {stdout}"
    );
}

/// The exact self-contradiction 01-REVIEW.md named in CR-01, now resolved
/// in the other direction: the two sides' decode digests differ (the
/// alpha bytes are different bytes), and the run must agree that
/// something changed, rather than reporting `identical` alongside two
/// decode digests that plainly do not match.
#[test]
fn the_decode_digests_differ_and_the_run_agrees_something_changed() {
    let pair_dir = repo_root().join("tests/golden/alpha-01");

    let hash_output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .arg("--hash-only")
        .output()
        .expect("the chrys binary runs");
    assert!(hash_output.status.success(), "--hash-only exits 0");
    let hash_stdout = String::from_utf8(hash_output.stdout).expect("stdout is UTF-8");
    let lines: Vec<&str> = hash_stdout.lines().collect();
    assert_eq!(lines.len(), 4, "the digest report has exactly four lines");
    assert_ne!(
        lines[0], lines[1],
        "decode-base and decode-candidate must differ: the two files' \
         alpha bytes are different bytes"
    );

    let compare_output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .output()
        .expect("the chrys binary runs");
    assert_eq!(
        compare_output.status.code(),
        Some(1),
        "a pair whose decode digests differ must report a change, not identical"
    );
}
