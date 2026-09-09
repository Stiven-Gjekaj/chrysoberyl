//! Integration tests for `SvgSource`, run through its public `Source`
//! implementation rather than against any private helper.

use std::path::PathBuf;

use chrys_source::Source;
use chrys_source_svg::{SvgError, SvgLimits, SvgSource};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> PathBuf {
    repo_root().join("tests/golden/formats/svg").join(name)
}

#[test]
fn identical_svgs_produce_identical_frames() {
    let source = SvgSource::new();

    let base_path = fixture("base.svg");
    let first = source
        .load(&base_path)
        .expect("base.svg decodes")
        .into_iter()
        .next()
        .expect("base.svg decodes to one frame");
    let second = source
        .load(&base_path)
        .expect("base.svg decodes a second time")
        .into_iter()
        .next()
        .expect("base.svg decodes to one frame a second time");

    assert_eq!(
        first, second,
        "loading the same SVG twice must be pixel-for-pixel identical"
    );
    assert_eq!(first.width, 256);
    assert_eq!(first.height, 256);
    assert_eq!(first.pixels.len(), 256 * 256 * 4);

    let candidate_path = fixture("candidate.svg");
    let candidate = source
        .load(&candidate_path)
        .expect("candidate.svg decodes")
        .into_iter()
        .next()
        .expect("candidate.svg decodes to one frame");

    assert_ne!(
        first.pixels, candidate.pixels,
        "the recoloured rectangle must change the decoded pixel buffer"
    );
}

/// Premultiplied and straight alpha are byte-identical wherever alpha is
/// 255, which is exactly what makes a skipped conversion silent: it hides
/// completely behind an opaque fixture and only reports a wrong verdict
/// on a real one. This test is built against a pixel whose alpha is not
/// 255, with an assertion a premultiplied buffer cannot satisfy at all.
///
/// The document is built here, as a string, rather than read from a
/// committed fixture: AGENTS.md records that a test should build the
/// state it needs rather than reading it out of a file an author edits.
#[test]
fn alpha_is_straight_not_premultiplied() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20">
  <rect x="0" y="0" width="20" height="20" fill="#ff0000" fill-opacity="0.5"/>
  <rect x="20" y="0" width="20" height="20" fill="#ff0000"/>
</svg>
"##;

    let dir = std::env::temp_dir();
    let path = dir.join("chrys-source-svg-alpha-is-straight-test.svg");
    std::fs::write(&path, svg).expect("write the temporary SVG");

    let source = SvgSource::new();
    let frame = source
        .load(&path)
        .expect("the temporary SVG decodes")
        .into_iter()
        .next()
        .expect("the temporary SVG decodes to one frame");
    std::fs::remove_file(&path).ok();

    let pixel_at = |x: u32, y: u32| -> [u8; 4] {
        let index = ((y * frame.width + x) * 4) as usize;
        [
            frame.pixels[index],
            frame.pixels[index + 1],
            frame.pixels[index + 2],
            frame.pixels[index + 3],
        ]
    };

    // The centre of the left rectangle: pure red at 50 per cent fill
    // opacity, over a transparent canvas.
    let translucent = pixel_at(10, 10);
    // The centre of the right rectangle: pure red at full opacity, a
    // second, disjoint sample proving the buffer is not wrong everywhere.
    let opaque = pixel_at(30, 10);

    // The decisive assertion. Under premultiplied storage the red channel
    // is scaled by the alpha, so red and alpha sit within a few counts of
    // each other and this difference can never reach 100.
    assert!(
        (translucent[0] as i32 - translucent[3] as i32) > 100,
        "translucent pixel is red={}, green={}, blue={}, alpha={}: straight alpha must report \
         a red channel that exceeds alpha by more than 100",
        translucent[0],
        translucent[1],
        translucent[2],
        translucent[3]
    );
    // Loose about the exact rounding of a half-opacity value on purpose:
    // the property under test is which convention the buffer is in, not
    // what a particular version rounds 127.5 to.
    assert!(
        (120..=140).contains(&translucent[3]),
        "translucent pixel is red={}, green={}, blue={}, alpha={}: alpha must lie between 120 \
         and 140",
        translucent[0],
        translucent[1],
        translucent[2],
        translucent[3]
    );
    assert!(
        translucent[1] < 8 && translucent[2] < 8,
        "translucent pixel is red={}, green={}, blue={}, alpha={}: green and blue must be below 8",
        translucent[0],
        translucent[1],
        translucent[2],
        translucent[3]
    );

    // A test that only read the translucent pixel could pass on a buffer
    // that was wrong everywhere.
    assert_eq!(
        opaque[3], 255,
        "opaque pixel is red={}, green={}, blue={}, alpha={}: alpha must be 255",
        opaque[0], opaque[1], opaque[2], opaque[3]
    );
    assert!(
        opaque[0] >= 250,
        "opaque pixel is red={}, green={}, blue={}, alpha={}: red must be at least 250",
        opaque[0],
        opaque[1],
        opaque[2],
        opaque[3]
    );
}

