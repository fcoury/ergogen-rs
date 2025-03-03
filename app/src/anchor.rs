use nalgebra::{Point2, Vector2};
use serde_json::Value;
use std::collections::HashMap;
use std::f64::consts::PI;

use crate::{point::Point, Error, Result};

/// Convert a reference to its mirrored version or vice versa
pub fn mirror_ref(reference: &str, mirror: bool) -> String {
    if mirror {
        if reference.starts_with("mirror_") {
            reference[7..].to_string()
        } else {
            format!("mirror_{}", reference)
        }
    } else {
        reference.to_string()
    }
}

/// Common fields for all aggregators
const AGGREGATOR_COMMON: [&str; 2] = ["parts", "method"];

/// Parse an anchor configuration and return a point
pub fn parse(
    raw: &Value,
    name: &str,
    points: &HashMap<String, Point>,
    start: Option<&Point>,
    mirror: bool,
    units: &HashMap<String, f64>,
) -> Result<Point> {
    // Default starting point
    let start = start.cloned().unwrap_or_else(|| Point::default());

    // Handle different anchor types
    match raw {
        // String reference
        Value::String(s) => parse(
            &Value::Object(serde_json::Map::from_iter(vec![(
                "ref".to_string(),
                Value::String(s.clone()),
            )])),
            name,
            points,
            Some(&start),
            mirror,
            units,
        ),

        // Array of successive anchors
        Value::Array(arr) => {
            let mut current = start.clone();
            for (i, step) in arr.iter().enumerate() {
                current = parse(
                    step,
                    &format!("{}[{}]", name, i + 1),
                    points,
                    Some(&current),
                    mirror,
                    units,
                )?;
            }
            Ok(current)
        }

        // Object with anchor properties
        Value::Object(obj) => {
            let mut point = start.clone();

            // Check if we have a reference or aggregate (they are mutually exclusive)
            if obj.contains_key("ref") && obj.contains_key("aggregate") {
                return Err(Error::Config(format!(
                    "Fields \"ref\" and \"aggregate\" cannot appear together in anchor \"{}\"!",
                    name
                )));
            }

            // Handle reference
            if let Some(reference) = obj.get("ref") {
                match reference {
                    // String reference
                    Value::String(ref_str) => {
                        let parsed_ref = mirror_ref(ref_str, mirror);
                        if let Some(referenced_point) = points.get(&parsed_ref) {
                            point = referenced_point.clone();
                        } else {
                            return Err(Error::Config(format!(
                                "Unknown point reference \"{}\" in anchor \"{}\"!",
                                parsed_ref, name
                            )));
                        }
                    }
                    // Recursive reference parsing
                    _ => {
                        point = parse(
                            reference,
                            &format!("{}.ref", name),
                            points,
                            Some(&start),
                            mirror,
                            units,
                        )?;
                    }
                }
            }

            // Handle aggregation
            if let Some(aggregate) = obj.get("aggregate") {
                if let Value::Object(agg_obj) = aggregate {
                    // Get aggregation method
                    let method = agg_obj
                        .get("method")
                        .and_then(|m| m.as_str())
                        .unwrap_or("average");

                    // Get parts to aggregate
                    let parts = match agg_obj.get("parts") {
                        Some(Value::Array(parts_arr)) => parts_arr,
                        _ => {
                            return Err(Error::Config(format!(
                                "Field \"{}.aggregate.parts\" must be an array!",
                                name
                            )))
                        }
                    };

                    // Parse each part
                    let mut parsed_parts = Vec::new();
                    for (i, part) in parts.iter().enumerate() {
                        let parsed = parse(
                            part,
                            &format!("{}.aggregate.parts[{}]", name, i + 1),
                            points,
                            Some(&start),
                            mirror,
                            units,
                        )?;
                        parsed_parts.push(parsed);
                    }

                    // Apply aggregation method
                    match method {
                        "average" => {
                            if parsed_parts.is_empty() {
                                point = Point::default();
                            } else {
                                let len = parsed_parts.len() as f64;
                                let mut x = 0.0;
                                let mut y = 0.0;
                                let mut r = 0.0;

                                for part in &parsed_parts {
                                    x += part.x;
                                    y += part.y;
                                    r += part.r;
                                }

                                point = Point::new(x / len, y / len, r / len, HashMap::new());
                            }
                        }
                        "intersect" => {
                            if parsed_parts.len() != 2 {
                                return Err(Error::Config(format!(
                                    "Intersect expects exactly two parts, but got {}!",
                                    parsed_parts.len()
                                )));
                            }

                            // Get the two points
                            let p1 = &parsed_parts[0];
                            let p2 = &parsed_parts[1];

                            // Create lines from the points

                            // Line 1: from point 1 along its Y axis (rotated)
                            let p1_origin = Point2::new(p1.x, p1.y);
                            let p1_vec = Vector2::new(0.0, -1.0); // Up direction
                            let p1_rot = p1.r * PI / 180.0;
                            let p1_dir = nalgebra::Rotation2::new(p1_rot) * p1_vec;

                            // Line 2: from point 2 along its Y axis (rotated)
                            let p2_origin = Point2::new(p2.x, p2.y);
                            let p2_vec = Vector2::new(0.0, -1.0); // Up direction
                            let p2_rot = p2.r * PI / 180.0;
                            let p2_dir = nalgebra::Rotation2::new(p2_rot) * p2_vec;

                            // Calculate intersection
                            // For two lines represented as p1 + t1 * dir1 and p2 + t2 * dir2
                            // Solve for t1 and t2
                            let det = p1_dir.x * p2_dir.y - p1_dir.y * p2_dir.x;

                            if det.abs() < 1e-10 {
                                return Err(Error::Config(format!(
                                    "The points under \"{}.parts\" do not intersect!",
                                    name
                                )));
                            }

                            let dx = p2_origin.x - p1_origin.x;
                            let dy = p2_origin.y - p1_origin.y;

                            let t1 = (dx * p2_dir.y - dy * p2_dir.x) / det;

                            // Calculate intersection point
                            let intersection_x = p1_origin.x + t1 * p1_dir.x;
                            let intersection_y = p1_origin.y + t1 * p1_dir.y;

                            point = Point::new(intersection_x, intersection_y, 0.0, HashMap::new());
                        }
                        _ => {
                            return Err(Error::Config(format!(
                                "Unknown aggregator method \"{}\" in anchor \"{}\"!",
                                method, name
                            )));
                        }
                    }
                }
            }

            // Apply transformations

            // Orient: rotate to look at another point or by a specified angle
            if let Some(orient) = obj.get("orient") {
                match orient {
                    Value::Number(n) => {
                        if let Some(angle) = n.as_f64() {
                            let resist =
                                obj.get("resist").and_then(|r| r.as_bool()).unwrap_or(false);
                            point.rotate(angle, None, resist);
                        }
                    }
                    _ => {
                        // Orient towards another point
                        let target = parse(
                            orient,
                            &format!("{}.orient", name),
                            points,
                            Some(&start),
                            mirror,
                            units,
                        )?;
                        point.r = point.angle(&target);
                    }
                }
            }

            // Shift: move the point by a specified amount
            if let Some(shift) = obj.get("shift") {
                let resist = obj.get("resist").and_then(|r| r.as_bool()).unwrap_or(false);

                match shift {
                    Value::Array(arr) => {
                        if arr.len() >= 2 {
                            if let (Some(Value::Number(x)), Some(Value::Number(y))) =
                                (arr.get(0), arr.get(1))
                            {
                                if let (Some(x_val), Some(y_val)) = (x.as_f64(), y.as_f64()) {
                                    point.shift([x_val, y_val], true, resist);
                                }
                            }
                        }
                    }
                    Value::Number(n) => {
                        if let Some(val) = n.as_f64() {
                            point.shift([val, val], true, resist);
                        }
                    }
                    _ => {
                        // TODO: Handle expressions or other formats
                    }
                }
            }

            // Rotate: rotate by a specified angle
            if let Some(rotate) = obj.get("rotate") {
                let resist = obj.get("resist").and_then(|r| r.as_bool()).unwrap_or(false);

                match rotate {
                    Value::Number(n) => {
                        if let Some(angle) = n.as_f64() {
                            point.rotate(angle, None, resist);
                        }
                    }
                    _ => {
                        // Rotate towards another point
                        let target = parse(
                            rotate,
                            &format!("{}.rotate", name),
                            points,
                            Some(&start),
                            mirror,
                            units,
                        )?;
                        point.r = point.angle(&target);
                    }
                }
            }

            // Affect: selectively apply x, y, or r from the calculated point to the starting point
            if let Some(affect) = obj.get("affect") {
                let candidate = point.clone();
                point = start.clone();

                let affect_list = match affect {
                    Value::String(s) => s.chars().collect::<Vec<char>>(),
                    Value::Array(arr) => {
                        let mut chars = Vec::new();
                        for item in arr {
                            if let Value::String(s) = item {
                                for c in s.chars() {
                                    chars.push(c);
                                }
                            }
                        }
                        chars
                    }
                    _ => Vec::new(),
                };

                for c in affect_list {
                    match c {
                        'x' => point.x = candidate.x,
                        'y' => point.y = candidate.y,
                        'r' => point.r = candidate.r,
                        _ => {
                            return Err(Error::Config(format!(
                                "Invalid affect value '{}' in \"{}.affect\". Expected 'x', 'y', or 'r'.",
                                c,
                                name
                            )));
                        }
                    }
                }
            }

            Ok(point)
        }

        // Other types
        _ => Err(Error::Config(format!(
            "Anchor \"{}\" must be a string, array, or object!",
            name
        ))),
    }
}
