use std::path::PathBuf;

use ergogen_export::dxf::{NormalizeOptions, compare_files_semantic};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn outline_dxf_goldens_are_self_semantically_consistent() {
    // This is a "harness test": it ensures we can enumerate the upstream DXF goldens and that
    // parse+normalize+compare works end-to-end.
    //
    // Once we have an outline generator + DXF writer, replace the "golden vs golden" comparison
    // with "generated vs golden".
    let root = workspace_root();
    let dir = root.join("fixtures/m5/outlines");

    let mut dxfs: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains("___outlines_") && n.ends_with("_dxf.dxf"))
        })
        .collect();
    dxfs.sort();

    assert!(!dxfs.is_empty(), "no outline goldens found in {dir:?}");

    for dxf in dxfs {
        compare_files_semantic(&dxf, &dxf, NormalizeOptions::default())
            .unwrap_or_else(|e| panic!("semantic compare failed for {dxf:?}: {e}"));
    }
}
