use std::f64::consts::PI;

use cavalier_contours::polyline::PlineSource;

use crate::{PlineVertex, Polyline};

fn rotate_about(x: f64, y: f64, cx: f64, cy: f64, deg: f64) -> (f64, f64) {
    let rad = deg * PI / 180.0;
    let (s, c) = rad.sin_cos();
    let dx = x - cx;
    let dy = y - cy;
    (cx + dx * c - dy * s, cy + dx * s + dy * c)
}

fn bulge_for_quarter_circle() -> f64 {
    (PI / 8.0).tan()
}

pub fn circle(center: (f64, f64), radius: f64) -> Polyline<f64> {
    let (cx, cy) = center;
    let mut pl = Polyline::new_closed();

    // Represent a full circle as two 180° arcs (two vertices, both bulge=1).
    pl.vertex_data.push(PlineVertex::new(cx - radius, cy, 1.0));
    pl.vertex_data.push(PlineVertex::new(cx + radius, cy, 1.0));
    pl
}

pub fn rectangle(center: (f64, f64), size: (f64, f64), rotation_deg: f64) -> Polyline<f64> {
    let (cx, cy) = center;
    let (w, h) = size;
    let hw = w / 2.0;
    let hh = h / 2.0;

    let mut pts = vec![
        (cx - hw, cy - hh),
        (cx + hw, cy - hh),
        (cx + hw, cy + hh),
        (cx - hw, cy + hh),
    ];

    if rotation_deg != 0.0 {
        for p in &mut pts {
            *p = rotate_about(p.0, p.1, cx, cy, rotation_deg);
        }
    }

    let mut pl = Polyline::new_closed();
    for (x, y) in pts {
        pl.vertex_data.push(PlineVertex::new(x, y, 0.0));
    }
    pl
}

pub fn rounded_rectangle(
    center: (f64, f64),
    size: (f64, f64),
    corner_radius: f64,
    rotation_deg: f64,
) -> Polyline<f64> {
    let (cx, cy) = center;
    let (w, h) = size;
    let hw = w / 2.0;
    let hh = h / 2.0;
    let r = corner_radius.min(hw).min(hh).max(0.0);

    if r == 0.0 {
        return rectangle(center, size, rotation_deg);
    }

    let b = bulge_for_quarter_circle();

    // CCW rounded rectangle: 8 vertices (line, arc, line, arc, ...). Bulge lives on the start of
    // the arc segment (vertex -> next vertex).
    let mut pts: Vec<(f64, f64, f64)> = vec![
        (cx + hw - r, cy - hh, b),   // bottom edge to bottom-right corner arc
        (cx + hw, cy - hh + r, 0.0), // right edge
        (cx + hw, cy + hh - r, b),   // right edge to top-right arc
        (cx + hw - r, cy + hh, 0.0), // top edge
        (cx - hw + r, cy + hh, b),   // top edge to top-left arc
        (cx - hw, cy + hh - r, 0.0), // left edge
        (cx - hw, cy - hh + r, b),   // left edge to bottom-left arc
        (cx - hw + r, cy - hh, 0.0), // bottom edge
    ];

    if rotation_deg != 0.0 {
        for p in &mut pts {
            let (x, y) = rotate_about(p.0, p.1, cx, cy, rotation_deg);
            p.0 = x;
            p.1 = y;
        }
    }

    let mut pl = Polyline::new_closed();
    for (x, y, bulge) in pts {
        pl.vertex_data.push(PlineVertex::new(x, y, bulge));
    }
    pl
}

pub fn beveled_rectangle(
    center: (f64, f64),
    size: (f64, f64),
    bevel: f64,
    rotation_deg: f64,
) -> Polyline<f64> {
    let (cx, cy) = center;
    let (w, h) = size;
    let hw = w / 2.0;
    let hh = h / 2.0;
    let b = bevel.min(hw).min(hh).max(0.0);

    if b == 0.0 {
        return rectangle(center, size, rotation_deg);
    }

    let mut pts: Vec<(f64, f64)> = vec![
        (cx + hw - b, cy - hh),
        (cx + hw, cy - hh + b),
        (cx + hw, cy + hh - b),
        (cx + hw - b, cy + hh),
        (cx - hw + b, cy + hh),
        (cx - hw, cy + hh - b),
        (cx - hw, cy - hh + b),
        (cx - hw + b, cy - hh),
    ];

    if rotation_deg != 0.0 {
        for p in &mut pts {
            *p = rotate_about(p.0, p.1, cx, cy, rotation_deg);
        }
    }

    let mut pl = Polyline::new_closed();
    for (x, y) in pts {
        pl.vertex_data.push(PlineVertex::new(x, y, 0.0));
    }
    pl
}

pub fn polygon(vertices: &[(f64, f64)]) -> Polyline<f64> {
    let mut pl = Polyline::new_closed();
    for &(x, y) in vertices {
        pl.vertex_data.push(PlineVertex::new(x, y, 0.0));
    }
    pl
}

pub fn is_valid_closed_polyline(pl: &Polyline<f64>) -> bool {
    pl.is_closed && pl.vertex_count() >= 2
}
