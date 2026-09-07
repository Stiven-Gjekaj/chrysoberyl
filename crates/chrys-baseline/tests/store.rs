//! Integration tests for `GoldenFileStore`, `chrys-baseline`'s own
//! committed-golden backend.
//!
//! **Why this file holds a third comment-and-string stripper.**
//! `crates/chrys-core/tests/determinism.rs` holds the first copy, and
//! `crates/chrys-cli/tests/decode_limits_guard.rs` holds a second, because
//! D-03 forbids editing anything under `crates/chrys-core/`, including its
//! own test directory, to export the first copy. This crate's own test
//! target cannot import the second copy either: it is `fn`, private to
//! `chrys-cli`'s own test binary, in a different crate entirely. Both
//! existing copies remain authoritative for their own guard; this one
//! exists only because neither is reachable from here.

use std::fs;
use std::path::{Path, PathBuf};

use chrys_baseline::BaselineStore;
use chrys_baseline::golden::GoldenFileStore;

/// A fresh temporary directory this test module owns, removed on drop. A
/// process ID plus a counter keeps two tests, or two runs of the whole
/// suite, from ever sharing one directory.
struct TempStore {
    root: PathBuf,
}

impl TempStore {
    fn new(label: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "chrys-baseline-test-{label}-{}-{n}",
            std::process::id()
        ));
        TempStore { root }
    }
}

impl Drop for TempStore {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

/// Write `contents` to a fresh file named `name` under a fresh temporary
/// directory, and return its path.
fn write_candidate_file(label: &str, name: &str, contents: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "chrys-baseline-candidate-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create candidate directory");
    let path = dir.join(name);
    std::fs::write(&path, contents).expect("write candidate file");
    path
}

/// Write every `(name, contents)` pair in `files` under a fresh temporary
/// directory, and return the directory's own path.
fn write_candidate_dir(label: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "chrys-baseline-candidate-dir-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create candidate directory");
    for (name, contents) in files {
        std::fs::write(dir.join(name), contents).expect("write candidate file");
    }
    dir
}

#[test]
fn accept_writes_a_manifest_toml_file() {
    let store_dir = TempStore::new("accept-writes-manifest");
    let candidate = write_candidate_file("accept-writes-manifest", "base.png", b"pixel bytes");
    let store = GoldenFileStore::new(&store_dir.root);

    store
        .accept(
            "pair-01",
            &candidate,
            &[("base.png".to_string(), "deadbeef".to_string())],
        )
        .expect("accept succeeds");

    let manifest_path = store_dir.root.join("MANIFEST.toml");
    assert!(
        manifest_path.is_file(),
        "accept did not write {}",
        manifest_path.display()
    );
    let text = std::fs::read_to_string(&manifest_path).expect("read manifest");
    assert!(
        text.contains("pair-01"),
        "manifest does not name pair-01: {text}"
    );
    assert!(
        text.contains("base.png"),
        "manifest does not name base.png: {text}"
    );
    assert!(
        text.contains("deadbeef"),
        "manifest does not carry the digest: {text}"
    );
}

#[test]
fn resolving_an_unaccepted_name_is_an_error() {
    let store_dir = TempStore::new("resolve-unaccepted");
    let store = GoldenFileStore::new(&store_dir.root);

    let error = store
        .resolve("never-accepted")
        .expect_err("a name nothing accepted is an error, not a path");
    let message = error.to_string();
    assert!(
        message.contains("never-accepted"),
        "error does not name the baseline: {message}"
    );
    assert!(
        message.contains(&store_dir.root.display().to_string()),
        "error does not name the store root: {message}"
    );
}

#[test]
fn accept_copies_the_candidate_file_byte_for_byte() {
    let store_dir = TempStore::new("accept-copies-bytes");
    let candidate_dir = write_candidate_dir(
        "accept-copies-bytes",
        &[
            ("frame1.png", b"\x00\x01\x02frame-one-bytes\xff"),
            ("frame2.png", b"\xffsecond-frame-bytes\x00\x01"),
        ],
    );
    let store = GoldenFileStore::new(&store_dir.root);

    store
        .accept(
            "sequence-01",
            &candidate_dir,
            &[
                ("frame1.png".to_string(), "digest-a".to_string()),
                ("frame2.png".to_string(), "digest-b".to_string()),
            ],
        )
        .expect("accept succeeds");

    let baseline_dir = store_dir.root.join("sequence-01");
    for name in ["frame1.png", "frame2.png"] {
        let original = std::fs::read(candidate_dir.join(name)).expect("read the original file");
        let stored = std::fs::read(baseline_dir.join(name)).expect("read the stored file");
        // Assert byte equality, not size equality: two files of the same
        // length with different contents would still pass a size check,
        // and AGENTS.md already records that a size is not a state.
        assert_eq!(
            original, stored,
            "{name} was not copied byte for byte into the store"
        );
    }

    std::fs::remove_dir_all(&candidate_dir).ok();
}

