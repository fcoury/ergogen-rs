use std::path::{Path, PathBuf};

use ergogen_export::dxf::{NormalizeOptions, compare_files_semantic};
use ergogen_export::dxf_geom::dxf_from_region;
use ergogen_outline::generate_outline_region_from_yaml_str;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn outlines_fixtures_dir() -> PathBuf {
    workspace_root().join("fixtures/m5/outlines")
}

fn fixture_dxf_opts() -> NormalizeOptions {
    NormalizeOptions {
        linear_eps: 1e-3,
        angle_eps_deg: 5e-3,
        ..NormalizeOptions::default()
    }
}

fn generate_outline_dxf_from_yaml(
    yaml_path: &Path,
    out_dxf_path: &Path,
    golden_name: &str,
) -> std::io::Result<()> {
    let yaml = std::fs::read_to_string(yaml_path)?;
    let outline_name = golden_name
        .split_once("__outlines_")
        .and_then(|(_, rest)| rest.strip_suffix("_dxf.dxf"))
        .unwrap_or("outline");
    let region = generate_outline_region_from_yaml_str(&yaml, outline_name)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let dxf = dxf_from_region(&region)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let opts = fixture_dxf_opts();
    let normalized = dxf
        .normalize(opts)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    let out_str = normalized
        .to_dxf_string(opts)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    std::fs::write(out_dxf_path, out_str)?;
    Ok(())
}

#[test]
fn generated_outline_dxfs_match_upstream_goldens_semantically() {
    let dir = outlines_fixtures_dir();
    let out_dir = std::env::temp_dir().join("ergogen-export-outlines-generated");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut yamls: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    yamls.sort();

    for yaml_path in yamls {
        let base = yaml_path.file_stem().unwrap().to_string_lossy().to_string();

        // Each fixture may have multiple outline golden variants.
        let mut goldens: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                    n.starts_with(&format!("{base}___outlines_")) && n.ends_with("_dxf.dxf")
                })
            })
            .collect();
        goldens.sort();

        if goldens.is_empty() {
            continue;
        }

        for golden in goldens {
            let golden_name = golden.file_name().unwrap().to_string_lossy().to_string();
            let out_path = out_dir.join(golden_name.replace("___outlines_", "___generated_"));

            generate_outline_dxf_from_yaml(&yaml_path, &out_path, &golden_name).unwrap();
            compare_files_semantic(&out_path, &golden, fixture_dxf_opts()).unwrap();
        }
    }
}
