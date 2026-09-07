//! The baseline store. A store resolves a name to a path, and it accepts a
//! new candidate under a name. It knows no pixel format and no verdict: it
//! is a store of digests and files, never a place that decodes an image or
//! judges a comparison.
//!
//! This crate depends on neither `chrys-core` nor `chrys-source`. Keeping
//! that true is what makes "a second baseline backend needs no change to
//! the engine" a structural fact rather than a hope: `cargo tree -p
//! chrys-baseline -e normal` names neither crate.

#![forbid(unsafe_code)]

pub mod golden;

use std::path::{Path, PathBuf};

/// A baseline store: resolve an accepted name to a path, and accept a new
/// candidate under a name.
///
/// Exactly two methods. A third would be a place for a backend to differ
/// in a way a caller would then have to know about.
pub trait BaselineStore {
    /// The error this store's own methods return.
    type Error;

    /// Return the path of the currently accepted baseline named `name`.
    ///
    /// This method is read only by contract: no implementation may create,
    /// write to, or remove anything while resolving a name. A name that
    /// was never accepted is an error naming the name and the store's own
    /// root, never a first silent accept.
    fn resolve(&self, name: &str) -> Result<PathBuf, Self::Error>;

    /// Record `frame_digests` as the new baseline for `name`, and copy
    /// `candidate_path`, a file or a directory, into the store.
    ///
    /// Each pair in `frame_digests` is a source file name and a
    /// lowercase-hexadecimal digest, in frame order. The caller has
    /// already decoded and already hashed; this method decodes nothing of
    /// its own.
    fn accept(
        &self,
        name: &str,
        candidate_path: &Path,
        frame_digests: &[(String, String)],
    ) -> Result<(), Self::Error>;
}
