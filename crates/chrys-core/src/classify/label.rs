//! Group residual pixels into labelled, bounded regions.
//!
//! This module owns a two-pass, union-find connected-component labelling
//! algorithm (Wu, Otoo and Suzuki, "Two Strategies to Speed Up Connected
//! Component Labeling Algorithms," 2009, cited by
//! `.planning/research/ARCHITECTURE.md`). This module used to call
//! `imageproc::region_labelling::connected_components` instead. That call
//! reached the `image` crate transitively, through `imageproc`'s own
//! re-export, and `.planning/research/ARCHITECTURE.md` states that
//! `chrys-core` may not import a format crate: the "no per-format special
//! case" rule is meant to be a compile-time fact, checked by the
//! dependency guard in `tests/determinism.rs`, not a rule a person has to
//! remember. A transitive `image` dependency defeated that fact.
//!
//! This module also settles a determinism risk this file used to carry as
//! an open comment: a version bump inside `imageproc` could reassign
//! labels in a different internal order, and this project's own verdict
//! must not change just because a dependency changed internally. Owning
//! the algorithm here makes the label order this project's own documented
//! decision, stated next, and testable by this project's own tests
//! instead of a dependency's changelog.
//!
//! **Label assignment order.** Pixels are visited in raster order: row by
//! row, top to bottom, and left to right within a row. A foreground pixel
//! that touches no already-visited, already-labelled neighbour (checking
//! the four 8-connected neighbours behind it in scan order: up-left, up,
//! up-right, left) opens a new label, one greater than the last label
//! opened. A pixel that touches one or more labelled neighbours takes the
//! smallest of their labels, and the union-find structure records that
//! every neighbour label found at that pixel names the same region,
//! always keeping the smallest label of a group as that group's root.
//! This is the conventional order for a two-pass raster labeller, and it
//! is deterministic by construction: two runs over the same residual
//! field visit pixels in the same order and open new labels in the same
//! order, on any platform. The final `Vec<LabelledRegion>` this module
//! returns is, in addition, re-sorted by bounding box position before it
//! is handed back (see `label_regions`), so no caller needs to depend on
//! the raw label numbers this scan assigns.

use std::collections::BTreeMap;

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
    /// The label this module's own union-find scan assigned this region,
    /// before this module's own sort re-orders the list. This is not a
    /// stable identity across runs; it exists for debugging only.
    pub label: u32,
}

/// A disjoint-set (union-find) structure over the provisional labels this
/// module's raster scan opens, with path compression on every lookup.
///
/// Labels are one-indexed; label `n` is stored at `parent[n - 1]`. Label
/// `0` is reserved, throughout this module, to mean "background."
struct UnionFind {
    parent: Vec<u32>,
}

impl UnionFind {
    fn new() -> Self {
        UnionFind { parent: Vec::new() }
    }

    /// Open a new label, one greater than the last label opened, and
    /// return it.
    fn make_label(&mut self) -> u32 {
        let label = self.parent.len() as u32 + 1;
        self.parent.push(label);
        label
    }

    /// Find `label`'s root, compressing every visited link to point at
    /// the root directly so later lookups on the same chain are shorter.
    fn find(&mut self, label: u32) -> u32 {
        let mut root = label;
        while self.parent[(root - 1) as usize] != root {
            root = self.parent[(root - 1) as usize];
        }
        let mut current = label;
        while current != root {
            let next = self.parent[(current - 1) as usize];
            self.parent[(current - 1) as usize] = root;
            current = next;
        }
        root
    }

    /// Record that `a` and `b` name the same region. The smaller of the
    /// two roots always becomes the surviving root, so a group's final
    /// identity is always its smallest member's label, independent of the
    /// order this function is called in.
    fn union(&mut self, a: u32, b: u32) {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a == root_b {
            return;
        }
        if root_a < root_b {
            self.parent[(root_b - 1) as usize] = root_a;
        } else {
            self.parent[(root_a - 1) as usize] = root_b;
        }
    }
}

