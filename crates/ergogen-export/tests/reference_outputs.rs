use std::path::{Path, PathBuf};

use ergogen_export::dxf::{NormalizeOptions, compare_files_semantic};
use ergogen_export::dxf_geom::dxf_from_region;
use ergogen_export::jscad::generate_cases_jscad_v2;
use ergogen_export::svg::svg_from_dxf;
use ergogen_outline::generate_outline_region;
use ergogen_parser::{PreparedConfig, Value};
use ergogen_pcb::generate_kicad_pcb;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn fixture_dxf_opts() -> NormalizeOptions {
    NormalizeOptions {
        linear_eps: 1e-3,
        angle_eps_deg: 5e-3,
        ..NormalizeOptions::default()
    }
}

fn normalize_text(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end_matches('\n').to_string()
}

fn temp_out_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("ergogen-reference-outputs");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn raw_output_root(root: &Path) -> PathBuf {
    root.join("target/reference-outputs/big")
}

fn compare_or_update(path: &Path, contents: &str, update: bool) {
    if update {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(path).unwrap();
    assert_eq!(normalize_text(contents), normalize_text(&expected));
}

fn compare_raw_text(golden: &Path, raw: &Path) {
    let golden_contents = std::fs::read_to_string(golden).unwrap();
    let raw_contents = std::fs::read_to_string(raw).unwrap();
    assert_eq!(normalize_text(&raw_contents), normalize_text(&golden_contents));
}

#[test]
fn reference_outputs_match_goldens() {
    let root = workspace_root();
    let yaml_path = root.join("fixtures/upstream/fixtures/big.yaml");
    let golden_root = root.join("fixtures/m6/reference_outputs/big");
    let raw_root = raw_output_root(&root);
    let upstream_root = std::env::var("UPSTREAM_BASELINE_DIR")
        .ok()
        .map(PathBuf::from);
    let update = std::env::var("UPDATE_GOLDENS").is_ok();
    let check_raw = !update && raw_root.exists();
    let check_upstream = !update && upstream_root.as_ref().is_some_and(|p| p.exists());

    let yaml = std::fs::read_to_string(&yaml_path).unwrap();
    let prepared = PreparedConfig::from_yaml_str(&yaml).unwrap();

    if let Some(Value::Map(outlines)) = prepared.canonical.get_path("outlines") {
        let opts = fixture_dxf_opts();
        for (name, _) in outlines {
            if name.starts_with('_') {
                continue;
            }
            let region = generate_outline_region(&prepared, name).unwrap();
            let dxf = dxf_from_region(&region).unwrap();
            let normalized = dxf.normalize(opts).unwrap();
            let dxf_str = normalized.to_dxf_string(opts).unwrap();
            let out_path = temp_out_dir().join(format!("{name}.dxf"));
            std::fs::write(&out_path, &dxf_str).unwrap();
            let golden = golden_root.join("outlines").join(format!("{name}.dxf"));

            if update {
                std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
                std::fs::write(&golden, &dxf_str).unwrap();
            } else {
                compare_files_semantic(&out_path, &golden, opts).unwrap();
            }

            if check_raw {
                let raw = raw_root.join("outlines").join(format!("{name}.dxf"));
                if raw.exists() {
                    compare_files_semantic(&raw, &golden, opts).unwrap();
                }
            }
            if check_upstream {
                if let Some(upstream_root) = upstream_root.as_ref() {
                    let upstream = upstream_root.join("outlines").join(format!("{name}.dxf"));
                    if upstream.exists() {
                        compare_files_semantic(&upstream, &golden, opts).unwrap();
                    }
                }
            }

            if let Ok(svg) = svg_from_dxf(&dxf) {
                let svg_path = golden_root.join("outlines").join(format!("{name}.svg"));
                compare_or_update(&svg_path, &svg, update);
                if check_raw {
                    let raw = raw_root.join("outlines").join(format!("{name}.svg"));
                    if raw.exists() {
                        compare_raw_text(&svg_path, &raw);
                    }
                }
                if check_upstream {
                    if let Some(upstream_root) = upstream_root.as_ref() {
                        let upstream = upstream_root.join("outlines").join(format!("{name}.svg"));
                        if upstream.exists() {
                            compare_raw_text(&svg_path, &upstream);
                        }
                    }
                }
            }
        }
    }

    if let Some(Value::Map(cases)) = prepared.canonical.get_path("cases") {
        for (name, _) in cases {
            if name.starts_with('_') {
                continue;
            }
            let jscad = generate_cases_jscad_v2(&prepared, name).unwrap();
            let out_path = golden_root.join("cases").join(format!("{name}.jscad"));
            compare_or_update(&out_path, &jscad, update);
            if check_raw {
                let raw = raw_root.join("cases").join(format!("{name}.jscad"));
                if raw.exists() {
                    compare_raw_text(&out_path, &raw);
                }
            }
            if check_upstream {
                if let Some(upstream_root) = upstream_root.as_ref() {
                    let upstream = upstream_root.join("cases").join(format!("{name}.jscad"));
                    if upstream.exists() {
                        compare_raw_text(&out_path, &upstream);
                    }
                }
            }
        }
    }

    if let Some(Value::Map(pcbs)) = prepared.canonical.get_path("pcbs") {
        for (name, _) in pcbs {
            if name.starts_with('_') {
                continue;
            }
            let pcb = generate_kicad_pcb(&prepared, name).unwrap();
            let out_path = golden_root.join("pcbs").join(format!("{name}.kicad_pcb"));
            compare_or_update(&out_path, &pcb, update);
            if check_raw {
                let raw = raw_root.join("pcbs").join(format!("{name}.kicad_pcb"));
                if raw.exists() {
                    compare_raw_text(&out_path, &raw);
                }
            }
            if check_upstream {
                if let Some(upstream_root) = upstream_root.as_ref() {
                    let upstream = upstream_root.join("pcbs").join(format!("{name}.kicad_pcb"));
                    if upstream.exists() {
                        compare_raw_text(&out_path, &upstream);
                    }
                }
            }
        }
    }
}
