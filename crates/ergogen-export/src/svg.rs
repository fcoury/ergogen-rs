use cavalier_contours::core::math::angle_from_bulge;
use cavalier_contours::polyline::{PlineVertex, seg_arc_radius_and_center, seg_bounding_box};

use crate::dxf::{Arc, Circle, Dxf, Entity, Line, LwPolyline, Point2};

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
    let mut has_non_line = false;
    for entity in &dxf.entities {
        match entity {
            Entity::Line(line) => lines.push(*line),
            Entity::Circle(_) | Entity::Arc(_) | Entity::LwPolyline(_) => {
                has_non_line = true;
            }
            Entity::Unsupported(unsupported) => {
                return Err(SvgError::UnsupportedEntity {
                    kind: unsupported.kind.clone(),
                });
            }
        }
    }
    if !has_non_line {
        return svg_from_lines(&lines);
    }

    svg_from_entities(dxf)
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

#[derive(Debug, Clone, Copy)]
struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl Bounds {
    fn new() -> Self {
        Self {
            min_x: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            min_y: f64::INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }

    fn update_point(&mut self, p: Point2) {
        self.min_x = self.min_x.min(p.x);
        self.max_x = self.max_x.max(p.x);
        self.min_y = self.min_y.min(p.y);
        self.max_y = self.max_y.max(p.y);
    }

    fn update_bbox(&mut self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
        self.min_x = self.min_x.min(min_x);
        self.max_x = self.max_x.max(max_x);
        self.min_y = self.min_y.min(min_y);
        self.max_y = self.max_y.max(max_y);
    }

    fn is_valid(&self) -> bool {
        self.min_x.is_finite()
            && self.max_x.is_finite()
            && self.min_y.is_finite()
            && self.max_y.is_finite()
    }
}

fn svg_from_entities(dxf: &Dxf) -> Result<String, SvgError> {
    let bounds = bounds_for_entities(dxf)?;
    if !bounds.is_valid() {
        return Err(SvgError::Empty);
    }

    let width = bounds.max_x - bounds.min_x;
    let height = bounds.max_y - bounds.min_y;
    if width <= 0.0 || height <= 0.0 {
        return Err(SvgError::Empty);
    }

    let transform = |p: Point2| Point2 {
        x: p.x - bounds.min_x,
        y: bounds.max_y - p.y,
    };

    let mut path = String::new();

    for entity in &dxf.entities {
        match entity {
            Entity::Line(line) => {
                push_subpath(&mut path, &line_path(*line, transform));
            }
            Entity::Arc(arc) => {
                push_subpath(&mut path, &arc_path(*arc, transform));
            }
            Entity::Circle(circle) => {
                push_subpath(&mut path, &circle_path(*circle, transform));
            }
            Entity::LwPolyline(poly) => {
                push_subpath(&mut path, &polyline_path(poly, transform));
            }
            Entity::Unsupported(unsupported) => {
                return Err(SvgError::UnsupportedEntity {
                    kind: unsupported.kind.clone(),
                });
            }
        }
    }

    if path.is_empty() {
        return Err(SvgError::Empty);
    }

    Ok(format!(
        "<svg width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\" xmlns=\"http://www.w3.org/2000/svg\"><g id=\"svgGroup\" stroke-linecap=\"round\" fill-rule=\"evenodd\" font-size=\"9pt\" stroke=\"#000\" stroke-width=\"0.25mm\" fill=\"none\" style=\"stroke:#000;stroke-width:0.25mm;fill:none\"><path d=\"{path}\" vector-effect=\"non-scaling-stroke\"/></g></svg>",
        w = fmt_num(width),
        h = fmt_num(height),
        path = path
    ))
}

fn push_subpath(path: &mut String, sub: &str) {
    if sub.is_empty() {
        return;
    }
    if !path.is_empty() {
        path.push(' ');
    }
    path.push_str(sub);
}

fn line_path(line: Line, transform: impl Fn(Point2) -> Point2) -> String {
    let start = transform(line.start);
    let end = transform(line.end);
    format!("M {} {} L {} {}", fmt_num(start.x), fmt_num(start.y), fmt_num(end.x), fmt_num(end.y))
}

fn arc_path(arc: Arc, transform: impl Fn(Point2) -> Point2) -> String {
    let start = arc_point(arc.center, arc.radius, arc.start_angle_deg);
    let end = arc_point(arc.center, arc.radius, arc.end_angle_deg);
    let start_t = transform(start);
    let end_t = transform(end);
    let sweep = arc_sweep_ccw(arc.start_angle_deg, arc.end_angle_deg);
    let large = if sweep > 180.0 { 1 } else { 0 };
    // Y-axis flip turns CCW into CW.
    let sweep_flag = 0;
    let r = arc.radius.abs();
    format!(
        "M {} {} A {} {} 0 {} {} {} {}",
        fmt_num(start_t.x),
        fmt_num(start_t.y),
        fmt_num(r),
        fmt_num(r),
        large,
        sweep_flag,
        fmt_num(end_t.x),
        fmt_num(end_t.y)
    )
}

fn circle_path(circle: Circle, transform: impl Fn(Point2) -> Point2) -> String {
    let r = circle.radius.abs();
    let start = Point2 {
        x: circle.center.x + r,
        y: circle.center.y,
    };
    let mid = Point2 {
        x: circle.center.x - r,
        y: circle.center.y,
    };
    let start_t = transform(start);
    let mid_t = transform(mid);
    let end_t = start_t;
    // Two half-arcs. Y-axis flip uses sweep_flag 0.
    format!(
        "M {} {} A {} {} 0 0 0 {} {} A {} {} 0 0 0 {} {}",
        fmt_num(start_t.x),
        fmt_num(start_t.y),
        fmt_num(r),
        fmt_num(r),
        fmt_num(mid_t.x),
        fmt_num(mid_t.y),
        fmt_num(r),
        fmt_num(r),
        fmt_num(end_t.x),
        fmt_num(end_t.y)
    )
}

fn polyline_path(poly: &LwPolyline, transform: impl Fn(Point2) -> Point2) -> String {
    if poly.vertices.len() < 2 {
        return String::new();
    }
    let mut out = String::new();
    let start = transform(poly.vertices[0]);
    out.push_str(&format!("M {} {}", fmt_num(start.x), fmt_num(start.y)));

    for i in 0..poly.vertices.len() {
        let next = if i + 1 < poly.vertices.len() {
            i + 1
        } else if poly.closed {
            0
        } else {
            break;
        };
        let v1 = poly.vertices[i];
        let v2 = poly.vertices[next];
        let bulge = poly.bulges.get(i).copied().unwrap_or(0.0);
        if bulge.abs() <= EPS {
            let end = transform(v2);
            out.push_str(&format!(" L {} {}", fmt_num(end.x), fmt_num(end.y)));
        } else {
            let v1_seg = PlineVertex::new(v1.x, v1.y, bulge);
            let v2_seg = PlineVertex::new(v2.x, v2.y, 0.0);
            let (radius, _) = seg_arc_radius_and_center(v1_seg, v2_seg);
            let bulge_svg = -bulge; // Y-flip in SVG space
            let angle = angle_from_bulge(bulge_svg).abs().to_degrees();
            let large = if angle > 180.0 { 1 } else { 0 };
            let sweep_flag = if bulge_svg >= 0.0 { 1 } else { 0 };
            let end = transform(v2);
            let r = radius.abs();
            out.push_str(&format!(
                " A {} {} 0 {} {} {} {}",
                fmt_num(r),
                fmt_num(r),
                large,
                sweep_flag,
                fmt_num(end.x),
                fmt_num(end.y)
            ));
        }
    }

    if poly.closed {
        out.push_str(" Z");
    }
    out
}

fn bounds_for_entities(dxf: &Dxf) -> Result<Bounds, SvgError> {
    let mut bounds = Bounds::new();

    for entity in &dxf.entities {
        match entity {
            Entity::Line(line) => {
                bounds.update_point(line.start);
                bounds.update_point(line.end);
            }
            Entity::Circle(circle) => {
                bounds.update_bbox(
                    circle.center.x - circle.radius.abs(),
                    circle.center.y - circle.radius.abs(),
                    circle.center.x + circle.radius.abs(),
                    circle.center.y + circle.radius.abs(),
                );
            }
            Entity::Arc(arc) => {
                update_bounds_for_arc(&mut bounds, *arc);
            }
            Entity::LwPolyline(poly) => {
                update_bounds_for_polyline(&mut bounds, poly);
            }
            Entity::Unsupported(unsupported) => {
                return Err(SvgError::UnsupportedEntity {
                    kind: unsupported.kind.clone(),
                });
            }
        }
    }

    Ok(bounds)
}

fn update_bounds_for_arc(bounds: &mut Bounds, arc: Arc) {
    let r = arc.radius.abs();
    let mut angles = vec![arc.start_angle_deg, arc.end_angle_deg];
    let start = norm_deg(arc.start_angle_deg);
    let end = norm_deg(arc.end_angle_deg);
    let full_circle = (start - end).abs() < 1e-9;
    let cardinals = [0.0, 90.0, 180.0, 270.0];
    if full_circle {
        angles.extend_from_slice(&cardinals);
    } else {
        for a in cardinals {
            if angle_in_ccw_sweep(start, end, a) {
                angles.push(a);
            }
        }
    }

    for angle in angles {
        let p = arc_point(arc.center, r, angle);
        bounds.update_point(p);
    }
}

fn update_bounds_for_polyline(bounds: &mut Bounds, poly: &LwPolyline) {
    if poly.vertices.len() < 2 {
        return;
    }
    for i in 0..poly.vertices.len() {
        let next = if i + 1 < poly.vertices.len() {
            i + 1
        } else if poly.closed {
            0
        } else {
            break;
        };
        let v1 = poly.vertices[i];
        let v2 = poly.vertices[next];
        let bulge = poly.bulges.get(i).copied().unwrap_or(0.0);
        let v1_seg = PlineVertex::new(v1.x, v1.y, bulge);
        let v2_seg = PlineVertex::new(v2.x, v2.y, 0.0);
        let aabb = seg_bounding_box(v1_seg, v2_seg);
        bounds.update_bbox(aabb.min_x, aabb.min_y, aabb.max_x, aabb.max_y);
    }
}

fn arc_point(center: Point2, radius: f64, angle_deg: f64) -> Point2 {
    let rad = angle_deg.to_radians();
    let (s, c) = rad.sin_cos();
    Point2 {
        x: center.x + radius * c,
        y: center.y + radius * s,
    }
}

fn arc_sweep_ccw(start_deg: f64, end_deg: f64) -> f64 {
    let s = norm_deg(start_deg);
    let e = norm_deg(end_deg);
    if s <= e {
        e - s
    } else {
        360.0 - (s - e)
    }
}

fn norm_deg(mut deg: f64) -> f64 {
    while deg < 0.0 {
        deg += 360.0;
    }
    while deg >= 360.0 {
        deg -= 360.0;
    }
    deg
}

fn angle_in_ccw_sweep(start: f64, end: f64, angle: f64) -> bool {
    let s = norm_deg(start);
    let e = norm_deg(end);
    let a = norm_deg(angle);
    if s <= e {
        a >= s && a <= e
    } else {
        a >= s || a <= e
    }
}
