//! The format-blind comparison engine.
//!
//! This crate depends on no format crate and on no graphics crate, and
//! that stays true for the whole project. It sees only `Frame` values from
//! `chrys-source` and produces a `Verdict`.

#![forbid(unsafe_code)]

pub mod hash;
pub mod residual;
pub mod verdict;

use chrys_source::Frame;
pub use verdict::{BoundingBox, ChangeKind, ColourDelta, RefusalReason, Region, Verdict};

/// The error `compare` returns when it cannot run at all.
#[derive(Debug, thiserror::Error)]
pub enum CompareError {
    /// A frame with zero pixels was given to `compare`.
    #[error("cannot compare an empty frame")]
    EmptyFrame,
    /// The base and candidate frames differ in size, so no per-pixel
    /// buffer can be built over them.
    #[error("shape mismatch: base is {base:?}, candidate is {candidate:?}")]
    ShapeMismatch {
        /// The base frame's width and height, in pixels.
        base: (u32, u32),
        /// The candidate frame's width and height, in pixels.
        candidate: (u32, u32),
    },
}

/// Compare a base frame to a candidate frame and return a verdict.
///
/// `compare` refuses a pair whose dimensions differ, because this project
/// assumes near-identical pairs and does not scale or crop to make an
/// unequal pair fit. When the shapes match, `compare` walks both buffers
/// once and collects the bounding box of every pixel whose RGBA bytes
/// differ.
///
/// The single-region bounding box this function returns is the degenerate
/// case of connected-component labelling, which plan 01-07 replaces with
/// `imageproc`'s labeller so that separate changed areas are reported as
/// separate regions. The `delta_e` this function reports is a stand-in
/// metric: it is the Euclidean distance in straight RGB, not a perceptual
/// colour distance, until plan 01-07 routes colour difference through
/// `palette`'s Lab colour space.
pub fn compare(base: &Frame, candidate: &Frame) -> Result<Verdict, CompareError> {
    if base.pixel_count() == 0 || candidate.pixel_count() == 0 {
        return Err(CompareError::EmptyFrame);
    }

    if !base.same_shape_as(candidate) {
        return Ok(Verdict::Refused {
            reason: RefusalReason::DimensionMismatch {
                base: (base.width, base.height),
                candidate: (candidate.width, candidate.height),
            },
        });
    }

    let base_pixels = base.rgba8();
    let candidate_pixels = candidate.rgba8();
    let width = base.width;
    let height = base.height;

    let mut min_x: Option<u32> = None;
    let mut min_y: Option<u32> = None;
    let mut max_x: Option<u32> = None;
    let mut max_y: Option<u32> = None;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            if base_pixels[idx..idx + 4] != candidate_pixels[idx..idx + 4] {
                min_x = Some(min_x.map_or(x, |v| v.min(x)));
                min_y = Some(min_y.map_or(y, |v| v.min(y)));
                max_x = Some(max_x.map_or(x, |v| v.max(x)));
                max_y = Some(max_y.map_or(y, |v| v.max(y)));
            }
        }
    }

    let (min_x, min_y, max_x, max_y) = match (min_x, min_y, max_x, max_y) {
        (Some(min_x), Some(min_y), Some(max_x), Some(max_y)) => (min_x, min_y, max_x, max_y),
        _ => return Ok(Verdict::Identical),
    };

    let bbox = BoundingBox {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    };

    let idx = ((min_y * width + min_x) * 4) as usize;
    let base_rgba = [
        base_pixels[idx],
        base_pixels[idx + 1],
        base_pixels[idx + 2],
        base_pixels[idx + 3],
    ];
    let candidate_rgba = [
        candidate_pixels[idx],
        candidate_pixels[idx + 1],
        candidate_pixels[idx + 2],
        candidate_pixels[idx + 3],
    ];
    let delta_e = euclidean_rgb_distance(base_rgba, candidate_rgba);

    Ok(Verdict::Changed {
        regions: vec![Region {
            kind: ChangeKind::Recoloured,
            bbox,
            offset_px: None,
            colour_delta: Some(ColourDelta {
                delta_e,
                base: base_rgba,
                candidate: candidate_rgba,
            }),
        }],
    })
}

/// The Euclidean distance between two RGBA8 colours, in straight RGB. This
/// is a stand-in for a perceptual colour-difference formula; see the doc
/// comment on `compare`.
fn euclidean_rgb_distance(base: [u8; 4], candidate: [u8; 4]) -> f32 {
    let mut sum_of_squares = 0.0f32;
    for channel in 0..3 {
        let diff = f32::from(base[channel]) - f32::from(candidate[channel]);
        sum_of_squares += diff * diff;
    }
    sum_of_squares.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrys_source::Frame;

    fn solid_frame(width: u32, height: u32, colour: [u8; 4]) -> Frame {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            pixels.extend_from_slice(&colour);
        }
        Frame {
            pixels,
            width,
            height,
            index: 0,
            hints: Vec::new(),
        }
    }

    #[test]
    fn identical_frames_return_identical() {
        let a = solid_frame(4, 4, [10, 20, 30, 255]);
        let b = solid_frame(4, 4, [10, 20, 30, 255]);
        assert_eq!(compare(&a, &b).unwrap(), Verdict::Identical);
    }

    #[test]
    fn different_dimensions_return_refused_and_no_regions() {
        let a = solid_frame(4, 4, [10, 20, 30, 255]);
        let b = solid_frame(5, 4, [10, 20, 30, 255]);
        let verdict = compare(&a, &b).unwrap();
        match verdict {
            Verdict::Refused {
                reason:
                    RefusalReason::DimensionMismatch {
                        base: (4, 4),
                        candidate: (5, 4),
                    },
            } => {}
            other => panic!("expected a dimension-mismatch refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_recoloured_rectangle_returns_one_recoloured_region() {
        let width = 8;
        let height = 8;
        let base = solid_frame(width, height, [240, 240, 240, 255]);
        let mut candidate = solid_frame(width, height, [240, 240, 240, 255]);

        // Recolour a 2x2 rectangle at (3, 3) in the candidate only.
        for y in 3..5u32 {
            for x in 3..5u32 {
                let idx = ((y * width + x) * 4) as usize;
                candidate.pixels[idx..idx + 4].copy_from_slice(&[10, 200, 10, 255]);
            }
        }

        let verdict = compare(&base, &candidate).unwrap();
        match verdict {
            Verdict::Changed { regions } => {
                assert_eq!(regions.len(), 1);
                let region = &regions[0];
                assert_eq!(region.kind, ChangeKind::Recoloured);
                assert_eq!(
                    region.bbox,
                    BoundingBox {
                        x: 3,
                        y: 3,
                        width: 2,
                        height: 2
                    }
                );
                let delta = region.colour_delta.as_ref().expect("colour delta present");
                assert_eq!(delta.base, [240, 240, 240, 255]);
                assert_eq!(delta.candidate, [10, 200, 10, 255]);
            }
            other => panic!("expected exactly one changed region, got {other:?}"),
        }
    }

    #[test]
    fn empty_frame_returns_compare_error() {
        let empty = Frame {
            pixels: Vec::new(),
            width: 0,
            height: 0,
            index: 0,
            hints: Vec::new(),
        };
        let other = solid_frame(1, 1, [0, 0, 0, 255]);
        assert!(matches!(
            compare(&empty, &other),
            Err(CompareError::EmptyFrame)
        ));
    }
}
