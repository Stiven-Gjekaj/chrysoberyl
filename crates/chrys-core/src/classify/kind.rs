//! Name a region by kind, using an ordered, reviewable decision list.
//!
//! A person can read this list top to bottom and predict the answer for
//! any region: there is no black-box classifier deciding a verdict here,
//! only five named rules, each guarded by a named constant.

use std::collections::BTreeMap;

use chrys_source::Frame;

use crate::classify::colour::colour_delta;
use crate::classify::label::LabelledRegion;
use crate::register::block_match::{BLOCK_SIDE, BlockOffset};
use crate::verdict::{BoundingBox, ChangeKind, ColourDelta};

/// The fraction of a region's own bounding box that must equal the
/// frame's modal colour for that region to count as background in that
/// frame.
pub const BACKGROUND_FRACTION: f32 = 0.9;

/// The fraction by which a region's non-background pixel count may
/// differ between frames before this rule calls it `Resized` rather than
/// `Recoloured`.
pub const RESIZE_FRACTION: f32 = 0.2;

/// Name `region`'s kind, and return its offset in pixels when the kind is
/// `Moved`, and its colour delta when the kind is `Recoloured`.
///
/// This function computes the frame's own background colour once, as the
/// modal RGBA value over `base`, and reuses that one value to decide
/// whether `region`'s own pixels count as background in either frame. The
/// ordered decision list:
///
/// One, the region is background in `base` and not in `candidate`, so it
/// is `Added`. Two, the reverse, so it is `Removed`. Three, both frames
/// hold content, but the non-background pixel count inside the bounding
/// box differs by more than `RESIZE_FRACTION` while the majority block
/// offset is zero, so it is `Resized`. Four, the majority block offset
/// over the blocks intersecting the bounding box is non-zero, so it is
/// `Moved`, and that offset is the reported offset in pixels. Five, the
/// fall-through is `Recoloured`: a region that reached the residual and
/// matches none of the first four differs in colour by definition, so
/// `Recoloured` is the honest default here rather than an unnamed sixth
/// category. Its colour delta is the difference between the mean colour
/// of the region's own bounding box in each frame; `LabelledRegion` does
/// not carry the region's own pixel mask, so the bounding box is the
/// closest data this function has to "the region."
pub fn classify_kind(
    region: &LabelledRegion,
    base: &Frame,
    candidate: &Frame,
    blocks: &[BlockOffset],
) -> (ChangeKind, Option<(i32, i32)>, Option<ColourDelta>) {
    let background = frame_background(base);
    let bbox = region.bbox;

    let base_is_background = is_region_background(base, &bbox, background);
    let candidate_is_background = is_region_background(candidate, &bbox, background);

    if base_is_background && !candidate_is_background {
        return (ChangeKind::Added, None, None);
    }
    if !base_is_background && candidate_is_background {
        return (ChangeKind::Removed, None, None);
    }

    let offset = majority_block_offset(blocks, &bbox);
    if offset != (0, 0) {
        return (ChangeKind::Moved, Some(offset), None);
    }

    let base_count = non_background_count(base, &bbox, background);
    let candidate_count = non_background_count(candidate, &bbox, background);
    let larger = base_count.max(candidate_count).max(1) as f32;
    let difference = base_count.abs_diff(candidate_count) as f32;
    if difference / larger > RESIZE_FRACTION {
        return (ChangeKind::Resized, None, None);
    }

    let delta = colour_delta(mean_colour(base, &bbox), mean_colour(candidate, &bbox));
    (ChangeKind::Recoloured, None, Some(delta))
}

/// The modal RGBA colour over every pixel in `frame`.
///
/// The histogram is a `BTreeMap`, not a `HashMap`: a `HashMap`'s
/// iteration order is not fixed across platforms, so a tie between two
/// colours with the same pixel count could pick a different winner on a
/// different machine. `BTreeMap` orders its keys, and `max_by_key`
/// returns the last of several equally-maximum elements, so a tie always
/// favours the numerically larger RGBA value, on every platform.
fn frame_background(frame: &Frame) -> [u8; 4] {
    let mut counts: BTreeMap<[u8; 4], usize> = BTreeMap::new();
    for chunk in frame.rgba8().chunks_exact(4) {
        let colour = [chunk[0], chunk[1], chunk[2], chunk[3]];
        *counts.entry(colour).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(colour, _)| colour)
        .unwrap_or([0, 0, 0, 0])
}

