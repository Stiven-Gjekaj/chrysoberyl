//! The format-blind comparison engine.
//!
//! This crate depends on no format crate and on no graphics crate, and
//! that stays true for the whole project. It sees only `Frame` values from
//! `chrys-source` and produces a `Verdict`.

#![forbid(unsafe_code)]

pub mod classify;
pub mod hash;
pub mod register;
pub mod residual;
pub mod sequence;
pub mod verdict;

use chrys_source::Frame;
pub use sequence::compare_sequence;
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
/// This is a thin wrapper over `compare_sequence`, called with a slice of
/// one frame on each side. There is one comparison pipeline in this crate;
/// see `sequence::compare_sequence` for the order of work it runs. A
/// one-against-one sequence can reach neither the empty-sequence check nor
/// the frame-count check inside `compare_sequence`, so the returned vector
/// always holds exactly one verdict. The empty case is handled explicitly
/// below rather than by unwrapping or indexing, because an engine that
/// produced no verdict is a refusal, and this project never panics on a
/// path a caller can reach.
pub fn compare(base: &Frame, candidate: &Frame) -> Result<Verdict, CompareError> {
    let mut verdicts =
        sequence::compare_sequence(std::slice::from_ref(base), std::slice::from_ref(candidate))?;
    Ok(verdicts.pop().unwrap_or(Verdict::Refused {
        reason: RefusalReason::EmptySequence,
    }))
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
        // A mid-grey distinct from the frame's own background: a region
        // whose base colour already equals the frame's modal colour is
        // background by classify_kind's own rule, and would report
        // `Added`, not `Recoloured`. This rectangle must already be
        // content, not background, for the test to name what it claims.
        paint_rect(
            &mut base.pixels,
            width,
            96,
            96,
            64,
            64,
            [180, 180, 180, 255],
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
                assert_eq!(delta.base, [180, 180, 180, 255]);
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
