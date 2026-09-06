//! Integration tests for the classify stage: labelling changed pixels into
//! regions, dropping an antialiasing-only difference, and naming every
//! region's kind and colour change.
//!
//! Plan 01-07 Task 1 covers `label_regions` and `LabelledRegion`. Task 2
//! adds `is_antialiasing` and `suppress_antialiasing`. Task 3 adds
//! `colour_delta` and `classify_kind`. Every residual field and frame pair
//! below is built inside the test by writing bytes into a buffer directly,
//! never by loading a fixture, so a red test names the shape that broke
//! it.

use chrys_core::BoundingBox;
use chrys_core::classify::{
    RESIDUAL_THRESHOLD, is_antialiasing, label_regions, suppress_antialiasing,
};
use chrys_core::register::ResidualField;
use chrys_source::Frame;

fn empty_residual(width: usize, height: usize) -> ResidualField {
    let mut samples = vec![0u8; width * height * 4];
    for chunk in samples.chunks_exact_mut(4) {
        chunk[3] = u8::MAX;
    }
    ResidualField {
        width,
        height,
        samples,
        blocks: Vec::new(),
    }
}

fn set_pixel(residual: &mut ResidualField, x: usize, y: usize, value: u8) {
    let idx = (y * residual.width + x) * 4;
    residual.samples[idx] = value;
    residual.samples[idx + 1] = value;
    residual.samples[idx + 2] = value;
}

#[test]
fn label_regions_on_an_all_zero_residual_returns_an_empty_list() {
    let residual = empty_residual(8, 8);
    assert!(label_regions(&residual).is_empty());
}

#[test]
fn label_regions_on_two_separated_blobs_returns_two_regions() {
    let mut residual = empty_residual(10, 10);
    set_pixel(&mut residual, 1, 1, 255);
    set_pixel(&mut residual, 8, 8, 255);
    assert_eq!(label_regions(&residual).len(), 2);
}

#[test]
fn label_regions_on_blobs_touching_only_at_a_corner_returns_one_region() {
    let mut residual = empty_residual(10, 10);
    set_pixel(&mut residual, 4, 4, 255);
    set_pixel(&mut residual, 5, 5, 255);
    assert_eq!(label_regions(&residual).len(), 1);
}

#[test]
fn each_bounding_box_is_the_smallest_rectangle_holding_its_region() {
    let mut residual = empty_residual(10, 10);
    // An L shape: the bounding box must cover the whole 2x2 square even
    // though the pixel at (3, 3) is never set.
    set_pixel(&mut residual, 2, 2, 255);
    set_pixel(&mut residual, 3, 2, 255);
    set_pixel(&mut residual, 2, 3, 255);
    let regions = label_regions(&residual);
    assert_eq!(regions.len(), 1);
    assert_eq!(
        regions[0].bbox,
        BoundingBox {
            x: 2,
            y: 2,
            width: 2,
            height: 2,
        }
    );
    assert_eq!(regions[0].pixel_count, 3);
}

#[test]
fn the_region_list_is_sorted_by_bounding_box_position_top_to_bottom_then_left_to_right() {
    let mut residual = empty_residual(20, 20);
    set_pixel(&mut residual, 15, 1, 255);
    set_pixel(&mut residual, 1, 1, 255);
    set_pixel(&mut residual, 1, 15, 255);
    let regions = label_regions(&residual);
    assert_eq!(regions.len(), 3);
    assert_eq!((regions[0].bbox.y, regions[0].bbox.x), (1, 1));
    assert_eq!((regions[1].bbox.y, regions[1].bbox.x), (1, 15));
    assert_eq!((regions[2].bbox.y, regions[2].bbox.x), (15, 1));
}

#[test]
fn a_residual_value_at_or_below_the_threshold_is_not_part_of_any_region() {
    let mut residual = empty_residual(8, 8);
    set_pixel(&mut residual, 3, 3, RESIDUAL_THRESHOLD);
    assert!(label_regions(&residual).is_empty());

    set_pixel(&mut residual, 3, 3, RESIDUAL_THRESHOLD + 1);
    assert_eq!(label_regions(&residual).len(), 1);
}

