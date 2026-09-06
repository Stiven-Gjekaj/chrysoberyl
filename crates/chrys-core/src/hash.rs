//! The digest contract. The digest covers raw RGBA8 bytes as they sit in
//! memory. The digest never covers a re-encoded file, because an encoder
//! is a second variable. The digest never covers a path, a timestamp or a
//! host name, because those are not properties of the comparison. The byte
//! order is the `Frame` row-major, straight-alpha layout. Any change to
//! this contract invalidates every committed digest in the repository.

use std::fmt;

use sha2::{Digest, Sha256};

use chrys_source::Frame;

use crate::CompareError;
use crate::residual::residual_rgba8;
use crate::verdict::Verdict;

/// Return the SHA-256 of `bytes`, as 64 lowercase hexadecimal characters.
///
/// An empty slice returns the digest of the empty input, not a panic. The
/// same slice always returns the same digest.
pub fn rgba8_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in result {
        use fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// The four digests one comparison produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestSet {
    /// The digest of the base frame's raw RGBA8 bytes.
    pub decode_base: String,
    /// The digest of the candidate frame's raw RGBA8 bytes.
    pub decode_candidate: String,
    /// The digest of the residual buffer between the two frames.
    pub residual: String,
    /// The digest of the verdict's `Display` text, encoded as UTF-8.
    pub verdict: String,
}

impl fmt::Display for DigestSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "decode-base {}", self.decode_base)?;
        writeln!(f, "decode-candidate {}", self.decode_candidate)?;
        writeln!(f, "residual {}", self.residual)?;
        writeln!(f, "verdict {}", self.verdict)
    }
}

/// Build the digest set for one comparison.
///
/// This does not run `compare` itself. The caller runs `compare` first and
/// hands the resulting verdict in, so the digest always covers the exact
/// verdict the caller reported.
pub fn digest_report(
    base: &Frame,
    candidate: &Frame,
    verdict: &Verdict,
) -> Result<DigestSet, CompareError> {
    let residual = residual_rgba8(base, candidate)?;
    Ok(DigestSet {
        decode_base: rgba8_digest(base.rgba8()),
        decode_candidate: rgba8_digest(candidate.rgba8()),
        residual: rgba8_digest(&residual),
        verdict: rgba8_digest(verdict.to_string().as_bytes()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::Verdict;

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
    fn rgba8_digest_on_an_empty_slice_returns_the_empty_input_digest() {
        let digest = rgba8_digest(&[]);
        assert_eq!(
            digest,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn rgba8_digest_is_stable_across_repeated_calls() {
        let bytes = [1u8, 2, 3, 4, 5];
        assert_eq!(rgba8_digest(&bytes), rgba8_digest(&bytes));
    }

    #[test]
    fn digest_report_returns_a_display_form_with_four_lines_in_order() {
        let base = solid_frame(2, 2, [10, 20, 30, 255]);
        let candidate = solid_frame(2, 2, [10, 20, 30, 255]);
        let set = digest_report(&base, &candidate, &Verdict::Identical).unwrap();
        let text = set.to_string();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].starts_with("decode-base "));
        assert!(lines[1].starts_with("decode-candidate "));
        assert!(lines[2].starts_with("residual "));
        assert!(lines[3].starts_with("verdict "));
        for line in lines {
            let digest = line.split(' ').nth(1).expect("a digest field");
            assert_eq!(digest.len(), 64);
            assert!(
                digest
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            );
        }
    }

    #[test]
    fn digest_report_on_identical_frames_gives_a_residual_digest_of_the_all_zero_channel_buffer() {
        let base = solid_frame(2, 2, [10, 20, 30, 255]);
        let candidate = solid_frame(2, 2, [10, 20, 30, 255]);
        let residual = residual_rgba8(&base, &candidate).unwrap();
        let set = digest_report(&base, &candidate, &Verdict::Identical).unwrap();
        assert_eq!(set.residual, rgba8_digest(&residual));
    }
}
