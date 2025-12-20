use indexmap::IndexMap;

#[test]
fn spec_search_paths_can_resolve_from_virtual_fs_without_disk_files() {
    let mut files = IndexMap::new();
    files.insert(
        "footprints/specs/test.yaml".to_string(),
        r#"
name: test
params:
  net:
    type: net
    required: true
primitives:
  - type: pad
    at: [0, 0]
    size: [1, 1]
    layers: [F.Cu]
    net: "{{ net }}"
"#
        .to_string(),
    );
    ergogen_pcb::set_virtual_files(files);

    let yaml = r#"
points.zones.matrix: {}
pcbs:
  pcb:
    template: kicad8
    outlines: {}
    footprints_search_paths:
      - footprints/specs
    footprints:
      - what: spec
        where: true
        params:
          spec: test.yaml
          net: GND
"#;

    let pcb = ergogen_pcb::generate_kicad_pcb_from_yaml_str(yaml, "pcb").expect("pcb renders");
    assert!(pcb.contains("\"GND\""));

    ergogen_pcb::clear_virtual_files();
}
