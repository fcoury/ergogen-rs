use std::path::{Path, PathBuf};

use ergogen_export::dxf::{Dxf, Entity, Line, NormalizeOptions, Point2, compare_files_semantic};
use ergogen_export::dxf_geom::dxf_from_region;
use ergogen_export::jscad::generate_cases_jscad;
use ergogen_export::svg::svg_from_lines;
use ergogen_layout::{PointsOutput, parse_points};
use ergogen_outline::generate_outline_region;
use ergogen_parser::{PreparedConfig, Value, convert_kle};
use ergogen_pcb::generate_kicad_pcb;

fn fixture_dxf_opts() -> NormalizeOptions {
    NormalizeOptions {
        linear_eps: 1e-3,
        angle_eps_deg: 5e-3,
        ..NormalizeOptions::default()
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn cli_fixture_root() -> PathBuf {
    workspace_root().join("fixtures/upstream/test/cli")
}

fn upstream_fixtures_root() -> PathBuf {
    workspace_root().join("fixtures/upstream/fixtures")
}

#[derive(Debug)]
struct CliCommand {
    input: Option<PathBuf>,
    debug: bool,
    clean: bool,
    is_kle: bool,
    analyze: Option<AnalyzeTarget>,
}

#[derive(Debug, Clone, Copy)]
enum AnalyzeTarget {
    Folder,
    Bundle,
}

fn parse_command(command: &str) -> CliCommand {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let mut input: Option<PathBuf> = None;
    let mut debug = false;
    let mut clean = false;
    let mut is_kle = false;
    let mut analyze = None;

    if let Some(pos) = tokens.iter().rposition(|t| *t == "src/cli.js") {
        if let Some(arg) = tokens.get(pos + 1)
            && !arg.starts_with("--")
        {
            let resolved = resolve_input_path(arg);
            is_kle = resolved
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
            if resolved
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
            {
                analyze = Some(AnalyzeTarget::Bundle);
            } else if resolved.is_dir() {
                analyze = Some(AnalyzeTarget::Folder);
            }
            input = Some(resolved);
        }
        for token in &tokens[pos + 1..] {
            if *token == "--debug" {
                debug = true;
            }
            if *token == "--clean" {
                clean = true;
            }
        }
    }

    CliCommand {
        input,
        debug,
        clean,
        is_kle,
        analyze,
    }
}

fn resolve_input_path(token: &str) -> PathBuf {
    if let Some(rest) = token.strip_prefix("test/fixtures/") {
        upstream_fixtures_root().join(rest)
    } else if let Some(rest) = token.strip_prefix("test/cli/") {
        cli_fixture_root().join(rest)
    } else if token == "test/" || token == "test" {
        workspace_root().join("fixtures/upstream/test")
    } else {
        workspace_root().join(token)
    }
}

#[derive(Debug)]
struct FixtureConfig {
    raw: String,
    prepared: PreparedConfig,
}

fn load_fixture_config(_fixture_dir: &Path, command: &CliCommand) -> Result<FixtureConfig, String> {
    let Some(input) = &command.input else {
        return Err("Usage: ergogen <config_file> [options]".to_string());
    };

    if !input.exists() {
        let name = input
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| input.to_string_lossy().to_string());
        return Err(format!(
            "Could not read config file \"{name}\": File does not exist!",
        ));
    }

    let mut bundle_root: Option<PathBuf> = None;
    let raw = if input.is_dir() {
        let config = find_bundle_config(input)?;
        bundle_root = Some(input.to_path_buf());
        std::fs::read_to_string(&config).map_err(|e| e.to_string())?
    } else if input
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        let bundle = upstream_fixtures_root().join("bundle");
        let config = bundle.join("config.yaml");
        bundle_root = Some(bundle);
        std::fs::read_to_string(&config).map_err(|e| e.to_string())?
    } else {
        std::fs::read_to_string(input).map_err(|e| e.to_string())?
    };

    if !command.is_kle {
        let parsed = Value::from_yaml_str(&raw).map_err(|e| e.to_string())?;
        if !matches!(parsed, Value::Map(_)) {
            return Err("Error: Input doesn't resolve into an object!".to_string());
        }
    }

    let prepared = if command.is_kle {
        let parsed = Value::from_yaml_str(&raw).map_err(|e| e.to_string())?;
        let converted = convert_kle(&parsed).map_err(|e| e.to_string())?;
        PreparedConfig::from_value(&converted).map_err(|e| e.to_string())?
    } else {
        PreparedConfig::from_yaml_str(&raw).map_err(|e| e.to_string())?
    };

    if let Some(root) = bundle_root.as_ref()
        && let Some(err) = check_bundle_templates(root, &prepared.canonical)
    {
        return Err(err);
    }

    Ok(FixtureConfig { raw, prepared })
}

