use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_test::wasm_bindgen_test;

use ergogen_export::dxf::{Dxf, NormalizeOptions};
use ergogen_export::svg;

#[wasm_bindgen(module = "/js/footprints.js")]
unsafe extern "C" {
    fn installErgogenJsFootprints();
    fn registerErgogenJsFootprintSource(path: &str, source: &str);
}

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
        .trim_end_matches('\n')
        .to_string()
}

#[wasm_bindgen_test]
fn renders_js_footprint_fixture() {
    installErgogenJsFootprints();
    let js_source = include_str!("../../../fixtures/m7/js_footprints/simple.js");
    registerErgogenJsFootprintSource("simple.js", js_source);

    let yaml = include_str!("../../../fixtures/m7/js_footprints/simple.yaml");
    let expected =
        include_str!("../../../fixtures/m7/js_footprints/simple___pcbs_pcb.kicad_pcb");

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
