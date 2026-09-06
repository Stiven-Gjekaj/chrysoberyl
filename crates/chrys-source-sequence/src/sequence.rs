//! The directory listing, the natural-order sort, and the per-file decode
//! for a numbered frame sequence.
//!
//! This module opens no `image::ImageReader` of its own. The one decode
//! path it has is `chrys_source_raster::decode::decode_guarded`, followed
//! by `chrys_source_raster::normalize::normalize_to_rgba8`.

use std::path::{Path, PathBuf};

use chrys_source::Frame;
use chrys_source_raster::DecodeLimits;
use chrys_source_raster::decode::decode_guarded;
use chrys_source_raster::normalize::normalize_to_rgba8;

use crate::{SequenceError, SequenceLimits};

/// List the immediate entries of `dir` whose metadata reports a regular
/// file, sorted by `natord::compare` over the file name unconditionally.
///
/// This never recurses into a nested directory, and a device node or a
/// symlink to a directory in `dir` cannot enter the returned list.
pub(crate) fn list_sorted(dir: &Path) -> Result<Vec<PathBuf>, SequenceError> {
    let entries = std::fs::read_dir(dir).map_err(|source| SequenceError::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| SequenceError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let metadata = entry.metadata().map_err(|source| SequenceError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        if metadata.is_file() {
            paths.push(entry.path());
        }
    }

    paths.sort_by(|a, b| {
        let a_name = a.file_name().and_then(|name| name.to_str()).unwrap_or("");
        let b_name = b.file_name().and_then(|name| name.to_str()).unwrap_or("");
        natord::compare(a_name, b_name)
    });

    Ok(paths)
}

/// Decode every file in `paths`, in order, through
/// `chrys_source_raster::decode::decode_guarded`, checking `limits`
/// against the accumulated frame count and pixel count as each file
/// decodes, not after the whole directory has loaded.
///
/// A file that does not decode fails the whole load and names itself; it
/// is never skipped, so a planted unreadable file cannot silently shorten
/// a sequence and change which indices pair.
pub(crate) fn decode_all(
    dir: &Path,
    paths: &[PathBuf],
    limits: &SequenceLimits,
    decode_limits: &DecodeLimits,
) -> Result<Vec<Frame>, SequenceError> {
    if paths.is_empty() {
        return Err(SequenceError::Empty {
            path: dir.to_path_buf(),
        });
    }
    if paths.len() > limits.max_frames {
        return Err(SequenceError::TooManyFrames {
            path: dir.to_path_buf(),
            count: paths.len(),
            limit: limits.max_frames,
        });
    }

    let mut frames = Vec::with_capacity(paths.len());
    let mut total_pixels: usize = 0;
    for (index, path) in paths.iter().enumerate() {
        let (dynamic, orientation) =
            decode_guarded(path, decode_limits).map_err(|source| SequenceError::Decode {
                file: path.clone(),
                source,
            })?;
        let (pixels, width, height) = normalize_to_rgba8(dynamic, orientation);

        total_pixels = total_pixels.saturating_add((width as usize) * (height as usize));
        if total_pixels > limits.max_total_pixels {
            return Err(SequenceError::TooManyPixels {
                path: dir.to_path_buf(),
                pixels: total_pixels,
                limit: limits.max_total_pixels,
            });
        }

        frames.push(Frame {
            pixels,
            width,
            height,
            index,
            hints: Vec::new(),
        });
    }

    Ok(frames)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    #[test]
    fn sorts_by_natural_order() {
        assert_eq!(natord::compare("frame2.png", "frame10.png"), Ordering::Less);

        let mut names = vec![
            "frame3.png",
            "frame1.png",
            "frame10.png",
            "frame11.png",
            "frame2.png",
            "frame9.png",
            "frame4.png",
            "frame8.png",
            "frame5.png",
            "frame7.png",
            "frame6.png",
        ];
        names.sort_by(|a, b| natord::compare(a, b));
        assert_eq!(
            names,
            vec![
                "frame1.png",
                "frame2.png",
                "frame3.png",
                "frame4.png",
                "frame5.png",
                "frame6.png",
                "frame7.png",
                "frame8.png",
                "frame9.png",
                "frame10.png",
                "frame11.png",
            ]
        );
    }
}
