use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

use ergogen_export::dxf::{Dxf, Entity, Line, NormalizeOptions, Point2};
use ergogen_export::{dxf_geom, svg};
use ergogen_layout::{PointsOutput, parse_points};
use ergogen_parser::{PreparedConfig, Value, convert_kle};
use indexmap::IndexMap;
use serde::Serialize;

/// Returns the current crate version. Used as a minimal wasm smoke export.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub fn set_virtual_fs(files: JsValue) -> Result<(), JsValue> {
    let map: IndexMap<String, String> = serde_wasm_bindgen::from_value(files)
        .map_err(|e| to_js_error("wasm", format!("invalid virtual fs map: {e}")))?;
    ergogen_pcb::set_virtual_files(map);
    Ok(())
}

#[wasm_bindgen]
pub fn clear_virtual_fs() {
    ergogen_pcb::clear_virtual_files();
}

#[derive(Serialize)]
struct ErgogenError {
    kind: String,
    message: String,
    target: Option<String>,
}

#[derive(Serialize)]
struct DemoOutput {
    dxf: String,
    svg: String,
}

#[derive(Serialize)]
struct OutlineOutput {
    dxf: String,
    svg: String,
}

#[derive(Serialize)]
struct CaseOutput {
    jscad: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    jscad_v2: Option<String>,
}
fn to_js_error(kind: &str, message: String) -> JsValue {
    let err = ErgogenError {
        kind: kind.to_string(),
        message,
        target: None,
    };
    serde_wasm_bindgen::to_value(&err)
        .unwrap_or_else(|_| JsValue::from_str(&format!("{kind}: {}", err.message)))
}

fn push_error(errors: &mut Vec<ErgogenError>, kind: &str, target: &str, message: String) {
    errors.push(ErgogenError {
        kind: kind.to_string(),
        message,
        target: Some(target.to_string()),
    });
}

fn outline_region(
    yaml: &str,
    outline_name: &str,
) -> Result<ergogen_geometry::region::Region, JsValue> {
    let prepared = prepare_config(yaml)?;
    ergogen_outline::generate_outline_region(&prepared, outline_name)
        .map_err(|e| to_js_error("outline", e.to_string()))
}

fn prepare_config(raw: &str) -> Result<PreparedConfig, JsValue> {
    let parsed = Value::from_yaml_str(raw).map_err(|e| to_js_error("parser", e.to_string()))?;
    let raw = match parsed {
        Value::Map(_) => parsed,
        _ => convert_kle(&parsed).map_err(|e| to_js_error("parser", e.to_string()))?,
    };
    PreparedConfig::from_value(&raw).map_err(|e| to_js_error("parser", e.to_string()))
}

#[derive(Serialize)]
struct RenderAllOutput {
    canonical: ergogen_parser::Value,
    points: PointsOutput,
    units: IndexMap<String, f64>,
    demo: DemoOutput,
    pcbs: IndexMap<String, String>,
    outlines: IndexMap<String, OutlineOutput>,
    cases: IndexMap<String, CaseOutput>,
    errors: Vec<ErgogenError>,
}