#[test]
fn resolve_survives_a_rename_inside_the_baseline_directory() {
    let store_dir = TempStore::new("resolve-survives-rename");
    let candidate =
        write_candidate_file("resolve-survives-rename", "base.png", b"single-file-bytes");
    let store = GoldenFileStore::new(&store_dir.root);

    store
        .accept(
            "pair-01",
            &candidate,
            &[("base.png".to_string(), "deadbeef".to_string())],
        )
        .expect("accept succeeds");

    let baseline_dir = store_dir.root.join("pair-01");
    let stored_original = baseline_dir.join("base.png");
    let stored_renamed = baseline_dir.join("renamed-by-a-person.png");
    std::fs::rename(&stored_original, &stored_renamed)
        .expect("rename the stored file inside its own directory");

    let resolved = store
        .resolve("pair-01")
        .expect("resolve still finds the baseline after the rename");
    assert_eq!(
        resolved, stored_renamed,
        "resolve did not find the renamed file; it must list the directory, not trust the \
         manifest's own source value"
    );
}

/// Every path under `root`, with its own length and modification time, so
/// a test can tell whether an operation changed anything at all. A count
/// alone is not a state (`AGENTS.md`, "What to measure").
fn snapshot(root: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    let mut entries = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let read_dir = match fs::read_dir(&dir) {
            Ok(read_dir) => read_dir,
            Err(_) => continue,
        };
        for entry in read_dir {
            let entry = entry.expect("read a directory entry");
            let path = entry.path();
            let metadata = entry.metadata().expect("read metadata");
            if metadata.is_dir() {
                stack.push(path.clone());
            }
            entries.push((
                path,
                metadata.len(),
                metadata.modified().expect("read the modified time"),
            ));
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

#[test]
fn resolve_writes_nothing_to_the_store() {
    let store_dir = TempStore::new("resolve-writes-nothing");
    let candidate = write_candidate_file(
        "resolve-writes-nothing",
        "base.png",
        b"resolve-writes-nothing",
    );
    let store = GoldenFileStore::new(&store_dir.root);

    store
        .accept(
            "pair-01",
            &candidate,
            &[("base.png".to_string(), "deadbeef".to_string())],
        )
        .expect("accept succeeds");

    let before = snapshot(&store_dir.root);

    store
        .resolve("pair-01")
        .expect("resolve finds the accepted baseline");
    let after_a_successful_resolve = snapshot(&store_dir.root);
    assert_eq!(
        before, after_a_successful_resolve,
        "a successful resolve wrote to the store"
    );

    let _ = store.resolve("never-accepted");
    let after_a_failed_resolve = snapshot(&store_dir.root);
    assert_eq!(
        before, after_a_failed_resolve,
        "a failed resolve wrote to the store"
    );
}

/// Read every file directly under `dir` (no recursion, matching
/// `write_baseline`'s own directory-candidate rule), as `(name, bytes)`
/// pairs, sorted by name. Bytes, not lengths and not a count: `AGENTS.md`
/// already records that a size is not a state.
fn read_all_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()))
        .map(|entry| entry.expect("read a directory entry"))
        .filter(|entry| entry.metadata().expect("read metadata").is_file())
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = fs::read(entry.path()).expect("read a stored file's bytes");
            (name, bytes)
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// BASE-04, T-03-30: a failed accept destroys nothing. This interrupts a
/// real `write_baseline` call after every candidate file is already
/// staged, by making the manifest's own temporary path a directory
/// instead of a file, which fails `fs::write` on every platform this
/// project's own CI matrix runs, with no permission change required.
#[test]
fn a_failed_accept_leaves_the_previous_baseline_intact() {
    let store_dir = TempStore::new("failed-accept-leaves-previous-intact");
    let store = GoldenFileStore::new(&store_dir.root);

    let first_candidate = write_candidate_file(
        "failed-accept-leaves-previous-intact-first",
        "base.png",
        b"the-first-accepted-bytes",
    );
    store
        .accept(
            "pair-01",
            &first_candidate,
            &[("base.png".to_string(), "first-digest".to_string())],
        )
        .expect("the first accept succeeds");

    let baseline_dir = store_dir.root.join("pair-01");
    let manifest_path = store_dir.root.join("MANIFEST.toml");
    let files_before = read_all_files(&baseline_dir);
    let manifest_before = fs::read(&manifest_path).expect("read the manifest before the drill");

    // A directory at the manifest's own temporary path makes fs::write
    // fail at exactly the point CR-02 names: after every candidate file
    // is already copied into the staging directory, but before either
    // rename that touches what a reader sees.
    let manifest_tmp_path = store_dir.root.join("MANIFEST.toml.tmp");
    fs::create_dir_all(&manifest_tmp_path).expect("plant a directory at the tmp manifest path");

    let second_candidate = write_candidate_file(
        "failed-accept-leaves-previous-intact-second",
        "base.png",
        b"a-different-second-candidate-bytes",
    );
    let error = store
        .accept(
            "pair-01",
            &second_candidate,
            &[("base.png".to_string(), "second-digest".to_string())],
        )
        .expect_err("a manifest write that fails must fail the whole accept");
    drop(error);

    let files_after = read_all_files(&baseline_dir);
    let manifest_after = fs::read(&manifest_path).expect("read the manifest after the drill");
    assert_eq!(
        files_before, files_after,
        "a failed accept changed the previously accepted baseline's own bytes"
    );
    assert_eq!(
        manifest_before, manifest_after,
        "a failed accept changed MANIFEST.toml's own bytes"
    );

    let resolved = store
        .resolve("pair-01")
        .expect("resolve still finds the previous baseline after the failed accept");
    let resolved_bytes = fs::read(&resolved).expect("read the resolved file's bytes");
    assert_eq!(
        resolved_bytes, b"the-first-accepted-bytes",
        "resolve should still return the first candidate's own bytes, not the second's"
    );
}