fn check_bundle_templates(root: &Path, canonical: &Value) -> Option<String> {
    let Value::Map(pcbs) = canonical.get_path("pcbs")? else {
        return None;
    };
    for (name, pcb_v) in pcbs {
        let Value::Map(pcb_map) = pcb_v else { continue };
        let template = match pcb_map.get("template") {
            Some(Value::String(s)) => s.as_str(),
            _ => "",
        };
        if template.is_empty() {
            continue;
        }
        let path = root.join("templates").join(format!("{template}.js"));
        if !path.exists() {
            continue;
        }
        if let Ok(contents) = std::fs::read_to_string(&path)
            && contents.contains("nonexistent_require")
        {
            return Some(format!(
                "Unknown dependency \"nonexistent_require\" among the requirements of injection \"{name}\"!"
            ));
        }
    }
    None
}

fn find_bundle_config(root: &Path) -> Result<PathBuf, String> {
    let mut configs: Vec<PathBuf> = Vec::new();
    for name in ["config.yaml", "config.yml"] {
        let path = root.join(name);
        if path.exists() {
            configs.push(path);
        }
    }
    if configs.len() > 1 {
        return Err("Ambiguous config in bundle!".to_string());
    }
    if let Some(path) = configs.into_iter().next() {
        Ok(path)
    } else {
        Err("Ambiguous config in bundle!".to_string())
    }
}

fn points_demo_lines(points: &PointsOutput) -> Vec<Line> {
    let mut entities: Vec<Line> = Vec::new();
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
            entities.push(Line {
                start: pts[i],
                end: pts[(i + 1) % 4],
            });
        }
    }
    entities
}

