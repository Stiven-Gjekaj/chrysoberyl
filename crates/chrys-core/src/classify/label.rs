//! Group residual pixels into labelled, bounded regions.
//!
//! `imageproc::region_labelling::connected_components` implements the
//! standard two-pass union-find algorithm (Wu, Otoo and Suzuki, "Two
//! Strategies to Speed Up Connected Component Labeling Algorithms," 2009).
//! This module does not hand-roll that algorithm; it builds the
//! thresholded image the crate's function needs, and turns its label image
//! back into bounding boxes this project's own callers can read.

use std::collections::HashMap;

use imageproc::image::{GrayImage, Luma};
use imageproc::region_labelling::{Connectivity, connected_components};

use crate::register::block_match::ResidualField;
use crate::verdict::BoundingBox;

/// The smallest residual byte value this engine treats as a change.
///
/// A residual sample at or below this value is background, not a change,
/// and never joins a region. This is a determinism input like every other
/// constant in the comparison path: two runs must agree on where the line
/// between noise and a real change sits.
pub const RESIDUAL_THRESHOLD: u8 = 4;

/// One connected region of changed pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelledRegion {
    /// The smallest rectangle holding every pixel of this region.
    pub bbox: BoundingBox,
    /// The number of changed pixels inside this region.
    pub pixel_count: usize,
    /// The label `connected_components` assigned this region, before this
    /// module's own sort re-orders the list. This is not a stable identity
    /// across runs; it exists for debugging only.
    pub label: u32,
}

/// Group `residual`'s changed pixels into labelled regions with bounding
/// boxes.
///
/// A pixel is a change when the largest of its three residual colour
/// channels exceeds `RESIDUAL_THRESHOLD`. Two regions touching only at a
/// corner are one region, because connectivity is eight-way.
///
/// The returned list is sorted by bounding box top edge, then left edge,
/// then label, never by `connected_components`'s own internal label order.
/// The labelling order is an implementation detail of the library, and a
/// verdict that depends on it is a verdict that can reorder under a
/// version bump.
pub fn label_regions(residual: &ResidualField) -> Vec<LabelledRegion> {
    let width = residual.width as u32;
    let height = residual.height as u32;

    let mut thresholded = GrayImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let r = residual.samples[idx];
            let g = residual.samples[idx + 1];
            let b = residual.samples[idx + 2];
            let magnitude = r.max(g).max(b);
            let value = if magnitude > RESIDUAL_THRESHOLD {
                255
            } else {
                0
            };
            thresholded.put_pixel(x, y, Luma([value]));
        }
    }

    let labels = connected_components(&thresholded, Connectivity::Eight, Luma([0u8]));

    // (min_x, min_y, max_x, max_y, pixel_count), keyed by the library's own
    // label. Folding every pixel once builds every region's bounding box
    // and pixel count in one pass.
    let mut by_label: HashMap<u32, (u32, u32, u32, u32, usize)> = HashMap::new();
    for y in 0..height {
        for x in 0..width {
            let label = labels.get_pixel(x, y).0[0];
            if label == 0 {
                continue;
            }
            let entry = by_label.entry(label).or_insert((x, y, x, y, 0));
            entry.0 = entry.0.min(x);
            entry.1 = entry.1.min(y);
            entry.2 = entry.2.max(x);
            entry.3 = entry.3.max(y);
            entry.4 += 1;
        }
    }

    let mut regions: Vec<LabelledRegion> = by_label
        .into_iter()
        .map(
            |(label, (min_x, min_y, max_x, max_y, pixel_count))| LabelledRegion {
                bbox: BoundingBox {
                    x: min_x,
                    y: min_y,
                    width: max_x - min_x + 1,
                    height: max_y - min_y + 1,
                },
                pixel_count,
                label,
            },
        )
        .collect();

    // Sorting by position, not by the library's own label order, is the
    // point: a version bump inside `imageproc` may assign labels in a
    // different scan order, and this project's own verdict must not
    // reorder its regions just because a dependency changed internally.
    regions.sort_unstable_by_key(|region| (region.bbox.y, region.bbox.x, region.label));
    regions
}
