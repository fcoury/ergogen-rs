use std::path::PathBuf;

use ergogen_export::dxf::{NormalizeOptions, compare_files_semantic};
use ergogen_export::dxf_geom::dxf_from_region;

fn fixture_dxf_opts() -> NormalizeOptions {
    // We treat DXF fixtures as *semantic* comparisons. DXFs produced from bulge polylines vs
    // direct ARC entities can differ slightly in derived circle centers/radii for very small spans
    // (large radii), so use a slightly coarser quantization.
    NormalizeOptions {
        linear_eps: 1e-3,
        angle_eps_deg: 5e-3,
        ..NormalizeOptions::default()
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn assert_fixture_matches_golden_dxfs_semantically(base: &str) {
    let root = workspace_root();
    let dir = root.join("fixtures/m5/outlines");
    let yaml_path = dir.join(format!("{base}.yaml"));
    let yaml = std::fs::read_to_string(&yaml_path).unwrap();

    let mut goldens: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.starts_with(&format!("{base}__"))
                    && n.contains("__outlines_")
                    && n.ends_with("_dxf.dxf")
            })
        })
        .collect();
    goldens.sort();
    assert!(!goldens.is_empty(), "no {base} outline goldens found");

    let opts = fixture_dxf_opts();
    for golden_path in goldens {
        let fname = golden_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let outline_name = fname
            .split_once("__outlines_")
            .and_then(|(_, rest)| rest.strip_suffix("_dxf.dxf"))
            .expect("golden name");

        let region =
            ergogen_outline::generate_outline_region_from_yaml_str(&yaml, outline_name).unwrap();

        let dxf = dxf_from_region(&region).unwrap();
        let normalized = dxf.normalize(opts).unwrap();
        let out_str = normalized.to_dxf_string(opts).unwrap();

        let out_dir = std::env::temp_dir().join("ergogen-outline-goldens");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out_path = out_dir.join(
            fname
                .replace("___outlines_", "___generated_")
                .replace("__outlines_", "__generated_"),
        );
        std::fs::write(&out_path, out_str).unwrap();

        if let Err(e) = compare_files_semantic(&out_path, &golden_path, opts) {
            panic!(
                "DXF semantic mismatch for {base}:{outline_name} ({fname}): {e:?}\n  ours: {:?}\n  upstream: {:?}",
                out_path, golden_path
            );
        }
    }
}

#[test]
fn basic_outline_matches_upstream_golden_dxf_semantically() {
    assert_fixture_matches_golden_dxfs_semantically("basic");
}

#[test]
fn rectangles_outline_matches_upstream_golden_dxf_semantically() {
    assert_fixture_matches_golden_dxfs_semantically("rectangles");
}

#[test]
fn circles_outline_matches_upstream_golden_dxf_semantically() {
    assert_fixture_matches_golden_dxfs_semantically("circles");
}

#[test]
fn polygons_outline_matches_upstream_golden_dxf_semantically() {
    assert_fixture_matches_golden_dxfs_semantically("polygons");
}

#[test]
fn binding_outline_matches_upstream_golden_dxf_semantically() {
    assert_fixture_matches_golden_dxfs_semantically("binding");
}

#[test]
fn affect_mirror_outline_matches_upstream_golden_dxf_semantically() {
    assert_fixture_matches_golden_dxfs_semantically("affect_mirror");
}

#[test]
fn expand_outlines_match_upstream_golden_dxf_semantically() {
    assert_fixture_matches_golden_dxfs_semantically("expand");
}

#[test]
fn hull_outlines_match_upstream_golden_dxf_semantically() {
    assert_fixture_matches_golden_dxfs_semantically("hull");
}

#[test]
fn outlines_outlines_match_upstream_golden_dxf_semantically() {
    assert_fixture_matches_golden_dxfs_semantically("outlines");
}

#[test]
fn path_outlines_match_upstream_golden_dxf_semantically() {
    assert_fixture_matches_golden_dxfs_semantically("path");
}
