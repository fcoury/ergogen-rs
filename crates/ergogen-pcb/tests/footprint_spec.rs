use std::path::PathBuf;

use ergogen_parser::Value;
use ergogen_pcb::footprint_spec::{
    FootprintSpec, ParamKind, Primitive, ResolvedDrill, ResolvedPrimitive, ScalarSpec, TextKind,
    parse_footprint_spec, resolve_footprint_spec,
};
use indexmap::IndexMap;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn parses_minimal_pad_footprint_spec() {
    let yaml_path = workspace_root().join("fixtures/m7/footprints/pad_minimal.yaml");
    let yaml = std::fs::read_to_string(yaml_path).unwrap();
    let spec: FootprintSpec = parse_footprint_spec(&yaml).unwrap();

    assert_eq!(spec.name, "pad_minimal");
    assert_eq!(spec.params.len(), 2);
    let net = spec.params.get("net").unwrap();
    assert_eq!(net.kind, ParamKind::Net);
    assert!(net.required);
    let mask = spec.params.get("mask").unwrap();
    assert_eq!(mask.kind, ParamKind::Boolean);
    assert!(!mask.required);
    assert!(mask.default.is_some());

    assert_eq!(spec.primitives.len(), 1);
    match &spec.primitives[0] {
        Primitive::Pad {
            at,
            size,
            layers,
            net,
            ..
        } => {
            assert_eq!(*at, [ScalarSpec::Number(0.0), ScalarSpec::Number(0.0)]);
            assert_eq!(*size, [ScalarSpec::Number(2.5), ScalarSpec::Number(2.5)]);
            assert_eq!(layers, &vec!["F.Cu".to_string(), "F.Mask".to_string()]);
            assert_eq!(net, "{{ net }}");
        }
        _ => panic!("expected pad primitive"),
    }
}

#[test]
fn resolves_pad_net_template_and_defaults() {
    let yaml_path = workspace_root().join("fixtures/m7/footprints/pad_minimal.yaml");
    let yaml = std::fs::read_to_string(yaml_path).unwrap();
    let spec = parse_footprint_spec(&yaml).unwrap();

    let mut params = IndexMap::new();
    params.insert("net".to_string(), Value::String("GND".to_string()));
    let resolved = resolve_footprint_spec(&spec, &params).unwrap();

    assert_eq!(resolved.params.get("mask"), Some(&Value::Bool(false)));
    assert_eq!(resolved.primitives.len(), 1);
    match &resolved.primitives[0] {
        ResolvedPrimitive::Pad { net, .. } => {
            assert_eq!(net, "GND");
        }
        _ => panic!("expected pad primitive"),
    }
}

#[test]
fn resolves_templated_vectors_and_layers() {
    let yaml_path = workspace_root().join("fixtures/m7/footprints/pad_templated.yaml");
    let yaml = std::fs::read_to_string(yaml_path).unwrap();
    let spec = parse_footprint_spec(&yaml).unwrap();

    let mut params = IndexMap::new();
    params.insert("net".to_string(), Value::String("VCC".to_string()));
    params.insert("dx".to_string(), Value::Number(1.25));
    params.insert("dy".to_string(), Value::Number(-2.5));
    params.insert("size_x".to_string(), Value::Number(3.0));
    params.insert("size_y".to_string(), Value::Number(1.5));
    params.insert("layer".to_string(), Value::String("B.Cu".to_string()));

    let resolved = resolve_footprint_spec(&spec, &params).unwrap();
    match &resolved.primitives[0] {
        ResolvedPrimitive::Pad {
            at,
            size,
            layers,
            net,
            ..
        } => {
            assert_eq!(*at, [1.25, -2.5]);
            assert_eq!(*size, [3.0, 1.5]);
            assert_eq!(layers, &vec!["B.Cu".to_string(), "F.Mask".to_string()]);
            assert_eq!(net, "VCC");
        }
        _ => panic!("expected pad primitive"),
    }
}

#[test]
fn missing_placeholder_param_errors() {
    let yaml_path = workspace_root().join("fixtures/m7/footprints/pad_missing_placeholder.yaml");
    let yaml = std::fs::read_to_string(yaml_path).unwrap();
    let spec = parse_footprint_spec(&yaml).unwrap();

    let mut params = IndexMap::new();
    params.insert("net".to_string(), Value::String("GND".to_string()));

    let err = resolve_footprint_spec(&spec, &params).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("missing param"));
}

#[test]
fn parses_new_primitives() {
    let yaml_path = workspace_root().join("fixtures/m7/footprints/pad_primitives.yaml");
    let yaml = std::fs::read_to_string(yaml_path).unwrap();
    let spec = parse_footprint_spec(&yaml).unwrap();

    assert_eq!(spec.primitives.len(), 4);
    assert!(matches!(spec.primitives[1], Primitive::PadThru { .. }));
    assert!(matches!(spec.primitives[2], Primitive::Circle { .. }));
    assert!(matches!(spec.primitives[3], Primitive::Line { .. }));
}

#[test]
fn parses_pad_thru_shape() {
    let yaml_path = workspace_root().join("fixtures/m7/footprints/diode.yaml");
    let yaml = std::fs::read_to_string(yaml_path).unwrap();
    let spec = parse_footprint_spec(&yaml).unwrap();

    let shapes: Vec<Option<String>> = spec
        .primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::PadThru { shape, .. } => Some(shape.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        shapes,
        vec![Some("rect".to_string()), Some("circle".to_string())]
    );
}

