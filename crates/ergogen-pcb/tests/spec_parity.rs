use ergogen_pcb::generate_kicad_pcb_from_yaml_str;

fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn parity_case(name: &str, template_yaml: &str, spec_yaml: &str) {
    let template = generate_kicad_pcb_from_yaml_str(template_yaml, "pcb").unwrap();
    let spec = generate_kicad_pcb_from_yaml_str(spec_yaml, "pcb").unwrap();
    assert_eq!(normalize(&spec), normalize(&template), "{name} parity mismatch");
}

#[test]
fn pad_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: pad
    params:
      net: P1
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: pad.yaml
      net: P1
"#;

    parity_case("pad", template_yaml, spec_yaml);
}

#[test]
fn diode_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: diode
    params:
      from: D_FROM
      to: D_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: diode.yaml
      from: D_FROM
      to: D_TO
"#;

    parity_case("diode", template_yaml, spec_yaml);
}

#[test]
fn button_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: button
    params:
      from: BTN_FROM
      to: BTN_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: button.yaml
      from: BTN_FROM
      to: BTN_TO
"#;

    parity_case("button", template_yaml, spec_yaml);
}

#[test]
fn mx_base_spec_matches_template() {
    let template_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints:
  - what: mx
    params:
      from: MX_FROM
      to: MX_TO
"#;

    let spec_yaml = r#"
meta.author: Ergogen Tests
meta.version: v9.9
points.zones.matrix:
pcbs.pcb.footprints_search_paths:
  - footprints
pcbs.pcb.footprints:
  - what: spec
    params:
      spec: mx_base.yaml
      from: MX_FROM
      to: MX_TO
"#;

    parity_case("mx_base", template_yaml, spec_yaml);
}
