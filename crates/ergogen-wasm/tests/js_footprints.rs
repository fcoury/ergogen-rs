use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_test::wasm_bindgen_test;

use ergogen_export::dxf::{Dxf, NormalizeOptions};
use ergogen_export::svg;
use ergogen_layout::PointsOutput;
use ergogen_parser::Value as ErgogenValue;
use indexmap::IndexMap;
use serde::Deserialize;

#[wasm_bindgen(module = "/js/footprints.js")]
unsafe extern "C" {
    fn installErgogenJsFootprints();
    fn registerErgogenJsFootprintSource(path: &str, source: &str);
}

#[derive(Deserialize)]
struct DemoOutput {
    dxf: String,
    svg: String,
}

#[derive(Deserialize)]
struct OutlineOutput {
    dxf: String,
    svg: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct CaseOutput {
    jscad: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ErgogenError {
    kind: String,
    message: String,
    target: Option<String>,
}

#[derive(Deserialize)]
struct RenderAllOutput {
    canonical: ErgogenValue,
    points: PointsOutput,
    units: IndexMap<String, f64>,
    demo: DemoOutput,
    pcbs: IndexMap<String, String>,
    outlines: IndexMap<String, OutlineOutput>,
    cases: IndexMap<String, CaseOutput>,
    errors: Vec<ErgogenError>,
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end_matches('\n').to_string()
}

#[wasm_bindgen_test]
fn renders_js_footprint_fixture() {
    installErgogenJsFootprints();
    let js_source = include_str!("../../../fixtures/m7/js_footprints/simple.js");
    registerErgogenJsFootprintSource("simple.js", js_source);

    let yaml = include_str!("../../../fixtures/m7/js_footprints/simple.yaml");
    let expected = include_str!("../../../fixtures/m7/js_footprints/simple___pcbs_pcb.kicad_pcb");

    let got = ergogen_wasm::render_pcb(yaml, "pcb").unwrap();
    assert_eq!(normalize(&got), normalize(expected));
}

#[wasm_bindgen_test]
fn renders_outline_dxf_fixture() {
    let yaml = include_str!("../../../fixtures/m5/outlines/basic.yaml");
    let expected = include_str!("../../../fixtures/m5/outlines/basic___outlines_outline_dxf.dxf");

    let got = ergogen_wasm::render_dxf(yaml, "outline").unwrap();

    let left = Dxf::parse_str(&got).unwrap();
    let right = Dxf::parse_str(expected).unwrap();
    let left_norm = left.normalize(NormalizeOptions::default()).unwrap();
    let right_norm = right.normalize(NormalizeOptions::default()).unwrap();
    left_norm.compare_semantic(&right_norm).unwrap();
}

#[wasm_bindgen_test]
fn renders_outline_svg_fixture() {
    let yaml = include_str!("../../../fixtures/m5/outlines/basic.yaml");
    let dxf = ergogen_wasm::render_dxf(yaml, "outline").unwrap();
    let expected_svg = svg::svg_from_dxf(&Dxf::parse_str(&dxf).unwrap()).unwrap();

    let got = ergogen_wasm::render_svg(yaml, "outline").unwrap();
    assert_eq!(normalize(&got), normalize(&expected_svg));
}

#[wasm_bindgen_test]
fn render_all_includes_points_units_demo() {
    let yaml = include_str!("../../../fixtures/m5/outlines/basic.yaml");
    let value = ergogen_wasm::render_all(yaml).unwrap();
    let output: RenderAllOutput = serde_wasm_bindgen::from_value(value).unwrap();

    assert!(output.canonical.as_map().is_some());
    assert!(!output.points.is_empty());
    assert!(!output.units.is_empty());
    assert!(!output.demo.dxf.trim().is_empty());
    assert!(!output.demo.svg.trim().is_empty());
    let outline = output.outlines.get("outline").expect("outline output");
    assert!(!outline.dxf.trim().is_empty());
    assert!(!outline.svg.trim().is_empty());
    assert!(output.pcbs.is_empty());
    assert!(output.cases.is_empty());
    assert!(output.errors.is_empty());
}

#[wasm_bindgen_test]
fn render_all_accepts_kle_json() {
    let kle_json = include_str!("../../../fixtures/upstream/fixtures/minimal_kle.json");
    let value = ergogen_wasm::render_all(kle_json).unwrap();
    let output: RenderAllOutput = serde_wasm_bindgen::from_value(value).unwrap();

    let zones = output
        .canonical
        .get_path("points.zones")
        .and_then(|v| v.as_map())
        .expect("zones map");
    assert_eq!(zones.len(), 6);

    let label = match output
        .canonical
        .get_path("points.zones.key1.columns.key1col.rows.key1row.label")
    {
        Some(ErgogenValue::String(s)) => s.as_str(),
        _ => "",
    };
    assert_eq!(label, "0_0");
}
