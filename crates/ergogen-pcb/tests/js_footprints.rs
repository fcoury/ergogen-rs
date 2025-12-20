#![cfg(feature = "js-footprints")]

use std::path::PathBuf;

use ergogen_pcb::generate_kicad_pcb_from_yaml_str;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn fixture_dir() -> PathBuf {
    workspace_root().join("fixtures/m7/js_footprints")
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end_matches('\n').to_string()
}

#[test]
fn js_footprint_fixtures_match_golden_kicad_pcbs() {
    let dir = fixture_dir();
    let mut yamls: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map(|ext| ext == "yaml").unwrap_or(false))
        .collect();
    yamls.sort();

    assert!(!yamls.is_empty(), "no js pcb fixtures found in {dir:?}");

    let update = std::env::var("UPDATE_GOLDENS").is_ok();

    for yaml_path in yamls {
        let stem = yaml_path
            .file_stem()
            .expect("fixture name")
            .to_string_lossy()
            .to_string();
        let expected_path = dir.join(format!("{stem}___pcbs_pcb.kicad_pcb"));
        let yaml = std::fs::read_to_string(&yaml_path).unwrap();
        let got = generate_kicad_pcb_from_yaml_str(&yaml, "pcb").unwrap();

        if update {
            std::fs::write(&expected_path, &got).unwrap();
            continue;
        }

        let expected = std::fs::read_to_string(&expected_path).unwrap();
        assert_eq!(
            normalize(&got),
            normalize(&expected),
            "fixture {stem} mismatch"
        );
    }
}
