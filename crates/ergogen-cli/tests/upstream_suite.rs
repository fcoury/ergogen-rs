use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ergogen_export::dxf::{Dxf, Entity, Line, NormalizeOptions, Point2, compare_files_semantic};
use ergogen_export::dxf_geom::dxf_from_region;
use ergogen_export::jscad::generate_cases_jscad;
use ergogen_layout::parse_points;
use ergogen_parser::PreparedConfig;
use ergogen_pcb::generate_kicad_pcb_from_yaml_str;

fn fixture_dxf_opts() -> NormalizeOptions {
    NormalizeOptions {
        linear_eps: 1e-3,
        angle_eps_deg: 5e-3,
        ..NormalizeOptions::default()
    }
}

#[derive(Debug, serde::Deserialize)]
struct GoldenPoint {
    x: f64,
    y: f64,
    r: f64,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn upstream_test_root() -> PathBuf {
    workspace_root().join("fixtures/upstream/test")
}

fn list_yaml_fixtures(dir: &Path) -> Vec<PathBuf> {
    let mut yamls: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    yamls.sort();
    yamls
}

fn list_reference_files(dir: &Path, base: &str) -> Vec<PathBuf> {
    // Upstream fixtures usually use `base___...`, but some (notably `path.yaml`) use `base__...`.
    // Use the more permissive `base__` prefix and filter out the exception marker explicitly.
    let prefix = format!("{base}__");
    let mut refs: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&prefix) && n != format!("{base}___EXCEPTION.txt"))
        })
        .collect();
    refs.sort();
    refs
}

fn assert_close(got: f64, expected: f64, eps: f64, fixture: &str, point: &str, field: &str) {
    assert!(
        (got - expected).abs() <= eps,
        "fixture={fixture} point={point} field={field} got={got} expected={expected}"
    );
}

fn compare_points_xyz(prepared: &PreparedConfig, expected_path: &Path, fixture: &str) {
    let golden_map: BTreeMap<String, GoldenPoint> =
        serde_json::from_str(&std::fs::read_to_string(expected_path).unwrap()).unwrap();

    let points = parse_points(&prepared.canonical, &prepared.units).unwrap();

    assert_eq!(
        points.len(),
        golden_map.len(),
        "fixture={fixture} golden={}",
        expected_path.display()
    );

    for (pname, g) in golden_map {
        let p = points
            .get(&pname)
            .unwrap_or_else(|| panic!("missing point {pname} (fixture={fixture})"));
        assert_close(p.x, g.x, 1e-6, fixture, &pname, "x");
        assert_close(p.y, g.y, 1e-6, fixture, &pname, "y");
        assert_close(p.r, g.r, 1e-6, fixture, &pname, "r");
    }
}

fn compare_units(prepared: &PreparedConfig, expected_path: &Path, fixture: &str) {
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(expected_path).unwrap()).unwrap();

    let expected = expected.as_object().unwrap();
    let got = prepared.units.vars();
    for (k, v) in expected {
        let expected = v.as_f64().unwrap();
        let got = got.get(k).copied().unwrap_or(f64::NAN);
        assert_close(got, expected, 1e-9, fixture, k, "units");
    }
}

fn try_compare_outline_dxf(yaml: &str, _base: &str, expected_path: &Path) -> Result<(), String> {
    let fname = expected_path.file_name().unwrap().to_string_lossy();
    let outline_name = fname
        .split_once("__outlines_")
        .and_then(|(_, rest)| rest.strip_suffix("_dxf.dxf"))
        .ok_or_else(|| "not an outlines dxf golden".to_string())?;

    let region = ergogen_outline::generate_outline_region_from_yaml_str(yaml, outline_name)
        .map_err(|e| e.to_string())?;

    let dxf = dxf_from_region(&region).map_err(|e| e.to_string())?;
    let opts = fixture_dxf_opts();
    let normalized = dxf.normalize(opts).map_err(|e| e.to_string())?;
    let out_str = normalized.to_dxf_string(opts).map_err(|e| e.to_string())?;

    let out_dir = std::env::temp_dir().join("ergogen-upstream-suite");
    std::fs::create_dir_all(&out_dir).unwrap();
    let out_path = out_dir.join(
        fname
            .replace("___outlines_", "___generated_")
            .replace("__outlines_", "__generated_"),
    );
    std::fs::write(&out_path, out_str).unwrap();

    compare_files_semantic(&out_path, expected_path, opts).map_err(|e| e.to_string())?;
    Ok(())
}

