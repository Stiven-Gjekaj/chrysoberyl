//! The numbered frame sequence adapter.
//!
//! This is the only crate that knows a directory of numbered frames is a
//! sequence. It depends on `chrys-source` for the `Frame` and `Source`
//! shapes and on `chrys-source-raster` for the one guarded decode entry
//! point. It opens no `image::ImageReader` of its own; every file this
//! crate reads decodes through `chrys_source_raster::decode::decode_guarded`.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use chrys_source::{Frame, Source};
use chrys_source_raster::{DecodeLimits, RasterError};

mod sequence;

/// Resource limits applied to a sequence load, so a directory holding many
/// small files cannot do what a single oversized file already cannot.
///
/// `max_total_pixels` is the 512 MiB `DecodeLimits::default().max_alloc`
/// divided by the four bytes an RGBA8 pixel occupies, checked as frames
/// accumulate rather than after the whole directory has decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceLimits {
    /// The largest number of frames this crate will load from one
    /// directory.
    pub max_frames: usize,
    /// The largest cumulative pixel count, summed across every frame, this
    /// crate will load from one directory.
    pub max_total_pixels: usize,
}

impl Default for SequenceLimits {
    fn default() -> Self {
        SequenceLimits {
            max_frames: 512,
            max_total_pixels: 134_217_728,
        }
    }
}

/// A `Source` that decodes a directory of numbered frame files into a
/// `Vec<Frame>`, ordered by the natural sort of their file names.
pub struct SequenceSource {
    limits: SequenceLimits,
    decode_limits: DecodeLimits,
}

impl SequenceSource {
    /// Build a `SequenceSource` with the default sequence and decode
    /// limits.
    pub fn new() -> Self {
        SequenceSource {
            limits: SequenceLimits::default(),
            decode_limits: DecodeLimits::default(),
        }
    }

    /// Build a `SequenceSource` with the given sequence limits, and the
    /// default decode limits.
    pub fn with_limits(limits: SequenceLimits) -> Self {
        SequenceSource {
            limits,
            decode_limits: DecodeLimits::default(),
        }
    }
}

impl Default for SequenceSource {
    fn default() -> Self {
        Self::new()
    }
}

/// The error a `SequenceSource` returns on a failed load.
#[derive(Debug, thiserror::Error)]
pub enum SequenceError {
    /// The directory could not be read from disk.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The directory that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// The directory holds no file.
    #[error("{path} holds no file")]
    Empty {
        /// The directory that held no file.
        path: PathBuf,
    },
    /// The directory holds more files than `SequenceLimits::max_frames`
    /// allows.
    #[error("{path} holds {count} frames, which exceeds the limit of {limit} frames")]
    TooManyFrames {
        /// The directory that exceeded the frame count limit.
        path: PathBuf,
        /// The number of files the directory held.
        count: usize,
        /// The limit that was exceeded.
        limit: usize,
    },
    /// The cumulative pixel count across every frame exceeds
    /// `SequenceLimits::max_total_pixels`.
    #[error("{path} holds {pixels} pixels total, which exceeds the limit of {limit} pixels")]
    TooManyPixels {
        /// The directory that exceeded the pixel budget.
        path: PathBuf,
        /// The cumulative pixel count reached when the limit was crossed.
        pixels: usize,
        /// The limit that was exceeded.
        limit: usize,
    },
    /// One file in the directory did not decode.
    #[error("cannot decode {file}: {source}")]
    Decode {
        /// The file that did not decode.
        file: PathBuf,
        /// The underlying decode error.
        source: RasterError,
    },
    /// One file's own hints sidecar did not read.
    #[error("cannot read hints for {file}: {source}")]
    Hint {
        /// The frame file whose sidecar failed.
        file: PathBuf,
        /// The underlying sidecar error.
        source: RasterError,
    },
}

impl Source for SequenceSource {
    type Error = SequenceError;

    fn load(&self, path: &Path) -> Result<Vec<Frame>, Self::Error> {
        let named = self.load_named(path)?;
        Ok(named.into_iter().map(|(_name, frame)| frame).collect())
    }

    /// List `path` once, decode once, and return each frame beside its
    /// own file's name, in the same order `load` returns the frames.
    ///
    /// `load` is implemented in terms of this method, not the other way
    /// round, so there is exactly one listing of the directory and one
    /// decode pass. Two independent computations of the same sorted order
    /// could drift from each other, and the pairing between a frame and
    /// its name is precisely what must not drift (T-03-17).
    fn load_named(&self, path: &Path) -> Result<Vec<(String, Frame)>, Self::Error> {
        let paths = sequence::list_sorted(path)?;
        let names: Vec<String> = paths
            .iter()
            .map(|file_path| {
                file_path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| file_path.display().to_string())
            })
            .collect();
        let frames = sequence::decode_all(path, &paths, &self.limits, &self.decode_limits)?;
        Ok(names.into_iter().zip(frames).collect())
    }
}
