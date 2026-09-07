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

use image::{AnimationDecoder as _, Frame, Rgba, RgbaImage};

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
    write_apng_pair(&base_frames, &candidate_frames);
    write_webp_anim_pair(&base_frames, &candidate_frames);
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

/// Write the APNG pair with the `png` crate directly, through
/// `Encoder::set_animated` and `Writer::set_frame_delay`. `image`'s own
/// high-level save path has no animated-PNG encoder, so this pair is
/// written with `png` directly, not through `image::DynamicImage::save`.
fn write_apng_pair(base_frames: &[RgbaImage], candidate_frames: &[RgbaImage]) {
    let out_dir = golden_root().join("formats").join("apng");
    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("create {}: {e}", out_dir.display()));

    write_apng(&out_dir.join("base.png"), base_frames);
    write_apng(&out_dir.join("candidate.png"), candidate_frames);
}

fn write_apng(path: &std::path::Path, frames: &[RgbaImage]) {
    let file =
        std::fs::File::create(path).unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
    let mut encoder = png::Encoder::new(file, WIDTH, HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .set_animated(frames.len() as u32, 0)
        .unwrap_or_else(|e| panic!("set_animated for {}: {e}", path.display()));

    let mut writer = encoder
        .write_header()
        .unwrap_or_else(|e| panic!("write_header for {}: {e}", path.display()));

    for frame in frames {
        writer
            .set_frame_delay(1, 10)
            .unwrap_or_else(|e| panic!("set_frame_delay for {}: {e}", path.display()));
        writer
            .write_image_data(frame.as_raw())
            .unwrap_or_else(|e| panic!("write_image_data for {}: {e}", path.display()));
    }
    writer
        .finish()
        .unwrap_or_else(|e| panic!("finish for {}: {e}", path.display()));

    println!("wrote {}", path.display());
}

/// Write the animated WebP pair. No crate in this workspace can encode an
/// animated WebP, so this container is assembled by hand around frames
/// the existing lossless single-image encoder already produces. Every
/// frame is lossless (VP8L), on purpose: phase 1 proved a lossless WebP
/// decode path bit-exact on six runners, and never exercised a lossy
/// (VP8) animated frame at all. If a lossy animated-WebP path is ever
/// needed, that is new, unproven determinism surface requiring its own
/// golden-hash fixture, not an extension of this one.
///
/// This is new, unreviewed container-assembly code, unlike every other
/// fixture this generator writes, which only calls an already-tested
/// encoder. It is gated on its own round-trip decode, run in memory
/// before a single byte reaches disk: `round_trip_check` decodes the
/// assembled bytes back through `WebPDecoder` and aborts, naming the
/// frame index, the moment a frame disagrees with what was fed in. No
/// other test may depend on this file before that check passes here.
const FRAME_DURATION_MS: u32 = 100;

fn write_webp_anim_pair(base_frames: &[RgbaImage], candidate_frames: &[RgbaImage]) {
    let out_dir = golden_root().join("formats").join("webp-anim");
    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("create {}: {e}", out_dir.display()));

    write_webp_anim(&out_dir.join("base.webp"), base_frames);
    write_webp_anim(&out_dir.join("candidate.webp"), candidate_frames);
}

fn write_webp_anim(path: &std::path::Path, frames: &[RgbaImage]) {
    let vp8l_payloads: Vec<Vec<u8>> = frames
        .iter()
        .map(|frame| extract_vp8l_payload(&encode_lossless_webp(frame)))
        .collect();

    let bytes = assemble_animated_webp(WIDTH, HEIGHT, &vp8l_payloads);
    round_trip_check(path, &bytes, frames);

    std::fs::write(path, &bytes).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    println!("wrote {}", path.display());
}

/// Encode one frame as a standalone lossless WebP file, through the one
/// encoder this workspace has for WebP: `image`'s own lossless-only
/// `WebPEncoder`.
fn encode_lossless_webp(image: &RgbaImage) -> Vec<u8> {
    let mut buf = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut buf)
        .encode(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .unwrap_or_else(|e| panic!("encode a lossless webp frame: {e}"));
    buf
}

/// Pull the `VP8L` chunk payload out of a standalone single-image WebP
/// file: `image`'s encoder writes the simple container form
/// `RIFF <size> WEBP VP8L <size> <payload>` whenever no ICC, Exif or XMP
/// metadata is set, which this generator never sets.
fn extract_vp8l_payload(single_image_webp: &[u8]) -> Vec<u8> {
    assert_eq!(&single_image_webp[0..4], b"RIFF");
    assert_eq!(&single_image_webp[8..12], b"WEBP");
    assert_eq!(&single_image_webp[12..16], b"VP8L");
    let chunk_size = u32::from_le_bytes(single_image_webp[16..20].try_into().unwrap()) as usize;
    single_image_webp[20..20 + chunk_size].to_vec()
}

/// Write one RIFF chunk: a 4-byte FourCC, a 4-byte little-endian payload
/// length, the payload, and one zero pad byte when the payload length is
/// odd, keeping every chunk boundary two-byte aligned as the RIFF format
/// requires. Every chunk this function returns therefore has an even
/// length, so nesting one inside another needs no separate bookkeeping:
/// the outer chunk's own payload length already counts the inner chunk's
/// padding as part of its content.
fn riff_chunk(fourcc: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + payload.len() + (payload.len() % 2));
    bytes.extend_from_slice(fourcc);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}