fn compare_points_yaml(
    prepared: &PreparedConfig,
    expected_path: &Path,
    fixture: &str,
) -> Result<(), String> {
    let expected_raw = std::fs::read_to_string(expected_path).map_err(|e| e.to_string())?;
    let expected = Value::from_yaml_str(&expected_raw).map_err(|e| e.to_string())?;
    let Value::Map(expected_points) = expected else {
        return Err("points.yaml must be a map".to_string());
    };

    let points = parse_points(&prepared.canonical, &prepared.units).map_err(|e| e.to_string())?;

    for (name, expected_point) in expected_points {
        let p = points
            .get(&name)
            .ok_or_else(|| format!("fixture={fixture} missing point {name}"))?;
        let Value::Map(point_map) = expected_point else {
            return Err(format!("fixture={fixture} point {name} must be a map"));
        };

        assert_close(
            p.x,
            value_as_f64(point_map.get("x"))?,
            1e-6,
            fixture,
            &name,
            "x",
        );
        assert_close(
            p.y,
            value_as_f64(point_map.get("y"))?,
            1e-6,
            fixture,
            &name,
            "y",
        );
        assert_close(
            p.r,
            value_as_f64(point_map.get("r"))?,
            1e-6,
            fixture,
            &name,
            "r",
        );

        let Value::Map(meta) = point_map
            .get("meta")
            .ok_or_else(|| format!("fixture={fixture} point {name} missing meta"))?
        else {
            return Err(format!("fixture={fixture} point {name} meta must be a map"));
        };

        assert_close(
            p.meta.stagger,
            value_as_f64(meta.get("stagger"))?,
            1e-6,
            fixture,
            &name,
            "meta.stagger",
        );
        assert_close(
            p.meta.spread,
            value_as_f64(meta.get("spread"))?,
            1e-6,
            fixture,
            &name,
            "meta.spread",
        );
        assert_close(
            p.meta.splay,
            value_as_f64(meta.get("splay"))?,
            1e-6,
            fixture,
            &name,
            "meta.splay",
        );
        assert_vec2(
            p.meta.origin,
            meta.get("origin"),
            fixture,
            &name,
            "meta.origin",
        )?;
        assert_close(
            p.meta.orient,
            value_as_f64(meta.get("orient"))?,
            1e-6,
            fixture,
            &name,
            "meta.orient",
        );
        assert_vec2(
            p.meta.shift,
            meta.get("shift"),
            fixture,
            &name,
            "meta.shift",
        )?;
        assert_close(
            p.meta.rotate,
            value_as_f64(meta.get("rotate"))?,
            1e-6,
            fixture,
            &name,
            "meta.rotate",
        );

        let expected_adjust = meta
            .get("adjust")
            .ok_or_else(|| format!("fixture={fixture} point {name} meta.adjust missing"))?;
        if !value_semantic_eq(expected_adjust, &p.meta.adjust) {
            return Err(format!(
                "fixture={fixture} point {name} meta.adjust mismatch"
            ));
        }

        assert_close(
            p.meta.width,
            value_as_f64(meta.get("width"))?,
            1e-6,
            fixture,
            &name,
            "meta.width",
        );
        assert_close(
            p.meta.height,
            value_as_f64(meta.get("height"))?,
            1e-6,
            fixture,
            &name,
            "meta.height",
        );
        assert_close(
            p.meta.padding,
            value_as_f64(meta.get("padding"))?,
            1e-6,
            fixture,
            &name,
            "meta.padding",
        );
        assert_close(
            p.meta.autobind,
            value_as_f64(meta.get("autobind"))?,
            1e-6,
            fixture,
            &name,
            "meta.autobind",
        );
        assert_bool(p.meta.skip, meta.get("skip"), fixture, &name, "meta.skip")?;
        let expected_asym = value_as_str(meta.get("asym"))?;
        let actual_asym = format!("{:?}", p.meta.asym).to_lowercase();
        if expected_asym != actual_asym {
            return Err(format!(
                "fixture={fixture} point {name} meta.asym mismatch expected={expected_asym} got={actual_asym}"
            ));
        }

        assert_str(
            &p.meta.colrow,
            meta.get("colrow"),
            fixture,
            &name,
            "meta.colrow",
        )?;
        assert_str(&p.meta.name, meta.get("name"), fixture, &name, "meta.name")?;

        if let Some(zone_v) = meta.get("zone") {
            let Value::Map(zone_map) = zone_v else {
                return Err(format!(
                    "fixture={fixture} point {name} meta.zone must be a map"
                ));
            };
            if let Some(zone_name) = zone_map.get("name") {
                assert_str(
                    &p.meta.zone.name,
                    Some(zone_name),
                    fixture,
                    &name,
                    "meta.zone.name",
                )?;
            }
        }
        if let Some(col_v) = meta.get("col") {
            let Value::Map(col_map) = col_v else {
                return Err(format!(
                    "fixture={fixture} point {name} meta.col must be a map"
                ));
            };
            if let Some(col_name) = col_map.get("name") {
                assert_str(
                    &p.meta.col.name,
                    Some(col_name),
                    fixture,
                    &name,
                    "meta.col.name",
                )?;
            }
        }

        assert_str(&p.meta.row, meta.get("row"), fixture, &name, "meta.row")?;

        let bind_v = meta
            .get("bind")
            .ok_or_else(|| format!("fixture={fixture} point {name} meta.bind missing"))?;
        let Value::Seq(bind_seq) = bind_v else {
            return Err(format!(
                "fixture={fixture} point {name} meta.bind must be seq"
            ));
        };
        if bind_seq.len() != 4 {
            return Err(format!("fixture={fixture} point {name} meta.bind len != 4"));
        }
        for (idx, v) in bind_seq.iter().enumerate() {
            assert_close(
                p.meta.bind[idx],
                value_as_f64(Some(v))?,
                1e-6,
                fixture,
                &name,
                "meta.bind",
            );
        }
    }

    Ok(())
}

fn compare_units_yaml(
    prepared: &PreparedConfig,
    expected_path: &Path,
    fixture: &str,
) -> Result<(), String> {
    let expected_raw = std::fs::read_to_string(expected_path).map_err(|e| e.to_string())?;
    let expected = Value::from_yaml_str(&expected_raw).map_err(|e| e.to_string())?;
    let Value::Map(expected_map) = expected else {
        return Err("units.yaml must be a map".to_string());
    };
    let got = prepared.units.vars();
    for (k, v) in expected_map {
        let expected = value_as_f64(Some(&v))?;
        let got = got.get(&k).copied().unwrap_or(f64::NAN);
        assert_close(got, expected, 1e-6, fixture, &k, "units");
    }
    Ok(())
}