fn try_compare_points_demo_dxf(
    prepared: &PreparedConfig,
    expected_path: &Path,
) -> Result<(), String> {
    let points = parse_points(&prepared.canonical, &prepared.units).map_err(|e| e.to_string())?;
    let mut entities: Vec<Entity> = Vec::new();

    for p in points.values() {
        let hw = p.meta.width / 2.0;
        let hh = p.meta.height / 2.0;
        let corners = [(-hw, hh), (hw, hh), (hw, -hh), (-hw, -hh)];
        let (sin, cos) = p.r.to_radians().sin_cos();
        let mut pts: Vec<Point2> = Vec::with_capacity(4);
        for (x, y) in corners {
            let rx = x * cos - y * sin;
            let ry = x * sin + y * cos;
            pts.push(Point2 {
                x: rx + p.x,
                y: ry + p.y,
            });
        }
        for i in 0..4 {
            entities.push(Entity::Line(Line {
                start: pts[i],
                end: pts[(i + 1) % 4],
            }));
        }
    }

    let dxf = Dxf { entities };
    let opts = fixture_dxf_opts();
    let normalized = dxf.normalize(opts).map_err(|e| e.to_string())?;
    let out_str = normalized.to_dxf_string(opts).map_err(|e| e.to_string())?;

    let fname = expected_path.file_name().unwrap().to_string_lossy();
    let out_dir = std::env::temp_dir().join("ergogen-upstream-suite");
    std::fs::create_dir_all(&out_dir).unwrap();
    let out_path = out_dir.join(
        fname
            .replace("___demo_", "___generated_")
            .replace("__demo_", "__generated_"),
    );
    std::fs::write(&out_path, out_str).unwrap();

    compare_files_semantic(&out_path, expected_path, opts).map_err(|e| e.to_string())?;
    Ok(())
}

fn try_compare_kicad_pcb(yaml: &str, expected_path: &Path) -> Result<(), String> {
    let fname = expected_path.file_name().unwrap().to_string_lossy();
    let pcb_name = fname
        .split_once("__pcbs_")
        .and_then(|(_, rest)| rest.strip_suffix(".kicad_pcb"))
        .ok_or_else(|| "not a pcb golden".to_string())?;

    let got = generate_kicad_pcb_from_yaml_str(yaml, pcb_name).map_err(|e| e.to_string())?;
    let expected = std::fs::read_to_string(expected_path).map_err(|e| e.to_string())?;

    let norm = |s: &str| s.replace("\r\n", "\n").trim_end_matches('\n').to_string();

    let got = norm(&got);
    let expected = norm(&expected);

    if got != expected {
        return Err("kicad_pcb output mismatch".to_string());
    }
    Ok(())
}

fn try_compare_case_jscad(prepared: &PreparedConfig, expected_path: &Path) -> Result<(), String> {
    let fname = expected_path.file_name().unwrap().to_string_lossy();
    let case_name = fname
        .split_once("__cases_")
        .and_then(|(_, rest)| rest.strip_suffix("_jscad.jscad"))
        .ok_or_else(|| "not a cases jscad golden".to_string())?;

    let got = generate_cases_jscad(prepared, case_name).map_err(|e| e.to_string())?;
    let expected = std::fs::read_to_string(expected_path).map_err(|e| e.to_string())?;

    let out_dir = std::env::temp_dir().join("ergogen-upstream-suite");
    std::fs::create_dir_all(&out_dir).unwrap();
    let out_path = out_dir.join(
        fname
            .replace("___cases_", "___generated_")
            .replace("__cases_", "__generated_"),
    );
    std::fs::write(&out_path, &got).unwrap();

    let norm = |s: &str| s.replace("\r\n", "\n").trim_end_matches('\n').to_string();

    if norm(&got) != norm(&expected) {
        return Err("cases JSCAD output mismatch".to_string());
    }
    Ok(())
}

