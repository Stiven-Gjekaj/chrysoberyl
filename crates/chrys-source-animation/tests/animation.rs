//! Integration tests for `chrys-source-animation`, reading the committed
//! fixtures under `tests/golden/formats/`.

use std::path::PathBuf;

use chrys_source::Source;
use chrys_source_animation::{AnimationError, AnimationLimits, AnimationSource};

fn golden_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("golden")
}

fn assert_five_composited_frames(frames: &[chrys_source::Frame]) {
    assert_eq!(frames.len(), 5);
    for (expected_index, frame) in frames.iter().enumerate() {
        assert_eq!(frame.index, expected_index);
        assert_eq!(
            frame.pixels.len(),
            (frame.width as usize) * (frame.height as usize) * 4,
            "frame {expected_index} is not a full-canvas RGBA8 buffer"
        );
    }
}

#[test]
fn gif() {
    let path = golden_root().join("formats").join("gif").join("base.gif");
    let source = AnimationSource::new();
    let frames = source.load(&path).expect("load the committed gif fixture");

    assert_five_composited_frames(&frames);
}

#[test]
fn a_frame_count_limit_below_the_fixture_size_refuses_and_names_the_limit() {
    let path = golden_root().join("formats").join("gif").join("base.gif");
    let source = AnimationSource::with_limits(AnimationLimits {
        max_frames: 2,
        ..AnimationLimits::default()
    });

    match source.load(&path) {
        Err(AnimationError::TooManyFrames { limit, .. }) => assert_eq!(limit, 2),
        other => panic!("expected AnimationError::TooManyFrames, got {other:?}"),
    }
}

#[test]
fn apng() {
    let path = golden_root().join("formats").join("apng").join("base.png");
    let source = AnimationSource::new();
    let frames = source.load(&path).expect("load the committed apng fixture");

    assert_five_composited_frames(&frames);
}

#[test]
fn is_animation_is_true_for_the_apng_fixture_and_false_for_a_still_png() {
    let apng_path = golden_root().join("formats").join("apng").join("base.png");
    let still_path = golden_root().join("formats").join("png").join("base.png");

    assert!(
        chrys_source_animation::sniff::is_animation(&apng_path)
            .expect("sniff the committed apng fixture")
    );
    assert!(
        !chrys_source_animation::sniff::is_animation(&still_path)
            .expect("sniff the committed still png fixture")
    );
}

#[test]
fn frame_zero_of_the_gif_and_apng_fixtures_hold_the_same_pixels() {
    let gif_path = golden_root().join("formats").join("gif").join("base.gif");
    let apng_path = golden_root().join("formats").join("apng").join("base.png");
    let source = AnimationSource::new();

    let gif_frames = source.load(&gif_path).expect("load the gif fixture");
    let apng_frames = source.load(&apng_path).expect("load the apng fixture");

    assert_eq!(gif_frames[0].width, apng_frames[0].width);
    assert_eq!(gif_frames[0].height, apng_frames[0].height);
    assert_eq!(
        gif_frames[0].pixels, apng_frames[0].pixels,
        "frame 0 of the gif and apng fixtures should hold identical pixels: both were built \
         from the same source frame, and both formats are lossless here"
    );
}
