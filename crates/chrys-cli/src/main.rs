//! The `chrys` binary. It wires a source adapter to the engine and prints
//! the verdict.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrys_source::{Frame, Source};
use chrys_source_animation::AnimationSource;
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

        /// Compare inside a named region only. Both the base and the
        /// candidate frame at each index must carry a hint of this name;
        /// each side crops to its own hint's rectangle before the
        /// unmodified comparison runs.
        #[arg(long)]
        region: Option<String>,

        /// Print a line for every frame of a sequence, including the
        /// frames that did not change.
        ///
        /// By default a sequence prints only the frames that changed and
        /// closes with a count of the frames that did not. A hundred
        /// frames with five changes printed two hundred lines, of which
        /// five carried a verdict, and a reader found the five only by
        /// piping the output through another tool. This flag returns that
        /// shape for a caller that already parses it.
        ///
        /// This flag does not affect `--hash-only`, which always reports
        /// every frame: the digest report is the determinism evidence and
        /// it must not depend on which frames happened to change.
        #[arg(long)]
        all_frames: bool,
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
            region,
            all_frames,
        } => run_compare(&base, &candidate, hash_only, region.as_deref(), all_frames),
    }
}

fn run_compare(
    base_path: &Path,
    candidate_path: &Path,
    hash_only: bool,
    region: Option<&str>,
    all_frames: bool,
) -> anyhow::Result<ExitCode> {
    let base_frames = frames_for(base_path)?;
    let candidate_frames = frames_for(candidate_path)?;

    let (base_frames, candidate_frames) = match region {
        Some(name) => (
            crop_frames_to_region(&base_frames, name, base_path)?,
            crop_frames_to_region(&candidate_frames, name, candidate_path)?,
        ),
        None => (base_frames, candidate_frames),
    };

    let verdicts = chrys_core::compare_sequence(&base_frames, &candidate_frames)?;

    if hash_only {
        // A one-against-one sequence prints exactly the four digest lines
        // phase 1's `compare --hash-only` printed, byte-identical to
        // before this crate learned about sequences. A longer sequence
        // prints one four-line block per index, with a `frame {index}`
        // header before each block so the report stays parseable per
        // frame. `verdicts.len() == 1` is the same rule the verdict-text
        // branch below already uses to draw this line.
        if verdicts.len() == 1 {
            let base_frame = base_frames
                .first()
                .ok_or_else(|| anyhow::anyhow!("{} decoded to zero frames", base_path.display()))?;
            let candidate_frame = candidate_frames.first().ok_or_else(|| {
                anyhow::anyhow!("{} decoded to zero frames", candidate_path.display())
            })?;
            let digests =
                chrys_core::hash::digest_report(base_frame, candidate_frame, &verdicts[0])?;
            print!("{digests}");
            return Ok(ExitCode::from(0));
        }

        for (position, verdict) in verdicts.iter().enumerate() {
            let base_frame = base_frames.get(position).ok_or_else(|| {
                anyhow::anyhow!("{} has no frame at index {position}", base_path.display())
            })?;
            let candidate_frame = candidate_frames.get(position).ok_or_else(|| {
                anyhow::anyhow!(
                    "{} has no frame at index {position}",
                    candidate_path.display()
                )
            })?;
            // The printed index is the frame's own `index` field, the same
            // number the engine paired on, not the loop position.
            println!("frame {}", base_frame.index);
            let digests = chrys_core::hash::digest_report(base_frame, candidate_frame, verdict)?;
            print!("{digests}");
        }
        return Ok(ExitCode::from(0));
    }

    // A one-against-one sequence prints exactly what phase 1's `compare`
    // printed, so a single-file pair's stdout stays byte-identical to what
    // it was before this crate learned about sequences.
    if verdicts.len() == 1 {
        print!("{}", verdicts[0]);
    } else if all_frames {
        for (index, verdict) in verdicts.iter().enumerate() {
            println!("frame {index}");
            print!("{verdict}");
        }
    } else {
        // Print the frames that changed, then say how many did not. A
        // reader looking for a change reads only lines that carry one,
        // and a run of adjacent changed frames reads as a run because
        // nothing sits between its members any more.
        let mut unchanged = 0usize;
        for (index, verdict) in verdicts.iter().enumerate() {
            if matches!(verdict, chrys_core::Verdict::Identical) {
                unchanged += 1;
                continue;
            }
            println!("frame {index}");
            print!("{verdict}");
        }
        // The tail states the whole count, not only the silent part, so a
        // reader never has to add two numbers to learn how much was
        // compared. It prints even when nothing changed, because "0 of
        // 100 frames changed" and an empty stdout are different answers
        // and only one of them is legible.
        let changed = verdicts.len() - unchanged;
        println!("{changed} of {} frames changed", verdicts.len());
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

/// Crop every frame in `frames` to the rectangle its own hint named
/// `region` describes, failing loudly when a frame carries no hint of that
/// name.
///
/// The error names `path`, the frame's own index, and the region names
/// that frame does carry, so a person who mistyped a region learns the
/// available ones instead of a bare refusal.
fn crop_frames_to_region(
    frames: &[Frame],
    region: &str,
    path: &Path,
) -> anyhow::Result<Vec<Frame>> {
    frames
        .iter()
        .map(|frame| {
            let hint = frame
                .hints
                .iter()
                .find(|hint| hint.name == region)
                .ok_or_else(|| {
                    let available: Vec<&str> =
                        frame.hints.iter().map(|hint| hint.name.as_str()).collect();
                    let names = if available.is_empty() {
                        "none".to_string()
                    } else {
                        available.join(", ")
                    };
                    anyhow::anyhow!(
                        "{} has no region named \"{region}\" at frame {}; it names: {names}",
                        path.display(),
                        frame.index
                    )
                })?;
            Ok(frame.crop_to_region(hint)?)
        })
        .collect()
}

/// Load the frames at `path`. A directory loads as a numbered frame
/// sequence through `SequenceSource`. A file sniffed as a GIF, an APNG or
/// an animated WebP loads through `AnimationSource`. Any other file loads
/// as a single raster image through `RasterSource`, with no change to that
/// path's behaviour.
fn frames_for(path: &Path) -> anyhow::Result<Vec<Frame>> {
    if path.is_dir() {
        return Ok(SequenceSource::new().load(path)?);
    }
    if chrys_source_animation::sniff::is_animation(path)? {
        return Ok(AnimationSource::new().load(path)?);
    }
    Ok(RasterSource::new().load(path)?)
}
