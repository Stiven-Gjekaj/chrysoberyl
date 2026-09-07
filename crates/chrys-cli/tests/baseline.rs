//! Integration tests for the baseline store's own CLI surface: `chrys
//! accept` and `chrys compare --baseline`.
//!
//! Every test here runs the real built binary through
//! `Command::new(env!("CARGO_BIN_EXE_chrys"))`, the same pattern
//! `digest.rs` and `report.rs` already use.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A fresh, unique temporary store root this test module owns, removed on
/// drop.
struct TempStore {
    root: PathBuf,
}

impl TempStore {
    fn new(label: &str) -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "chrys-cli-baseline-test-{label}-{}-{n}",
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

#[test]
fn a_manifest_digest_that_disagrees_with_its_file_fails_the_run() {
    let store = TempStore::new("disagreeing-digest");
    let root = repo_root();

    let accept_output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("accept")
        .arg("pair-01")
        .arg(root.join("tests/golden/pair-01/base.png"))
        .arg("--store")
        .arg(&store.root)
        .output()
        .expect("the chrys binary runs");
    assert!(
        accept_output.status.success(),
        "accept did not exit 0: {}",
        String::from_utf8_lossy(&accept_output.stderr)
    );

    let manifest_path = store.root.join("MANIFEST.toml");
    let mut manifest_text =
        std::fs::read_to_string(&manifest_path).expect("read the manifest accept wrote");

    // Flip one hexadecimal character of the recorded digest, keeping the
    // manifest well formed so the failure below comes from the digest
    // disagreement, not from a parse error.
    let digest_key = "digest = \"";
    let digest_key_at = manifest_text
        .find(digest_key)
        .expect("the manifest carries a digest line");
    let digest_value_start = digest_key_at + digest_key.len();
    let original_digest = manifest_text[digest_value_start..digest_value_start + 64].to_string();
    let flipped_char = if manifest_text.as_bytes()[digest_value_start] == b'0' {
        '1'
    } else {
        '0'
    };
    manifest_text.replace_range(
        digest_value_start..digest_value_start + 1,
        &flipped_char.to_string(),
    );
    let flipped_digest = manifest_text[digest_value_start..digest_value_start + 64].to_string();
    assert_ne!(
        original_digest, flipped_digest,
        "the edit must actually change the recorded digest"
    );
    std::fs::write(&manifest_path, &manifest_text).expect("write the edited manifest");

    let compare_output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg("--baseline")
        .arg("pair-01")
        .arg("--store")
        .arg(&store.root)
        .arg(root.join("tests/golden/pair-01/candidate.png"))
        .output()
        .expect("the chrys binary runs");

    assert!(
        !compare_output.status.success(),
        "a manifest digest that disagrees with its file must fail the run"
    );
    let stderr = String::from_utf8_lossy(&compare_output.stderr);
    assert!(
        stderr.contains("pair-01"),
        "stderr does not name the baseline: {stderr}"
    );
    assert!(
        stderr.contains(&flipped_digest),
        "stderr does not name the digest the manifest recorded: {stderr}"
    );
    assert!(
        stderr.contains(&original_digest),
        "stderr does not name the digest the stored file produces: {stderr}"
    );
}

#[test]
fn a_baseline_that_was_never_accepted_fails_loudly() {
    let store = TempStore::new("never-accepted");
    let root = repo_root();

    let compare_output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg("--baseline")
        .arg("never-accepted")
        .arg("--store")
        .arg(&store.root)
        .arg(root.join("tests/golden/pair-01/candidate.png"))
        .output()
        .expect("the chrys binary runs");

    assert!(
        !compare_output.status.success(),
        "a comparison against a name nothing accepted must fail, not silently accept it"
    );
    let stderr = String::from_utf8_lossy(&compare_output.stderr);
    assert!(
        stderr.contains("never-accepted"),
        "stderr does not name the baseline: {stderr}"
    );
    assert!(
        stderr.contains(&store.root.display().to_string()),
        "stderr does not name the store root: {stderr}"
    );
    assert!(
        !store.root.exists(),
        "resolve must write nothing: a failed comparison must not create the store directory"
    );
}

#[test]
fn accept_help_states_that_an_accept_is_a_statement() {
    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("accept")
        .arg("--help")
        .output()
        .expect("the chrys binary runs");
    assert!(output.status.success(), "accept --help must exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("statement"),
        "accept --help does not say plainly that an accept is a statement: {stdout}"
    );
}