/// Build the 10-byte `VP8X` payload: a flags byte with the Animation and
/// Alpha bits set (every frame in this fixture carries a genuine alpha
/// channel), 3 reserved bytes, and the canvas width and height, each
/// stored minus one, per Google's WebP container specification.
fn vp8x_payload(width: u32, height: u32) -> [u8; 10] {
    const ANIMATION_BIT: u8 = 0b0000_0010;
    const ALPHA_BIT: u8 = 0b0001_0000;

    let mut payload = [0u8; 10];
    payload[0] = ANIMATION_BIT | ALPHA_BIT;

    let width_minus_one = (width - 1).to_le_bytes();
    let height_minus_one = (height - 1).to_le_bytes();
    payload[4..7].copy_from_slice(&width_minus_one[..3]);
    payload[7..10].copy_from_slice(&height_minus_one[..3]);
    payload
}

/// Build the 6-byte `ANIM` payload: a background colour (irrelevant here,
/// since every frame in this fixture covers the full canvas) and a loop
/// count of 0, meaning infinite.
fn anim_payload() -> [u8; 6] {
    [0xFF, 0xFF, 0xFF, 0xFF, 0, 0]
}

/// Build one `ANMF` chunk's payload: the 16-byte frame descriptor (frame
/// X and Y offsets stored as the offset divided by two, frame width and
/// height stored minus one, a duration in milliseconds, and a flags byte)
/// followed by the frame's own `VP8L` sub-chunk.
///
/// The flags byte sets the "do not blend" bit. A decoder's alpha-blend
/// path composites a frame onto the previous canvas with an approximate
/// fixed-point formula that is not exact even at full opacity (confirmed
/// by this generator's own round-trip check, which failed on frame 0
/// until this bit was set); "do not blend" instead takes the exact
/// full-canvas overwrite path, since every frame here already covers the
/// whole canvas and needs no compositing against what came before. The
/// dispose bit stays unset for the same reason: nothing here is ever
/// partially transparent over a disposed region.
fn anmf_payload(width: u32, height: u32, duration_ms: u32, vp8l_payload: &[u8]) -> Vec<u8> {
    const DO_NOT_BLEND_BIT: u8 = 0b0000_0010;

    let mut payload = Vec::new();

    let x_offset_div2 = 0u32.to_le_bytes();
    let y_offset_div2 = 0u32.to_le_bytes();
    let width_minus_one = (width - 1).to_le_bytes();
    let height_minus_one = (height - 1).to_le_bytes();
    let duration = duration_ms.to_le_bytes();

    payload.extend_from_slice(&x_offset_div2[..3]);
    payload.extend_from_slice(&y_offset_div2[..3]);
    payload.extend_from_slice(&width_minus_one[..3]);
    payload.extend_from_slice(&height_minus_one[..3]);
    payload.extend_from_slice(&duration[..3]);
    payload.push(DO_NOT_BLEND_BIT);

    payload.extend_from_slice(&riff_chunk(b"VP8L", vp8l_payload));
    payload
}

/// Assemble the full `RIFF`/`WEBP` container: a `VP8X` header with the
/// animation bit set, one `ANIM` chunk, and one `ANMF` chunk per frame.
fn assemble_animated_webp(width: u32, height: u32, vp8l_payloads: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&riff_chunk(b"VP8X", &vp8x_payload(width, height)));
    payload.extend_from_slice(&riff_chunk(b"ANIM", &anim_payload()));
    for vp8l_payload in vp8l_payloads {
        let frame = anmf_payload(width, height, FRAME_DURATION_MS, vp8l_payload);
        payload.extend_from_slice(&riff_chunk(b"ANMF", &frame));
    }

    let mut file = Vec::with_capacity(12 + payload.len());
    file.extend_from_slice(b"RIFF");
    file.extend_from_slice(&((payload.len() + 4) as u32).to_le_bytes());
    file.extend_from_slice(b"WEBP");
    file.extend_from_slice(&payload);
    file
}

/// Decode `bytes` back through `WebPDecoder` and assert the frame count
/// and every frame's pixel buffer equal `expected_frames`, the frames the
/// container was built from. Aborts, naming the frame index, on the
/// first disagreement, so a wrong container field never reaches
/// `tests/golden/`.
fn round_trip_check(path: &std::path::Path, bytes: &[u8], expected_frames: &[RgbaImage]) {
    let decoder = image::codecs::webp::WebPDecoder::new(std::io::Cursor::new(bytes))
        .unwrap_or_else(|e| panic!("round-trip decode {}: {e}", path.display()));

    if !decoder.has_animation() {
        panic!(
            "round-trip decode {}: the assembled container does not report an animation",
            path.display()
        );
    }

    let decoded_frames = decoder
        .into_frames()
        .collect_frames()
        .unwrap_or_else(|e| panic!("round-trip decode {}: {e}", path.display()));

    if decoded_frames.len() != expected_frames.len() {
        panic!(
            "round-trip decode {}: expected {} frames, decoded {}",
            path.display(),
            expected_frames.len(),
            decoded_frames.len()
        );
    }

    for (index, (decoded, expected)) in decoded_frames
        .iter()
        .zip(expected_frames.iter())
        .enumerate()
    {
        let decoded_buffer = decoded.buffer();
        if decoded_buffer.dimensions() != expected.dimensions()
            || decoded_buffer.as_raw() != expected.as_raw()
        {
            panic!(
                "round-trip decode {}: frame {index} does not match the frame it was built from",
                path.display()
            );
        }
    }
}

pub(crate) fn golden_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("golden")
}
