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
    write_refusal_corpus();
    write_sequence_01();
    write_hint_01();
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

/// One pair in the CORE-06/CORE-07 calibration corpus: a directory name
/// under `should-register/` or `should-refuse/`, and the two images it
/// holds.
struct RefusalPair {
    directory: &'static str,
    base: RgbaImage,
    candidate: RgbaImage,
}

/// Three rectangles, each at a fixed offset from a chosen origin, in a
/// fixed colour. This is the structured content every `should-register`
/// canvas is built from: content with enough edges for phase correlation
/// to lock onto, translated as one block when the origin moves.
const STRUCTURED_RECTS: [(i64, i64, u32, u32, Rgba<u8>); 3] = [
    (40, 40, 50, 40, Rgba([200, 40, 40, 255])),
    (140, 90, 60, 30, Rgba([40, 160, 40, 255])),
    (60, 160, 40, 50, Rgba([40, 40, 200, 255])),
];

/// Four rectangles at a disjoint set of positions, sizes and colours, over
/// a dark background instead of `BACKGROUND`. This is a shape set a person
/// looking at both canvases would call a different picture, not the same
/// picture moved or touched up.
const ALTERNATE_RECTS: [(i64, i64, u32, u32, Rgba<u8>); 4] = [
    (10, 200, 30, 30, Rgba([250, 250, 40, 255])),
    (180, 20, 70, 20, Rgba([40, 250, 250, 255])),
    (90, 90, 20, 90, Rgba([250, 40, 250, 255])),
    (200, 150, 40, 40, Rgba([250, 250, 250, 255])),
];

fn paint_rect(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, colour: Rgba<u8>) {
    for yy in y..(y + height).min(image.height()) {
        for xx in x..(x + width).min(image.width()) {
            image.put_pixel(xx, yy, colour);
        }
    }
}

/// Build a canvas from `STRUCTURED_RECTS`, each rectangle moved by
/// `(dx, dy)` from its listed position and clamped to stay inside the
/// canvas. Moving `dx` and `dy` together shifts the whole picture by one
/// fixed translation, exactly the near-identical case a pair "shifted by a
/// few pixels" models.
fn build_structured_canvas(dx: i64, dy: i64) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(WIDTH, HEIGHT, Rgba([230, 230, 230, 255]));
    for &(rect_x, rect_y, width, height, colour) in &STRUCTURED_RECTS {
        let x = (rect_x + dx).clamp(0, WIDTH as i64 - 1) as u32;
        let y = (rect_y + dy).clamp(0, HEIGHT as i64 - 1) as u32;
        paint_rect(&mut image, x, y, width, height, colour);
    }
    image
}

/// Build a canvas from `ALTERNATE_RECTS`, shifted horizontally by `dx`.
/// `dx` only ever varies this shape set's own two committed instances from
/// each other; it never makes this canvas resemble `build_structured_canvas`.
fn build_alternate_canvas(dx: i64) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(WIDTH, HEIGHT, Rgba([15, 15, 15, 255]));
    for &(rect_x, rect_y, width, height, colour) in &ALTERNATE_RECTS {
        let x = (rect_x + dx).clamp(0, WIDTH as i64 - 1) as u32;
        let y = rect_y.clamp(0, HEIGHT as i64 - 1) as u32;
        paint_rect(&mut image, x, y, width, height, colour);
    }
    image
}

/// Advance a Numerical-Recipes linear congruential generator one step and
/// return its new state. Deterministic in `seed` alone: the same seed
/// reproduces the same sequence of states on every machine, because every
/// step is one `u32` multiply and one `u32` add, both wrapping.
fn next_lcg_state(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed
}

/// Build a canvas of pseudorandom per-pixel colour, from the linear
/// congruential generator seeded at `seed`. Two different seeds give two
/// canvases with no shared structure for a correlation to lock onto.
fn build_noise_canvas(seed: u32) -> RgbaImage {
    let mut state = seed;
    let mut image = RgbaImage::new(WIDTH, HEIGHT);
    for pixel in image.pixels_mut() {
        let red = (next_lcg_state(&mut state) >> 24) as u8;
        let green = (next_lcg_state(&mut state) >> 24) as u8;
        let blue = (next_lcg_state(&mut state) >> 24) as u8;
        *pixel = Rgba([red, green, blue, 255]);
    }
    image
}

/// Build a canvas filled with one flat colour, with no edge anywhere for a
/// correlation to lock onto.
fn build_solid_canvas(colour: Rgba<u8>) -> RgbaImage {
    RgbaImage::from_pixel(WIDTH, HEIGHT, colour)
}

/// Recolour the first `STRUCTURED_RECTS` rectangle in `image`, in place, to
/// `colour`. Used to build a `should-register` pair whose only difference
/// from its base is one rectangle's colour.
fn recolour_first_rect(image: &mut RgbaImage, colour: Rgba<u8>) {
    let (rect_x, rect_y, width, height, _) = STRUCTURED_RECTS[0];
    paint_rect(image, rect_x as u32, rect_y as u32, width, height, colour);
}

