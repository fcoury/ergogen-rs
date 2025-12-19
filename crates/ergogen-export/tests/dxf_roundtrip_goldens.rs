use std::path::PathBuf;

use ergogen_export::dxf::{Dxf, NormalizeOptions};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn assert_roundtrip_dir(relative: &str) {
    let root = workspace_root();
    let dir = root.join(relative);

    let opts = NormalizeOptions::default();

    let mut dxfs: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "dxf"))
        .collect();
    dxfs.sort();

    assert!(!dxfs.is_empty(), "no .dxf files found in {dir:?}");

    for path in dxfs {
        let orig = Dxf::parse_file(&path)
            .unwrap_or_else(|e| panic!("parse {path:?} failed: {e}"))
            .normalize(opts)
            .unwrap_or_else(|e| panic!("normalize {path:?} failed: {e}"));

        let written = orig
            .to_dxf_string(opts)
            .unwrap_or_else(|e| panic!("write {path:?} failed: {e}"));

        let reparsed = Dxf::parse_str(&written)
            .unwrap_or_else(|e| panic!("reparse written DXF from {path:?} failed: {e}"))
            .normalize(opts)
            .unwrap_or_else(|e| panic!("renormalize written DXF from {path:?} failed: {e}"));

        orig.compare_semantic(&reparsed)
            .unwrap_or_else(|e| panic!("semantic roundtrip mismatch for {path:?}: {e}"));
    }
}

#[test]
fn points_dxf_goldens_roundtrip_through_writer() {
    assert_roundtrip_dir("fixtures/m3/points");
}

#[test]
fn outlines_dxf_goldens_roundtrip_through_writer() {
    assert_roundtrip_dir("fixtures/m5/outlines");
}
