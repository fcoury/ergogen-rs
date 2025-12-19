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

#[test]
fn injected_footprint_matches_golden_kicad_pcb() {
    let root = workspace_root();
    let yaml_path = root.join("fixtures/m6/pcbs/injected.yaml");
    let expected_path = root.join("fixtures/m6/pcbs/injected___pcbs_pcb.kicad_pcb");
    let yaml = std::fs::read_to_string(&yaml_path).unwrap();
    let got = generate_kicad_pcb_from_yaml_str(&yaml, "pcb").unwrap();

    if std::env::var("UPDATE_GOLDENS").is_ok() {
        std::fs::write(&expected_path, &got).unwrap();
        return;
    }

    let expected = std::fs::read_to_string(&expected_path).unwrap();
    let norm = |s: &str| {
        s.replace("\r\n", "\n")
            .trim_end_matches('\n')
            .to_string()
    };
    assert_eq!(norm(&got), norm(&expected));
}
