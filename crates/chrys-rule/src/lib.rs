//! The `.toml` rule file reader. A rule file names a kind of change, a
//! scope, and a tolerance, and this crate reads it as plain data: no
//! field here can hold an expression, an operator, or a reference to
//! another rule.
//!
//! This crate depends on `chrys-core` and `chrys-source`, never the other
//! way round. `chrys-core` declares no `serde` and no `toml`, and this
//! crate is the reason that stays true: a rule file arrives from wherever
//! the input pair arrived from, and its parser lives outside the engine.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml::Spanned;

pub mod evaluate;

pub use evaluate::{RegionOutcome, RuleOutcome, evaluate, overlapping_hint_name};

/// The largest rule file this reader accepts, in bytes.
///
/// A rule file names a handful of tolerances. The same posture, and the
/// same limit, `crates/chrys-source-raster/src/hints.rs` already takes for
/// its own sidecar: bound the allocation from `std::fs::metadata` before
/// any byte is read, because a rule file comes from wherever the input
/// arrived from.
pub const MAX_RULE_FILE_BYTES: u64 = 1024 * 1024;

/// One `[[rule]]` table's own `kind` value, read as a closed set of five
/// names, one per `chrys_core::ChangeKind` variant.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum KindName {
    Moved,
    Added,
    Removed,
    Recoloured,
    Resized,
}

/// One `[[rule]]` table, deserialized permissively before the semantic
/// validation pass below converts it into a `Rule`.
///
/// Every field is wrapped in `toml::Spanned<T>`, which plain
/// `toml::from_str` already populates with no special deserializer entry
/// point (verified against the pinned `toml =1.1.5` this project uses).
/// That span is what lets the validation pass name the exact line of a
/// field `serde` accepted but this schema does not.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleRow {
    kind: Spanned<KindName>,
    #[serde(default)]
    region: Option<Spanned<String>>,
    #[serde(default)]
    mask: Option<Spanned<PathBuf>>,
    #[serde(default)]
    max_offset_px: Option<Spanned<u32>>,
    #[serde(default)]
    max_delta_e: Option<Spanned<f32>>,
    #[serde(default)]
    max_alpha_delta: Option<Spanned<u8>>,
    #[serde(default)]
    max_area_px: Option<Spanned<u64>>,
    #[serde(default)]
    allow: Option<Spanned<bool>>,
}

/// The rule file's own document shape: an array of tables named `rule`.
/// Absent entirely when the document holds no `[[rule]]` table at all,
/// which is not an error: a rule file that says nothing tolerates
/// nothing.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleDocument {
    #[serde(rename = "rule", default)]
    rule: Vec<Spanned<RuleRow>>,
}

/// A scope names exactly one of two things a rule answers to. There is no
/// third variant and no `Option`, so a rule with no scope cannot be
/// represented at all: an unscoped rule is refused at parse time, at the
/// type level, not by a lint (D-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// A named region: matched against a hint already present on the
    /// compared frame.
    Region(String),
    /// A mask image: the tolerance area its own pixels name. Not yet
    /// decoded by this plan; a later plan fills in the pixels this
    /// variant's path names.
    Mask(PathBuf),
}

/// Exactly one tolerance statement per rule, so "what does this rule
/// tolerate" is a type-level fact rather than a set of fields a reader
/// must cross-reference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tolerance {
    /// Tolerates a `Recoloured` region whose colour difference is at or
    /// below `max_delta_e` and whose alpha difference is at or below
    /// `max_alpha_delta`.
    ///
    /// Two numbers, not one: `chrys_core::ColourDelta`'s own doc comment
    /// states that `delta_e` covers colour only, that Lab has no axis for
    /// alpha, and that a rule engine reading the type must read the alpha
    /// difference from the two colours directly, or a tolerance meant for
    /// colour would silently swallow an alpha-only change.
    /// `max_alpha_delta` defaults to 0 when the rule file omits it, so a
    /// colour tolerance tolerates no alpha movement by default.
    Colour {
        max_delta_e: f32,
        max_alpha_delta: u8,
    },
    /// Tolerates every region of this rule's own kind, inside this rule's
    /// own scope, when `true`.
    Allow(bool),
    /// Tolerates a `Moved` region whose offset is within this many pixels
    /// on both axes.
    MaxOffsetPx(u32),
    /// Tolerates an `Added`, `Removed` or `Resized` region whose bounding
    /// box area is at or below this many pixels.
    ///
    /// The engine publishes no size delta for a `Resized` region, only its
    /// bounding box, so the box's own area is what the published data
    /// supports; reaching further would mean reaching into the engine,
    /// which D-03 forbids.
    MaxAreaPx(u64),
}

