//! Tests for the mesh adapter, through its public API.

use std::path::PathBuf;

use chrys_source::Source;
use chrys_source_mesh::{BlindSpot, MeshError, MeshSource, raster, rig};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/formats/mesh")
        .join(name)
}

fn scratch(test: &str, bytes: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("chrys-mesh-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("make the scratch directory");
    let path = dir.join("input");
    std::fs::write(&path, bytes).expect("write");
    path
}

#[test]
fn a_mesh_becomes_eight_views() {
    let frames = MeshSource::new().load(&fixture("base.obj")).expect("load");
    assert_eq!(frames.len(), rig::VIEWS.len());
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(frame.index, index);
        assert_eq!((frame.width, frame.height), (raster::SIZE, raster::SIZE));
        let empty = raster::render(&[], &rig::VIEWS[index]);
        let drawn = frame
            .pixels
            .chunks_exact(4)
            .zip(empty.pixels.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        assert!(drawn > 1000, "view {index} draws {drawn} pixels");
    }
}

#[test]
fn two_renders_give_the_same_bytes() {
    let source = MeshSource::new();
    assert_eq!(
        source.load(&fixture("base.obj")).expect("first"),
        source.load(&fixture("base.obj")).expect("second")
    );
}

#[test]
fn the_blind_spot_counts_the_hidden_triangles() {
    // The inner cube (12 triangles) and the base of the top block, which
    // sits flat on the lower block (2 triangles).
    let blind = MeshSource::new()
        .blind_spot(&fixture("base.obj"))
        .expect("render");
    assert_eq!(
        blind,
        BlindSpot {
            hidden: 14,
            total: 36
        }
    );
    assert!(
        blind
            .to_string()
            .starts_with("blind spot: 14 of 36 triangles")
    );
}

#[test]
fn the_candidate_changes_the_views_that_see_the_new_cube() {
    let source = MeshSource::new();
    let base = source.load(&fixture("base.obj")).expect("base");
    let candidate = source.load(&fixture("candidate.obj")).expect("candidate");
    let changed: Vec<&str> = base
        .iter()
        .zip(&candidate)
        .zip(rig::VIEWS.iter())
        .filter(|((b, c), _)| b.pixels != c.pixels)
        .map(|(_, view)| view.name)
        .collect();
    // The new cube sits on the top of the base block. The left view does
    // not see it, because the top block is in front of it. The bottom and
    // the lower corner views look from below.
    assert_eq!(
        changed,
        vec!["front", "back", "right", "top", "upper corner"]
    );
}

#[test]
fn a_binary_stl_renders_the_same_as_the_obj() {
    let source = MeshSource::new();
    let triangles = source.triangles(&fixture("base.obj")).expect("parse");
    let mut bytes = vec![0u8; 80];
    bytes.extend_from_slice(&(triangles.len() as u32).to_le_bytes());
    for triangle in &triangles {
        bytes.extend_from_slice(&[0u8; 12]);
        for value in triangle.iter().flatten() {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&[0, 0]);
    }
    let stl = scratch("stl", &bytes);
    assert_eq!(
        source.load(&stl).expect("stl"),
        source.load(&fixture("base.obj")).expect("obj")
    );
}

#[test]
fn each_frame_is_named_for_its_view() {
    let named = MeshSource::new()
        .load_named(&fixture("base.obj"))
        .expect("load");
    assert_eq!(named[4].0, "base.obj, view top");
}

#[test]
fn a_file_that_is_not_a_mesh_is_refused() {
    let path = scratch("not-mesh", b"hello");
    let error = MeshSource::new().load(&path).expect_err("refused");
    assert!(matches!(error, MeshError::NotAMesh { .. }), "{error}");
}
