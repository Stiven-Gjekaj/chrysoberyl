//! The mask image loader: the white-and-opaque membership rule (D-02), and
//! the path-traversal refusal that keeps the one file-path field this
//! schema holds from reaching outside the rule file's own directory.
//!
//! `load_mask` calls `chrys_source_raster::decode::decode_guarded` and
//! `chrys_source_raster::normalize::normalize_to_rgba8` and opens no
//! reader of its own, so a mask decodes through the one guarded entry
//! point every other raster decode in this project already uses, with the
//! same memory limit (CLI-04).

use std::path::{Component, Path, PathBuf};

use crate::RuleError;

/// The lowest red, green or blue byte value a mask pixel may hold and
/// still count toward "white" (D-02).
///
/// A tolerated pixel must sit at or above this floor on every one of its
/// three colour channels, AND hold exactly `TOLERATED_ALPHA`. White alone
/// is not enough: a mask authored as white on a transparent background
/// holds the bytes 255, 255, 255, 0 everywhere outside its drawn area, and
/// a convention that read only the colour channels would tolerate that
/// whole canvas without ever saying so. Phase 1 settled that alpha is part
/// of what "changed" means; this constant is that decision reaching the
/// mask.
pub const TOLERATED_CHANNEL_FLOOR: u8 = 250;

/// The alpha byte value a mask pixel must hold exactly to count as
/// tolerated (D-02).
///
/// Anything below full opacity is not drawn, whatever its colour bytes
/// say: a mask pixel that is white with an alpha byte below 255 does not
/// count as tolerated, because a partially transparent pixel is a pixel a
/// mask's own author did not fully commit to.
pub const TOLERATED_ALPHA: u8 = 255;

/// A decoded mask image: one boolean per pixel, `true` where that pixel is
/// tolerated (white and opaque, D-02), alongside the mask's own width and
/// height.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    tolerated: Vec<bool>,
    width: u32,
    height: u32,
}

impl Mask {
    /// The mask's own width, in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The mask's own height, in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Whether `bbox` is tolerated by this mask: at least half of `bbox`'s
    /// own area sits on tolerated (white and opaque) mask pixels, the same
    /// majority rule `crate::evaluate::overlapping_hint_name` already uses
    /// for a named region, so a reader learns one convention rather than
    /// two.
    ///
    /// A box that reaches outside the mask is clamped to the mask for the
    /// count, but never for the area: the denominator is always `bbox`'s
    /// own full area, so a box half outside the mask can never be
    /// tolerated by pixels that do not exist.
    ///
    /// Every calculation here is `u64` integer arithmetic: no `f32`, no
    /// `f64`.
    pub fn tolerates(&self, bbox: &chrys_core::BoundingBox) -> bool {
        let box_area = u64::from(bbox.width) * u64::from(bbox.height);
        if box_area == 0 {
            return false;
        }

        let x0 = bbox.x.min(self.width);
        let y0 = bbox.y.min(self.height);
        let x1 = bbox.x.saturating_add(bbox.width).min(self.width);
        let y1 = bbox.y.saturating_add(bbox.height).min(self.height);

        let mut tolerated_count: u64 = 0;
        for y in y0..y1 {
            let row_start = (y as usize) * (self.width as usize);
            for x in x0..x1 {
                if self.tolerated[row_start + x as usize] {
                    tolerated_count += 1;
                }
            }
        }

        tolerated_count * 2 >= box_area
    }
}

/// Decode the mask image at `path`, with `limits` enforced by the guarded
/// decoder before any pixel buffer is allocated.
///
/// `path` is resolved and safety-checked by the caller (`load_rules`)
/// before this function ever sees it; this function only decodes and
/// reduces the result to a `Mask`. Taking `limits` as a parameter rather
/// than hard-coding the default is what lets a test prove the limit is
/// applied instead of ignored; `load_rules` calls this with
/// `chrys_source_raster::DecodeLimits::default()`, the same 512 MiB
/// allocation ceiling and the same 16384-pixel side limit every other
/// decode in this project already uses (CLI-04).
pub fn load_mask(
    path: &Path,
    limits: &chrys_source_raster::DecodeLimits,
) -> Result<Mask, RuleError> {
    let (dynamic, orientation) = chrys_source_raster::decode::decode_guarded(path, limits)
        .map_err(|source| RuleError::MaskDecode {
            path: path.to_path_buf(),
            message: source.to_string(),
        })?;
    let (pixels, width, height) =
        chrys_source_raster::normalize::normalize_to_rgba8(dynamic, orientation);

    let pixel_count = (width as usize) * (height as usize);
    let mut tolerated = Vec::with_capacity(pixel_count);
    for chunk in pixels.chunks_exact(4) {
        let is_tolerated = chunk[0] >= TOLERATED_CHANNEL_FLOOR
            && chunk[1] >= TOLERATED_CHANNEL_FLOOR
            && chunk[2] >= TOLERATED_CHANNEL_FLOOR
            && chunk[3] == TOLERATED_ALPHA;
        tolerated.push(is_tolerated);
    }

    Ok(Mask {
        tolerated,
        width,
        height,
    })
}

