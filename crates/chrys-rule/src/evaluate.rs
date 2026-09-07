//! Turning a computed `Verdict` into a per-region tolerated-or-violation
//! outcome, against a loaded rule set.
//!
//! The region/hint overlap test in this file holds no `f32` and no `f64`:
//! every rectangle calculation is `u64` integer arithmetic, the same
//! discipline phase 1 established for the engine's own geometry. The one
//! floating point comparison a colour tolerance needs lives in
//! `Tolerance::tolerates` (`crate::lib`), which reads an already-computed
//! `f32` field and compares it once; it introduces no new floating point
//! calculation of its own. `Mask::tolerates` (`crate::mask`) is `u64`
//! integer arithmetic too, for the same reason.

use crate::{LoadedRules, Rule, RuleError, Scope};

/// Whether one region, under one rule set, was tolerated or is a
/// violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionOutcome {
    /// A rule of the same kind, whose scope this region's box falls
    /// inside, holds its own tolerance. `rule_index` names which rule of
    /// `LoadedRules::rules` matched, in that vector's own order.
    Tolerated { rule_index: usize },
    /// No rule tolerates this region.
    Violation,
}

/// The outcome of evaluating one `Verdict` against a rule set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleOutcome {
    /// One outcome per region of a `Changed` verdict, in the same order
    /// the verdict named them. Empty for `Identical` and for `Refused`.
    pub regions: Vec<RegionOutcome>,
}

impl RuleOutcome {
    /// True when this outcome holds no `Violation`.
    ///
    /// An empty outcome is clean by this definition, which is correct for
    /// `Identical` (nothing to tolerate) but a caller must never read an
    /// empty outcome from a `Refused` verdict as a pass: `evaluate` never
    /// produces one for a reason other than "nothing to check here", and a
    /// refusal is not a change a rule can tolerate.
    pub fn is_clean(&self) -> bool {
        !self
            .regions
            .iter()
            .any(|outcome| matches!(outcome, RegionOutcome::Violation))
    }
}

/// Return the name of the hint in `hints` whose rectangle covers at least
/// half of `bbox`'s own area, or `None` when no hint covers that much.
///
/// Every calculation here is `u64` integer arithmetic: no `f32`, no `f64`.
/// The majority-wins rule mirrors
/// `chrys_core::classify::kind::majority_block_offset`'s own idiom rather
/// than inventing a second convention for the same question.
pub fn overlapping_hint_name<'a>(
    bbox: &chrys_core::BoundingBox,
    hints: &'a [chrys_source::RegionHint],
) -> Option<&'a str> {
    let bbox_area = u64::from(bbox.width) * u64::from(bbox.height);
    if bbox_area == 0 {
        return None;
    }
    let bbox_x1 = u64::from(bbox.x) + u64::from(bbox.width);
    let bbox_y1 = u64::from(bbox.y) + u64::from(bbox.height);

    for hint in hints {
        let hint_x1 = u64::from(hint.x) + u64::from(hint.width);
        let hint_y1 = u64::from(hint.y) + u64::from(hint.height);

        let overlap_x0 = u64::from(bbox.x).max(u64::from(hint.x));
        let overlap_y0 = u64::from(bbox.y).max(u64::from(hint.y));
        let overlap_x1 = bbox_x1.min(hint_x1);
        let overlap_y1 = bbox_y1.min(hint_y1);

        if overlap_x1 <= overlap_x0 || overlap_y1 <= overlap_y0 {
            continue;
        }

        let overlap_area = (overlap_x1 - overlap_x0) * (overlap_y1 - overlap_y0);
        if overlap_area * 2 >= bbox_area {
            return Some(&hint.name);
        }
    }
    None
}

