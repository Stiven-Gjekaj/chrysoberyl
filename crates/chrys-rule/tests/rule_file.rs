//! The loud-failure surface: every way a rule file can be wrong ends in a
//! message that names a line, and the one shape a rule may never take
//! (D-01, an unscoped rule) cannot be deserialized at all.
//!
//! Follows `crates/chrys-source-raster/src/hints.rs`'s own test module
//! shape: a temporary file per case, an assertion on the error variant,
//! and an assertion that the message carries the parser's own position.
//! No broken rule file is committed; the state each test needs is built
//! inside that test.

use std::path::PathBuf;

use chrys_rule::{MAX_RULE_FILE_BYTES, RuleError, load_rules};

/// Write `contents` to a fresh temporary rule file, call `load_rules` on
/// it, remove the file, and return the result. `label` keeps every
/// temporary file this test file writes from colliding with another
/// test's own file.
fn load_temp_rules(label: &str, contents: &str) -> Result<chrys_rule::LoadedRules, RuleError> {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("chrys-rule-file-test-{label}.toml"));
    std::fs::write(&path, contents).expect("write rule file");
    let result = load_rules(&path);
    std::fs::remove_file(&path).ok();
    result
}

#[test]
fn an_unknown_key_fails_naming_the_line() {
    let error = load_temp_rules(
        "unknown-key",
        "[[rule]]\nkind = \"recoloured\"\nregion = \"badge\"\nmax_delta_e = 5.0\nbogus = 1\n",
    )
    .expect_err("an unknown key is refused, not silently dropped");
    match error {
        RuleError::Parse { message, .. } => {
            assert!(
                message.contains("line"),
                "message does not name a line: {message}"
            );
            assert!(
                message.contains("bogus"),
                "message does not name the unknown field: {message}"
            );
        }
        other => panic!("expected RuleError::Parse, got {other:?}"),
    }
}

#[test]
fn an_invalid_kind_fails_naming_at_least_two_of_the_five_valid_kinds() {
    let error = load_temp_rules(
        "invalid-kind",
        "[[rule]]\nkind = \"teleported\"\nregion = \"badge\"\nmax_delta_e = 5.0\n",
    )
    .expect_err("a kind outside the five named variants is refused");
    match error {
        RuleError::Parse { message, .. } => {
            assert!(
                message.contains("line"),
                "message does not name a line: {message}"
            );
            let named_kinds = ["moved", "added", "removed", "recoloured", "resized"]
                .iter()
                .filter(|kind| message.contains(**kind))
                .count();
            assert!(
                named_kinds >= 2,
                "message should name at least two of the five valid kinds: {message}"
            );
        }
        other => panic!("expected RuleError::Parse, got {other:?}"),
    }
}

#[test]
fn a_wrong_typed_value_fails_naming_the_line() {
    let error = load_temp_rules(
        "wrong-type",
        "[[rule]]\nkind = \"recoloured\"\nregion = \"badge\"\nmax_delta_e = \"high\"\n",
    )
    .expect_err("a string does not fit an f32 field");
    match error {
        RuleError::Parse { message, .. } => {
            assert!(
                message.contains("line"),
                "message does not name a line: {message}"
            );
        }
        other => panic!("expected RuleError::Parse, got {other:?}"),
    }
}