/// Return the label at `(x, y)` when that pixel is inside the field and is
/// foreground and already labelled; `None` otherwise, including when the
/// coordinate falls outside the field.
fn labelled_neighbour(
    x: isize,
    y: isize,
    width: usize,
    height: usize,
    is_foreground: &[bool],
    labels: &[u32],
) -> Option<u32> {
    if x < 0 || y < 0 {
        return None;
    }
    let (x, y) = (x as usize, y as usize);
    if x >= width || y >= height {
        return None;
    }
    let idx = y * width + x;
    if is_foreground[idx] && labels[idx] != 0 {
        Some(labels[idx])
    } else {
        None
    }
}

/// Group `residual`'s changed pixels into labelled regions with bounding
/// boxes.
///
/// A pixel is a change when the largest of its four residual bytes exceeds
/// `RESIDUAL_THRESHOLD`. Alpha is part of what "changed" means (see
/// PROJECT.md's Key Decisions), so the fourth byte is read on the same
/// footing as the other three, not dropped. Two regions touching only at a
/// corner are one region, because connectivity is eight-way.
///
/// The returned list is sorted by bounding box top edge, then left edge,
/// then label, never by this module's own raw scan-assigned label order.
/// See this module's own doc comment for what that scan order is and why
/// a caller must not depend on it.
pub fn label_regions(residual: &ResidualField) -> Vec<LabelledRegion> {
    let width = residual.width;
    let height = residual.height;

    let mut is_foreground = vec![false; width * height];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            let r = residual.samples[idx];
            let g = residual.samples[idx + 1];
            let b = residual.samples[idx + 2];
            let a = residual.samples[idx + 3];
            let magnitude = r.max(g).max(b).max(a);
            is_foreground[y * width + x] = magnitude > RESIDUAL_THRESHOLD;
        }
    }

    let mut labels = vec![0u32; width * height];
    let mut union_find = UnionFind::new();

    // First pass: raster scan, eight-way connectivity, checking the four
    // neighbours this scan order has already visited (up-left, up,
    // up-right, left).
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if !is_foreground[idx] {
                continue;
            }
            let xi = x as isize;
            let yi = y as isize;
            let found: Vec<u32> = [
                labelled_neighbour(xi - 1, yi - 1, width, height, &is_foreground, &labels),
                labelled_neighbour(xi, yi - 1, width, height, &is_foreground, &labels),
                labelled_neighbour(xi + 1, yi - 1, width, height, &is_foreground, &labels),
                labelled_neighbour(xi - 1, yi, width, height, &is_foreground, &labels),
            ]
            .into_iter()
            .flatten()
            .collect();

            if found.is_empty() {
                labels[idx] = union_find.make_label();
            } else {
                let min_label = *found.iter().min().expect("found is non-empty");
                labels[idx] = min_label;
                for &neighbour_label in &found {
                    union_find.union(min_label, neighbour_label);
                }
            }
        }
    }

    // Second pass: resolve every provisional label to its union-find
    // root, and fold each pixel into that root's bounding box and pixel
    // count. A `BTreeMap`, not a `HashMap`, keeps this fold's own
    // iteration order fixed across platforms, matching this project's
    // wider rule for any structure whose order could otherwise vary.
    let mut by_root: BTreeMap<u32, (u32, u32, u32, u32, usize)> = BTreeMap::new();
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let label = labels[idx];
            if label == 0 {
                continue;
            }
            let root = union_find.find(label);
            let (x, y) = (x as u32, y as u32);
            let entry = by_root.entry(root).or_insert((x, y, x, y, 0));
            entry.0 = entry.0.min(x);
            entry.1 = entry.1.min(y);
            entry.2 = entry.2.max(x);
            entry.3 = entry.3.max(y);
            entry.4 += 1;
        }
    }

    let mut regions: Vec<LabelledRegion> = by_root
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

    // Sorting by position, not by this module's own raw scan-assigned
    // label order, is the point: the label a pixel happens to receive
    // during the first pass is an implementation detail of this scan, and
    // this project's own verdict must not reorder its regions based on
    // it.
    regions.sort_unstable_by_key(|region| (region.bbox.y, region.bbox.x, region.label));
    regions
}