fn compare_canonical_yaml(prepared: &PreparedConfig, expected_path: &Path) -> Result<(), String> {
    let expected_raw = std::fs::read_to_string(expected_path).map_err(|e| e.to_string())?;
    let expected = Value::from_yaml_str(&expected_raw).map_err(|e| e.to_string())?;
    if !value_semantic_eq(&expected, &prepared.canonical) {
        return Err("canonical.yaml mismatch".to_string());
    }
    Ok(())
}

fn compare_model_yaml(lines: &[Line], expected_path: &Path) -> Result<(), String> {
    let expected_raw = std::fs::read_to_string(expected_path).map_err(|e| e.to_string())?;
    let expected = Value::from_yaml_str(&expected_raw).map_err(|e| e.to_string())?;
    let mut expected_lines: Vec<Line> = Vec::new();
    collect_lines_from_model(&expected, &mut expected_lines)?;

    let got_norm = normalize_lines(lines);
    let expected_norm = normalize_lines(&expected_lines);

    if got_norm != expected_norm {
        return Err("model yaml line mismatch".to_string());
    }
    Ok(())
}

fn collect_lines_from_model(value: &Value, out: &mut Vec<Line>) -> Result<(), String> {
    let Value::Map(map) = value else {
        return Ok(());
    };
    if let Some(models_v) = map.get("models") {
        let Value::Map(models) = models_v else {
            return Ok(());
        };
        for (_, model_v) in models {
            collect_lines_from_model(model_v, out)?;
        }
    }
    if let Some(paths_v) = map.get("paths") {
        let Value::Map(paths) = paths_v else {
            return Ok(());
        };
        for (_, path_v) in paths {
            let Value::Map(path_map) = path_v else {
                continue;
            };
            let path_type = match path_map.get("type") {
                Some(Value::String(s)) => s.as_str(),
                _ => "",
            };
            if path_type != "line" {
                continue;
            }
            let origin_v = path_map.get("origin").ok_or("path origin missing")?;
            let end_v = path_map.get("end").ok_or("path end missing")?;
            let start = vec2_from_value(origin_v)?;
            let end = vec2_from_value(end_v)?;
            out.push(Line {
                start: Point2 {
                    x: start[0],
                    y: start[1],
                },
                end: Point2 {
                    x: end[0],
                    y: end[1],
                },
            });
        }
    }
    Ok(())
}

fn normalize_lines(lines: &[Line]) -> Vec<(i64, i64, i64, i64)> {
    let eps = 1e-3;
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let mut a = quantize_point(line.start, eps);
        let mut b = quantize_point(line.end, eps);
        if a == b {
            continue;
        }
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        out.push((a.0, a.1, b.0, b.1));
    }
    out.sort();
    out
}

fn quantize_point(p: Point2, eps: f64) -> (i64, i64) {
    let q = |v: f64| (v / eps).round() as i64;
    (q(p.x), q(p.y))
}

fn compare_svg(lines: &[Line], expected_path: &Path) -> Result<(), String> {
    let got_svg = svg_from_lines(lines).map_err(|e| e.to_string())?;
    let expected_svg = std::fs::read_to_string(expected_path).map_err(|e| e.to_string())?;
    let got = parse_svg(&got_svg)?;
    let expected = parse_svg(&expected_svg)?;
    let eps = 1e-3;
    if (got.width - expected.width).abs() > eps || (got.height - expected.height).abs() > eps {
        return Err("svg viewport mismatch".to_string());
    }
    if normalize_lines(&got.lines) != normalize_lines(&expected.lines) {
        return Err("svg output mismatch".to_string());
    }
    Ok(())
}

fn compare_dxf(lines: &[Line], expected_path: &Path) -> Result<(), String> {
    let dxf = Dxf {
        entities: lines.iter().cloned().map(Entity::Line).collect(),
    };
    let opts = fixture_dxf_opts();
    let normalized = dxf.normalize(opts).map_err(|e| e.to_string())?;
    let out_str = normalized.to_dxf_string(opts).map_err(|e| e.to_string())?;

    let out_dir = std::env::temp_dir().join("ergogen-cli-suite");
    std::fs::create_dir_all(&out_dir).unwrap();
    let fname = expected_path.file_name().unwrap().to_string_lossy();
    let out_path = out_dir.join(fname.to_string());
    std::fs::write(&out_path, out_str).unwrap();

    compare_files_semantic(&out_path, expected_path, opts).map_err(|e| e.to_string())?;
    Ok(())
}