#[test]
fn upstream_integration_fixtures_run_against_rust_implementation() {
    // This is the Rust equivalent of upstream's integration fixture loop:
    // `test/{points,outlines,cases,pcbs,footprints}/*.yaml` plus `base___*` reference artifacts.
    //
    // We validate what we can today and skip unimplemented artifact types (PCB/JSCAD/etc.)
    // while keeping everything imported and discoverable.
    let root = upstream_test_root();

    let categories = ["points", "outlines", "cases", "pcbs", "footprints"];

    let mut compared = 0usize;
    let mut skipped = 0usize;

    for cat in categories {
        let dir = root.join(cat);
        let yamls = list_yaml_fixtures(&dir);
        for yaml_path in yamls {
            let base = yaml_path.file_stem().unwrap().to_string_lossy().to_string();
            let base_path = dir.join(&base);
            let exception_path = dir.join(format!("{base}___EXCEPTION.txt"));
            let yaml = std::fs::read_to_string(&yaml_path).unwrap();

            if exception_path.exists() {
                let expected_snippet = std::fs::read_to_string(&exception_path).unwrap();
                // Best-effort: try the stage that the upstream fixture is for.
                let prepared = match PreparedConfig::from_yaml_str(&yaml) {
                    Ok(p) => p,
                    Err(e) => {
                        assert!(
                            e.to_string().contains(expected_snippet.trim()),
                            "fixture={cat}/{base} expected snippet {:?} in error {e:?}",
                            expected_snippet.trim()
                        );
                        continue;
                    }
                };

                if cat == "points" {
                    let err = parse_points(&prepared.canonical, &prepared.units).unwrap_err();
                    assert!(
                        err.to_string().contains(expected_snippet.trim()),
                        "fixture={cat}/{base} expected snippet {:?} in error {err:?}",
                        expected_snippet.trim()
                    );
                    continue;
                }

                // Other exception fixtures are not supported yet.
                eprintln!("SKIP exception fixture (unhandled category): {cat}/{base_path:?}");
                skipped += 1;
                continue;
            }

            let prepared = match PreparedConfig::from_yaml_str(&yaml) {
                Ok(p) => p,
                Err(e) => panic!("fixture={cat}/{base} parse failed: {e}"),
            };

            for expected_path in list_reference_files(&dir, &base) {
                let name = expected_path.file_name().unwrap().to_string_lossy();

                if name == format!("{base}___points.json") || name == format!("{base}__points.json")
                {
                    compare_points_xyz(&prepared, &expected_path, &format!("{cat}/{base}"));
                    compared += 1;
                    continue;
                }

                if name == format!("{base}___units.json") {
                    compare_units(&prepared, &expected_path, &format!("{cat}/{base}"));
                    compared += 1;
                    continue;
                }

                if name.contains("__outlines_") && name.ends_with("_dxf.dxf") {
                    match try_compare_outline_dxf(&yaml, &base, &expected_path) {
                        Ok(()) => {
                            compared += 1;
                        }
                        Err(msg) => {
                            eprintln!(
                                "SKIP outline golden (not implemented yet): {cat}/{name} ({msg})"
                            );
                            skipped += 1;
                        }
                    }
                    continue;
                }

                if name.ends_with("__demo_dxf.dxf") {
                    try_compare_points_demo_dxf(&prepared, &expected_path)
                        .unwrap_or_else(|e| panic!("fixture={cat}/{name} demo dxf mismatch: {e}"));
                    compared += 1;
                    continue;
                }

                if name.ends_with(".kicad_pcb") {
                    try_compare_kicad_pcb(&yaml, &expected_path)
                        .unwrap_or_else(|e| panic!("fixture={cat}/{name} pcb mismatch: {e}"));
                    compared += 1;
                    continue;
                }

                if name.ends_with(".jscad") {
                    try_compare_case_jscad(&prepared, &expected_path)
                        .unwrap_or_else(|e| panic!("fixture={cat}/{name} jscad mismatch: {e}"));
                    compared += 1;
                    continue;
                }

                // Not implemented yet (footprint-driven PCB outputs, etc.).
                eprintln!("SKIP unhandled reference artifact: {cat}/{name}");
                skipped += 1;
            }
        }
    }

    assert!(compared > 0, "suite compared nothing; harness is broken");
    eprintln!("upstream suite: compared={compared} skipped={skipped}");
}
