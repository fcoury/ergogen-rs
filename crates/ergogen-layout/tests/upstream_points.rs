use ergogen_layout::{LayoutError, parse_points};
use ergogen_parser::PreparedConfig;

#[derive(Debug, serde::Deserialize)]
struct GoldenPoint {
    x: f64,
    y: f64,
    r: f64,
    #[serde(default)]
    meta: Option<GoldenMeta>,
}

#[derive(Debug, serde::Deserialize)]
struct GoldenMeta {
    #[serde(default)]
    mirrored: Option<bool>,
    #[serde(default)]
    colrow: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    row: Option<String>,
    #[serde(default)]
    asym: Option<String>,
    #[serde(default)]
    bind: Option<Vec<f64>>,
    #[serde(default)]
    zone: Option<GoldenZone>,
    #[serde(default)]
    col: Option<GoldenCol>,
    #[serde(default)]
    skip: Option<bool>,
    #[serde(default)]
    width: Option<f64>,
    #[serde(default)]
    height: Option<f64>,
    #[serde(default)]
    padding: Option<f64>,
    #[serde(default)]
    autobind: Option<f64>,
}

#[derive(Debug, serde::Deserialize)]
struct GoldenZone {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct GoldenCol {
    #[serde(default)]
    name: Option<String>,
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

fn asym_to_str(a: &ergogen_layout::PlacedPoint) -> String {
    format!("{:?}", a.meta.asym).to_lowercase()
}

fn assert_meta_eq(
    fixture: &str,
    point: &str,
    field: &str,
    got: impl std::fmt::Debug,
    expected: impl std::fmt::Debug,
    equal: bool,
) {
    assert!(
        equal,
        "fixture={fixture} point={point} field={field} got={got:?} expected={expected:?}"
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

            if let Some(meta) = g.meta {
                if let Some(expected) = meta.mirrored {
                    let got = p.meta.mirrored.unwrap_or(false);
                    assert_meta_eq(&base, &pname, "mirrored", got, expected, got == expected);
                }
                if let Some(expected) = meta.colrow {
                    assert_meta_eq(
                        &base,
                        &pname,
                        "colrow",
                        &p.meta.colrow,
                        &expected,
                        p.meta.colrow == expected,
                    );
                }
                if let Some(expected) = meta.name {
                    assert_meta_eq(
                        &base,
                        &pname,
                        "name",
                        &p.meta.name,
                        &expected,
                        p.meta.name == expected,
                    );
                }
                if let Some(expected) = meta.row {
                    assert_meta_eq(
                        &base,
                        &pname,
                        "row",
                        &p.meta.row,
                        &expected,
                        p.meta.row == expected,
                    );
                }
                if let Some(expected) = meta.asym {
                    let got = asym_to_str(p);
                    assert_meta_eq(&base, &pname, "asym", &got, &expected, got == expected);
                }
                if let Some(expected) = meta.bind {
                    assert_eq!(
                        expected.len(),
                        4,
                        "fixture={base} point={pname} field=bind expected length 4"
                    );
                    for (idx, expected_v) in expected.iter().enumerate() {
                        assert_close(
                            p.meta.bind[idx],
                            *expected_v,
                            1e-6,
                            &base,
                            &pname,
                            &format!("bind[{idx}]"),
                        );
                    }
                }
                if let Some(zone) = meta.zone {
                    if let Some(expected) = zone.name {
                        assert_meta_eq(
                            &base,
                            &pname,
                            "zone.name",
                            &p.meta.zone.name,
                            &expected,
                            p.meta.zone.name == expected,
                        );
                    }
                }
                if let Some(col) = meta.col {
                    if let Some(expected) = col.name {
                        assert_meta_eq(
                            &base,
                            &pname,
                            "col.name",
                            &p.meta.col.name,
                            &expected,
                            p.meta.col.name == expected,
                        );
                    }
                }
                if let Some(expected) = meta.skip {
                    assert_meta_eq(
                        &base,
                        &pname,
                        "skip",
                        p.meta.skip,
                        expected,
                        p.meta.skip == expected,
                    );
                }
                if let Some(expected) = meta.width {
                    assert_close(p.meta.width, expected, 1e-6, &base, &pname, "width");
                }
                if let Some(expected) = meta.height {
                    assert_close(p.meta.height, expected, 1e-6, &base, &pname, "height");
                }
                if let Some(expected) = meta.padding {
                    assert_close(p.meta.padding, expected, 1e-6, &base, &pname, "padding");
                }
                if let Some(expected) = meta.autobind {
                    assert_close(p.meta.autobind, expected, 1e-6, &base, &pname, "autobind");
                }
            }
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
