//! The local determinism guard. It reads the committed decode digest and
//! compares it against a live run of the binary, so a change to decode
//! output goes red here, on this machine, before it ever reaches CI.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run_hash_only() -> String {
    let root = repo_root();
    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(root.join("tests/golden/pair-01/base.png"))
        .arg(root.join("tests/golden/pair-01/candidate.png"))
        .arg("--hash-only")
        .output()
        .expect("the chrys binary runs");
    assert!(output.status.success(), "chrys --hash-only did not exit 0");
    String::from_utf8(output.stdout).expect("stdout is UTF-8")
}

#[test]
fn the_two_decode_lines_match_the_committed_digest_file() {
    let live = run_hash_only();
    let live_lines: Vec<&str> = live.lines().collect();
    let decode_base = live_lines[0];
    let decode_candidate = live_lines[1];

    let committed_path = repo_root().join("tests/golden/pair-01/expected-decode.sha256");
    let committed = std::fs::read_to_string(&committed_path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", committed_path.display()));
    let committed_lines: Vec<&str> = committed.lines().collect();

    assert_eq!(
        committed_lines.len(),
        2,
        "the committed file holds two lines"
    );
    assert_eq!(decode_base, committed_lines[0]);
    assert_eq!(decode_candidate, committed_lines[1]);
}

#[test]
fn the_report_has_four_lines_and_every_digest_is_sixty_four_characters() {
    let live = run_hash_only();
    let lines: Vec<&str> = live.lines().collect();
    assert_eq!(lines.len(), 4, "the digest report has exactly four lines");
    for line in &lines {
        let digest = line
            .split(' ')
            .nth(1)
            .unwrap_or_else(|| panic!("line has no digest field: {line}"));
        assert_eq!(digest.len(), 64, "digest is not 64 characters: {digest}");
        assert!(
            digest
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "digest is not lowercase hexadecimal: {digest}"
        );
    }
}

#[test]
fn a_second_run_gives_byte_identical_stdout() {
    let first = run_hash_only();
    let second = run_hash_only();
    assert_eq!(first, second);
}

/// The names of the four digest lines, in the order
/// `crates/chrys-core/src/hash.rs`'s `DigestSet` prints them, so a
/// mismatch below can name which of the four meanings moved rather than
/// only that two strings differ.
const DIGEST_LINE_NAMES: [&str; 4] = ["decode-base", "decode-candidate", "residual", "verdict"];

#[test]
fn the_four_digests_match_the_committed_digest_file() {
    let live = run_hash_only();
    let live_lines: Vec<&str> = live.lines().collect();
    assert_eq!(
        live_lines.len(),
        4,
        "the live report has exactly four lines"
    );

    let committed_path = repo_root().join("tests/golden/pair-01/expected-digest.sha256");
    let committed = std::fs::read_to_string(&committed_path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", committed_path.display()));
    // A leading `#` line is this fixture's own header comment, read by a
    // person, and is not one of the four digest lines this test compares.
    let committed_lines: Vec<&str> = committed
        .lines()
        .map(str::trim_end) // normalise a Windows checkout's trailing \r
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .collect();
    assert_eq!(
        committed_lines.len(),
        4,
        "the committed file holds four non-comment lines"
    );

    let mut moved: Vec<&str> = Vec::new();
    for (index, name) in DIGEST_LINE_NAMES.iter().enumerate() {
        if live_lines[index] != committed_lines[index] {
            moved.push(name);
        }
    }
    assert!(
        moved.is_empty(),
        "the live digest report no longer matches tests/golden/pair-01/expected-digest.sha256. \
         Line(s) that moved: {}.\nlive:\n{}\ncommitted:\n{}",
        moved.join(", "),
        live_lines.join("\n"),
        committed_lines.join("\n")
    );
}
