use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use ergogen_export::{dxf_geom, svg};
use ergogen_export::dxf::NormalizeOptions;
use serde::Serialize;

/// Returns the current crate version. Used as a minimal wasm smoke export.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Serialize)]
struct ErgogenError {
    kind: String,
    message: String,
}

fn to_js_error(kind: &str, message: String) -> JsValue {
    let err = ErgogenError {
        kind: kind.to_string(),
        message,
    };
    serde_wasm_bindgen::to_value(&err)
        .unwrap_or_else(|_| JsValue::from_str(&format!("{kind}: {}", err.message)))
}

fn outline_region(yaml: &str, outline_name: &str) -> Result<ergogen_geometry::region::Region, JsValue> {
    ergogen_outline::generate_outline_region_from_yaml_str(yaml, outline_name)
        .map_err(|e| to_js_error("outline", e.to_string()))
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
    svg::svg_from_dxf(&dxf)
        .map_err(|e| to_js_error("export", e.to_string()))
}

#[wasm_bindgen]
pub fn render_outlines(config_yaml: &str, outline_name: &str) -> Result<String, JsValue> {
    render_dxf(config_yaml, outline_name)
}
