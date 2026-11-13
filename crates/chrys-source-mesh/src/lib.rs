//! The mesh adapter: a 3D mesh, seen through a fixed rig of eight views.
//!
//! This is the only crate that knows a file can be a mesh. It reads
//! Wavefront OBJ and STL (ASCII and binary). It renders each mesh through
//! the eight views in `rig::VIEWS` with the rasterizer in `raster`, which
//! this repository writes itself. Each view becomes one `Frame`, so a mesh
//! is a sequence of eight frames.
//!
//! The rig leaves a blind spot: a surface that no view sees. `blind_spot`
//! measures it for one mesh, as a count of triangles.

#![forbid(unsafe_code)]

pub mod parse;
pub mod raster;
pub mod rig;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use chrys_source::{Frame, Source};
use parse::{MeshFormat, Triangle};

/// The resource limits for one mesh load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshLimits {
    /// The largest file size, in bytes.
    pub max_file_bytes: u64,
    /// The largest number of triangles.
    pub max_triangles: usize,
}

impl Default for MeshLimits {
    fn default() -> Self {
        MeshLimits {
            max_file_bytes: 256 * 1024 * 1024,
            max_triangles: 2_000_000,
        }
    }
}

/// The error that the mesh adapter returns.
#[derive(Debug, thiserror::Error)]
pub enum MeshError {
    /// The path cannot be read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The path.
        path: PathBuf,
        /// The I/O error.
        source: std::io::Error,
    },
    /// The file is larger than the limit.
    #[error("{path} is larger than the limit of {limit} bytes")]
    FileTooLarge {
        /// The path.
        path: PathBuf,
        /// The limit.
        limit: u64,
    },
    /// The content is not a mesh that this adapter reads.
    #[error("{path} is not an OBJ or an STL file")]
    NotAMesh {
        /// The path.
        path: PathBuf,
    },
    /// The file does not parse.
    #[error("{path}: {reason}")]
    Parse {
        /// The path.
        path: PathBuf,
        /// The reason.
        reason: parse::ParseError,
    },
    /// The mesh holds no triangle, or more than the limit.
    #[error("{path} holds {triangles} triangles, and the range is 1 to {limit}")]
    TriangleCount {
        /// The path.
        path: PathBuf,
        /// The number of triangles.
        triangles: usize,
        /// The limit.
        limit: usize,
    },
}

/// The surface that the rig does not see, for one mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlindSpot {
    /// The triangles that own no pixel in any of the eight views.
    pub hidden: usize,
    /// All triangles.
    pub total: usize,
}

impl std::fmt::Display for BlindSpot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "blind spot: {} of {} triangles are hidden in all eight views; {}",
            self.hidden,
            self.total,
            rig::BLIND_SPOT
        )
    }
}

/// Return true when the content of `path` is an OBJ or an STL file.
pub fn looks_like_mesh(path: &Path) -> std::io::Result<bool> {
    let mut head = Vec::new();
    File::open(path)?.take(4096).read_to_end(&mut head)?;
    let length = std::fs::metadata(path)?.len();
    Ok(parse::sniff(&head, length).is_some())
}

/// The mesh `Source`.
pub struct MeshSource {
    limits: MeshLimits,
}

impl MeshSource {
    /// Make a source with the default limits.
    pub fn new() -> Self {
        MeshSource {
            limits: MeshLimits::default(),
        }
    }

    /// Make a source with the given limits.
    pub fn with_limits(limits: MeshLimits) -> Self {
        MeshSource { limits }
    }

    /// Read the triangles of the mesh at `path`.
    pub fn triangles(&self, path: &Path) -> Result<Vec<Triangle>, MeshError> {
        let io = |source| MeshError::Io {
            path: path.to_path_buf(),
            source,
        };
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(io)?
            .take(self.limits.max_file_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(io)?;
        if bytes.len() as u64 > self.limits.max_file_bytes {
            return Err(MeshError::FileTooLarge {
                path: path.to_path_buf(),
                limit: self.limits.max_file_bytes,
            });
        }
        let parse_error = |reason| MeshError::Parse {
            path: path.to_path_buf(),
            reason,
        };
        let text = || {
            std::str::from_utf8(&bytes)
                .map_err(|_| parse_error(parse::ParseError("the text is not UTF-8".to_string())))
        };
        let triangles = match parse::sniff(&bytes, bytes.len() as u64) {
            Some(MeshFormat::Obj) => parse::obj(text()?).map_err(parse_error)?,
            Some(MeshFormat::AsciiStl) => parse::ascii_stl(text()?).map_err(parse_error)?,
            Some(MeshFormat::BinaryStl) => parse::binary_stl(&bytes).map_err(parse_error)?,
            None => {
                return Err(MeshError::NotAMesh {
                    path: path.to_path_buf(),
                });
            }
        };
        if triangles.is_empty() || triangles.len() > self.limits.max_triangles {
            return Err(MeshError::TriangleCount {
                path: path.to_path_buf(),
                triangles: triangles.len(),
                limit: self.limits.max_triangles,
            });
        }
        Ok(triangles)
    }

    /// Render the eight views of the mesh at `path`, and measure its blind
    /// spot.
    pub fn render(&self, path: &Path) -> Result<(Vec<Frame>, BlindSpot), MeshError> {
        let triangles = raster::normalise(&self.triangles(path)?);
        let mut seen = vec![false; triangles.len()];
        let mut frames = Vec::with_capacity(rig::VIEWS.len());
        for (index, view) in rig::VIEWS.iter().enumerate() {
            let rendered = raster::render(&triangles, view);
            for (was_seen, visible) in seen.iter_mut().zip(&rendered.visible) {
                *was_seen |= *visible;
            }
            frames.push(Frame {
                pixels: rendered.pixels,
                width: raster::SIZE,
                height: raster::SIZE,
                index,
                hints: Vec::new(),
            });
        }
        let blind = BlindSpot {
            hidden: seen.iter().filter(|seen| !**seen).count(),
            total: triangles.len(),
        };
        Ok((frames, blind))
    }

    /// Measure the blind spot of the mesh at `path`.
    pub fn blind_spot(&self, path: &Path) -> Result<BlindSpot, MeshError> {
        Ok(self.render(path)?.1)
    }
}

impl Default for MeshSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for MeshSource {
    type Error = MeshError;

    fn load(&self, path: &Path) -> Result<Vec<Frame>, Self::Error> {
        Ok(self.render(path)?.0)
    }

    /// Name each frame for its file and its view, for example
    /// `part.obj, view top`.
    fn load_named(&self, path: &Path) -> Result<Vec<(String, Frame)>, Self::Error> {
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Ok(self
            .load(path)?
            .into_iter()
            .zip(rig::VIEWS.iter())
            .map(|(frame, view)| (format!("{file}, view {}", view.name), frame))
            .collect())
    }
}
