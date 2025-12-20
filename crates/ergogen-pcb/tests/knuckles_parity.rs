#![cfg(feature = "js-footprints")]

use std::path::{Path, PathBuf};

use ergogen_export::dxf::{NormalizeOptions, compare_files_semantic};
use ergogen_export::dxf_geom::dxf_from_region;
use ergogen_outline::generate_outline_region_from_yaml_str;

use ergogen_pcb::generate_kicad_pcb_from_yaml_str;

fn fixture_dxf_opts() -> NormalizeOptions {
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

fn fixture_root() -> PathBuf {
    workspace_root().join("fixtures/m8/knuckles")
}

fn norm_text(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end_matches('\n').to_string()
}

fn write_if_changed(path: &Path, contents: &str) {
    let write = match std::fs::read_to_string(path) {
        Ok(existing) => existing != contents,
        Err(_) => true,
    };
    if write {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }
}

#[test]
fn knuckles_assets_match_goldens_and_optional_upstream() {
    let fixture_dir = fixture_root();
    let yaml_path = fixture_dir.join("knuckles_assets.yaml");
    let yaml = std::fs::read_to_string(&yaml_path).unwrap();

    let out_dir = workspace_root().join("target/knuckles-parity");
    std::fs::create_dir_all(&out_dir).unwrap();

    // ---- Outline DXF ----
    let region = generate_outline_region_from_yaml_str(&yaml, "pcb").unwrap();
    let dxf = dxf_from_region(&region).unwrap();
    let opts = fixture_dxf_opts();
    let normalized = dxf.normalize(opts).unwrap();
    let got_dxf = normalized.to_dxf_string(opts).unwrap();

    let golden_dxf = fixture_dir.join("knuckles_assets___outlines_pcb_dxf.dxf");
    let got_dxf_path = out_dir.join("knuckles_assets__generated_pcb.dxf");
    std::fs::write(&got_dxf_path, &got_dxf).unwrap();

    // ---- KiCad PCB ----
    let got_pcb = generate_kicad_pcb_from_yaml_str(&yaml, "pcb").unwrap();
    let got_pcb = norm_text(&got_pcb);
    let golden_pcb = fixture_dir.join("knuckles_assets___pcbs_pcb.kicad_pcb");
    let got_pcb_path = out_dir.join("knuckles_assets__generated_pcb.kicad_pcb");
    std::fs::write(&got_pcb_path, &got_pcb).unwrap();

    // ---- Update goldens ----
    if std::env::var("UPDATE_GOLDENS").as_deref() == Ok("1") {
        write_if_changed(&golden_dxf, &got_dxf);
        write_if_changed(&golden_pcb, &got_pcb);
        return;
    }

    // ---- Golden checks ----
    assert!(
        golden_dxf.exists(),
        "missing golden: {} (set UPDATE_GOLDENS=1 to generate)",
        golden_dxf.display()
    );
    assert!(
        golden_pcb.exists(),
        "missing golden: {} (set UPDATE_GOLDENS=1 to generate)",
        golden_pcb.display()
    );

    compare_files_semantic(&got_dxf_path, &golden_dxf, opts).unwrap();

    let expected_pcb = norm_text(&std::fs::read_to_string(&golden_pcb).unwrap());
    assert_eq!(
        got_pcb,
        expected_pcb,
        "kicad_pcb output mismatch (wrote got output to {})",
        got_pcb_path.display()
    );

    // ---- Optional upstream checks ----
    if let Ok(upstream_dir) = std::env::var("UPSTREAM_BASELINE_DIR") {
        let base = PathBuf::from(upstream_dir);

        let upstream_dxf = base.join("outlines/pcb.dxf");
        if upstream_dxf.exists() {
            compare_files_semantic(&got_dxf_path, &upstream_dxf, opts).unwrap();
        }

        let upstream_pcb = base.join("pcbs/pcb.kicad_pcb");
        if upstream_pcb.exists() {
            let expected = norm_text(&std::fs::read_to_string(&upstream_pcb).unwrap());
            assert_eq!(
                got_pcb,
                expected,
                "kicad_pcb output mismatch vs upstream baseline (wrote got output to {})",
                got_pcb_path.display()
            );
        }
    }
}
