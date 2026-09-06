//! The `chrys` binary. It wires a source adapter to the engine and prints
//! the verdict.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrys_source::{Frame, Source};
use chrys_source_raster::RasterSource;
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
    let source = RasterSource::new();
    let base_frame = load_first_frame(&source, base_path)?;
    let candidate_frame = load_first_frame(&source, candidate_path)?;

    let verdict = chrys_core::compare(&base_frame, &candidate_frame)?;

    if hash_only {
        let digests = chrys_core::hash::digest_report(&base_frame, &candidate_frame, &verdict)?;
        print!("{digests}");
        return Ok(ExitCode::from(0));
    }

    print!("{verdict}");

    Ok(match verdict {
        chrys_core::Verdict::Identical => ExitCode::from(0),
        chrys_core::Verdict::Changed { .. } => ExitCode::from(1),
        chrys_core::Verdict::Refused { .. } => ExitCode::from(2),
    })
}

fn load_first_frame(source: &RasterSource, path: &Path) -> anyhow::Result<Frame> {
    let mut frames = source.load(path)?;
    if frames.is_empty() {
        anyhow::bail!("{} decoded to zero frames", path.display());
    }
    Ok(frames.remove(0))
}
