//! The mask path safety tests (T-03-08): a `Scope::Mask` path cannot leave
//! the rule file's own directory, by two independent checks, and a mask
//! above the decode limits is refused by the same guarded decoder every
//! other raster decode in this project already uses (CLI-04).
//!
//! Each test builds its own temporary directory holding a rule file, and
//! where a mask is needed, writes one, so no test here depends on another
//! test's own file or on execution order.

use std::path::{Path, PathBuf};

use chrys_rule::RuleError;

/// Build a fresh, empty temporary directory for one test. `label` names
/// the test, so no two tests in this file can collide over the same path.
fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chrys-rule-mask-path-test-{label}"));
    std::fs::create_dir_all(&dir).expect("create temp test directory");
    dir
}

/// Write a one-rule rule file into `dir`, naming `mask_value` as its
/// `mask` field, and return the rule file's own path.
fn write_rule_file(dir: &Path, mask_value: &str) -> PathBuf {
    let rule_path = dir.join("rule.toml");
    let contents =
        format!("[[rule]]\nkind = \"recoloured\"\nmask = \"{mask_value}\"\nmax_delta_e = 10.0\n");
    std::fs::write(&rule_path, contents).expect("write rule file");
    rule_path
}

/// Write a small, well-formed, fully white and opaque PNG to `path`. The
/// exact pixel content does not matter to any test in this file beyond "a
/// mask that decodes".
fn write_tiny_mask(path: &Path) {
    image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 255, 255, 255]))
        .save(path)
        .expect("write temp mask png");
}

/// The exact case `03-VALIDATION.md` asked for by name: a mask path shaped
/// like `../../etc/passwd`. Refused lexically, before any byte of any file
/// is read, so the path need not name a file that exists at all.
#[test]
fn a_mask_path_that_leaves_the_rule_file_directory_is_refused() {
    let dir = temp_dir("dotdot-etc-passwd");
    let rule_path = write_rule_file(&dir, "../../etc/passwd");

    let error = chrys_rule::load_rules(&rule_path)
        .expect_err("a path shaped like ../../etc/passwd is refused");
    let message = error.to_string();
    match error {
        RuleError::MaskPathEscapesDirectory {
            rule_dir,
            mask_path,
        } => {
            assert_eq!(rule_dir, dir);
            assert_eq!(mask_path, PathBuf::from("../../etc/passwd"));
            assert!(
                message.contains(dir.to_str().expect("temp dir is UTF-8")),
                "message does not name the rule file's own directory: {message}"
            );
            assert!(
                message.contains("../../etc/passwd"),
                "message does not name the offending path: {message}"
            );
        }
        other => panic!("expected RuleError::MaskPathEscapesDirectory, got {other:?}"),
    }
}

/// A single leading parent-directory component is refused the same way a
/// longer traversal is: the lexical check counts any `..` component, not
/// only a chain of them.
#[test]
fn a_mask_path_with_a_single_leading_parent_component_is_refused() {
    let dir = temp_dir("single-parent-component");
    let rule_path = write_rule_file(&dir, "../sibling.png");

    let error =
        chrys_rule::load_rules(&rule_path).expect_err("a single leading .. component is refused");
    match error {
        RuleError::MaskPathEscapesDirectory { .. } => {}
        other => panic!("expected RuleError::MaskPathEscapesDirectory, got {other:?}"),
    }
}

/// An absolute mask path is refused outright: relative to the rule file's
/// own directory is the only meaning a mask path has.
#[test]
fn an_absolute_mask_path_is_refused() {
    let dir = temp_dir("absolute-path");
    let rule_path = write_rule_file(&dir, "/etc/passwd");

    let error = chrys_rule::load_rules(&rule_path).expect_err("an absolute path is refused");
    match error {
        RuleError::MaskPathEscapesDirectory { .. } => {}
        other => panic!("expected RuleError::MaskPathEscapesDirectory, got {other:?}"),
    }
}

/// A symbolic link inside the rule file's own directory, pointing to a
/// file outside it, is refused by the canonical check: the lexical check
/// alone cannot see it, because the link's own name holds no `..` and is
/// not absolute.
///
/// Skipped, with a named message rather than a silent pass, on a platform
/// where this test process cannot create a symbolic link at all (for
/// example, Windows without Developer Mode or an elevated process). A test
/// that quietly does nothing is the failure mode phase 2 recorded for a
/// command that selected no test.
#[test]
fn a_mask_path_that_reaches_outside_through_a_symbolic_link_is_refused() {
    let dir = temp_dir("symlink-outside");
    let outside_dir = temp_dir("symlink-outside-target");
    let outside_mask = outside_dir.join("outside.png");
    write_tiny_mask(&outside_mask);

    let link_path = dir.join("link.png");
    std::fs::remove_file(&link_path).ok();

    if !create_symlink(&outside_mask, &link_path) {
        eprintln!(
            "skipped a_mask_path_that_reaches_outside_through_a_symbolic_link_is_refused: \
             this test process cannot create a symbolic link on this platform"
        );
        return;
    }

    let rule_path = write_rule_file(&dir, "link.png");
    let error = chrys_rule::load_rules(&rule_path)
        .expect_err("a symlink pointing outside the rule file's directory is refused");
    match error {
        RuleError::MaskPathEscapesDirectory { .. } => {}
        other => panic!("expected RuleError::MaskPathEscapesDirectory, got {other:?}"),
    }
}

/// Create a symbolic link at `link_path` pointing to `target`, on whatever
/// platform this test runs on. Returns `false`, without panicking, when
/// this process lacks the privilege to create one at all, so the caller
/// can skip with a named message instead of failing on an environment
/// question this test does not exist to answer.
fn create_symlink(target: &Path, link_path: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path).is_ok()
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link_path).is_ok()
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link_path);
        false
    }
}

/// A mask path naming a file in a subdirectory below the rule file is
/// accepted: the checks refuse a path that leaves the rule file's own
/// directory, not one that merely names a nested path inside it.
#[test]
fn a_mask_path_in_a_subdirectory_below_the_rule_file_is_accepted() {
    let dir = temp_dir("subdirectory-accepted");
    let sub_dir = dir.join("masks");
    std::fs::create_dir_all(&sub_dir).expect("create subdirectory");
    let mask_path = sub_dir.join("badge.png");
    write_tiny_mask(&mask_path);

    let rule_path = write_rule_file(&dir, "masks/badge.png");
    let loaded = chrys_rule::load_rules(&rule_path)
        .expect("a mask path naming a file in a subdirectory below the rule file is accepted");
    assert_eq!(loaded.rules.len(), 1);
}

/// A mask above the decode limits is refused through the guarded decoder
/// itself, not through a second limit check written in this crate
/// (CLI-04). `load_mask` takes its limits as a parameter rather than
/// hard-coding the default, which is what lets this test supply a limit
/// small enough to trigger the refusal without needing to construct a
/// multi-gigabyte fixture.
#[test]
fn a_mask_above_the_decode_limits_is_refused() {
    let dir = temp_dir("above-decode-limits");
    let mask_path = dir.join("oversized.png");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]))
        .save(&mask_path)
        .expect("write temp mask png");

    let tiny_limits = chrys_source_raster::DecodeLimits {
        max_width: 4,
        max_height: 4,
        max_alloc: chrys_source_raster::DecodeLimits::default().max_alloc,
    };
    let error = chrys_rule::mask::load_mask(&mask_path, &tiny_limits)
        .expect_err("a mask above the supplied decode limits is refused");
    match error {
        RuleError::MaskDecode { .. } => {}
        other => panic!("expected RuleError::MaskDecode, got {other:?}"),
    }
}
