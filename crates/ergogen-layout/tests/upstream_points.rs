use ergogen_layout::{LayoutError, parse_points};
use ergogen_parser::PreparedConfig;

#[derive(Debug, serde::Deserialize)]
struct GoldenPoint {
    x: f64,
    y: f64,
    r: f64,
}

fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .join("fixtures/m3/points")
}

fn find_golden_points_json(dir: &std::path::Path, base: &str) -> Option<std::path::PathBuf> {
    let triple = dir.join(format!("{base}___points.json"));
    if triple.exists() {
        return Some(triple);
    }

    // Upstream has at least one golden with a double-underscore separator.
    let double = dir.join(format!("{base}__points.json"));
    if double.exists() {
        return Some(double);
    }

    None
}

fn assert_close(got: f64, expected: f64, eps: f64, fixture: &str, point: &str, field: &str) {
    assert!(
        (got - expected).abs() <= eps,
        "fixture={fixture} point={point} field={field} got={got} expected={expected}"
    );
}

#[test]
fn upstream_points_with_golden_points_match() {
    let dir = fixtures_dir();
    let mut yamls: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    yamls.sort();

    for yaml_path in yamls {
        let base = yaml_path.file_stem().unwrap().to_string_lossy().to_string();
        let exception_txt = dir.join(format!("{base}___EXCEPTION.txt"));

        let yaml = std::fs::read_to_string(&yaml_path).unwrap();

        if exception_txt.exists() {
            let expected_snippet = std::fs::read_to_string(exception_txt).unwrap();
            let prepared = PreparedConfig::from_yaml_str(&yaml).unwrap();
            let err = parse_points(&prepared.canonical, &prepared.units).unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains(expected_snippet.trim()),
                "fixture={base} expected snippet {:?} in error {msg:?}",
                expected_snippet.trim()
            );
            continue;
        }

        let Some(points_json) = find_golden_points_json(&dir, &base) else {
            continue;
        };

        let golden_map: std::collections::BTreeMap<String, GoldenPoint> =
            serde_json::from_str(&std::fs::read_to_string(points_json).unwrap()).unwrap();

        let prepared = PreparedConfig::from_yaml_str(&yaml).unwrap();
        let points = parse_points(&prepared.canonical, &prepared.units).unwrap();

        assert_eq!(points.len(), golden_map.len(), "fixture={base}");
        for (pname, g) in golden_map {
            let p = points
                .get(&pname)
                .unwrap_or_else(|| panic!("missing point {pname} (fixture={base})"));
            assert_close(p.x, g.x, 1e-6, &base, &pname, "x");
            assert_close(p.y, g.y, 1e-6, &base, &pname, "y");
            assert_close(p.r, g.r, 1e-6, &base, &pname, "r");
        }
    }
}

#[test]
fn samename_errors_like_upstream() {
    // Ensure our error is stable and helpful (upstream checks for a substring).
    let dir = fixtures_dir();
    let yaml = std::fs::read_to_string(dir.join("samename.yaml")).unwrap();
    let prepared = PreparedConfig::from_yaml_str(&yaml).unwrap();
    let err = parse_points(&prepared.canonical, &prepared.units).unwrap_err();

    match err {
        LayoutError::DuplicateKey { .. } => {}
        other => panic!("expected DuplicateKey, got {other:?}"),
    }
}
