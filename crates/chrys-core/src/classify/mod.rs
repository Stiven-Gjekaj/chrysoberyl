//! Turn a residual field into a verdict a person can read.
//!
//! This module groups changed pixels into labelled regions, drops a
//! difference that is only antialiasing, computes a colour difference
//! through a pure-Rust colour space, and names every remaining region by
//! kind. Every child module owns exactly one of those steps.

pub mod antialias;
pub mod colour;
pub mod kind;
pub mod label;

pub use antialias::{is_antialiasing, suppress_antialiasing};
pub use colour::colour_delta;
pub use kind::classify_kind;
pub use label::{LabelledRegion, RESIDUAL_THRESHOLD, label_regions};
