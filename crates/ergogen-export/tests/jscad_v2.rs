use ergogen_export::jscad::generate_cases_jscad_v2;
use ergogen_parser::PreparedConfig;

#[test]
fn jscad_v2_uses_modern_api_for_jscad_app() {
    let yaml_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/upstream/test/cases/cube.yaml");
    let yaml = std::fs::read_to_string(yaml_path).unwrap();
    let prepared = PreparedConfig::from_yaml_str(&yaml).unwrap();

    let jscad = generate_cases_jscad_v2(&prepared, "cube").unwrap();
    assert!(jscad.contains("require('@jscad/modeling')"));
    assert!(jscad.contains("module.exports = { main };"));
    assert!(!jscad.contains("CSG"));
}
