//! Build the committed golden animation fixtures under
//! `tests/golden/formats/`.
//!
//! This generator is committed so a reader can see how the bytes were
//! made, in the shape `chrys-source-raster`'s own generator established:
//! integer formulas only, no external asset, no network call. Every format
//! this generator writes wraps the same five frames, so a cross-format
//! agreement test can assert two formats built from the same content
//! decode to the same pixels.
//!
//! Run with: `cargo run -p chrys-source-animation --example make-fixtures`

use image::{Frame, Rgba, RgbaImage};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
const FRAME_COUNT: i64 = 5;
const CHANGED_FRAME_INDEX: i64 = 3;

/// Three rectangles, each at a fixed offset from a chosen origin, in a
/// fixed colour. The same structured-rectangle idea
/// `chrys-source-raster/examples/make-fixtures.rs` uses: content with
/// enough edges to read as motion when the origin shifts a few pixels
/// between frames.
const RECTS: [(i64, i64, u32, u32, Rgba<u8>); 3] = [
    (40, 40, 50, 40, Rgba([200, 40, 40, 255])),
    (140, 90, 60, 30, Rgba([40, 160, 40, 255])),
    (60, 160, 40, 50, Rgba([40, 40, 200, 255])),
];

fn paint_rect(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, colour: Rgba<u8>) {
    for yy in y..(y + height).min(image.height()) {
        for xx in x..(x + width).min(image.width()) {
            image.put_pixel(xx, yy, colour);
        }
    }
}

/// Build one canvas at frame `index`, each rectangle shifted `index`
/// pixels right and down from its listed position, so the animation moves
/// a little every frame.
fn build_canvas(index: i64) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(WIDTH, HEIGHT, Rgba([230, 230, 230, 255]));
    for &(rect_x, rect_y, width, height, colour) in &RECTS {
        let x = (rect_x + index).clamp(0, WIDTH as i64 - 1) as u32;
        let y = (rect_y + index).clamp(0, HEIGHT as i64 - 1) as u32;
        paint_rect(&mut image, x, y, width, height, colour);
    }
    image
}

/// Recolour the first rectangle at frame `index`'s own shifted position, so
/// the candidate side carries exactly one changed index.
fn recolour_first_rect(image: &mut RgbaImage, index: i64, colour: Rgba<u8>) {
    let (rect_x, rect_y, width, height, _) = RECTS[0];
    let x = (rect_x + index).clamp(0, WIDTH as i64 - 1) as u32;
    let y = (rect_y + index).clamp(0, HEIGHT as i64 - 1) as u32;
    paint_rect(image, x, y, width, height, colour);
}

/// Build the base and candidate canvas for one frame index. Every index
/// registers as a near-identical pair except `CHANGED_FRAME_INDEX`, where
/// the candidate side recolours one rectangle.
fn build_frame_pair(index: i64) -> (RgbaImage, RgbaImage) {
    let base = build_canvas(index);
    let mut candidate = build_canvas(index);
    if index == CHANGED_FRAME_INDEX {
        recolour_first_rect(&mut candidate, index, Rgba([250, 200, 40, 255]));
    }
    (base, candidate)
}

/// Build the five base frames and the five candidate frames every format
/// in this generator wraps.
pub(crate) fn build_frames() -> (Vec<RgbaImage>, Vec<RgbaImage>) {
    let mut base_frames = Vec::with_capacity(FRAME_COUNT as usize);
    let mut candidate_frames = Vec::with_capacity(FRAME_COUNT as usize);
    for index in 0..FRAME_COUNT {
        let (base, candidate) = build_frame_pair(index);
        base_frames.push(base);
        candidate_frames.push(candidate);
    }
    (base_frames, candidate_frames)
}

fn main() {
    let (base_frames, candidate_frames) = build_frames();
    write_gif_pair(&base_frames, &candidate_frames);
}

/// Write the GIF pair with `image::codecs::gif::GifEncoder::encode_frames`,
/// the encoder this crate's own `AnimationSource` decodes back through
/// `GifDecoder::into_frames()`.
fn write_gif_pair(base_frames: &[RgbaImage], candidate_frames: &[RgbaImage]) {
    let out_dir = golden_root().join("formats").join("gif");
    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("create {}: {e}", out_dir.display()));

    write_gif(&out_dir.join("base.gif"), base_frames);
    write_gif(&out_dir.join("candidate.gif"), candidate_frames);
}

fn write_gif(path: &std::path::Path, frames: &[RgbaImage]) {
    let file =
        std::fs::File::create(path).unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    let animation_frames: Vec<Frame> = frames.iter().cloned().map(Frame::new).collect();
    encoder
        .encode_frames(animation_frames)
        .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));

    println!("wrote {}", path.display());
}

pub(crate) fn golden_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("golden")
}
