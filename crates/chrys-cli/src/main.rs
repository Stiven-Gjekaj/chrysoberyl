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
        Command::Compare { base, candidate } => run_compare(&base, &candidate),
    }
}

fn run_compare(base_path: &Path, candidate_path: &Path) -> anyhow::Result<ExitCode> {
    let source = RasterSource::new();
    let base_frame = load_first_frame(&source, base_path)?;
    let candidate_frame = load_first_frame(&source, candidate_path)?;

    let verdict = chrys_core::compare(&base_frame, &candidate_frame)?;
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
