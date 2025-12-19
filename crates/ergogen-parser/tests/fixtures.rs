use std::path::PathBuf;

use ergogen_parser::PreparedConfig;

#[test]
fn parses_fixture_minimal_yaml() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    let yaml = std::fs::read_to_string(workspace_root.join("fixtures/m1/minimal.yaml")).unwrap();

    let prepared = PreparedConfig::from_yaml_str(&yaml).unwrap();
    assert_eq!(prepared.units.get("foo"), Some(22.0));
}
