//! DET-05's source-level guard: no file in this workspace's own source
//! tree names `fontdb::Database::load_system_fonts`, the host-font-
//! database loader, by name.
//!
//! **A second, independent check on the same property.** `04-01-SUMMARY.
//! md` and `svg_boundary_guard.rs` together give DET-05 a compile-time
//! guard: `resvg`'s `system-fonts` Cargo feature is off workspace-wide,
//! so the loader is absent from the built binary. `01-LEARNINGS.md`
//! already recorded, for a different property, that a guard resting on
//! one mechanism can be blind exactly where it is needed. If a future
//! change re-enables the Cargo feature, `svg_boundary_guard.rs` catches
//! it. If a future change reaches the loader some other way — a second
//! crate elsewhere in this workspace that already carries `fontdb` with
//! its `fs` feature on, say — this file catches it. Neither replaces the
//! other.
//!
//! **A deliberate deviation from `04-VALIDATION.md`'s own test map,
//! recorded here so it is not read later as an omission.** The
//! validation contract offers this guard two homes: extend
//! `crates/chrys-core/tests/determinism.rs`, or add it to
//! `chrys-source-svg` as `no_source_file_calls_load_system_fonts`.
//! Neither is taken. The first sits inside the subtree whose tree object
//! id (`c97a6fb778c4b1373e5c4dc563481cc18e4c98c0`, read at commit
//! `199477d`) this phase's own headline architectural claim says must
//! not move, so taking it would break the claim in order to guard it.
//! The second would cover one crate, while the symbol this file searches
//! for is a `fontdb` method: any crate that gains a path to `fontdb`
//! gains a path to it, so a one-crate search would report a
//! workspace-wide property it never measured. This guard lives in
//! `chrys-cli`, the crate that already depends on every other one, and
//! searches every crate's own source tree, the same reasoning
//! `unsafe_guard.rs` and `decode_limits_guard.rs` already give for living
//! in this same directory.
//!
//! **Why this file holds a third copy of `strip_comments_and_strings`.**
//! `crates/chrys-core/tests/determinism.rs` holds the original; this
//! phase may not edit anything under `crates/chrys-core/`, including to
//! export it. `crates/chrys-cli/tests/decode_limits_guard.rs` already
//! holds a second copy for the identical reason, and its own header
//! comment already explains why a shared helper crate would cost more
//! review than the lines it would save. This file follows the one answer
//! already in this directory rather than inventing a third.
//!
//! **Why every string and comment is stripped before the search runs.**
//! Every plan, research document and doc comment in this phase discusses
//! `load_system_fonts` by name, this file's own module comment above
//! included. A search that counted prose would go red on a comment
//! explaining why the call must not appear, which would make the rule
//! self-invalidating and would teach a future reader to delete the guard
//! rather than the call it exists to catch.

use std::fs;
use std::path::{Path, PathBuf};

/// The identifier this guard searches for: `fontdb::Database`'s own
/// system-font-scanning method. Generated through `stringify!` on the
/// bare identifier rather than typed as a quoted string literal, so this
/// file's own search needle is not itself erased by
/// `strip_comments_and_strings`'s string-stripping pass below — it is
/// the one occurrence `ALLOW_LIST` exempts, because it is the constant
/// the search uses.
const HOST_FONT_LOADER: &str = stringify!(load_system_fonts);

/// One allow-listed file, and the reason it is allowed to name
/// `HOST_FONT_LOADER` outside a comment or a string.
struct AllowedFile {
    /// The path, relative to the workspace root.
    path: &'static str,
    /// Why this file is allowed to name the loader.
    reason: &'static str,
}

const ALLOW_LIST: &[AllowedFile] = &[AllowedFile {
    path: "crates/chrys-cli/tests/system_font_guard.rs",
    reason: "this file's own search needle names the symbol it searches for, generated \
             through stringify! so the string-literal-stripping pass this guard applies to \
             every other file does not erase the needle in this one file too",
}];

