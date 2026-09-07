//! The `<stem>.hints.toml` sidecar reader.
//!
//! A hint is metadata a producer supplies next to the image file it
//! describes. This is the only place in the workspace that derives
//! `serde::Deserialize`, and it derives on structs private to this file,
//! never on `chrys_source::RegionHint` itself: `chrys-source` is a
//! dependency of `chrys-core`, so deriving there would put `serde` in the
//! engine's dependency graph for a parser the engine never reads.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrys_source::RegionHint;
use serde::Deserialize;

use crate::RasterError;

/// The largest `<stem>.hints.toml` this reader accepts, in bytes.
///
/// A sidecar names rectangles. The committed example names one region in
/// under a hundred bytes, and a producer that names a thousand regions
/// stays far below this number. One mebibyte is therefore a ceiling no
/// real sidecar reaches, which is what makes it safe to refuse above it
/// rather than read and hope the parser fails first.
///
/// This is the same posture `DecodeLimits` takes on the image path:
/// bound the allocation before it happens, because the file comes from
/// wherever the input file came from.
pub const MAX_SIDECAR_BYTES: u64 = 1024 * 1024;

/// The sidecar's own document shape: an array of tables named `hint`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HintsDocument {
    /// Every named region the sidecar declares. Absent entirely when the
    /// document holds no `[[hint]]` table.
    #[serde(rename = "hint", default)]
    hint: Vec<HintRow>,
}

/// One `[[hint]]` table, deserialized before it is converted into
/// `chrys_source::RegionHint`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HintRow {
    name: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl From<HintRow> for RegionHint {
    fn from(row: HintRow) -> Self {
        RegionHint {
            name: row.name,
            x: row.x,
            y: row.y,
            width: row.width,
            height: row.height,
        }
    }
}

/// Read the `<stem>.hints.toml` sidecar next to `image_path`, if one
/// exists.
///
/// A missing sidecar is not an error: most sources supply no hint, so this
/// returns an empty vector. A present sidecar is parsed with `toml`, with
/// `deny_unknown_fields` set on both structs above, so a misspelled key is
/// a loud failure rather than a hint that silently does not exist. A
/// sidecar naming the same region twice is refused: a hint is looked up by
/// name, and two answers to one name is a question this function cannot
/// answer.
pub fn read_hints_sidecar(image_path: &Path) -> Result<Vec<RegionHint>, RasterError> {
    let sidecar_path = sidecar_path_for(image_path);

    // Ask the file system for the size before reading the bytes. A read
    // that starts before the size is known puts the whole file in memory
    // whatever the parser later decides, which is the shape of the
    // decompression-bomb class `DecodeLimits` already defends the image
    // path against. The sidecar crosses the same boundary: it sits beside
    // an input file, and whoever supplies the input supplies it too.
    match std::fs::metadata(&sidecar_path) {
        Ok(metadata) if metadata.len() > MAX_SIDECAR_BYTES => {
            return Err(RasterError::HintsTooLarge {
                path: sidecar_path,
                size: metadata.len(),
                limit: MAX_SIDECAR_BYTES,
            });
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(RasterError::Io {
                path: sidecar_path,
                source: error,
            });
        }
    }

    let text = match std::fs::read_to_string(&sidecar_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(RasterError::Io {
                path: sidecar_path,
                source: error,
            });
        }
    };

    let document: HintsDocument =
        toml::from_str(&text).map_err(|error| RasterError::MalformedHints {
            path: sidecar_path.clone(),
            message: error.to_string(),
        })?;

    let mut seen: HashSet<String> = HashSet::new();
    let mut hints = Vec::with_capacity(document.hint.len());
    for row in document.hint {
        if !seen.insert(row.name.clone()) {
            return Err(RasterError::DuplicateHintName {
                path: sidecar_path,
                name: row.name,
            });
        }
        hints.push(RegionHint::from(row));
    }

    Ok(hints)
}

