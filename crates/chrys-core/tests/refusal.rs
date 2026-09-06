//! The CORE-06/CORE-07 refusal tests.
//!
//! This file has two jobs. The first is the separation test: it measures
//! `register::confidence::assess_peak`'s ratio over every pair in the
//! committed `tests/golden/refuse-01/` corpus, and proves the two groups
//! separate cleanly. Those measured numbers are the evidence
//! `REFUSAL_THRESHOLD` is derived from, in `register::confidence`, not a
//! number chosen by inspection.
//!
//! The second job, added once the threshold exists, is the refusal
//! behaviour itself: that `compare` refuses every `should-refuse` pair and
//! never refuses a `should-register` pair, in words and in the CLI's exit
//! code.

use std::path::{Path, PathBuf};

use chrys_core::register::{assess_peak, peak_index, phase_correlate};
use chrys_source::{Frame, Source};
use chrys_source_raster::RasterSource;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One pair's ratio, tagged with the directory it came from, so a failing
/// assertion can name the pair that broke the separation instead of only
/// reporting the numbers.
struct MeasuredPair {
    directory: String,
    ratio: f32,
}

/// Load `path` as a single frame through the real raster decoder, the same
/// decoder `chrys-cli` uses. This exercises the whole pipeline the
/// separation test measures: real PNG bytes in, a phase-correlation ratio
/// out.
fn load_frame(path: &Path) -> Frame {
    let source = RasterSource::new();
    let mut frames = source
        .load(path)
        .unwrap_or_else(|error| panic!("decoding {}: {error}", path.display()));
    assert!(
        !frames.is_empty(),
        "{} decoded to zero frames",
        path.display()
    );
    frames.remove(0)
}

/// Measure the peak-confidence ratio for the `base.png`/`candidate.png`
/// pair at `pair_dir`.
fn measure_pair(pair_dir: &Path) -> f32 {
    let base = load_frame(&pair_dir.join("base.png"));
    let candidate = load_frame(&pair_dir.join("candidate.png"));
    let (refined, surface) = phase_correlate(&base, &candidate)
        .unwrap_or_else(|error| panic!("correlating {}: {error}", pair_dir.display()));
    let index = peak_index(refined.whole, surface.resolution);
    assess_peak(&surface, index).ratio
}

/// Measure every pair directory under `group_dir`, in a fixed, sorted
/// order, so this function's own output does not depend on the order the
/// filesystem happens to list entries in.
fn measure_group(group_dir: &Path) -> Vec<MeasuredPair> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(group_dir)
        .unwrap_or_else(|error| panic!("reading {}: {error}", group_dir.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    entries.sort();

    entries
        .into_iter()
        .map(|pair_dir| {
            let ratio = measure_pair(&pair_dir);
            let directory = pair_dir
                .file_name()
                .expect("a pair directory has a name")
                .to_string_lossy()
                .into_owned();
            MeasuredPair { directory, ratio }
        })
        .collect()
}

fn refuse_01_root() -> PathBuf {
    repo_root().join("tests/golden/refuse-01")
}

#[test]
fn should_register_pairs_all_score_above_every_should_refuse_pair() {
    let should_register = measure_group(&refuse_01_root().join("should-register"));
    let should_refuse = measure_group(&refuse_01_root().join("should-refuse"));

    assert!(
        should_register.len() >= 4,
        "expected at least four should-register pairs, found {}",
        should_register.len()
    );
    assert!(
        should_refuse.len() >= 4,
        "expected at least four should-refuse pairs, found {}",
        should_refuse.len()
    );

    let lowest_register = should_register
        .iter()
        .min_by(|a, b| a.ratio.total_cmp(&b.ratio))
        .expect("at least one should-register pair");
    let highest_refuse = should_refuse
        .iter()
        .max_by(|a, b| a.ratio.total_cmp(&b.ratio))
        .expect("at least one should-refuse pair");

    if lowest_register.ratio <= highest_refuse.ratio {
        print_measured("should-register", &should_register);
        print_measured("should-refuse", &should_refuse);
        panic!(
            "separation failed: should-register/{} scored {}, \
             should-refuse/{} scored {}",
            lowest_register.directory,
            lowest_register.ratio,
            highest_refuse.directory,
            highest_refuse.ratio
        );
    }

    let gap = lowest_register.ratio - highest_refuse.ratio;
    if gap <= 0.0 {
        print_measured("should-register", &should_register);
        print_measured("should-refuse", &should_refuse);
    }
    assert!(gap > 0.0, "the gap between the two groups must be positive");

    print_measured("should-register", &should_register);
    print_measured("should-refuse", &should_refuse);
    println!(
        "lowest should-register ratio: {} ({})",
        lowest_register.ratio, lowest_register.directory
    );
    println!(
        "highest should-refuse ratio: {} ({})",
        highest_refuse.ratio, highest_refuse.directory
    );
    println!("gap: {gap}");
}

fn print_measured(group: &str, pairs: &[MeasuredPair]) {
    for pair in pairs {
        println!("{group}/{}: ratio {}", pair.directory, pair.ratio);
    }
}