impl Tolerance {
    /// Whether this tolerance covers `region`'s own measured value. Called
    /// only after the caller has already matched `region`'s kind against
    /// this tolerance's own rule and confirmed the region falls inside
    /// that rule's scope; this method reads only the one field of
    /// `region` its own kind of tolerance depends on.
    fn tolerates(&self, region: &chrys_core::Region) -> bool {
        match self {
            Tolerance::Allow(allow) => *allow,
            Tolerance::Colour {
                max_delta_e,
                max_alpha_delta,
            } => match &region.colour_delta {
                Some(delta) => {
                    let alpha_delta = delta.base[3].abs_diff(delta.candidate[3]);
                    delta.delta_e <= *max_delta_e && alpha_delta <= *max_alpha_delta
                }
                // A Recoloured region with no colour delta was never told
                // one; a rule cannot tolerate a measurement it never
                // received.
                None => false,
            },
            Tolerance::MaxOffsetPx(limit) => match region.offset_px {
                Some((dx, dy)) => dx.unsigned_abs() <= *limit && dy.unsigned_abs() <= *limit,
                // A Moved region with no offset was never told one,
                // for the same reason as above.
                None => false,
            },
            Tolerance::MaxAreaPx(limit) => {
                let area = u64::from(region.bbox.width) * u64::from(region.bbox.height);
                area <= *limit
            }
        }
    }
}

/// One validated rule: a kind of change, a scope, and a tolerance.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub kind: chrys_core::ChangeKind,
    pub scope: Scope,
    pub tolerance: Tolerance,
}

/// The rules loaded from one rule file.
///
/// A mask-scoped rule's path is carried but not yet resolved to decoded
/// pixels; a later plan fills this value in, at which point a bad mask
/// path fails while the rule file loads, before any comparison runs, and
/// `evaluate`'s own signature does not change.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LoadedRules {
    pub rules: Vec<Rule>,
}

/// Why a rule file failed to load.
#[derive(Debug, thiserror::Error)]
pub enum RuleError {
    /// The rule file could not be read from disk.
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The rule file is larger than `MAX_RULE_FILE_BYTES`.
    #[error("{path} is {size} bytes, which exceeds the rule file limit of {limit} bytes")]
    TooLarge {
        path: PathBuf,
        size: u64,
        limit: u64,
    },
    /// The file was read, but its bytes did not parse as a well-formed
    /// rule document: an unknown key, an invalid `kind`, or a wrongly
    /// typed value. The parser's own message carries its line and column.
    #[error("cannot parse {path}: {message}")]
    Parse { path: PathBuf, message: String },
    /// A `[[rule]]` table named neither `region` nor `mask` (D-01).
    #[error(
        "{path} line {line}: a rule must name exactly one of `region` or `mask`, but this rule names neither"
    )]
    MissingScope { path: PathBuf, line: usize },
    /// A `[[rule]]` table named both `region` and `mask`.
    #[error(
        "{path} line {line}: a rule must name exactly one of `region` or `mask`, but this rule names both"
    )]
    AmbiguousScope { path: PathBuf, line: usize },
    /// A `[[rule]]` table named a kind and a scope, but no tolerance field
    /// at all.
    #[error(
        "{path} line {line}: a rule names a scope but no tolerance; a rule must say what it tolerates"
    )]
    MissingTolerance { path: PathBuf, line: usize },
    /// A tolerance field was set that does not belong to its own row's
    /// `kind`.
    #[error("{path} line {line}: `{field}` does not apply to a rule of kind `{kind}`")]
    InvalidToleranceField {
        path: PathBuf,
        line: usize,
        field: &'static str,
        kind: &'static str,
    },
    /// A row set `allow` alongside a numeric tolerance field, two
    /// statements that contradict each other.
    #[error("{path} line {line}: `allow` cannot be combined with a numeric tolerance field")]
    ConflictingTolerance { path: PathBuf, line: usize },
}

