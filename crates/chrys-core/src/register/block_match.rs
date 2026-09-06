//! The local registration step: a coarse-to-fine block match over the
//! candidate, warped by the global offset, against the base.
//!
//! This borrows two motion-estimation practices from the x264 reference
//! encoder (`doc/motion_est.txt`; see also Zhu and Ma, "A New Diamond
//! Search Algorithm for Fast Block-Matching Motion Estimation," IEEE
//! Trans. Image Processing, 2000, and Tourapis, "Enhanced Predictive Zonal
//! Search," VCIP, 2002): a spatial median predictor for a block's starting
//! point, taken from the already-computed offsets of its left, top and
//! top-right neighbours at the same pyramid level, since motion is usually
//! spatially coherent; and early termination the moment a candidate scores
//! zero, since no later candidate can beat a perfect match. The search
//! shape stays a full window scan at this block size, rather than x264's
//! diamond or hexagon pattern; see the plan 01-06 summary for the reason.
//!
//! Every score in this module comes from one `IntegralImage` built once
//! per candidate offset over the whole level image, answered by every
//! block through `window_sum`. Nothing here loops pixel by pixel inside a
//! block for a candidate; that is the anti-pattern summed-area tables
//! exist to remove.

use std::collections::HashMap;

use chrys_source::Frame;

use crate::CompareError;
use crate::register::integral::IntegralImage;
use crate::register::phase_correlation::CoarseOffset;
use crate::register::warp::{FILL_VALUE, difference_image, warp_by_offset};

/// The side length, in pixels, of one block at the finest pyramid level.
///
/// A block search over a region this size finds a translated area's own
/// offset, separately from the whole-frame shift the global stage already
/// removed. This is a determinism input, not a performance knob: two
/// machines must divide the frame into the same blocks to report the same
/// per-block offsets.
pub const BLOCK_SIDE: usize = 32;

/// The number of pyramid levels the coarse-to-fine search builds, coarsest
/// to finest. Level `PYRAMID_LEVELS - 1` is the coarsest; level `0` is the
/// finest, at full resolution.
///
/// Two levels give the search a starting point at half resolution before
/// it ever evaluates a candidate at full resolution, while keeping the
/// pyramid itself cheap to build: each level is one two-by-two integer box
/// reduction of the level below it.
///
/// This count and `SEARCH_HALF_WIDTH` together bound the largest
/// full-resolution offset the whole hierarchy can ever report, at
/// `(2 ^ PYRAMID_LEVELS - 1) * SEARCH_HALF_WIDTH` (each finer level's
/// predicted centre is the coarser level's own winner, doubled, plus that
/// level's own search can add `SEARCH_HALF_WIDTH` more). That bound must
/// stay comfortably below `BLOCK_SIDE`: a bound at or above it lets an
/// entirely flat block (for example, a solid-coloured recoloured
/// rectangle sized to whole blocks) "escape" its own area and alias
/// against an unrelated, coincidentally identical region elsewhere,
/// scoring a false perfect match instead of reporting that it did not
/// move. With two levels and a half-width of eight, the bound is
/// `(2^2 - 1) * 8 = 24`, eight pixels below `BLOCK_SIDE`'s 32.
pub const PYRAMID_LEVELS: usize = 2;

/// The half-width, in that level's own pixel units, of the window a
/// block's search scans around its predicted centre.
///
/// This constant is the same count at every level; the coarse-to-fine
/// narrowing comes from the geometry, not from a shrinking count: the same
/// window at a coarse level's reduced resolution spans a much larger
/// full-resolution footprint than the same window does at the finest
/// level. A block's search never evaluates a candidate outside
/// `centre - SEARCH_HALF_WIDTH ..= centre + SEARCH_HALF_WIDTH` on either
/// axis, at the level it is currently run at. See `PYRAMID_LEVELS`'s own
/// doc comment for the bound this constant and the level count together
/// place on the largest full-resolution offset the search can report.
pub const SEARCH_HALF_WIDTH: i32 = 8;

