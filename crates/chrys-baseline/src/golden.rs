//! The committed-golden-file `BaselineStore` backend.
//!
//! A baseline named `name` lives at `<root>/<name>/`, holding the accepted
//! file or directory contents under their own original names.
//! `<root>/MANIFEST.toml` sits beside every baseline directory and records
//! one `[[baseline]]` table per name, each holding one `[[baseline.frame]]`
//! table per frame with that frame's own source file name and digest. A
//! single accepted change to one frame of one baseline then changes exactly
//! one `digest` line, which is what makes the diff readable in a pull
//! request without opening the image.
//!
//! **Every write lives in one function.** `write_baseline`, below, is the
//! only place in this crate that creates a directory, copies a file, writes
//! a file, removes one, or renames one. `accept` calls it and nothing else
//! does. This is what BASE-04 rests on: a second, unnamed way to write into
//! the store would be a second, unreviewed path to the same drift an
//! explicit accept exists to prevent. `crates/chrys-baseline/tests/store.rs`
//! holds a static guard that checks this claim over this file's own source
//! text.
//!
//! **The write is staged, not destructive.** `write_baseline` builds the
//! new baseline and the new manifest beside the old ones, complete, before
//! it touches either. See `write_baseline`'s own doc comment for the full
//! contract and the one window it does not close.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::BaselineStore;

/// The largest `MANIFEST.toml` this store reads, in bytes.
///
/// A manifest holds a handful of short lines per accepted frame. One
/// mebibyte is a ceiling no real manifest reaches, which is what makes it
/// safe to refuse above it rather than read and hope the parser fails
/// first. This is the same posture `chrys-source-raster`'s `hints.rs`
/// already takes on its own sidecar.
pub const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

/// The golden-file `BaselineStore` backend: every accepted baseline is a
/// committed file or directory under `root`, indexed by `MANIFEST.toml`.
pub struct GoldenFileStore {
    /// The store's own root directory.
    pub root: PathBuf,
}

impl GoldenFileStore {
    /// Build a store rooted at `root`. `root` need not exist yet: `accept`
    /// creates it on the first call.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        GoldenFileStore { root: root.into() }
    }

    /// Return the frame digests `MANIFEST.toml` records for `name`, in
    /// frame order: each pair is the source file name `accept` recorded
    /// and the lowercase-hexadecimal digest it computed for that frame.
    ///
    /// This reads `MANIFEST.toml` only. It decodes no pixel and compares
    /// nothing; the caller that holds the pixels is the one that can tell
    /// whether they still agree with what is recorded here.
    pub fn manifest_digests(&self, name: &str) -> Result<Vec<(String, String)>, BaselineError> {
        let manifest_path = self.root.join("MANIFEST.toml");
        let manifest = read_manifest(&manifest_path)?;
        let entry = manifest
            .baseline
            .into_iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| BaselineError::NotAccepted {
                name: name.to_string(),
                root: self.root.clone(),
            })?;
        Ok(entry
            .frame
            .into_iter()
            .map(|frame| (frame.source, frame.digest))
            .collect())
    }
}

/// A `GoldenFileStore` error.
#[derive(Debug, thiserror::Error)]
pub enum BaselineError {
    /// `resolve`, or `manifest_digests`, was asked for a name that
    /// `accept` has never written.
    #[error(
        "baseline \"{name}\" was never accepted; the store at {root} holds no baseline of that name"
    )]
    NotAccepted {
        /// The name that was asked for.
        name: String,
        /// The store's own root directory.
        root: PathBuf,
    },
    /// A filesystem operation on `path` failed.
    #[error("{path}: {source}")]
    Io {
        /// The path the operation was on.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// `MANIFEST.toml` was read, but its bytes did not parse as a
    /// well-formed manifest.
    #[error("{path}: {message}")]
    MalformedManifest {
        /// The manifest path that failed to parse.
        path: PathBuf,
        /// The parser's own error message.
        message: String,
    },
    /// `MANIFEST.toml` is larger than `MAX_MANIFEST_BYTES`.
    #[error("{path} is {size} bytes, over the {limit} byte limit")]
    ManifestTooLarge {
        /// The manifest path that exceeded the limit.
        path: PathBuf,
        /// The size the filesystem reported, in bytes.
        size: u64,
        /// The limit that was exceeded.
        limit: u64,
    },
}