fn frame_from(width: u32, height: u32, pixels: Vec<u8>) -> Frame {
    Frame {
        pixels,
        width,
        height,
        index: 0,
        hints: Vec::new(),
    }
}

fn solid_frame(width: u32, height: u32, value: u8) -> Frame {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        pixels.extend_from_slice(&[value, value, value, 255]);
    }
    frame_from(width, height, pixels)
}

/// A frame with a one-pixel-wide vertical transition column at `edge_x`:
/// every column left of it is `bg`, every column right of it is `fg`, and
/// the column itself is `mid`. Grey (R = G = B) pixels are used throughout
/// so the weighted luma this module computes equals the channel value
/// exactly, which keeps the arithmetic in this test's own comments exact.
fn vertical_step_frame(width: u32, height: u32, bg: u8, mid: u8, fg: u8, edge_x: u32) -> Frame {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for _y in 0..height {
        for x in 0..width {
            let value = match x.cmp(&edge_x) {
                std::cmp::Ordering::Less => bg,
                std::cmp::Ordering::Equal => mid,
                std::cmp::Ordering::Greater => fg,
            };
            pixels.extend_from_slice(&[value, value, value, 255]);
        }
    }
    frame_from(width, height, pixels)
}

fn paint_rect(pixels: &mut [u8], stride: u32, x: u32, y: u32, width: u32, height: u32, value: u8) {
    for row in y..y + height {
        for col in x..x + width {
            let idx = ((row * stride + col) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
}

/// Build a base/candidate pair that differs only along a one-pixel-wide
/// diagonal antialiasing band, using integer coverage arithmetic: every
/// pixel's value is a plain function of `x - y`, giving both frames the
/// same unbroken diagonal edge with no artificial seam. The two frames
/// differ only at the transition pixels that sit `margin` pixels or more
/// from every edge, using `mid_base` in the base frame and
/// `mid_candidate` in the candidate, the way the same edge looks when
/// rendered at two different subpixel positions. A transition pixel
/// closer than `margin` to an edge repeats the base's own value in the
/// candidate, since a feature that close to a frame edge has no room for
/// the eight-neighbour check this rule needs, in either frame.
fn diagonal_edge_pair(
    size: u32,
    margin: u32,
    bg: u8,
    mid_base: u8,
    mid_candidate: u8,
    fg: u8,
) -> (Frame, Frame) {
    // The diagonal `d = x - y` runs the whole frame, unbroken, so its bg
    // and fg sides are the plain, uninterrupted shape a real diagonal edge
    // has: no artificial seam from clipping the pattern to a box. The two
    // frames differ only at a transition pixel that sits `margin` pixels
    // or more from every edge, so every differing pixel keeps the full
    // eight-neighbour clearance this rule's own "many siblings" check
    // needs on both sides of the edge; nearer the frame's own edges the
    // candidate repeats the base's value, exactly like the antialiasing
    // rule's own written blind spot: a feature too close to an edge to
    // have room for its own neighbourhood is not this test's concern.
    let mut base_pixels = Vec::with_capacity((size * size * 4) as usize);
    let mut candidate_pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let d = x as i32 - y as i32;
            let base_value = match d.cmp(&0) {
                std::cmp::Ordering::Less => bg,
                std::cmp::Ordering::Equal => mid_base,
                std::cmp::Ordering::Greater => fg,
            };
            let is_safe_interior =
                x >= margin && x < size - margin && y >= margin && y < size - margin;
            let candidate_value = if d == 0 && is_safe_interior {
                mid_candidate
            } else {
                base_value
            };
            base_pixels.extend_from_slice(&[base_value, base_value, base_value, 255]);
            candidate_pixels.extend_from_slice(&[
                candidate_value,
                candidate_value,
                candidate_value,
                255,
            ]);
        }
    }
    (
        frame_from(size, size, base_pixels),
        frame_from(size, size, candidate_pixels),
    )
}

/// A residual field built from the raw per-channel absolute difference of
/// two same-position frames, never by loading a fixture. This is a
/// stand-in for the registration pipeline's own residual: the tests in
/// this file exercise the classify stage on its own, on a pair that never
/// moved, so a raw pixel difference is the same residual a real,
/// zero-offset block match would report.
fn residual_from_raw_diff(base: &Frame, candidate: &Frame) -> ResidualField {
    let width = base.width as usize;
    let height = base.height as usize;
    let base_pixels = base.rgba8();
    let candidate_pixels = candidate.rgba8();
    let mut samples = vec![0u8; width * height * 4];
    for (idx, chunk) in samples.chunks_exact_mut(4).enumerate() {
        let byte_idx = idx * 4;
        chunk[0] = base_pixels[byte_idx].abs_diff(candidate_pixels[byte_idx]);
        chunk[1] = base_pixels[byte_idx + 1].abs_diff(candidate_pixels[byte_idx + 1]);
        chunk[2] = base_pixels[byte_idx + 2].abs_diff(candidate_pixels[byte_idx + 2]);
        chunk[3] = u8::MAX;
    }
    ResidualField {
        width,
        height,
        samples,
        blocks: Vec::new(),
    }
}

#[test]
fn is_antialiasing_returns_false_in_a_solid_colour_area() {
    let frame = solid_frame(10, 10, 120);
    assert!(!is_antialiasing(&frame, &frame, 5, 5));
}

#[test]
fn is_antialiasing_returns_true_for_a_pixel_on_a_smooth_ramp_both_frames_share() {
    let frame = vertical_step_frame(12, 12, 30, 150, 250, 5);
    assert!(is_antialiasing(&frame, &frame, 5, 6));
}

#[test]
fn is_antialiasing_returns_false_at_the_centre_of_a_solid_block_that_changed_colour() {
    let base = solid_frame(20, 20, 30);
    let mut candidate_pixels = base.pixels.clone();
    paint_rect(&mut candidate_pixels, 20, 5, 5, 10, 10, 220);
    let candidate = frame_from(20, 20, candidate_pixels);
    // The base frame is flat everywhere the recolour will land: it has no
    // edge of its own for this rule to find, so neither endpoint of a
    // local ramp can look like a gradient, because there is no ramp.
    assert!(!is_antialiasing(&base, &candidate, 10, 10));
}

#[test]
fn suppress_antialiasing_on_a_diagonal_edge_only_difference_yields_no_region() {
    let (base, candidate) = diagonal_edge_pair(20, 5, 30, 90, 190, 220);
    let mut field = residual_from_raw_diff(&base, &candidate);

    // Sanity: before suppression, the diagonal band is a real difference.
    assert!(!label_regions(&field).is_empty());

    suppress_antialiasing(&mut field, &base, &candidate);
    assert!(label_regions(&field).is_empty());
}

#[test]
fn suppression_runs_before_labelling_so_an_edge_pixel_never_joins_a_region() {
    let (base, candidate) = diagonal_edge_pair(20, 5, 30, 90, 190, 220);
    let mut field = residual_from_raw_diff(&base, &candidate);
    suppress_antialiasing(&mut field, &base, &candidate);

    // The diagonal band runs through the interior box at local (x, x);
    // (10, 10) sits on it (margin 5, local (5, 5)).
    let idx = (10 * field.width + 10) * 4;
    assert_eq!(&field.samples[idx..idx + 3], &[0, 0, 0]);
}

#[test]
fn suppress_antialiasing_keeps_a_real_recoloured_blocks_region() {
    let base = solid_frame(20, 20, 30);
    let mut candidate_pixels = base.pixels.clone();
    paint_rect(&mut candidate_pixels, 20, 5, 5, 6, 6, 220);
    let candidate = frame_from(20, 20, candidate_pixels);

    let mut field = residual_from_raw_diff(&base, &candidate);
    suppress_antialiasing(&mut field, &base, &candidate);

    let regions = label_regions(&field);
    assert_eq!(regions.len(), 1);
    assert_eq!(
        regions[0].bbox,
        BoundingBox {
            x: 5,
            y: 5,
            width: 6,
            height: 6,
        }
    );
}
