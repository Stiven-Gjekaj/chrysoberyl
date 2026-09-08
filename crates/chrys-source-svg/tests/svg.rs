//! Integration tests for `SvgSource`, run through its public `Source`
//! implementation rather than against any private helper.

use std::path::PathBuf;

use chrys_source::Source;
use chrys_source_svg::SvgSource;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> PathBuf {
    repo_root().join("tests/golden/formats/svg").join(name)
}

#[test]
fn identical_svgs_produce_identical_frames() {
    let source = SvgSource::new();

    let base_path = fixture("base.svg");
    let first = source
        .load(&base_path)
        .expect("base.svg decodes")
        .into_iter()
        .next()
        .expect("base.svg decodes to one frame");
    let second = source
        .load(&base_path)
        .expect("base.svg decodes a second time")
        .into_iter()
        .next()
        .expect("base.svg decodes to one frame a second time");

    assert_eq!(
        first, second,
        "loading the same SVG twice must be pixel-for-pixel identical"
    );
    assert_eq!(first.width, 256);
    assert_eq!(first.height, 256);
    assert_eq!(first.pixels.len(), 256 * 256 * 4);

    let candidate_path = fixture("candidate.svg");
    let candidate = source
        .load(&candidate_path)
        .expect("candidate.svg decodes")
        .into_iter()
        .next()
        .expect("candidate.svg decodes to one frame");

    assert_ne!(
        first.pixels, candidate.pixels,
        "the recoloured rectangle must change the decoded pixel buffer"
    );
}
