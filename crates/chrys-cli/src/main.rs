//! The `chrys` binary. It wires a source adapter to the engine and prints
//! the verdict.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use chrys_baseline::BaselineStore;
use chrys_baseline::golden::GoldenFileStore;
use chrys_source::{Frame, Source};
use chrys_source_animation::AnimationSource;
use chrys_source_raster::RasterSource;
use chrys_source_sequence::SequenceSource;
use clap::{Parser, Subcommand};

mod report;

/// The default baseline store root: a plain directory at the top of the
/// repository, not a hidden one, so a person finds `MANIFEST.toml` and
/// reads its diff in a pull request (BASE-01).
const DEFAULT_STORE_ROOT: &str = "chrys-baselines";

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
    ///
    /// With no `--baseline`, this takes exactly two paths: the base image,
    /// then the candidate. With `--baseline NAME`, this takes exactly one
    /// path, the candidate, and the base side comes from the baseline
    /// store instead.
    Compare {
        /// The paths to compare. With no `--baseline`: the base image,
        /// then the candidate, in that order. With `--baseline`: the
        /// candidate alone.
        #[arg(required = true, num_args = 1..=2)]
        paths: Vec<PathBuf>,

        /// Compare against the baseline of this name, read from the
        /// baseline store, instead of a second positional path.
        #[arg(long)]
        baseline: Option<String>,

        /// The baseline store root `--baseline` and `accept` both read
        /// and write.
        #[arg(long, default_value = DEFAULT_STORE_ROOT)]
        store: PathBuf,

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

    /// Accept PATH as the new baseline named NAME.
    ///
    /// An accept is a statement: this candidate is now correct. Every
    /// later comparison against NAME is measured against it, and nothing
    /// in this tool can tell a correct accept from a mistaken one.
    Accept {
        /// The baseline's own name.
        name: String,
        /// The path to accept: a file or a directory.
        path: PathBuf,
        /// The baseline store root to accept into.
        #[arg(long, default_value = DEFAULT_STORE_ROOT)]
        store: PathBuf,
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
            paths,
            baseline,
            store,
            hash_only,
            region,
            all_frames,
            rule,
            report,
        } => {
            let (base_path, candidate_path) =
                base_and_candidate(paths, baseline.as_deref(), &store)?;
            run_compare(
                &base_path,
                &candidate_path,
                hash_only,
                region.as_deref(),
                all_frames,
                rule.as_deref(),
                report.as_deref(),
                baseline.as_deref(),
                &store,
            )
        }
        Command::Accept { name, path, store } => run_accept(&name, &path, &store),
    }
}

/// Resolve `paths` and an optional `--baseline` name into the base and
/// candidate paths `run_compare` operates on.
///
/// With no baseline name, `paths` must hold exactly two entries: the base,
/// then the candidate, in that order. With a baseline name, `paths` must
/// hold exactly one entry, the candidate; the base side comes from
/// `store` by way of `BaselineStore::resolve`, which is itself the only
/// place a never-accepted name becomes a loud error rather than a first
/// silent accept (BASE-04). Clap is not asked to guess between these two
/// shapes: the count is validated here, explicitly, in both directions.
fn base_and_candidate(
    paths: Vec<PathBuf>,
    baseline: Option<&str>,
    store: &Path,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    match baseline {
        Some(name) => {
            if paths.len() != 1 {
                anyhow::bail!(
                    "compare --baseline takes exactly one path (the candidate); got {} path(s)",
                    paths.len()
                );
            }
            let baseline_store = GoldenFileStore::new(store.to_path_buf());
            let base_path = baseline_store.resolve(name)?;
            Ok((base_path, paths[0].clone()))
        }
        None => {
            if paths.len() != 2 {
                anyhow::bail!(
                    "compare with no --baseline takes exactly two paths (the base, then the \
                     candidate); got {} path(s)",
                    paths.len()
                );
            }
            Ok((paths[0].clone(), paths[1].clone()))
        }
    }
}

