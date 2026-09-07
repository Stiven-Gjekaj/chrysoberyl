//! The TOML report artifact `chrys compare --report <PATH>` writes.
//!
//! This module and its structs live in `chrys-cli` and nowhere else:
//! phase 1's own responsibility map assigned the file-writing form of a
//! verdict to the CLI, and this phase fills that assignment in. `toml`
//! and `serde` are added to this crate's own `Cargo.toml` from the
//! workspace table, where both are already pinned and already audited;
//! neither crosses into `chrys-core`, `chrys-source` or `chrys-rule`
//! through this module.
//!
//! The document holds a `meta` table, naming the base path, the candidate
//! path, and, when `--rule` was given, the rule file's own path. Then an
//! array of tables named `frame`, one per compared index, each carrying
//! `index`, `source` (the base side's own file name from
//! `Source::load_named`) and `verdict`. A refused frame carries `reason`
//! instead of any `change` table. A changed frame carries an array of
//! tables named `change`, one per region, each naming its `kind`,
//! bounding box, `size` (width times height, in pixels), the `region` it
//! fell inside when one applies, a `moved` change's own pixel offset, a
//! `recoloured` change's own colour difference (including the alpha term
//! taken from the two colours' own fourth bytes, per
//! `chrys_core::ColourDelta`'s own doc comment), and, when `--rule` was
//! given, that change's own `rule_outcome`.

use std::path::Path;

use anyhow::Context;
use serde::Serialize;

use chrys_core::{ChangeKind, Region, Verdict};
use chrys_rule::RegionOutcome;
use chrys_source::RegionHint;

/// The whole report document.
#[derive(Debug, Serialize)]
pub struct Report {
    pub meta: Meta,
    pub frame: Vec<FrameReport>,
}

/// The `meta` table: the paths this run compared, and the rule file it
/// gated on, when one was given.
#[derive(Debug, Serialize)]
pub struct Meta {
    pub base: String,
    pub candidate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
}

/// One `[[frame]]` table: one compared index.
#[derive(Debug, Serialize)]
pub struct FrameReport {
    /// The frame's own index inside the compared sequence.
    pub index: usize,
    /// The base side's own file name, from `Source::load_named`. Never
    /// only this frame's index: success criterion 6 exists because an
    /// index alone does not name the file a producer wrote.
    pub source: String,
    /// One of `identical`, `changed` or `refused`.
    pub verdict: &'static str,
    /// Why this frame was refused. Present only when `verdict` is
    /// `refused`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// One entry per changed region. Empty for `identical` and for
    /// `refused`: a refusal is not a change, and this array never
    /// invents one to fill the gap.
    #[serde(rename = "change", skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<ChangeReport>,
}

/// One `[[frame.change]]` table: one changed region.
#[derive(Debug, Serialize)]
pub struct ChangeReport {
    /// One of `moved`, `added`, `removed`, `recoloured` or `resized`.
    pub kind: &'static str,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// `width` times `height`, in pixels.
    pub size: u64,
    /// The named region this change's box fell inside, when one covers
    /// at least half its own area (the same majority rule
    /// `chrys_rule::overlapping_hint_name` already uses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// The pixel offset this region moved by. Present only for a `moved`
    /// change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_y: Option<i32>,
    /// The Lab colour distance. Present only for a `recoloured` change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_e: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_colour: Option<[u8; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_colour: Option<[u8; 4]>,
    /// The difference between the two colours' own fourth bytes.
    /// `delta_e` carries no alpha axis (`chrys_core::ColourDelta`'s own
    /// doc comment), so this is read from the two colours directly, not
    /// derived from `delta_e`. Present only for a `recoloured` change
    /// whose alpha difference is not zero, matching
    /// `chrys_core::verdict::Region`'s own `Display` impl.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha_delta: Option<u8>,
    /// One of `tolerated` or `violation`. Absent entirely, not present
    /// with a default value, when `--rule` was not given: an absent key
    /// says "nothing asked this question", and a default value would say
    /// "something asked and the answer was no", which is a different
    /// fact a reader must not be made to guess at.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_outcome: Option<&'static str>,
}