/// T-03-31: a staging directory left behind by an earlier crashed accept
/// holds a file from a candidate nobody accepted; it must contribute
/// nothing to the next accepted baseline.
#[test]
fn a_stale_staging_directory_contributes_no_file_to_the_next_accept() {
    let store_dir = TempStore::new("stale-staging-contributes-nothing");
    let store = GoldenFileStore::new(&store_dir.root);

    let staging_dir = store_dir.root.join(".pair-01.accept-tmp");
    fs::create_dir_all(&staging_dir).expect("plant a stale staging directory");
    fs::write(
        staging_dir.join("leftover-from-a-crash.png"),
        b"nobody accepted this",
    )
    .expect("plant a stale file inside the staging directory");

    let candidate = write_candidate_file(
        "stale-staging-contributes-nothing",
        "base.png",
        b"the-only-file-this-accept-should-store",
    );
    store
        .accept(
            "pair-01",
            &candidate,
            &[("base.png".to_string(), "deadbeef".to_string())],
        )
        .expect("accept succeeds");

    let baseline_dir = store_dir.root.join("pair-01");
    let stored_files = read_all_files(&baseline_dir);
    let stored_names: Vec<&str> = stored_files.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        stored_names,
        vec!["base.png"],
        "the accepted baseline should hold only base.png, not the stale staged file: {stored_names:?}"
    );
}

/// A smaller copy of `crates/chrys-cli/tests/decode_limits_guard.rs`'s own
/// `strip_comments_and_strings`; see this file's own module comment for
/// why a third copy exists rather than a shared one.
fn strip_comments_and_strings(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;

    while i < n {
        let c = chars[i];

        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

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

        if let Some(after) = raw_string_end(&chars, i) {
            i = after;
            continue;
        }

        if c == '"' || (c == 'b' && i + 1 < n && chars[i + 1] == '"') {
            let mut j = if c == 'b' { i + 1 } else { i };
            j += 1;
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

/// Every write call this guard searches for. A new way to write must be
/// added to this list, deliberately, or the guard stops covering it.
///
/// `fs::rename(` was added by the staged-write plan that introduced
/// `write_baseline`'s swap-into-place step: this is the case this
/// constant's own comment was written for.
const WRITE_CALLS: &[&str] = &[
    "fs::create_dir_all(",
    "fs::remove_dir_all(",
    "fs::remove_file(",
    "fs::copy(",
    "fs::write(",
    "fs::rename(",
];

/// Return every `.rs` file directly under `dir` (this crate's own `src`
/// holds no subdirectory, so this does not need to recurse).
fn rust_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()))
        .map(|entry| entry.expect("read a directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    files
}

/// Return the byte-offset span `(open_brace + 1, close_brace)` of the body
/// of the function named `name` (searched for as `"fn {name}("`) in
/// `stripped` (already run through `strip_comments_and_strings`).
fn function_body_span(stripped: &str, name: &str) -> (usize, usize) {
    let needle = format!("fn {name}(");
    let start = stripped
        .find(&needle)
        .unwrap_or_else(|| panic!("no `fn {name}` found"));
    let bytes = stripped.as_bytes();
    let open = stripped[start..]
        .find('{')
        .map(|i| start + i)
        .unwrap_or_else(|| panic!("fn {name} has no body"));
    let mut depth = 0i32;
    let mut idx = open;
    loop {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return (open + 1, idx);
                }
            }
            _ => {}
        }
        idx += 1;
        if idx >= bytes.len() {
            panic!("fn {name}'s body never closes");
        }
    }
}