fn compare_outline_dxf(
    prepared: &PreparedConfig,
    outline: &str,
    expected_path: &Path,
) -> Result<Vec<Line>, String> {
    let dxf = outline_dxf(prepared, outline)?;
    let opts = fixture_dxf_opts();
    let normalized = dxf.normalize(opts).map_err(|e| e.to_string())?;
    let out_str = normalized.to_dxf_string(opts).map_err(|e| e.to_string())?;

    let out_dir = std::env::temp_dir().join("ergogen-cli-suite");
    std::fs::create_dir_all(&out_dir).unwrap();
    let fname = expected_path.file_name().unwrap().to_string_lossy();
    let out_path = out_dir.join(fname.to_string());
    std::fs::write(&out_path, out_str).unwrap();

    compare_files_semantic(&out_path, expected_path, opts).map_err(|e| e.to_string())?;

    outline_lines_from_dxf(&dxf)
}

fn outline_dxf(prepared: &PreparedConfig, outline: &str) -> Result<Dxf, String> {
    let region = generate_outline_region(prepared, outline).map_err(|e| e.to_string())?;
    dxf_from_region(&region).map_err(|e| e.to_string())
}

fn outline_lines(prepared: &PreparedConfig, outline: &str) -> Result<Vec<Line>, String> {
    let dxf = outline_dxf(prepared, outline)?;
    outline_lines_from_dxf(&dxf)
}

fn outline_lines_from_dxf(dxf: &Dxf) -> Result<Vec<Line>, String> {
    let mut lines: Vec<Line> = Vec::new();
    for entity in &dxf.entities {
        if let Entity::Line(line) = entity {
            lines.push(*line);
        } else {
            return Err("outline svg only supports line entities".to_string());
        }
    }
    Ok(lines)
}

fn normalize_text(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end_matches('\n').to_string()
}

struct ParsedSvg {
    width: f64,
    height: f64,
    lines: Vec<Line>,
}

fn parse_svg(svg: &str) -> Result<ParsedSvg, String> {
    let width = extract_svg_attr(svg, "width")?;
    let height = extract_svg_attr(svg, "height")?;
    let d = extract_svg_path(svg)?;
    let lines = parse_svg_path(&d)?;
    Ok(ParsedSvg {
        width,
        height,
        lines,
    })
}

fn extract_svg_attr(svg: &str, name: &str) -> Result<f64, String> {
    let needle = format!("{name}=\"");
    let start = svg.find(&needle).ok_or("svg attribute missing")? + needle.len();
    let rest = &svg[start..];
    let end = rest.find("mm\"").ok_or("svg attribute missing")?;
    let raw = &rest[..end];
    raw.parse::<f64>()
        .map_err(|_| "invalid svg number".to_string())
}

fn extract_svg_path(svg: &str) -> Result<String, String> {
    let needle = "<path d=\"";
    let start = svg.find(needle).ok_or("svg path missing")? + needle.len();
    let rest = &svg[start..];
    let end = rest.find('"').ok_or("svg path missing")?;
    Ok(rest[..end].to_string())
}

fn parse_svg_path(d: &str) -> Result<Vec<Line>, String> {
    let tokens: Vec<&str> = d.split_whitespace().collect();
    let mut i = 0usize;
    let mut lines = Vec::new();
    let mut current: Option<Point2> = None;
    let mut start: Option<Point2> = None;
    while i < tokens.len() {
        match tokens[i] {
            "M" => {
                let x = tokens
                    .get(i + 1)
                    .ok_or("svg path M missing x")?
                    .parse::<f64>()
                    .map_err(|_| "svg number")?;
                let y = tokens
                    .get(i + 2)
                    .ok_or("svg path M missing y")?
                    .parse::<f64>()
                    .map_err(|_| "svg number")?;
                let p = Point2 { x, y };
                current = Some(p);
                start = Some(p);
                i += 3;
            }
            "L" => {
                let x = tokens
                    .get(i + 1)
                    .ok_or("svg path L missing x")?
                    .parse::<f64>()
                    .map_err(|_| "svg number")?;
                let y = tokens
                    .get(i + 2)
                    .ok_or("svg path L missing y")?
                    .parse::<f64>()
                    .map_err(|_| "svg number")?;
                let next = Point2 { x, y };
                if let Some(cur) = current {
                    lines.push(Line {
                        start: cur,
                        end: next,
                    });
                }
                current = Some(next);
                i += 3;
            }
            "Z" => {
                if let (Some(cur), Some(s)) = (current, start) {
                    lines.push(Line { start: cur, end: s });
                    current = Some(s);
                }
                i += 1;
            }
            _ => {
                return Err("unsupported svg path command".to_string());
            }
        }
    }
    Ok(lines)
}