/// A block's own translation, found by the local search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockOffset {
    /// The block's column index in the fixed `BLOCK_SIDE` grid.
    pub block_x: usize,
    /// The block's row index in the fixed `BLOCK_SIDE` grid.
    pub block_y: usize,
    /// The block's own horizontal offset, in pixels, on top of the global
    /// offset already removed.
    pub dx: i32,
    /// The block's own vertical offset, in pixels, on top of the global
    /// offset already removed.
    pub dy: i32,
    /// The sum of absolute RGB differences over the block at `(dx, dy)`.
    pub score: u64,
}

/// The output of the local registration step: the per-block offsets, and
/// the residual RGBA8 image, the difference that remains once both the
/// global and each block's own local shift are removed.
#[derive(Debug, Clone, PartialEq)]
pub struct ResidualField {
    /// The field's width, in pixels. Equal to the compared frames' width.
    pub width: usize,
    /// The field's height, in pixels. Equal to the compared frames'
    /// height.
    pub height: usize,
    /// The residual RGBA8 buffer, row major, no row padding, alpha always
    /// `u8::MAX`.
    pub samples: Vec<u8>,
    /// Every block's own offset, in row-major block order.
    pub blocks: Vec<BlockOffset>,
}

/// Register `candidate` against `base` at block granularity, given the
/// whole-frame offset `coarse` the global stage already found.
///
/// The steps: warp `candidate` by `coarse` so both frames start aligned;
/// build a luma pyramid of `base` and of the aligned candidate, coarsest
/// to finest, by repeated two-by-two integer box reduction; at the
/// coarsest level, search every block from a predicted centre of zero (the
/// global offset already accounts for the frame's own shift); carry each
/// level's winning offset down, doubled, as the next, finer level's
/// predicted centre; and at the finest level, record each block's winning
/// offset and render the residual RGBA8 difference at that offset into
/// this block's own pixels.
///
/// Returns `CompareError::ShapeMismatch` when the frames differ in size.
pub fn block_match(
    base: &Frame,
    candidate: &Frame,
    coarse: CoarseOffset,
) -> Result<ResidualField, CompareError> {
    if !base.same_shape_as(candidate) {
        return Err(CompareError::ShapeMismatch {
            base: (base.width, base.height),
            candidate: (candidate.width, candidate.height),
        });
    }

    let width = base.width as usize;
    let height = base.height as usize;

    // Align both frames to the same origin before any local search begins.
    // warp_by_offset(frame, dx, dy) reads output(x, y) from input
    // (x - dx, y - dy); to undo `coarse` (candidate content sits `coarse`
    // pixels from the base's), warp the candidate by the negated offset.
    let aligned_pixels = warp_by_offset(candidate, -coarse.dx, -coarse.dy);
    let aligned_frame = Frame {
        pixels: aligned_pixels,
        width: base.width,
        height: base.height,
        index: 0,
        hints: Vec::new(),
    };

    let base_luma = luma_bytes(base.rgba8());
    let candidate_luma = luma_bytes(aligned_frame.rgba8());

    let mut base_levels: Vec<(Vec<u8>, usize, usize)> = vec![(base_luma, width, height)];
    let mut candidate_levels: Vec<(Vec<u8>, usize, usize)> = vec![(candidate_luma, width, height)];
    for _ in 1..PYRAMID_LEVELS {
        let (prev_base, pw, ph) = base_levels.last().expect("at least one level");
        let reduced_base = box_reduce(prev_base, *pw, *ph);
        base_levels.push(reduced_base);
        let (prev_candidate, pw2, ph2) = candidate_levels.last().expect("at least one level");
        let reduced_candidate = box_reduce(prev_candidate, *pw2, *ph2);
        candidate_levels.push(reduced_candidate);
    }

    let blocks_x = width.div_ceil(BLOCK_SIDE).max(1);
    let blocks_y = height.div_ceil(BLOCK_SIDE).max(1);

    // winners[block_y][block_x] holds the current best (dx, dy), in the
    // level just processed's own pixel units, carried down (doubled) to
    // seed the next, finer level.
    let mut winners: Vec<Vec<(i32, i32)>> = vec![vec![(0, 0); blocks_x]; blocks_y];

    for level in (0..PYRAMID_LEVELS).rev() {
        let (base_level, level_w, level_h) = &base_levels[level];
        let (candidate_level, _, _) = &candidate_levels[level];
        let block_side_at_level = (BLOCK_SIDE >> level).max(1);

        let mut new_winners = vec![vec![(0i32, 0i32); blocks_x]; blocks_y];
        let mut table_cache: HashMap<(i32, i32), IntegralImage> = HashMap::new();

        for by in 0..blocks_y {
            for bx in 0..blocks_x {
                let block_x0 = bx * block_side_at_level;
                let block_y0 = by * block_side_at_level;
                let block_w = block_side_at_level.min(level_w.saturating_sub(block_x0));
                let block_h = block_side_at_level.min(level_h.saturating_sub(block_y0));

                let pyramid_centre = if level == PYRAMID_LEVELS - 1 {
                    (0, 0)
                } else {
                    let (px, py) = winners[by][bx];
                    (px * 2, py * 2)
                };

                if block_w == 0 || block_h == 0 {
                    new_winners[by][bx] = pyramid_centre;
                    continue;
                }

                let median = median_predictor(&new_winners, bx, by, blocks_x, pyramid_centre);
                let winner = search_block(
                    base_level,
                    candidate_level,
                    *level_w,
                    *level_h,
                    block_x0,
                    block_y0,
                    block_w,
                    block_h,
                    pyramid_centre,
                    median,
                    &mut table_cache,
                );
                new_winners[by][bx] = winner;
            }
        }
        winners = new_winners;
    }

    // Render the residual field at the finest level's winning offsets,
    // caching one warped-and-differenced full-resolution buffer per
    // distinct absolute offset, since many blocks typically share the
    // same winner (most of all, (0, 0)).
    let mut full_diff_cache: HashMap<(i32, i32), Vec<u8>> = HashMap::new();
    let mut samples = vec![FILL_VALUE; base.rgba8().len()];
    for chunk in samples.chunks_exact_mut(4) {
        chunk[3] = u8::MAX;
    }

    let mut blocks = Vec::with_capacity(blocks_x * blocks_y);
    #[allow(clippy::needless_range_loop)]
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let (dx, dy) = winners[by][bx];
            let diff = full_diff_cache.entry((dx, dy)).or_insert_with(|| {
                // `(dx, dy)` means this block's content sits `dx, dy`
                // pixels right and down of the base's, matching
                // `CoarseOffset`'s own convention; bringing the aligned
                // candidate back into the base's frame therefore warps by
                // the negated offset, exactly as the global alignment
                // above does with `coarse`.
                let warped = warp_by_offset(&aligned_frame, -dx, -dy);
                difference_image(base.rgba8(), &warped)
            });

            let x0 = bx * BLOCK_SIDE;
            let y0 = by * BLOCK_SIDE;
            let x1 = (x0 + BLOCK_SIDE).min(width);
            let y1 = (y0 + BLOCK_SIDE).min(height);

            let mut score: u64 = 0;
            for y in y0..y1 {
                let row_start = (y * width + x0) * 4;
                let row_end = (y * width + x1) * 4;
                samples[row_start..row_end].copy_from_slice(&diff[row_start..row_end]);
                for chunk in diff[row_start..row_end].chunks_exact(4) {
                    score += u64::from(chunk[0]) + u64::from(chunk[1]) + u64::from(chunk[2]);
                }
            }

            blocks.push(BlockOffset {
                block_x: bx,
                block_y: by,
                dx,
                dy,
                score,
            });
        }
    }

    Ok(ResidualField {
        width,
        height,
        samples,
        blocks,
    })
}

