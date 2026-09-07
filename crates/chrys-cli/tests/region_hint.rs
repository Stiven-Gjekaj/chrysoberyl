//! The observable proof of SRC-09: the engine registers inside a named
//! region instead of searching the whole frame for it. The difference
//! between the region run and the whole-frame run over the same pair is
//! the evidence, so this test asserts on both runs' exit code and stdout,
//! not on either alone.

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
fn region_hint() {
    let root = repo_root();
    let base = root.join("tests/golden/hint-01/base.png");
    let candidate = root.join("tests/golden/hint-01/candidate.png");

    let region_output = run_compare(&[
        base.to_str().expect("base path is UTF-8"),
        candidate.to_str().expect("candidate path is UTF-8"),
        "--region",
        "logo",
    ]);
    assert!(
        region_output.status.success(),
        "the region run over the named hint should exit 0, but exited {:?}. stderr: {}",
        region_output.status.code(),
        String::from_utf8_lossy(&region_output.stderr)
    );
    let region_stdout = String::from_utf8(region_output.stdout).expect("stdout is UTF-8");
    assert_eq!(
        region_stdout, "identical\n",
        "the region run should print the identical verdict"
    );

    let whole_output = run_compare(&[
        base.to_str().expect("base path is UTF-8"),
        candidate.to_str().expect("candidate path is UTF-8"),
    ]);
    assert!(
        !whole_output.status.success(),
        "the whole-frame run over the same pair should not exit 0, which would mean it agrees \
         with the region run and the fixture proves nothing"
    );
    let whole_stdout = String::from_utf8(whole_output.stdout).expect("stdout is UTF-8");
    assert_ne!(
        whole_stdout, "identical\n",
        "the whole-frame run should not report the same verdict as the region run"
    );
}

#[test]
fn an_unknown_region_name_exits_non_zero_and_names_the_declared_regions() {
    let root = repo_root();
    let base = root.join("tests/golden/hint-01/base.png");
    let candidate = root.join("tests/golden/hint-01/candidate.png");

    let output = run_compare(&[
        base.to_str().expect("base path is UTF-8"),
        candidate.to_str().expect("candidate path is UTF-8"),
        "--region",
        "nope",
    ]);

    assert!(
        !output.status.success(),
        "an unknown region name should exit non-zero"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("logo"),
        "stderr should list the declared region names: {stderr}"
    );
}