#[wasm_bindgen]
pub fn render_all(config_yaml: &str) -> Result<JsValue, JsValue> {
    let prepared = prepare_config(config_yaml)?;

    let canonical = prepared.canonical.clone();
    let units = prepared.units.vars().clone();
    let points = parse_points(&prepared.canonical, &prepared.units)
        .map_err(|e| to_js_error("points", e.to_string()))?;
    let demo = demo_from_points(&points).map_err(|e| to_js_error("demo", e))?;

    let mut pcbs = IndexMap::new();
    let mut outlines = IndexMap::new();
    let mut cases = IndexMap::new();
    let mut errors = Vec::new();

    if let Some(map) = prepared.canonical.get_path("pcbs").and_then(|v| v.as_map()) {
        for name in map.keys() {
            match render_pcb(config_yaml, name) {
                Ok(pcb) => {
                    pcbs.insert(name.clone(), pcb);
                }
                Err(err) => {
                    push_error(&mut errors, "pcb", name, format!("{err:?}"));
                }
            }
        }
    }

    if let Some(map) = prepared
        .canonical
        .get_path("outlines")
        .and_then(|v| v.as_map())
    {
        for name in map.keys() {
            let mut dxf = String::new();
            let mut svg_str = String::new();
            match render_dxf(config_yaml, name) {
                Ok(value) => dxf = value,
                Err(err) => push_error(&mut errors, "outline", name, format!("{err:?}")),
            }
            match render_svg(config_yaml, name) {
                Ok(value) => svg_str = value,
                Err(err) => push_error(&mut errors, "svg", name, format!("{err:?}")),
            }
            if !dxf.is_empty() || !svg_str.is_empty() {
                outlines.insert(name.clone(), OutlineOutput { dxf, svg: svg_str });
            }
        }
    }

    if let Some(map) = prepared
        .canonical
        .get_path("cases")
        .and_then(|v| v.as_map())
    {
        for name in map.keys() {
            let mut v1: Option<String> = None;
            let mut v2: Option<String> = None;

            match ergogen_export::jscad::generate_cases_jscad(&prepared, name) {
                Ok(jscad) => v1 = Some(jscad),
                Err(err) => push_error(&mut errors, "case", name, format!("{err:?}")),
            }
            match ergogen_export::jscad::generate_cases_jscad_v2(&prepared, name) {
                Ok(jscad) => v2 = Some(jscad),
                Err(err) => push_error(&mut errors, "case_v2", name, format!("{err:?}")),
            }

            if let Some(jscad) = v1 {
                cases.insert(
                    name.clone(),
                    CaseOutput {
                        jscad,
                        jscad_v2: v2,
                    },
                );
            } else if v2.is_some() {
                cases.insert(
                    name.clone(),
                    CaseOutput {
                        jscad: String::new(),
                        jscad_v2: v2,
                    },
                );
            }
        }
    }

    let out = RenderAllOutput {
        canonical,
        points,
        units,
        demo,
        pcbs,
        outlines,
        cases,
        errors,
    };
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    out.serialize(&serializer)
        .map_err(|e| to_js_error("wasm", e.to_string()))
}

#[wasm_bindgen]
pub fn render_pcb(config_yaml: &str, pcb_name: &str) -> Result<String, JsValue> {
    let prepared = prepare_config(config_yaml)?;
    ergogen_pcb::generate_kicad_pcb(&prepared, pcb_name).map_err(|e| to_js_error("pcb", e.to_string()))
}

#[wasm_bindgen]
pub fn render_dxf(config_yaml: &str, outline_name: &str) -> Result<String, JsValue> {
    let region = outline_region(config_yaml, outline_name)?;
    let dxf =
        dxf_geom::dxf_from_region(&region).map_err(|e| to_js_error("export", e.to_string()))?;
    let normalized = dxf
        .normalize(NormalizeOptions::default())
        .map_err(|e| to_js_error("export", e.to_string()))?;
    normalized
        .to_dxf_string(NormalizeOptions::default())
        .map_err(|e| to_js_error("export", e.to_string()))
}

#[wasm_bindgen]
pub fn render_svg(config_yaml: &str, outline_name: &str) -> Result<String, JsValue> {
    let region = outline_region(config_yaml, outline_name)?;
    let dxf =
        dxf_geom::dxf_from_region(&region).map_err(|e| to_js_error("export", e.to_string()))?;
    let normalized = dxf
        .normalize(NormalizeOptions::default())
        .map_err(|e| to_js_error("export", e.to_string()))?;
    let dxf_str = normalized
        .to_dxf_string(NormalizeOptions::default())
        .map_err(|e| to_js_error("export", e.to_string()))?;
    let reparsed = ergogen_export::dxf::Dxf::parse_str(&dxf_str)
        .map_err(|e| to_js_error("export", e.to_string()))?;
    svg::svg_from_dxf(&reparsed).map_err(|e| to_js_error("export", e.to_string()))
}

#[wasm_bindgen]
pub fn render_outlines(config_yaml: &str, outline_name: &str) -> Result<String, JsValue> {
    render_dxf(config_yaml, outline_name)
}

#[wasm_bindgen]
pub fn render_case_jscad_v2(config_yaml: &str, case_name: &str) -> Result<String, JsValue> {
    let prepared = prepare_config(config_yaml)?;
    ergogen_export::jscad::generate_cases_jscad_v2(&prepared, case_name)
        .map_err(|e| to_js_error("case_v2", e.to_string()))
}

fn demo_from_points(points: &PointsOutput) -> Result<DemoOutput, String> {
    let lines = points_demo_lines(points);
    let dxf = Dxf {
        entities: lines.iter().cloned().map(Entity::Line).collect(),
    };
    let opts = NormalizeOptions::default();
    let normalized = dxf.normalize(opts).map_err(|e| e.to_string())?;
    let dxf_str = normalized.to_dxf_string(opts).map_err(|e| e.to_string())?;
    let svg_str = svg::svg_from_lines(&lines).map_err(|e| e.to_string())?;
    Ok(DemoOutput {
        dxf: dxf_str,
        svg: svg_str,
    })
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
