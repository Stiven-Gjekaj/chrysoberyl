//! Integer-only luma reduction, from a decoded frame down to the fixed
//! working grid the register stage transforms.
//!
//! Every step in this file uses integer arithmetic. There is no
//! floating-point type and no floating-point literal anywhere below, and no
//! interpolation kernel. The integer path is chosen because it removes a
//! whole class of rounding divergence before the transform ever runs: an
//! integer sum and an integer shift give the same result on every machine,
//! with no rounding mode and no transcendental function involved.

use chrys_source::Frame;

use crate::register::window::WORKING_RESOLUTION;

/// Reduce `frame` to a single luma byte per pixel, then box-average that
/// down to a `WORKING_RESOLUTION` by `WORKING_RESOLUTION` grid.
///
/// The luma weight is the fixed-point form that multiplies red by 77, green
/// by 150 and blue by 29 and shifts the sum right by 8 (`77 + 150 + 29 =
/// 256`). The result always has exactly `WORKING_RESOLUTION *
/// WORKING_RESOLUTION` samples, whatever the input frame's own width and
/// height are.
pub fn to_luma_downsampled(frame: &Frame) -> Vec<u8> {
    let width = frame.width as usize;
    let height = frame.height as usize;
    let pixels = frame.rgba8();

    let mut luma = vec![0u8; width * height];
    for (index, sample) in luma.iter_mut().enumerate() {
        let idx = index * 4;
        let red = u32::from(pixels[idx]);
        let green = u32::from(pixels[idx + 1]);
        let blue = u32::from(pixels[idx + 2]);
        *sample = ((red * 77 + green * 150 + blue * 29) >> 8) as u8;
    }

    let mut out = vec![0u8; WORKING_RESOLUTION * WORKING_RESOLUTION];
    for out_y in 0..WORKING_RESOLUTION {
        let (y0, y1) = source_range(out_y, height);
        for out_x in 0..WORKING_RESOLUTION {
            let (x0, x1) = source_range(out_x, width);
            let mut sum: u32 = 0;
            let mut count: u32 = 0;
            for y in y0..y1 {
                for x in x0..x1 {
                    sum += u32::from(luma[y * width + x]);
                    count += 1;
                }
            }
            out[out_y * WORKING_RESOLUTION + out_x] = (sum / count.max(1)) as u8;
        }
    }
    out
}

/// Return the half-open `[start, end)` range of source samples that box
/// index `out_index` of `WORKING_RESOLUTION` output samples averages over,
/// out of `source_len` total source samples.
///
/// The range always holds at least one sample, even when `source_len` is
/// smaller than `WORKING_RESOLUTION`, so an input narrower than the working
/// grid still produces a full grid, by replicating its own last sample
/// across the remaining output positions rather than leaving a box empty.
fn source_range(out_index: usize, source_len: usize) -> (usize, usize) {
    if source_len == 0 {
        return (0, 0);
    }
    let start = out_index * source_len / WORKING_RESOLUTION;
    let end = ((out_index + 1) * source_len / WORKING_RESOLUTION)
        .max(start + 1)
        .min(source_len);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_frame(width: u32, height: u32, colour: [u8; 4]) -> Frame {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            pixels.extend_from_slice(&colour);
        }
        Frame {
            pixels,
            width,
            height,
            index: 0,
            hints: Vec::new(),
        }
    }

    #[test]
    fn returns_exactly_working_resolution_squared_samples_for_a_small_frame() {
        let frame = solid_frame(8, 8, [10, 20, 30, 255]);
        let luma = to_luma_downsampled(&frame);
        assert_eq!(luma.len(), WORKING_RESOLUTION * WORKING_RESOLUTION);
    }

    #[test]
    fn returns_exactly_working_resolution_squared_samples_for_a_large_frame() {
        let frame = solid_frame(1024, 768, [10, 20, 30, 255]);
        let luma = to_luma_downsampled(&frame);
        assert_eq!(luma.len(), WORKING_RESOLUTION * WORKING_RESOLUTION);
    }

    #[test]
    fn a_solid_colour_frame_downsamples_to_one_flat_luma_value() {
        let frame = solid_frame(
            WORKING_RESOLUTION as u32,
            WORKING_RESOLUTION as u32,
            [120, 120, 120, 255],
        );
        let luma = to_luma_downsampled(&frame);
        let expected = ((120u32 * 77 + 120 * 150 + 120 * 29) >> 8) as u8;
        assert!(luma.iter().all(|&sample| sample == expected));
    }

    #[test]
    fn the_same_frame_always_gives_the_same_samples() {
        let frame = solid_frame(300, 200, [5, 250, 60, 255]);
        assert_eq!(to_luma_downsampled(&frame), to_luma_downsampled(&frame));
    }
}
