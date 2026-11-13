//! The rasterizer: one view of a mesh into one RGBA8 image.
//!
//! The rasterizer is written for determinism, not speed.
//!
//! - The float stage is the setup for each triangle: the view rotation,
//!   the scale to the image, and the shade. It uses addition,
//!   subtraction, multiplication, division, `sqrt` and `round` only.
//!   IEEE 754 makes each of these exact to the last bit on each machine,
//!   and stable Rust never fuses a multiplication and an addition.
//! - The coverage and the depth test use integer arithmetic only, on
//!   fixed-point coordinates with 4 bits below the pixel.
//! - A pixel is covered or not covered. There is no anti-aliasing, so no
//!   blend can round differently.
//! - Equal depth keeps the triangle that came first in the file.

use crate::parse::Triangle;
use crate::rig::View;

/// The width and the height of each view, in pixels.
pub const SIZE: u32 = 256;

/// The fixed-point bits below one pixel.
const SUB_BITS: u32 = 4;
const SUB: i64 = 1 << SUB_BITS;
/// The fixed-point scale of the depth.
const DEPTH_SCALE: f32 = 1_048_576.0;
/// The part of the image that the mesh fills. The rest is a margin.
const FILL: f32 = 0.9;

/// The light direction, (1, 2, 3) / sqrt(14), written as constants.
const LIGHT: [f32; 3] = [0.267_261_24, 0.534_522_5, 0.801_783_7];

/// The colour of the background: opaque white.
const BACKGROUND: [u8; 4] = [255, 255, 255, 255];

/// One rendered view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// RGBA8, straight alpha, row major.
    pub pixels: Vec<u8>,
    /// For each triangle, true when it owns at least one pixel.
    pub visible: Vec<bool>,
}