/// The family name this crate falls back to is read out of the pinned
/// database, not typed a second time as a string literal: a literal
/// typed twice is two things that can drift, and the point of this
/// assertion is that they cannot.
#[test]
fn the_pinned_database_holds_one_face_whose_family_the_options_name() {
    let db = chrys_source_svg::fonts::pinned_fontdb();
    assert_eq!(
        db.len(),
        1,
        "the pinned database must hold exactly one face"
    );

    let face = db
        .faces()
        .next()
        .expect("the pinned database holds exactly one face");
    let declared_family = &face
        .families
        .first()
        .expect("a TrueType face names at least one family")
        .0;

    let options_family = chrys_source_svg::fonts::pinned_family_name(&db);
    assert_eq!(
        *declared_family, options_family,
        "the face's own declared family must equal the name the crate sets as the fallback family"
    );
}

/// Renders `svg` (a `{}` placeholder carries the text element's
/// `font-family` attribute, empty when the caller passes no family at
/// all) to a temporary file, loads it through `SvgSource`, and returns
/// the decoded frame.
fn render_text_document(font_family_attr: &str) -> chrys_source::Frame {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="40">
  <text x="5" y="28"{font_family_attr} font-size="28" fill="#000000">Chrys</text>
</svg>
"##
    );

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "chrys-source-svg-fallback-test-{}.svg",
        font_family_attr.len()
    ));
    std::fs::write(&path, svg).expect("write the temporary SVG");

    let source = SvgSource::new();
    let result = source
        .load(&path)
        .expect("the temporary SVG decodes")
        .into_iter()
        .next()
        .expect("the temporary SVG decodes to one frame");
    std::fs::remove_file(&path).ok();
    result
}

/// DET-05's rendering assertion: a text node naming a family this
/// database certainly does not carry, and one carrying no family
/// attribute at all, must render byte-identical to one naming the pinned
/// family. Equality is only reachable when the host font database was
/// never consulted; on a machine that happens to have the unmatched
/// family installed, a build that could reach it would render that
/// document differently from the other two.
#[test]
fn an_unmatched_font_family_falls_back_to_the_pinned_font() {
    let pinned = render_text_document(" font-family=\"Noto Sans\"");
    // "Arial" is a family this crate's one-font database certainly does
    // not carry, and one many developer machines certainly do.
    let unmatched = render_text_document(" font-family=\"Arial\"");
    let unspecified = render_text_document("");

    assert_eq!(
        pinned.pixels, unmatched.pixels,
        "a document naming a family the database does not carry must render identically to one \
         naming the pinned family"
    );
    assert_eq!(
        pinned.pixels, unspecified.pixels,
        "a document naming no family at all must render identically to one naming the pinned \
         family"
    );

    // The floor that stops this test passing for the wrong reason: all
    // three could agree perfectly by all rendering nothing. The floor
    // below was set from a measured run: rendering "Chrys" at font-size
    // 28 in this fixture produced 571 non-transparent pixels; 100 is
    // comfortably below that measurement and comfortably above zero.
    let non_transparent = pinned
        .pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[3] != 0)
        .count();
    assert!(
        non_transparent > 100,
        "expected a real glyph area, not a blank canvas; measured {non_transparent} \
         non-transparent pixels"
    );
}

