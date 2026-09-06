//! Build the committed golden pair for `tests/golden/pair-01`.
//!
//! This generator is committed so a reader can see how the bytes were
//! made. The PNG files it writes are committed too, because every later
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

    let out_dir = golden_dir();
    std::fs::create_dir_all(&out_dir).expect("create tests/golden/pair-01");

    base.save(out_dir.join("base.png")).expect("write base.png");
    candidate
        .save(out_dir.join("candidate.png"))
        .expect("write candidate.png");

    println!("wrote {}", out_dir.join("base.png").display());
    println!("wrote {}", out_dir.join("candidate.png").display());
}

fn golden_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("golden")
        .join("pair-01")
}
