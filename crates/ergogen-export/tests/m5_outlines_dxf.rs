use std::path::PathBuf;

use ergogen_export::dxf::{Dxf, NormalizeOptions};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn parses_and_normalizes_all_m5_outline_goldens() {
    let root = workspace_root();
    let dir = root.join("fixtures/m5/outlines");

    let mut dxfs: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "dxf"))
        .collect();
    dxfs.sort();

    assert!(!dxfs.is_empty(), "no .dxf files found in {dir:?}");

    for path in dxfs {
        let dxf =
            Dxf::parse_file(&path).unwrap_or_else(|e| panic!("failed to parse {path:?}: {e}"));
        dxf.normalize(NormalizeOptions::default())
            .unwrap_or_else(|e| panic!("failed to normalize {path:?}: {e}"));
    }
}
