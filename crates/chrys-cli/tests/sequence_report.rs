//! The observable proof that a sequence report stays readable at length.
//!
//! Phase 2 acceptance test 12 failed on the committed shape: a hundred
//! frames with five changes printed two hundred lines, of which five
//! carried a verdict. A reader found the five only by piping the output
//! through another tool. These tests hold the shape that replaced it.
//!
//! They run the real binary over the committed eleven-frame pair rather
//! than calling a function, because the thing under test is what a person
//! reads on a terminal, not what a function returns.

use std::path::PathBuf;
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_compare(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .args(args)
        .output()
        .expect("the chrys binary runs")
}

/// The committed eleven-frame pair, whose frame 4 is the only one that
/// differs. `tests/golden/sequence-01` is the same fixture the determinism
/// workflow hashes on all six runners.
fn sequence_paths() -> (String, String) {
    let root = repo_root();
    (
        root.join("tests/golden/sequence-01/base")
            .to_str()
            .expect("base path is UTF-8")
            .to_string(),
        root.join("tests/golden/sequence-01/candidate")
            .to_str()
            .expect("candidate path is UTF-8")
            .to_string(),
    )
}

#[test]
fn a_sequence_prints_only_the_frames_that_changed() {
    let (base, candidate) = sequence_paths();
    let output = run_compare(&[&base, &candidate]);
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");

    // One frame of eleven differs, so exactly one header may appear. A
    // header per frame is the shape this test exists to keep out.
    let headers = stdout.lines().filter(|l| l.starts_with("frame ")).count();
    assert_eq!(
        headers, 1,
        "only the changed frame prints a header, got:\n{stdout}"
    );

    assert!(
        !stdout.lines().any(|l| l == "identical"),
        "an unchanged frame prints nothing, got:\n{stdout}"
    );

    assert!(
        stdout.contains("frame 4"),
        "the changed frame is named, got:\n{stdout}"
    );
}

#[test]
fn a_sequence_closes_with_a_count_of_every_frame() {
    let (base, candidate) = sequence_paths();
    let output = run_compare(&[&base, &candidate]);
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");

    // The tail states both numbers. A reader who wants to know how much
    // was compared must not have to add the silent frames to the loud
    // ones to find out.
    let last = stdout.lines().last().expect("stdout is not empty");
    assert_eq!(
        last, "1 of 11 frames changed",
        "the tail names the changed count and the whole count, got:\n{stdout}"
    );
}

#[test]
fn a_sequence_with_no_change_still_says_how_much_it_compared() {
    // An empty stdout and "0 of 11 frames changed" are different answers.
    // Only one of them tells a reader the tool looked at anything.
    let (base, _) = sequence_paths();
    let output = run_compare(&[&base, &base]);
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");

    assert_eq!(stdout.trim_end(), "0 of 11 frames changed");
    assert_eq!(
        output.status.code(),
        Some(0),
        "a sequence that did not change exits 0"
    );
}

#[test]
fn all_frames_returns_the_line_per_frame_shape() {
    // A caller that already parses a line per frame keeps it behind the
    // flag, so this change removes nothing that existed.
    let (base, candidate) = sequence_paths();
    let output = run_compare(&[&base, &candidate, "--all-frames"]);
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");

    let headers = stdout.lines().filter(|l| l.starts_with("frame ")).count();
    assert_eq!(headers, 11, "every frame prints a header, got:\n{stdout}");

    let identical = stdout.lines().filter(|l| *l == "identical").count();
    assert_eq!(
        identical, 10,
        "the ten unchanged frames each print identical, got:\n{stdout}"
    );

    assert!(
        !stdout.contains("frames changed"),
        "the per-frame shape prints no tail, got:\n{stdout}"
    );
}

#[test]
fn the_digest_report_still_covers_every_frame() {
    // The digest report is the determinism evidence. It must not depend on
    // which frames happened to change, so `--hash-only` reports all
    // eleven whether or not they differ.
    let (base, candidate) = sequence_paths();
    let output = run_compare(&[&base, &candidate, "--hash-only"]);
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");

    let headers = stdout.lines().filter(|l| l.starts_with("frame ")).count();
    assert_eq!(
        headers, 11,
        "the digest report covers every frame, got:\n{stdout}"
    );

    assert!(
        !stdout.contains("frames changed"),
        "the digest report prints no tail, got:\n{stdout}"
    );
}

#[test]
fn a_single_pair_prints_neither_a_header_nor_a_tail() {
    // A one-against-one comparison keeps the exact stdout phase 1 shipped.
    // The sequence shape must not reach it.
    let root = repo_root();
    let base = root.join("tests/golden/pair-01/base.png");
    let candidate = root.join("tests/golden/pair-01/candidate.png");
    let output = run_compare(&[
        base.to_str().expect("base path is UTF-8"),
        candidate.to_str().expect("candidate path is UTF-8"),
    ]);
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");

    assert!(!stdout.contains("frame "), "got:\n{stdout}");
    assert!(!stdout.contains("frames changed"), "got:\n{stdout}");
    assert!(stdout.starts_with("Recoloured region at"), "got:\n{stdout}");
}
