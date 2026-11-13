//! The video adapter: software decode of AV1 in an IVF file.
//!
//! This is the only crate that knows a file can be a video.
//!
//! - The decoder is `re_rav1d`, a Rust port of dav1d, built without
//!   assembly. It runs on one thread. AV1 decode is exact by the
//!   specification, and the conformance test checks this against libdav1d.
//! - The colour conversion reads the matrix and the range tags of the
//!   stream, and uses integer arithmetic only. An unknown tag is an error.
//! - Two videos pair by presentation time, not by frame index. See
//!   `pair_by_time`.
//!
//! The shipped codec path is AV1, 8 bits, in IVF. Each other codec and
//! container is out of v1. A stream of more than 8 bits is refused.

#![forbid(unsafe_code)]

pub mod colour;
pub mod ivf;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use chrys_source::{Frame, Source};
use colour::{Coefficients, Matrix, Range};
use re_rav1d::dav1d::{
    Decoder, Error as DecodeError, Picture, PixelLayout, PlanarImageComponent, Settings,
};

/// The resource limits for one video load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoLimits {
    /// The largest file size, in bytes.
    pub max_file_bytes: u64,
    /// The largest number of frames.
    pub max_frames: usize,
    /// The largest number of pixels in one frame.
    pub max_frame_pixels: u32,
}

impl Default for VideoLimits {
    fn default() -> Self {
        VideoLimits {
            max_file_bytes: 256 * 1024 * 1024,
            max_frames: 10_000,
            max_frame_pixels: 16384 * 16384,
        }
    }
}

/// The error that the video adapter returns.
#[derive(Debug, thiserror::Error)]
pub enum VideoError {
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
    /// The container does not parse.
    #[error("{path}: {reason}")]
    Container {
        /// The path.
        path: PathBuf,
        /// The reason.
        reason: ivf::IvfError,
    },
    /// The codec is not AV1.
    #[error("{path} holds the codec {fourcc:?}, and only AV01 is supported")]
    Codec {
        /// The path.
        path: PathBuf,
        /// The four-character code.
        fourcc: String,
    },
    /// The decoder refused the stream.
    #[error("{path}: the AV1 decoder failed: {message}")]
    Decode {
        /// The path.
        path: PathBuf,
        /// The decoder message.
        message: String,
    },
    /// The stream uses a feature that this adapter does not support.
    #[error("{path}: {message}")]
    Unsupported {
        /// The path.
        path: PathBuf,
        /// What is not supported.
        message: String,
    },
    /// The stream holds no frame, or more frames than the limit.
    #[error("{path} holds {frames} frames, and the range is 1 to {limit}")]
    FrameCount {
        /// The path.
        path: PathBuf,
        /// The number of frames.
        frames: usize,
        /// The limit.
        limit: usize,
    },
}

/// One decoded frame and its presentation timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedFrame {
    /// The frame, in the canonical pixel format.
    pub frame: Frame,
    /// The presentation timestamp, in time base units.
    pub pts: i64,
}

/// A decoded video.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    /// The file name.
    pub name: String,
    /// One timestamp unit is `time_base.0 / time_base.1` seconds.
    pub time_base: (u32, u32),
    /// The frames, in presentation order.
    pub frames: Vec<TimedFrame>,
}

/// One plane of a decoded picture, as plain bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaneData {
    /// The bytes of each row, without the stride padding.
    pub bytes: Vec<u8>,
    /// The width, in samples.
    pub width: u32,
    /// The height, in samples.
    pub height: u32,
}

/// Return true when the file at `path` starts with the IVF signature.
pub fn looks_like_video(path: &Path) -> std::io::Result<bool> {
    let mut head = [0u8; 4];
    let read = File::open(path)?.read(&mut head)?;
    Ok(ivf::is_ivf(&head[..read]))
}

/// The time base and the decoded pictures, each with its timestamp.
type Decoded = ((u32, u32), Vec<(Picture, i64)>);

/// The video `Source`.
pub struct VideoSource {
    limits: VideoLimits,
}

impl VideoSource {
    /// Make a source with the default limits.
    pub fn new() -> Self {
        VideoSource {
            limits: VideoLimits::default(),
        }
    }

    /// Make a source with the given limits.
    pub fn with_limits(limits: VideoLimits) -> Self {
        VideoSource { limits }
    }

