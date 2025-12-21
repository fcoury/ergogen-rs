use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use ergogen_export::dxf::{Dxf, Entity, Line, NormalizeOptions};
use ergogen_export::dxf_geom::dxf_from_region;
use ergogen_export::jscad::{generate_cases_jscad, generate_cases_jscad_v2};
use ergogen_export::svg::{SvgError, svg_from_dxf};
use ergogen_layout::{PointsOutput, parse_points};
use ergogen_outline::generate_outline_region;
use ergogen_parser::{PreparedConfig, Value, convert_kle};
use ergogen_pcb::generate_kicad_pcb;
use serde::Serialize;
use tempfile::TempDir;
use zip::ZipArchive;

use crate::error::CliError;

fn fixture_dxf_opts() -> NormalizeOptions {
    NormalizeOptions {
        linear_eps: 1e-3,
        angle_eps_deg: 5e-3,
        ..NormalizeOptions::default()
    }
}

pub fn run_render(
    input: PathBuf,
    output: PathBuf,
    debug: bool,
    clean: bool,
    jscad_v2: bool,
) -> Result<(), CliError> {
    let debug_requested = debug;
    let orig_cwd = std::env::current_dir().map_err(|e| CliError::processing(e.to_string()))?;
    let input = absolutize_path(&orig_cwd, &input);
    let output = absolutize_path(&orig_cwd, &output);

    let resolved = resolve_config_path(&input)?;
    let _bundle_guard = resolved.tempdir;
    let (bundle_root, config_path) = (resolved.bundle_root, resolved.config_path);
    let _cwd_guard = CwdGuard::set(&bundle_root)?;

    let raw = std::fs::read_to_string(&config_path).map_err(|e| {
        CliError::input(format!(
            "Could not read config {}: {e}",
            config_path.display()
        ))
    })?;

    let parsed = Value::from_yaml_str(&raw).map_err(|e| CliError::input(e.to_string()))?;
    let (prepared, auto_debug, is_kle) = match parsed {
        Value::Map(_) => (
            PreparedConfig::from_value(&parsed).map_err(|e| CliError::input(e.to_string()))?,
            false,
            false,
        ),
        _ => {
            let converted = convert_kle(&parsed).map_err(|e| CliError::input(e.to_string()))?;
            (
                PreparedConfig::from_value(&converted)
                    .map_err(|e| CliError::input(e.to_string()))?,
                true,
                true,
            )
        }
    };
    let debug = debug || auto_debug;

    let outline_names = collect_names(&prepared.canonical, "outlines", debug);
    let pcb_names = collect_names(&prepared.canonical, "pcbs", debug);
    let case_names = collect_names(&prepared.canonical, "cases", debug);
    let has_primary_outputs =
        !(outline_names.is_empty() && pcb_names.is_empty() && case_names.is_empty());

    log_header(debug_requested);
    log_interpret(is_kle);
    println!("Preprocessing input...");
    println!("Calculating variables...");
    println!("Parsing points...");
    println!("Generating outlines...");
    println!("Modeling cases...");
    println!("Scaffolding PCBs...");
    if !is_kle && !debug_requested && !has_primary_outputs {
        println!("Output would be empty, rerunning in debug mode...");
    }
    if clean {
        println!("Cleaning output folder...");
    }
    println!("Writing output to disk...");

    if clean && output.exists() {
        std::fs::remove_dir_all(&output).map_err(|e| CliError::processing(e.to_string()))?;
    }
    std::fs::create_dir_all(&output).map_err(|e| CliError::processing(e.to_string()))?;

    if debug || !has_primary_outputs {
        write_source_outputs(&output, &raw, &prepared)?;
        write_points_outputs(&output, &prepared)?;
    }

    if !outline_names.is_empty() {
        write_outline_outputs(&output, &prepared, &outline_names, debug)?;
    }
    if !pcb_names.is_empty() {
        write_pcb_outputs(&output, &prepared, &pcb_names)?;
    }
    if !case_names.is_empty() {
        write_case_outputs(&output, &prepared, &case_names, jscad_v2)?;
    }

    println!("Done.");
    Ok(())
}

