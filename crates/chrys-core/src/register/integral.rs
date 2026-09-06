//! The summed-area table: a rectangle sum in four lookups, whatever the
//! rectangle size.
//!
//! This whole module uses unsigned integer accumulation. No floating-point
//! type appears anywhere in it. An exact integer sum has one answer, so no
//! lane width and no summation order can change a block score; a
//! floating-point accumulator would reopen exactly the non-associative
//! summation risk this project's determinism proof exists to close.

/// A summed-area table over a single-byte-per-pixel image, answering any
/// axis-aligned rectangle's sum in four table reads.
///
/// Crow, "Summed-Area Tables for Texture Mapping," SIGGRAPH (1984)
/// introduced this construction. Viola and Jones, "Rapid Object Detection
/// using a Boosted Cascade of Simple Features," CVPR (2001) popularized the
/// same structure under the name "integral image," which is the name most
/// computer-vision code uses today.
pub struct IntegralImage {
    width: usize,
    height: usize,
    /// The table, with one extra row and one extra column of zeros, so a
    /// rectangle at the image edge needs no bounds branch. `table[y][x]`
    /// holds the sum of every sample in `0..y, 0..x` of the source image.
    table: Vec<u64>,
}

impl IntegralImage {
    /// Build a summed-area table over `samples`, a `width` by `height`
    /// grid of single-byte samples, row major.
    ///
    /// Each row's running sum is added into the row above's own table
    /// entry for that column, so the whole table is built in one pass,
    /// with no floating-point type and no division anywhere in it.
    pub fn from_luma(samples: &[u8], width: usize, height: usize) -> Self {
        debug_assert_eq!(
            samples.len(),
            width * height,
            "from_luma requires exactly width * height samples"
        );
        let stride = width + 1;
        let mut table = vec![0u64; stride * (height + 1)];
        for y in 0..height {
            let mut row_sum: u64 = 0;
            let above_row_start = y * stride;
            let this_row_start = (y + 1) * stride;
            for x in 0..width {
                row_sum += u64::from(samples[y * width + x]);
                table[this_row_start + x + 1] = table[above_row_start + x + 1] + row_sum;
            }
        }
        Self {
            width,
            height,
            table,
        }
    }

    /// Return the sum of the `w` by `h` rectangle whose top-left corner is
    /// `(x, y)`, in exactly four table reads and three integer operations,
    /// whatever `w` and `h` are.
    ///
    /// The caller keeps the rectangle inside the image; a debug build
    /// asserts that.
    pub fn window_sum(&self, x: usize, y: usize, w: usize, h: usize) -> u64 {
        debug_assert!(
            x + w <= self.width && y + h <= self.height,
            "window_sum rectangle must lie inside the image"
        );
        let stride = self.width + 1;
        let x0 = x;
        let y0 = y;
        let x1 = x + w;
        let y1 = y + h;
        self.table[y1 * stride + x1] + self.table[y0 * stride + x0]
            - self.table[y0 * stride + x1]
            - self.table[y1 * stride + x0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small, fixed integer sequence generator (xorshift32), seeded with
    /// a fixed constant, never with the clock, so a red result is
    /// reproducible. This is not a cryptographic generator and is never
    /// used for anything but picking test rectangles.
    struct FixedSequence {
        state: u32,
    }

    impl FixedSequence {
        fn new(seed: u32) -> Self {
            Self { state: seed.max(1) }
        }

        fn next_u32(&mut self) -> u32 {
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.state = x;
            x
        }

        fn next_below(&mut self, bound: usize) -> usize {
            (self.next_u32() as usize) % bound.max(1)
        }
    }

    fn direct_sum(samples: &[u8], width: usize, x: usize, y: usize, w: usize, h: usize) -> u64 {
        let mut total = 0u64;
        for row in y..y + h {
            for col in x..x + w {
                total += u64::from(samples[row * width + col]);
            }
        }
        total
    }

    #[test]
    fn window_sum_matches_a_direct_sum_over_twenty_random_rectangles() {
        let width = 37;
        let height = 29;
        let mut sequence = FixedSequence::new(0xC0FF_EE01);
        let samples: Vec<u8> = (0..width * height)
            .map(|_| (sequence.next_u32() % 256) as u8)
            .collect();
        let table = IntegralImage::from_luma(&samples, width, height);

        for _ in 0..20 {
            let w = 1 + sequence.next_below(width);
            let h = 1 + sequence.next_below(height);
            let x = sequence.next_below(width - w + 1);
            let y = sequence.next_below(height - h + 1);
            assert_eq!(
                table.window_sum(x, y, w, h),
                direct_sum(&samples, width, x, y, w, h),
                "mismatch at x={x} y={y} w={w} h={h}"
            );
        }
    }

    #[test]
    fn window_sum_over_one_pixel_equals_that_pixel() {
        let width = 4;
        let height = 4;
        let samples: Vec<u8> = vec![
            1, 2, 3, 4, //
            5, 6, 7, 8, //
            9, 10, 11, 12, //
            13, 14, 15, 16,
        ];
        let table = IntegralImage::from_luma(&samples, width, height);
        for y in 0..height {
            for x in 0..width {
                assert_eq!(
                    table.window_sum(x, y, 1, 1),
                    u64::from(samples[y * width + x])
                );
            }
        }
    }

    #[test]
    fn window_sum_over_the_whole_image_equals_the_total() {
        let width = 6;
        let height = 5;
        let samples: Vec<u8> = (0..width * height).map(|i| (i % 250) as u8).collect();
        let table = IntegralImage::from_luma(&samples, width, height);
        let expected: u64 = samples.iter().map(|&b| u64::from(b)).sum();
        assert_eq!(table.window_sum(0, 0, width, height), expected);
    }

    #[test]
    fn a_maximum_sized_image_of_the_largest_byte_value_does_not_overflow_the_accumulator() {
        // A width and height chosen so the total, at the largest byte
        // value, exceeds u32::MAX: 4200 * 4200 * 255 = 4_498_200_000, while
        // u32::MAX is 4_294_967_295. This proves the accumulator holds a
        // value a narrower integer could not, without allocating gigabytes
        // of memory to reach the real decode-time ceiling. The real
        // ceiling (chrys-source-raster's DecodeLimits::default(), 16384 by
        // 16384) scales the same total linearly to about 6.87e10, still
        // over eight orders of magnitude below u64::MAX (about 1.8e19), so
        // this smaller size proves the same property the real ceiling
        // would.
        let width = 4200usize;
        let height = 4200usize;
        let samples = vec![u8::MAX; width * height];
        let table = IntegralImage::from_luma(&samples, width, height);
        let expected = width as u64 * height as u64 * u64::from(u8::MAX);
        assert!(expected > u64::from(u32::MAX));
        assert_eq!(table.window_sum(0, 0, width, height), expected);
    }
}