/// Read and validate the rule file at `rule_path`.
///
/// The file is bounded by `MAX_RULE_FILE_BYTES`, read from
/// `std::fs::metadata`, before a single byte is read. A well-formed but
/// empty document (no `[[rule]]` table at all) loads into an empty rule
/// list, which is not an error: a rule file that says nothing tolerates
/// nothing.
pub fn load_rules(rule_path: &Path) -> Result<LoadedRules, RuleError> {
    let metadata = std::fs::metadata(rule_path).map_err(|source| RuleError::Io {
        path: rule_path.to_path_buf(),
        source,
    })?;
    if metadata.len() > MAX_RULE_FILE_BYTES {
        return Err(RuleError::TooLarge {
            path: rule_path.to_path_buf(),
            size: metadata.len(),
            limit: MAX_RULE_FILE_BYTES,
        });
    }

    let text = std::fs::read_to_string(rule_path).map_err(|source| RuleError::Io {
        path: rule_path.to_path_buf(),
        source,
    })?;

    let document: RuleDocument = toml::from_str(&text).map_err(|error| RuleError::Parse {
        path: rule_path.to_path_buf(),
        message: error.to_string(),
    })?;

    let mut rules = Vec::with_capacity(document.rule.len());
    for spanned_row in document.rule {
        rules.push(validate_row(rule_path, &text, spanned_row)?);
    }

    Ok(LoadedRules { rules })
}

/// The line number (1-based) that byte offset `offset` of `text` falls on.
fn line_of(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count() + 1
}

/// Return the name a rule file uses for `kind`, for an error message.
fn kind_name_str(kind: chrys_core::ChangeKind) -> &'static str {
    match kind {
        chrys_core::ChangeKind::Moved => "moved",
        chrys_core::ChangeKind::Added => "added",
        chrys_core::ChangeKind::Removed => "removed",
        chrys_core::ChangeKind::Recoloured => "recoloured",
        chrys_core::ChangeKind::Resized => "resized",
    }
}

/// Refuse `field` when it is set but `allowed` is `false`, naming the
/// field's own line rather than the line the enclosing table starts on.
fn reject_if_present<T>(
    field: &Option<Spanned<T>>,
    field_name: &'static str,
    allowed: bool,
    kind_name: &'static str,
    path: &Path,
    text: &str,
) -> Result<(), RuleError> {
    if allowed {
        return Ok(());
    }
    if let Some(spanned) = field {
        let line = line_of(text, spanned.span().start);
        return Err(RuleError::InvalidToleranceField {
            path: path.to_path_buf(),
            line,
            field: field_name,
            kind: kind_name,
        });
    }
    Ok(())
}

