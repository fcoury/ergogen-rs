use indexmap::IndexMap;

use ergogen_core::{Point, PointMeta};
use ergogen_parser::Units;
use ergogen_parser::Value;

use crate::points::{LayoutError, eval_affect, eval_bool_opt, eval_wh, mirror_ref};

pub fn parse_anchor(
    raw: &Value,
    name: &str,
    points: &IndexMap<String, Point>,
    start: Point,
    units: &Units,
    mirror: bool,
) -> Result<Point, LayoutError> {
    match raw {
        Value::String(s) => parse_anchor(
            &Value::Map(IndexMap::from([(
                "ref".to_string(),
                Value::String(s.clone()),
            )])),
            name,
            points,
            start,
            units,
            mirror,
        ),
        Value::Seq(steps) => {
            let mut current = start.clone();
            for (idx, step) in steps.iter().enumerate() {
                current = parse_anchor(
                    step,
                    &format!("{name}[{}]", idx + 1),
                    points,
                    current,
                    units,
                    mirror,
                )?;
            }
            Ok(current)
        }
        Value::Map(m) => parse_anchor_map(m, name, points, start, units, mirror),
        _ => Ok(start),
    }
}

fn parse_anchor_map(
    m: &IndexMap<String, Value>,
    name: &str,
    points: &IndexMap<String, Point>,
    start: Point,
    units: &Units,
    mirror: bool,
) -> Result<Point, LayoutError> {
    let resist = eval_bool_opt(m.get("resist"), &format!("{name}.resist"))?.unwrap_or(false);
    let ctx = AnchorCtx {
        points,
        start: start.clone(),
        units,
        mirror,
        resist,
    };

    let has_ref = m.contains_key("ref");
    let has_agg = m.contains_key("aggregate");
    if has_ref && has_agg {
        return Err(LayoutError::InvalidAnchor {
            at: name.to_string(),
            message: "Fields \"ref\" and \"aggregate\" cannot appear together!".to_string(),
        });
    }

    let mut point = start.clone();
    if let Some(ref_v) = m.get("ref") {
        point = resolve_ref(
            ref_v,
            &format!("{name}.ref"),
            points,
            start.clone(),
            units,
            mirror,
        )?;
    }

    if let Some(agg_v) = m.get("aggregate") {
        point = resolve_aggregate(
            agg_v,
            &format!("{name}.aggregate"),
            points,
            start.clone(),
            units,
            mirror,
        )?;
    }

    if let Some(orient) = m.get("orient") {
        apply_rotator(orient, &format!("{name}.orient"), &ctx, &mut point)?;
    }
    if let Some(shift) = m.get("shift") {
        let xy = eval_wh(units, shift, &format!("{name}.shift"))?;
        point.shift(xy, true, resist);
    }
    if let Some(rot) = m.get("rotate") {
        apply_rotator(rot, &format!("{name}.rotate"), &ctx, &mut point)?;
    }

    if let Some(affect) = m.get("affect") {
        let candidate = point.clone();
        let mut base = start.clone();
        base.meta = candidate.meta.clone();
        let affects = eval_affect(affect, &format!("{name}.affect"))?;
        for a in affects {
            match a {
                'x' => base.x = candidate.x,
                'y' => base.y = candidate.y,
                'r' => base.r = candidate.r,
                _ => {}
            }
        }
        point = base;
    }

    Ok(point)
}

fn resolve_ref(
    raw: &Value,
    name: &str,
    points: &IndexMap<String, Point>,
    start: Point,
    units: &Units,
    mirror: bool,
) -> Result<Point, LayoutError> {
    match raw {
        Value::String(s) => {
            let r = mirror_ref(s, mirror);
            points.get(&r).cloned().ok_or(LayoutError::UnknownPointRef {
                name: r,
                at: name.to_string(),
            })
        }
        other => parse_anchor(other, name, points, start, units, mirror),
    }
}

