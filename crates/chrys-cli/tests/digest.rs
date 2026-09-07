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

/// Run `--hash-only` over the committed 11-frame `tests/golden/sequence-01`
/// pair, the fixture plan 02-01 committed.
fn run_hash_only_sequence() -> String {
    let root = repo_root();
    let output = Command::new(env!("CARGO_BIN_EXE_chrys"))
        .arg("compare")
        .arg(root.join("tests/golden/sequence-01/base"))
        .arg(root.join("tests/golden/sequence-01/candidate"))
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

#[test]
fn a_sequence_reports_one_digest_block_per_frame_index() {
    let live = run_hash_only_sequence();
    let lines: Vec<&str> = live.lines().collect();
    assert_eq!(
        lines.len(),
        55,
        "expected 11 header lines and 44 digest lines (55 total), got {}:\n{live}",
        lines.len()
    );

    let mut header_indices: Vec<u32> = Vec::new();
    for chunk in lines.chunks(5) {
        assert_eq!(
            chunk.len(),
            5,
            "a frame block holds a header and four digest lines"
        );
        let header = chunk[0];
        let index_text = header
            .strip_prefix("frame ")
            .unwrap_or_else(|| panic!("header line does not start with 'frame ': {header}"));
        let index: u32 = index_text
            .parse()
            .unwrap_or_else(|error| panic!("header index is not a number ({error}): {header}"));
        header_indices.push(index);

        for digest_line in &chunk[1..] {
            let digest = digest_line
                .split(' ')
                .nth(1)
                .unwrap_or_else(|| panic!("line has no digest field: {digest_line}"));
            assert_eq!(digest.len(), 64, "digest is not 64 characters: {digest}");
            assert!(
                digest
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "digest is not lowercase hexadecimal: {digest}"
            );
        }
    }

    let expected_indices: Vec<u32> = (0..11).collect();
    assert_eq!(
        header_indices, expected_indices,
        "the header indices must run 0 through 10 in order"
    );
}

#[test]
fn a_second_run_over_the_sequence_gives_byte_identical_stdout() {
    let first = run_hash_only_sequence();
    let second = run_hash_only_sequence();
    assert_eq!(first, second);
}
