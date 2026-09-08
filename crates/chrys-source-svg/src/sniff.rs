//! The one content sniff that decides whether a file is an SVG document.
//!
//! `looks_like_svg` reads a bounded prefix, never the whole file: a sniff
//! that reads its whole input to decide whether to read its whole input
//! is itself the decompression-bomb surface every other adapter in this
//! workspace already guards against.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::SvgError;

/// The largest prefix, in bytes, `looks_like_svg` reads from any file.
const SNIFF_PREFIX_BYTES: u64 = 1024;

/// The UTF-8 byte-order mark, skipped when present before the sniff looks
/// at the prefix's first meaningful byte.
const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Return true when the file at `path` is an SVG document.
///
/// The decision is made on content, never on the file's extension. It
/// reads at most `SNIFF_PREFIX_BYTES` bytes through a bounded read, skips
/// a UTF-8 byte-order mark if one is present, then answers true only when
/// both of two things hold over what remains: the first byte that is not
/// ASCII whitespace is a less-than sign, and the prefix contains the four
/// characters that open an SVG root element (`<svg`). The first condition
/// is what keeps a binary raster file out: a PNG's first byte is `0x89`
/// and a GIF's is `G`, so neither can reach the second condition by
/// accident.
///
/// An I/O error propagates as `SvgError`; it is never silently read as
/// false.
pub fn looks_like_svg(path: &Path) -> Result<bool, SvgError> {
    let mut file = File::open(path).map_err(|source| SvgError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let mut prefix = Vec::new();
    file.by_ref()
        .take(SNIFF_PREFIX_BYTES)
        .read_to_end(&mut prefix)
        .map_err(|source| SvgError::Io {
            path: path.to_path_buf(),
            source,
        })?;

    let bytes = prefix
        .strip_prefix(&UTF8_BOM[..])
        .unwrap_or(prefix.as_slice());

    let first_non_whitespace = bytes.iter().find(|byte| !byte.is_ascii_whitespace());
    let starts_with_angle_bracket = matches!(first_non_whitespace, Some(b'<'));
    let carries_svg_root = bytes.windows(4).any(|window| window == b"<svg");

    Ok(starts_with_angle_bracket && carries_svg_root)
}