/// Search one block's window at one pyramid level, and return its winning
/// `(dx, dy)`.
///
/// The candidate list is the window scan around `pyramid_centre`, plus
/// `median` when it differs from `pyramid_centre`, sorted once by
/// ascending squared magnitude (a candidate's own `dx * dx + dy * dy`,
/// never relative to the search centre). Sorting this way makes the early
/// termination below safe: once a score of zero is found, no later
/// candidate in the sorted order can have a smaller magnitude, so no later
/// candidate can win the tie-break rule below either. A tie on score
/// favours the smaller offset magnitude, then the lower position in this
/// sorted order, so the result never depends on the order candidates were
/// generated in.
#[allow(clippy::too_many_arguments)]
fn search_block(
    base_level: &[u8],
    candidate_level: &[u8],
    level_w: usize,
    level_h: usize,
    block_x0: usize,
    block_y0: usize,
    block_w: usize,
    block_h: usize,
    pyramid_centre: (i32, i32),
    median: (i32, i32),
    table_cache: &mut HashMap<(i32, i32), IntegralImage>,
) -> (i32, i32) {
    let mut candidates = window_scan_candidates(pyramid_centre);
    if median != pyramid_centre {
        candidates.push(median);
    }
    candidates.sort_unstable_by_key(|&(dx, dy)| {
        (
            i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy),
            dy,
            dx,
        )
    });
    candidates.dedup();

    let mut best_key: Option<(u64, i64, usize)> = None;
    let mut best_offset = candidates[0];

    for (index, &(dx, dy)) in candidates.iter().enumerate() {
        let table = table_cache.entry((dx, dy)).or_insert_with(|| {
            // Test candidate `(dx, dy)` the same way the residual render
            // does: `(dx, dy)` is the shift being hypothesised for this
            // block's content relative to the base, so bringing the
            // candidate into alignment with the base warps by the
            // negated value.
            let warped = warp_single_channel(candidate_level, level_w, level_h, -dx, -dy);
            let scores: Vec<u8> = base_level
                .iter()
                .zip(warped.iter())
                .map(|(&a, &b)| a.abs_diff(b))
                .collect();
            IntegralImage::from_luma(&scores, level_w, level_h)
        });
        let score = table.window_sum(block_x0, block_y0, block_w, block_h);
        let magnitude = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
        let key = (score, magnitude, index);

        let better = match best_key {
            None => true,
            Some(current) => key < current,
        };
        if better {
            best_key = Some(key);
            best_offset = (dx, dy);
        }
        if score == 0 {
            break;
        }
    }

    best_offset
}