/// Return true when at least `BACKGROUND_FRACTION` of `bbox`'s pixels in
/// `frame` equal `background`.
fn is_region_background(frame: &Frame, bbox: &BoundingBox, background: [u8; 4]) -> bool {
    let (matches, total) = count_matches(frame, bbox, background);
    if total == 0 {
        return false;
    }
    (matches as f32 / total as f32) >= BACKGROUND_FRACTION
}

/// The number of pixels inside `bbox`, in `frame`, that do not equal
/// `background`.
fn non_background_count(frame: &Frame, bbox: &BoundingBox, background: [u8; 4]) -> usize {
    let (matches, total) = count_matches(frame, bbox, background);
    total - matches
}

fn count_matches(frame: &Frame, bbox: &BoundingBox, colour: [u8; 4]) -> (usize, usize) {
    let pixels = frame.rgba8();
    let width = frame.width;
    let mut matches = 0usize;
    let mut total = 0usize;
    for y in bbox.y..bbox.y + bbox.height {
        for x in bbox.x..bbox.x + bbox.width {
            let idx = ((y * width + x) * 4) as usize;
            let pixel = [
                pixels[idx],
                pixels[idx + 1],
                pixels[idx + 2],
                pixels[idx + 3],
            ];
            if pixel == colour {
                matches += 1;
            }
            total += 1;
        }
    }
    (matches, total)
}

/// The mean RGBA colour over every pixel inside `bbox`, in `frame`.
fn mean_colour(frame: &Frame, bbox: &BoundingBox) -> [u8; 4] {
    let pixels = frame.rgba8();
    let width = frame.width;
    let mut sums = [0u64; 4];
    let mut total = 0u64;
    for y in bbox.y..bbox.y + bbox.height {
        for x in bbox.x..bbox.x + bbox.width {
            let idx = ((y * width + x) * 4) as usize;
            for (channel, sum) in sums.iter_mut().enumerate() {
                *sum += u64::from(pixels[idx + channel]);
            }
            total += 1;
        }
    }
    let total = total.max(1);
    [
        (sums[0] / total) as u8,
        (sums[1] / total) as u8,
        (sums[2] / total) as u8,
        (sums[3] / total) as u8,
    ]
}

/// The majority `(dx, dy)` offset, by count, over every block in `blocks`
/// whose own pixel rectangle intersects `bbox`. Returns `(0, 0)` when no
/// block intersects `bbox`. A tie between two offsets favours the smaller
/// magnitude, matching `block_match`'s own tie-break convention; a
/// remaining tie favours the lexicographically smaller offset, since
/// `BTreeMap` iterates its keys in a fixed order.
fn majority_block_offset(blocks: &[BlockOffset], bbox: &BoundingBox) -> (i32, i32) {
    let block_side = BLOCK_SIDE as u32;
    let bbox_x1 = bbox.x + bbox.width;
    let bbox_y1 = bbox.y + bbox.height;

    let mut counts: BTreeMap<(i32, i32), usize> = BTreeMap::new();
    for block in blocks {
        let block_x0 = block.block_x as u32 * block_side;
        let block_y0 = block.block_y as u32 * block_side;
        let block_x1 = block_x0 + block_side;
        let block_y1 = block_y0 + block_side;
        let intersects =
            block_x0 < bbox_x1 && block_x1 > bbox.x && block_y0 < bbox_y1 && block_y1 > bbox.y;
        if intersects {
            *counts.entry((block.dx, block.dy)).or_insert(0) += 1;
        }
    }

    let Some(max_count) = counts.values().copied().max() else {
        return (0, 0);
    };

    counts
        .into_iter()
        .filter(|(_, count)| *count == max_count)
        .min_by_key(|((dx, dy), _)| {
            i64::from(*dx) * i64::from(*dx) + i64::from(*dy) * i64::from(*dy)
        })
        .map(|(offset, _)| offset)
        .unwrap_or((0, 0))
}
