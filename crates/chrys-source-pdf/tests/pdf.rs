//! Tests for `PdfSource`, through its public `Source` implementation.

use std::path::{Path, PathBuf};

use chrys_source::Source;
use chrys_source_pdf::{PdfError, PdfLimits, PdfSource, looks_like_pdf};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/formats/pdf")
        .join(name)
}

/// Write `bytes` to a new file in a directory that this test owns.
fn scratch_file(test: &str, bytes: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chrys-pdf-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("make the scratch directory");
    let path = dir.join("input");
    std::fs::write(&path, bytes).expect("write the scratch file");
    path
}

/// A minimal PDF with `pages` empty pages of 100 by 50 points.
fn minimal_pdf(pages: usize) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {pages} >>",
            (0..pages)
                .map(|n| format!("{} 0 R", n + 3))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    ];
    for _ in 0..pages {
        objects.push("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 50] >>".to_string());
    }
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", number + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn dark_pixels(pixels: &[u8]) -> usize {
    pixels
        .chunks_exact(4)
        .filter(|p| p[0] < 128 && p[1] < 128 && p[2] < 128)
        .count()
}

#[test]
fn each_page_becomes_one_frame_at_two_pixels_per_point() {
    let frames = PdfSource::new().load(&fixture("base.pdf")).expect("load");
    assert_eq!(frames.len(), 2);
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(frame.index, index);
        assert_eq!((frame.width, frame.height), (640, 560));
        assert_eq!(frame.pixels.len(), 640 * 560 * 4);
        assert!(frame.pixels.chunks_exact(4).all(|p| p[3] == 255));
    }
}

#[test]
fn the_standard_font_draws_glyphs() {
    let frames = PdfSource::new().load(&fixture("base.pdf")).expect("load");
    // Page 2 holds text only. Without a font it is all white.
    assert!(dark_pixels(&frames[1].pixels) > 100);
}

#[test]
fn two_loads_give_the_same_bytes() {
    let source = PdfSource::new();
    let first = source.load(&fixture("base.pdf")).expect("first load");
    let second = source.load(&fixture("base.pdf")).expect("second load");
    assert_eq!(first, second);
}

#[test]
fn the_candidate_changes_page_one_and_keeps_page_two() {
    let source = PdfSource::new();
    let base = source.load(&fixture("base.pdf")).expect("base");
    let candidate = source.load(&fixture("candidate.pdf")).expect("candidate");
    assert_ne!(base[0].pixels, candidate[0].pixels);
    assert_eq!(base[1].pixels, candidate[1].pixels);
}

#[test]
fn the_sniff_reads_the_content_not_the_extension() {
    let pdf = scratch_file("sniff-yes", &minimal_pdf(1));
    let text = scratch_file("sniff-no", b"not a document");
    assert!(looks_like_pdf(&pdf).expect("read"));
    assert!(!looks_like_pdf(&text).expect("read"));
}

#[test]
fn a_file_above_the_byte_limit_is_refused() {
    let path = scratch_file("too-large", &minimal_pdf(1));
    let limits = PdfLimits {
        max_file_bytes: 64,
        ..PdfLimits::default()
    };
    let error = PdfSource::with_limits(limits)
        .load(&path)
        .expect_err("refused");
    assert!(matches!(error, PdfError::FileTooLarge { limit: 64, .. }));
}

#[test]
fn a_document_above_the_page_limit_is_refused() {
    let path = scratch_file("too-many", &minimal_pdf(3));
    let limits = PdfLimits {
        max_pages: 2,
        ..PdfLimits::default()
    };
    let error = PdfSource::with_limits(limits)
        .load(&path)
        .expect_err("refused");
    assert!(matches!(
        error,
        PdfError::TooManyPages {
            pages: 3,
            limit: 2,
            ..
        }
    ));
}

#[test]
fn a_page_above_the_canvas_limit_is_refused() {
    let path = scratch_file("too-wide", &minimal_pdf(1));
    let limits = PdfLimits {
        max_width: 199,
        ..PdfLimits::default()
    };
    let error = PdfSource::with_limits(limits)
        .load(&path)
        .expect_err("refused");
    assert!(matches!(
        error,
        PdfError::BadPageSize {
            page: 1,
            width: 200,
            height: 100,
            ..
        }
    ));
}

#[test]
fn a_file_that_is_not_a_pdf_is_refused() {
    let path = scratch_file("garbage", b"%PDF-1.4\nthis is not a document\n");
    let error = PdfSource::new().load(&path).expect_err("refused");
    assert!(matches!(error, PdfError::Parse { .. }), "{error:?}");
}

#[test]
fn a_missing_file_is_an_io_error() {
    let error = PdfSource::new()
        .load(Path::new("/no/such/file.pdf"))
        .expect_err("refused");
    assert!(matches!(error, PdfError::Io { .. }));
}
