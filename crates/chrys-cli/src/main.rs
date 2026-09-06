//! The `chrys` binary. It wires a source adapter to the engine and prints
//! the verdict.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrys_source::{Frame, Source};
use chrys_source_raster::RasterSource;
use chrys_source_sequence::SequenceSource;
use clap::{Parser, Subcommand};

/// A structural diff for raster images.
#[derive(Parser)]
#[command(name = "chrys")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare two raster images and print a verdict.
    Compare {
        /// The path to the base image.
        base: PathBuf,
        /// The path to the candidate image.
        candidate: PathBuf,
        /// Print the four-line digest report instead of the verdict text,
        /// and exit 0 even when the pair differs. Every digest covers raw
        /// RGBA8 bytes or the verdict's `Display` text, never a re-encoded
        /// file.
        #[arg(long)]
        hash_only: bool,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            // An error goes to stderr only, so stdout stays parseable.
            eprintln!("{error}");
            ExitCode::from(3)
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    match cli.command {
        Command::Compare {
            base,
            candidate,
            hash_only,
        } => run_compare(&base, &candidate, hash_only),
    }
}

fn run_compare(
    base_path: &Path,
    candidate_path: &Path,
    hash_only: bool,
) -> anyhow::Result<ExitCode> {
    let base_frames = frames_for(base_path)?;
    let candidate_frames = frames_for(candidate_path)?;

    let verdicts = chrys_core::compare_sequence(&base_frames, &candidate_frames)?;

    if hash_only {
        let base_frame = base_frames
            .first()
            .ok_or_else(|| anyhow::anyhow!("{} decoded to zero frames", base_path.display()))?;
        let candidate_frame = candidate_frames.first().ok_or_else(|| {
            anyhow::anyhow!("{} decoded to zero frames", candidate_path.display())
        })?;
        let verdict = verdicts
            .first()
            .ok_or_else(|| anyhow::anyhow!("compare_sequence returned no verdict"))?;
        let digests = chrys_core::hash::digest_report(base_frame, candidate_frame, verdict)?;
        print!("{digests}");
        return Ok(ExitCode::from(0));
    }

    // A one-against-one sequence prints exactly what phase 1's `compare`
    // printed, so a single-file pair's stdout stays byte-identical to what
    // it was before this crate learned about sequences.
    if verdicts.len() == 1 {
        print!("{}", verdicts[0]);
    } else {
        for (index, verdict) in verdicts.iter().enumerate() {
            println!("frame {index}");
            print!("{verdict}");
        }
    }

    let worst = verdicts.iter().map(verdict_rank).max().unwrap_or(0);
    Ok(ExitCode::from(worst))
}

/// Rank a verdict for the process exit code: refused above changed above
/// identical. A caller branching on the exit code of a sequence pair keeps
/// the same three meanings a single-pair comparison already reports.
fn verdict_rank(verdict: &chrys_core::Verdict) -> u8 {
    match verdict {
        chrys_core::Verdict::Identical => 0,
        chrys_core::Verdict::Changed { .. } => 1,
        chrys_core::Verdict::Refused { .. } => 2,
    }
}

/// Load the frames at `path`. A directory loads as a numbered frame
/// sequence through `SequenceSource`; any other path loads as a single
/// raster image through `RasterSource`.
fn frames_for(path: &Path) -> anyhow::Result<Vec<Frame>> {
    if path.is_dir() {
        Ok(SequenceSource::new().load(path)?)
    } else {
        Ok(RasterSource::new().load(path)?)
    }
}
