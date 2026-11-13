//! The conversion from Y'CbCr to RGB, in integer arithmetic only.
//!
//! The matrix and the range come from the tags in the stream. This module
//! never guesses a tag. An unknown matrix is an error.
//!
//! Each coefficient is a rational number from the standard, scaled by
//! 2^16 and rounded once. The conversion of a pixel uses integer
//! additions, multiplications and shifts only, so the result is the same
//! on each machine.

/// The matrix that a stream declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matrix {
    /// ITU-R BT.601 (also tagged BT.470BG or SMPTE 170M).
    Bt601,
    /// ITU-R BT.709.
    Bt709,
    /// ITU-R BT.2020, non-constant luminance.
    Bt2020Ncl,
    /// The planes hold G, B and R directly.
    Identity,
}

/// The range that a stream declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    /// Luma from 16 to 235, chroma from 16 to 240.
    Limited,
    /// Each value from 0 to 255.
    Full,
}

const SHIFT: u32 = 16;
const ONE: i64 = 1 << SHIFT;
/// The denominator of the luma weights below.
const D: i64 = 10_000;

/// Divide and round half away from zero.
fn div_round(numerator: i64, denominator: i64) -> i64 {
    let half = denominator / 2;
    if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        (numerator - half) / denominator
    }
}

/// The fixed-point coefficients for one matrix and one range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coefficients {
    identity: bool,
    y_offset: i64,
    y_scale: i64,
    cr_to_r: i64,
    cb_to_g: i64,
    cr_to_g: i64,
    cb_to_b: i64,
}

impl Coefficients {
    /// Make the coefficients for `matrix` and `range`.
    pub fn new(matrix: Matrix, range: Range) -> Self {
        // The luma weights Kr and Kb, as parts of D.
        let (kr, kb) = match matrix {
            Matrix::Bt601 => (2_990, 1_140),
            Matrix::Bt709 => (2_126, 722),
            Matrix::Bt2020Ncl => (2_627, 593),
            Matrix::Identity => (0, 0),
        };
        let kg = D - kr - kb;
        let (y_offset, y_scale, c_scale_num, c_scale_den) = match range {
            Range::Full => (0, ONE, 1, 1),
            Range::Limited => (16, div_round(255 * ONE, 219), 255, 224),
        };
        let scaled = |numerator: i64, denominator: i64| {
            div_round(numerator * ONE * c_scale_num, denominator * c_scale_den)
        };
        Coefficients {
            identity: matrix == Matrix::Identity,
            y_offset,
            y_scale,
            cr_to_r: scaled(2 * (D - kr), D),
            cb_to_b: scaled(2 * (D - kb), D),
            cb_to_g: if kg == 0 {
                0
            } else {
                scaled(2 * kb * (D - kb), D * kg)
            },
            cr_to_g: if kg == 0 {
                0
            } else {
                scaled(2 * kr * (D - kr), D * kg)
            },
        }
    }

    /// Convert one sample to RGB.
    pub fn to_rgb(&self, y: u8, cb: u8, cr: u8) -> [u8; 3] {
        if self.identity {
            // The planes hold G, B and R. The range still applies.
            let channel = |value: u8| {
                let v = (i64::from(value) - self.y_offset) * self.y_scale;
                clamp((v + ONE / 2) >> SHIFT)
            };
            return [channel(cr), channel(y), channel(cb)];
        }
        let luma = (i64::from(y) - self.y_offset) * self.y_scale;
        let cb = i64::from(cb) - 128;
        let cr = i64::from(cr) - 128;
        let round = ONE / 2;
        let r = (luma + cr * self.cr_to_r + round) >> SHIFT;
        let g = (luma - cb * self.cb_to_g - cr * self.cr_to_g + round) >> SHIFT;
        let b = (luma + cb * self.cb_to_b + round) >> SHIFT;
        [clamp(r), clamp(g), clamp(b)]
    }
}

fn clamp(value: i64) -> u8 {
    value.clamp(0, 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limited_black_and_white_map_to_0_and_255() {
        let c = Coefficients::new(Matrix::Bt709, Range::Limited);
        assert_eq!(c.to_rgb(16, 128, 128), [0, 0, 0]);
        assert_eq!(c.to_rgb(235, 128, 128), [255, 255, 255]);
    }

    #[test]
    fn full_range_grey_stays_grey() {
        let c = Coefficients::new(Matrix::Bt601, Range::Full);
        assert_eq!(c.to_rgb(77, 128, 128), [77, 77, 77]);
    }

    #[test]
    fn the_matrix_changes_the_result() {
        // A strong red in BT.709 limited range. BT.601 gives another colour,
        // so a stream with a wrong tag gives a wrong colour.
        let bt709 = Coefficients::new(Matrix::Bt709, Range::Limited).to_rgb(63, 102, 240);
        let bt601 = Coefficients::new(Matrix::Bt601, Range::Limited).to_rgb(63, 102, 240);
        // The code values (63, 102, 240) are the rounded encoding of red,
        // so green is 0.59 before the rounding, not 0.
        assert_eq!(bt709, [255, 1, 0]);
        assert_ne!(bt709, bt601);
    }

    #[test]
    fn full_range_bt601_red_matches_the_standard() {
        // Y' = 0.299 * 255, Cb = 128 - 0.1687 * 255, Cr = 255 (clipped).
        let c = Coefficients::new(Matrix::Bt601, Range::Full);
        assert_eq!(c.to_rgb(76, 85, 255), [254, 0, 0]);
    }

    #[test]
    fn identity_reads_g_b_r() {
        let c = Coefficients::new(Matrix::Identity, Range::Full);
        assert_eq!(c.to_rgb(10, 20, 30), [30, 10, 20]);
    }
}
