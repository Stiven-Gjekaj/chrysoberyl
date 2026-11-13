//! The video dispatch tests.
//!
//! With the `video` feature, two AV1 files compare by presentation time.
//! Without it, the binary refuses a video and names the feature.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/formats/av1")
        .join(name)
}

fn compare(base: &str, candidate: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(fixture(base))
        .arg(fixture(candidate))
        .output()
        .expect("the chrys binary runs")
}

#[cfg(feature = "video")]
#[test]
fn a_video_pair_reports_the_changed_frames() {
    let output = compare("base.ivf", "candidate.ivf");
    let stdout = String::from_utf8(output.stdout).expect("UTF-8");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.ends_with("4 of 8 frames changed\n"), "{stdout}");
}

#[cfg(feature = "video")]
#[test]
fn a_frame_rate_change_alone_exits_zero() {
    let output = compare("base.ivf", "double.ivf");
    let stdout = String::from_utf8(output.stdout).expect("UTF-8");
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert_eq!(stdout, "0 of 16 frames changed\n");
}

#[cfg(feature = "video")]
#[test]
fn a_video_against_a_still_image_is_refused() {
    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(fixture("base.ivf"))
        .arg(fixture("../png/base.png"))
        .output()
        .expect("the chrys binary runs");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8");
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("not both videos"), "{stderr}");
}

#[cfg(not(feature = "video"))]
#[test]
fn a_binary_without_the_feature_refuses_a_video_and_names_the_feature() {
    let output = compare("base.ivf", "candidate.ivf");
    let stderr = String::from_utf8(output.stderr).expect("UTF-8");
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("`video` feature"), "{stderr}");
}
