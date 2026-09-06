//! The sequence comparison pipeline. This module holds the one
//! implementation `compare` and `compare_sequence` both run: a single pair
//! is a sequence of one frame on each side.

use chrys_source::Frame;

use crate::verdict::{RefusalReason, Region, Verdict};
use crate::{CompareError, classify, register};

/// Compare a base frame to a candidate frame and return a verdict.
///
/// The order of work is fixed: a shape check, then registration, then the
/// confidence test that decides a refusal, then the local block match, and
/// only then classification. `compare_pair` refuses a pair whose dimensions
/// differ, because this project assumes near-identical pairs and does not
/// scale or crop to make an unequal pair fit. When the shapes match,
/// `compare_pair` runs phase correlation and scores how sharply it peaks; a
/// pair whose peak is not sharp enough to trust is refused there, before
/// any classification work runs on it, because CORE-06 and CORE-07
/// require the engine to say it cannot register a pair rather than guess
/// at one. When the pair clears that test, `compare_pair` runs the local
/// block match, drops any residual pixel that is only antialiasing, and
/// groups what remains into labelled regions, so a pair whose content
/// moved is not reported as changed everywhere just because it was not
/// compared in place, and so two separate changed areas are reported as
/// two regions rather than one that spans both.
///
/// Each region's kind, its offset when it moved, and its colour delta
/// when it was recoloured, all come from `classify::classify_kind`'s own
/// ordered decision list; see that function's own doc comment for the
/// five named rules it applies in order.
fn compare_pair(base: &Frame, candidate: &Frame) -> Result<Verdict, CompareError> {
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

    let mut field = register::block_match(base, candidate, refined.whole)?;
    classify::suppress_antialiasing(&mut field, base, candidate);
    let labelled_regions = classify::label_regions(&field);

    if labelled_regions.is_empty() {
        return Ok(Verdict::Identical);
    }

    let regions = labelled_regions
        .into_iter()
        .map(|labelled| {
            let (kind, offset_px, colour_delta) =
                classify::classify_kind(&labelled, base, candidate, &field.blocks);
            Region {
                kind,
                bbox: labelled.bbox,
                offset_px,
                colour_delta,
            }
        })
        .collect();

    Ok(Verdict::Changed { regions })
}

/// Compare a base sequence to a candidate sequence, index by index, and
/// return one verdict per index.
///
/// Either side holding no frame at all refuses the whole sequence with
/// `RefusalReason::EmptySequence`. Unequal lengths refuse the whole
/// sequence with `RefusalReason::FrameCountMismatch`, naming both counts:
/// this engine pairs frames by index and does not pair a prefix or
/// interpolate a missing frame. When the lengths agree, every index is
/// compared on its own with `compare_pair`, so one index that cannot be
/// compared refuses on its own index and every other index still reports
/// its own verdict.
pub fn compare_sequence(base: &[Frame], candidate: &[Frame]) -> Result<Vec<Verdict>, CompareError> {
    if base.is_empty() || candidate.is_empty() {
        return Ok(vec![Verdict::Refused {
            reason: RefusalReason::EmptySequence,
        }]);
    }

    if base.len() != candidate.len() {
        return Ok(vec![Verdict::Refused {
            reason: RefusalReason::FrameCountMismatch {
                base: base.len(),
                candidate: candidate.len(),
            },
        }]);
    }

    base.iter()
        .zip(candidate.iter())
        .map(|(b, c)| compare_pair(b, c))
        .collect()
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
    fn empty_sequence_refuses_with_a_single_verdict_naming_no_frame() {
        let base: Vec<Frame> = Vec::new();
        let candidate = vec![solid_frame(4, 4, [10, 20, 30, 255])];
        let result = compare_sequence(&base, &candidate).unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            Verdict::Refused {
                reason: RefusalReason::EmptySequence,
            } => {
                let text = result[0].to_string();
                assert!(text.contains("no frame"));
            }
            other => panic!("expected an empty-sequence refusal, got {other:?}"),
        }
    }

    #[test]
    fn unequal_length_sequences_refuse_and_name_both_counts() {
        let base = vec![
            solid_frame(4, 4, [10, 20, 30, 255]),
            solid_frame(4, 4, [10, 20, 30, 255]),
        ];
        let candidate = vec![solid_frame(4, 4, [10, 20, 30, 255])];
        let result = compare_sequence(&base, &candidate).unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            Verdict::Refused {
                reason:
                    RefusalReason::FrameCountMismatch {
                        base: base_count,
                        candidate: candidate_count,
                    },
            } => {
                assert_eq!(*base_count, 2);
                assert_eq!(*candidate_count, 1);
                let text = result[0].to_string();
                assert!(text.contains('2'));
                assert!(text.contains('1'));
            }
            other => panic!("expected a frame-count-mismatch refusal, got {other:?}"),
        }
    }
}