/// The name a report gives `kind`. Lower case, matching this crate's own
/// `--rule` file schema, so a reader who has already read a rule file
/// recognises the word.
fn kind_str(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Moved => "moved",
        ChangeKind::Added => "added",
        ChangeKind::Removed => "removed",
        ChangeKind::Recoloured => "recoloured",
        ChangeKind::Resized => "resized",
    }
}

/// The name a report gives a `RegionOutcome`.
fn rule_outcome_str(outcome: &RegionOutcome) -> &'static str {
    match outcome {
        RegionOutcome::Tolerated { .. } => "tolerated",
        RegionOutcome::Violation => "violation",
    }
}

/// Build one `[[frame.change]]` table for `region`, resolving its own
/// named region against `hints` and, when `rule_outcome` is `Some`,
/// carrying that outcome.
pub fn change_report(
    region: &Region,
    hints: &[RegionHint],
    rule_outcome: Option<&RegionOutcome>,
) -> ChangeReport {
    let bbox = region.bbox;
    let region_name = chrys_rule::overlapping_hint_name(&bbox, hints).map(|name| name.to_string());

    let (offset_x, offset_y) = match region.offset_px {
        Some((dx, dy)) => (Some(dx), Some(dy)),
        None => (None, None),
    };

    let (delta_e, base_colour, candidate_colour, alpha_delta) = match &region.colour_delta {
        Some(delta) => {
            let alpha_delta = delta.base[3].abs_diff(delta.candidate[3]);
            (
                Some(delta.delta_e),
                Some(delta.base),
                Some(delta.candidate),
                if alpha_delta != 0 {
                    Some(alpha_delta)
                } else {
                    None
                },
            )
        }
        None => (None, None, None, None),
    };

    ChangeReport {
        kind: kind_str(region.kind),
        x: bbox.x,
        y: bbox.y,
        width: bbox.width,
        height: bbox.height,
        size: u64::from(bbox.width) * u64::from(bbox.height),
        region: region_name,
        offset_x,
        offset_y,
        delta_e,
        base_colour,
        candidate_colour,
        alpha_delta,
        rule_outcome: rule_outcome.map(rule_outcome_str),
    }
}

/// Build one `[[frame]]` table for one compared index.
///
/// `hints` is the same frame's own base-side hints, used to resolve a
/// changed region's named region. `outcomes` is `Some` only when
/// `--rule` was given, and, when present, carries one `RegionOutcome`
/// per region of `verdict`, in the same order.
pub fn frame_report(
    index: usize,
    source: String,
    verdict: &Verdict,
    hints: &[RegionHint],
    outcomes: Option<&[RegionOutcome]>,
) -> FrameReport {
    match verdict {
        Verdict::Identical => FrameReport {
            index,
            source,
            verdict: "identical",
            reason: None,
            changes: Vec::new(),
        },
        Verdict::Changed { regions } => {
            let changes = regions
                .iter()
                .enumerate()
                .map(|(region_index, region)| {
                    let outcome = outcomes.and_then(|outcomes| outcomes.get(region_index));
                    change_report(region, hints, outcome)
                })
                .collect();
            FrameReport {
                index,
                source,
                verdict: "changed",
                reason: None,
                changes,
            }
        }
        Verdict::Refused { reason } => FrameReport {
            index,
            source,
            verdict: "refused",
            reason: Some(reason.to_string()),
            changes: Vec::new(),
        },
    }
}

/// Serialize `report` as TOML and write it to `path`.
///
/// A failure to serialize or to write is returned rather than swallowed:
/// a report a caller asked for and did not get is a silent hole in a CI
/// job's own evidence, and this project's own rule is to refuse rather
/// than to guess. Both failure messages name `path`.
pub fn write_report(path: &Path, report: &Report) -> anyhow::Result<()> {
    let text = toml::to_string_pretty(report)
        .with_context(|| format!("failed to serialize the report for {}", path.display()))?;
    std::fs::write(path, text)
        .with_context(|| format!("failed to write the report to {}", path.display()))?;
    Ok(())
}
