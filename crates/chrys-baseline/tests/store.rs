//! Integration tests for `GoldenFileStore`, `chrys-baseline`'s own
//! committed-golden backend.

use std::path::PathBuf;

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