#[test]
fn an_unscoped_rule_is_refused() {
    // D-01: a kind and a tolerance, and no scope at all.
    let error = load_temp_rules(
        "unscoped",
        "[[rule]]\nkind = \"recoloured\"\nmax_delta_e = 5.0\n",
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
        "[[rule]]\nkind = \"recoloured\"\nregion = \"badge\"\nmask = \"m.png\"\nmax_delta_e = 5.0\n",
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
fn a_rule_file_larger_than_the_limit_is_refused_before_it_is_read() {
    let dir = std::env::temp_dir();
    let path = dir.join("chrys-rule-file-test-oversize.toml");

    // One byte over the limit is enough. The test does not need a large
    // file to prove the check runs, and a large file would make this
    // test pay the cost the check exists to refuse.
    let oversize = (MAX_RULE_FILE_BYTES + 1) as usize;
    let mut document = String::with_capacity(oversize);
    document.push('#');
    while document.len() < oversize {
        document.push('a');
    }
    std::fs::write(&path, &document).expect("write rule file");
    let written = std::fs::metadata(&path).expect("stat rule file").len();

    let result = load_rules(&path);
    std::fs::remove_file(&path).ok();

    assert!(
        written > MAX_RULE_FILE_BYTES,
        "the fixture must exceed the limit to test it: {written} bytes"
    );
    match result.expect_err("a rule file over the limit is refused") {
        RuleError::TooLarge { size, limit, .. } => {
            assert_eq!(size, written);
            assert_eq!(limit, MAX_RULE_FILE_BYTES);
        }
        other => panic!("expected RuleError::TooLarge, got {other:?}"),
    }
}

#[test]
fn a_rule_file_exactly_at_the_limit_is_still_read() {
    // The boundary belongs to the accepted side. A test that only proves
    // the refusal cannot tell a correct limit from one that refuses every
    // rule file.
    let dir = std::env::temp_dir();
    let path = dir.join("chrys-rule-file-test-at-limit.toml");

    let rule = "[[rule]]\nkind = \"recoloured\"\nregion = \"badge\"\nmax_delta_e = 5.0\n";
    let mut document = String::from(rule);
    while document.len() < MAX_RULE_FILE_BYTES as usize {
        document.push('#');
    }
    document.truncate(MAX_RULE_FILE_BYTES as usize);
    std::fs::write(&path, &document).expect("write rule file");
    let written = std::fs::metadata(&path).expect("stat rule file").len();

    let result = load_rules(&path);
    std::fs::remove_file(&path).ok();

    assert_eq!(written, MAX_RULE_FILE_BYTES);
    let loaded = result.expect("a rule file exactly at the limit is read");
    assert_eq!(loaded.rules.len(), 1);
}

#[test]
fn a_rule_file_with_no_rule_table_loads_into_an_empty_list() {
    let loaded = load_temp_rules("empty-document", "# nothing here\n")
        .expect("a document with no [[rule]] table is not an error");
    assert!(loaded.rules.is_empty());
}

/// RULE-03 says a rule file is read as data: no field may hold an
/// expression, an operator, or a reference to another rule. This is
/// enforced by the row type's own field list having no field that could
/// hold such a thing; this test reads that struct's own source and fails
/// when a ninth field appears, which is what would break the property
/// this test names, not by re-checking every existing field's own type
/// signature.
#[test]
fn rule_row_has_no_computable_field() {
    let lib_rs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let source = std::fs::read_to_string(&lib_rs_path).expect("read chrys-rule/src/lib.rs");

    let struct_start = source
        .find("struct RuleRow {")
        .expect("RuleRow's struct definition exists in src/lib.rs");
    let after_start = &source[struct_start..];
    let struct_end = after_start
        .find('}')
        .expect("RuleRow's struct body has a closing brace");
    let struct_body = &after_start[..struct_end];

    let field_names: Vec<&str> = struct_body
        .lines()
        .skip(1) // the "struct RuleRow {" line itself
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#[") {
                return None;
            }
            trimmed.split(':').next()
        })
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect();

    let expected = [
        "kind",
        "region",
        "mask",
        "max_offset_px",
        "max_delta_e",
        "max_alpha_delta",
        "max_area_px",
        "allow",
    ];
    assert_eq!(
        field_names, expected,
        "RuleRow's own field list changed. RULE-03 is enforced by this schema having no \
         field that can hold a computation; this assertion is what notices when a ninth \
         field appears."
    );
}

/// RULE-04: the shipped example shows a scoped exclusion and never a bare
/// global threshold. This test asserts only two properties: the file
/// parses, and every rule it holds carries a scope. It does not assert a
/// tolerance value, a region name, or an exact rule count, because this is
/// the one place in this phase where a test reads a file the author edits
/// (AGENTS.md's own "What a test can hold on to" warns against exactly
/// that), and an author who improves the example's wording or its numbers
/// must not also have to update this test.
#[test]
fn the_shipped_example_rule_file_parses_and_is_fully_scoped() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let example_path = repo_root.join("examples/rules/example.toml");

    let loaded = load_rules(&example_path)
        .unwrap_or_else(|error| panic!("{} failed to load: {error}", example_path.display()));

    assert!(
        loaded.rules.len() >= 2,
        "the shipped example should hold at least two rules, a region-scoped one and a \
         mask-scoped one, but it loaded {}",
        loaded.rules.len()
    );

    // `Rule::scope` has no `Option` variant (D-01): a rule with no scope
    // cannot be represented at all, so every loaded rule already carries
    // one. This assertion documents that property rather than testing
    // anything new; it exists so a future reader who removes D-01's
    // type-level guard sees this test still name the requirement.
    for rule in &loaded.rules {
        match &rule.scope {
            chrys_rule::Scope::Region(_) | chrys_rule::Scope::Mask(_) => {}
        }
    }
}
