//! The fixed eight-view rig.
//!
//! The rig is part of Chrysoberyl. A person cannot change it in v1.
//! Each view is an orthographic camera that looks at the centre of the
//! mesh. Each matrix is a constant: the rows are the right, the up and the
//! towards-the-camera axes of the view, in model coordinates.
//!
//! The isometric entries are written as decimal constants. They are not
//! calculated with a square root at run time, so no platform function
//! takes part in a view matrix.

/// A view: a name and a rotation from model space to view space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// The name that a report shows.
    pub name: &'static str,
    /// The rows: right, up, and towards the camera.
    pub rows: [[f32; 3]; 3],
}

/// 1 / sqrt(2).
const A: f32 = 0.707_106_77;
/// 1 / sqrt(6).
const B: f32 = 0.408_248_3;
/// 2 / sqrt(6).
const B2: f32 = 0.816_496_6;
/// 1 / sqrt(3).
const C: f32 = 0.577_350_26;

/// The eight views, in the order of the frames.
pub const VIEWS: [View; 8] = [
    View {
        name: "front",
        rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    },
    View {
        name: "back",
        rows: [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]],
    },
    View {
        name: "right",
        rows: [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
    },
    View {
        name: "left",
        rows: [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
    },
    View {
        name: "top",
        rows: [[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]],
    },
    View {
        name: "bottom",
        rows: [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]],
    },
    View {
        name: "upper corner",
        rows: [[A, 0.0, -A], [-B, B2, -B], [C, C, C]],
    },
    View {
        name: "lower corner",
        rows: [[-A, 0.0, A], [-B, B2, -B], [-C, -C, -C]],
    },
];

/// The blind spot of the rig, in words. A report prints this beside the
/// measured count of hidden triangles.
pub const BLIND_SPOT: &str = "a surface that no camera of the eight views sees, such as the inside of a closed cavity or a deep concave pocket, is not compared";

#[cfg(test)]
mod tests {
    use super::*;

    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    #[test]
    fn each_view_is_a_right_handed_rotation() {
        for view in VIEWS {
            let [right, up, towards] = view.rows;
            for row in view.rows {
                assert!((dot(row, row) - 1.0).abs() < 1e-6, "{}", view.name);
            }
            assert!(dot(right, up).abs() < 1e-6, "{}", view.name);
            assert!(dot(up, towards).abs() < 1e-6, "{}", view.name);
            let product = cross(right, up);
            for axis in 0..3 {
                assert!(
                    (product[axis] - towards[axis]).abs() < 1e-6,
                    "{}",
                    view.name
                );
            }
        }
    }

    #[test]
    fn the_eight_views_look_from_eight_directions() {
        for (i, a) in VIEWS.iter().enumerate() {
            for b in &VIEWS[i + 1..] {
                assert_ne!(a.rows[2], b.rows[2], "{} and {}", a.name, b.name);
            }
        }
    }
}
