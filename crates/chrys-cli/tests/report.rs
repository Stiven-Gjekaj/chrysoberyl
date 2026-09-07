//! Integration tests for `chrys compare --report <PATH>`.
//!
//! Every test here runs the real built binary through
//! `Command::new(env!("CARGO_BIN_EXE_chrys"))`, the same pattern
//! `digest.rs` already uses, and parses the report file it writes with
//! `toml` rather than searching its text: a substring search would also
//! pass on a file whose structure is wrong.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh, unique report path under the system temp directory, so this
/// test module's own runs cannot collide with each other or with another
/// test module's own temporary files.
fn temp_report_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "chrys-cli-report-test-{label}-{}.toml",
        std::process::id()
    ))
}

#[test]
fn the_report_names_kind_region_and_size_per_change() {
    let pair_dir = repo_root().join("tests/golden/rule-01");
    let report_path = temp_report_path("kind-region-size");

    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .arg("--report")
        .arg(&report_path)
        .output()
        .expect("the chrys binary runs");

    let text = std::fs::read_to_string(&report_path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", report_path.display()));
    std::fs::remove_file(&report_path).ok();

    assert!(
        output.status.success() || output.status.code() == Some(1),
        "expected the compare to run to completion, got {:?}",
        output.status.code()
    );

    let document: toml::Value = toml::from_str(&text).expect("the report parses as TOML");
    let frames = document
        .get("frame")
        .and_then(toml::Value::as_array)
        .expect("the report holds a frame array");
    assert_eq!(frames.len(), 1, "a single-file pair reports one frame");

    let changes = frames[0]
        .get("change")
        .and_then(toml::Value::as_array)
        .expect("the changed frame holds a change array");
    assert_eq!(changes.len(), 1, "the rule-01 fixture holds one region");

    let change = &changes[0];
    assert_eq!(
        change.get("kind").and_then(toml::Value::as_str),
        Some("recoloured")
    );
    assert_eq!(
        change.get("region").and_then(toml::Value::as_str),
        Some("badge"),
        "the change's box falls inside the badge hint"
    );
    let width = change
        .get("width")
        .and_then(toml::Value::as_integer)
        .expect("the change carries a width");
    let height = change
        .get("height")
        .and_then(toml::Value::as_integer)
        .expect("the change carries a height");
    let size = change
        .get("size")
        .and_then(toml::Value::as_integer)
        .expect("the change carries a size");
    assert_eq!(size, width * height, "size is width times height");
}

#[test]
fn the_report_is_written_when_the_run_exits_non_zero() {
    let pair_dir = repo_root().join("tests/golden/rule-01");
    let report_path = temp_report_path("nonzero-exit");

    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .arg("--report")
        .arg(&report_path)
        .output()
        .expect("the chrys binary runs");

    assert_eq!(
        output.status.code(),
        Some(1),
        "the rule-01 pair changes with no --rule given, so this exits 1"
    );

    let text = std::fs::read_to_string(&report_path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", report_path.display()));
    std::fs::remove_file(&report_path).ok();

    let _: toml::Value =
        toml::from_str(&text).expect("the report written on a non-zero exit still parses as TOML");
}

#[test]
fn a_refused_pair_is_reported_with_its_reason() {
    let pair_dir = repo_root().join("tests/golden/refuse-01/should-refuse/pair-01");
    let report_path = temp_report_path("refused");

    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .arg("--report")
        .arg(&report_path)
        .output()
        .expect("the chrys binary runs");

    assert_eq!(output.status.code(), Some(2), "a refusal exits 2");

    let text = std::fs::read_to_string(&report_path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", report_path.display()));
    std::fs::remove_file(&report_path).ok();

    let document: toml::Value = toml::from_str(&text).expect("the report parses as TOML");
    let frames = document
        .get("frame")
        .and_then(toml::Value::as_array)
        .expect("the report holds a frame array");
    assert_eq!(frames.len(), 1);

    let frame = &frames[0];
    assert_eq!(
        frame.get("verdict").and_then(toml::Value::as_str),
        Some("refused")
    );
    let reason = frame
        .get("reason")
        .and_then(toml::Value::as_str)
        .expect("a refused frame names its reason");
    assert!(!reason.is_empty(), "the reason is not an empty string");
    assert!(
        frame.get("change").is_none(),
        "a refusal is not a change, and holds no change table"
    );
}

#[test]
fn the_rule_outcome_is_absent_without_a_rule_file() {
    let pair_dir = repo_root().join("tests/golden/rule-01");
    let no_rule_report = temp_report_path("no-rule");
    let with_rule_report = temp_report_path("with-rule");

    Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .arg("--report")
        .arg(&no_rule_report)
        .output()
        .expect("the chrys binary runs");

    let without_rule_text = std::fs::read_to_string(&no_rule_report)
        .unwrap_or_else(|error| panic!("reading {}: {error}", no_rule_report.display()));
    std::fs::remove_file(&no_rule_report).ok();

    assert!(
        !without_rule_text.contains("rule_outcome"),
        "no --rule was given, so no change should carry a rule_outcome key at all:\n{without_rule_text}"
    );

    Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .arg("--rule")
        .arg(pair_dir.join("tolerate.toml"))
        .arg("--report")
        .arg(&with_rule_report)
        .output()
        .expect("the chrys binary runs");

    let with_rule_text = std::fs::read_to_string(&with_rule_report)
        .unwrap_or_else(|error| panic!("reading {}: {error}", with_rule_report.display()));
    std::fs::remove_file(&with_rule_report).ok();

    let document: toml::Value = toml::from_str(&with_rule_text).expect("the report parses");
    let frames = document
        .get("frame")
        .and_then(toml::Value::as_array)
        .expect("the report holds a frame array");
    let changes = frames[0]
        .get("change")
        .and_then(toml::Value::as_array)
        .expect("the changed frame holds a change array");
    assert!(!changes.is_empty());
    for change in changes {
        assert!(
            change
                .get("rule_outcome")
                .and_then(toml::Value::as_str)
                .is_some(),
            "--rule was given, so every change should carry a rule_outcome:\n{with_rule_text}"
        );
    }
}

#[test]
fn a_report_path_that_cannot_be_written_fails_loudly() {
    let pair_dir = repo_root().join("tests/golden/rule-01");
    let report_path = PathBuf::from("/no/such/directory/chrys-report-test.toml");

    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(pair_dir.join("base.png"))
        .arg(pair_dir.join("candidate.png"))
        .arg("--report")
        .arg(&report_path)
        .output()
        .expect("the chrys binary runs");

    assert!(
        !output.status.success(),
        "a report path in a directory that does not exist must fail the run"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("/no/such/directory/chrys-report-test.toml"),
        "stderr does not name the path that could not be written: {stderr}"
    );
}