/// Whether `rule` tolerates `region`, given the hints available on the
/// frame `region` was found on, and that frame's own size (needed only to
/// check a mask-scoped rule's mask against it).
///
/// A rule never tolerates a region of a kind it does not name. A
/// mask-scoped rule reads its own decoded mask from `rules`, the same
/// `LoadedRules` value `load_rules` already resolved and decoded every
/// mask into while the rule file loaded (RULE-02's mask half); this
/// function decodes nothing itself.
fn rule_tolerates(
    rule: &Rule,
    region: &chrys_core::Region,
    hints: &[chrys_source::RegionHint],
    rules: &LoadedRules,
    frame_size: (u32, u32),
) -> Result<bool, RuleError> {
    if rule.kind != region.kind {
        return Ok(false);
    }

    let in_scope = match &rule.scope {
        Scope::Region(name) => overlapping_hint_name(&region.bbox, hints) == Some(name.as_str()),
        Scope::Mask(mask_path) => {
            let mask = rules.mask_for(mask_path).unwrap_or_else(|| {
                panic!(
                    "a Scope::Mask rule's own mask was not decoded by load_rules: {}",
                    mask_path.display()
                )
            });
            let (frame_width, frame_height) = frame_size;
            if mask.width() != frame_width || mask.height() != frame_height {
                return Err(RuleError::MaskSizeMismatch {
                    mask_path: mask_path.clone(),
                    mask_width: mask.width(),
                    mask_height: mask.height(),
                    frame_width,
                    frame_height,
                });
            }
            mask.tolerates(&region.bbox)
        }
    };
    if !in_scope {
        return Ok(false);
    }

    Ok(rule.tolerance.tolerates(region))
}

