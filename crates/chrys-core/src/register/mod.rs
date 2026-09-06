//! The global registration stage: reduce a pair to a fixed working grid,
//! correlate it in the frequency domain on one arithmetic path, and report
//! the translation between the pair in pixels, refined below one pixel.
//!
//! This stage produces an offset, never a verdict. The engine assumes
//! near-identical pairs, so phase correlation plus a later block match is
//! the whole registration scope here: no feature matching, no homography,
//! no optical flow.

pub mod luma;
pub mod phase_correlation;
pub mod subpixel;
pub mod window;

pub use luma::to_luma_downsampled;
pub use phase_correlation::{CoarseOffset, CorrelationSurface, phase_correlate};
pub use subpixel::{RefinedOffset, refine_peak};
pub use window::{WORKING_RESOLUTION, hann_table, plan_scalar_fft};