/// Enumerate every candidate offset the window scan around `centre`
/// evaluates: `centre.0 + ddx, centre.1 + ddy` for `ddx` and `ddy` each
/// ranging over `-SEARCH_HALF_WIDTH..=SEARCH_HALF_WIDTH`. This never
/// produces a candidate more than `SEARCH_HALF_WIDTH` away from `centre`
/// on either axis; `search_block` never evaluates a candidate this
/// function did not generate (aside from the one extra median-predictor
/// candidate it may add separately).
fn window_scan_candidates(centre: (i32, i32)) -> Vec<(i32, i32)> {
    let mut candidates =
        Vec::with_capacity(((2 * SEARCH_HALF_WIDTH + 1) * (2 * SEARCH_HALF_WIDTH + 1)) as usize);
    for ddy in -SEARCH_HALF_WIDTH..=SEARCH_HALF_WIDTH {
        for ddx in -SEARCH_HALF_WIDTH..=SEARCH_HALF_WIDTH {
            candidates.push((centre.0 + ddx, centre.1 + ddy));
        }
    }
    candidates
}

/// Predict a block's search centre from the component-wise median of its
/// left, top and top-right neighbours' already-computed offsets at the
/// same pyramid level, following the x264 reference encoder's spatial
/// predictor. Falls back to `fallback` when no such neighbour has been
/// processed yet (raster order visits a block's left and top neighbours
/// before the block itself, but the very first row and column have none).
///
/// Fewer than three neighbours are padded by repeating the last one found,
/// so the median of exactly three values is always well defined.
fn median_predictor(
    new_winners: &[Vec<(i32, i32)>],
    bx: usize,
    by: usize,
    blocks_x: usize,
    fallback: (i32, i32),
) -> (i32, i32) {
    let mut neighbours: Vec<(i32, i32)> = Vec::with_capacity(3);
    if bx > 0 {
        neighbours.push(new_winners[by][bx - 1]);
    }
    if by > 0 {
        neighbours.push(new_winners[by - 1][bx]);
    }
    if by > 0 && bx + 1 < blocks_x {
        neighbours.push(new_winners[by - 1][bx + 1]);
    }
    if neighbours.is_empty() {
        return fallback;
    }
    while neighbours.len() < 3 {
        let last = *neighbours.last().expect("neighbours is non-empty here");
        neighbours.push(last);
    }
    let mut dxs: Vec<i32> = neighbours.iter().map(|o| o.0).collect();
    let mut dys: Vec<i32> = neighbours.iter().map(|o| o.1).collect();
    dxs.sort_unstable();
    dys.sort_unstable();
    (dxs[1], dys[1])
}

