//! Drop a difference that is only antialiasing.
//!
//! This is pixelmatch's own heuristic (github.com/mapbox/pixelmatch,
//! `antialiased()` and `hasManySiblings()`), not an implementation of Yee
//! 2004: Yee's paper handles antialiasing implicitly, through a Laplacian
//! pyramid and a contrast sensitivity function, and states no explicit
//! pixel rule of its own. A thin, single-pixel feature such as a hairline
//! or a small dot can fail the neighbour-count test below even when it is
//! genuinely just antialiased, because its own neighbourhood has too few
//! pixels sharing its exact value. A real, small change that happens to
//! produce a smooth local gradient in both frames will be classified as
//! antialiasing and suppressed. Both limits are known and accepted for
//! this phase; a future perceptual metric is the fix, not a wider
//! threshold.

use chrys_source::Frame;

use crate::register::block_match::ResidualField;

/// The number of neighbours that decide whether a pixel sits in a
/// solid-colour area rather than on an edge, and whether an edge's own
/// extreme neighbour looks like part of a smooth gradient. pixelmatch
/// names this threshold as "more than two" in both roles.
const ANTIALIAS_SIBLING_THRESHOLD: usize = 2;

/// Return true when the pixel at `(x, y)` looks like antialiasing rather
/// than a structural change.
///
/// Walk the pixel's eight neighbours in `base` and track the smallest and
/// the largest brightness delta, and the neighbour position at each
/// extreme. Return false immediately when more than
/// `ANTIALIAS_SIBLING_THRESHOLD` neighbours have a zero delta, because
/// that is a solid-colour area, not an edge. When a genuine smallest and
/// largest pair exists, test whether the pixel at each of those two
/// extreme positions has more than `ANTIALIAS_SIBLING_THRESHOLD`
/// neighbours of its own with an identical value, in both `base` and
/// `candidate`. Only when both endpoints of the local brightness ramp
/// look like part of a smooth gradient in both frames does this function
/// return true.
pub fn is_antialiasing(base: &Frame, candidate: &Frame, x: u32, y: u32) -> bool {
    let centre = brightness(base, x, y);

    let mut zero_delta_neighbours = 0usize;
    let mut min_delta: Option<i32> = None;
    let mut max_delta: Option<i32> = None;
    let mut min_pos = (x, y);
    let mut max_pos = (x, y);

    for_each_neighbour(x, y, base.width, base.height, |nx, ny| {
        let delta = brightness(base, nx, ny) - centre;
        if delta == 0 {
            zero_delta_neighbours += 1;
        }
        if min_delta.is_none_or(|current| delta < current) {
            min_delta = Some(delta);
            min_pos = (nx, ny);
        }
        if max_delta.is_none_or(|current| delta > current) {
            max_delta = Some(delta);
            max_pos = (nx, ny);
        }
    });

    if zero_delta_neighbours > ANTIALIAS_SIBLING_THRESHOLD {
        return false;
    }

    // A genuine smallest-and-largest pair means the neighbourhood carries
    // real variation, not just one outlier repeated as both extremes.
    match (min_delta, max_delta) {
        (Some(min), Some(max)) if min != max => {}
        _ => return false,
    }

    let min_is_gradient_endpoint = has_many_siblings(base, min_pos.0, min_pos.1)
        && has_many_siblings(candidate, min_pos.0, min_pos.1);
    let max_is_gradient_endpoint = has_many_siblings(base, max_pos.0, max_pos.1)
        && has_many_siblings(candidate, max_pos.0, max_pos.1);

    min_is_gradient_endpoint && max_is_gradient_endpoint
}

/// Zero every residual sample whose pixel `is_antialiasing` accepts.
///
/// Call this before `label_regions`, not from inside it, so the two
/// stages stay separately testable: suppression decides which pixels are
/// noise, and labelling groups whatever is left.
pub fn suppress_antialiasing(residual: &mut ResidualField, base: &Frame, candidate: &Frame) {
    let width = residual.width as u32;
    let height = residual.height as u32;

    for y in 0..height {
        for x in 0..width {
            if is_antialiasing(base, candidate, x, y) {
                let idx = ((y * width + x) * 4) as usize;
                residual.samples[idx] = 0;
                residual.samples[idx + 1] = 0;
                residual.samples[idx + 2] = 0;
            }
        }
    }
}

/// Return true when `(x, y)`'s own eight neighbours in `frame` include
/// more than `ANTIALIAS_SIBLING_THRESHOLD` pixels with a brightness
/// identical to `(x, y)`'s own, which is what a smooth gradient's endpoint
/// looks like: most of its own neighbourhood still shares its value.
fn has_many_siblings(frame: &Frame, x: u32, y: u32) -> bool {
    let centre = brightness(frame, x, y);
    let mut identical_neighbours = 0usize;
    for_each_neighbour(x, y, frame.width, frame.height, |nx, ny| {
        if brightness(frame, nx, ny) == centre {
            identical_neighbours += 1;
        }
    });
    identical_neighbours > ANTIALIAS_SIBLING_THRESHOLD
}

/// Call `f` once for every pixel in `(x, y)`'s eight-neighbour
/// neighbourhood that lies inside a `width` by `height` frame, clamped at
/// an edge or corner rather than reading past it.
fn for_each_neighbour(x: u32, y: u32, width: u32, height: u32, mut f: impl FnMut(u32, u32)) {
    let x0 = x.saturating_sub(1);
    let y0 = y.saturating_sub(1);
    let x2 = (x + 1).min(width.saturating_sub(1));
    let y2 = (y + 1).min(height.saturating_sub(1));
    for ny in y0..=y2 {
        for nx in x0..=x2 {
            if nx == x && ny == y {
                continue;
            }
            f(nx, ny);
        }
    }
}

/// The weighted luma of the pixel at `(x, y)` in `frame`, using the same
/// fixed-point weights `to_luma_downsampled` uses: red by 77, green by
/// 150, blue by 29, shifted right by 8. Signed, so a delta between two
/// neighbours can be negative.
fn brightness(frame: &Frame, x: u32, y: u32) -> i32 {
    let idx = ((y * frame.width + x) * 4) as usize;
    let pixels = frame.rgba8();
    let red = i32::from(pixels[idx]);
    let green = i32::from(pixels[idx + 1]);
    let blue = i32::from(pixels[idx + 2]);
    (red * 77 + green * 150 + blue * 29) >> 8
}
