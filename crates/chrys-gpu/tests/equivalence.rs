//! The GPU equivalence tests. Each test needs a GPU adapter.
//!
//! The tests are ignored by default, because a CI runner has no GPU.
//! Run them on a machine with a GPU:
//!
//!     cargo test -p chrys-gpu -- --ignored
//!
//! A pass on one vendor proves that vendor only. GPU-03 asks for three.

use std::path::PathBuf;

use chrys_gpu::{Agreement, GpuMirror, agree};
use chrys_source::{Frame, Source};
use chrys_source_raster::RasterSource;

fn load(relative: &str) -> Frame {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    RasterSource::new().load(&path).expect("decode").remove(0)
}

/// A frame of `width` by `height` whose bytes follow a fixed pattern.
fn patterned(width: u32, height: u32, seed: u32) -> Frame {
    let pixels = (0..width * height * 4)
        .map(|i| (i.wrapping_mul(2_654_435_761).wrapping_add(seed) >> 13) as u8)
        .collect();
    Frame {
        pixels,
        width,
        height,
        index: 0,
        hints: Vec::new(),
    }
}

#[test]
#[ignore = "needs a GPU adapter"]
fn the_gpu_mask_is_the_cpu_mask_byte_for_byte() {
    let mirror = GpuMirror::new().expect("a GPU adapter");
    println!("adapter: {:?}", mirror.report());
    // 37 by 23 is not a multiple of the workgroup size, so the edge test
    // in the shader runs.
    let base = patterned(37, 23, 1);
    let candidate = patterned(37, 23, 99);
    let mask = mirror.difference(&base, &candidate).expect("difference");
    let expected: Vec<u8> = base
        .pixels
        .chunks_exact(4)
        .zip(candidate.pixels.chunks_exact(4))
        .map(|(a, b)| (0..4).map(|c| a[c].abs_diff(b[c])).max().unwrap_or(0))
        .collect();
    assert_eq!(mask.values, expected);
}

#[test]
#[ignore = "needs a GPU adapter"]
fn the_gpu_agrees_with_the_cpu_on_a_recoloured_pair() {
    let mirror = GpuMirror::new().expect("a GPU adapter");
    let base = load("tests/golden/rule-01/base.png");
    let candidate = load("tests/golden/rule-01/candidate.png");
    let verdict = chrys_core::compare(&base, &candidate).expect("compare");
    assert!(verdict.is_change(), "the fixture must hold a change");
    let boxes = mirror
        .difference(&base, &candidate)
        .expect("difference")
        .changed_boxes();
    assert_eq!(
        agree(&verdict, &boxes, 2),
        Agreement::Agree,
        "{verdict:?} {boxes:?}"
    );
}

#[test]
#[ignore = "needs a GPU adapter"]
fn the_gpu_agrees_with_the_cpu_on_an_identical_pair() {
    let mirror = GpuMirror::new().expect("a GPU adapter");
    let base = load("tests/golden/rule-01/base.png");
    let verdict = chrys_core::compare(&base, &base).expect("compare");
    let boxes = mirror
        .difference(&base, &base)
        .expect("difference")
        .changed_boxes();
    assert_eq!(agree(&verdict, &boxes, 0), Agreement::Agree);
}
