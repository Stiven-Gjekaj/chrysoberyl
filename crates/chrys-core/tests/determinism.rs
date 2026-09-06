//! Guards for DET-06 and, once Task 2 adds it, DET-02: the comparison path
//! calls no platform transcendental function, and no crate that can
//! produce a GPU pixel or decode a raster format enters `chrys-core`'s own
//! dependency graph.
//!
//! **What DET-06 turned out to mean.** The requirement reads as one rule,
//! but it splits into two very unequal halves.
//!
//! The first half, disabling floating-point contraction on the comparison
//! path, needs no runtime work, because there is no default to switch
//! off. Rust's own fused-multiply-add intrinsic documentation
//! (`doc.rust-lang.org/nightly/core/intrinsics/fn.fmuladdf64.html`) states
//! that the fused-or-not choice for an ordinary `a * b + c` is reachable
//! only through that unstable, nightly-only intrinsic, never through
//! plain arithmetic in stable Rust. Stable Rust never silently fuses a
//! multiply and an add on its own. What replaces the disable-contraction
//! work is an audit of the two real risks that remain: an explicit call
//! to the fused multiply-add method, `mul_add`, and a dependency whose
//! own build opts into a fused instruction set or a fast-maths flag for
//! its own compiled code. This module's second test performs the first
//! audit; the second audit is a manifest- and build-configuration read,
//! folded into this module's third test.
//!
//! The second half is exactly as large as it looked. The standard
//! library's own floating-point method documentation
//! (`doc.rust-lang.org/std/primitive.f64.html`) documents `sqrt` and
//! `mul_add` as guaranteed not to change, and documents `sin`, `cos`,
//! `tan`, and every other transcendental this module's forbidden list
//! names as having unspecified precision that varies by platform, by
//! Rust version, and even between two calls inside one execution. A
//! comparison path that reached one of those functions would make the
//! verdict itself a function of which machine ran it. Plans 01-04 and
//! 01-07 already mitigate this, by routing every transcendental on the
//! comparison path through the pure-Rust `libm` crate (directly in
//! `register/window.rs`, and through `palette`'s own `libm` feature in
//! `classify/colour.rs`) instead of the platform library. This module's
//! first test is the guard that keeps that mitigation in place: it fails
//! the moment a future change reaches for the platform method instead of
//! the pure-Rust one.

use std::fs;
use std::path::{Path, PathBuf};

/// Standard-library floating-point methods this project's comparison path
/// may never call. Every one of these is documented, at
/// `doc.rust-lang.org/std/primitive.f64.html`, as having precision that is
/// unspecified and may vary by platform, by Rust version, and even
/// between two calls inside one execution. A verdict that reached any of
/// these would stop being a function of its two input images alone.
const FORBIDDEN_TRANSCENDENTALS: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "exp", "ln", "log", "log2", "log10",
    "powf", "powi", "cbrt", "hypot", "sinh", "cosh", "tanh",
];

/// Standard-library floating-point methods this project's comparison path
/// is free to call directly, with the guarantee that justifies each one.
const ALLOWED_MATHS: &[(&str, &str)] = &[(
    "sqrt",
    "doc.rust-lang.org/std/primitive.f64.html documents sqrt as the \
     correctly rounded result, guaranteed not to change across platforms, \
     Rust versions or calls",
)];

/// The explicit fused multiply-add method. Not forbidden outright: a call
/// site needs an entry in this test's own allow-list (there is none yet)
/// plus a cross-architecture test case near subnormal values, because
/// this method's own software fallback has had real, historical rounding
/// faults in shipped standard libraries. See this module's own doc
/// comment and `.planning/phases/01-raster-engine-and-determinism-proof/
/// 01-RESEARCH.md`'s Pitfall 2.
const FUSED_MULTIPLY_ADD: &str = "mul_add";