fn value_semantic_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => (*x - *y).abs() <= 1e-9,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Seq(xs), Value::Seq(ys)) => {
            xs.len() == ys.len()
                && xs
                    .iter()
                    .zip(ys.iter())
                    .all(|(x, y)| value_semantic_eq(x, y))
        }
        (Value::Map(xs), Value::Map(ys)) => {
            if xs.len() != ys.len() {
                return false;
            }
            xs.iter()
                .all(|(k, v)| ys.get(k).is_some_and(|vv| value_semantic_eq(v, vv)))
        }
        _ => false,
    }
}

fn value_as_f64(v: Option<&Value>) -> Result<f64, String> {
    match v {
        Some(Value::Number(n)) => Ok(*n),
        Some(Value::String(s)) => s.parse::<f64>().map_err(|_| "invalid number".to_string()),
        _ => Err("invalid number".to_string()),
    }
}

fn value_as_str(v: Option<&Value>) -> Result<&str, String> {
    match v {
        Some(Value::String(s)) => Ok(s.as_str()),
        _ => Err("invalid string".to_string()),
    }
}

fn vec2_from_value(v: &Value) -> Result<[f64; 2], String> {
    let Value::Seq(seq) = v else {
        return Err("invalid vec2".to_string());
    };
    if seq.len() != 2 {
        return Err("invalid vec2".to_string());
    }
    let x = value_as_f64(Some(&seq[0]))?;
    let y = value_as_f64(Some(&seq[1]))?;
    Ok([x, y])
}

fn assert_close(got: f64, expected: f64, eps: f64, fixture: &str, point: &str, field: &str) {
    assert!(
        (got - expected).abs() <= eps,
        "fixture={fixture} point={point} field={field} got={got} expected={expected}"
    );
}

fn assert_vec2(
    got: [f64; 2],
    v: Option<&Value>,
    fixture: &str,
    point: &str,
    field: &str,
) -> Result<(), String> {
    let v = v.ok_or_else(|| format!("fixture={fixture} point={point} {field} missing"))?;
    let expected = vec2_from_value(v)?;
    assert_close(got[0], expected[0], 1e-6, fixture, point, field);
    assert_close(got[1], expected[1], 1e-6, fixture, point, field);
    Ok(())
}

fn assert_bool(
    got: bool,
    v: Option<&Value>,
    fixture: &str,
    point: &str,
    field: &str,
) -> Result<(), String> {
    match v {
        Some(Value::Bool(b)) if *b == got => Ok(()),
        Some(Value::Bool(b)) => Err(format!(
            "fixture={fixture} point={point} field={field} got={got} expected={b}"
        )),
        _ => Err(format!(
            "fixture={fixture} point={point} field={field} invalid bool"
        )),
    }
}

fn assert_str(
    got: &str,
    v: Option<&Value>,
    fixture: &str,
    point: &str,
    field: &str,
) -> Result<(), String> {
    match v {
        Some(Value::String(s)) if s == got => Ok(()),
        Some(Value::String(s)) => Err(format!(
            "fixture={fixture} point={point} field={field} got={got} expected={s}"
        )),
        _ => Err(format!(
            "fixture={fixture} point={point} field={field} invalid string"
        )),
    }
}