/// The sidecar path for `image_path`: its own directory, its own file
/// stem, with `.hints.toml` appended.
fn sidecar_path_for(image_path: &Path) -> PathBuf {
    let stem = image_path.file_stem().unwrap_or_default();
    let mut name = stem.to_os_string();
    name.push(".hints.toml");
    image_path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_sidecar_returns_an_empty_vector() {
        let path = Path::new("/no/such/directory/chrys-hints-test-missing.png");
        let hints = read_hints_sidecar(path).expect("a missing sidecar is not an error");
        assert!(hints.is_empty());
    }

    #[test]
    fn a_present_sidecar_parses_into_region_hints() {
        let dir = std::env::temp_dir();
        let image_path = dir.join("chrys-hints-test-present.png");
        let sidecar_path = dir.join("chrys-hints-test-present.hints.toml");
        std::fs::write(
            &sidecar_path,
            "[[hint]]\nname = \"logo\"\nx = 10\ny = 20\nwidth = 30\nheight = 40\n",
        )
        .expect("write sidecar");

        let result = read_hints_sidecar(&image_path);
        std::fs::remove_file(&sidecar_path).ok();

        let hints = result.expect("a well-formed sidecar parses");
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].name, "logo");
        assert_eq!(hints[0].x, 10);
        assert_eq!(hints[0].y, 20);
        assert_eq!(hints[0].width, 30);
        assert_eq!(hints[0].height, 40);
    }

    /// Write `contents` to a fresh temporary sidecar and image path pair,
    /// call `read_hints_sidecar` on the image path, and return the result
    /// with the sidecar removed. `label` keeps every temporary file this
    /// test module writes from colliding with another test's own file.
    fn read_temp_sidecar(label: &str, contents: &str) -> Result<Vec<RegionHint>, RasterError> {
        let dir = std::env::temp_dir();
        let image_path = dir.join(format!("chrys-hints-test-{label}.png"));
        let sidecar_path = dir.join(format!("chrys-hints-test-{label}.hints.toml"));
        std::fs::write(&sidecar_path, contents).expect("write sidecar");
        let result = read_hints_sidecar(&image_path);
        std::fs::remove_file(&sidecar_path).ok();
        result
    }

    #[test]
    fn invalid_toml_fails_with_a_message_naming_the_line() {
        let error = read_temp_sidecar("invalid-toml", "[[hint]\nname = \"logo\"\n")
            .expect_err("malformed TOML is refused");
        match error {
            RasterError::MalformedHints { message, .. } => {
                assert!(
                    message.contains("line"),
                    "message does not name a line: {message}"
                );
            }
            other => panic!("expected RasterError::MalformedHints, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_key_fails_rather_than_being_ignored() {
        let error = read_temp_sidecar(
            "unknown-key",
            "[[hint]]\nname = \"logo\"\nw = 10\ny = 20\nwidth = 30\nheight = 40\nx = 5\n",
        )
        .expect_err("an unknown key is refused, not silently dropped");
        match error {
            RasterError::MalformedHints { message, .. } => {
                assert!(
                    message.contains('w'),
                    "message does not name the unknown field: {message}"
                );
            }
            other => panic!("expected RasterError::MalformedHints, got {other:?}"),
        }
    }

    #[test]
    fn a_negative_coordinate_fails_with_a_message_naming_the_field() {
        let error = read_temp_sidecar(
            "negative-coordinate",
            "[[hint]]\nname = \"logo\"\nx = -5\ny = 20\nwidth = 30\nheight = 40\n",
        )
        .expect_err("a negative coordinate does not fit a u32 field");
        match error {
            RasterError::MalformedHints { message, .. } => {
                assert!(
                    message.contains('x'),
                    "message does not name the field: {message}"
                );
            }
            other => panic!("expected RasterError::MalformedHints, got {other:?}"),
        }
    }

    #[test]
    fn a_non_integer_coordinate_fails_with_a_message_naming_the_field() {
        let error = read_temp_sidecar(
            "non-integer-coordinate",
            "[[hint]]\nname = \"logo\"\nx = \"ten\"\ny = 20\nwidth = 30\nheight = 40\n",
        )
        .expect_err("a string does not fit a u32 field");
        match error {
            RasterError::MalformedHints { message, .. } => {
                assert!(
                    message.contains('x'),
                    "message does not name the field: {message}"
                );
            }
            other => panic!("expected RasterError::MalformedHints, got {other:?}"),
        }
    }

    #[test]
    fn a_sidecar_larger_than_the_limit_is_refused_before_it_is_read() {
        let dir = std::env::temp_dir();
        let image_path = dir.join("chrys-hints-test-oversize.png");
        let sidecar_path = dir.join("chrys-hints-test-oversize.hints.toml");

        // One byte over the limit is enough. The test does not need a
        // large file to prove the check runs, and a large file would make
        // this test pay the cost the check exists to refuse.
        let oversize = (MAX_SIDECAR_BYTES + 1) as usize;
        let mut document = String::with_capacity(oversize);
        document.push('#');
        while document.len() < oversize {
            document.push('a');
        }
        std::fs::write(&sidecar_path, &document).expect("write sidecar");
        let written = std::fs::metadata(&sidecar_path)
            .expect("stat sidecar")
            .len();

        let result = read_hints_sidecar(&image_path);
        std::fs::remove_file(&sidecar_path).ok();

        assert!(
            written > MAX_SIDECAR_BYTES,
            "the fixture must exceed the limit to test it: {written} bytes"
        );
        match result.expect_err("a sidecar over the limit is refused") {
            RasterError::HintsTooLarge { size, limit, .. } => {
                assert_eq!(size, written);
                assert_eq!(limit, MAX_SIDECAR_BYTES);
            }
            other => panic!("expected RasterError::HintsTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn a_sidecar_at_the_limit_is_still_read() {
        // The boundary belongs to the accepted side. A test that only
        // proves the refusal cannot tell a correct limit from one that
        // refuses every sidecar.
        let dir = std::env::temp_dir();
        let image_path = dir.join("chrys-hints-test-at-limit.png");
        let sidecar_path = dir.join("chrys-hints-test-at-limit.hints.toml");

        let hint = "[[hint]]\nname = \"logo\"\nx = 10\ny = 20\nwidth = 30\nheight = 40\n";
        let mut document = String::from(hint);
        while document.len() < MAX_SIDECAR_BYTES as usize {
            document.push('#');
        }
        document.truncate(MAX_SIDECAR_BYTES as usize);
        std::fs::write(&sidecar_path, &document).expect("write sidecar");
        let written = std::fs::metadata(&sidecar_path)
            .expect("stat sidecar")
            .len();

        let result = read_hints_sidecar(&image_path);
        std::fs::remove_file(&sidecar_path).ok();

        assert_eq!(written, MAX_SIDECAR_BYTES);
        let hints = result.expect("a sidecar exactly at the limit is read");
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].name, "logo");
    }

    #[test]
    fn a_sidecar_naming_the_same_region_twice_is_refused() {
        let error = read_temp_sidecar(
            "duplicate-name",
            "[[hint]]\nname = \"logo\"\nx = 0\ny = 0\nwidth = 10\nheight = 10\n\
             [[hint]]\nname = \"logo\"\nx = 20\ny = 20\nwidth = 10\nheight = 10\n",
        )
        .expect_err("a duplicate region name is refused");
        match error {
            RasterError::DuplicateHintName { name, .. } => {
                assert_eq!(name, "logo");
            }
            other => panic!("expected RasterError::DuplicateHintName, got {other:?}"),
        }
    }
}
