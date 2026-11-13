//! Tests for the video adapter, on the AV1 fixtures.
//!
//! The conformance test compares the decoded planes with the hashes that
//! libdav1d wrote (see scripts/make-video-fixtures.sh). libdav1d is not the
//! decoder in this repository, so a pass means two decoders agree.

use std::path::PathBuf;

use chrys_source::Source;
use chrys_source_video::{VideoError, VideoSource, pair_by_time};
use sha2::{Digest, Sha256};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/formats/av1")
        .join(name)
}

/// Read the reference hashes of `file`, in frame order.
fn reference(file: &str) -> Vec<String> {
    let text = std::fs::read_to_string(fixture("REFERENCE.sha256")).expect("read the reference");
    let hashes: Vec<String> = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|fields| fields.len() == 3 && fields[0] == file)
        .map(|fields| fields[2].to_string())
        .collect();
    assert!(!hashes.is_empty(), "no reference hash for {file}");
    hashes
}

#[test]
fn each_fixture_decodes_to_the_libdav1d_planes() {
    for file in [
        "base.ivf",
        "candidate.ivf",
        "double.ivf",
        "full601.ivf",
        "untagged.ivf",
    ] {
        let decoded = VideoSource::new()
            .decode_planes(&fixture(file))
            .expect("decode");
        let expected = reference(file);
        assert_eq!(decoded.len(), expected.len(), "{file}: frame count");
        for (index, (planes, expected)) in decoded.iter().zip(&expected).enumerate() {
            let mut hasher = Sha256::new();
            for plane in planes {
                hasher.update(&plane.bytes);
            }
            let actual: String = hasher
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            assert_eq!(&actual, expected, "{file} frame {index}");
        }
    }
}

#[test]
fn a_frame_rate_change_alone_is_not_a_change() {
    let source = VideoSource::new();
    let base = source.load_video(&fixture("base.ivf")).expect("base");
    let double = source.load_video(&fixture("double.ivf")).expect("double");
    assert_eq!(base.frames.len(), 8);
    assert_eq!(double.frames.len(), 16);
    let paired = pair_by_time(&base, &double);
    assert_eq!(paired.base.len(), 16);
    for (index, (b, c)) in paired.base.iter().zip(&paired.candidate).enumerate() {
        assert_eq!(b.pixels, c.pixels, "pair {index} ({})", paired.names[index]);
    }
}

#[test]
fn the_candidate_differs_from_frame_four() {
    let source = VideoSource::new();
    let base = source.load_video(&fixture("base.ivf")).expect("base");
    let candidate = source
        .load_video(&fixture("candidate.ivf"))
        .expect("candidate");
    let paired = pair_by_time(&base, &candidate);
    let changed: Vec<usize> = paired
        .base
        .iter()
        .zip(&paired.candidate)
        .enumerate()
        .filter(|(_, (b, c))| b.pixels != c.pixels)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(changed, vec![4, 5, 6, 7]);
}

#[test]
fn a_frame_is_opaque_rgba_at_the_stream_size() {
    let frames = VideoSource::new()
        .load(&fixture("full601.ivf"))
        .expect("load");
    assert_eq!(frames.len(), 2);
    assert_eq!((frames[0].width, frames[0].height), (64, 48));
    assert!(frames[0].pixels.chunks_exact(4).all(|p| p[3] == 255));
}

#[test]
fn a_stream_without_a_colour_matrix_is_refused() {
    let error = VideoSource::new()
        .load(&fixture("untagged.ivf"))
        .expect_err("refused");
    assert!(
        matches!(&error, VideoError::Unsupported { message, .. } if message.contains("colour matrix")),
        "{error}"
    );
}

#[test]
fn a_file_that_is_not_ivf_is_refused() {
    let path = fixture("REFERENCE.sha256");
    let error = VideoSource::new().load(&path).expect_err("refused");
    assert!(matches!(error, VideoError::Container { .. }), "{error}");
}