fn log_header(debug: bool) {
    if debug {
        println!("Ergogen <version> CLI (Debug Mode)\n");
    } else {
        println!("Ergogen <version> CLI\n");
    }
}

fn log_interpret(is_kle: bool) {
    if is_kle {
        println!("Interpreting format: KLE (Auto-debug)");
    } else {
        println!("Interpreting format: YAML");
    }
}

fn absolutize_path(cwd: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}

struct CwdGuard {
    prev: PathBuf,
}

impl CwdGuard {
    fn set(new_dir: &Path) -> Result<Self, CliError> {
        let prev = std::env::current_dir().map_err(|e| CliError::processing(e.to_string()))?;
        std::env::set_current_dir(new_dir).map_err(|e| CliError::processing(e.to_string()))?;
        Ok(Self { prev })
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prev);
    }
}

struct ResolvedInput {
    bundle_root: PathBuf,
    config_path: PathBuf,
    tempdir: Option<TempDir>,
}

fn resolve_config_path(input: &Path) -> Result<ResolvedInput, CliError> {
    if input.is_dir() {
        let config = find_bundle_config(input)?;
        return Ok(ResolvedInput {
            bundle_root: input.to_path_buf(),
            config_path: config,
            tempdir: None,
        });
    }

    if is_bundle_archive_path(input) {
        let (bundle_root, tempdir) = extract_bundle_archive(input)?;
        let config = find_bundle_config(&bundle_root)?;
        return Ok(ResolvedInput {
            bundle_root,
            config_path: config,
            tempdir: Some(tempdir),
        });
    }

    let root = input
        .parent()
        .ok_or_else(|| CliError::input("input path has no parent".to_string()))?
        .to_path_buf();
    Ok(ResolvedInput {
        bundle_root: root,
        config_path: input.to_path_buf(),
        tempdir: None,
    })
}

fn find_bundle_config(root: &Path) -> Result<PathBuf, CliError> {
    let mut configs: Vec<PathBuf> = Vec::new();
    for name in ["config.yaml", "config.yml"] {
        let path = root.join(name);
        if path.exists() {
            configs.push(path);
        }
    }
    if configs.len() > 1 {
        return Err(CliError::input("Ambiguous config in bundle!".to_string()));
    }
    if let Some(path) = configs.into_iter().next() {
        Ok(path)
    } else {
        Err(CliError::input(
            "Missing config in bundle (expected config.yaml or config.yml)".to_string(),
        ))
    }
}

fn is_bundle_archive_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(ext.to_ascii_lowercase().as_str(), "zip" | "ekb")
}

fn extract_bundle_archive(input: &Path) -> Result<(PathBuf, TempDir), CliError> {
    let file = File::open(input)
        .map_err(|e| CliError::input(format!("Could not open bundle {}: {e}", input.display())))?;
    let mut archive = ZipArchive::new(file).map_err(|e| CliError::input(e.to_string()))?;

    let dir = tempfile::tempdir().map_err(|e| CliError::processing(e.to_string()))?;
    for idx in 0..archive.len() {
        let mut entry = archive
            .by_index(idx)
            .map_err(|e| CliError::input(e.to_string()))?;
        let Some(name) = entry.enclosed_name() else {
            return Err(CliError::input(
                "Invalid path in bundle archive".to_string(),
            ));
        };
        let out_path = dir.path().join(name);
        if entry.name().ends_with('/') {
            std::fs::create_dir_all(&out_path).map_err(|e| CliError::processing(e.to_string()))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CliError::processing(e.to_string()))?;
        }
        let mut out_file =
            File::create(&out_path).map_err(|e| CliError::processing(e.to_string()))?;
        std::io::copy(&mut entry, &mut out_file)
            .map_err(|e| CliError::processing(e.to_string()))?;
    }

    Ok((dir.path().to_path_buf(), dir))
}

fn collect_names(canonical: &Value, key: &str, debug: bool) -> Vec<String> {
    let Some(Value::Map(map)) = canonical.get_path(key) else {
        return Vec::new();
    };
    map.keys()
        .filter(|name| debug || !name.starts_with('_'))
        .cloned()
        .collect()
}