/// `MANIFEST.toml`'s own document shape: an array of tables named
/// `baseline`.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    /// Every baseline this manifest names. Absent entirely when the
    /// manifest holds no `[[baseline]]` table.
    #[serde(rename = "baseline", default)]
    baseline: Vec<BaselineEntry>,
}

/// One `[[baseline]]` table: a name, and the frames accepted under it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BaselineEntry {
    /// The baseline's own name.
    name: String,
    /// Every frame this baseline holds, in frame order.
    #[serde(rename = "frame", default)]
    frame: Vec<FrameEntry>,
}

/// One `[[baseline.frame]]` table: one accepted frame's own source file
/// name and digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameEntry {
    /// The source file name `accept` was given for this frame.
    source: String,
    /// The lowercase-hexadecimal digest `accept` recorded for this frame.
    digest: String,
}

/// Read and parse `manifest_path`, bounding its size before it is read.
///
/// A missing manifest is not an error: the first `accept` into a fresh
/// store finds none. This function performs no write of its own; it is
/// safe to call from anywhere, including `resolve`'s own read-only
/// contract, without becoming a second way to change the store.
fn read_manifest(manifest_path: &Path) -> Result<Manifest, BaselineError> {
    match fs::metadata(manifest_path) {
        Ok(metadata) if metadata.len() > MAX_MANIFEST_BYTES => {
            return Err(BaselineError::ManifestTooLarge {
                path: manifest_path.to_path_buf(),
                size: metadata.len(),
                limit: MAX_MANIFEST_BYTES,
            });
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Manifest::default());
        }
        Err(error) => {
            return Err(BaselineError::Io {
                path: manifest_path.to_path_buf(),
                source: error,
            });
        }
    }

    let text = fs::read_to_string(manifest_path).map_err(|source| BaselineError::Io {
        path: manifest_path.to_path_buf(),
        source,
    })?;

    toml::from_str(&text).map_err(|error| BaselineError::MalformedManifest {
        path: manifest_path.to_path_buf(),
        message: error.to_string(),
    })
}

impl BaselineStore for GoldenFileStore {
    type Error = BaselineError;

    /// Find `name` by listing `<root>/<name>/` rather than by trusting
    /// `MANIFEST.toml`'s own `source` value. A person who renames the
    /// stored file with `git mv`, while it stays inside its own directory,
    /// keeps a working baseline; the identity a person tracks is the name,
    /// not a path. A directory holding exactly one file resolves to that
    /// file's own path; a directory holding more resolves to the
    /// directory itself.
    ///
    /// This reads the directory listing and each entry's metadata only.
    /// No branch of this method creates, writes to, or removes anything.
    fn resolve(&self, name: &str) -> Result<PathBuf, Self::Error> {
        let baseline_dir = self.root.join(name);

        let entries = match fs::read_dir(&baseline_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(BaselineError::NotAccepted {
                    name: name.to_string(),
                    root: self.root.clone(),
                });
            }
            Err(error) => {
                return Err(BaselineError::Io {
                    path: baseline_dir,
                    source: error,
                });
            }
        };

