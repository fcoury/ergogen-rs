use ergogen_parser::PreparedConfig;

#[test]
fn units_fixture_matches_upstream_snapshot() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf();
    let dir = root.join("fixtures/m3/points");

    let yaml = std::fs::read_to_string(dir.join("units.yaml")).unwrap();
    let expected: std::collections::BTreeMap<String, f64> =
        serde_json::from_str(&std::fs::read_to_string(dir.join("units___units.json")).unwrap())
            .unwrap();

    let prepared = PreparedConfig::from_yaml_str(&yaml).unwrap();
    let got: std::collections::BTreeMap<String, f64> = prepared
        .units
        .snapshot()
        .into_iter()
        .map(|e| (e.name, e.value))
        .collect();

    assert_eq!(got, expected);
}
