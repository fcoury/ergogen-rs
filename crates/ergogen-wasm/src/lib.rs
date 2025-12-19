use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use ergogen_export::{dxf_geom, svg};
use ergogen_export::dxf::NormalizeOptions;
use ergogen_parser::PreparedConfig;
use serde::Serialize;
use indexmap::IndexMap;

/// Returns the current crate version. Used as a minimal wasm smoke export.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Serialize)]
struct ErgogenError {
    kind: String,
    message: String,
    target: Option<String>,
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

fn outline_region(yaml: &str, outline_name: &str) -> Result<ergogen_geometry::region::Region, JsValue> {
    ergogen_outline::generate_outline_region_from_yaml_str(yaml, outline_name)
        .map_err(|e| to_js_error("outline", e.to_string()))
}

#[derive(Serialize)]
struct RenderAllOutput {
    pcbs: IndexMap<String, String>,
    outlines: IndexMap<String, String>,
    svgs: IndexMap<String, String>,
    errors: Vec<ErgogenError>,
}

#[wasm_bindgen]
pub fn render_all(config_yaml: &str) -> Result<JsValue, JsValue> {
    let prepared = PreparedConfig::from_yaml_str(config_yaml)
        .map_err(|e| to_js_error("parser", e.to_string()))?;

    let mut pcbs = IndexMap::new();
    let mut outlines = IndexMap::new();
    let mut svgs = IndexMap::new();
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
            match render_dxf(config_yaml, name) {
                Ok(dxf) => {
                    outlines.insert(name.clone(), dxf);
                }
                Err(err) => {
                    push_error(&mut errors, "outline", name, format!("{err:?}"));
                }
            }
            match render_svg(config_yaml, name) {
                Ok(svg_str) => {
                    svgs.insert(name.clone(), svg_str);
                }
                Err(err) => {
                    push_error(&mut errors, "svg", name, format!("{err:?}"));
                }
            }
        }
    }

    let out = RenderAllOutput {
        pcbs,
        outlines,
        svgs,
        errors,
    };
    serde_wasm_bindgen::to_value(&out).map_err(|e| to_js_error("wasm", e.to_string()))
}

#[wasm_bindgen]
pub fn render_pcb(config_yaml: &str, pcb_name: &str) -> Result<String, JsValue> {
    ergogen_pcb::generate_kicad_pcb_from_yaml_str(config_yaml, pcb_name)
        .map_err(|e| to_js_error("pcb", e.to_string()))
}

#[wasm_bindgen]
pub fn render_dxf(config_yaml: &str, outline_name: &str) -> Result<String, JsValue> {
    let region = outline_region(config_yaml, outline_name)?;
    let dxf = dxf_geom::dxf_from_region(&region)
        .map_err(|e| to_js_error("export", e.to_string()))?;
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
    let dxf = dxf_geom::dxf_from_region(&region)
        .map_err(|e| to_js_error("export", e.to_string()))?;
    let normalized = dxf
        .normalize(NormalizeOptions::default())
        .map_err(|e| to_js_error("export", e.to_string()))?;
    let dxf_str = normalized
        .to_dxf_string(NormalizeOptions::default())
        .map_err(|e| to_js_error("export", e.to_string()))?;
    let reparsed = ergogen_export::dxf::Dxf::parse_str(&dxf_str)
        .map_err(|e| to_js_error("export", e.to_string()))?;
    svg::svg_from_dxf(&reparsed)
        .map_err(|e| to_js_error("export", e.to_string()))
}

#[wasm_bindgen]
pub fn render_outlines(config_yaml: &str, outline_name: &str) -> Result<String, JsValue> {
    render_dxf(config_yaml, outline_name)
}
