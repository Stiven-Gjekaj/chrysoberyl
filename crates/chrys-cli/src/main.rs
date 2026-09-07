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

mod report;

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

        /// Gate the exit code on a TOML rule file. Every `Changed` verdict
        /// whose regions are all tolerated by a matching rule then exits
        /// 0; a verdict with at least one untolerated region still exits
        /// 1, exactly as it did before this flag existed. When this flag
        /// is absent nothing changes at all.
        #[arg(long)]
        rule: Option<PathBuf>,

        /// Write a TOML report naming every compared frame's own source
        /// file, and every change's kind, region and size, to this path.
        /// Composes with every other flag above and changes no exit code
        /// and no stdout. When this flag is absent nothing at all
        /// changes.
        #[arg(long)]
        report: Option<PathBuf>,
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
            rule,
            report,
        } => run_compare(
            &base,
            &candidate,
            hash_only,
            region.as_deref(),
            all_frames,
            rule.as_deref(),
            report.as_deref(),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_compare(
    base_path: &Path,
    candidate_path: &Path,
    hash_only: bool,
    region: Option<&str>,
    all_frames: bool,
    rule_path: Option<&Path>,
    report_path: Option<&Path>,
) -> anyhow::Result<ExitCode> {
    // Loaded before either side is decoded, so a bad rule file fails
    // before any image decode work happens at all.
    let rules = rule_path.map(chrys_rule::load_rules).transpose()?;

    let (base_names, base_frames) = named_frames_for(base_path)?;
    let (_candidate_names, candidate_frames) = named_frames_for(candidate_path)?;

    let (base_frames, candidate_frames) = match region {
        Some(name) => (
            crop_frames_to_region(&base_frames, name, base_path)?,
            crop_frames_to_region(&candidate_frames, name, candidate_path)?,
        ),
        None => (base_frames, candidate_frames),
    };

    let verdicts = chrys_core::compare_sequence(&base_frames, &candidate_frames)?;

    // A rule outcome is computed per frame, against that frame's own
    // base-side hints and size (the size is read only to check a
    // mask-scoped rule's own mask against it), only when --rule was
    // given. When it is absent this stays entirely unevaluated, so the
    // exit code below reads exactly as it did before this flag existed
    // (CLI-03's regression guard), and the report below writes no
    // `rule_outcome` key at all. Computed here, before any output branch
    // runs, so the report below can carry it whichever branch below this
    // point returns.
    let outcomes: Option<Vec<chrys_rule::RuleOutcome>> = rules
        .as_ref()
        .map(|rules| {
            verdicts
                .iter()
                .enumerate()
                .map(|(index, verdict)| {
                    let hints = base_frames
                        .get(index)
                        .map(|frame| frame.hints.as_slice())
                        .unwrap_or(&[]);
                    let frame_size = base_frames
                        .get(index)
                        .map(|frame| (frame.width, frame.height))
                        .unwrap_or((0, 0));
                    chrys_rule::evaluate(verdict, hints, rules, frame_size)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;

    // The report is written before any exit-code-bearing branch below
    // returns, on every path that produced a verdict, including a
    // non-zero exit and a refused pair: a report that only exists on
    // success reports nothing about the run a person actually needs to
    // read (T-03-15).
    if let Some(report_path) = report_path {
        let frames: Vec<report::FrameReport> = verdicts
            .iter()
            .enumerate()
            .map(|(index, verdict)| {
                let source = base_names.get(index).cloned().unwrap_or_default();
                let hints = base_frames
                    .get(index)
                    .map(|frame| frame.hints.as_slice())
                    .unwrap_or(&[]);
                let region_outcomes = outcomes
                    .as_ref()
                    .map(|outcomes| outcomes[index].regions.as_slice());
                report::frame_report(index, source, verdict, hints, region_outcomes)
            })
            .collect();
        let document = report::Report {
            meta: report::Meta {
                base: base_path.display().to_string(),
                candidate: candidate_path.display().to_string(),
                rule: rule_path.map(|path| path.display().to_string()),
            },
            frame: frames,
        };
        report::write_report(report_path, &document)?;
    }

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

    // `outcomes` was computed above, before the output branches, so the
    // report could carry it whichever branch returned; the exit code
    // below reads it the same way it did before this flag existed
    // (CLI-03's regression guard).
    let worst = verdicts
        .iter()
        .enumerate()
        .map(|(index, verdict)| {
            let outcome = outcomes.as_ref().map(|outcomes| &outcomes[index]);
            verdict_rank(verdict, outcome)
        })
        .max()
        .unwrap_or(0);
    Ok(ExitCode::from(worst))
}

/// Rank a verdict for the process exit code: refused above changed above
/// identical. A caller branching on the exit code of a sequence pair keeps
/// the same three meanings a single-pair comparison already reports.
///
/// `outcome` is `Some` only when `--rule` was given. A `Changed` verdict
/// whose outcome holds no violation ranks 0 instead of 1; every other case
/// is unchanged from before this flag existed. `Refused` always ranks 2:
/// a rule tolerates a change, and a refusal is not a change, so no outcome
/// can move a refusal's rank.
fn verdict_rank(verdict: &chrys_core::Verdict, outcome: Option<&chrys_rule::RuleOutcome>) -> u8 {
    match verdict {
        chrys_core::Verdict::Identical => 0,
        chrys_core::Verdict::Changed { .. } => match outcome {
            Some(outcome) if outcome.is_clean() => 0,
            _ => 1,
        },
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

/// Load the frames at `path`, each paired with the name of the file it
/// came from, through `Source::load_named`. A directory loads as a
/// numbered frame sequence through `SequenceSource`, each frame beside
/// its own file's name. A file sniffed as a GIF, an APNG or an animated
/// WebP loads through `AnimationSource`, every frame beside that
/// container's own name. Any other file loads as a single raster image
/// through `RasterSource`, that one frame beside its own file's name.
/// The three-way dispatch itself is unchanged from before this crate
/// learned a frame's own name.
fn named_frames_for(path: &Path) -> anyhow::Result<(Vec<String>, Vec<Frame>)> {
    let named = if path.is_dir() {
        SequenceSource::new().load_named(path)?
    } else if chrys_source_animation::sniff::is_animation(path)? {
        AnimationSource::new().load_named(path)?
    } else {
        RasterSource::new().load_named(path)?
    };
    Ok(named.into_iter().unzip())
}