/// Reduce an RGBA8 buffer to one luma byte per pixel, at the buffer's own
/// resolution (no resampling), using the same fixed-point weights
/// `to_luma_downsampled` uses: red by 77, green by 150, blue by 29,
/// shifted right by 8.
fn luma_bytes(rgba: &[u8]) -> Vec<u8> {
    let pixel_count = rgba.len() / 4;
    let mut out = vec![0u8; pixel_count];
    for (index, sample) in out.iter_mut().enumerate() {
        let idx = index * 4;
        let red = u32::from(rgba[idx]);
        let green = u32::from(rgba[idx + 1]);
        let blue = u32::from(rgba[idx + 2]);
        *sample = ((red * 77 + green * 150 + blue * 29) >> 8) as u8;
    }
    out
}

/// Reduce a single-channel `width` by `height` buffer to roughly half its
/// size on each axis, by an exact-integer two-by-two box average (integer
/// division, rounding down). An odd width or height averages a smaller,
/// one-sample-wide final row or column rather than reading past the edge.
fn box_reduce(samples: &[u8], width: usize, height: usize) -> (Vec<u8>, usize, usize) {
    let out_width = width.div_ceil(2).max(1);
    let out_height = height.div_ceil(2).max(1);
    let mut out = vec![0u8; out_width * out_height];
    for oy in 0..out_height {
        let y0 = oy * 2;
        let y1 = (y0 + 2).min(height);
        for ox in 0..out_width {
            let x0 = ox * 2;
            let x1 = (x0 + 2).min(width);
            let mut sum: u32 = 0;
            let mut count: u32 = 0;
            for y in y0..y1 {
                for x in x0..x1 {
                    sum += u32::from(samples[y * width + x]);
                    count += 1;
                }
            }
            out[oy * out_width + ox] = (sum / count.max(1)) as u8;
        }
    }
    (out, out_width, out_height)
}

/// Move a single-channel `width` by `height` buffer by exactly `dx`
/// horizontal and `dy` vertical whole pixels, filling a vacated edge with
/// `FILL_VALUE` rather than wrapping, matching `warp_by_offset`'s own
/// convention for the RGBA8 warp.
fn warp_single_channel(samples: &[u8], width: usize, height: usize, dx: i32, dy: i32) -> Vec<u8> {
    let mut out = vec![FILL_VALUE; samples.len()];
    for y in 0..height {
        let src_y = y as i32 - dy;
        if src_y < 0 || src_y as usize >= height {
            continue;
        }
        let src_y = src_y as usize;
        for x in 0..width {
            let src_x = x as i32 - dx;
            if src_x < 0 || src_x as usize >= width {
                continue;
            }
            out[y * width + x] = samples[src_y * width + src_x as usize];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_scan_candidates_never_exceeds_the_named_half_width_on_either_axis() {
        for centre in [(0, 0), (5, -3), (-7, 12)] {
            let candidates = window_scan_candidates(centre);
            for (dx, dy) in candidates {
                assert!((dx - centre.0).abs() <= SEARCH_HALF_WIDTH);
                assert!((dy - centre.1).abs() <= SEARCH_HALF_WIDTH);
            }
        }
    }

    #[test]
    fn window_scan_candidates_covers_every_offset_in_the_window_exactly_once() {
        let candidates = window_scan_candidates((0, 0));
        let expected = ((2 * SEARCH_HALF_WIDTH + 1) * (2 * SEARCH_HALF_WIDTH + 1)) as usize;
        assert_eq!(candidates.len(), expected);
        let mut unique = candidates.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), expected);
    }
}