    /// Decode each frame of the file at `path` and return the raw planes,
    /// without a colour conversion. The conformance test reads this.
    pub fn decode_planes(&self, path: &Path) -> Result<Vec<Vec<PlaneData>>, VideoError> {
        let pictures = self.decode(path)?;
        Ok(pictures
            .iter()
            .map(|(picture, _)| planes(picture))
            .collect())
    }

    /// Decode the file at `path` into frames with timestamps.
    pub fn load_video(&self, path: &Path) -> Result<Video, VideoError> {
        let (time_base, pictures) = self.decode_with_time_base(path)?;
        let mut frames = Vec::with_capacity(pictures.len());
        for (index, (picture, pts)) in pictures.iter().enumerate() {
            frames.push(TimedFrame {
                frame: to_frame(picture, index, path)?,
                pts: *pts,
            });
        }
        // Presentation order. A stable sort keeps decode order for a tie.
        frames.sort_by_key(|timed| timed.pts);
        for (index, timed) in frames.iter_mut().enumerate() {
            timed.frame.index = index;
        }
        Ok(Video {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
            time_base,
            frames,
        })
    }

    fn decode(&self, path: &Path) -> Result<Vec<(Picture, i64)>, VideoError> {
        Ok(self.decode_with_time_base(path)?.1)
    }

    fn decode_with_time_base(&self, path: &Path) -> Result<Decoded, VideoError> {
        let bytes = read_bounded(path, self.limits.max_file_bytes)?;
        let (header, packets) = ivf::parse(&bytes).map_err(|reason| VideoError::Container {
            path: path.to_path_buf(),
            reason,
        })?;
        if &header.fourcc != b"AV01" {
            return Err(VideoError::Codec {
                path: path.to_path_buf(),
                fourcc: String::from_utf8_lossy(&header.fourcc).into_owned(),
            });
        }
        if packets.is_empty() || packets.len() > self.limits.max_frames {
            return Err(VideoError::FrameCount {
                path: path.to_path_buf(),
                frames: packets.len(),
                limit: self.limits.max_frames,
            });
        }

        let decode_error = |error: DecodeError| VideoError::Decode {
            path: path.to_path_buf(),
            message: format!("{error:?}"),
        };
        let mut settings = Settings::new();
        settings.set_n_threads(1);
        settings.set_max_frame_delay(1);
        settings.set_frame_size_limit(self.limits.max_frame_pixels);
        let mut decoder = Decoder::with_settings(&settings).map_err(decode_error)?;

        let mut pictures = Vec::new();
        for packet in packets {
            let mut result = decoder.send_data(packet.data, None, Some(packet.pts), None);
            loop {
                match result {
                    Ok(()) => break,
                    Err(DecodeError::Again) => {
                        drain(&mut decoder, &mut pictures).map_err(decode_error)?;
                        result = decoder.send_pending_data();
                    }
                    Err(error) => return Err(decode_error(error)),
                }
            }
            drain(&mut decoder, &mut pictures).map_err(decode_error)?;
        }
        drain(&mut decoder, &mut pictures).map_err(decode_error)?;

        if pictures.len() > self.limits.max_frames {
            return Err(VideoError::FrameCount {
                path: path.to_path_buf(),
                frames: pictures.len(),
                limit: self.limits.max_frames,
            });
        }
        Ok(((header.numerator, header.denominator), pictures))
    }
}

impl Default for VideoSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for VideoSource {
    type Error = VideoError;