/// Return the name of the function whose `fn` keyword most closely
/// precedes byte offset `at` in `stripped` (already run through
/// `strip_comments_and_strings`), so a failure can name the function an
/// offending call was found inside, not only its byte offset.
fn enclosing_function_name(stripped: &str, at: usize) -> String {
    let before = &stripped[..at];
    let fn_at = before.rfind("fn ").unwrap_or_else(|| {
        panic!("no `fn` keyword precedes offset {at}; the offending call is outside any function")
    });
    let after_fn = &stripped[fn_at + "fn ".len()..];
    after_fn
        .split(|c: char| c == '(' || c.is_whitespace())
        .next()
        .unwrap_or("")
        .to_string()
}

#[test]
fn only_the_accept_path_writes_to_the_store() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = rust_files_under(&src_dir);
    assert!(
        !files.is_empty(),
        "the file walk found no .rs file under {}",
        src_dir.display()
    );

    let golden_path = src_dir.join("golden.rs");
    assert!(
        files.contains(&golden_path),
        "the walk did not find {}",
        golden_path.display()
    );
    let golden_source = fs::read_to_string(&golden_path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", golden_path.display()));
    let golden_stripped = strip_comments_and_strings(&golden_source);
    let (write_body_start, write_body_end) = function_body_span(&golden_stripped, "write_baseline");

    let mut offenders: Vec<(PathBuf, &str, usize, String)> = Vec::new();
    for file in &files {
        let source = fs::read_to_string(file)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()));
        let stripped = strip_comments_and_strings(&source);
        for &call in WRITE_CALLS {
            let mut search_from = 0usize;
            while let Some(relative) = stripped[search_from..].find(call) {
                let at = search_from + relative;
                let inside_write_baseline =
                    *file == golden_path && at >= write_body_start && at < write_body_end;
                if !inside_write_baseline {
                    let enclosing_fn = enclosing_function_name(&stripped, at);
                    offenders.push((file.clone(), call, at, enclosing_fn));
                }
                search_from = at + call.len();
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "the following write call(s) live outside write_baseline, the one function every write \
         in this crate must go through: {}",
        offenders
            .iter()
            .map(|(file, call, at, enclosing_fn)| format!(
                "{}@{at}, inside fn {enclosing_fn}: `{call}`",
                file.display()
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );

    // write_baseline itself must be called from exactly one place: inside
    // `accept`. The identifier appears once as its own definition
    // (`fn write_baseline(`) and must appear exactly once more, as a call.
    let mut call_sites: Vec<usize> = Vec::new();
    let mut definition_sites: Vec<usize> = Vec::new();
    for (at, _) in golden_stripped.match_indices("write_baseline(") {
        if golden_stripped[..at].trim_end().ends_with("fn") {
            definition_sites.push(at);
        } else {
            call_sites.push(at);
        }
    }
    assert_eq!(
        definition_sites.len(),
        1,
        "write_baseline must be defined exactly once"
    );
    assert_eq!(
        call_sites.len(),
        1,
        "write_baseline must be called from exactly one place, found {}",
        call_sites.len()
    );

    let (accept_body_start, accept_body_end) = function_body_span(&golden_stripped, "accept");
    let call_at = call_sites[0];
    assert!(
        call_at >= accept_body_start && call_at < accept_body_end,
        "write_baseline's only call site is not inside accept's own body"
    );
}

#[test]
fn the_trait_has_exactly_resolve_and_accept() {
    let lib_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let source = fs::read_to_string(&lib_path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", lib_path.display()));
    let stripped = strip_comments_and_strings(&source);

    let trait_start = stripped
        .find("trait BaselineStore")
        .expect("BaselineStore is declared in lib.rs");
    let open = stripped[trait_start..]
        .find('{')
        .map(|i| trait_start + i)
        .expect("the trait has a body");
    let bytes = stripped.as_bytes();
    let mut depth = 0i32;
    let mut idx = open;
    let close = loop {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    break idx;
                }
            }
            _ => {}
        }
        idx += 1;
    };
    let body = &stripped[open + 1..close];

    let method_count = body.matches("fn ").count();
    assert_eq!(
        method_count, 2,
        "BaselineStore must declare exactly two methods, found {method_count}: {body}"
    );
    assert!(
        body.contains("fn resolve"),
        "the trait does not declare resolve: {body}"
    );
    assert!(
        body.contains("fn accept"),
        "the trait does not declare accept: {body}"
    );
}