#[test]
fn parses_arc_rect_text_primitives() {
    let yaml_path = workspace_root().join("fixtures/m7/footprints/primitives_templated.yaml");
    let yaml = std::fs::read_to_string(yaml_path).unwrap();
    let spec = parse_footprint_spec(&yaml).unwrap();

    assert!(
        spec.primitives
            .iter()
            .any(|p| matches!(p, Primitive::Arc { .. }))
    );
    assert!(
        spec.primitives
            .iter()
            .any(|p| matches!(p, Primitive::Rect { .. }))
    );
    assert!(
        spec.primitives
            .iter()
            .any(|p| matches!(p, Primitive::Text { .. }))
    );
}

#[test]
fn parses_pad_thru_kind_and_oval_drill() {
    let yaml = r#"
name: pad_thru_kind
params:
  net:
    type: net
    default: ""
primitives:
  - type: pad_thru
    at: [0, 0]
    size: [1.0, 2.0]
    drill: [0.9, 1.5]
    layers:
      - "*.Cu"
      - "*.Mask"
    net: ""
    kind: np_thru_hole
    number: ""
"#;
    let spec = parse_footprint_spec(yaml).unwrap();
    match &spec.primitives[0] {
        Primitive::PadThru { drill, kind, .. } => {
            assert!(matches!(
                drill,
                ergogen_pcb::footprint_spec::DrillSpec::Vector(_)
            ));
            assert_eq!(kind.as_deref(), Some("np_thru_hole"));
        }
        _ => panic!("expected pad_thru primitive"),
    }

    let resolved = resolve_footprint_spec(&spec, &IndexMap::new()).unwrap();
    match &resolved.primitives[0] {
        ResolvedPrimitive::PadThru { drill, kind, .. } => {
            assert_eq!(kind.as_deref(), Some("np_thru_hole"));
            assert_eq!(*drill, ResolvedDrill::Vector([0.9, 1.5]));
        }
        _ => panic!("expected pad_thru primitive"),
    }
}

#[test]
fn resolves_shape_templates() {
    let yaml_path = workspace_root().join("fixtures/m7/footprints/primitives_templated.yaml");
    let yaml = std::fs::read_to_string(yaml_path).unwrap();
    let spec = parse_footprint_spec(&yaml).unwrap();

    let mut params = IndexMap::new();
    params.insert("radius".to_string(), Value::Number(3.0));
    params.insert("line_w".to_string(), Value::Number(0.25));
    params.insert("arc_angle".to_string(), Value::Number(45.0));
    params.insert("rect_w".to_string(), Value::Number(5.0));
    params.insert("rect_h".to_string(), Value::Number(2.5));
    params.insert("text_size".to_string(), Value::Number(1.1));
    params.insert("label".to_string(), Value::String("HELLO".to_string()));

    let resolved = resolve_footprint_spec(&spec, &params).unwrap();

    let circle = resolved
        .primitives
        .iter()
        .find_map(|p| {
            if let ResolvedPrimitive::Circle { radius, width, .. } = p {
                Some((*radius, *width))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(circle.0, 3.0);
    assert_eq!(circle.1, 0.25);

    let line = resolved
        .primitives
        .iter()
        .find_map(|p| {
            if let ResolvedPrimitive::Line { width, .. } = p {
                Some(*width)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(line, 0.25);

    let arc = resolved
        .primitives
        .iter()
        .find_map(|p| {
            if let ResolvedPrimitive::Arc { angle, .. } = p {
                Some(*angle)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(arc, 45.0);

    let rect = resolved
        .primitives
        .iter()
        .find_map(|p| {
            if let ResolvedPrimitive::Rect { size, .. } = p {
                Some(*size)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(rect, [5.0, 2.5]);

    let text = resolved
        .primitives
        .iter()
        .find_map(|p| {
            if let ResolvedPrimitive::Text {
                text,
                size,
                thickness,
                justify,
                ..
            } = p
            {
                Some((text.as_str(), *size, *thickness, justify.clone()))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(text.0, "HELLO");
    assert_eq!(text.1, [1.1, 1.1]);
    assert_eq!(text.2, 0.25);
    assert_eq!(text.3, Some("left".to_string()));
}

#[test]
fn parses_and_resolves_text_kind_reference_and_value() {
    let yaml = r#"
name: text_kinds
params: {}
primitives:
  - type: text
    kind: reference
    at: [0, 14.2]
    layer: Dwgs.User
  - type: text
    kind: value
    text: TRRS-PJ-320A-dual
    at: [0, -5.6]
    layer: F.Fab
  - type: text
    text: HELLO
    at: [1, 2]
    layer: F.SilkS
    rotation: 90
    hide: true
"#;
    let spec = parse_footprint_spec(yaml).unwrap();

    assert!(matches!(
        spec.primitives[0],
        Primitive::Text {
            kind: TextKind::Reference,
            ..
        }
    ));
    assert!(matches!(
        spec.primitives[1],
        Primitive::Text {
            kind: TextKind::Value,
            ..
        }
    ));

    let resolved = resolve_footprint_spec(&spec, &IndexMap::new()).unwrap();
    assert!(matches!(
        resolved.primitives[0],
        ResolvedPrimitive::Text {
            kind: TextKind::Reference,
            ..
        }
    ));
    match &resolved.primitives[1] {
        ResolvedPrimitive::Text { kind, text, .. } => {
            assert_eq!(*kind, TextKind::Value);
            assert_eq!(text, "TRRS-PJ-320A-dual");
        }
        _ => panic!("expected text primitive"),
    }
    match &resolved.primitives[2] {
        ResolvedPrimitive::Text {
            kind,
            text,
            rotation,
            hide,
            ..
        } => {
            assert_eq!(*kind, TextKind::User);
            assert_eq!(text, "HELLO");
            assert_eq!(*rotation, 90.0);
            assert!(*hide);
        }
        _ => panic!("expected text primitive"),
    }
}