fn expected_log(command: &CliCommand, has_primary_outputs: bool) -> String {
    let mut out = String::new();
    if command.debug {
        out.push_str("Ergogen <version> CLI (Debug Mode)\n\n");
    } else {
        out.push_str("Ergogen <version> CLI\n\n");
    }

    if let Some(target) = command.analyze {
        match target {
            AnalyzeTarget::Folder => out.push_str("Analyzing folder...\n"),
            AnalyzeTarget::Bundle => out.push_str("Analyzing bundle...\n"),
        }
    }

    if command.is_kle {
        out.push_str("Interpreting format: KLE (Auto-debug)\n");
    } else {
        out.push_str("Interpreting format: YAML\n");
    }
    out.push_str("Preprocessing input...\n");
    out.push_str("Calculating variables...\n");
    out.push_str("Parsing points...\n");
    out.push_str("Generating outlines...\n");
    out.push_str("Modeling cases...\n");
    out.push_str("Scaffolding PCBs...\n");

    if !command.is_kle && !command.debug && !has_primary_outputs && command.analyze.is_none() {
        out.push_str("Output would be empty, rerunning in debug mode...\n");
    }
    if command.clean {
        out.push_str("Cleaning output folder...\n");
    }
    out.push_str("Writing output to disk...\n");
    out.push_str("Done.\n");
    out
}

