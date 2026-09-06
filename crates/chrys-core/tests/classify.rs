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
use chrys_core::classify::{RESIDUAL_THRESHOLD, label_regions};
use chrys_core::register::ResidualField;

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