    fn load(&self, path: &Path) -> Result<Vec<Frame>, Self::Error> {
        Ok(self
            .load_video(path)?
            .frames
            .into_iter()
            .map(|timed| timed.frame)
            .collect())
    }
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, VideoError> {
    let io = |source| VideoError::Io {
        path: path.to_path_buf(),
        source,
    };
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(io)?
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() as u64 > limit {
        return Err(VideoError::FileTooLarge {
            path: path.to_path_buf(),
            limit,
        });
    }
    Ok(bytes)
}

/// Take each picture that the decoder holds now.
fn drain(decoder: &mut Decoder, pictures: &mut Vec<(Picture, i64)>) -> Result<(), DecodeError> {
    loop {
        match decoder.get_picture() {
            Ok(picture) => {
                let pts = picture.timestamp().unwrap_or(0);
                pictures.push((picture, pts));
            }
            Err(DecodeError::Again) => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn plane_geometry(picture: &Picture, component: PlanarImageComponent) -> (u32, u32) {
    let (width, height) = (picture.width(), picture.height());
    match (component, picture.pixel_layout()) {
        (PlanarImageComponent::Y, _) | (_, PixelLayout::I444) => (width, height),
        (_, PixelLayout::I422) => (width.div_ceil(2), height),
        (_, PixelLayout::I420) => (width.div_ceil(2), height.div_ceil(2)),
        (_, PixelLayout::I400) => (0, 0),
    }
}

fn planes(picture: &Picture) -> Vec<PlaneData> {
    let components = match picture.pixel_layout() {
        PixelLayout::I400 => vec![PlanarImageComponent::Y],
        _ => vec![
            PlanarImageComponent::Y,
            PlanarImageComponent::U,
            PlanarImageComponent::V,
        ],
    };
    components
        .into_iter()
        .map(|component| {
            let (width, height) = plane_geometry(picture, component);
            let stride = picture.stride(component) as usize;
            let plane = picture.plane(component);
            let mut bytes = Vec::with_capacity((width * height) as usize);
            for row in 0..height as usize {
                let start = row * stride;
                bytes.extend_from_slice(&plane[start..start + width as usize]);
            }
            PlaneData {
                bytes,
                width,
                height,
            }
        })
        .collect()
}

/// Read the matrix tag. An unknown or a missing tag is an error.
fn matrix_of(picture: &Picture, path: &Path) -> Result<Matrix, VideoError> {
    use re_rav1d::dav1d::pixel::MatrixCoefficients as M;
    match picture.matrix_coefficients() {
        M::BT709 => Ok(Matrix::Bt709),
        M::BT470BG | M::ST170M => Ok(Matrix::Bt601),
        M::BT2020NonConstantLuminance => Ok(Matrix::Bt2020Ncl),
        M::Identity => Ok(Matrix::Identity),
        other => Err(VideoError::Unsupported {
            path: path.to_path_buf(),
            message: format!(
                "the stream declares the colour matrix {other:?}; tag the matrix as BT.601, BT.709 or BT.2020"
            ),
        }),
    }
}

fn to_frame(picture: &Picture, index: usize, path: &Path) -> Result<Frame, VideoError> {
    if picture.bit_depth() != 8 {
        return Err(VideoError::Unsupported {
            path: path.to_path_buf(),
            message: format!(
                "the stream has {} bits per sample, and only 8 is supported",
                picture.bit_depth()
            ),
        });
    }
    let range = match picture.color_range() {
        re_rav1d::dav1d::pixel::YUVRange::Full => Range::Full,
        re_rav1d::dav1d::pixel::YUVRange::Limited => Range::Limited,
    };
    let layout = picture.pixel_layout();
    let matrix = if layout == PixelLayout::I400 {
        Matrix::Bt709
    } else {
        matrix_of(picture, path)?
    };
    let coefficients = Coefficients::new(matrix, range);
    let planes = planes(picture);
    let (width, height) = (picture.width(), picture.height());
    let (shift_x, shift_y) = match layout {
        PixelLayout::I420 => (1, 1),
        PixelLayout::I422 => (1, 0),
        PixelLayout::I444 | PixelLayout::I400 => (0, 0),
    };

    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let luma = planes[0].bytes[(y * planes[0].width + x) as usize];
            let (cb, cr) = if layout == PixelLayout::I400 {
                (128, 128)
            } else {
                // The nearest chroma sample. No filter, so no rounding
                // choice exists here.
                let at = ((y >> shift_y) * planes[1].width + (x >> shift_x)) as usize;
                (planes[1].bytes[at], planes[2].bytes[at])
            };
            let [r, g, b] = coefficients.to_rgb(luma, cb, cr);
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Ok(Frame {
        pixels,
        width,
        height,
        index,
        hints: Vec::new(),
    })
}

/// Two videos, paired by presentation time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedVideo {
    /// The base frame on screen at each time.
    pub base: Vec<Frame>,
    /// The candidate frame on screen at each time.
    pub candidate: Vec<Frame>,
    /// A label for each time: the file, the frame and the time.
    pub names: Vec<String>,
}

/// The time of `pts` as a fraction: `(pts * numerator, denominator)`.
fn time_of(pts: i64, time_base: (u32, u32)) -> (i128, i128) {
    (
        i128::from(pts) * i128::from(time_base.0),
        i128::from(time_base.1),
    )
}

fn compare_times(a: (i128, i128), b: (i128, i128)) -> std::cmp::Ordering {
    (a.0 * b.1).cmp(&(b.0 * a.1))
}

/// Pair two videos by presentation time.
///
/// The pairs are made at each time where either video shows a new frame,
/// from the first time where both videos show a frame. At each such time,
/// each side gives the frame that is on screen: the last frame whose
/// timestamp is not after that time.
///
/// This is why a change in frame rate alone is not a change. A video at
/// 20 frames per second that shows each frame of a 10 frames per second
/// video two times gives the same picture at each time.
pub fn pair_by_time(base: &Video, candidate: &Video) -> PairedVideo {
    let times = |video: &Video| -> Vec<(i128, i128)> {
        video
            .frames
            .iter()
            .map(|timed| time_of(timed.pts, video.time_base))
            .collect()
    };
    let base_times = times(base);
    let candidate_times = times(candidate);
    let mut events: Vec<(i128, i128)> = base_times
        .iter()
        .chain(candidate_times.iter())
        .copied()
        .collect();
    events.sort_by(|a, b| compare_times(*a, *b));
    events.dedup_by(|a, b| compare_times(*a, *b).is_eq());

    let on_screen = |frame_times: &[(i128, i128)], at: (i128, i128)| {
        frame_times
            .iter()
            .rposition(|time| compare_times(*time, at).is_le())
    };

    let mut paired = PairedVideo {
        base: Vec::new(),
        candidate: Vec::new(),
        names: Vec::new(),
    };
    for event in events {
        let (Some(b), Some(c)) = (
            on_screen(&base_times, event),
            on_screen(&candidate_times, event),
        ) else {
            continue;
        };
        let index = paired.base.len();
        let mut base_frame = base.frames[b].frame.clone();
        let mut candidate_frame = candidate.frames[c].frame.clone();
        base_frame.index = index;
        candidate_frame.index = index;
        // Milliseconds, rounded down, in integer arithmetic.
        let millis = event.0 * 1000 / event.1;
        paired
            .names
            .push(format!("{} frame {} at {millis} ms", base.name, b));
        paired.base.push(base_frame);
        paired.candidate.push(candidate_frame);
    }
    paired
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video(name: &str, time_base: (u32, u32), frames: &[(i64, u8)]) -> Video {
        Video {
            name: name.to_string(),
            time_base,
            frames: frames
                .iter()
                .enumerate()
                .map(|(index, &(pts, value))| TimedFrame {
                    frame: Frame {
                        pixels: vec![value; 4],
                        width: 1,
                        height: 1,
                        index,
                        hints: Vec::new(),
                    },
                    pts,
                })
                .collect(),
        }
    }

    #[test]
    fn a_double_rate_copy_pairs_each_frame_with_itself() {
        let base = video("a", (1, 10), &[(0, 1), (1, 2), (2, 3)]);
        let double = video(
            "b",
            (1, 20),
            &[(0, 1), (1, 1), (2, 2), (3, 2), (4, 3), (5, 3)],
        );
        let paired = pair_by_time(&base, &double);
        assert_eq!(paired.base.len(), 6);
        for (b, c) in paired.base.iter().zip(&paired.candidate) {
            assert_eq!(b.pixels, c.pixels);
        }
        assert_eq!(paired.names[1], "a frame 0 at 50 ms");
    }

    #[test]
    fn a_pair_by_index_would_differ_where_a_pair_by_time_does_not() {
        // The same check from the other side: by index, frame 1 of the
        // double-rate copy is picture 1, and frame 1 of the base is picture 2.
        let base = video("a", (1, 10), &[(0, 1), (1, 2)]);
        let double = video("b", (1, 20), &[(0, 1), (1, 1), (2, 2), (3, 2)]);
        assert_ne!(base.frames[1].frame.pixels, double.frames[1].frame.pixels);
        let paired = pair_by_time(&base, &double);
        assert!(
            paired
                .base
                .iter()
                .zip(&paired.candidate)
                .all(|(b, c)| b.pixels == c.pixels)
        );
    }

    #[test]
    fn a_time_before_both_videos_start_is_not_paired() {
        let base = video("a", (1, 10), &[(2, 1), (3, 2)]);
        let candidate = video("b", (1, 10), &[(0, 9), (3, 2)]);
        let paired = pair_by_time(&base, &candidate);
        assert_eq!(paired.base.len(), 2);
        assert_eq!(paired.candidate[0].pixels, vec![9; 4]);
    }

    #[test]
    fn the_paired_index_counts_the_pairs() {
        let base = video("a", (1, 10), &[(0, 1), (1, 2)]);
        let paired = pair_by_time(&base, &base);
        assert_eq!(
            paired.base.iter().map(|f| f.index).collect::<Vec<_>>(),
            vec![0, 1]
        );
    }
}