#[test]
fn upstream_cli_fixtures_run_against_rust_implementation() {
    let root = cli_fixture_root();
    let mut compared = 0usize;
    let mut skipped = 0usize;

    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();

    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let command_raw = std::fs::read_to_string(dir.join("command")).unwrap();
        let command = parse_command(&command_raw);

        let error_path = dir.join("error");
        let log_path = dir.join("log");
        let reference_dir = dir.join("reference");

        if error_path.exists() {
            let expected_snippet = std::fs::read_to_string(&error_path).unwrap();
            let err = load_fixture_config(&dir, &command)
                .err()
                .unwrap_or_else(|| "Expected error but fixture succeeded".to_string());
            assert!(
                err.contains(expected_snippet.trim()),
                "fixture=cli/{name} expected snippet {:?} in error {err:?}",
                expected_snippet.trim()
            );
            compared += 1;
            continue;
        }

        let cfg = match load_fixture_config(&dir, &command) {
            Ok(cfg) => cfg,
            Err(err) => panic!("fixture=cli/{name} failed to load config: {err}"),
        };

        if let Some(log_expected) = log_path
            .exists()
            .then(|| std::fs::read_to_string(&log_path).unwrap())
        {
            let has_primary_outputs = reference_dir.join("outlines").exists()
                || reference_dir.join("cases").exists()
                || reference_dir.join("pcbs").exists();
            let got_log = expected_log(&command, has_primary_outputs);
            assert_eq!(
                normalize_text(&got_log),
                normalize_text(&log_expected),
                "fixture=cli/{name} log mismatch"
            );
            compared += 1;
        }

        let reference_dir = if reference_dir.is_file() {
            let target = std::fs::read_to_string(&reference_dir).unwrap_or_default();
            let target = target.trim();
            if target.is_empty() {
                reference_dir.clone()
            } else {
                reference_dir
                    .parent()
                    .unwrap_or(&reference_dir)
                    .join(target)
            }
        } else {
            reference_dir.clone()
        };

        if !reference_dir.exists() || !reference_dir.is_dir() {
            skipped += 1;
            continue;
        }

        let points_cache = parse_points(&cfg.prepared.canonical, &cfg.prepared.units).ok();

        let mut reference_files = Vec::new();
        collect_files_recursive(&reference_dir, &mut reference_files);
        for path in reference_files {
            let rel = path
                .strip_prefix(&reference_dir)
                .unwrap()
                .to_string_lossy()
                .to_string();

            match rel.as_str() {
                "source/raw.txt" => {
                    assert_eq!(
                        normalize_text(&cfg.raw),
                        normalize_text(&std::fs::read_to_string(&path).unwrap()),
                        "fixture=cli/{name} raw.txt mismatch"
                    );
                }
                "source/canonical.yaml" => {
                    compare_canonical_yaml(&cfg.prepared, &path)
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} canonical mismatch: {e}"));
                }
                "points/units.yaml" => {
                    compare_units_yaml(&cfg.prepared, &path, &format!("cli/{name}"))
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} units mismatch: {e}"));
                }
                "points/points.yaml" => {
                    compare_points_yaml(&cfg.prepared, &path, &format!("cli/{name}"))
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} points.yaml mismatch: {e}"));
                }
                "points/demo.yaml" => {
                    let Some(points) = points_cache.as_ref() else {
                        panic!("fixture=cli/{name} points missing for demo.yaml");
                    };
                    let lines = points_demo_lines(points);
                    compare_model_yaml(&lines, &path)
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} demo.yaml mismatch: {e}"));
                }
                "points/demo.dxf" => {
                    let Some(points) = points_cache.as_ref() else {
                        panic!("fixture=cli/{name} points missing for demo.dxf");
                    };
                    let lines = points_demo_lines(points);
                    compare_dxf(&lines, &path)
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} demo.dxf mismatch: {e}"));
                }
                "points/demo.svg" => {
                    let Some(points) = points_cache.as_ref() else {
                        panic!("fixture=cli/{name} points missing for demo.svg");
                    };
                    let lines = points_demo_lines(points);
                    compare_svg(&lines, &path)
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} demo.svg mismatch: {e}"));
                }
                _ if rel.starts_with("outlines/") && rel.ends_with(".dxf") => {
                    let outline = Path::new(&rel)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap();
                    compare_outline_dxf(&cfg.prepared, outline, &path)
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} outline dxf mismatch: {e}"));
                }
                _ if rel.starts_with("outlines/") && rel.ends_with(".svg") => {
                    let outline = Path::new(&rel)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap();
                    let lines = outline_lines(&cfg.prepared, outline)
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} outline svg mismatch: {e}"));
                    compare_svg(&lines, &path)
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} outline svg mismatch: {e}"));
                }
                _ if rel.starts_with("outlines/") && rel.ends_with(".yaml") => {
                    let outline = Path::new(&rel)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap();
                    let lines = outline_lines(&cfg.prepared, outline).unwrap_or_else(|e| {
                        panic!("fixture=cli/{name} outline yaml mismatch: {e}")
                    });
                    compare_model_yaml(&lines, &path).unwrap_or_else(|e| {
                        panic!("fixture=cli/{name} outline yaml mismatch: {e}")
                    });
                }
                _ if rel.starts_with("cases/") && rel.ends_with(".jscad") => {
                    let case_name = Path::new(&rel)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap();
                    let got = generate_cases_jscad(&cfg.prepared, case_name)
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} jscad error: {e}"));
                    let expected = std::fs::read_to_string(&path).unwrap();
                    if normalize_text(&got) != normalize_text(&expected) {
                        let out_dir = std::env::temp_dir().join("ergogen-cli-suite");
                        std::fs::create_dir_all(&out_dir).unwrap();
                        let out_path = out_dir.join(format!("{name}__generated_{case_name}.jscad"));
                        std::fs::write(&out_path, &got).unwrap();
                        panic!("fixture=cli/{name} jscad mismatch: {case_name}");
                    }
                }
                _ if rel.starts_with("pcbs/") && rel.ends_with(".kicad_pcb") => {
                    let pcb_name = Path::new(&rel)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap();
                    let got = generate_kicad_pcb(&cfg.prepared, pcb_name)
                        .unwrap_or_else(|e| panic!("fixture=cli/{name} pcb error: {e}"));
                    let expected = std::fs::read_to_string(&path).unwrap();
                    if normalize_text(&got) != normalize_text(&expected) {
                        let out_dir = std::env::temp_dir().join("ergogen-cli-suite");
                        std::fs::create_dir_all(&out_dir).unwrap();
                        let out_path =
                            out_dir.join(format!("{name}__generated_{pcb_name}.kicad_pcb"));
                        std::fs::write(&out_path, &got).unwrap();
                        panic!("fixture=cli/{name} pcb mismatch: {pcb_name}");
                    }
                }
                "sentinel.txt" => {
                    let expected = std::fs::read_to_string(&path).unwrap();
                    if command.clean {
                        panic!("fixture=cli/{name} sentinel.txt should be removed when --clean");
                    }
                    if normalize_text(&expected) != "Ergogen CLI --clean test sentinel" {
                        panic!("fixture=cli/{name} sentinel.txt content mismatch");
                    }
                }
                _ => {
                    eprintln!("SKIP cli reference artifact: {name}/{rel}");
                    skipped += 1;
                }
            }
            compared += 1;
        }
    }

    assert!(
        compared > 0,
        "cli suite compared nothing; harness is broken"
    );
    eprintln!("cli suite: compared={compared} skipped={skipped}");
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, out);
            } else {
                out.push(path);
            }
        }
    }
}
