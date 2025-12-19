use std::path::{Path, PathBuf};

use ergogen_export::dxf::{NormalizeOptions, compare_files_semantic};

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

fn generate_outline_dxf_from_yaml(
    _yaml_path: &Path,
    _out_dxf_path: &Path,
    _golden_name: &str,
) -> std::io::Result<()> {
    // Intentionally left unimplemented until M5/M6:
    // - M5: compile outline config to geometry IR
    // - M6: write geometry IR as DXF
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "outline DXF generation not implemented yet (requires M5+M6)",
    ))
}

#[test]
#[ignore = "requires outline generator + DXF writer (M5/M6)"]
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
            compare_files_semantic(&out_path, &golden, NormalizeOptions::default()).unwrap();
        }
    }
}
