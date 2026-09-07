//! `--region` together with `--rule`, and `--region` together with
//! `--report`: no test in this workspace exercised either combination
//! before this file. That absence is why CR-01 shipped: a mask-scoped
//! rule's own size check ran against the cropped rectangle instead of
//! the frame the mask was authored against, and a real, untolerated
//! recoloured change (colour delta 115.65) reported exit 0 under
//! `--region` and exit 3 without it, reproduced twice against a fresh
//! build at commit `f227250`.
//!
//! Every assertion here compares an observed exit code (or an observed
//! line) against an expected absolute value, never only two runs against
//! each other: two runs that agree can both still be wrong, and this
//! repository has already recorded once that a test can pass for the
//! wrong reason.

use std::path::PathBuf;
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rule_01_dir() -> PathBuf {
    repo_root().join("tests/golden/rule-01")
}

fn run_compare(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .args(args)
        .output()
        .expect("the chrys binary runs")
}

fn exit_code(output: &Output) -> i32 {
    output
        .status
        .code()
        .unwrap_or_else(|| panic!("the process was terminated by a signal: {output:?}"))
}

/// One rule file, and the exit code `compare` must produce for it, with
/// and without `--region badge`, over the `rule-01` fixture pair.
struct RuleCase {
    rule_file: &'static str,
    expected_exit: i32,
}

const RULE_CASES: [RuleCase; 4] = [
    // The mask is 70x60, the badge hint's own cropped size, not the
    // 256x256 frame: refused in both runs, exactly the property CR-01
    // exists to fix. This is the case that used to exit 0 under
    // `--region`.
    RuleCase {
        rule_file: "mask-cropped-size.toml",
        expected_exit: 3,
    },
    // The mask is 256x256, the frame's own size: matches and forgives
    // the change in both runs.
    RuleCase {
        rule_file: "mask-tolerate.toml",
        expected_exit: 0,
    },
    // A region-scoped rule keeps matching its own region under the flag,
    // rather than silently stopping to match because a cropped frame
    // carries no hints (T-03-28).
    RuleCase {
        rule_file: "tolerate.toml",
        expected_exit: 0,
    },
    // A limit below the measured delta stays a violation either way.
    RuleCase {
        rule_file: "too-tight.toml",
        expected_exit: 1,
    },
];

#[test]
fn a_rule_gives_the_same_exit_code_with_and_without_a_region() {
    let dir = rule_01_dir();
    let base = dir.join("base.png");
    let candidate = dir.join("candidate.png");

    for case in RULE_CASES {
        let rule_path = dir.join(case.rule_file);

        let plain = run_compare(&[
            base.to_str().expect("base path is UTF-8"),
            candidate.to_str().expect("candidate path is UTF-8"),
            "--rule",
            rule_path.to_str().expect("rule path is UTF-8"),
        ]);
        let plain_exit = exit_code(&plain);

        let region = run_compare(&[
            base.to_str().expect("base path is UTF-8"),
            candidate.to_str().expect("candidate path is UTF-8"),
            "--rule",
            rule_path.to_str().expect("rule path is UTF-8"),
            "--region",
            "badge",
        ]);
        let region_exit = exit_code(&region);

        assert_eq!(
            plain_exit,
            case.expected_exit,
            "{}: the whole-frame run exited {plain_exit}, expected {}. stderr: {}",
            case.rule_file,
            case.expected_exit,
            String::from_utf8_lossy(&plain.stderr)
        );
        assert_eq!(
            region_exit,
            case.expected_exit,
            "{}: the --region run exited {region_exit}, expected {}. stderr: {}",
            case.rule_file,
            case.expected_exit,
            String::from_utf8_lossy(&region.stderr)
        );
        assert_eq!(
            plain_exit, region_exit,
            "{}: the whole-frame run exited {plain_exit} but the --region run exited \
             {region_exit}; a rule file must give the same verdict whether or not a region is \
             named",
            case.rule_file
        );
    }
}

#[test]
fn a_region_run_reports_the_rectangle_in_frame_coordinates() {
    let dir = rule_01_dir();
    let base = dir.join("base.png");
    let candidate = dir.join("candidate.png");

    let output = run_compare(&[
        base.to_str().expect("base path is UTF-8"),
        candidate.to_str().expect("candidate path is UTF-8"),
        "--region",
        "badge",
    ]);
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");

    assert!(
        stdout.contains("x=40, y=40"),
        "the --region run should report the change at the base frame's own coordinates \
         (x=40, y=40), not the crop-local ones: {stdout}"
    );
    assert!(
        !stdout.contains("x=10, y=10"),
        "the --region run should not report the crop-local rectangle (x=10, y=10): {stdout}"
    );
}

#[test]
fn a_region_report_names_the_region_each_change_falls_in() {
    let dir = rule_01_dir();
    let base = dir.join("base.png");
    let candidate = dir.join("candidate.png");
    let report_path = std::env::temp_dir().join(format!(
        "chrys-cli-region-rule-test-report-{}.toml",
        std::process::id()
    ));

    let output = run_compare(&[
        base.to_str().expect("base path is UTF-8"),
        candidate.to_str().expect("candidate path is UTF-8"),
        "--region",
        "badge",
        "--report",
        report_path.to_str().expect("report path is UTF-8"),
    ]);
    let text = std::fs::read_to_string(&report_path).unwrap_or_else(|error| {
        panic!(
            "reading {}: {error}; stderr: {}",
            report_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    std::fs::remove_file(&report_path).ok();

    let document: toml::Value = toml::from_str(&text).expect("the report parses as TOML");

    let meta_region = document
        .get("meta")
        .and_then(|meta| meta.get("region"))
        .and_then(toml::Value::as_str);
    assert_eq!(
        meta_region,
        Some("badge"),
        "meta.region should name the region this run compared inside: {text}"
    );

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

    let region_name = changes[0].get("region").and_then(toml::Value::as_str);
    assert_eq!(
        region_name,
        Some("badge"),
        "the one change's own region should be badge: {text}"
    );
}

/// The mask-scoped false-green case, driven directly against the built
/// binary rather than through the harness's own exit-code helper, so the
/// exact reproduction command from `03-VERIFICATION.md` and this plan's
/// own `<verify>` block is proven by an independent test, not only by
/// `a_rule_gives_the_same_exit_code_with_and_without_a_region`'s own loop.
#[test]
fn the_mask_scoped_rule_refuses_the_untolerated_change_under_a_region() {
    let dir = rule_01_dir();
    let base = dir.join("base.png");
    let candidate = dir.join("candidate.png");
    let rule_path = dir.join("mask-cropped-size.toml");

    let output = run_compare(&[
        base.to_str().expect("base path is UTF-8"),
        candidate.to_str().expect("candidate path is UTF-8"),
        "--rule",
        rule_path.to_str().expect("rule path is UTF-8"),
        "--region",
        "badge",
    ]);

    assert_eq!(
        exit_code(&output),
        3,
        "a mask sized to the cropped region must still be refused under --region, naming \
         70x60 against 256x256; an exit of 0 is the reproduced false green itself. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("70x60"),
        "stderr does not name 70x60: {stderr}"
    );
    assert!(
        stderr.contains("256x256"),
        "stderr does not name 256x256: {stderr}"
    );
}
