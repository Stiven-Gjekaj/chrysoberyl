//! Bounding boxes of 8-connected groups in a boolean mask.
//!
//! The engine has its own labeller in `chrys-core`. This one is separate,
//! so the GPU mirror cannot share a fault with the engine that it checks.

use chrys_core::BoundingBox;

/// Return the bounding box of each 8-connected group of `true` values in
/// `mask`, sorted by top edge, then by left edge.
///
/// `mask` is row major, `width` by `height`.
pub fn boxes(mask: &[bool], width: u32, height: u32) -> Vec<BoundingBox> {
    let (w, h) = (width as usize, height as usize);
    let mut seen = vec![false; mask.len()];
    let mut result = Vec::new();
    let mut stack = Vec::new();

    for start in 0..mask.len() {
        if !mask[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let (mut left, mut top) = (start % w, start / w);
        let (mut right, mut bottom) = (left, top);
        while let Some(index) = stack.pop() {
            let (x, y) = (index % w, index / w);
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
            for dy in [-1isize, 0, 1] {
                for dx in [-1isize, 0, 1] {
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                        continue;
                    }
                    let next = ny as usize * w + nx as usize;
                    if mask[next] && !seen[next] {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
        }
        result.push(BoundingBox {
            x: left as u32,
            y: top as u32,
            width: (right - left + 1) as u32,
            height: (bottom - top + 1) as u32,
        });
    }
    result.sort_by_key(|bbox| (bbox.y, bbox.x));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(rows: &[&str]) -> (Vec<bool>, u32, u32) {
        let width = rows[0].len() as u32;
        let values = rows
            .iter()
            .flat_map(|row| row.chars().map(|c| c == '#'))
            .collect();
        (values, width, rows.len() as u32)
    }

    #[test]
    fn an_empty_mask_has_no_box() {
        let (values, w, h) = mask(&["....", "...."]);
        assert!(boxes(&values, w, h).is_empty());
    }

    #[test]
    fn a_diagonal_neighbour_joins_the_same_group() {
        let (values, w, h) = mask(&["#...", ".#..", "...."]);
        assert_eq!(
            boxes(&values, w, h),
            vec![BoundingBox {
                x: 0,
                y: 0,
                width: 2,
                height: 2
            }]
        );
    }

    #[test]
    fn two_separate_groups_give_two_sorted_boxes() {
        let (values, w, h) = mask(&["...##", "...##", ".....", "#...."]);
        let found = boxes(&values, w, h);
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].x, found[0].y, found[0].width), (3, 0, 2));
        assert_eq!((found[1].x, found[1].y), (0, 3));
    }
}