/// The eight `should-register` pairs: content a person looking at both
/// images would call the same picture, unchanged or barely touched up.
fn should_register_pairs() -> Vec<RefusalPair> {
    let base = build_structured_canvas(0, 0);

    let mut recoloured_small = base.clone();
    recolour_first_rect(&mut recoloured_small, Rgba([180, 60, 40, 255]));

    let mut recoloured_large = base.clone();
    recolour_first_rect(&mut recoloured_large, Rgba([40, 200, 200, 255]));

    let mut added_element = base.clone();
    paint_rect(
        &mut added_element,
        210,
        210,
        20,
        20,
        Rgba([250, 200, 40, 255]),
    );

    let mut combined = build_structured_canvas(2, -1);
    recolour_first_rect(&mut combined, Rgba([180, 60, 40, 255]));

    vec![
        RefusalPair {
            directory: "pair-01",
            base: base.clone(),
            candidate: base.clone(),
        },
        RefusalPair {
            directory: "pair-02",
            base: base.clone(),
            candidate: build_structured_canvas(3, 2),
        },
        RefusalPair {
            directory: "pair-03",
            base: base.clone(),
            candidate: build_structured_canvas(-4, 6),
        },
        RefusalPair {
            directory: "pair-04",
            base: base.clone(),
            candidate: build_structured_canvas(1, 1),
        },
        RefusalPair {
            directory: "pair-05",
            base: base.clone(),
            candidate: recoloured_small,
        },
        RefusalPair {
            directory: "pair-06",
            base: base.clone(),
            candidate: recoloured_large,
        },
        RefusalPair {
            directory: "pair-07",
            base: base.clone(),
            candidate: added_element,
        },
        RefusalPair {
            directory: "pair-08",
            base,
            candidate: combined,
        },
    ]
}

/// The eight `should-refuse` pairs: content a person looking at both
/// images would call two unrelated pictures.
fn should_refuse_pairs() -> Vec<RefusalPair> {
    let structured = build_structured_canvas(0, 0);

    vec![
        RefusalPair {
            directory: "pair-01",
            base: structured.clone(),
            candidate: build_alternate_canvas(0),
        },
        RefusalPair {
            directory: "pair-02",
            base: structured.clone(),
            candidate: image::imageops::flip_horizontal(&structured),
        },
        RefusalPair {
            directory: "pair-03",
            base: structured.clone(),
            candidate: build_noise_canvas(20_260_906),
        },
        RefusalPair {
            directory: "pair-04",
            base: structured.clone(),
            candidate: build_solid_canvas(Rgba([128, 128, 128, 255])),
        },
        RefusalPair {
            directory: "pair-05",
            base: structured.clone(),
            candidate: build_alternate_canvas(30),
        },
        RefusalPair {
            directory: "pair-06",
            base: structured.clone(),
            candidate: image::imageops::flip_vertical(&structured),
        },
        RefusalPair {
            directory: "pair-07",
            base: structured.clone(),
            candidate: build_noise_canvas(314_159_265),
        },
        RefusalPair {
            directory: "pair-08",
            base: structured,
            candidate: build_solid_canvas(Rgba([20, 200, 20, 255])),
        },
    ]
}

/// Write the CORE-06/CORE-07 calibration corpus: the `should-register` and
/// `should-refuse` pairs `crates/chrys-core/tests/refusal.rs` measures a
/// threshold from. Every pair is built from the deterministic integer
/// formulas above, so a reader can reproduce every byte this function
/// writes without running this generator.
fn write_refusal_corpus() {
    write_refusal_group("should-register", should_register_pairs());
    write_refusal_group("should-refuse", should_refuse_pairs());
}

fn write_refusal_group(group: &str, pairs: Vec<RefusalPair>) {
    for pair in pairs {
        let out_dir = golden_root()
            .join("refuse-01")
            .join(group)
            .join(pair.directory);
        std::fs::create_dir_all(&out_dir)
            .unwrap_or_else(|_| panic!("create {}", out_dir.display()));

        let base_path = out_dir.join("base.png");
        let candidate_path = out_dir.join("candidate.png");

        pair.base
            .save(&base_path)
            .unwrap_or_else(|e| panic!("write {}: {e}", base_path.display()));
        pair.candidate
            .save(&candidate_path)
            .unwrap_or_else(|e| panic!("write {}: {e}", candidate_path.display()));

        println!("wrote {}", base_path.display());
        println!("wrote {}", candidate_path.display());
    }
}

