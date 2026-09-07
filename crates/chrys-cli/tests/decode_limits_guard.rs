//! CLI-04's static guard: every direct call to `image::ImageReader::open`
//! anywhere under a crate's own `src` directory lives inside one of two
//! allow-listed files, each with the exact number of call sites it holds.
//!
//! **Why this crate, and not `chrys-core`.** `chrys-cli` already depends
//! on every adapter crate this workspace ships, and it is not the engine:
//! D-03 forbids editing anything under `crates/chrys-core/`, and this
//! guard's own subject is source text under every crate's `src`
//! directory, which a test living inside the engine's own directory could
//! not read without becoming the very kind of engine-boundary crossing
//! D-03 exists to prevent.
//!
//! **Why this file holds a second comment-and-string stripper.**
//! `crates/chrys-core/tests/determinism.rs` already holds a
//! `strip_comments_and_strings` helper of exactly the right shape: line
//! comments, nested block comments, raw strings and ordinary strings, all
//! replaced with nothing before a search runs. This guard cannot import
//! it. That function is `fn`, private to `determinism.rs`'s own test
//! binary, inside `crates/chrys-core/tests/`, and D-03 forbids editing
//! anything under `crates/chrys-core/` to export it, including its own
//! test directory. Extracting a shared helper into a dev-only crate would
//! cost more review than the roughly one hundred lines this file
//! duplicates below save. `crates/chrys-core/tests/determinism.rs`
//! remains the authoritative copy; this file's own copy exists only
//! because D-03 makes sharing the first one impossible.
//!
//! **Drilled, not only written.** Phase 1 found a guard that could not go
//! red and had been reporting green for a whole phase
//! (`01-LEARNINGS.md`, "A guard that cannot go red reports green
//! forever"). This guard was planted with two defects, each in a
//! disposable git worktree, and each drill's own output is recorded
//! verbatim in `03-03-SUMMARY.md`: one planting an unexpected call site
//! inside `crates/chrys-rule/src/mask.rs`, the other deleting one of the
//! two allow-listed call sites from
//! `crates/chrys-source-raster/src/decode.rs`. Both drills went red and
//! named what was planted.

use std::fs;
use std::path::{Path, PathBuf};

/// The identifier this guard searches for, followed by an opening
/// parenthesis. Every direct construction of a reader in this workspace
/// takes this exact shape: `image::ImageReader::open(path)`. Searching
/// for the trailing `(` means a bare mention of the type name with no
/// call (there is none in this workspace today) cannot trip this guard.
const READER_CONSTRUCTOR: &str = "ImageReader::open(";

/// One allow-listed file: the exact number of call sites it holds, and
/// the reason it is allowed to hold them. Both counts were measured at
/// planning time with a search over this workspace's own current source,
/// and are asserted exactly, in both directions, below.
struct AllowedFile {
    /// The path, relative to the workspace root.
    path: &'static str,
    /// The exact number of call sites this file holds today.
    count: usize,
    /// Why this file is allowed to call the reader constructor directly.
    reason: &'static str,
}

