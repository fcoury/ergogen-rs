use std::path::PathBuf;

use ergogen_export::dxf::NormalizeOptions;
use ergogen_export::dxf_geom::dxf_from_region;
use ergogen_export::jscad::generate_cases_jscad_v2;
use ergogen_export::svg::svg_from_dxf;
use ergogen_outline::generate_outline_region;
use ergogen_parser::{PreparedConfig, Value};
use ergogen_pcb::generate_kicad_pcb;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
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

fn main() {
    let mut args = std::env::args().skip(1);
    let yaml_path = args
        .next()
        .expect("usage: reference_outputs <yaml> [out-dir]");
    let out_dir = if let Some(dir) = args.next() {
        PathBuf::from(dir)
    } else {
        let stem = PathBuf::from(&yaml_path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        workspace_root().join("target/reference-outputs").join(stem)
    };

    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).expect("clean output dir");
    }
    std::fs::create_dir_all(&out_dir).expect("create output dir");
    let outlines_dir = out_dir.join("outlines");
    let cases_dir = out_dir.join("cases");
    let pcbs_dir = out_dir.join("pcbs");
    std::fs::create_dir_all(&outlines_dir).expect("create outlines dir");
    std::fs::create_dir_all(&cases_dir).expect("create cases dir");
    std::fs::create_dir_all(&pcbs_dir).expect("create pcbs dir");
    let yaml = std::fs::read_to_string(&yaml_path).expect("read yaml");
    let prepared = PreparedConfig::from_yaml_str(&yaml).expect("prepare config");

    if let Some(Value::Map(outlines)) = prepared.canonical.get_path("outlines") {
        let opts = fixture_dxf_opts();
        for (name, _) in outlines {
            if name.starts_with('_') {
                continue;
            }
            let region = generate_outline_region(&prepared, name).expect("outline region");
            let dxf = dxf_from_region(&region).expect("outline dxf");
            let normalized = dxf.normalize(opts).expect("normalize dxf");
            let dxf_str = normalized.to_dxf_string(opts).expect("write dxf");
            let dxf_path = outlines_dir.join(format!("{name}.dxf"));
            std::fs::write(&dxf_path, dxf_str).expect("write dxf file");

            if let Ok(svg) = svg_from_dxf(&dxf) {
                let svg_path = outlines_dir.join(format!("{name}.svg"));
                std::fs::write(&svg_path, svg).expect("write svg file");
            }
        }
    }

    if let Some(Value::Map(cases)) = prepared.canonical.get_path("cases") {
        for (name, _) in cases {
            if name.starts_with('_') {
                continue;
            }
            let jscad = generate_cases_jscad_v2(&prepared, name).expect("generate jscad");
            let out_path = cases_dir.join(format!("{name}.jscad"));
            std::fs::write(&out_path, jscad).expect("write jscad file");
        }
    }

    if let Some(Value::Map(pcbs)) = prepared.canonical.get_path("pcbs") {
        for (name, _) in pcbs {
            if name.starts_with('_') {
                continue;
            }
            let pcb = generate_kicad_pcb(&prepared, name).expect("generate pcb");
            let out_path = pcbs_dir.join(format!("{name}.kicad_pcb"));
            std::fs::write(&out_path, pcb).expect("write pcb file");
        }
    }

    println!("Wrote outputs to {}", out_dir.display());
}