fn write_source_outputs(
    output: &Path,
    raw: &str,
    prepared: &PreparedConfig,
) -> Result<(), CliError> {
    let dir = output.join("source");
    std::fs::create_dir_all(&dir).map_err(|e| CliError::processing(e.to_string()))?;

    std::fs::write(dir.join("raw.txt"), raw).map_err(|e| CliError::processing(e.to_string()))?;

    let canonical_yaml = serialize_yaml_no_doc(&prepared.canonical)?;
    std::fs::write(dir.join("canonical.yaml"), canonical_yaml)
        .map_err(|e| CliError::processing(e.to_string()))?;

    Ok(())
}

fn write_points_outputs(output: &Path, prepared: &PreparedConfig) -> Result<(), CliError> {
    let dir = output.join("points");
    std::fs::create_dir_all(&dir).map_err(|e| CliError::processing(e.to_string()))?;

    let points = parse_points(&prepared.canonical, &prepared.units)
        .map_err(|e| CliError::processing(e.to_string()))?;

    let units_vars = prepared.units.vars();
    let mut units_sorted: BTreeMap<String, f64> = BTreeMap::new();
    for (k, v) in units_vars {
        units_sorted.insert(k.clone(), *v);
    }

    std::fs::write(
        dir.join("units.yaml"),
        serialize_yaml_no_doc(&units_sorted)?,
    )
    .map_err(|e| CliError::processing(e.to_string()))?;
    std::fs::write(dir.join("points.yaml"), serialize_yaml_no_doc(&points)?)
        .map_err(|e| CliError::processing(e.to_string()))?;

    let demo_lines = points_demo_lines(&points);
    let demo_dxf = Dxf {
        entities: demo_lines.iter().cloned().map(Entity::Line).collect(),
    };
    write_dxf(&dir.join("demo.dxf"), &demo_dxf)?;
    std::fs::write(
        dir.join("demo.svg"),
        svg_from_dxf(&demo_dxf).map_err(|e| CliError::processing(e.to_string()))?,
    )
    .map_err(|e| CliError::processing(e.to_string()))?;
    std::fs::write(
        dir.join("demo.yaml"),
        serialize_yaml_no_doc(&model_yaml_from_lines(&demo_lines))?,
    )
    .map_err(|e| CliError::processing(e.to_string()))?;

    Ok(())
}

fn write_outline_outputs(
    output: &Path,
    prepared: &PreparedConfig,
    names: &[String],
    debug: bool,
) -> Result<(), CliError> {
    let dir = output.join("outlines");
    std::fs::create_dir_all(&dir).map_err(|e| CliError::processing(e.to_string()))?;

    for name in names {
        let region = generate_outline_region(prepared, name)
            .map_err(|e| CliError::processing(e.to_string()))?;
        let dxf = dxf_from_region(&region).map_err(|e| CliError::processing(e.to_string()))?;

        write_dxf(&dir.join(format!("{name}.dxf")), &dxf)?;
        match svg_from_dxf(&dxf) {
            Ok(svg) => {
                std::fs::write(dir.join(format!("{name}.svg")), svg)
                    .map_err(|e| CliError::processing(e.to_string()))?;
            }
            Err(SvgError::Empty) => {}
            Err(e) => return Err(CliError::processing(e.to_string())),
        }

        if debug && let Ok(lines) = collect_line_entities(&dxf) {
            std::fs::write(
                dir.join(format!("{name}.yaml")),
                serialize_yaml_no_doc(&model_yaml_from_lines(&lines))?,
            )
            .map_err(|e| CliError::processing(e.to_string()))?;
        }
    }

    Ok(())
}

fn write_pcb_outputs(
    output: &Path,
    prepared: &PreparedConfig,
    names: &[String],
) -> Result<(), CliError> {
    let dir = output.join("pcbs");
    std::fs::create_dir_all(&dir).map_err(|e| CliError::processing(e.to_string()))?;

    for name in names {
        let pcb =
            generate_kicad_pcb(prepared, name).map_err(|e| CliError::processing(e.to_string()))?;
        std::fs::write(dir.join(format!("{name}.kicad_pcb")), pcb)
            .map_err(|e| CliError::processing(e.to_string()))?;
    }

    Ok(())
}

