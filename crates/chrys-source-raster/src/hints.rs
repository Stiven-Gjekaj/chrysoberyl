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
}