/// Evaluate `verdict` against `rules`, using `hints` (the frame's own
/// region hints, the same side the verdict's regions were found on) to
/// resolve a region-scoped rule, and `frame_size` (that same frame's own
/// width and height) to check a mask-scoped rule's mask against it.
///
/// A `Changed` verdict produces one `RegionOutcome` per region, in the
/// order `verdict` names them. An `Identical` verdict produces an empty
/// outcome. A `Refused` verdict produces an empty outcome too: `compare`
/// could not even decide whether this pair changed, and a rule tolerates
/// a change, not a refusal, so a caller must not read that emptiness as a
/// pass.
///
/// Returns `Err` only when a mask-scoped rule's own mask does not match
/// `frame_size`: refusing rather than scaling, cropping or padding a mask
/// that does not fit, the same posture `chrys_core::RefusalReason::DimensionMismatch`
/// already takes for a pair of frames (T-03-11).
pub fn evaluate(
    verdict: &chrys_core::Verdict,
    hints: &[chrys_source::RegionHint],
    rules: &LoadedRules,
    frame_size: (u32, u32),
) -> Result<RuleOutcome, RuleError> {
    let regions = match verdict {
        chrys_core::Verdict::Changed { regions } => regions,
        chrys_core::Verdict::Identical | chrys_core::Verdict::Refused { .. } => {
            return Ok(RuleOutcome {
                regions: Vec::new(),
            });
        }
    };

    let mut outcomes = Vec::with_capacity(regions.len());
    for region in regions {
        let mut tolerated_by = None;
        for (rule_index, rule) in rules.rules.iter().enumerate() {
            if rule_tolerates(rule, region, hints, rules, frame_size)? {
                tolerated_by = Some(rule_index);
                break;
            }
        }
        outcomes.push(match tolerated_by {
            Some(rule_index) => RegionOutcome::Tolerated { rule_index },
            None => RegionOutcome::Violation,
        });
    }

    Ok(RuleOutcome { regions: outcomes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tolerance;
    use chrys_core::{BoundingBox, ChangeKind, ColourDelta, Region, Verdict};
    use chrys_source::RegionHint;
    use std::collections::HashMap;

    fn hint(name: &str, x: u32, y: u32, width: u32, height: u32) -> RegionHint {
        RegionHint {
            name: name.to_string(),
            x,
            y,
            width,
            height,
        }
    }

    fn bbox(x: u32, y: u32, width: u32, height: u32) -> BoundingBox {
        BoundingBox {
            x,
            y,
            width,
            height,
        }
    }

    /// Write a flat-colour PNG to a fresh temporary path, decode it through
    /// `crate::mask::load_mask`, remove the file, and return the decoded
    /// mask. `label` keeps every temporary file this test module writes
    /// from colliding with another test's own file.
    fn temp_mask(
        label: &str,
        width: u32,
        height: u32,
        pixel: [u8; 4],
    ) -> (std::path::PathBuf, crate::mask::Mask) {
        let path = std::env::temp_dir().join(format!("chrys-rule-evaluate-test-{label}.png"));
        image::RgbaImage::from_pixel(width, height, image::Rgba(pixel))
            .save(&path)
            .expect("write temp mask png");
        let mask = crate::mask::load_mask(&path, &chrys_source_raster::DecodeLimits::default())
            .expect("a well-formed PNG decodes");
        std::fs::remove_file(&path).ok();
        (path, mask)
    }

    #[test]
    fn region_scoped_rule_matches_overlapping_region() {
        // The bbox sits wholly inside the hint, well over half its area.
        let hints = vec![hint("badge", 0, 0, 100, 100)];
        let bbox = bbox(10, 10, 20, 20);
        assert_eq!(overlapping_hint_name(&bbox, &hints), Some("badge"));
    }

    #[test]
    fn overlapping_hint_name_returns_none_when_less_than_half_overlaps() {
        // Only a quarter of the bbox's own area lies inside the hint.
        let hints = vec![hint("badge", 50, 50, 50, 50)];
        let bbox = bbox(40, 40, 20, 20);
        assert_eq!(overlapping_hint_name(&bbox, &hints), None);
    }

    #[test]
    fn evaluate_tolerates_a_recoloured_region_inside_its_named_region() {
        let rules = LoadedRules {
            rules: vec![Rule {
                kind: ChangeKind::Recoloured,
                scope: Scope::Region("badge".to_string()),
                tolerance: Tolerance::Colour {
                    max_delta_e: 20.0,
                    max_alpha_delta: 0,
                },
            }],
            ..Default::default()
        };
        let hints = vec![hint("badge", 0, 0, 100, 100)];
        let region = Region {
            kind: ChangeKind::Recoloured,
            bbox: bbox(10, 10, 20, 20),
            offset_px: None,
            colour_delta: Some(ColourDelta {
                delta_e: 15.0,
                base: [0, 0, 0, 255],
                candidate: [10, 10, 10, 255],
            }),
        };
        let verdict = Verdict::Changed {
            regions: vec![region],
        };

        let outcome = evaluate(&verdict, &hints, &rules, (100, 100)).expect("no mask involved");
        assert_eq!(
            outcome.regions,
            vec![RegionOutcome::Tolerated { rule_index: 0 }]
        );
        assert!(outcome.is_clean());
    }

    /// RULE-02's mask half: a rule scoped by a mask matches a region whose
    /// box sits on tolerated (white and opaque) mask pixels, using the same
    /// majority rule a named region already uses.
    #[test]
    fn mask_scoped_rule_matches_overlapping_region() {
        let (mask_path, mask) = temp_mask("mask-scoped-match", 100, 100, [255, 255, 255, 255]);

        let rule = Rule {
            kind: ChangeKind::Recoloured,
            scope: Scope::Mask(mask_path.clone()),
            tolerance: Tolerance::Colour {
                max_delta_e: 20.0,
                max_alpha_delta: 0,
            },
        };
        let mut masks = HashMap::new();
        masks.insert(mask_path, mask);
        let rules = LoadedRules {
            rules: vec![rule],
            masks,
        };

        let region = Region {
            kind: ChangeKind::Recoloured,
            bbox: bbox(10, 10, 20, 20),
            offset_px: None,
            colour_delta: Some(ColourDelta {
                delta_e: 15.0,
                base: [0, 0, 0, 255],
                candidate: [10, 10, 10, 255],
            }),
        };
        let verdict = Verdict::Changed {
            regions: vec![region],
        };

        let outcome = evaluate(&verdict, &[], &rules, (100, 100)).expect("mask size agrees");
        assert_eq!(
            outcome.regions,
            vec![RegionOutcome::Tolerated { rule_index: 0 }]
        );
        assert!(outcome.is_clean());
    }

    /// A mask-scoped rule never matches a region outside the mask's own
    /// tolerated pixels, the mirror case of the match above.
    #[test]
    fn mask_scoped_rule_does_not_match_a_region_over_untolerated_pixels() {
        // White on transparency: every pixel fails the alpha term of D-02,
        // so nothing is tolerated anywhere on this mask.
        let (mask_path, mask) = temp_mask("mask-scoped-no-match", 100, 100, [255, 255, 255, 0]);

        let rule = Rule {
            kind: ChangeKind::Recoloured,
            scope: Scope::Mask(mask_path.clone()),
            tolerance: Tolerance::Colour {
                max_delta_e: 1_000.0,
                max_alpha_delta: 255,
            },
        };
        let mut masks = HashMap::new();
        masks.insert(mask_path, mask);
        let rules = LoadedRules {
            rules: vec![rule],
            masks,
        };

        let region = Region {
            kind: ChangeKind::Recoloured,
            bbox: bbox(10, 10, 20, 20),
            offset_px: None,
            colour_delta: Some(ColourDelta {
                delta_e: 0.0,
                base: [0, 0, 0, 255],
                candidate: [1, 1, 1, 255],
            }),
        };
        let verdict = Verdict::Changed {
            regions: vec![region],
        };

        let outcome = evaluate(&verdict, &[], &rules, (100, 100)).expect("mask size agrees");
        assert_eq!(outcome.regions, vec![RegionOutcome::Violation]);
    }

    /// A mask whose own size differs from the frame it is scoped against is
    /// refused, naming both sizes, rather than scaled, cropped or padded to
    /// fit (T-03-11).
    #[test]
    fn a_mask_sized_differently_from_the_frame_is_refused_naming_both_sizes() {
        let (mask_path, mask) = temp_mask("mask-size-mismatch", 50, 50, [255, 255, 255, 255]);

        let rule = Rule {
            kind: ChangeKind::Recoloured,
            scope: Scope::Mask(mask_path.clone()),
            tolerance: Tolerance::Colour {
                max_delta_e: 20.0,
                max_alpha_delta: 0,
            },
        };
        let mut masks = HashMap::new();
        masks.insert(mask_path, mask);
        let rules = LoadedRules {
            rules: vec![rule],
            masks,
        };

        let region = Region {
            kind: ChangeKind::Recoloured,
            bbox: bbox(10, 10, 20, 20),
            offset_px: None,
            colour_delta: Some(ColourDelta {
                delta_e: 15.0,
                base: [0, 0, 0, 255],
                candidate: [10, 10, 10, 255],
            }),
        };
        let verdict = Verdict::Changed {
            regions: vec![region],
        };

        // The mask is 50x50; the frame is 100x100.
        let error = evaluate(&verdict, &[], &rules, (100, 100))
            .expect_err("a size mismatch between mask and frame is refused");
        let message = error.to_string();
        match error {
            RuleError::MaskSizeMismatch {
                mask_width,
                mask_height,
                frame_width,
                frame_height,
                ..
            } => {
                assert_eq!((mask_width, mask_height), (50, 50));
                assert_eq!((frame_width, frame_height), (100, 100));
                assert!(message.contains("50x50"), "message: {message}");
                assert!(message.contains("100x100"), "message: {message}");
            }
            other => panic!("expected RuleError::MaskSizeMismatch, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_returns_empty_outcome_for_a_refused_verdict() {
        let rules = LoadedRules::default();
        let verdict = Verdict::Refused {
            reason: chrys_core::RefusalReason::EmptySequence,
        };
        let outcome = evaluate(&verdict, &[], &rules, (0, 0)).expect("no mask to check");
        assert!(outcome.regions.is_empty());
        // Empty is not a pass in the caller's own reading; this test only
        // proves evaluate() itself stays empty here, per its own doc
        // comment.
        assert!(outcome.is_clean());
    }

    /// One test per `ChangeKind`, proving a tolerance means one thing per
    /// kind of change: `moved` reads an offset, `recoloured` reads a
    /// colour difference, and `added`/`removed`/`resized` read a bounding
    /// box area. The final case proves a rule never tolerates a region of
    /// a kind it does not itself name.
    #[test]
    fn tolerance_is_scoped_by_change_kind() {
        let hints = vec![hint("badge", 0, 0, 100, 100)];

        // moved: within the offset limit on both axes tolerates; above it
        // on either axis does not.
        let moved_rule = Rule {
            kind: ChangeKind::Moved,
            scope: Scope::Region("badge".to_string()),
            tolerance: Tolerance::MaxOffsetPx(5),
        };
        let rules = LoadedRules {
            rules: vec![moved_rule],
            ..Default::default()
        };
        let within = Region {
            kind: ChangeKind::Moved,
            bbox: bbox(10, 10, 20, 20),
            offset_px: Some((5, -5)),
            colour_delta: None,
        };
        let outcome = evaluate(
            &Verdict::Changed {
                regions: vec![within],
            },
            &hints,
            &rules,
            (100, 100),
        )
        .expect("no mask involved");
        assert_eq!(
            outcome.regions,
            vec![RegionOutcome::Tolerated { rule_index: 0 }]
        );
        let above = Region {
            kind: ChangeKind::Moved,
            bbox: bbox(10, 10, 20, 20),
            offset_px: Some((6, 0)),
            colour_delta: None,
        };
        let outcome = evaluate(
            &Verdict::Changed {
                regions: vec![above],
            },
            &hints,
            &rules,
            (100, 100),
        )
        .expect("no mask involved");
        assert_eq!(outcome.regions, vec![RegionOutcome::Violation]);

        // recoloured: at or below the colour limit tolerates.
        let recoloured_rule = Rule {
            kind: ChangeKind::Recoloured,
            scope: Scope::Region("badge".to_string()),
            tolerance: Tolerance::Colour {
                max_delta_e: 10.0,
                max_alpha_delta: 0,
            },
        };
        let rules = LoadedRules {
            rules: vec![recoloured_rule],
            ..Default::default()
        };
        let region = Region {
            kind: ChangeKind::Recoloured,
            bbox: bbox(10, 10, 20, 20),
            offset_px: None,
            colour_delta: Some(ColourDelta {
                delta_e: 10.0,
                base: [0, 0, 0, 255],
                candidate: [5, 5, 5, 255],
            }),
        };
        let outcome = evaluate(
            &Verdict::Changed {
                regions: vec![region],
            },
            &hints,
            &rules,
            (100, 100),
        )
        .expect("no mask involved");
        assert_eq!(
            outcome.regions,
            vec![RegionOutcome::Tolerated { rule_index: 0 }]
        );

        // added, removed, resized: at or below the area limit tolerates.
        for kind in [ChangeKind::Added, ChangeKind::Removed, ChangeKind::Resized] {
            let rule = Rule {
                kind,
                scope: Scope::Region("badge".to_string()),
                tolerance: Tolerance::MaxAreaPx(500),
            };
            let rules = LoadedRules {
                rules: vec![rule],
                ..Default::default()
            };
            // 20 * 20 = 400, at or below the 500-pixel limit.
            let region = Region {
                kind,
                bbox: bbox(10, 10, 20, 20),
                offset_px: None,
                colour_delta: None,
            };
            let outcome = evaluate(
                &Verdict::Changed {
                    regions: vec![region],
                },
                &hints,
                &rules,
                (100, 100),
            )
            .expect("no mask involved");
            assert_eq!(
                outcome.regions,
                vec![RegionOutcome::Tolerated { rule_index: 0 }],
                "kind {kind:?} should be tolerated at or below its area limit"
            );
        }

        // allow = true tolerates every region of its own kind, in scope.
        let allow_rule = Rule {
            kind: ChangeKind::Added,
            scope: Scope::Region("badge".to_string()),
            tolerance: Tolerance::Allow(true),
        };
        let rules = LoadedRules {
            rules: vec![allow_rule],
            ..Default::default()
        };
        let large_added_region = Region {
            kind: ChangeKind::Added,
            bbox: bbox(0, 0, 100, 100),
            offset_px: None,
            colour_delta: None,
        };
        let outcome = evaluate(
            &Verdict::Changed {
                regions: vec![large_added_region],
            },
            &hints,
            &rules,
            (100, 100),
        )
        .expect("no mask involved");
        assert_eq!(
            outcome.regions,
            vec![RegionOutcome::Tolerated { rule_index: 0 }]
        );

        // A rule of one kind never tolerates a region of another kind,
        // even with a generous limit and a matching scope.
        let moved_only_rule = Rule {
            kind: ChangeKind::Moved,
            scope: Scope::Region("badge".to_string()),
            tolerance: Tolerance::MaxOffsetPx(1_000),
        };
        let rules = LoadedRules {
            rules: vec![moved_only_rule],
            ..Default::default()
        };
        let recoloured_region = Region {
            kind: ChangeKind::Recoloured,
            bbox: bbox(10, 10, 20, 20),
            offset_px: None,
            colour_delta: Some(ColourDelta {
                delta_e: 0.0,
                base: [0, 0, 0, 255],
                candidate: [0, 0, 0, 255],
            }),
        };
        let outcome = evaluate(
            &Verdict::Changed {
                regions: vec![recoloured_region],
            },
            &hints,
            &rules,
            (100, 100),
        )
        .expect("no mask involved");
        assert_eq!(outcome.regions, vec![RegionOutcome::Violation]);
    }

    #[test]
    fn a_moved_region_with_no_offset_is_a_violation_not_a_pass() {
        let rule = Rule {
            kind: ChangeKind::Moved,
            scope: Scope::Region("badge".to_string()),
            tolerance: Tolerance::MaxOffsetPx(1_000),
        };
        let hints = vec![hint("badge", 0, 0, 100, 100)];
        let region = Region {
            kind: ChangeKind::Moved,
            bbox: bbox(10, 10, 20, 20),
            offset_px: None,
            colour_delta: None,
        };
        let rules = LoadedRules {
            rules: vec![rule],
            ..Default::default()
        };
        let outcome = evaluate(
            &Verdict::Changed {
                regions: vec![region],
            },
            &hints,
            &rules,
            (100, 100),
        )
        .expect("no mask involved");
        assert_eq!(outcome.regions, vec![RegionOutcome::Violation]);
    }

    #[test]
    fn a_recoloured_region_with_no_colour_delta_is_a_violation_not_a_pass() {
        let rule = Rule {
            kind: ChangeKind::Recoloured,
            scope: Scope::Region("badge".to_string()),
            tolerance: Tolerance::Colour {
                max_delta_e: 1_000.0,
                max_alpha_delta: 255,
            },
        };
        let hints = vec![hint("badge", 0, 0, 100, 100)];
        let region = Region {
            kind: ChangeKind::Recoloured,
            bbox: bbox(10, 10, 20, 20),
            offset_px: None,
            colour_delta: None,
        };
        let rules = LoadedRules {
            rules: vec![rule],
            ..Default::default()
        };
        let outcome = evaluate(
            &Verdict::Changed {
                regions: vec![region],
            },
            &hints,
            &rules,
            (100, 100),
        )
        .expect("no mask involved");
        assert_eq!(outcome.regions, vec![RegionOutcome::Violation]);
    }

    /// The failure `chrys_core::ColourDelta`'s own doc comment predicts,
    /// and phase 1 already paid for once: a colour tolerance with no
    /// alpha limit must not silently swallow an alpha-only change.
    #[test]
    fn a_recoloured_rule_with_a_generous_colour_limit_does_not_tolerate_an_alpha_only_change() {
        let rule = Rule {
            kind: ChangeKind::Recoloured,
            scope: Scope::Region("badge".to_string()),
            tolerance: Tolerance::Colour {
                max_delta_e: 1_000.0,
                max_alpha_delta: 0,
            },
        };
        let hints = vec![hint("badge", 0, 0, 100, 100)];
        // Equal red, green and blue bytes; only the fourth byte differs.
        let region = Region {
            kind: ChangeKind::Recoloured,
            bbox: bbox(10, 10, 20, 20),
            offset_px: None,
            colour_delta: Some(ColourDelta {
                delta_e: 0.0,
                base: [10, 20, 30, 255],
                candidate: [10, 20, 30, 0],
            }),
        };
        let rules = LoadedRules {
            rules: vec![rule],
            ..Default::default()
        };
        let outcome = evaluate(
            &Verdict::Changed {
                regions: vec![region],
            },
            &hints,
            &rules,
            (100, 100),
        )
        .expect("no mask involved");
        assert_eq!(outcome.regions, vec![RegionOutcome::Violation]);
    }
}