/// `SvgSource::with_limits` carrying a `max_file_bytes` below the
/// committed fixture's own size refuses that fixture with the file-size
/// variant, naming both the file's size and the limit.
#[test]
fn an_oversized_file_is_refused_before_parsing() {
    let base_path = fixture("base.svg");
    let actual_size = std::fs::metadata(&base_path)
        .expect("read base.svg's own metadata")
        .len();
    let limit = actual_size - 1;
    let source = SvgSource::with_limits(SvgLimits {
        max_file_bytes: limit,
        ..SvgLimits::default()
    });

    match source.load(&base_path) {
        Err(SvgError::FileTooLarge {
            size,
            limit: reported_limit,
            ..
        }) => {
            assert_eq!(
                size, actual_size,
                "the refusal must name the file's real size"
            );
            assert_eq!(
                reported_limit, limit,
                "the refusal must name the limit it was checked against"
            );
        }
        other => panic!("expected SvgError::FileTooLarge, got {other:?}"),
    }
}

/// The half a metadata check alone cannot prove: the bounded read, called
/// directly, refuses more than its cap without consulting the file's own
/// declared metadata length. A length read from metadata and a length
/// read from the file are two measurements of two moments, and only the
/// second bounds what this process actually allocated.
#[test]
fn the_bounded_read_refuses_more_than_its_cap_without_consulting_metadata() {
    let base_path = fixture("base.svg");
    let actual_size = std::fs::metadata(&base_path)
        .expect("read base.svg's own metadata")
        .len();

    match chrys_source_svg::bounded_read(&base_path, actual_size - 1) {
        Err(SvgError::FileTooLarge { .. }) => {}
        other => panic!("expected SvgError::FileTooLarge below the file's size, got {other:?}"),
    }

    let buffer = chrys_source_svg::bounded_read(&base_path, actual_size + 1)
        .expect("a cap above the file's size must return its whole content");
    let expected = std::fs::read(&base_path).expect("read base.svg directly");
    assert_eq!(
        buffer, expected,
        "a cap above the file's size must return the file's whole byte content"
    );
}

/// An SVG declaring a canvas above `SvgLimits::max_width`/`max_height` is
/// refused with the canvas variant, naming the width, the height and the
/// limit, before any pixel buffer is allocated.
///
/// The declared size is built in the test itself, as a string, per
/// AGENTS.md: build the state a test needs inside the test. 100000 was
/// measured to produce this variant, not the parse variant `usvg` would
/// return for a still-larger declared size; see the plan summary for the
/// values tried.
#[test]
fn an_oversized_declared_canvas_is_refused() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100000" height="100000">
  <rect x="0" y="0" width="10" height="10" fill="#ff0000"/>
</svg>
"##;
    let dir = std::env::temp_dir();
    let path = dir.join("chrys-source-svg-oversized-canvas-test.svg");
    std::fs::write(&path, svg).expect("write the temporary SVG");

    let source = SvgSource::new();
    let result = source.load(&path);
    std::fs::remove_file(&path).ok();

    match result {
        Err(SvgError::CanvasTooLarge {
            width,
            height,
            max_width,
            max_height,
            ..
        }) => {
            assert_eq!(width, 100000, "the refusal must name the declared width");
            assert_eq!(height, 100000, "the refusal must name the declared height");
            assert_eq!(max_width, SvgLimits::default().max_width);
            assert_eq!(max_height, SvgLimits::default().max_height);
        }
        other => panic!("expected SvgError::CanvasTooLarge, got {other:?}"),
    }
}

/// The boundary case that cannot depend on how `usvg` handles a very
/// large declared number at all: `SvgSource::with_limits` carrying a
/// `max_width` of 1 refuses the committed 256 by 256 fixture with the
/// same canvas variant.
#[test]
fn a_max_width_of_one_refuses_the_committed_fixture() {
    let source = SvgSource::with_limits(SvgLimits {
        max_width: 1,
        ..SvgLimits::default()
    });

    match source.load(&fixture("base.svg")) {
        Err(SvgError::CanvasTooLarge {
            width,
            height,
            max_width,
            ..
        }) => {
            assert_eq!(width, 256);
            assert_eq!(height, 256);
            assert_eq!(max_width, 1);
        }
        other => panic!("expected SvgError::CanvasTooLarge, got {other:?}"),
    }
}