/// Return every `.rs` file under `dir`, recursively, sorted so this
/// test's own file walk visits files in the same order on every run and
/// every platform.
fn rust_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = fs::read_dir(&current)
            .unwrap_or_else(|err| panic!("cannot read directory {}: {err}", current.display()));
        for entry in entries {
            let entry = entry.expect("cannot read a directory entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Strip line comments, block comments (including nested ones) and every
/// string, byte-string and character literal out of `source`, replacing
/// each with nothing. What is left is source code only: a mention of a
/// forbidden method's name in prose, or hidden inside a string literal,
/// can no longer trip a search over the result, and a real call can no
/// longer hide inside a string either.
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

/// When `start` opens a raw string or raw byte string, return the index
/// just past its close. Otherwise return `None`.
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

/// When the quote at `quote_at` opens a character literal (or byte
/// character literal) that is actually closed by a matching quote,
/// return the index just past that close. Otherwise return `None`,
/// meaning the quote is a lifetime, not a literal.
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

/// Every hit this module's searches found: the file it was in, the
/// one-based line number, and the method name that matched.
struct Hit {
    file: PathBuf,
    line: usize,
    method: &'static str,
}

impl std::fmt::Display for Hit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{} calls `.{}(`",
            self.file.display(),
            self.line,
            self.method
        )
    }
}

/// Search `stripped` (already run through `strip_comments_and_strings`)
/// for a call to `method` as a method (`.method(`), and return the
/// one-based line number of every occurrence.
fn find_method_calls(stripped: &str, method: &str) -> Vec<usize> {
    let pattern = format!(".{method}(");
    let mut hits = Vec::new();
    for (line_index, line) in stripped.lines().enumerate() {
        if line.contains(&pattern) {
            hits.push(line_index + 1);
        }
    }
    hits
}

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_comparison_path_calls_no_forbidden_transcendental() {
    let mut hits: Vec<Hit> = Vec::new();
    for file in rust_files_under(&src_dir()) {
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("cannot read {}: {err}", file.display()));
        let stripped = strip_comments_and_strings(&source);
        for &method in FORBIDDEN_TRANSCENDENTALS {
            for line in find_method_calls(&stripped, method) {
                hits.push(Hit {
                    file: file.clone(),
                    line,
                    method,
                });
            }
        }
    }

    assert!(
        hits.is_empty(),
        "the comparison path calls a platform transcendental with \
         unspecified, platform-varying precision (permitted: {}):\n{}",
        ALLOWED_MATHS
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(", "),
        hits.iter()
            .map(Hit::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn a_fused_multiply_add_call_needs_an_allow_list_entry_and_a_subnormal_test() {
    let mut hits: Vec<Hit> = Vec::new();
    for file in rust_files_under(&src_dir()) {
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("cannot read {}: {err}", file.display()));
        let stripped = strip_comments_and_strings(&source);
        for line in find_method_calls(&stripped, FUSED_MULTIPLY_ADD) {
            hits.push(Hit {
                file: file.clone(),
                line,
                method: FUSED_MULTIPLY_ADD,
            });
        }
    }

    assert!(
        hits.is_empty(),
        "a call to the explicit fused multiply-add method is not \
         forbidden outright, but it is not free either: add an entry to \
         this test's own allow-list, plus a cross-architecture test case \
         near subnormal values in the cross-architecture matrix, because \
         mul_add's software fallback has had real, historical rounding \
         faults in shipped standard libraries (see 01-RESEARCH.md \
         Pitfall 2):\n{}",
        hits.iter()
            .map(Hit::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_colour_crate_declares_no_default_features_and_the_pure_rust_maths_feature() {
    let manifest_path = workspace_root().join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", manifest_path.display()));

    let palette_line = manifest
        .lines()
        .find(|line| line.trim_start().starts_with("palette "))
        .unwrap_or_else(|| {
            panic!(
                "{} has no `palette` dependency entry to check",
                manifest_path.display()
            )
        });

    assert!(
        palette_line.contains("default-features = false"),
        "{}: `{}` must declare `default-features = false`, so a future \
         default feature cannot reach a platform transcendental behind \
         this project's back",
        manifest_path.display(),
        palette_line.trim()
    );
    assert!(
        palette_line.contains("\"libm\""),
        "{}: `{}` must enable the `libm` feature, which routes every \
         colour transcendental through pure Rust instead of the \
         platform library",
        manifest_path.display(),
        palette_line.trim()
    );

    let cargo_config_path = workspace_root().join(".cargo").join("config.toml");
    if let Ok(cargo_config) = fs::read_to_string(&cargo_config_path) {
        for (line_index, line) in cargo_config.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            assert!(
                !lower.contains("target-feature") && !lower.contains("fast-math"),
                "{}:{}: `{}` enables an extra instruction set or a \
                 fast-maths option for this workspace, either of which \
                 can change comparison-path rounding between machines",
                cargo_config_path.display(),
                line_index + 1,
                line.trim()
            );
        }
    }
}
