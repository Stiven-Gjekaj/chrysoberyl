//! Build the committed golden pairs under `tests/golden/`.
//!
//! This generator is committed so a reader can see how the bytes were
//! made. The image files it writes are committed too, because every later
//! determinism check must read a real file from disk, not a buffer built
//! in memory.
//!
//! Run with: `cargo run -p chrys-source-raster --example make-fixtures`

use image::{Rgba, RgbaImage};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
const BACKGROUND: Rgba<u8> = Rgba([240, 240, 240, 255]);
const RECT_X: u32 = 64;
const RECT_Y: u32 = 64;
const RECT_WIDTH: u32 = 96;
const RECT_HEIGHT: u32 = 64;
const BASE_RECT_COLOUR: Rgba<u8> = Rgba([40, 90, 200, 255]);
const CANDIDATE_RECT_COLOUR: Rgba<u8> = Rgba([200, 90, 40, 255]);

/// One raster format's committed pair, named by its directory under
/// `tests/golden/formats/` and its two file names.
struct FormatFixture {
    directory: &'static str,
    base_name: &'static str,
    candidate_name: &'static str,
}

const FORMAT_FIXTURES: [FormatFixture; 4] = [
    FormatFixture {
        directory: "png",
        base_name: "base.png",
        candidate_name: "candidate.png",
    },
    FormatFixture {
        directory: "jpeg",
        base_name: "base.jpg",
        candidate_name: "candidate.jpg",
    },
    FormatFixture {
        directory: "webp",
        base_name: "base.webp",
        candidate_name: "candidate.webp",
    },
    FormatFixture {
        directory: "tiff",
        base_name: "base.tif",
        candidate_name: "candidate.tif",
    },
];

fn build_image(rect_colour: Rgba<u8>) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(WIDTH, HEIGHT, BACKGROUND);
    for y in RECT_Y..(RECT_Y + RECT_HEIGHT) {
        for x in RECT_X..(RECT_X + RECT_WIDTH) {
            image.put_pixel(x, y, rect_colour);
        }
    }
    image
}

fn main() {
    let base = build_image(BASE_RECT_COLOUR);
    let candidate = build_image(CANDIDATE_RECT_COLOUR);

    write_pair_01(&base, &candidate);
    write_format_fixtures(&base, &candidate);
}

/// Write the first committed pair, read by plan 01-03. Its bytes must not
/// move, so this function's output path and encoding stay exactly as they
/// were before this plan.
fn write_pair_01(base: &RgbaImage, candidate: &RgbaImage) {
    let out_dir = golden_root().join("pair-01");
    std::fs::create_dir_all(&out_dir).expect("create tests/golden/pair-01");

    base.save(out_dir.join("base.png")).expect("write base.png");
    candidate
        .save(out_dir.join("candidate.png"))
        .expect("write candidate.png");

    println!("wrote {}", out_dir.join("base.png").display());
    println!("wrote {}", out_dir.join("candidate.png").display());
}

/// Write one pair per raster format SRC-01 names, from the same two
/// in-memory buffers `write_pair_01` already wrote as PNG. Each format
/// re-encodes the identical pixel content, so the PNG, WebP and TIFF
/// pairs (all lossless here) decode back to the exact same bytes, and the
/// JPEG pair proves shape and repeat-decode stability instead, since JPEG
/// is lossy.
///
/// Each buffer is wrapped in a `DynamicImage` before it is saved, because
/// only `DynamicImage::save` converts to a colour type the target encoder
/// supports; a JPEG encoder does not accept straight RGBA8 input, since
/// JPEG has no alpha channel.
fn write_format_fixtures(base: &RgbaImage, candidate: &RgbaImage) {
    let base = image::DynamicImage::ImageRgba8(base.clone());
    let candidate = image::DynamicImage::ImageRgba8(candidate.clone());

    for fixture in FORMAT_FIXTURES {
        let out_dir = golden_root().join("formats").join(fixture.directory);
        std::fs::create_dir_all(&out_dir)
            .unwrap_or_else(|_| panic!("create {}", out_dir.display()));

        let base_path = out_dir.join(fixture.base_name);
        let candidate_path = out_dir.join(fixture.candidate_name);

        base.save(&base_path)
            .unwrap_or_else(|e| panic!("write {}: {e}", base_path.display()));
        candidate
            .save(&candidate_path)
            .unwrap_or_else(|e| panic!("write {}: {e}", candidate_path.display()));

        println!("wrote {}", base_path.display());
        println!("wrote {}", candidate_path.display());
    }
}

fn golden_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("golden")
}
