use ergogen_export::dxf::{Arc, Circle, Dxf, Entity, Line, LwPolyline, Point2, Unsupported};
use ergogen_export::svg::{SvgError, svg_from_dxf, svg_from_lines};

fn line(a: (f64, f64), b: (f64, f64)) -> Line {
    Line {
        start: Point2 { x: a.0, y: a.1 },
        end: Point2 { x: b.0, y: b.1 },
    }
}

fn extract_path(svg: &str) -> &str {
    let needle = "<path d=\"";
    let start = svg
        .find(needle)
        .map(|idx| idx + needle.len())
        .expect("missing path");
    let rest = &svg[start..];
    let end = rest.find('"').expect("missing path end");
    &rest[..end]
}

fn count_command(path: &str, cmd: &str) -> usize {
    path.split_whitespace().filter(|tok| *tok == cmd).count()
}

fn parse_svg_path(path: &str) -> Vec<Line> {
    let tokens: Vec<&str> = path.split_whitespace().collect();
    let mut i = 0usize;
    let mut lines = Vec::new();
    let mut current: Option<Point2> = None;
    let mut start: Option<Point2> = None;
    while i < tokens.len() {
        match tokens[i] {
            "M" => {
                let x = tokens[i + 1].parse::<f64>().unwrap();
                let y = tokens[i + 2].parse::<f64>().unwrap();
                let p = Point2 { x, y };
                current = Some(p);
                start = Some(p);
                i += 3;
            }
            "L" => {
                let x = tokens[i + 1].parse::<f64>().unwrap();
                let y = tokens[i + 2].parse::<f64>().unwrap();
                let next = Point2 { x, y };
                if let Some(cur) = current {
                    lines.push(Line {
                        start: cur,
                        end: next,
                    });
                }
                current = Some(next);
                i += 3;
            }
            "Z" => {
                if let (Some(cur), Some(s)) = (current, start) {
                    lines.push(Line { start: cur, end: s });
                    current = Some(s);
                }
                i += 1;
            }
            other => panic!("unsupported svg path command: {other}"),
        }
    }
    lines
}

fn normalize_lines(lines: &[Line]) -> Vec<(i64, i64, i64, i64)> {
    let eps = 1e-3;
    let quant = |v: f64| (v / eps).round() as i64;
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let mut a = (quant(line.start.x), quant(line.start.y));
        let mut b = (quant(line.end.x), quant(line.end.y));
        if a == b {
            continue;
        }
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        out.push((a.0, a.1, b.0, b.1));
    }
    out.sort();
    out
}

fn transform_to_svg(lines: &[Line]) -> Vec<Line> {
    let mut min_x = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for line in lines {
        for p in [line.start, line.end] {
            min_x = min_x.min(p.x);
            max_y = max_y.max(p.y);
        }
    }
    lines
        .iter()
        .map(|line| Line {
            start: Point2 {
                x: line.start.x - min_x,
                y: max_y - line.start.y,
            },
            end: Point2 {
                x: line.end.x - min_x,
                y: max_y - line.end.y,
            },
        })
        .collect()
}

#[test]
fn svg_from_lines_emits_expected_square() {
    let lines = vec![
        line((0.0, 0.0), (2.0, 0.0)),
        line((2.0, 0.0), (2.0, 2.0)),
        line((2.0, 2.0), (0.0, 2.0)),
        line((0.0, 2.0), (0.0, 0.0)),
    ];

    let got = svg_from_lines(&lines).unwrap();
    let expected = "<svg width=\"2mm\" height=\"2mm\" viewBox=\"0 0 2 2\" xmlns=\"http://www.w3.org/2000/svg\"><g id=\"svgGroup\" stroke-linecap=\"round\" fill-rule=\"evenodd\" font-size=\"9pt\" stroke=\"#000\" stroke-width=\"0.25mm\" fill=\"none\" style=\"stroke:#000;stroke-width:0.25mm;fill:none\"><path d=\"M 0 2 L 2 2 L 2 0 L 0 0 L 0 2 Z\" vector-effect=\"non-scaling-stroke\"/></g></svg>";
    assert_eq!(got, expected);
}

#[test]
fn svg_from_lines_emits_multiple_subpaths() {
    let lines = vec![
        line((0.0, 0.0), (2.0, 0.0)),
        line((2.0, 0.0), (2.0, 2.0)),
        line((2.0, 2.0), (0.0, 2.0)),
        line((0.0, 2.0), (0.0, 0.0)),
        line((3.0, 0.0), (5.0, 0.0)),
        line((5.0, 0.0), (5.0, 2.0)),
        line((5.0, 2.0), (3.0, 2.0)),
        line((3.0, 2.0), (3.0, 0.0)),
    ];

    let got = svg_from_lines(&lines).unwrap();
    let expected = "<svg width=\"5mm\" height=\"2mm\" viewBox=\"0 0 5 2\" xmlns=\"http://www.w3.org/2000/svg\"><g id=\"svgGroup\" stroke-linecap=\"round\" fill-rule=\"evenodd\" font-size=\"9pt\" stroke=\"#000\" stroke-width=\"0.25mm\" fill=\"none\" style=\"stroke:#000;stroke-width:0.25mm;fill:none\"><path d=\"M 0 2 L 2 2 L 2 0 L 0 0 L 0 2 Z M 3 2 L 5 2 L 5 0 L 3 0 L 3 2 Z\" vector-effect=\"non-scaling-stroke\"/></g></svg>";
    assert_eq!(got, expected);
}