/// The lowest number of `.rs` files this guard's own walk may report
/// having visited. Measured from a real run over this workspace's own
/// source, at this task's own authoring time (68 files visited: `find
/// crates -type d \( -name src -o -name tests -o -name examples \) -exec
/// find {} -name "*.rs" \; | wc -l`), set a little below that measurement
/// rather than guessed: a guard that checks nothing passes, and a floor
/// invented rather than measured proves nothing.
const MIN_FILES_VISITED: usize = 60;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Return every `.rs` file under every crate's own `src`, `tests` and
/// `examples` directories, sorted, so this guard's own walk visits files
/// in the same order on every run and every platform. Not restricted to
/// one crate: `HOST_FONT_LOADER` is a `fontdb` method, and any crate that
/// gains a path to `fontdb` gains a path to it.
fn rust_files_under_workspace(root: &Path) -> Vec<PathBuf> {
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
        for subdir in ["src", "tests", "examples"] {
            let dir = crate_dir.join(subdir);
            if dir.is_dir() {
                collect_rust_files(&dir, &mut files);
            }
        }
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
/// source code only: a mention of `HOST_FONT_LOADER` in prose, or hidden
/// inside a string literal, can no longer trip the search below, and a
/// real call cannot hide inside a string either.
///
/// A copy of `crates/chrys-cli/tests/decode_limits_guard.rs`'s own
/// `strip_comments_and_strings`, itself a copy of
/// `crates/chrys-core/tests/determinism.rs`'s original; see this file's
/// own module comment for why a third copy exists rather than a shared
/// one.
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

/// Search `stripped` (already run through `strip_comments_and_strings`)
/// for the literal text `needle`, and return the one-based line number of
/// every occurrence. Unlike a method-call search, this does not assume a
/// leading `.`, so it also finds a path-qualified reference such as
/// `fontdb::Database::load_system_fonts`.
fn find_substring_occurrences(stripped: &str, needle: &str) -> Vec<usize> {
    let mut hits = Vec::new();
    for (line_index, line) in stripped.lines().enumerate() {
        if line.contains(needle) {
            hits.push(line_index + 1);
        }
    }
    hits
}

/// Every hit this module's search found: the file it was in and the
/// one-based line number.
struct Hit {
    file: PathBuf,
    line: usize,
}

impl std::fmt::Display for Hit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.file.display(), self.line)
    }
}

/// `path`, relative to `root`, with every path separator normalised to
/// `/` so this guard reads the same relative path on every platform.
fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn no_source_file_in_the_workspace_names_the_host_font_loader() {
    let root = workspace_root();
    let files = rust_files_under_workspace(&root);

    assert!(
        files.len() >= MIN_FILES_VISITED,
        "this guard's own file walk visited only {} .rs files, below the measured floor of {}; \
         a walk that stops finding files must not read as a pass",
        files.len(),
        MIN_FILES_VISITED
    );

    let mut hits: Vec<Hit> = Vec::new();
    for file in &files {
        let source = fs::read_to_string(file)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()));
        let stripped = strip_comments_and_strings(&source);
        for line in find_substring_occurrences(&stripped, HOST_FONT_LOADER) {
            hits.push(Hit {
                file: file.clone(),
                line,
            });
        }
    }

    let offenders: Vec<&Hit> = hits
        .iter()
        .filter(|hit| {
            let relative = relative_to(&root, &hit.file);
            !ALLOW_LIST.iter().any(|allowed| allowed.path == relative)
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "the following source location(s) name `{HOST_FONT_LOADER}`, the host-font-database \
         loader DET-05 forbids, outside this guard's own one-entry allow-list:\n{}",
        offenders
            .iter()
            .map(|hit| hit.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Every allow-list entry names why it is allowed, not only which file it
/// covers, and the list holds exactly one entry: this guard's own file,
/// which names the symbol it searches for.
#[test]
fn every_allow_listed_file_names_its_own_reason() {
    assert_eq!(
        ALLOW_LIST.len(),
        1,
        "the allow-list must hold exactly one entry: this guard's own file"
    );
    for allowed in ALLOW_LIST {
        assert!(
            !allowed.reason.trim().is_empty(),
            "{} has no reason recorded in ALLOW_LIST",
            allowed.path
        );
    }
}
