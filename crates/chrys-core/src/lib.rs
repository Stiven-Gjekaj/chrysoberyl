//! The format-blind comparison engine.
//!
//! This crate depends on no format crate and on no graphics crate, and
//! that stays true for the whole project. It sees only `Frame` values from
//! `chrys-source` and produces a `Verdict`.

#![forbid(unsafe_code)]

pub mod hash;
pub mod register;
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
/// The order of work is fixed: a shape check, then registration, then the
/// confidence test that decides a refusal, then the local block match, and
/// only then classification. `compare` refuses a pair whose dimensions
/// differ, because this project assumes near-identical pairs and does not
/// scale or crop to make an unequal pair fit. When the shapes match,
/// `compare` runs phase correlation and scores how sharply it peaks; a
/// pair whose peak is not sharp enough to trust is refused there, before
/// any classification work runs on it, because CORE-06 and CORE-07
/// require the engine to say it cannot register a pair rather than guess
/// at one. When the pair clears that test, `compare` runs the local block
/// match and walks the resulting `ResidualField` once, collecting the
/// bounding box of every pixel whose residual colour bytes are non-zero,
/// so a pair whose content moved is not reported as changed everywhere
/// just because it was not compared in place.
///
/// The single-region bounding box this function returns is the degenerate
/// case of connected-component labelling, which plan 01-07 replaces with
/// `imageproc`'s labeller so that separate changed areas are reported as
/// separate regions, and plan 01-07 is also where the `ResidualField`'s
/// own per-block offsets and colour bytes at the bounding box's own
/// registered position become the region's reported data. The `delta_e`
/// this function reports is a stand-in metric: it is the Euclidean
/// distance in straight RGB, not a perceptual colour distance, until plan
/// 01-07 routes colour difference through `palette`'s Lab colour space.
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

    // Refusing before classifying is the point, not an optimisation. This
    // project's core value only holds for a near-identical pair; a pair
    // whose registration cannot find a peak sharp enough to trust falls
    // outside that assumption, so nothing past this point may run on it.
    let (refined, surface) = register::phase_correlate(base, candidate)?;
    let peak_index = register::peak_index(refined.whole, surface.resolution);
    let confidence = register::assess_peak(&surface, peak_index);
    if confidence.ratio < register::REFUSAL_THRESHOLD {
        return Ok(Verdict::Refused {
            reason: RefusalReason::PeakConfidenceTooLow {
                ratio: confidence.ratio,
                threshold: register::REFUSAL_THRESHOLD,
            },
        });
    }

    let field = register::block_match(base, candidate, refined.whole)?;

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
            if field.samples[idx..idx + 3] != [0, 0, 0] {
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

    fn paint_rect(
        pixels: &mut [u8],
        stride: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        colour: [u8; 4],
    ) {
        for row in y..y + height {
            for col in x..x + width {
                let idx = ((row * stride + col) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&colour);
            }
        }
    }

    /// A frame with four anchor rectangles, away from the region a test
    /// changes, so phase correlation has real edges to lock onto. A flat
    /// frame with only the tested change gives registration almost no
    /// structure, and reads as unregisterable even on a pair a person
    /// would call the same picture with one patch touched up.
    fn frame_with_anchor_rects(width: u32, height: u32, background: [u8; 4]) -> Frame {
        let mut frame = solid_frame(width, height, background);
        paint_rect(&mut frame.pixels, width, 4, 4, 80, 60, [200, 40, 40, 255]);
        paint_rect(
            &mut frame.pixels,
            width,
            width - 90,
            4,
            80,
            50,
            [40, 160, 40, 255],
        );
        paint_rect(
            &mut frame.pixels,
            width,
            4,
            height - 80,
            60,
            70,
            [40, 40, 200, 255],
        );
        paint_rect(
            &mut frame.pixels,
            width,
            width - 90,
            height - 90,
            70,
            80,
            [200, 40, 200, 255],
        );
        frame
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
        let width = 256;
        let height = 256;
        let mut base = frame_with_anchor_rects(width, height, [240, 240, 240, 255]);

        // Plan 01-06 wires a local block match into `compare`. A block
        // match cannot tell a purely recoloured, flat 64x64 rectangle from
        // a block that moved a few pixels into an equally flat
        // surrounding background: both explanations score the same low
        // sum of absolute differences, and the plan's own tie-break rule
        // only breaks a tie between offsets, not between a real recolour
        // and a coincidental colour match found by drifting off the
        // block's own area. A moat of a colour far from both the
        // background and the recolour, wide enough to cover the block
        // search's own reachable radius, removes that coincidence: any
        // block drifting into it scores worse, not better, than staying
        // in place, so the recoloured block's own true, zero offset stays
        // the only good answer.
        paint_rect(&mut base.pixels, width, 72, 72, 112, 112, [0, 0, 0, 255]);
        paint_rect(
            &mut base.pixels,
            width,
            96,
            96,
            64,
            64,
            [240, 240, 240, 255],
        );
        let mut candidate = base.clone();

        // Recolour a 64x64 rectangle at (96, 96) in the candidate only.
        // The anchor rectangles `frame_with_anchor_rects` paints (and the
        // moat above) stay identical in both frames, so registration has
        // structure to register this near-identical pair with confidence,
        // before classification ever sees it.
        for y in 96..160u32 {
            for x in 96..160u32 {
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
                        x: 96,
                        y: 96,
                        width: 64,
                        height: 64
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
