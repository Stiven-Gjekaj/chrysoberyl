//! The SVG dispatch tests: an SVG file reaches `SvgSource` on its content
//! alone, and no fixture already committed to this repository is stolen
//! from the adapter it already reaches.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_compare(base: &std::path::Path, candidate: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(base)
        .arg(candidate)
        .output()
        .expect("the chrys binary runs")
}

/// Success criterion 1: an SVG pair compares at the same quality a raster
/// pair does, meaning a named change kind and a bounding box, never a
/// bare pixel count. The expected kind is read from what the binary
/// prints for the already-committed, already-recoloured raster pair
/// under `tests/golden/rule-01/`, rather than typed from memory.
#[test]
fn an_svg_pair_reports_a_recoloured_region() {
    let root = repo_root();

    let raster_output = run_compare(
        &root.join("tests/golden/rule-01/base.png"),
        &root.join("tests/golden/rule-01/candidate.png"),
    );
    let raster_stdout = String::from_utf8(raster_output.stdout).expect("stdout is UTF-8");
    let raster_kind_line = raster_stdout
        .lines()
        .find(|line| line.contains("region at"))
        .expect("the raster recolour pair reports at least one region");
    let expected_kind = raster_kind_line
        .split_whitespace()
        .next()
        .expect("a region line starts with its kind");

    let svg_output = run_compare(
        &root.join("tests/golden/formats/svg/base.svg"),
        &root.join("tests/golden/formats/svg/candidate.svg"),
    );
    let svg_stdout = String::from_utf8(svg_output.stdout).expect("stdout is UTF-8");

    // Record the line the SVG run actually printed, so a reader of this
    // test's own output can see it without re-running the binary.
    println!("svg compare stdout:\n{svg_stdout}");

    assert_eq!(
        svg_output.status.code(),
        Some(1),
        "an SVG pair with a real change must exit 1; stdout was:\n{svg_stdout}"
    );
    assert!(
        svg_stdout.contains(expected_kind),
        "expected the SVG run to name the change kind {expected_kind:?}; stdout was:\n{svg_stdout}"
    );
    assert!(
        svg_stdout.contains("x="),
        "expected the SVG run to name a rectangle; stdout was:\n{svg_stdout}"
    );
}

/// An extension-driven dispatch cannot pass this: the same two fixtures,
/// copied to paths with no extension at all, must still compare as SVG
/// and report a change rather than a decode failure.
#[test]
fn an_svg_file_with_no_extension_still_dispatches_to_svg_source() {
    let root = repo_root();
    let dir = std::env::temp_dir();

    let base_no_ext = dir.join("chrys-svg-dispatch-test-base");
    let candidate_no_ext = dir.join("chrys-svg-dispatch-test-candidate");
    std::fs::copy(root.join("tests/golden/formats/svg/base.svg"), &base_no_ext)
        .expect("copy base.svg to an extensionless path");
    std::fs::copy(
        root.join("tests/golden/formats/svg/candidate.svg"),
        &candidate_no_ext,
    )
    .expect("copy candidate.svg to an extensionless path");

    let output = run_compare(&base_no_ext, &candidate_no_ext);

    std::fs::remove_file(&base_no_ext).ok();
    std::fs::remove_file(&candidate_no_ext).ok();

    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!(
        output.status.code(),
        Some(1),
        "an extensionless SVG pair must still dispatch to SvgSource and report a change; stdout was:\n{stdout}"
    );
    assert!(
        stdout.contains("region at"),
        "expected a change verdict, not a decode failure; stdout was:\n{stdout}"
    );
}

/// A sniff that says yes to everything passes a one-file test and fails
/// this one: `looks_like_svg` must answer false for every committed still
/// and animated fixture this repository already carries.
#[test]
fn no_committed_fixture_is_taken_from_the_adapter_it_already_used() {
    let root = repo_root();
    let formats = root.join("tests/golden/formats");

    let candidates = [
        formats.join("png/base.png"),
        formats.join("png/candidate.png"),
        formats.join("jpeg/base.jpg"),
        formats.join("jpeg/candidate.jpg"),
        formats.join("webp/base.webp"),
        formats.join("webp/candidate.webp"),
        formats.join("tiff/base.tif"),
        formats.join("tiff/candidate.tif"),
        formats.join("gif/base.gif"),
        formats.join("gif/candidate.gif"),
        formats.join("apng/base.png"),
        formats.join("apng/candidate.png"),
        formats.join("webp-anim/base.webp"),
        formats.join("webp-anim/candidate.webp"),
    ];

    for path in candidates {
        assert!(
            path.is_file(),
            "fixture {} must exist for this guard to check anything",
            path.display()
        );
        let sniffed = chrys_source_svg::looks_like_svg(&path)
            .unwrap_or_else(|error| panic!("sniffing {}: {error}", path.display()));
        assert!(
            !sniffed,
            "{} sniffed as an SVG document, which would steal it from its own adapter",
            path.display()
        );
    }
}