fn write_case_outputs(
    output: &Path,
    prepared: &PreparedConfig,
    names: &[String],
    write_v2: bool,
) -> Result<(), CliError> {
    let dir = output.join("cases");
    std::fs::create_dir_all(&dir).map_err(|e| CliError::processing(e.to_string()))?;

    for name in names {
        let jscad = generate_cases_jscad(prepared, name)
            .map_err(|e| CliError::processing(e.to_string()))?;
        std::fs::write(dir.join(format!("{name}.jscad")), jscad)
            .map_err(|e| CliError::processing(e.to_string()))?;

        if write_v2 {
            let jscad_v2 = generate_cases_jscad_v2(prepared, name)
                .map_err(|e| CliError::processing(e.to_string()))?;
            std::fs::write(dir.join(format!("{name}.v2.jscad")), jscad_v2)
                .map_err(|e| CliError::processing(e.to_string()))?;
        }
    }

    Ok(())
}

fn write_dxf(path: &Path, dxf: &Dxf) -> Result<(), CliError> {
    let opts = fixture_dxf_opts();
    let normalized = dxf
        .normalize(opts)
        .map_err(|e| CliError::processing(e.to_string()))?;
    let out_str = normalized
        .to_dxf_string(opts)
        .map_err(|e| CliError::processing(e.to_string()))?;
    std::fs::write(path, out_str).map_err(|e| CliError::processing(e.to_string()))
}

fn serialize_yaml_no_doc<T: Serialize>(value: &T) -> Result<String, CliError> {
    let mut s = serde_yaml::to_string(value).map_err(|e| CliError::processing(e.to_string()))?;
    if let Some(rest) = s.strip_prefix("---\n") {
        s = rest.to_string();
    }
    Ok(s)
}

#[derive(Debug, Serialize)]
struct ModelYamlRoot {
    models: BTreeMap<String, ModelYamlModel>,
    units: String,
    origin: [f64; 2],
}

#[derive(Debug, Serialize)]
struct ModelYamlModel {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    models: BTreeMap<String, ModelYamlModel>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    paths: BTreeMap<String, ModelYamlPath>,
    origin: [f64; 2],
}

#[derive(Debug, Serialize)]
struct ModelYamlPath {
    #[serde(rename = "type")]
    kind: String,
    origin: [f64; 2],
    end: [f64; 2],
}

fn model_yaml_from_lines(lines: &[Line]) -> ModelYamlRoot {
    let mut paths: BTreeMap<String, ModelYamlPath> = BTreeMap::new();
    for (idx, line) in lines.iter().enumerate() {
        paths.insert(
            format!("ShapeLine{}", idx + 1),
            ModelYamlPath {
                kind: "line".to_string(),
                origin: [line.start.x, line.start.y],
                end: [line.end.x, line.end.y],
            },
        );
    }

    let mut export_models = BTreeMap::new();
    export_models.insert(
        "shape".to_string(),
        ModelYamlModel {
            models: BTreeMap::new(),
            paths,
            origin: [0.0, 0.0],
        },
    );

    let mut models = BTreeMap::new();
    models.insert(
        "export".to_string(),
        ModelYamlModel {
            models: export_models,
            paths: BTreeMap::new(),
            origin: [0.0, 0.0],
        },
    );

    ModelYamlRoot {
        models,
        units: "mm".to_string(),
        origin: [0.0, 0.0],
    }
}

fn collect_line_entities(dxf: &Dxf) -> Result<Vec<Line>, String> {
    let mut lines = Vec::new();
    for entity in &dxf.entities {
        match entity {
            Entity::Line(line) => lines.push(*line),
            _ => return Err("model yaml export only supports line entities".to_string()),
        }
    }
    Ok(lines)
}

fn points_demo_lines(points: &PointsOutput) -> Vec<Line> {
    let mut entities: Vec<Line> = Vec::new();
    for p in points.values() {
        let hw = p.meta.width / 2.0;
        let hh = p.meta.height / 2.0;
        let corners = [(-hw, hh), (hw, hh), (hw, -hh), (-hw, -hh)];
        let (sin, cos) = p.r.to_radians().sin_cos();
        let mut pts = Vec::with_capacity(4);
        for (x, y) in corners {
            let rx = x * cos - y * sin;
            let ry = x * sin + y * cos;
            pts.push(ergogen_export::dxf::Point2 {
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
