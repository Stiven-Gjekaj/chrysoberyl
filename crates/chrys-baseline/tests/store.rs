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
