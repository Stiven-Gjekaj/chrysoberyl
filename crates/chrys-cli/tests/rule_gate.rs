//! The three-way exit-code proof over the committed `rule-01` fixture: a
//! rule file, read as data, decides whether a real comparison passes.

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

#[test]
fn exits_zero_when_every_change_is_tolerated() {
    let root = repo_root();
    let base = root.join("tests/golden/rule-01/base.png");
    let candidate = root.join("tests/golden/rule-01/candidate.png");
    let tolerate = root.join("tests/golden/rule-01/tolerate.toml");

    let output = run_compare(&[
        base.to_str().expect("base path is UTF-8"),
        candidate.to_str().expect("candidate path is UTF-8"),
        "--rule",
        tolerate.to_str().expect("rule path is UTF-8"),
    ]);

    assert!(
        output.status.success(),
        "compare --rule tolerate.toml should exit 0, but exited {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(
        stdout.contains("Recoloured"),
        "stdout should still print the verdict text, not suppress it: {stdout}"
    );
}

#[test]
fn exits_nonzero_when_a_change_has_no_tolerating_rule() {
    let root = repo_root();
    let base = root.join("tests/golden/rule-01/base.png");
    let candidate = root.join("tests/golden/rule-01/candidate.png");
    let too_tight = root.join("tests/golden/rule-01/too-tight.toml");

    let ruled_output = run_compare(&[
        base.to_str().expect("base path is UTF-8"),
        candidate.to_str().expect("candidate path is UTF-8"),
        "--rule",
        too_tight.to_str().expect("rule path is UTF-8"),
    ]);
    assert!(
        !ruled_output.status.success(),
        "compare --rule too-tight.toml should not exit 0, which would mean the gate passes \
         whatever the tolerance says"
    );
    let ruled_stdout = String::from_utf8(ruled_output.stdout).expect("stdout is UTF-8");
    assert!(
        ruled_stdout.contains("Recoloured"),
        "stdout should still print the verdict text: {ruled_stdout}"
    );

    // The same pair with no --rule at all must also exit non-zero, so this
    // test holds the evidence that the flag changed the answer rather
    // than that the pair happened to fail regardless.
    let unruled_output = run_compare(&[
        base.to_str().expect("base path is UTF-8"),
        candidate.to_str().expect("candidate path is UTF-8"),
    ]);
    assert!(
        !unruled_output.status.success(),
        "the unruled pair should also exit non-zero: this fixture must report a change with no \
         rule file, or it proves nothing about what a rule tolerates"
    );
}