        let mut files: Vec<PathBuf> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| BaselineError::Io {
                path: baseline_dir.clone(),
                source,
            })?;
            let metadata = entry.metadata().map_err(|source| BaselineError::Io {
                path: baseline_dir.clone(),
                source,
            })?;
            if metadata.is_file() {
                files.push(entry.path());
            }
        }
        files.sort();

        match files.len() {
            0 => Err(BaselineError::NotAccepted {
                name: name.to_string(),
                root: self.root.clone(),
            }),
            1 => Ok(files.into_iter().next().expect("checked length is 1")),
            _ => Ok(baseline_dir),
        }
    }

    fn accept(
        &self,
        name: &str,
        candidate_path: &Path,
        frame_digests: &[(String, String)],
    ) -> Result<(), Self::Error> {
        write_baseline(&self.root, name, candidate_path, frame_digests)
    }
}

/// The one function in this crate that creates a directory, copies a file,
/// writes a file, removes one, or renames one. `accept` is the only
/// caller.
///
/// **Nothing a reader or a later accept can see is touched until the new
/// state is complete on disk.** The new baseline is built in a staging
/// directory beside the real one, and the new manifest is built in a
/// temporary file beside the real one; only once both are complete does
/// this function rename the staging directory into place, rename the
/// temporary manifest over `MANIFEST.toml`, and remove what the rename
/// displaced. A failure in any step before that point leaves the
/// previous, complete baseline and the previous, complete manifest
/// exactly as they were: nothing is deleted before the replacement exists
/// (BASE-04, T-03-30). The previous approach, `fs::remove_dir_all` on the
/// live baseline directory before writing the new one, is the one this
/// replaces: a failure between those two points used to leave the store
/// holding neither the old baseline nor the new one.
///
/// **The window this does not close.** Two renames are not one atomic
/// act. Between the first (the new baseline directory swapped into
/// place) and the second (the new manifest swapped into place), the
/// baseline directory and `MANIFEST.toml` briefly disagree: the directory
/// already holds the new candidate's bytes, and the manifest still
/// records the previous candidate's digests. This function does not
/// claim that gap is atomic, because it is not. It is safe because
/// `verify_baseline_digests` (the CLI's own caller of
/// `manifest_digests`) fails every later `compare --baseline` loudly,
/// naming the baseline, the frame, and both digests, for as long as the
/// two disagree (T-03-21): the residual window fails closed rather than
/// silently.
///
/// **Why a stale staging directory is cleared but a stale temporary
/// manifest is not.** A staging directory left behind by an earlier
/// crashed accept holds files from a candidate nobody accepted; reusing
/// it without clearing it first would copy those leftover files into the
/// baseline this accept is building, alongside the current candidate's
/// own files. `fs::write`, by contrast, truncates a file that already
/// exists, so a stale temporary manifest is fully overwritten by this
/// accept's own write and contributes nothing. A stale directory can
/// contribute a file; a stale file cannot contribute a byte. That
/// asymmetry is why only the directory is cleared before use.
fn write_baseline(
    root: &Path,
    name: &str,
    candidate_path: &Path,
    frame_digests: &[(String, String)],
) -> Result<(), BaselineError> {
    let baseline_dir = root.join(name);
    let staging_dir = root.join(format!(".{name}.accept-tmp"));
    let previous_dir = root.join(format!(".{name}.previous-tmp"));
    let manifest_path = root.join("MANIFEST.toml");
    let manifest_tmp_path = root.join("MANIFEST.toml.tmp");

    // The whole staged write, as one closure, so a failure at any point
    // below can be caught once, in one place, without repeating the same
    // cleanup after every `?`.
    let stage_and_swap = || -> Result<(), BaselineError> {
        // A stale staging directory, left by an earlier crashed accept,
        // holds files from a candidate nobody accepted: cleared before
        // use, so it cannot contribute one of them to the baseline this
        // accept builds.
        if staging_dir.is_dir() {
            fs::remove_dir_all(&staging_dir).map_err(|source| BaselineError::Io {
                path: staging_dir.clone(),
                source,
            })?;
        }
        fs::create_dir_all(&staging_dir).map_err(|source| BaselineError::Io {
            path: staging_dir.clone(),
            source,
        })?;

        if candidate_path.is_dir() {
            // A directory candidate copies every regular file it
            // directly holds, under the same name, byte for byte. This
            // does not recurse into a nested directory and does not copy
            // anything that is not a regular file, matching the rule
            // `chrys_source_sequence::sequence::list_sorted` already
            // applies for the same reason.
            let entries = fs::read_dir(candidate_path).map_err(|source| BaselineError::Io {
                path: candidate_path.to_path_buf(),
                source,
            })?;
            for entry in entries {
                let entry = entry.map_err(|source| BaselineError::Io {
                    path: candidate_path.to_path_buf(),
                    source,
                })?;
                let metadata = entry.metadata().map_err(|source| BaselineError::Io {
                    path: candidate_path.to_path_buf(),
                    source,
                })?;
                if !metadata.is_file() {
                    continue;
                }
                let destination = staging_dir.join(entry.file_name());
                fs::copy(entry.path(), &destination).map_err(|source| BaselineError::Io {
                    path: entry.path(),
                    source,
                })?;
            }
        } else {
            let file_name = candidate_path
                .file_name()
                .ok_or_else(|| BaselineError::Io {
                    path: candidate_path.to_path_buf(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "the candidate path has no file name",
                    ),
                })?;
            let destination = staging_dir.join(file_name);
            fs::copy(candidate_path, &destination).map_err(|source| BaselineError::Io {
                path: candidate_path.to_path_buf(),
                source,
            })?;
        }

        let mut manifest = read_manifest(&manifest_path)?;
        manifest.baseline.retain(|entry| entry.name != name);
        manifest.baseline.push(BaselineEntry {
            name: name.to_string(),
            frame: frame_digests
                .iter()
                .map(|(source, digest)| FrameEntry {
                    source: source.clone(),
                    digest: digest.clone(),
                })
                .collect(),
        });
        // Sorted by name, so accepting a second, unrelated baseline never
        // reorders an already-committed one in the diff: BASE-01 wants a
        // single accepted change to read as one changed digest line, not
        // as a reshuffled file.
        manifest.baseline.sort_by(|a, b| a.name.cmp(&b.name));

        let serialized = toml::to_string_pretty(&manifest).map_err(|error| {
            BaselineError::MalformedManifest {
                path: manifest_path.clone(),
                message: error.to_string(),
            }
        })?;
        // `fs::write` truncates a file that already exists, so a stale
        // temporary manifest from an earlier crashed accept is fully
        // overwritten here and contributes nothing: see this asymmetry
        // explained in this function's own doc comment.
        fs::write(&manifest_tmp_path, serialized).map_err(|source| BaselineError::Io {
            path: manifest_tmp_path.clone(),
            source,
        })?;

        // Only now does this function touch what a reader sees. The old
        // baseline directory, if one exists, is renamed aside rather than
        // deleted, so it survives until both of the following renames
        // succeed.
        let had_previous = baseline_dir.is_dir();
        if had_previous {
            fs::rename(&baseline_dir, &previous_dir).map_err(|source| BaselineError::Io {
                path: baseline_dir.clone(),
                source,
            })?;
        }
        fs::rename(&staging_dir, &baseline_dir).map_err(|source| BaselineError::Io {
            path: staging_dir.clone(),
            source,
        })?;
        fs::rename(&manifest_tmp_path, &manifest_path).map_err(|source| BaselineError::Io {
            path: manifest_tmp_path.clone(),
            source,
        })?;
        if had_previous {
            // Removed only after both renames above have already
            // succeeded: by this point the new baseline and the new
            // manifest are both in place, so the directory moved aside is
            // no longer needed by anything this accept can still fail at.
            let _ = fs::remove_dir_all(&previous_dir);
        }

        Ok(())
    };

    let result = stage_and_swap();
    if result.is_err() {
        // Best effort: a cleanup failure must not replace the error that
        // caused it. A person needs to read why the accept failed, not
        // why the tidying failed.
        let _ = fs::remove_dir_all(&staging_dir);
    }
    result
}
