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
/// the first element the prefix opens, after any leading whitespace, XML
/// comment, XML declaration and `<!DOCTYPE ...>` are skipped, is named
/// `svg` (optionally with a namespace prefix, such as `svg:svg`). A
/// binary raster file cannot reach this check: a PNG's first byte is
/// `0x89` and a GIF's is `G`, so neither file opens with a `<` at all.
///
/// The match is on the root element's own tag name, not on the raw
/// four-byte sequence `<svg` appearing anywhere in the prefix, so a
/// well-formed, non-SVG document whose root or a descendant element is
/// merely named with that prefix (`<svgProfile>`, for example) is
/// correctly refused.
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

    Ok(carries_svg_root(bytes))
}

/// Return the index of the first occurrence of `needle` in `haystack`, or
/// `None` when `needle` does not occur within the bound `haystack` already
/// carries.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Return true when the first real element `bytes` opens is named `svg`,
/// with an optional namespace prefix.
///
/// Leading whitespace, an XML comment (`<!-- ... -->`), an XML
/// declaration (`<?xml ... ?>`) and a `<!DOCTYPE ...>` are all skipped
/// before the check, because a real SVG document may carry any of them
/// before its root element, and this crate's own fixtures do (a one-line
/// comment naming why the header is kept short). A construct that is not
/// closed within `bytes` (because it runs past the bounded prefix
/// `looks_like_svg` read) is treated as not carrying an SVG root, since
/// the prefix does not hold enough to prove otherwise.
fn carries_svg_root(bytes: &[u8]) -> bool {
    let mut rest = bytes;
    loop {
        rest = skip_ascii_whitespace(rest);
        if let Some(after) = rest.strip_prefix(b"<!--") {
            match find_subslice(after, b"-->") {
                Some(end) => {
                    rest = &after[end + 3..];
                    continue;
                }
                None => return false,
            }
        }
        if let Some(after) = rest.strip_prefix(b"<?") {
            match find_subslice(after, b"?>") {
                Some(end) => {
                    rest = &after[end + 2..];
                    continue;
                }
                None => return false,
            }
        }
        if let Some(after) = rest.strip_prefix(b"<!") {
            match after.iter().position(|byte| *byte == b'>') {
                Some(end) => {
                    rest = &after[end + 1..];
                    continue;
                }
                None => return false,
            }
        }
        break;
    }

    let Some(after_angle_bracket) = rest.strip_prefix(b"<") else {
        return false;
    };
    let name_end = after_angle_bracket
        .iter()
        .position(|byte| !is_xml_name_byte(*byte))
        .unwrap_or(after_angle_bracket.len());
    let tag_name = &after_angle_bracket[..name_end];

    tag_name == b"svg" || tag_name.ends_with(b":svg")
}

/// Return `bytes` with every leading ASCII whitespace byte removed.
fn skip_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let first_non_whitespace = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[first_non_whitespace..]
}

/// Return true when `byte` may appear in an XML element name. This is a
/// deliberately narrow subset of the XML name-character grammar: ASCII
/// letters, digits, `:`, `-` and `_` cover every tag name this sniff must
/// recognise, and being narrow rather than exhaustive is the safer
/// direction for a security-relevant content check.
fn is_xml_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b':' || byte == b'-' || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A well-formed, non-SVG XML document whose only relation to `<svg`
    /// is that a descendant element's name starts with those four bytes.
    /// This is the exact case WR-01 named: the old implementation
    /// searched the whole prefix for the raw substring `<svg` and
    /// answered true here, even though the document's root element is
    /// `mydoc`, not `svg`.
    #[test]
    fn a_non_svg_document_naming_svg_only_as_a_longer_tag_prefix_is_rejected() {
        let xml = br#"<mydoc xmlns="urn:example">
  <note>this is not an svg document but mentions svg-icon and <svgProfile>data</svgProfile></note>
</mydoc>
"#;
        assert!(
            !carries_svg_root(xml),
            "a document whose root is `mydoc` and whose only `<svg` occurrence is the prefix of \
             the longer tag name `svgProfile` must not be sniffed as an SVG document"
        );
    }

    /// The positive case this crate's own committed fixtures rely on: a
    /// leading comment, then the real `<svg` root element.
    #[test]
    fn an_svg_document_with_a_leading_comment_is_accepted() {
        let xml = br#"<!-- keep this header short -->
<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"></svg>
"#;
        assert!(
            carries_svg_root(xml),
            "a leading comment must not stop the sniff from finding the real `<svg` root that \
             follows it"
        );
    }

    /// A namespace-prefixed root element is still an SVG root.
    #[test]
    fn a_namespace_prefixed_svg_root_is_accepted() {
        let xml =
            br#"<svg:svg xmlns:svg="http://www.w3.org/2000/svg" width="1" height="1"></svg:svg>"#;
        assert!(
            carries_svg_root(xml),
            "a namespace-prefixed root element named `svg:svg` must still be recognised as an \
             SVG root"
        );
    }

    /// The root element name must match exactly, not merely start with
    /// `svg`: a root named `svgProfile` is a different element.
    #[test]
    fn a_root_element_merely_starting_with_svg_is_rejected() {
        let xml = br#"<svgProfile xmlns="urn:example"></svgProfile>"#;
        assert!(
            !carries_svg_root(xml),
            "a root element named `svgProfile` must not be mistaken for an `svg` root"
        );
    }
}