/// Write the committed sequence fixture, read by `chrys-source-sequence`.
///
/// 11 frames per side, named `frame1.png` through `frame11.png` with no
/// zero padding, on purpose: the natural-sort trap sits in a committed
/// file from the first commit rather than in a later bug report. Frame
/// `i` on both sides is `build_structured_canvas(i as i64, 0)`, so every
/// index is a near-identical pair that registers. On the candidate side
/// only, frame 5 is recoloured, so exactly one index carries a change and
/// the other ten are the same picture.
fn write_sequence_01() {
    let base_dir = golden_root().join("sequence-01").join("base");
    let candidate_dir = golden_root().join("sequence-01").join("candidate");
    std::fs::create_dir_all(&base_dir)
        .unwrap_or_else(|e| panic!("create {}: {e}", base_dir.display()));
    std::fs::create_dir_all(&candidate_dir)
        .unwrap_or_else(|e| panic!("create {}: {e}", candidate_dir.display()));

    for i in 1..=11i64 {
        let base_frame = build_structured_canvas(i, 0);
        let mut candidate_frame = build_structured_canvas(i, 0);
        if i == 5 {
            recolour_first_rect(&mut candidate_frame, Rgba([40, 200, 200, 255]));
        }

        let base_path = base_dir.join(format!("frame{i}.png"));
        let candidate_path = candidate_dir.join(format!("frame{i}.png"));

        base_frame
            .save(&base_path)
            .unwrap_or_else(|e| panic!("write {}: {e}", base_path.display()));
        candidate_frame
            .save(&candidate_path)
            .unwrap_or_else(|e| panic!("write {}: {e}", candidate_path.display()));

        println!("wrote {}", base_path.display());
        println!("wrote {}", candidate_path.display());
    }
}

/// The four rectangles painted inside the named `logo` region, at
/// `(64, 64)` sized 128x128. Byte-identical between `base.png` and
/// `candidate.png`: four different colours and sizes, so the block match
/// over the cropped 128x128 region (exactly sixteen 32x32 blocks) has
/// edges in most of them to lock onto.
const HINT_REGION_RECTS: [(u32, u32, u32, u32, Rgba<u8>); 4] = [
    (80, 80, 24, 20, Rgba([200, 40, 40, 255])),
    (150, 90, 28, 18, Rgba([40, 160, 40, 255])),
    (90, 140, 20, 30, Rgba([40, 40, 200, 255])),
    (150, 150, 22, 26, Rgba([200, 200, 40, 255])),
];

/// Write the committed `hint-01` fixture, read by `chrys-cli/tests/region_hint.rs`.
///
/// Both `base.hints.toml` and `candidate.hints.toml` name one region,
/// `logo`, at `(64, 64)` sized 128x128 (`HINT_REGION_RECTS`, painted
/// identically on both sides). Outside that region, two small rectangles
/// are painted a different flat colour on each side: a colour change, not
/// a shift, so no translation the engine's own registration step can find
/// hides the difference. The whole-frame comparison therefore reports
/// something other than `identical`, while the region-cropped comparison,
/// over bytes that are identical between the two files, reports
/// `identical`.
fn write_hint_01() {
    let mut base = RgbaImage::from_pixel(WIDTH, HEIGHT, Rgba([230, 230, 230, 255]));
    let mut candidate = RgbaImage::from_pixel(WIDTH, HEIGHT, Rgba([230, 230, 230, 255]));

    paint_rect(&mut base, 16, 16, 40, 30, Rgba([40, 90, 200, 255]));
    paint_rect(&mut candidate, 16, 16, 40, 30, Rgba([200, 90, 40, 255]));
    paint_rect(&mut base, 196, 196, 40, 30, Rgba([40, 90, 200, 255]));
    paint_rect(&mut candidate, 196, 196, 40, 30, Rgba([200, 90, 40, 255]));

    for &(x, y, width, height, colour) in &HINT_REGION_RECTS {
        paint_rect(&mut base, x, y, width, height, colour);
        paint_rect(&mut candidate, x, y, width, height, colour);
    }

    let out_dir = golden_root().join("hint-01");
    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| panic!("create {}: {e}", out_dir.display()));

    let base_path = out_dir.join("base.png");
    let candidate_path = out_dir.join("candidate.png");
    base.save(&base_path)
        .unwrap_or_else(|e| panic!("write {}: {e}", base_path.display()));
    candidate
        .save(&candidate_path)
        .unwrap_or_else(|e| panic!("write {}: {e}", candidate_path.display()));

    let sidecar = "[[hint]]\nname = \"logo\"\nx = 64\ny = 64\nwidth = 128\nheight = 128\n";
    let base_sidecar_path = out_dir.join("base.hints.toml");
    let candidate_sidecar_path = out_dir.join("candidate.hints.toml");
    std::fs::write(&base_sidecar_path, sidecar)
        .unwrap_or_else(|e| panic!("write {}: {e}", base_sidecar_path.display()));
    std::fs::write(&candidate_sidecar_path, sidecar)
        .unwrap_or_else(|e| panic!("write {}: {e}", candidate_sidecar_path.display()));

    println!("wrote {}", base_path.display());
    println!("wrote {}", candidate_path.display());
    println!("wrote {}", base_sidecar_path.display());
    println!("wrote {}", candidate_sidecar_path.display());
}

fn golden_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("golden")
}
