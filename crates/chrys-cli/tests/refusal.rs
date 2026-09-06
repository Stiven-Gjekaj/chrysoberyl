//! The CORE-06/CORE-07 refusal, as the `chrys` binary itself reports it.
//!
//! `crates/chrys-core/tests/refusal.rs` covers the refusal decision at the
//! library level: the measured corpus, the separation test, and
//! `compare`'s own `Verdict::Refused`. This file covers the one piece that
//! test cannot reach: `CARGO_BIN_EXE_chrys` is only set for a package that
//! itself declares that binary target, and that package is `chrys-cli`.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_cli_exits_2_and_names_the_refusal_on_a_should_refuse_pair() {
    let pair_dir = repo_root().join("tests/golden/refuse-01/should-refuse/pair-01");
    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .output()
        .expect("the chrys binary runs");

    assert_eq!(output.status.code(), Some(2), "expected exit code 2");
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(
        stdout.to_lowercase().contains("too different"),
        "stdout did not name the refusal: {stdout}"
    );
}

#[test]
fn the_cli_exits_1_on_a_should_register_pair() {
    let pair_dir = repo_root().join("tests/golden/refuse-01/should-register/pair-05");
    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .output()
        .expect("the chrys binary runs");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 (changed), not a refusal"
    );
}