const ALLOW_LIST: &[AllowedFile] = &[
    AllowedFile {
        path: "crates/chrys-source-raster/src/decode.rs",
        count: 2,
        reason: "decode_guarded applies the configured decode limits to the reader it opens, \
                 and probe_dimensions deliberately reads only the header, with no limit, so a \
                 TooLarge error can report the image's true declared size",
    },
    AllowedFile {
        path: "crates/chrys-source-animation/src/lib.rs",
        count: 1,
        reason: "open_guessed is the animation adapter's own guarded decode entry point, the \
                 one place this crate opens a reader before building a format-specific decoder",
    },
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Return every `.rs` file under every crate's own `src` directory,
/// sorted, so this guard's own walk visits files in the same order on
/// every run and every platform. Never descends into a `tests`
/// directory: this guard's own subject is the shipped source under
/// `src/`, not a crate's own test fixtures.
fn rust_files_under_every_src(root: &Path) -> Vec<PathBuf> {
    let crates_dir = root.join("crates");
    let mut files = Vec::new();
    let entries = fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", crates_dir.display()));
    for entry in entries {
        let entry = entry.expect("cannot read a directory entry");
        let crate_dir = entry.path();
        if !crate_dir.is_dir() {
            continue;
        }
        let src_dir = crate_dir.join("src");
        if !src_dir.is_dir() {
            continue;
        }
        collect_rust_files(&src_dir, &mut files);
    }
    files.sort();
    files
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot read directory {}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.expect("cannot read a directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

/// Strip line comments, nested block comments, raw strings and ordinary
/// strings out of `source`, replacing each with nothing. What remains is
/// source code only: a mention of the reader constructor's name in prose,
/// or hidden inside a string literal, can no longer trip the search
/// below, and a real call cannot hide inside a string either.
///
/// This is a smaller copy of
/// `crates/chrys-core/tests/determinism.rs`'s own
/// `strip_comments_and_strings`; see this file's own module comment for
/// why a second copy exists rather than a shared one.
fn strip_comments_and_strings(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;

    while i < n {
        let c = chars[i];

        // Line comment: runs to the end of the line.
        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Block comment: Rust nests these, so track a depth counter.
        if c == '/' && i + 1 < n && chars[i + 1] == '*' {
            let mut depth = 1usize;
            i += 2;
            while i < n && depth > 0 {
                if chars[i] == '/' && i + 1 < n && chars[i + 1] == '*' {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && i + 1 < n && chars[i + 1] == '/' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // Raw string or raw byte string: r"...", r#"..."#, br"...",
        // br#"..."#, with any number of `#` delimiters.
        if let Some(after) = raw_string_end(&chars, i) {
            i = after;
            continue;
        }

        // Ordinary or byte string literal.
        if c == '"' || (c == 'b' && i + 1 < n && chars[i + 1] == '"') {
            let mut j = if c == 'b' { i + 1 } else { i };
            j += 1; // past the opening quote
            while j < n {
                if chars[j] == '\\' {
                    j += 2;
                    continue;
                }
                if chars[j] == '"' {
                    j += 1;
                    break;
                }
                j += 1;
            }
            i = j;
            continue;
        }

        // Character literal, or a byte-character literal, but not a
        // lifetime: only consume the quote as a char literal when it is
        // actually closed by a matching quote.
        if c == '\'' || (c == 'b' && i + 1 < n && chars[i + 1] == '\'') {
            let quote_at = if c == 'b' { i + 1 } else { i };
            if let Some(after) = char_literal_end(&chars, quote_at) {
                i = after;
                continue;
            }
        }

        out.push(c);
        i += 1;
    }

    out
}

fn raw_string_end(chars: &[char], start: usize) -> Option<usize> {
    let n = chars.len();
    let mut j = start;
    if j < n && chars[j] == 'b' {
        j += 1;
    }
    if j >= n || chars[j] != 'r' {
        return None;
    }
    j += 1;
    let mut hashes = 0usize;
    while j < n && chars[j] == '#' {
        hashes += 1;
        j += 1;
    }
    if j >= n || chars[j] != '"' {
        return None;
    }
    j += 1;
    while j < n {
        if chars[j] == '"' {
            let close_hashes = chars[j + 1..].iter().take_while(|&&ch| ch == '#').count();
            if close_hashes >= hashes {
                return Some(j + 1 + hashes);
            }
        }
        j += 1;
    }
    Some(n)
}

fn char_literal_end(chars: &[char], quote_at: usize) -> Option<usize> {
    let n = chars.len();
    let mut j = quote_at + 1;
    if j >= n {
        return None;
    }
    if chars[j] == '\\' {
        j += 2;
        if j < n && chars[j] == '\'' {
            return Some(j + 1);
        }
        return None;
    }
    if chars[j] != '\'' && j + 1 < n && chars[j + 1] == '\'' {
        return Some(j + 2);
    }
    None
}

/// Count every occurrence of `READER_CONSTRUCTOR` in `stripped` (already
/// run through `strip_comments_and_strings`).
fn count_call_sites(stripped: &str) -> usize {
    stripped.matches(READER_CONSTRUCTOR).count()
}

#[test]
fn only_allow_listed_call_sites_open_an_image_reader_directly() {
    let root = workspace_root();
    let files = rust_files_under_every_src(&root);
    assert!(
        !files.is_empty(),
        "the file walk found no .rs file under any crate's src directory; the walk itself is \
         broken"
    );

    // Every allow-listed path must exist: a guard whose allow-list names
    // a file that has since moved or been renamed would silently stop
    // checking that file's own call sites, and report green about a file
    // it is no longer reading.
    for allowed in ALLOW_LIST {
        let full_path = root.join(allowed.path);
        assert!(
            full_path.is_file(),
            "the allow-list names {}, but no such file exists; the guard is checking a file \
             that no longer exists, which means it is checking nothing at all",
            allowed.path
        );
    }

    let mut unexpected: Vec<(PathBuf, usize)> = Vec::new();
    let mut counted_paths: Vec<&str> = Vec::new();

    for file in &files {
        let source = fs::read_to_string(file)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()));
        let stripped = strip_comments_and_strings(&source);
        let found = count_call_sites(&stripped);
        if found == 0 {
            continue;
        }

        let relative = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");

        match ALLOW_LIST.iter().find(|allowed| allowed.path == relative) {
            Some(allowed) => {
                counted_paths.push(allowed.path);
                assert_eq!(
                    found, allowed.count,
                    "{} holds {found} call site(s) to `{READER_CONSTRUCTOR}`, but the \
                     allow-list expects exactly {}. A decode that does not go through \
                     decode_guarded reopens the decompression-bomb risk CLI-04 exists to \
                     close; if this file's own call count moved on purpose, the allow-list's \
                     own `count` must be updated to match, deliberately, in the same change.",
                    allowed.path, allowed.count
                );
            }
            None => unexpected.push((file.clone(), found)),
        }
    }

    assert!(
        unexpected.is_empty(),
        "the following file(s) call `{READER_CONSTRUCTOR}` directly but are not in this \
         guard's own allow-list: {}. A decode that does not go through decode_guarded (or the \
         animation adapter's own equivalent) reopens the decompression-bomb risk CLI-04 exists \
         to close; either route this call through the existing guarded entry point, or, if a \
         new guarded entry point is genuinely needed, add it to ALLOW_LIST in this file, \
         deliberately, in the same change.",
        unexpected
            .iter()
            .map(|(path, found)| format!("{} ({found} site(s))", path.display()))
            .collect::<Vec<_>>()
            .join(", ")
    );

    for allowed in ALLOW_LIST {
        assert!(
            counted_paths.contains(&allowed.path),
            "the allow-list names {}, but the walk found no call site there at all; the \
             file's own call site(s) may have been refactored away, in which case this entry \
             should be removed from ALLOW_LIST, deliberately, in the same change",
            allowed.path
        );
    }
}

/// Every allow-list entry names why it is allowed, not only how many call
/// sites it holds. An empty reason would leave a future reader guessing
/// at a decision this file's own module comment already states plainly
/// for both entries.
#[test]
fn every_allow_listed_file_names_its_own_reason() {
    for allowed in ALLOW_LIST {
        assert!(
            !allowed.reason.trim().is_empty(),
            "{} has no reason recorded in ALLOW_LIST",
            allowed.path
        );
    }
}