#[test]
fn svg_from_lines_joins_and_closes_within_epsilon() {
    let lines = vec![
        line((0.0, 0.0), (1.0, 0.0)),
        line((1.0, 0.0), (1.0, 1.0)),
        line((1.0, 1.0), (0.0, 1.0)),
        line((0.0, 1.0), (0.0005, 0.0)),
    ];

    let got = svg_from_lines(&lines).unwrap();
    let path = extract_path(&got);
    assert_eq!(count_command(path, "M"), 1);
    assert!(path.ends_with('Z'), "expected closed path");
}

#[test]
fn svg_from_dxf_matches_svg_from_lines() {
    let lines = vec![
        line((0.0, 0.0), (2.0, 0.0)),
        line((2.0, 0.0), (2.0, 2.0)),
        line((2.0, 2.0), (0.0, 2.0)),
        line((0.0, 2.0), (0.0, 0.0)),
    ];

    let dxf = Dxf {
        entities: lines.iter().cloned().map(Entity::Line).collect(),
    };

    let got = svg_from_dxf(&dxf).unwrap();
    let expected = svg_from_lines(&lines).unwrap();
    assert_eq!(got, expected);
}

#[test]
fn svg_roundtrips_to_lines_semantically() {
    let lines = vec![
        line((0.0, 0.0), (2.0, 0.0)),
        line((2.0, 0.0), (2.0, 1.0)),
        line((2.0, 1.0), (0.5, 1.5)),
    ];

    let svg = svg_from_lines(&lines).unwrap();
    let path = extract_path(&svg);
    let parsed = parse_svg_path(path);

    let transformed = transform_to_svg(&lines);
    assert_eq!(normalize_lines(&transformed), normalize_lines(&parsed));
}

#[test]
fn svg_roundtrips_closed_path_semantically() {
    let lines = vec![
        line((0.0, 0.0), (2.0, 0.0)),
        line((2.0, 0.0), (2.0, 2.0)),
        line((2.0, 2.0), (0.0, 2.0)),
        line((0.0, 2.0), (0.0, 0.0)),
    ];

    let svg = svg_from_lines(&lines).unwrap();
    let path = extract_path(&svg);
    let parsed = parse_svg_path(path);
    let transformed = transform_to_svg(&lines);

    assert_eq!(normalize_lines(&transformed), normalize_lines(&parsed));
}

#[test]
fn svg_roundtrips_multiple_subpaths_semantically() {
    let lines = vec![
        line((0.0, 0.0), (2.0, 0.0)),
        line((2.0, 0.0), (2.0, 2.0)),
        line((2.0, 2.0), (0.0, 2.0)),
        line((0.0, 2.0), (0.0, 0.0)),
        line((3.0, 0.0), (5.0, 0.0)),
        line((5.0, 0.0), (5.0, 2.0)),
        line((5.0, 2.0), (3.0, 2.0)),
        line((3.0, 2.0), (3.0, 0.0)),
    ];

    let svg = svg_from_lines(&lines).unwrap();
    let path = extract_path(&svg);
    let parsed = parse_svg_path(path);
    let transformed = transform_to_svg(&lines);

    assert_eq!(normalize_lines(&transformed), normalize_lines(&parsed));
}

#[test]
fn svg_from_lines_handles_mixed_coordinate_ranges() {
    let lines = vec![
        line((-10.0, -5.0), (5.0, -5.0)),
        line((5.0, -5.0), (5.0, 15.0)),
        line((5.0, 15.0), (-10.0, 15.0)),
        line((-10.0, 15.0), (-10.0, -5.0)),
    ];

    let svg = svg_from_lines(&lines).unwrap();
    assert!(svg.contains("width=\"15mm\""));
    assert!(svg.contains("height=\"20mm\""));
    assert!(svg.contains("viewBox=\"0 0 15 20\""));

    let path = extract_path(&svg);
    let parsed = parse_svg_path(path);
    let transformed = transform_to_svg(&lines);
    assert_eq!(normalize_lines(&transformed), normalize_lines(&parsed));
}

#[test]
fn svg_from_dxf_rejects_non_line_entities() {
    let dxf = Dxf {
        entities: vec![Entity::Unsupported(Unsupported {
            kind: "SPLINE".to_string(),
        })],
    };
    let err = svg_from_dxf(&dxf).unwrap_err();
    match err {
        SvgError::UnsupportedEntity { .. } => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn svg_from_dxf_supports_arcs_and_circles() {
    let dxf = Dxf {
        entities: vec![
            Entity::Arc(Arc {
                center: Point2 { x: 0.0, y: 0.0 },
                radius: 5.0,
                start_angle_deg: 0.0,
                end_angle_deg: 180.0,
            }),
            Entity::Circle(Circle {
                center: Point2 { x: 15.0, y: 0.0 },
                radius: 2.0,
            }),
        ],
    };
    let svg = svg_from_dxf(&dxf).unwrap();
    assert!(svg.contains("A "));
    assert!(svg.contains("viewBox=\"0 0 22 7\""));
}

#[test]
fn svg_from_dxf_supports_lwpolyline_bulges() {
    let dxf = Dxf {
        entities: vec![Entity::LwPolyline(LwPolyline {
            vertices: vec![Point2 { x: 0.0, y: 0.0 }, Point2 { x: 10.0, y: 0.0 }],
            bulges: vec![1.0, 0.0],
            closed: false,
        })],
    };
    let svg = svg_from_dxf(&dxf).unwrap();
    assert!(svg.contains("A "));
}

#[test]
fn svg_from_lines_rejects_empty_inputs() {
    let err = svg_from_lines(&[]).unwrap_err();
    match err {
        SvgError::Empty => {}
        other => panic!("unexpected error: {other:?}"),
    }
}
