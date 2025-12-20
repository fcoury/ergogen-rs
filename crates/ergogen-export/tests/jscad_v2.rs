use ergogen_export::jscad::generate_cases_jscad_v2;
use ergogen_parser::PreparedConfig;

fn load_prepared(rel_path_from_workspace_root: &str) -> PreparedConfig {
    let yaml_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(rel_path_from_workspace_root);
    let yaml = std::fs::read_to_string(yaml_path).unwrap();
    PreparedConfig::from_yaml_str(&yaml).unwrap()
}

fn assert_is_v2(jscad: &str) {
    assert!(jscad.contains("require('@jscad/modeling')"));
    assert!(jscad.contains("module.exports = { main };"));
    assert!(!jscad.contains("CSG"));
}

#[test]
fn jscad_v2_uses_modern_api_for_jscad_app() {
    let prepared = load_prepared("fixtures/upstream/test/cases/cube.yaml");

    let jscad = generate_cases_jscad_v2(&prepared, "cube").unwrap();
    assert_is_v2(&jscad);
}

#[test]
fn jscad_v2_supports_boolean_operations_fixture() {
    let prepared = load_prepared("fixtures/upstream/test/cases/operations.yaml");
    let jscad = generate_cases_jscad_v2(&prepared, "combination").unwrap();
    assert_is_v2(&jscad);
}

#[test]
fn jscad_v2_supports_knuckles_case() {
    let prepared = load_prepared("knuckles/ergogen/config.yaml");
    let jscad = generate_cases_jscad_v2(&prepared, "knuckles_bottom_tray").unwrap();
    assert_is_v2(&jscad);
}
