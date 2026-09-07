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

/// Every frame of a container file's `load_named` names the file the
/// whole container came from: an animation's frames are composited out of
/// one file and have no name of their own more specific than that.
#[test]
fn load_named_names_every_frame_after_the_containers_own_file() {
    let path = golden_root().join("formats").join("gif").join("base.gif");
    let source = AnimationSource::new();
    let named = source
        .load_named(&path)
        .expect("load_named the committed gif fixture");

    assert_eq!(named.len(), 5);
    for (name, _frame) in &named {
        assert_eq!(name, "base.gif");
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

#[test]
fn webp_anim() {
    let path = golden_root()
        .join("formats")
        .join("webp-anim")
        .join("base.webp");
    let source = AnimationSource::new();
    let frames = source
        .load(&path)
        .expect("load and re-verify the committed webp-anim fixture");

    assert_five_composited_frames(&frames);
}

#[test]
fn is_animation_is_true_for_the_animated_webp_fixture_and_false_for_a_still_webp() {
    let animated_path = golden_root()
        .join("formats")
        .join("webp-anim")
        .join("base.webp");
    let still_path = golden_root().join("formats").join("webp").join("base.webp");

    assert!(
        chrys_source_animation::sniff::is_animation(&animated_path)
            .expect("sniff the committed animated webp fixture")
    );
    assert!(
        !chrys_source_animation::sniff::is_animation(&still_path)
            .expect("sniff the committed still webp fixture")
    );
}

#[test]
fn frame_zero_of_the_gif_and_animated_webp_fixtures_hold_the_same_pixels() {
    let gif_path = golden_root().join("formats").join("gif").join("base.gif");
    let webp_path = golden_root()
        .join("formats")
        .join("webp-anim")
        .join("base.webp");
    let source = AnimationSource::new();

    let gif_frames = source.load(&gif_path).expect("load the gif fixture");
    let webp_frames = source.load(&webp_path).expect("load the webp-anim fixture");

    assert_eq!(gif_frames[0].width, webp_frames[0].width);
    assert_eq!(gif_frames[0].height, webp_frames[0].height);
    assert_eq!(
        gif_frames[0].pixels, webp_frames[0].pixels,
        "frame 0 of the gif and animated webp fixtures should hold identical pixels: both \
         were built from the same source frame, and both formats are lossless here"
    );
}