/// Convert one deserialized `RuleRow` into a validated `Rule`, applying
/// every semantic check `serde` cannot express: exactly one scope,
/// exactly one tolerance, and a tolerance field that belongs to its own
/// row's kind.
fn validate_row(path: &Path, text: &str, spanned_row: Spanned<RuleRow>) -> Result<Rule, RuleError> {
    let row_line = line_of(text, spanned_row.span().start);
    let row = spanned_row.into_inner();

    let kind = match row.kind.into_inner() {
        KindName::Moved => chrys_core::ChangeKind::Moved,
        KindName::Added => chrys_core::ChangeKind::Added,
        KindName::Removed => chrys_core::ChangeKind::Removed,
        KindName::Recoloured => chrys_core::ChangeKind::Recoloured,
        KindName::Resized => chrys_core::ChangeKind::Resized,
    };
    let kind_name = kind_name_str(kind);

    let scope = match (row.region, row.mask) {
        (Some(region), None) => Scope::Region(region.into_inner()),
        (None, Some(mask)) => Scope::Mask(mask.into_inner()),
        (None, None) => {
            return Err(RuleError::MissingScope {
                path: path.to_path_buf(),
                line: row_line,
            });
        }
        (Some(_), Some(_)) => {
            return Err(RuleError::AmbiguousScope {
                path: path.to_path_buf(),
                line: row_line,
            });
        }
    };

    reject_if_present(
        &row.max_offset_px,
        "max_offset_px",
        kind == chrys_core::ChangeKind::Moved,
        kind_name,
        path,
        text,
    )?;
    reject_if_present(
        &row.max_delta_e,
        "max_delta_e",
        kind == chrys_core::ChangeKind::Recoloured,
        kind_name,
        path,
        text,
    )?;
    reject_if_present(
        &row.max_alpha_delta,
        "max_alpha_delta",
        kind == chrys_core::ChangeKind::Recoloured,
        kind_name,
        path,
        text,
    )?;
    reject_if_present(
        &row.max_area_px,
        "max_area_px",
        matches!(
            kind,
            chrys_core::ChangeKind::Added
                | chrys_core::ChangeKind::Removed
                | chrys_core::ChangeKind::Resized
        ),
        kind_name,
        path,
        text,
    )?;

    let has_numeric = row.max_offset_px.is_some()
        || row.max_delta_e.is_some()
        || row.max_alpha_delta.is_some()
        || row.max_area_px.is_some();

    if row.allow.is_some() && has_numeric {
        return Err(RuleError::ConflictingTolerance {
            path: path.to_path_buf(),
            line: row_line,
        });
    }

    let tolerance = if let Some(allow) = row.allow {
        Tolerance::Allow(allow.into_inner())
    } else {
        match kind {
            chrys_core::ChangeKind::Recoloured => match row.max_delta_e {
                Some(max_delta_e) => Tolerance::Colour {
                    max_delta_e: max_delta_e.into_inner(),
                    max_alpha_delta: row
                        .max_alpha_delta
                        .map(|spanned| spanned.into_inner())
                        .unwrap_or(0),
                },
                None => {
                    return Err(RuleError::MissingTolerance {
                        path: path.to_path_buf(),
                        line: row_line,
                    });
                }
            },
            chrys_core::ChangeKind::Moved => match row.max_offset_px {
                Some(max_offset_px) => Tolerance::MaxOffsetPx(max_offset_px.into_inner()),
                None => {
                    return Err(RuleError::MissingTolerance {
                        path: path.to_path_buf(),
                        line: row_line,
                    });
                }
            },
            chrys_core::ChangeKind::Added
            | chrys_core::ChangeKind::Removed
            | chrys_core::ChangeKind::Resized => match row.max_area_px {
                Some(max_area_px) => Tolerance::MaxAreaPx(max_area_px.into_inner()),
                None => {
                    return Err(RuleError::MissingTolerance {
                        path: path.to_path_buf(),
                        line: row_line,
                    });
                }
            },
        }
    };

    Ok(Rule {
        kind,
        scope,
        tolerance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write `contents` to a fresh temporary rule file, call `load_rules`
    /// on it, remove the file, and return the result. `label` keeps every
    /// temporary file this test module writes from colliding with
    /// another test's own file.
    fn load_temp_rules(label: &str, contents: &str) -> Result<LoadedRules, RuleError> {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("chrys-rule-test-{label}.toml"));
        std::fs::write(&path, contents).expect("write rule file");
        let result = load_rules(&path);
        std::fs::remove_file(&path).ok();
        result
    }

    #[test]
    fn a_scoped_recoloured_rule_loads_into_one_rule() {
        let loaded = load_temp_rules(
            "one-rule",
            "[[rule]]\nkind = \"recoloured\"\nregion = \"badge\"\nmax_delta_e = 12.0\n",
        )
        .expect("a well-formed rule file loads");
        assert_eq!(loaded.rules.len(), 1);
        let rule = &loaded.rules[0];
        assert_eq!(rule.kind, chrys_core::ChangeKind::Recoloured);
        assert_eq!(rule.scope, Scope::Region("badge".to_string()));
        assert_eq!(
            rule.tolerance,
            Tolerance::Colour {
                max_delta_e: 12.0,
                max_alpha_delta: 0,
            }
        );
    }

    #[test]
    fn an_unscoped_rule_is_refused() {
        // D-01: a kind and a tolerance, and no scope at all.
        let error = load_temp_rules(
            "unscoped",
            "[[rule]]\nkind = \"recoloured\"\nmax_delta_e = 12.0\n",
        )
        .expect_err("a rule naming no scope is refused");
        match error {
            RuleError::MissingScope { .. } => {}
            other => panic!("expected RuleError::MissingScope, got {other:?}"),
        }
    }

    #[test]
    fn a_rule_naming_both_region_and_mask_is_refused() {
        let error = load_temp_rules(
            "both-scopes",
            "[[rule]]\nkind = \"recoloured\"\nregion = \"badge\"\nmask = \"m.png\"\nmax_delta_e = 12.0\n",
        )
        .expect_err("a rule naming two scopes is refused");
        match error {
            RuleError::AmbiguousScope { .. } => {}
            other => panic!("expected RuleError::AmbiguousScope, got {other:?}"),
        }
    }

    #[test]
    fn a_rule_with_no_tolerance_field_is_refused() {
        let error = load_temp_rules(
            "no-tolerance",
            "[[rule]]\nkind = \"recoloured\"\nregion = \"badge\"\n",
        )
        .expect_err("a rule naming no tolerance is refused");
        match error {
            RuleError::MissingTolerance { .. } => {}
            other => panic!("expected RuleError::MissingTolerance, got {other:?}"),
        }
    }

    #[test]
    fn a_tolerance_field_wrong_for_its_kind_fails_naming_the_line() {
        // max_delta_e does not belong to a `moved` rule.
        let contents = "[[rule]]\nkind = \"moved\"\nregion = \"badge\"\nmax_delta_e = 5.0\n";
        let error = load_temp_rules("wrong-field-for-kind", contents)
            .expect_err("a tolerance field outside its own kind's set is refused");
        match error {
            RuleError::InvalidToleranceField {
                line, field, kind, ..
            } => {
                assert_eq!(field, "max_delta_e");
                assert_eq!(kind, "moved");
                assert_eq!(
                    line, 4,
                    "the message should name the field's own line (line 4), not the \
                     table's first line (line 1)"
                );
            }
            other => panic!("expected RuleError::InvalidToleranceField, got {other:?}"),
        }
    }

    #[test]
    fn allow_combined_with_a_numeric_tolerance_is_refused() {
        let contents = "[[rule]]\nkind = \"recoloured\"\nregion = \"badge\"\nallow = true\nmax_delta_e = 5.0\n";
        let error = load_temp_rules("conflicting-tolerance", contents)
            .expect_err("allow alongside a numeric tolerance is refused");
        match error {
            RuleError::ConflictingTolerance { .. } => {}
            other => panic!("expected RuleError::ConflictingTolerance, got {other:?}"),
        }
    }

    #[test]
    fn a_rule_file_with_no_rule_table_loads_into_an_empty_list() {
        let loaded = load_temp_rules("empty-document", "# nothing here\n")
            .expect("a document with no [[rule]] table is not an error");
        assert!(loaded.rules.is_empty());
    }
}