/// Accept `path` as the new baseline named `name`, in the store rooted at
/// `store`.
///
/// An accept is a statement: this candidate is now correct. Every later
/// comparison against `name` is measured against it, and nothing in this
/// tool can tell a correct accept from a mistaken one. This is the only
/// call site of `BaselineStore::accept` in the whole workspace.
fn run_accept(name: &str, path: &Path, store: &Path) -> anyhow::Result<ExitCode> {
    let (names, frames) = named_frames_for(path)?;
    let digests: Vec<(String, String)> = names
        .into_iter()
        .zip(frames.iter())
        .map(|(source, frame)| (source, chrys_core::hash::rgba8_digest(frame.rgba8())))
        .collect();

    let baseline_store = GoldenFileStore::new(store.to_path_buf());
    baseline_store.accept(name, path, &digests)?;
    Ok(ExitCode::from(0))
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
    baseline: Option<&str>,
    store: &Path,
) -> anyhow::Result<ExitCode> {
    // Loaded before either side is decoded, so a bad rule file fails
    // before any image decode work happens at all.
    let rules = rule_path.map(chrys_rule::load_rules).transpose()?;

    let (base_names, base_frames) = named_frames_for(base_path)?;
    let (_candidate_names, candidate_frames) = named_frames_for(candidate_path)?;

    // The manifest is checked, not decorated: when the base side came
    // from the baseline store, every frame's digest is recomputed and
    // compared against what MANIFEST.toml recorded for this name. A
    // disagreement fails the run and names the baseline, the frame, and
    // both digests, rather than silently comparing against a file the
    // manifest no longer describes.
    if let Some(name) = baseline {
        verify_baseline_digests(name, store, &base_frames)?;
    }

    // The pixel comparison reads the CROPPED frames when `--region` was
    // given: `compared_base_frames` and `compared_candidate_frames`, each
    // its own binding, never reusing `base_frames` or `candidate_frames`.
    // Those two names stay bound to the UNCROPPED frames `named_frames_for`
    // returned above for the rest of this function, so the rule engine,
    // the report writer and every printed rectangle below read the frame a
    // rule was authored against, not the crop (CR-01, T-03-27, T-03-28).
    let region_crop: Option<RegionCrop> = region
        .map(|name| -> anyhow::Result<RegionCrop> {
            let base_cropped = crop_frames_to_region(&base_frames, name, base_path)?;
            let candidate_cropped = crop_frames_to_region(&candidate_frames, name, candidate_path)?;
            let origins = base_cropped.iter().map(|(_, origin)| *origin).collect();
            let base_only: Vec<Frame> = base_cropped.into_iter().map(|(frame, _)| frame).collect();
            let candidate_only: Vec<Frame> = candidate_cropped
                .into_iter()
                .map(|(frame, _)| frame)
                .collect();
            Ok((base_only, candidate_only, origins))
        })
        .transpose()?;

    let compared_base_frames: &[Frame] = region_crop
        .as_ref()
        .map(|(base, _, _)| base.as_slice())
        .unwrap_or(&base_frames);
    let compared_candidate_frames: &[Frame] = region_crop
        .as_ref()
        .map(|(_, candidate, _)| candidate.as_slice())
        .unwrap_or(&candidate_frames);
    // With no `--region`, every origin is `(0, 0)`, so
    // `translate_verdicts` below is a no-op by value and the flag-less
    // path shares one code path with the `--region` path rather than a
    // second, divergent one.
    let origins: Vec<(u32, u32)> = region_crop
        .as_ref()
        .map(|(_, _, origins)| origins.clone())
        .unwrap_or_else(|| vec![(0, 0); base_frames.len()]);

    let verdicts = translate_verdicts(
        chrys_core::compare_sequence(compared_base_frames, compared_candidate_frames)?,
        &origins,
    )?;

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
                region: region.map(|name| name.to_string()),
            },
            frame: frames,
        };
        report::write_report(report_path, &document)?;
    }

    if hash_only {
        // `--hash-only` hashes the frames that were actually compared:
        // `compared_base_frames`/`compared_candidate_frames`, explicitly,
        // never `base_frames`/`candidate_frames` (which stay uncropped
        // above this point and would otherwise hash the wrong bytes under
        // `--region`, now that this function no longer shadows either
        // name with a cropped value). A one-against-one sequence prints
        // exactly the four digest lines phase 1's `compare --hash-only`
        // printed, byte-identical to before this crate learned about
        // sequences. A longer sequence prints one four-line block per
        // index, with a `frame {index}` header before each block so the
        // report stays parseable per frame. `verdicts.len() == 1` is the
        // same rule the verdict-text branch below already uses to draw
        // this line.
        if verdicts.len() == 1 {
            let base_frame = compared_base_frames
                .first()
                .ok_or_else(|| anyhow::anyhow!("{} decoded to zero frames", base_path.display()))?;
            let candidate_frame = compared_candidate_frames.first().ok_or_else(|| {
                anyhow::anyhow!("{} decoded to zero frames", candidate_path.display())
            })?;
            let digests =
                chrys_core::hash::digest_report(base_frame, candidate_frame, &verdicts[0])?;
            print!("{digests}");
            return Ok(ExitCode::from(0));
        }

        for (position, verdict) in verdicts.iter().enumerate() {
            let base_frame = compared_base_frames.get(position).ok_or_else(|| {
                anyhow::anyhow!("{} has no frame at index {position}", base_path.display())
            })?;
            let candidate_frame = compared_candidate_frames.get(position).ok_or_else(|| {
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

/// Recompute each of `base_frames`' own digest and compare it against what
/// `MANIFEST.toml` records for the baseline named `name`, in the store
/// rooted at `store`.
///
/// `GoldenFileStore::resolve` never reads a digest; this is the caller
/// that has the pixels, and checking here is what makes the manifest
/// evidence rather than decoration. A disagreement, in either the frame
/// count or one frame's own digest, fails the run and names the baseline,
/// the frame, and both digests, so a hand-edited or a stale manifest
/// cannot make a green run mean nothing.
fn verify_baseline_digests(name: &str, store: &Path, base_frames: &[Frame]) -> anyhow::Result<()> {
    let baseline_store = GoldenFileStore::new(store.to_path_buf());
    let recorded = baseline_store.manifest_digests(name)?;

    if recorded.len() != base_frames.len() {
        anyhow::bail!(
            "baseline \"{name}\" records {} frame(s) in its manifest, but the stored baseline \
             decoded to {} frame(s)",
            recorded.len(),
            base_frames.len()
        );
    }

    for (index, (source, recorded_digest)) in recorded.iter().enumerate() {
        let computed_digest = chrys_core::hash::rgba8_digest(base_frames[index].rgba8());
        if &computed_digest != recorded_digest {
            anyhow::bail!(
                "baseline \"{name}\" frame {index} (\"{source}\"): the manifest records digest \
                 {recorded_digest}, but the stored file now produces {computed_digest}"
            );
        }
    }

    Ok(())
}

/// The cropped base frames, the cropped candidate frames, and the base
/// side's own per-frame crop origins, built once by `run_compare` when
/// `--region` was given.
type RegionCrop = (Vec<Frame>, Vec<Frame>, Vec<(u32, u32)>);

/// Crop every frame in `frames` to the rectangle its own hint named
/// `region` describes, paired with that frame's own crop origin (the
/// hint's `x` and `y`), failing loudly when a frame carries no hint of
/// that name.
///
/// The origin is produced here, at the one place the hint is looked up,
/// so it cannot drift from the hint it came from: a sequence gets one
/// origin per frame, because a hint can sit at a different place on each
/// frame. A caller adds this origin back onto every rectangle the pixel
/// comparison reports over the cropped frame, so the frame's own
/// coordinate space wins over the crop's (CR-01).
///
/// The error names `path`, the frame's own index, and the region names
/// that frame does carry, so a person who mistyped a region learns the
/// available ones instead of a bare refusal.
fn crop_frames_to_region(
    frames: &[Frame],
    region: &str,
    path: &Path,
) -> anyhow::Result<Vec<(Frame, (u32, u32))>> {
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
            let origin = (hint.x, hint.y);
            let cropped = frame.crop_to_region(hint)?;
            Ok((cropped, origin))
        })
        .collect()
}

/// Translate every `Region`'s own bounding box in `verdicts` by that
/// frame's own crop origin from `origins` (`origins[index]` for
/// `verdicts[index]`), so every rectangle a run reports afterwards, on
/// standard output, in the report and in the rule engine, is stated in
/// the UNCROPPED base frame's own coordinate space rather than the
/// cropped rectangle the pixel comparison actually ran over.
///
/// This runs unconditionally, on both the flag-less path and the
/// `--region` path: when `--region` was absent every entry of `origins`
/// is `(0, 0)`, so the translated copy equals the original by value and
/// standard output stays byte for byte what it was before this function
/// existed (CLI-03). Sharing one code path for both is the point: no
/// second place exists where a coordinate space can diverge from the
/// other.
///
/// `Region.offset_px` is how far content moved, not where it is, so it
/// is copied through unchanged; `bbox.width` and `bbox.height` are sizes
/// and are copied through unchanged too. `Verdict::Identical` and
/// `Verdict::Refused` carry no rectangle and are copied through
/// unchanged.
///
/// The addition uses `checked_add` and fails the run, naming both
/// numbers, when it overflows: `Frame::crop_to_region` already refuses a
/// hint that does not fit inside the frame, so the sum provably fits and
/// an overflow here means an assumption broke and a person needs to read
/// which, not a value silently wrapped to a wrong answer.
fn translate_verdicts(
    verdicts: Vec<chrys_core::Verdict>,
    origins: &[(u32, u32)],
) -> anyhow::Result<Vec<chrys_core::Verdict>> {
    verdicts
        .into_iter()
        .enumerate()
        .map(|(index, verdict)| {
            let (origin_x, origin_y) = origins.get(index).copied().unwrap_or((0, 0));
            translate_verdict(verdict, origin_x, origin_y)
        })
        .collect()
}

/// Translate one `Verdict`'s own regions by `(origin_x, origin_y)`. See
/// `translate_verdicts`' own doc comment for what moves and what does
/// not.
fn translate_verdict(
    verdict: chrys_core::Verdict,
    origin_x: u32,
    origin_y: u32,
) -> anyhow::Result<chrys_core::Verdict> {
    match verdict {
        chrys_core::Verdict::Changed { regions } => {
            let translated = regions
                .into_iter()
                .map(|region| translate_region(region, origin_x, origin_y))
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(chrys_core::Verdict::Changed {
                regions: translated,
            })
        }
        identical_or_refused => Ok(identical_or_refused),
    }
}

/// Translate one `Region`'s own `bbox.x` and `bbox.y` by
/// `(origin_x, origin_y)`, using `checked_add` so an overflow fails
/// loudly rather than wrapping to a wrong rectangle.
fn translate_region(
    mut region: chrys_core::Region,
    origin_x: u32,
    origin_y: u32,
) -> anyhow::Result<chrys_core::Region> {
    region.bbox.x = region.bbox.x.checked_add(origin_x).ok_or_else(|| {
        anyhow::anyhow!(
            "translating a change's own x ({}) by its frame's crop origin ({origin_x}) \
             overflows a u32",
            region.bbox.x
        )
    })?;
    region.bbox.y = region.bbox.y.checked_add(origin_y).ok_or_else(|| {
        anyhow::anyhow!(
            "translating a change's own y ({}) by its frame's crop origin ({origin_y}) \
             overflows a u32",
            region.bbox.y
        )
    })?;
    Ok(region)
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
