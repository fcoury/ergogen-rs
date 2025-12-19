use crate::dxf::{Dxf, Entity, Line, Point2};

#[derive(Debug, thiserror::Error)]
pub enum SvgError {
    #[error("SVG export requires at least one line entity")]
    Empty,
    #[error("SVG export only supports line entities (found {kind})")]
    UnsupportedEntity { kind: String },
}

const EPS: f64 = 1e-3;

pub fn svg_from_dxf(dxf: &Dxf) -> Result<String, SvgError> {
    let mut lines: Vec<Line> = Vec::new();
    for entity in &dxf.entities {
        match entity {
            Entity::Line(line) => lines.push(*line),
            Entity::Unsupported(unsupported) => {
                return Err(SvgError::UnsupportedEntity {
                    kind: unsupported.kind.clone(),
                });
            }
            other => {
                return Err(SvgError::UnsupportedEntity {
                    kind: format!("{other:?}"),
                });
            }
        }
    }
    svg_from_lines(&lines)
}

pub fn svg_from_lines(lines: &[Line]) -> Result<String, SvgError> {
    if lines.is_empty() {
        return Err(SvgError::Empty);
    }

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for line in lines {
        for p in [line.start, line.end] {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
    }

    let width = max_x - min_x;
    let height = max_y - min_y;

    let transform = |p: Point2| Point2 {
        x: p.x - min_x,
        y: max_y - p.y,
    };

    let mut path = String::new();
    let mut current: Option<Point2> = None;
    let mut sub_start: Option<Point2> = None;

    for line in lines {
        let start = transform(line.start);
        let end = transform(line.end);

        let need_move = current.map_or(true, |p| !close(p, start));
        if need_move {
            if !path.is_empty() {
                path.push(' ');
            }
            path.push_str(&format!("M {} {}", fmt_num(start.x), fmt_num(start.y)));
            sub_start = Some(start);
        }

        path.push_str(&format!(" L {} {}", fmt_num(end.x), fmt_num(end.y)));
        current = Some(end);

        if let Some(s) = sub_start {
            if close(end, s) {
                path.push_str(" Z");
                current = None;
                sub_start = None;
            }
        }
    }

    Ok(format!(
        "<svg width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\" xmlns=\"http://www.w3.org/2000/svg\"><g id=\"svgGroup\" stroke-linecap=\"round\" fill-rule=\"evenodd\" font-size=\"9pt\" stroke=\"#000\" stroke-width=\"0.25mm\" fill=\"none\" style=\"stroke:#000;stroke-width:0.25mm;fill:none\"><path d=\"{path}\" vector-effect=\"non-scaling-stroke\"/></g></svg>",
        w = fmt_num(width),
        h = fmt_num(height),
        path = path
    ))
}

fn close(a: Point2, b: Point2) -> bool {
    (a.x - b.x).abs() <= EPS && (a.y - b.y).abs() <= EPS
}

fn fmt_num(v: f64) -> String {
    let v = if v.abs() < 1e-9 { 0.0 } else { v };
    let mut buf = ryu::Buffer::new();
    let s = buf.format(v);
    s.strip_suffix(".0").unwrap_or(s).to_string()
}