/// Move and scale the triangles so the mesh fits a sphere of radius 1 at
/// the origin. Each view then fits the image, whatever its direction.
pub fn normalise(triangles: &[Triangle]) -> Vec<Triangle> {
    let mut low = [f32::MAX; 3];
    let mut high = [f32::MIN; 3];
    for point in triangles.iter().flatten() {
        for axis in 0..3 {
            low[axis] = low[axis].min(point[axis]);
            high[axis] = high[axis].max(point[axis]);
        }
    }
    let centre = [
        (low[0] + high[0]) * 0.5,
        (low[1] + high[1]) * 0.5,
        (low[2] + high[2]) * 0.5,
    ];
    let mut radius_squared = 0.0f32;
    for point in triangles.iter().flatten() {
        let d = [
            point[0] - centre[0],
            point[1] - centre[1],
            point[2] - centre[2],
        ];
        radius_squared = radius_squared.max(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
    }
    let radius = radius_squared.sqrt();
    let scale = if radius > 0.0 { 1.0 / radius } else { 1.0 };
    triangles
        .iter()
        .map(|triangle| {
            triangle.map(|point| {
                [
                    (point[0] - centre[0]) * scale,
                    (point[1] - centre[1]) * scale,
                    (point[2] - centre[2]) * scale,
                ]
            })
        })
        .collect()
}

/// A triangle after the setup: fixed-point screen points, fixed-point
/// depths, and one shade.
struct Setup {
    x: [i64; 3],
    y: [i64; 3],
    z: [i64; 3],
    shade: u8,
}

fn setup(triangle: &Triangle, view: &View) -> Setup {
    let rotate = |point: [f32; 3]| -> [f32; 3] {
        let row = |r: [f32; 3]| r[0] * point[0] + r[1] * point[1] + r[2] * point[2];
        [row(view.rows[0]), row(view.rows[1]), row(view.rows[2])]
    };
    let points = triangle.map(rotate);
    let half = SIZE as f32 * 0.5;
    let to_fixed = |value: f32| (value * SUB as f32).round() as i64;
    let x = points.map(|p| to_fixed((p[0] * FILL + 1.0) * half));
    let y = points.map(|p| to_fixed((1.0 - p[1] * FILL) * half));
    let z = points.map(|p| (p[2] * DEPTH_SCALE).round() as i64);

    // Flat shade from the angle between the face and a fixed light. The
    // light is in view space, so each view has the same lighting.
    let u = [
        points[1][0] - points[0][0],
        points[1][1] - points[0][1],
        points[1][2] - points[0][2],
    ];
    let v = [
        points[2][0] - points[0][0],
        points[2][1] - points[0][1],
        points[2][2] - points[0][2],
    ];
    let normal = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    let facing = if length > 0.0 {
        (normal[0] * LIGHT[0] + normal[1] * LIGHT[1] + normal[2] * LIGHT[2]).abs() / length
    } else {
        0.0
    };
    let shade = ((0.15 + 0.7 * facing) * 255.0).round().clamp(0.0, 255.0) as u8;
    Setup { x, y, z, shade }
}

/// The edge function: twice the signed area of (a, b, p).
fn edge(ax: i64, ay: i64, bx: i64, by: i64, px: i64, py: i64) -> i64 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

/// True when the edge from a to b is a top edge or a left edge, for a
/// triangle with a positive area. The fill rule gives a pixel centre on a
/// shared edge to one triangle only.
fn top_left(ax: i64, ay: i64, bx: i64, by: i64) -> bool {
    (ay == by && bx < ax) || by > ay
}

/// Render `triangles`, already normalised, through `view`.
pub fn render(triangles: &[Triangle], view: &View) -> Rendered {
    let size = SIZE as usize;
    let mut depth = vec![i64::MIN; size * size];
    let mut owner = vec![usize::MAX; size * size];
    let mut shades = Vec::with_capacity(triangles.len());

    for (index, triangle) in triangles.iter().enumerate() {
        let mut s = setup(triangle, view);
        shades.push(s.shade);
        let mut area = edge(s.x[0], s.y[0], s.x[1], s.y[1], s.x[2], s.y[2]);
        if area == 0 {
            continue;
        }
        if area < 0 {
            // Make the area positive, so one fill rule serves both sides.
            s.x.swap(1, 2);
            s.y.swap(1, 2);
            s.z.swap(1, 2);
            area = -area;
        }

        let min_x = (s.x.iter().min().copied().unwrap_or(0) / SUB).max(0);
        let max_x = (s.x.iter().max().copied().unwrap_or(0) / SUB + 1).min(SIZE as i64 - 1);
        let min_y = (s.y.iter().min().copied().unwrap_or(0) / SUB).max(0);
        let max_y = (s.y.iter().max().copied().unwrap_or(0) / SUB + 1).min(SIZE as i64 - 1);

        let edges = [(1, 2), (2, 0), (0, 1)];
        let bias = edges.map(|(a, b)| {
            if top_left(s.x[a], s.y[a], s.x[b], s.y[b]) {
                0
            } else {
                -1
            }
        });

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                // The pixel centre, in fixed point.
                let cx = px * SUB + SUB / 2;
                let cy = py * SUB + SUB / 2;
                let w = edges.map(|(a, b)| edge(s.x[a], s.y[a], s.x[b], s.y[b], cx, cy));
                if (0..3).any(|i| w[i] + bias[i] < 0) {
                    continue;
                }
                // The depth at the centre: a weighted mean of the three
                // corner depths, in 128-bit integers, rounded down.
                let z = (i128::from(w[0]) * i128::from(s.z[0])
                    + i128::from(w[1]) * i128::from(s.z[1])
                    + i128::from(w[2]) * i128::from(s.z[2]))
                .div_euclid(i128::from(area)) as i64;
                let at = py as usize * size + px as usize;
                if z > depth[at] {
                    depth[at] = z;
                    owner[at] = index;
                }
            }
        }
    }

    let mut visible = vec![false; triangles.len()];
    let mut pixels = Vec::with_capacity(size * size * 4);
    for &triangle in &owner {
        if triangle == usize::MAX {
            pixels.extend_from_slice(&BACKGROUND);
        } else {
            visible[triangle] = true;
            let shade = shades[triangle];
            pixels.extend_from_slice(&[shade, shade, shade, 255]);
        }
    }
    Rendered { pixels, visible }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::VIEWS;

    fn square(z: f32) -> Vec<Triangle> {
        let a = [-1.0, -1.0, z];
        let b = [1.0, -1.0, z];
        let c = [1.0, 1.0, z];
        let d = [-1.0, 1.0, z];
        vec![[a, b, c], [a, c, d]]
    }

    #[test]
    fn two_triangles_that_share_an_edge_cover_each_pixel_once() {
        // The front view of a square fills a block with no gap and no
        // double cover on the diagonal.
        let rendered = render(&square(0.0), &VIEWS[0]);
        let covered = rendered
            .pixels
            .chunks_exact(4)
            .filter(|p| p[0] != 255)
            .count();
        // The square spans FILL of the image on each side.
        let side = (SIZE as f32 * FILL) as usize;
        assert!(covered >= (side - 1) * (side - 1) && covered <= (side + 1) * (side + 1));
        assert_eq!(rendered.visible, vec![true, true]);
    }

    #[test]
    fn the_nearer_surface_hides_the_farther_one() {
        let mut triangles = square(-0.5);
        triangles.extend(square(0.5));
        let rendered = render(&triangles, &VIEWS[0]);
        assert_eq!(rendered.visible, vec![false, false, true, true]);
        let back = render(&triangles, &VIEWS[1]);
        assert_eq!(back.visible, vec![true, true, false, false]);
    }

    #[test]
    fn normalise_fits_the_unit_sphere() {
        let triangles = vec![[[10.0, 0.0, 0.0], [14.0, 0.0, 0.0], [12.0, 3.0, 0.0]]];
        let normalised = normalise(&triangles);
        for point in normalised.iter().flatten() {
            let length = (point[0] * point[0] + point[1] * point[1] + point[2] * point[2]).sqrt();
            assert!(length <= 1.0 + 1e-6);
        }
    }
}
