//! One decode test per raster format SRC-01 names, plus a test proving the
//! format is guessed from content, not from a file's extension.
//!
//! Every fixture here is committed under `tests/golden/formats/`, built by
//! `examples/make-fixtures.rs` from the same two in-memory pixel buffers
//! this file rebuilds in `expected_rgba`, so a lossless format's decode
//! can be checked against a buffer this test builds itself, per AGENTS.md,
//! rather than against a file whose bytes could silently drift.

use std::path::{Path, PathBuf};

use chrys_source::{Frame, Source};
use chrys_source_raster::RasterSource;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
const RECT_X: u32 = 64;
const RECT_Y: u32 = 64;
const RECT_WIDTH: u32 = 96;
const RECT_HEIGHT: u32 = 64;
const BACKGROUND: [u8; 4] = [240, 240, 240, 255];
const BASE_RECT_COLOUR: [u8; 4] = [40, 90, 200, 255];
const CANDIDATE_RECT_COLOUR: [u8; 4] = [200, 90, 40, 255];

/// Rebuild, byte for byte, the RGBA8 buffer
/// `crates/chrys-source-raster/examples/make-fixtures.rs` encoded into
/// each lossless format's fixture. A lossless decode must return exactly
/// this buffer.
fn expected_rgba(rect_colour: [u8; 4]) -> Vec<u8> {
    let mut pixels = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let idx = ((y * WIDTH + x) * 4) as usize;
            let colour = if (RECT_X..RECT_X + RECT_WIDTH).contains(&x)
                && (RECT_Y..RECT_Y + RECT_HEIGHT).contains(&y)
            {
                rect_colour
            } else {
                BACKGROUND
            };
            pixels[idx..idx + 4].copy_from_slice(&colour);
        }
    }
    pixels
}

fn fixture_dir(format: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("golden")
        .join("formats")
        .join(format)
}

fn load_one_frame(path: &Path) -> Frame {
    let source = RasterSource::new();
    let mut frames = source
        .load(path)
        .unwrap_or_else(|e| panic!("decode {}: {e}", path.display()));
    assert_eq!(
        frames.len(),
        1,
        "{} decoded to {} frames, expected 1",
        path.display(),
        frames.len()
    );
    frames.remove(0)
}

fn assert_repeat_decode_is_identical(path: &Path) {
    let first = load_one_frame(path);
    let second = load_one_frame(path);
    assert_eq!(
        first.pixels,
        second.pixels,
        "{} did not decode identically twice",
        path.display()
    );
}

#[test]
fn png() {
    let dir = fixture_dir("png");

    let base = load_one_frame(&dir.join("base.png"));
    assert_eq!((base.width, base.height), (WIDTH, HEIGHT));
    assert_eq!(base.pixels, expected_rgba(BASE_RECT_COLOUR));

    let candidate = load_one_frame(&dir.join("candidate.png"));
    assert_eq!(candidate.pixels, expected_rgba(CANDIDATE_RECT_COLOUR));

    assert_repeat_decode_is_identical(&dir.join("base.png"));
}

#[test]
fn webp() {
    let dir = fixture_dir("webp");

    let base = load_one_frame(&dir.join("base.webp"));
    assert_eq!((base.width, base.height), (WIDTH, HEIGHT));
    assert_eq!(base.pixels, expected_rgba(BASE_RECT_COLOUR));

    let candidate = load_one_frame(&dir.join("candidate.webp"));
    assert_eq!(candidate.pixels, expected_rgba(CANDIDATE_RECT_COLOUR));

    assert_repeat_decode_is_identical(&dir.join("base.webp"));
}

#[test]
fn tiff() {
    let dir = fixture_dir("tiff");

    let base = load_one_frame(&dir.join("base.tif"));
    assert_eq!((base.width, base.height), (WIDTH, HEIGHT));
    assert_eq!(base.pixels, expected_rgba(BASE_RECT_COLOUR));

    let candidate = load_one_frame(&dir.join("candidate.tif"));
    assert_eq!(candidate.pixels, expected_rgba(CANDIDATE_RECT_COLOUR));

    assert_repeat_decode_is_identical(&dir.join("base.tif"));
}

#[test]
fn jpeg() {
    let dir = fixture_dir("jpeg");

    // JPEG is lossy: the decoded buffer is checked for shape and
    // repeat-decode stability, never for byte equality against the
    // buffer the generator encoded.
    let base = load_one_frame(&dir.join("base.jpg"));
    assert_eq!((base.width, base.height), (WIDTH, HEIGHT));

    let candidate = load_one_frame(&dir.join("candidate.jpg"));
    assert_eq!((candidate.width, candidate.height), (WIDTH, HEIGHT));

    assert_repeat_decode_is_identical(&dir.join("base.jpg"));
}

#[test]
fn a_file_decodes_by_content_even_with_a_mismatched_extension() {
    let source_path = fixture_dir("png").join("base.png");
    let renamed = std::env::temp_dir().join(format!(
        "chrys-formats-test-{}-mismatched-extension.jpg",
        std::process::id()
    ));
    std::fs::copy(&source_path, &renamed).expect("copy the PNG fixture to a .jpg path");

    let result = RasterSource::new().load(&renamed);
    std::fs::remove_file(&renamed).ok();

    let frames =
        result.expect("a PNG named .jpg still decodes, because the format comes from its content");
    assert_eq!(frames.len(), 1);
    assert_eq!((frames[0].width, frames[0].height), (WIDTH, HEIGHT));
}
