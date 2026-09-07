//! Integration test over the committed `tests/golden/sequence-01` fixture.
//! This proves the order a `SequenceSource` loads in, not only the count,
//! and proves the limits a `SequenceSource` refuses over.

use std::path::PathBuf;

use chrys_source::Source;
use chrys_source_raster::RasterSource;
use chrys_source_sequence::{SequenceLimits, SequenceSource};

fn fixture_dir(side: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/golden/sequence-01")
        .join(side)
}

#[test]
fn loads_eleven_frames_in_natural_filename_order() {
    let dir = fixture_dir("base");
    let frames = SequenceSource::new()
        .load(&dir)
        .expect("load the base fixture");

    assert_eq!(frames.len(), 11);
    for (position, frame) in frames.iter().enumerate() {
        assert_eq!(frame.index, position);
    }

    // frame2.png is the second file in natural filename order, at index 1.
    // Decode it directly to prove the frame at index 1 holds that file's
    // own canvas, not only that eleven frames loaded.
    let frame2_direct = RasterSource::new()
        .load(&dir.join("frame2.png"))
        .expect("decode frame2.png directly")
        .remove(0);
    assert_eq!(frames[1].pixels, frame2_direct.pixels);
    assert_eq!(frames[1].width, frame2_direct.width);
    assert_eq!(frames[1].height, frame2_direct.height);
}

#[test]
fn load_named_pairs_each_frame_with_its_own_file_name_in_order() {
    let dir = fixture_dir("base");
    let named = SequenceSource::new()
        .load_named(&dir)
        .expect("load_named the base fixture");

    assert_eq!(named.len(), 11);
    let names: Vec<&str> = named.iter().map(|(name, _frame)| name.as_str()).collect();
    assert_eq!(names[0], "frame1.png");
    assert_eq!(names[1], "frame2.png");
    assert_eq!(names[10], "frame11.png");

    // The pairing carries the same frames `load` returns, in the same
    // order, so load_named is not a second, independently computed view.
    let frames = SequenceSource::new()
        .load(&dir)
        .expect("load the base fixture");
    for (position, (_name, frame)) in named.iter().enumerate() {
        assert_eq!(frame.pixels, frames[position].pixels);
        assert_eq!(frame.index, position);
    }
}

#[test]
fn a_frame_count_limit_below_the_fixture_size_refuses_and_names_the_limit() {
    let dir = fixture_dir("base");
    let source = SequenceSource::with_limits(SequenceLimits {
        max_frames: 3,
        ..SequenceLimits::default()
    });

    let error = source
        .load(&dir)
        .expect_err("max_frames: 3 must refuse an 11-frame directory");
    let text = error.to_string();
    assert!(
        text.contains('3'),
        "error text does not name the limit: {text}"
    );
}
