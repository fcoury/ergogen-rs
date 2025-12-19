use std::path::PathBuf;

use ergogen_parser::PreparedIr;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn assert_canonical_fixture(name: &str) {
    let root = workspace_root();
    let yaml_path = root.join(format!("fixtures/m2/upstream/{name}.yaml"));
    let json_path = root.join(format!("fixtures/m2/upstream/{name}.canonical.json"));

    let yaml = std::fs::read_to_string(&yaml_path).unwrap();
    let expected = std::fs::read_to_string(&json_path).unwrap();

    let ir = PreparedIr::from_yaml_str(&yaml).unwrap();
    let got = ir.canonical.to_json_compact_string();

    assert_eq!(got, expected.trim_end(), "fixture={name}");
}

#[test]
fn canonical_matches_snapshots_for_upstream_points_fixtures() {
    for name in [
        "points_default",
        "points_units",
        "points_overrides",
        "points_mirrors",
        "points_rotations",
    ] {
        assert_canonical_fixture(name);
    }
}
