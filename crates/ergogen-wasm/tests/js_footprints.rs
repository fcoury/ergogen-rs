use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_test::wasm_bindgen_test;

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

    let got = ergogen_pcb::generate_kicad_pcb_from_yaml_str(yaml, "pcb").unwrap();
    assert_eq!(normalize(&got), normalize(expected));
}