/// Resolve `mask_path` (a `Scope::Mask` rule's own, as-written path)
/// against `rule_dir` (the rule file's own directory), refusing a path
/// that leaves `rule_dir`, and return the resolved path otherwise.
///
/// A rule file is structured input read as data, and after this plan it
/// holds a file path: the one field in this schema that reaches outside
/// the document. `03-VALIDATION.md` states the rule this function
/// enforces: a mask resolves relative to the rule file's own directory and
/// must not escape it, the same convention `hints.rs` already uses for a
/// sidecar.
///
/// Two checks, not one, and both of them, run in this order:
///
/// 1. **Lexical.** Refuse `mask_path` outright when it is absolute or
///    holds any parent-directory (`..`) component. This check runs before
///    the file system is touched at all, which is what makes a hostile
///    path cost nothing: no byte of any file is read.
/// 2. **Canonical.** Resolve `rule_dir` and the joined path to their
///    canonical form and refuse when the mask's canonical path does not
///    sit inside the directory's canonical path. The lexical check alone
///    cannot see a symbolic link that points outside `rule_dir`; this one
///    can, the same defence-in-depth pairing phase 1 recorded for a
///    source-level guard and one that depends on the environment.
///
/// Neither check falls back to reading the file, silently drops the rule,
/// or canonicalizes first and asks afterwards (T-03-08).
pub(crate) fn resolve_mask_path(rule_dir: &Path, mask_path: &Path) -> Result<PathBuf, RuleError> {
    if mask_path.is_absolute()
        || mask_path
            .components()
            .any(|component| component == Component::ParentDir)
    {
        return Err(RuleError::MaskPathEscapesDirectory {
            rule_dir: rule_dir.to_path_buf(),
            mask_path: mask_path.to_path_buf(),
        });
    }

    let joined = rule_dir.join(mask_path);

    let canonical_dir = rule_dir.canonicalize().map_err(|source| RuleError::Io {
        path: rule_dir.to_path_buf(),
        source,
    })?;
    let canonical_mask = joined.canonicalize().map_err(|source| RuleError::Io {
        path: joined.clone(),
        source,
    })?;

    if !canonical_mask.starts_with(&canonical_dir) {
        return Err(RuleError::MaskPathEscapesDirectory {
            rule_dir: rule_dir.to_path_buf(),
            mask_path: mask_path.to_path_buf(),
        });
    }

    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::path::PathBuf;

    /// Write a flat-colour PNG to a fresh temporary path, so a test can
    /// decode it through `load_mask` without depending on a committed
    /// fixture. `label` keeps every temporary file this test module
    /// writes from colliding with another test's own file.
    fn temp_mask_png(label: &str, width: u32, height: u32, pixel: [u8; 4]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("chrys-rule-mask-test-{label}.png"));
        RgbaImage::from_pixel(width, height, Rgba(pixel))
            .save(&path)
            .expect("write temp mask png");
        path
    }

    #[test]
    fn a_white_pixel_that_is_not_opaque_is_not_tolerated() {
        // White on every colour channel, one byte short of full opacity:
        // D-02 requires alpha to equal 255 exactly, not merely be high.
        let path = temp_mask_png("not-opaque", 4, 4, [255, 255, 255, 254]);
        let mask = load_mask(&path, &chrys_source_raster::DecodeLimits::default())
            .expect("a well-formed PNG decodes");
        std::fs::remove_file(&path).ok();

        let bbox = chrys_core::BoundingBox {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        assert!(
            !mask.tolerates(&bbox),
            "a white pixel below full opacity must not be tolerated (D-02)"
        );
    }

    #[test]
    fn a_white_and_opaque_pixel_is_tolerated() {
        let path = temp_mask_png("opaque", 4, 4, [255, 255, 255, 255]);
        let mask = load_mask(&path, &chrys_source_raster::DecodeLimits::default())
            .expect("a well-formed PNG decodes");
        std::fs::remove_file(&path).ok();

        let bbox = chrys_core::BoundingBox {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        assert!(mask.tolerates(&bbox));
    }

    #[test]
    fn a_channel_just_below_the_floor_is_not_tolerated() {
        // 249 is one below TOLERATED_CHANNEL_FLOOR (250): white enough to
        // look white, but not white enough to count.
        let path = temp_mask_png("below-floor", 4, 4, [249, 249, 249, 255]);
        let mask = load_mask(&path, &chrys_source_raster::DecodeLimits::default())
            .expect("a well-formed PNG decodes");
        std::fs::remove_file(&path).ok();

        let bbox = chrys_core::BoundingBox {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        assert!(!mask.tolerates(&bbox));
    }

    #[test]
    fn a_box_reaching_outside_the_mask_is_not_tolerated_by_pixels_that_do_not_exist() {
        // A 4x4 mask, fully tolerated. A box twice as wide as the mask can
        // have at most half its own area counted, which is exactly the
        // boundary: not strictly over half, so not tolerated.
        let path = temp_mask_png("clamped", 4, 4, [255, 255, 255, 255]);
        let mask = load_mask(&path, &chrys_source_raster::DecodeLimits::default())
            .expect("a well-formed PNG decodes");
        std::fs::remove_file(&path).ok();

        let bbox = chrys_core::BoundingBox {
            x: 0,
            y: 0,
            width: 8,
            height: 4,
        };
        // Only the left half (4x4 = 16 px) of the 8x4 = 32 px box overlaps
        // the mask at all, and every one of those 16 px is tolerated:
        // 16 * 2 = 32 >= 32, so this box is tolerated at exactly the
        // boundary.
        assert!(mask.tolerates(&bbox));

        let bbox_mostly_outside = chrys_core::BoundingBox {
            x: 0,
            y: 0,
            width: 12,
            height: 4,
        };
        // Now only 16 of 48 px overlap the mask: 16 * 2 = 32 < 48, so this
        // box is not tolerated by pixels that do not exist.
        assert!(!mask.tolerates(&bbox_mostly_outside));
    }
}
