//! Lab colour difference through `palette`'s pure-Rust maths.
//!
//! Lab conversion needs a cube root regardless of which colour-difference
//! formula sits above it, and Rust's own standard library documents that
//! cube root as varying by platform, by Rust version, and even between
//! two calls inside one execution. `palette`'s `libm` feature routes
//! that cube root, and every other colour transcendental this module
//! reaches, through pure Rust instead of the platform library. See
//! `Cargo.toml`'s own `palette` entry: `default-features = false`,
//! `features = ["libm"]`.

use palette::color_difference::EuclideanDistance;
use palette::{IntoColor, Lab, Srgb};

use crate::verdict::ColourDelta;

/// The colour difference between `base` and `candidate`, both RGBA8.
///
/// This reports the Euclidean distance in Lab space, the CIE76-style
/// delta E, not the CIEDE2000 formula: CIEDE2000 needs an arc tangent, a
/// sine, a cosine and a power function on top of the cube root every Lab
/// conversion already needs, while the Euclidean distance needs only a
/// square root, which Rust's own precision documentation states is
/// guaranteed not to change. Shipping the simpler formula first is a
/// deliberate choice, not an oversight: the richer, more perceptually
/// accurate CIEDE2000 formula is the intended upgrade once this phase's
/// six-runner determinism matrix is green a second time, on this module,
/// and a second data point exists. That upgrade still routes through
/// this crate's own pure-Rust `libm` feature; it changes the formula, not
/// the transcendental-safety discipline around it.
pub fn colour_delta(base: [u8; 4], candidate: [u8; 4]) -> ColourDelta {
    let base_lab: Lab = to_srgb_f32(base).into_color();
    let candidate_lab: Lab = to_srgb_f32(candidate).into_color();
    let delta_e = base_lab.distance(candidate_lab);

    ColourDelta {
        delta_e,
        base,
        candidate,
    }
}

/// Convert an RGBA8 colour's red, green and blue channels to `palette`'s
/// sRGB type at `f32` precision. Alpha carries no colour information and
/// plays no part in a Lab conversion, so it is dropped here.
fn to_srgb_f32(rgba: [u8; 4]) -> Srgb<f32> {
    Srgb::<u8>::new(rgba[0], rgba[1], rgba[2]).into_format()
}
