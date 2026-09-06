//! The verdict shape. `chrys-core` reports one of three outcomes for a
//! pair, and this module is the only place that defines what those
//! outcomes look like.

use std::fmt;

/// A rectangle of changed pixels, in pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
    /// The left edge of the box, in pixels.
    pub x: u32,
    /// The top edge of the box, in pixels.
    pub y: u32,
    /// The width of the box, in pixels.
    pub width: u32,
    /// The height of the box, in pixels.
    pub height: u32,
}

impl fmt::Display for BoundingBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "x={}, y={}, width={}, height={}",
            self.x, self.y, self.width, self.height
        )
    }
}

/// The colour difference between a base pixel and a candidate pixel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColourDelta {
    /// The distance between the two colours. Phase 1 reports the
    /// Euclidean distance in straight RGB as a stand-in metric; plan
    /// 01-07 routes this through `palette`'s Lab colour space instead.
    pub delta_e: f32,
    /// The base colour, RGBA8.
    pub base: [u8; 4],
    /// The candidate colour, RGBA8.
    pub candidate: [u8; 4],
}

/// The kind of change a region carries.
///
/// This list is a reviewable decision, not a black box: a region is
/// classified by an ordered set of named rules, and every rule maps to one
/// of these five variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Content moved from one place to another.
    Moved,
    /// Content is present in the candidate and absent in the base.
    Added,
    /// Content is present in the base and absent in the candidate.
    Removed,
    /// Content stayed in place but its colour changed.
    Recoloured,
    /// Content changed size.
    Resized,
}

/// One labelled region of change.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    /// The kind of change this region carries.
    pub kind: ChangeKind,
    /// The bounding box of the changed pixels.
    pub bbox: BoundingBox,
    /// The pixel offset this region moved by, when the kind is `Moved`.
    pub offset_px: Option<(i32, i32)>,
    /// The colour change this region carries, when the kind is
    /// `Recoloured`.
    pub colour_delta: Option<ColourDelta>,
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} region at {}", self.kind, self.bbox)?;
        if let Some(delta) = &self.colour_delta {
            write!(
                f,
                ", colour delta {:.2} (base {:?}, candidate {:?})",
                delta.delta_e, delta.base, delta.candidate
            )?;
        }
        Ok(())
    }
}

/// The reason `compare` refused to produce a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    /// The base and candidate frames have different dimensions. `compare`
    /// assumes near-identical pairs and does not scale or crop to make
    /// them match.
    DimensionMismatch {
        /// The base frame's width and height, in pixels.
        base: (u32, u32),
        /// The candidate frame's width and height, in pixels.
        candidate: (u32, u32),
    },
}

impl fmt::Display for RefusalReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefusalReason::DimensionMismatch { base, candidate } => write!(
                f,
                "dimension mismatch: base is {}x{}, candidate is {}x{}",
                base.0, base.1, candidate.0, candidate.1
            ),
        }
    }
}

/// The outcome of comparing a base frame to a candidate frame.
///
/// A verdict is one of exactly three shapes. There is no fourth meaning of
/// "changed" anywhere in this project: every stage that produces a verdict
/// produces one of these three.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// The base and candidate frames are pixel-identical.
    Identical,
    /// The base and candidate frames differ. `regions` names every
    /// changed area and its kind.
    Changed {
        /// The changed regions, in the order they were found.
        regions: Vec<Region>,
    },
    /// `compare` could not produce a verdict for this pair.
    Refused {
        /// Why the pair was refused.
        reason: RefusalReason,
    },
}

impl Verdict {
    /// Return true when this verdict reports a change. A refusal is not a
    /// change: it is a distinct outcome, so a caller cannot read it as a
    /// pass.
    pub fn is_change(&self) -> bool {
        matches!(self, Verdict::Changed { .. })
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::Identical => writeln!(f, "identical"),
            Verdict::Changed { regions } => {
                for region in regions {
                    writeln!(f, "{region}")?;
                }
                Ok(())
            }
            Verdict::Refused { reason } => writeln!(f, "refused: {reason}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_change_is_true_only_for_changed() {
        assert!(!Verdict::Identical.is_change());
        assert!(Verdict::Changed { regions: vec![] }.is_change());
        assert!(
            !Verdict::Refused {
                reason: RefusalReason::DimensionMismatch {
                    base: (1, 1),
                    candidate: (2, 2),
                },
            }
            .is_change()
        );
    }

    #[test]
    fn display_never_prints_a_bare_pixel_count() {
        let region = Region {
            kind: ChangeKind::Recoloured,
            bbox: BoundingBox {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            offset_px: None,
            colour_delta: None,
        };
        let text = format!(
            "{}",
            Verdict::Changed {
                regions: vec![region]
            }
        );
        assert!(text.contains("Recoloured"));
        assert!(!text.trim().chars().all(|c| c.is_ascii_digit()));
    }
}