fn resolve_aggregate(
    raw: &Value,
    name: &str,
    points: &IndexMap<String, Point>,
    start: Point,
    units: &Units,
    mirror: bool,
) -> Result<Point, LayoutError> {
    let Value::Map(m) = raw else {
        return Err(LayoutError::InvalidAnchor {
            at: name.to_string(),
            message: "\"aggregate\" must be an object".to_string(),
        });
    };

    let method = m
        .get("method")
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("average");

    let parts_v = m.get("parts").cloned().unwrap_or(Value::Seq(Vec::new()));
    let Value::Seq(parts) = parts_v else {
        return Err(LayoutError::InvalidAnchor {
            at: format!("{name}.parts"),
            message: "\"parts\" must be an array".to_string(),
        });
    };

    let mut resolved: Vec<Point> = Vec::new();
    for (idx, part) in parts.iter().enumerate() {
        resolved.push(parse_anchor(
            part,
            &format!("{name}.parts[{}]", idx + 1),
            points,
            start.clone(),
            units,
            mirror,
        )?);
    }

    match method {
        "average" => Ok(aggregate_average(&resolved)),
        "intersect" => aggregate_intersect(&resolved, name),
        other => Err(LayoutError::InvalidAnchor {
            at: format!("{name}.method"),
            message: format!("Unknown aggregator method \"{other}\""),
        }),
    }
}

fn aggregate_average(parts: &[Point]) -> Point {
    if parts.is_empty() {
        return Point::new(0.0, 0.0, 0.0, PointMeta::default());
    }
    let len = parts.len() as f64;
    let (mut x, mut y, mut r) = (0.0, 0.0, 0.0);
    for p in parts {
        x += p.x;
        y += p.y;
        r += p.r;
    }
    Point::new(x / len, y / len, r / len, PointMeta::default())
}

fn aggregate_intersect(parts: &[Point], name: &str) -> Result<Point, LayoutError> {
    if parts.len() != 2 {
        return Err(LayoutError::InvalidAnchor {
            at: format!("{name}.parts"),
            message: format!(
                "Intersect expects exactly two parts, but it got {}!",
                parts.len()
            ),
        });
    }
    let p1 = &parts[0];
    let p2 = &parts[1];

    let d1 = rotate_vec([0.0, 1.0], p1.r);
    let d2 = rotate_vec([0.0, 1.0], p2.r);

    let denom = cross(d1, d2);
    if denom.abs() < 1e-12 {
        return Err(LayoutError::InvalidAnchor {
            at: format!("{name}.parts"),
            message: format!("The points under \"{name}.parts\" do not intersect!"),
        });
    }

    let delta = [p2.x - p1.x, p2.y - p1.y];
    let t = cross(delta, d2) / denom;

    let x = p1.x + t * d1[0];
    let y = p1.y + t * d1[1];
    Ok(Point::new(x, y, 0.0, PointMeta::default()))
}

fn rotate_vec(v: [f64; 2], angle_deg: f64) -> [f64; 2] {
    let a = angle_deg.to_radians();
    let (s, c) = a.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c]
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

struct AnchorCtx<'a> {
    points: &'a IndexMap<String, Point>,
    start: Point,
    units: &'a Units,
    mirror: bool,
    resist: bool,
}

fn apply_rotator(
    config: &Value,
    name: &str,
    ctx: &AnchorCtx<'_>,
    point: &mut Point,
) -> Result<(), LayoutError> {
    // Upstream behavior:
    // - Numbers (or numeric expressions) add to rotation
    // - Otherwise, treat config as an anchor and "turn towards" it.
    match config {
        Value::Number(n) => {
            point.rotate(*n, None, ctx.resist);
            Ok(())
        }
        Value::String(s) => match ctx.units.eval(name, s) {
            Ok(angle) => {
                point.rotate(angle, None, ctx.resist);
                Ok(())
            }
            Err(_) => {
                // Treat as an anchor reference, e.g. orient: "ten"
                let target = parse_anchor(
                    config,
                    name,
                    ctx.points,
                    ctx.start.clone(),
                    ctx.units,
                    ctx.mirror,
                )?;
                point.r = point.angle_to(&Point::xy(target.x, target.y));
                Ok(())
            }
        },
        _ => {
            let target = parse_anchor(
                config,
                name,
                ctx.points,
                ctx.start.clone(),
                ctx.units,
                ctx.mirror,
            )?;
            point.r = point.angle_to(&Point::xy(target.x, target.y));
            Ok(())
        }
    }
}
