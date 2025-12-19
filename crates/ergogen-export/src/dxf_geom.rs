use std::f64::consts::PI;

use cavalier_contours::core::math::Vector2;
use cavalier_contours::polyline::{PlineSource, seg_arc_radius_and_center};
use ergogen_geometry::{PlineVertex, Polyline, region::Region};

use crate::dxf::{Arc, Circle, Dxf, Entity, Line, Point2};

#[derive(Debug, thiserror::Error)]
pub enum DxfGeomError {
    #[error("polyline must be closed with at least 2 vertices")]
    InvalidPolyline,
}

pub fn dxf_from_region(region: &Region) -> Result<Dxf, DxfGeomError> {
    let mut entities: Vec<Entity> = Vec::new();
    for p in &region.pos {
        entities.extend(entities_from_polyline(p)?);
    }
    for p in &region.neg {
        entities.extend(entities_from_polyline(p)?);
    }
    Ok(Dxf { entities })
}

pub fn entities_from_polyline(pline: &Polyline<f64>) -> Result<Vec<Entity>, DxfGeomError> {
    if !pline.is_closed() || pline.vertex_count() < 2 {
        return Err(DxfGeomError::InvalidPolyline);
    }

    // Build in polyline order, so we can merge adjacent arc segments that are actually a single
    // continuous arc on the same circle (this matters for upstream fixture parity, where MakerJS
    // tends to emit longer arcs than our bulge-segment representation).
    let mut out: Vec<Entity> = Vec::new();

    for i in 0..pline.vertex_count() {
        let v1 = pline.at(i);
        let v2 = pline.at((i + 1) % pline.vertex_count());

        if v1.bulge_is_zero() {
            let line = Line {
                start: Point2 { x: v1.x, y: v1.y },
                end: Point2 { x: v2.x, y: v2.y },
            };

            if let Some(Entity::Line(prev)) = out.last_mut()
                && can_merge_adjacent_lines(prev, &line)
            {
                prev.end = line.end;
                continue;
            }

            out.push(Entity::Line(line));
            continue;
        }

        let (radius, center) = seg_arc_radius_and_center(v1, v2);
        let (start, end) = arc_angles_for_segment(v1, v2, center);

        let arc = Arc {
            center: Point2 {
                x: center.x,
                y: center.y,
            },
            radius,
            start_angle_deg: start,
            end_angle_deg: end,
        };

        if let Some(Entity::Arc(prev)) = out.last_mut()
            && can_merge_adjacent_arcs(prev, &arc)
        {
            prev.end_angle_deg = arc.end_angle_deg;
            continue;
        }

        out.push(Entity::Arc(arc));
    }

    // Merge wrap-around lines if the polyline started mid-segment.
    if out.len() >= 2 {
        let last_start_to_merge = {
            let len = out.len();
            let (head, tail) = out.split_at_mut(len - 1);
            let first = &mut head[0];
            let last = &tail[0];
            match (first, last) {
                (Entity::Line(first), Entity::Line(last)) if can_merge_wrap_lines(first, last) => {
                    Some(last.start)
                }
                _ => None,
            }
        };

        if let Some(last_start) = last_start_to_merge {
            out.pop();
            if let Entity::Line(first) = &mut out[0] {
                first.start = last_start;
            }
        }
    }

    // If our bulge-based circle representation merged into a single ARC with start==end, emit a
    // DXF CIRCLE instead to match upstream fixtures.
    for e in &mut out {
        let Entity::Arc(a) = e else { continue };
        if arc_is_full_circle(a) {
            *e = entities_from_circle((a.center.x, a.center.y), a.radius);
        }
    }

    Ok(out)
}

pub fn entities_from_circle(center: (f64, f64), radius: f64) -> Entity {
    Entity::Circle(Circle {
        center: Point2 {
            x: center.0,
            y: center.1,
        },
        radius,
    })
}

fn arc_angles_for_segment(
    v1: PlineVertex<f64>,
    v2: PlineVertex<f64>,
    center: Vector2<f64>,
) -> (f64, f64) {
    let a1 = angle_deg(center, v1.pos());
    let a2 = angle_deg(center, v2.pos());

    if v1.bulge_is_neg() {
        // CW arc from v1->v2 equals CCW arc from v2->v1.
        (a2, a1)
    } else {
        (a1, a2)
    }
}

fn angle_deg(center: Vector2<f64>, p: Vector2<f64>) -> f64 {
    let dx = p.x - center.x;
    let dy = p.y - center.y;
    let mut deg = dy.atan2(dx) * 180.0 / PI;
    if deg < 0.0 {
        deg += 360.0;
    }
    deg
}

fn can_merge_adjacent_arcs(a: &Arc, b: &Arc) -> bool {
    const EPS: f64 = 1e-6;

    if (a.center.x - b.center.x).abs() > EPS || (a.center.y - b.center.y).abs() > EPS {
        return false;
    }
    if (a.radius - b.radius).abs() > EPS {
        return false;
    }

    let a_end = arc_point(a.center, a.radius, a.end_angle_deg);
    let b_start = arc_point(b.center, b.radius, b.start_angle_deg);
    (a_end.x - b_start.x).abs() <= EPS && (a_end.y - b_start.y).abs() <= EPS
}

fn arc_point(center: Point2, radius: f64, angle_deg: f64) -> Point2 {
    let rad = angle_deg * PI / 180.0;
    let (s, c) = rad.sin_cos();
    Point2 {
        x: center.x + radius * c,
        y: center.y + radius * s,
    }
}

fn arc_is_full_circle(a: &Arc) -> bool {
    const EPS: f64 = 1e-6;

    let start = arc_point(a.center, a.radius, a.start_angle_deg);
    let end = arc_point(a.center, a.radius, a.end_angle_deg);
    (start.x - end.x).abs() <= EPS && (start.y - end.y).abs() <= EPS
}

fn can_merge_adjacent_lines(a: &mut Line, b: &Line) -> bool {
    const EPS: f64 = 1e-6;
    if (a.end.x - b.start.x).abs() > EPS || (a.end.y - b.start.y).abs() > EPS {
        return false;
    }
    // Check collinearity of (a.start -> a.end) and (a.end -> b.end).
    let ax = a.end.x - a.start.x;
    let ay = a.end.y - a.start.y;
    let bx = b.end.x - b.start.x;
    let by = b.end.y - b.start.y;
    let cross = ax * by - ay * bx;
    cross.abs() <= EPS
}

fn can_merge_wrap_lines(first: &Line, last: &Line) -> bool {
    const EPS: f64 = 1e-6;
    if (last.end.x - first.start.x).abs() > EPS || (last.end.y - first.start.y).abs() > EPS {
        return false;
    }
    // Check collinearity of (last.start -> first.start) and (first.start -> first.end).
    let ax = first.start.x - last.start.x;
    let ay = first.start.y - last.start.y;
    let bx = first.end.x - first.start.x;
    let by = first.end.y - first.start.y;
    let cross = ax * by - ay * bx;
    cross.abs() <= EPS
}
