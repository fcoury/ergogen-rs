//! Outline generation logic.

use indexmap::IndexMap;
use regex::Regex;
use std::collections::HashSet;

use cavalier_contours::polyline::{PlineOffsetOptions, PlineOrientation, PlineSource};
use ergogen_core::{Point, PointMeta};
use ergogen_geometry::region::Region;
use ergogen_geometry::{BooleanOp, Polyline, primitives};
use ergogen_layout::{PointsOutput, anchor, parse_points};
use ergogen_parser::{Error as ParserError, PreparedConfig, Value};

mod hulljs;
mod makerjs_path;

#[derive(Debug, thiserror::Error)]
pub enum OutlineError {
    #[error("failed to parse/prepare config: {0}")]
    Parser(#[from] ParserError),
    #[error("failed to parse points: {0}")]
    Points(#[from] ergogen_layout::LayoutError),
    #[error("outline reference cycle involving \"{name}\"")]
    OutlineCycle { name: String },
    #[error("unsupported outline config: {0}")]
    Unsupported(&'static str),
    #[error("path error: {0}")]
    Path(#[from] makerjs_path::MakerJsPathError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asym {
    Both,
    Source,
    Clone,
}

#[derive(Debug, Clone, Copy)]
struct Placement {
    x: f64,
    y: f64,
    r: f64,
    mirrored: bool,
    bind_trbl: [f64; 4],
}

/// Minimal outline generator for the simplest upstream fixtures.
///
/// Current support:
/// - `what: rectangle`
/// - `where: true` (all points)
/// - `where: <anchor object>` (single/mirrored anchors via `ergogen-layout` anchor parsing)
/// - `size: <number>` or `size: [w, h]` (supports string expressions)
/// - `corner: <number>` (rounded rectangle)
/// - `operation: subtract`
pub fn generate_outline_region_from_yaml_str(
    yaml: &str,
    outline_name: &str,
) -> Result<Region, OutlineError> {
    let prepared = PreparedConfig::from_yaml_str(yaml)?;
    generate_outline_region(&prepared, outline_name)
}

pub fn generate_outline_region(
    prepared: &PreparedConfig,
    outline_name: &str,
) -> Result<Region, OutlineError> {
    let mut visiting = HashSet::<String>::new();
    generate_outline_region_inner(prepared, outline_name, &mut visiting)
}

fn generate_outline_region_inner(
    prepared: &PreparedConfig,
    outline_name: &str,
    visiting: &mut HashSet<String>,
) -> Result<Region, OutlineError> {
    if !visiting.insert(outline_name.to_string()) {
        return Err(OutlineError::OutlineCycle {
            name: outline_name.to_string(),
        });
    }

    let points = parse_points(&prepared.canonical, &prepared.units)?;
    let ref_points = points_to_ref(&points);

    let outline = prepared
        .canonical
        .get_path(&format!("outlines.{outline_name}"))
        .ok_or(OutlineError::Unsupported("missing outlines.<name>"))?;

    let parts: Vec<&Value> = match outline {
        // Full-form: outlines.<name>.<part_name>: { what, where, ... }
        Value::Map(m) => m.values().collect(),
        // Shorthand: outlines.<name>: [ { what, where, ... }, ... ]
        Value::Seq(seq) => seq.iter().collect(),
        _ => {
            return Err(OutlineError::Unsupported(
                "outlines.<name> must be a map or sequence",
            ));
        }
    };

    let mut region = Region::empty();
    let mut stack: Vec<Polyline<f64>> = Vec::new();
    let mut carry_neg: Vec<Polyline<f64>> = Vec::new();

    for part in parts {
        let obj = match part {
            Value::Map(obj) => Some(obj),
            Value::String(s) => {
                let (op, name) = parse_outline_ref(s);
                let referenced = generate_outline_region_inner(prepared, name, visiting)?;

                match op {
                    OutlineRefOp::Subtract => {
                        apply_region_op(
                            &mut region,
                            "subtract",
                            referenced,
                            &mut stack,
                            &mut carry_neg,
                        );
                    }
                    OutlineRefOp::Stack => {
                        apply_region_op(
                            &mut region,
                            "stack",
                            referenced,
                            &mut stack,
                            &mut carry_neg,
                        );
                    }
                    OutlineRefOp::Intersect => {
                        apply_region_op(
                            &mut region,
                            "intersect",
                            referenced,
                            &mut stack,
                            &mut carry_neg,
                        );
                    }
                    OutlineRefOp::Add => {
                        apply_region_op(&mut region, "add", referenced, &mut stack, &mut carry_neg);
                    }
                }

                continue;
            }
            _ => None,
        };

        let Some(obj) = obj else { continue };

        let what = obj
            .get("what")
            .and_then(|v| match v {
                Value::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("");

        let op = obj
            .get("operation")
            .and_then(|v| match v {
                Value::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("add");

        // This is Ergogen's “outline referencing” feature:
        // - explicit: `{ what: outline, name: other, ... }`
        // - shorthand: `{ name: other, expand: ..., joints: ... }`
        if what.is_empty() && obj.contains_key("name") || what == "outline" {
            let name_v = obj
                .get("name")
                .ok_or(OutlineError::Unsupported("missing name"))?;
            let Value::String(name) = name_v else {
                return Err(OutlineError::Unsupported("name must be a string"));
            };

            let mut referenced = generate_outline_region_inner(prepared, name, visiting)?;

            // MakerJS outlines.js applies: scale -> expand -> fillet (per-part).
            let scale = match obj.get("scale") {
                None | Some(Value::Null) => 1.0,
                Some(v) => eval_number(&prepared.units, v, "outlines.scale")?,
            };
            if scale != 1.0 {
                referenced = scale_region(&referenced, scale);
            }

            if obj.contains_key("expand") {
                let (amount, joints) =
                    parse_expand_spec(obj.get("expand"), obj.get("joints"), &prepared.units)?;
                referenced = if amount == 0.0 {
                    referenced
                } else if referenced.pos.len() == 1
                    && referenced.neg.is_empty()
                    && try_rectangle_params(&referenced.pos[0]).is_some()
                {
                    // Preserve our existing "rectangle-only" behavior for fixtures that validate
                    // pointy/beveled joints.
                    expand_region_rect_only(&referenced, amount, joints)?
                } else {
                    // For now, only round joints are supported for general regions (sufficient for
                    // `outlines.yaml`).
                    if joints != ExpandJoints::Round {
                        return Err(OutlineError::Unsupported(
                            "general expand only supports round joints for now",
                        ));
                    }
                    expand_region_round(&referenced, amount)?
                };
            }

            let fillet = match obj.get("fillet") {
                None | Some(Value::Null) => 0.0,
                Some(v) => eval_number(&prepared.units, v, "outlines.fillet")?,
            };
            if fillet != 0.0 {
                referenced = fillet_region_round(&referenced, fillet)?;
            }

            apply_region_op(&mut region, op, referenced, &mut stack, &mut carry_neg);
            continue;
        }

        let where_v = obj.get("where").unwrap_or(&Value::Null);
        let asym = parse_asym(obj.get("asym"), where_v);
        let bound = obj
            .get("bound")
            .and_then(|v| match v {
                Value::Bool(b) => Some(*b),
                _ => None,
            })
            .unwrap_or(false);

        match what {
            "rectangle" => {
                let size = obj
                    .get("size")
                    .ok_or(OutlineError::Unsupported("missing size"))?;
                let (w, h) = parse_size(&prepared.units, size, "outlines.size")?;

                let mut corner = match obj.get("corner") {
                    None | Some(Value::Null) => 0.0,
                    Some(v) => eval_number(&prepared.units, v, "outlines.corner")?,
                };
                let mut corner_from_fillet = false;
                if corner == 0.0 {
                    if let Some(Value::Null) = obj.get("fillet") {
                        // noop
                    } else if let Some(v) = obj.get("fillet") {
                        corner = eval_number(&prepared.units, v, "outlines.fillet")?;
                        corner_from_fillet = corner > 0.0;
                    }
                }
                let bevel = match obj.get("bevel") {
                    None | Some(Value::Null) => 0.0,
                    Some(v) => eval_number(&prepared.units, v, "outlines.bevel")?,
                };
                let bevel = if bevel > 0.0 {
                    bevel.next_down()
                } else {
                    bevel
                };
                let corner = if corner_from_fillet {
                    corner.next_up()
                } else {
                    corner
                };

                // Ergogen provides `sx`/`sy` as the shape size in the expression env for anchor math
                // within outlines.
                let units = prepared
                    .units
                    .with_extra_vars([("sx".to_string(), w), ("sy".to_string(), h)]);

                let placements = placements_for_where(where_v, asym, &points, &ref_points, &units)?;
                for p in placements {
                    let p = apply_adjust_if_present(obj.get("adjust"), p, &ref_points, &units)?;
                    let (cx, cy, w, h) = if bound {
                        apply_bind_to_centered_rect((p.x, p.y), (w, h), p.bind_trbl, p.r)
                    } else {
                        (p.x, p.y, w, h)
                    };
                    let rect = if bevel > 0.0 {
                        primitives::beveled_rectangle((cx, cy), (w, h), bevel, p.r)
                    } else if corner > 0.0 {
                        primitives::rounded_rectangle((cx, cy), (w, h), corner, p.r)
                    } else {
                        primitives::rectangle((cx, cy), (w, h), p.r)
                    };
                    apply_region_op(
                        &mut region,
                        op,
                        Region::from_pos(vec![rect]),
                        &mut stack,
                        &mut carry_neg,
                    );
                }
            }
            "circle" => {
                let radius_v = obj
                    .get("radius")
                    .ok_or(OutlineError::Unsupported("missing radius"))?;
                let radius = eval_number(&prepared.units, radius_v, "outlines.radius")?;

                // Circles still provide `sx`/`sy` for compatibility, though most fixtures won't
                // reference them.
                let units = prepared.units.with_extra_vars([
                    ("r".to_string(), radius),
                    ("sx".to_string(), radius * 2.0),
                    ("sy".to_string(), radius * 2.0),
                ]);

                let placements = placements_for_where(where_v, asym, &points, &ref_points, &units)?;
                for p in placements {
                    let p = apply_adjust_if_present(obj.get("adjust"), p, &ref_points, &units)?;
                    let c = primitives::circle((p.x, p.y), radius);
                    apply_region_op(
                        &mut region,
                        op,
                        Region::from_pos(vec![c]),
                        &mut stack,
                        &mut carry_neg,
                    );
                }
            }
            "polygon" => {
                let points_v = obj
                    .get("points")
                    .ok_or(OutlineError::Unsupported("missing points"))?;
                let Value::Seq(steps) = points_v else {
                    return Err(OutlineError::Unsupported("points must be a sequence"));
                };

                let placements =
                    placements_for_where(where_v, asym, &points, &ref_points, &prepared.units)?;

                for p in placements {
                    let p = apply_adjust_if_present(
                        obj.get("adjust"),
                        p,
                        &ref_points,
                        &prepared.units,
                    )?;
                    let mut current = Point::new(
                        p.x,
                        p.y,
                        p.r,
                        PointMeta {
                            mirrored: p.mirrored,
                        },
                    );
                    let mut vertices: Vec<(f64, f64)> = Vec::with_capacity(steps.len());

                    for (idx, step) in steps.iter().enumerate() {
                        current = anchor::parse_anchor(
                            step,
                            &format!("outlines.points[{}]", idx + 1),
                            &ref_points,
                            current,
                            &prepared.units,
                            false,
                        )?;
                        vertices.push((current.x, current.y));
                    }

                    let poly = primitives::polygon(&vertices);
                    apply_region_op(
                        &mut region,
                        op,
                        Region::from_pos(vec![poly]),
                        &mut stack,
                        &mut carry_neg,
                    );
                }
            }
            "hull" => {
                let concavity = match obj.get("concavity") {
                    None | Some(Value::Null) => 50.0,
                    Some(v) => eval_number(&prepared.units, v, "outlines.concavity")?,
                };
                // Upstream defaults `extend` to true when missing.
                let extend = match obj.get("extend") {
                    None | Some(Value::Null) => true,
                    Some(Value::Bool(b)) => *b,
                    _ => true,
                };
                let hull_points_v = obj
                    .get("points")
                    .ok_or(OutlineError::Unsupported("missing points"))?;
                let Value::Seq(hull_points) = hull_points_v else {
                    return Err(OutlineError::Unsupported("points must be a sequence"));
                };

                let placements =
                    placements_for_where(where_v, asym, &points, &ref_points, &prepared.units)?;

                for p in placements {
                    let p = apply_adjust_if_present(
                        obj.get("adjust"),
                        p,
                        &ref_points,
                        &prepared.units,
                    )?;

                    let mut samples: Vec<[f64; 2]> = Vec::new();
                    let mut last = AnchorWithKeyMeta {
                        point: Point::new(
                            0.0,
                            0.0,
                            0.0,
                            PointMeta {
                                mirrored: p.mirrored,
                            },
                        ),
                        width: 0.0,
                        height: 0.0,
                    };

                    for (idx, hp) in hull_points.iter().enumerate() {
                        last = parse_anchor_with_key_meta(
                            hp,
                            &format!("outlines.hull.points[{}]", idx + 1),
                            &points,
                            &ref_points,
                            last,
                            &prepared.units,
                        )?;

                        if !extend {
                            samples.push([last.point.x, last.point.y]);
                            continue;
                        }

                        let w = last.width;
                        let h = last.height;
                        if w == 0.0 && h == 0.0 {
                            samples.push([last.point.x, last.point.y]);
                            continue;
                        }

                        let (tl, tr, br, bl) =
                            rect_corners((last.point.x, last.point.y), (w, h), last.point.r);
                        samples.push([tl.0, tl.1]);
                        samples.push([tr.0, tr.1]);
                        samples.push([br.0, br.1]);
                        samples.push([bl.0, bl.1]);

                        let l = 18.0;
                        let corners = [tl, tr, br, bl];
                        if w > l {
                            let n = 2 + (w / l).floor() as usize;
                            add_intermediate_line_points(&mut samples, tl, tr, n, corners);
                            add_intermediate_line_points(&mut samples, bl, br, n, corners);
                        }
                        if h > l {
                            let n = 2 + (h / l).floor() as usize;
                            add_intermediate_line_points(&mut samples, tl, bl, n, corners);
                            add_intermediate_line_points(&mut samples, tr, br, n, corners);
                        }
                    }

                    if std::env::var_os("ERGOGEN_DUMP_HULL_SAMPLES").is_some() {
                        let dump_dir = std::env::temp_dir().join("ergogen-hull-dumps");
                        let _ = std::fs::create_dir_all(&dump_dir);
                        let mut fname =
                            format!("{outline_name}__concavity-{}__extend-{}", concavity, extend);
                        fname = fname.replace(['/', '\\', ' '], "_");
                        let path = dump_dir.join(format!("{fname}.samples.txt"));
                        let mut out = String::new();
                        for p in &samples {
                            out.push_str(&format!("{},{}\n", p[0], p[1]));
                        }
                        let _ = std::fs::write(path, out);
                    }

                    let hull = hulljs::hull(samples, concavity);
                    if std::env::var_os("ERGOGEN_DUMP_HULL_RAW").is_some() {
                        let dump_dir = std::env::temp_dir().join("ergogen-hull-dumps");
                        let _ = std::fs::create_dir_all(&dump_dir);
                        let mut fname =
                            format!("{outline_name}__concavity-{}__extend-{}", concavity, extend);
                        fname = fname.replace(['/', '\\', ' '], "_");
                        let path = dump_dir.join(format!("{fname}.hull_raw.txt"));
                        let mut out = String::new();
                        for p in &hull {
                            out.push_str(&format!("{},{}\n", p[0], p[1]));
                        }
                        let _ = std::fs::write(path, out);
                    }
                    let hull = simplify_closed_ring_points(hull);

                    let vertices: Vec<(f64, f64)> = hull
                        .into_iter()
                        .map(|v| position_xy((v[0], v[1]), p))
                        .collect();

                    let poly = primitives::polygon(&vertices);
                    apply_region_op(
                        &mut region,
                        op,
                        Region::from_pos(vec![poly]),
                        &mut stack,
                        &mut carry_neg,
                    );
                }
            }
            "path" => {
                let segments_v = obj
                    .get("segments")
                    .ok_or(OutlineError::Unsupported("missing segments"))?;
                let Value::Seq(segments) = segments_v else {
                    return Err(OutlineError::Unsupported("segments must be a sequence"));
                };

                let placements =
                    placements_for_where(where_v, asym, &points, &ref_points, &prepared.units)?;

                for p in placements {
                    let p = apply_adjust_if_present(
                        obj.get("adjust"),
                        p,
                        &ref_points,
                        &prepared.units,
                    )?;

                    let mut first_anchor: Option<Point> = None;
                    let mut last_anchor = Point::new(
                        0.0,
                        0.0,
                        0.0,
                        PointMeta {
                            mirrored: p.mirrored,
                        },
                    );

                    let mut prims: Vec<makerjs_path::Primitive> = Vec::new();

                    for (seg_index, seg_v) in segments.iter().enumerate() {
                        let Value::Map(seg_obj) = seg_v else {
                            return Err(OutlineError::Unsupported(
                                "segments entries must be objects",
                            ));
                        };

                        let Some(Value::String(seg_type)) = seg_obj.get("type") else {
                            return Err(OutlineError::Unsupported("segment.type must be a string"));
                        };
                        let Some(Value::Seq(seg_points)) = seg_obj.get("points") else {
                            return Err(OutlineError::Unsupported(
                                "segment.points must be a sequence",
                            ));
                        };

                        let mut parsed_points: Vec<[f64; 2]> = Vec::new();
                        if seg_index > 0 {
                            parsed_points.push([last_anchor.x, last_anchor.y]);
                        }

                        for (idx, sp) in seg_points.iter().enumerate() {
                            last_anchor = anchor::parse_anchor(
                                sp,
                                &format!("outlines.path.segments.{seg_index}.points[{idx}]"),
                                &ref_points,
                                last_anchor,
                                &prepared.units,
                                false,
                            )?;
                            if first_anchor.is_none() {
                                first_anchor = Some(last_anchor.clone());
                            }
                            parsed_points.push([last_anchor.x, last_anchor.y]);
                        }

                        match seg_type.as_str() {
                            "line" => {
                                for w in parsed_points.windows(2) {
                                    prims.push(makerjs_path::Primitive::Line { a: w[0], b: w[1] });
                                }
                            }
                            "arc" => {
                                if parsed_points.len() != 3 {
                                    return Err(OutlineError::Unsupported(
                                        "arc segments require 3 points (start, mid, end)",
                                    ));
                                }
                                let arc = makerjs_path::arc_from_3_points(
                                    parsed_points[0],
                                    parsed_points[1],
                                    parsed_points[2],
                                )?;
                                let (a, b) = makerjs_path::arc_endpoints(arc);
                                prims.push(makerjs_path::Primitive::Arc {
                                    arc,
                                    a,
                                    b,
                                    reversed: false,
                                });
                            }
                            "s_curve" => {
                                if parsed_points.len() != 2 {
                                    return Err(OutlineError::Unsupported(
                                        "s_curve segments require 2 points (from, to)",
                                    ));
                                }
                                let segs = makerjs_path::s_curve_primitives(
                                    parsed_points[0],
                                    parsed_points[1],
                                )?;
                                prims.extend(segs);
                            }
                            "bezier" => {
                                if parsed_points.len() != 3 && parsed_points.len() != 4 {
                                    return Err(OutlineError::Unsupported(
                                        "bezier segments require 3 (quadratic) or 4 (cubic) points",
                                    ));
                                }
                                let seed = makerjs_path::BezierSeed {
                                    origin: parsed_points[0],
                                    controls: parsed_points[1..parsed_points.len() - 1].to_vec(),
                                    end: *parsed_points.last().unwrap(),
                                };
                                let segs = makerjs_path::bezier_curve_primitives(seed, None);
                                prims.extend(segs);
                            }
                            _ => {
                                return Err(OutlineError::Unsupported(
                                    "unsupported path segment type",
                                ));
                            }
                        }
                    }

                    let Some(first_anchor) = first_anchor else {
                        return Err(OutlineError::Unsupported("path had no points"));
                    };
                    if first_anchor.x != last_anchor.x || first_anchor.y != last_anchor.y {
                        prims.push(makerjs_path::Primitive::Line {
                            a: [first_anchor.x, first_anchor.y],
                            b: [last_anchor.x, last_anchor.y],
                        });
                    }

                    // Apply placement transform (rotate about origin, then translate).
                    let prims = prims
                        .into_iter()
                        .map(|seg| match seg {
                            makerjs_path::Primitive::Line { a, b } => {
                                makerjs_path::Primitive::Line {
                                    a: {
                                        let (x, y) = position_xy((a[0], a[1]), p);
                                        [x, y]
                                    },
                                    b: {
                                        let (x, y) = position_xy((b[0], b[1]), p);
                                        [x, y]
                                    },
                                }
                            }
                            makerjs_path::Primitive::Arc {
                                arc,
                                a,
                                b,
                                reversed,
                            } => {
                                let (ox, oy) = position_xy((arc.origin[0], arc.origin[1]), p);
                                let a = {
                                    let (x, y) = position_xy((a[0], a[1]), p);
                                    [x, y]
                                };
                                let b = {
                                    let (x, y) = position_xy((b[0], b[1]), p);
                                    [x, y]
                                };
                                makerjs_path::Primitive::Arc {
                                    arc: makerjs_path::MakerArc {
                                        origin: [ox, oy],
                                        radius: arc.radius,
                                        start_angle_deg: arc.start_angle_deg + p.r,
                                        end_angle_deg: arc.end_angle_deg + p.r,
                                    },
                                    a,
                                    b,
                                    reversed,
                                }
                            }
                        })
                        .collect::<Vec<_>>();

                    let (sx, sy) = position_xy((first_anchor.x, first_anchor.y), p);
                    let pl = makerjs_path::chain_to_closed_polyline(prims, Some([sx, sy]))?;

                    apply_region_op(
                        &mut region,
                        op,
                        Region::from_pos(vec![pl]),
                        &mut stack,
                        &mut carry_neg,
                    );
                }
            }
            _ => {
                return Err(OutlineError::Unsupported(
                    "only what: rectangle|circle|polygon|hull|path is supported for now",
                ));
            }
        };
    }

    region.pos.extend(stack);
    region.neg.extend(carry_neg);

    visiting.remove(outline_name);
    Ok(region)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutlineRefOp {
    Add,
    Subtract,
    Intersect,
    Stack,
}

fn parse_outline_ref(raw: &str) -> (OutlineRefOp, &str) {
    if let Some(rest) = raw.strip_prefix('-') {
        return (OutlineRefOp::Subtract, rest);
    }
    if let Some(rest) = raw.strip_prefix('+') {
        return (OutlineRefOp::Add, rest);
    }
    if let Some(rest) = raw.strip_prefix('~') {
        return (OutlineRefOp::Intersect, rest);
    }
    if let Some(rest) = raw.strip_prefix('^') {
        return (OutlineRefOp::Stack, rest);
    }
    (OutlineRefOp::Add, raw)
}

fn apply_region_op(
    region: &mut Region,
    op: &str,
    part: Region,
    stack: &mut Vec<Polyline<f64>>,
    carry_neg: &mut Vec<Polyline<f64>>,
) {
    match op {
        "stack" => {
            stack.extend(part.pos);
            carry_neg.extend(part.neg);
        }
        "subtract" => apply_sub_region(region, part),
        "intersect" => {
            if region.pos.is_empty() && region.neg.is_empty() {
                return;
            }
            *region = intersect_region(region, &part);
        }
        _ => apply_add_region(region, part),
    }
}

fn apply_add_region(region: &mut Region, part: Region) {
    if region.pos.is_empty() && region.neg.is_empty() {
        *region = part;
        return;
    }

    let Region {
        pos: part_pos,
        neg: part_neg,
    } = part;

    let mut pos = region.pos.clone();
    pos.extend(part_pos.clone());
    let mut out = Region::union_all(pos);

    let mut new_neg: Vec<Polyline<f64>> = Vec::new();
    new_neg.extend(out.neg.clone());

    if !region.neg.is_empty() {
        let mut holes = Region::from_pos(region.neg.clone());
        if !part_pos.is_empty() {
            holes.subtract_all(&part_pos);
        }
        new_neg.extend(holes.pos);
        new_neg.extend(holes.neg);
    }

    if !part_neg.is_empty() {
        new_neg.extend(part_neg);
    }

    if !new_neg.is_empty() {
        out.neg = new_neg;
    }

    *region = out;
}

fn apply_sub_region(region: &mut Region, part: Region) {
    if region.pos.is_empty() && region.neg.is_empty() {
        return;
    }

    let original = region.clone();
    if !part.pos.is_empty() {
        region.subtract_all(&part.pos);
    }

    if !part.neg.is_empty() {
        let back = intersect_region(&original, &Region::from_pos(part.neg));
        apply_add_region(region, back);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpandJoints {
    Beveled,
    Round,
    Pointy,
}

#[derive(Debug, Clone, Copy)]
struct RectParams {
    center: (f64, f64),
    size: (f64, f64),
    rotation_deg: f64,
}

fn parse_expand_spec(
    expand: Option<&Value>,
    joints: Option<&Value>,
    units: &ergogen_parser::Units,
) -> Result<(f64, ExpandJoints), OutlineError> {
    let Some(expand) = expand else {
        return Err(OutlineError::Unsupported("missing expand"));
    };

    // Shorthand: "5]" / "6)" / "7>"
    if let Value::String(s) = expand
        && let Some((amount, joints)) = parse_expand_shorthand(units, s)?
    {
        return Ok((amount, joints));
    }

    let amount = eval_number(units, expand, "outlines.expand")?;
    let joints = match joints {
        None | Some(Value::Null) => ExpandJoints::Round,
        Some(Value::Number(n)) => {
            let i = (*n).round() as i64;
            match i {
                2 => ExpandJoints::Beveled,
                1 => ExpandJoints::Pointy,
                _ => ExpandJoints::Round,
            }
        }
        Some(Value::String(s)) => match s.as_str() {
            "beveled" | "bevel" => ExpandJoints::Beveled,
            "round" | "rounded" => ExpandJoints::Round,
            "pointy" | "miter" | "mitered" => ExpandJoints::Pointy,
            _ => ExpandJoints::Round,
        },
        _ => ExpandJoints::Round,
    };

    Ok((amount, joints))
}

fn parse_expand_shorthand(
    units: &ergogen_parser::Units,
    s: &str,
) -> Result<Option<(f64, ExpandJoints)>, OutlineError> {
    let s = s.trim();
    let (suffix, joints) = match s.chars().last() {
        Some(']') => (']', ExpandJoints::Beveled),
        Some(')') => (')', ExpandJoints::Round),
        Some('>') => ('>', ExpandJoints::Pointy),
        _ => return Ok(None),
    };

    let num = s.trim_end_matches(suffix).trim();
    if num.is_empty() {
        return Err(OutlineError::Unsupported("invalid expand shorthand"));
    }
    let amount = units
        .eval("outlines.expand", num)
        .map_err(OutlineError::Parser)?;
    Ok(Some((amount, joints)))
}

fn expand_region_rect_only(
    region: &Region,
    amount: f64,
    joints: ExpandJoints,
) -> Result<Region, OutlineError> {
    if amount == 0.0 {
        return Ok(region.clone());
    }
    if region.pos.len() != 1 || !region.neg.is_empty() {
        return Err(OutlineError::Unsupported(
            "expand only supports single-rectangle regions for now",
        ));
    }

    let pl = &region.pos[0];
    let Some(rect) = try_rectangle_params(pl) else {
        return Err(OutlineError::Unsupported(
            "expand only supports rectangle polylines for now",
        ));
    };

    let size = (rect.size.0 + 2.0 * amount, rect.size.1 + 2.0 * amount);
    let out = match joints {
        ExpandJoints::Pointy => primitives::rectangle(rect.center, size, rect.rotation_deg),
        ExpandJoints::Round => {
            primitives::rounded_rectangle(rect.center, size, amount, rect.rotation_deg)
        }
        ExpandJoints::Beveled => {
            // MakerJS-style bevel for a round-cap expansion: the cut distance along the expanded
            // edges is smaller than `amount`.
            let cut = amount * (2.0 - std::f64::consts::SQRT_2);
            // Match upstream's decimal rounding so DXF semantic compare (1e-6 quantization) lands
            // on the same side of half-way cases.
            let cut = (cut / 1e-7).round() * 1e-7;
            primitives::beveled_rectangle(rect.center, size, cut, rect.rotation_deg)
        }
    };

    Ok(Region::from_pos(vec![out]))
}

fn scale_region(region: &Region, scale: f64) -> Region {
    if scale == 1.0 {
        return region.clone();
    }
    let scale_pline = |p: &Polyline<f64>| {
        let mut p = p.clone();
        for v in &mut p.vertex_data {
            v.x *= scale;
            v.y *= scale;
        }
        p
    };
    Region {
        pos: region.pos.iter().map(scale_pline).collect(),
        neg: region.neg.iter().map(scale_pline).collect(),
    }
}

fn intersect_region(a: &Region, b: &Region) -> Region {
    let mut out_pos: Vec<Polyline<f64>> = Vec::new();
    let mut out_neg: Vec<Polyline<f64>> = Vec::new();

    for pa in &a.pos {
        for pb in &b.pos {
            let res = pa.boolean(pb, BooleanOp::And);
            out_pos.extend(res.pos_plines.into_iter().map(|p| p.pline));
            out_neg.extend(res.neg_plines.into_iter().map(|p| p.pline));
        }
    }

    let mut region = Region::union_all(out_pos);
    if !out_neg.is_empty() {
        region.subtract_all(&out_neg);
    }
    if !a.neg.is_empty() {
        region.subtract_all(&a.neg);
    }
    if !b.neg.is_empty() {
        region.subtract_all(&b.neg);
    }
    region
}

fn signed_offset_for(pline: &Polyline<f64>, abs: f64, inside: bool) -> f64 {
    match pline.orientation() {
        PlineOrientation::CounterClockwise => {
            // CCW: interior is on the left; left-offset is inward.
            if inside { abs } else { -abs }
        }
        PlineOrientation::Clockwise => {
            // CW: interior is on the right; left-offset is outward.
            if inside { -abs } else { abs }
        }
        PlineOrientation::Open => {
            // Should not happen for our regions.
            if inside { abs } else { -abs }
        }
    }
}

fn expand_region_round(region: &Region, expand: f64) -> Result<Region, OutlineError> {
    if expand == 0.0 {
        return Ok(region.clone());
    }

    let abs = expand.abs();
    let inside_pos = expand < 0.0;
    let inside_neg = !inside_pos;

    let opts = PlineOffsetOptions {
        handle_self_intersects: true,
        ..Default::default()
    };

    let mut pos_out: Vec<Polyline<f64>> = Vec::new();
    for p in &region.pos {
        let off = signed_offset_for(p, abs, inside_pos);
        pos_out.extend(p.parallel_offset_opt(off, &opts));
    }

    let mut neg_out: Vec<Polyline<f64>> = Vec::new();
    for p in &region.neg {
        let off = signed_offset_for(p, abs, inside_neg);
        neg_out.extend(p.parallel_offset_opt(off, &opts));
    }

    let mut out = Region::union_all(pos_out);
    if !neg_out.is_empty() {
        out.subtract_all(&neg_out);
    }
    Ok(out)
}

fn fillet_region_round(region: &Region, radius: f64) -> Result<Region, OutlineError> {
    let r = radius.abs();
    if r == 0.0 {
        return Ok(region.clone());
    }
    // Approximate MakerJS `chain.fillet` via a morphological opening:
    // inset by r, then offset back out by r (round joins).
    let inset = expand_region_round(region, -r)?;
    expand_region_round(&inset, r)
}

fn try_rectangle_params(pl: &Polyline<f64>) -> Option<RectParams> {
    if !pl.is_closed || pl.vertex_data.len() != 4 {
        return None;
    }
    for i in 0..4 {
        if !pl.vertex_data[i].bulge_is_zero() {
            return None;
        }
    }

    let v0 = pl.vertex_data[0];
    let v1 = pl.vertex_data[1];
    let v2 = pl.vertex_data[2];
    let v3 = pl.vertex_data[3];

    let cx = (v0.x + v1.x + v2.x + v3.x) / 4.0;
    let cy = (v0.y + v1.y + v2.y + v3.y) / 4.0;

    let dx01 = v1.x - v0.x;
    let dy01 = v1.y - v0.y;
    let dx12 = v2.x - v1.x;
    let dy12 = v2.y - v1.y;

    let w = (dx01 * dx01 + dy01 * dy01).sqrt();
    let h = (dx12 * dx12 + dy12 * dy12).sqrt();
    if w == 0.0 || h == 0.0 {
        return None;
    }

    let rotation_deg = dy01.atan2(dx01).to_degrees();
    Some(RectParams {
        center: (cx, cy),
        size: (w, h),
        rotation_deg,
    })
}

fn apply_adjust_if_present(
    adjust: Option<&Value>,
    p: Placement,
    ref_points: &IndexMap<String, Point>,
    units: &ergogen_parser::Units,
) -> Result<Placement, OutlineError> {
    let Some(adjust) = adjust else {
        return Ok(p);
    };
    if matches!(adjust, Value::Null) {
        return Ok(p);
    }

    let start = Point::new(
        p.x,
        p.y,
        p.r,
        PointMeta {
            mirrored: p.mirrored,
        },
    );
    let adjusted =
        anchor::parse_anchor(adjust, "outlines.adjust", ref_points, start, units, false)?;
    Ok(Placement {
        x: adjusted.x,
        y: adjusted.y,
        r: adjusted.r,
        mirrored: p.mirrored,
        bind_trbl: p.bind_trbl,
    })
}

fn parse_size(
    units: &ergogen_parser::Units,
    v: &Value,
    at: &str,
) -> Result<(f64, f64), OutlineError> {
    match v {
        Value::Number(n) => Ok((*n, *n)),
        Value::String(_) => {
            let s = eval_number(units, v, at)?;
            Ok((s, s))
        }
        Value::Seq(seq) if seq.len() == 2 => {
            let w = eval_number(units, &seq[0], at)?;
            let h = eval_number(units, &seq[1], at)?;
            Ok((w, h))
        }
        _ => Err(OutlineError::Unsupported("size must be number or [w, h]")),
    }
}

fn eval_number(units: &ergogen_parser::Units, v: &Value, at: &str) -> Result<f64, OutlineError> {
    match v {
        Value::Number(n) => Ok(*n),
        Value::String(s) => units.eval(at, s).map_err(OutlineError::Parser),
        _ => Err(OutlineError::Unsupported("expected number")),
    }
}

fn parse_asym(v: Option<&Value>, where_v: &Value) -> Asym {
    let default = if matches!(where_v, Value::Bool(true)) {
        Asym::Both
    } else {
        Asym::Source
    };
    let Some(v) = v else { return default };
    let Value::String(s) = v else { return default };
    match s.as_str() {
        "both" => Asym::Both,
        "source" | "origin" | "base" | "primary" | "left" => Asym::Source,
        "clone" | "image" | "derived" | "secondary" | "right" => Asym::Clone,
        _ => default,
    }
}

fn points_to_ref(points: &PointsOutput) -> IndexMap<String, Point> {
    points
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                Point::new(
                    v.x,
                    v.y,
                    v.r,
                    PointMeta {
                        mirrored: v.meta.mirrored.unwrap_or(false),
                    },
                ),
            )
        })
        .collect()
}

#[derive(Debug, Clone)]
struct AnchorWithKeyMeta {
    point: Point,
    width: f64,
    height: f64,
}

fn parse_anchor_with_key_meta(
    raw: &Value,
    at: &str,
    points: &PointsOutput,
    ref_points: &IndexMap<String, Point>,
    start: AnchorWithKeyMeta,
    units: &ergogen_parser::Units,
) -> Result<AnchorWithKeyMeta, OutlineError> {
    let point = anchor::parse_anchor(raw, at, ref_points, start.point.clone(), units, false)?;

    let mut width = start.width;
    let mut height = start.height;

    if let Some(ref_name) = anchor_ref_string(raw)
        && let Some(p) = points.get(ref_name)
    {
        width = p.meta.width;
        height = p.meta.height;
    }

    Ok(AnchorWithKeyMeta {
        point,
        width,
        height,
    })
}

fn anchor_ref_string(raw: &Value) -> Option<&str> {
    match raw {
        Value::String(s) => Some(s.as_str()),
        Value::Map(m) => match m.get("ref") {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn makerjs_round(n: f64, accuracy: f64) -> f64 {
    if n.fract() == 0.0 {
        return n;
    }
    let temp = 1.0 / accuracy;
    ((n + f64::EPSILON) * temp).round() / temp
}

fn points_equal_xy(a: [f64; 2], b: [f64; 2]) -> bool {
    makerjs_round(a[0] - b[0], 1e-7) == 0.0 && makerjs_round(a[1] - b[1], 1e-7) == 0.0
}

fn points_collinear(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    // Snap to a MakerJS-like epsilon grid first (prevents float-noise like 7.199999999999999).
    let a = [makerjs_round(a[0], 1e-7), makerjs_round(a[1], 1e-7)];
    let b = [makerjs_round(b[0], 1e-7), makerjs_round(b[1], 1e-7)];
    let c = [makerjs_round(c[0], 1e-7), makerjs_round(c[1], 1e-7)];
    let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    makerjs_round(cross, 1e-7) == 0.0
}

fn simplify_closed_ring_points(mut pts: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    if pts.len() < 3 {
        return pts;
    }

    // hull.js returns a closed point list (last == first).
    if pts.len() >= 2 && points_equal_xy(pts[0], *pts.last().unwrap()) {
        pts.pop();
    }

    // Drop consecutive duplicates.
    let mut deduped: Vec<[f64; 2]> = Vec::with_capacity(pts.len());
    for p in pts {
        if deduped.last().is_some_and(|last| points_equal_xy(*last, p)) {
            continue;
        }
        deduped.push(p);
    }

    // Remove collinear points until stable (matches how upstream DXFs end up with merged segments).
    let mut pts = deduped;
    loop {
        if pts.len() < 3 {
            break;
        }
        let n = pts.len();
        let mut out: Vec<[f64; 2]> = Vec::with_capacity(n);
        let mut removed = false;
        for i in 0..n {
            let prev = pts[(i + n - 1) % n];
            let cur = pts[i];
            let next = pts[(i + 1) % n];
            if points_collinear(prev, cur, next) {
                removed = true;
                continue;
            }
            out.push(cur);
        }
        pts = out;
        if !removed {
            break;
        }
    }

    pts
}

fn rotate_about_origin(x: f64, y: f64, deg: f64) -> (f64, f64) {
    let rad = deg.to_radians();
    let (s, c) = rad.sin_cos();
    (x * c - y * s, x * s + y * c)
}

type RectCorners = ((f64, f64), (f64, f64), (f64, f64), (f64, f64));

fn rect_corners(center: (f64, f64), size: (f64, f64), rotation_deg: f64) -> RectCorners {
    let (cx, cy) = center;
    let (w, h) = size;
    let hw = w / 2.0;
    let hh = h / 2.0;

    let mut tl = (-hw, hh);
    let mut tr = (hw, hh);
    let mut br = (hw, -hh);
    let mut bl = (-hw, -hh);

    if rotation_deg != 0.0 {
        tl = rotate_about_origin(tl.0, tl.1, rotation_deg);
        tr = rotate_about_origin(tr.0, tr.1, rotation_deg);
        br = rotate_about_origin(br.0, br.1, rotation_deg);
        bl = rotate_about_origin(bl.0, bl.1, rotation_deg);
    }

    tl = (tl.0 + cx, tl.1 + cy);
    tr = (tr.0 + cx, tr.1 + cy);
    br = (br.0 + cx, br.1 + cy);
    bl = (bl.0 + cx, bl.1 + cy);

    (tl, tr, br, bl)
}

fn add_intermediate_line_points(
    out: &mut Vec<[f64; 2]>,
    a: (f64, f64),
    b: (f64, f64),
    number_of_points: usize,
    corners: [(f64, f64); 4],
) {
    if number_of_points <= 2 {
        return;
    }
    let base = (number_of_points - 1) as f64;
    for i in 0..number_of_points {
        let t = (i as f64) / base;
        let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        if corners
            .iter()
            .any(|&c| points_equal_xy([p.0, p.1], [c.0, c.1]))
        {
            continue;
        }
        out.push([p.0, p.1]);
    }
}

fn position_xy(p: (f64, f64), placement: Placement) -> (f64, f64) {
    let (mut x, mut y) = p;
    if placement.r != 0.0 {
        (x, y) = rotate_about_origin(x, y, placement.r);
    }
    (x + placement.x, y + placement.y)
}

fn placements_for_where(
    where_v: &Value,
    asym: Asym,
    points: &PointsOutput,
    ref_points: &IndexMap<String, Point>,
    units: &ergogen_parser::Units,
) -> Result<Vec<Placement>, OutlineError> {
    match where_v {
        Value::Bool(true) => {
            let mut out = Vec::new();
            for p in points.values() {
                let mirrored = p.meta.mirrored.unwrap_or(false);
                if (asym == Asym::Source && mirrored) || (asym == Asym::Clone && !mirrored) {
                    continue;
                }
                out.push(Placement {
                    x: p.x,
                    y: p.y,
                    r: p.r,
                    mirrored,
                    bind_trbl: p.meta.bind,
                });
            }
            Ok(out)
        }
        // Upstream `where` defaults to a single point at [0, 0].
        Value::Null => Ok(vec![Placement {
            x: 0.0,
            y: 0.0,
            r: 0.0,
            mirrored: false,
            bind_trbl: [0.0; 4],
        }]),
        Value::Bool(false) => Ok(Vec::new()),
        Value::String(s) if looks_like_regex_literal(s) => {
            let re =
                parse_regex_literal(s).map_err(|_| OutlineError::Unsupported("invalid regex"))?;

            let mut out = Vec::new();
            for (name, p) in points.iter() {
                if !re.is_match(name) {
                    continue;
                }
                let mirrored = p.meta.mirrored.unwrap_or(false);
                if (asym == Asym::Source && mirrored) || (asym == Asym::Clone && !mirrored) {
                    continue;
                }
                out.push(Placement {
                    x: p.x,
                    y: p.y,
                    r: p.r,
                    mirrored,
                    bind_trbl: p.meta.bind,
                });
            }
            Ok(out)
        }
        other => {
            let start = Point::new(0.0, 0.0, 0.0, PointMeta::default());
            let base = anchor::parse_anchor(
                other,
                "outlines.where",
                ref_points,
                start.clone(),
                units,
                false,
            )?;

            match asym {
                Asym::Source => Ok(vec![Placement {
                    x: base.x,
                    y: base.y,
                    r: base.r,
                    mirrored: base.meta.mirrored,
                    bind_trbl: [0.0; 4],
                }]),
                Asym::Clone => {
                    let m = anchor::parse_anchor(
                        other,
                        "outlines.where",
                        ref_points,
                        start,
                        units,
                        true,
                    )?;
                    Ok(vec![Placement {
                        x: m.x,
                        y: m.y,
                        r: m.r,
                        mirrored: m.meta.mirrored,
                        bind_trbl: [0.0; 4],
                    }])
                }
                Asym::Both => {
                    let m = anchor::parse_anchor(
                        other,
                        "outlines.where",
                        ref_points,
                        start,
                        units,
                        true,
                    )?;
                    if (base.x - m.x).abs() < 1e-9
                        && (base.y - m.y).abs() < 1e-9
                        && (base.r - m.r).abs() < 1e-9
                    {
                        Ok(vec![Placement {
                            x: base.x,
                            y: base.y,
                            r: base.r,
                            mirrored: base.meta.mirrored,
                            bind_trbl: [0.0; 4],
                        }])
                    } else {
                        Ok(vec![
                            Placement {
                                x: base.x,
                                y: base.y,
                                r: base.r,
                                mirrored: base.meta.mirrored,
                                bind_trbl: [0.0; 4],
                            },
                            Placement {
                                x: m.x,
                                y: m.y,
                                r: m.r,
                                mirrored: m.meta.mirrored,
                                bind_trbl: [0.0; 4],
                            },
                        ])
                    }
                }
            }
        }
    }
}

fn apply_bind_to_centered_rect(
    center: (f64, f64),
    size: (f64, f64),
    bind_trbl: [f64; 4],
    rotation_deg: f64,
) -> (f64, f64, f64, f64) {
    let (cx, cy) = center;
    let (w, h) = size;
    // Upstream uses `-1` as a sentinel meaning “no bind on this side unless autobind fills it”.
    let t = bind_trbl[0].max(0.0);
    let r = bind_trbl[1].max(0.0);
    let b = bind_trbl[2].max(0.0);
    let l = bind_trbl[3].max(0.0);

    let w2 = w + l + r;
    let h2 = h + t + b;

    // Bind expands in the key's local axes. Expanding asymmetrically shifts the rectangle center.
    let dx_local = (r - l) / 2.0;
    let dy_local = (t - b) / 2.0;

    if rotation_deg == 0.0 {
        return (cx + dx_local, cy + dy_local, w2, h2);
    }

    let a = rotation_deg.to_radians();
    let (s, c) = a.sin_cos();
    let dx = dx_local * c - dy_local * s;
    let dy = dx_local * s + dy_local * c;
    (cx + dx, cy + dy, w2, h2)
}

fn looks_like_regex_literal(s: &str) -> bool {
    s.starts_with('/') && s.len() >= 2 && s[1..].contains('/')
}

fn parse_regex_literal(raw: &str) -> Result<Regex, ()> {
    // JS-style: /pattern/flags (we only care about `i` today)
    let mut chars = raw.chars();
    if chars.next() != Some('/') {
        return Err(());
    }

    let last_slash = raw.rfind('/').ok_or(())?;
    if last_slash == 0 {
        return Err(());
    }

    let pat = &raw[1..last_slash];
    let flags = &raw[last_slash + 1..];
    let mut pat = pat.to_string();

    if flags.contains('i') {
        pat = format!("(?i){pat}");
    }

    Regex::new(&pat).map_err(|_| ())
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use cavalier_contours::polyline::{PlineOrientation, PlineSource};
    use ergogen_geometry::primitives::{rectangle, rounded_rectangle};
    use proptest::prelude::*;

    fn region_bbox(region: &Region) -> (f64, f64, f64, f64) {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for pl in &region.pos {
            for v in &pl.vertex_data {
                min_x = min_x.min(v.x);
                min_y = min_y.min(v.y);
                max_x = max_x.max(v.x);
                max_y = max_y.max(v.y);
            }
        }
        (min_x, min_y, max_x, max_y)
    }

    fn bbox_close(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
        let eps = 1e-3;
        (a.0 - b.0).abs() < eps
            && (a.1 - b.1).abs() < eps
            && (a.2 - b.2).abs() < eps
            && (a.3 - b.3).abs() < eps
    }

    fn region_is_simple(region: &Region) -> bool {
        region
            .pos
            .iter()
            .chain(region.neg.iter())
            .all(|pl| pl.is_closed() && !pl.scan_for_self_intersect())
    }

    fn region_has_valid_winding(region: &Region) -> bool {
        region
            .pos
            .iter()
            .chain(region.neg.iter())
            .all(|pl| pl.orientation() != PlineOrientation::Open)
    }

    fn region_has_expected_winding(region: &Region) -> bool {
        region
            .pos
            .iter()
            .all(|pl| pl.orientation() == PlineOrientation::CounterClockwise)
            && region
                .neg
                .iter()
                .all(|pl| pl.orientation() == PlineOrientation::Clockwise)
    }

    proptest! {
        #[test]
        fn expand_zero_preserves_bbox(w in 2.0f64..50.0, h in 2.0f64..50.0) {
            let region = Region::from_pos(vec![rectangle((0.0, 0.0), (w, h), 0.0)]);
            let out = expand_region_round(&region, 0.0).unwrap();
            prop_assert!(bbox_close(region_bbox(&region), region_bbox(&out)));
            prop_assert!(region_is_simple(&out));
            prop_assert!(region_has_valid_winding(&out));
            prop_assert!(region_has_expected_winding(&out));
        }

        #[test]
        fn expand_positive_grows_bbox(
            w in 2.0f64..50.0,
            h in 2.0f64..50.0,
            expand in 0.25f64..10.0,
        ) {
            let region = Region::from_pos(vec![rectangle((0.0, 0.0), (w, h), 0.0)]);
            let (min_x, min_y, max_x, max_y) = region_bbox(&region);
            let out = expand_region_round(&region, expand).unwrap();
            let got = region_bbox(&out);
            let expected = (min_x - expand, min_y - expand, max_x + expand, max_y + expand);
            prop_assert!(bbox_close(got, expected));
            prop_assert!(region_is_simple(&out));
            prop_assert!(region_has_valid_winding(&out));
            prop_assert!(region_has_expected_winding(&out));
        }

        #[test]
        fn expand_negative_shrinks_bbox(
            w in 4.0f64..50.0,
            h in 4.0f64..50.0,
            expand in 0.25f64..10.0,
        ) {
            prop_assume!(expand < w / 2.0);
            prop_assume!(expand < h / 2.0);
            let region = Region::from_pos(vec![rectangle((0.0, 0.0), (w, h), 0.0)]);
            let (min_x, min_y, max_x, max_y) = region_bbox(&region);
            let out = expand_region_round(&region, -expand).unwrap();
            prop_assume!(!out.pos.is_empty());
            let got = region_bbox(&out);
            let expected = (min_x + expand, min_y + expand, max_x - expand, max_y - expand);
            prop_assert!(bbox_close(got, expected));
            prop_assert!(region_is_simple(&out));
            prop_assert!(region_has_valid_winding(&out));
            prop_assert!(region_has_expected_winding(&out));
        }

        #[test]
        fn fillet_preserves_bbox_for_rectangles(
            w in 4.0f64..50.0,
            h in 4.0f64..50.0,
            radius in 0.25f64..10.0,
        ) {
            prop_assume!(radius < w / 2.0);
            prop_assume!(radius < h / 2.0);
            let region = Region::from_pos(vec![rectangle((0.0, 0.0), (w, h), 0.0)]);
            let out = fillet_region_round(&region, radius).unwrap();
            prop_assert!(bbox_close(region_bbox(&region), region_bbox(&out)));
            prop_assert!(region_is_simple(&out));
            prop_assert!(region_has_valid_winding(&out));
            prop_assert!(region_has_expected_winding(&out));
        }

        #[test]
        fn expand_rounded_rectangle_preserves_winding_and_simplicity(
            w in 6.0f64..50.0,
            h in 6.0f64..50.0,
            radius in 0.5f64..8.0,
            expand in 0.25f64..6.0,
        ) {
            prop_assume!(radius < w / 2.0);
            prop_assume!(radius < h / 2.0);
            let region = Region::from_pos(vec![rounded_rectangle((0.0, 0.0), (w, h), radius, 0.0)]);
            let out = expand_region_round(&region, expand).unwrap();
            prop_assert!(region_is_simple(&out));
            prop_assert!(region_has_valid_winding(&out));
            prop_assert!(region_has_expected_winding(&out));
        }
    }
}
