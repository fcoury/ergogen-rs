use crate::{
    measure,
    paths::{self, Line, PathLine},
    schema::Point,
    Slope,
};

/// Rotate a point.
///
/// # Arguments
///
/// * `point` - The point to rotate.
/// * `angle` - The amount of rotation, in degrees.
/// * `origin` - The center point of rotation, defaults to (0.0, 0.0).
///
/// # Returns
///
/// A new point after rotation.
pub fn rotate(point: Point, angle: f64, origin: Option<Point>) -> (f64, f64) {
    let origin = origin.unwrap_or((0.0, 0.0));

    // Calculate the angle of the point relative to the origin in radians
    let point_angle = (point.1 - origin.1).atan2(point.0 - origin.0);

    // Calculate the distance between the origin and the point
    let distance = ((point.0 - origin.0).powi(2) + (point.1 - origin.1).powi(2)).sqrt();

    // Convert the angle to radians and add it to the point's angle
    let new_angle = point_angle + angle.to_radians();

    // Convert from polar coordinates back to Cartesian
    let rotated_point = (new_angle.cos() * distance, new_angle.sin() * distance);

    // Add the rotated point to the origin to get the final position
    (origin.0 + rotated_point.0, origin.1 + rotated_point.1)
}

pub fn add(a: Point, b: Point) -> Point {
    (a.0 + b.0, a.1 + b.1)
}

pub fn subtract(a: Point, b: Point) -> Point {
    (a.0 - b.0, a.1 - b.1)
}

pub fn from_polar(angle_in_radians: f64, radius: f64) -> Point {
    // Define special angles where cos or sin should be zero
    let is_zero_cos = |angle: f64| -> bool {
        let normalized = angle % (2.0 * std::f64::consts::PI);
        (normalized - std::f64::consts::PI / 2.0).abs() < f64::EPSILON
            || (normalized - 3.0 * std::f64::consts::PI / 2.0).abs() < f64::EPSILON
    };

    let is_zero_sin = |angle: f64| -> bool {
        let normalized = angle % (2.0 * std::f64::consts::PI);
        normalized.abs() < f64::EPSILON
            || (normalized - std::f64::consts::PI).abs() < f64::EPSILON
            || (normalized - 2.0 * std::f64::consts::PI).abs() < f64::EPSILON
    };

    let x = if is_zero_cos(angle_in_radians) {
        0.0
    } else {
        (radius * angle_in_radians.cos()).round()
    };

    let y = if is_zero_sin(angle_in_radians) {
        0.0
    } else {
        (radius * angle_in_radians.sin()).round()
    };

    (x, y)
}

pub fn vertical_intersection_point<P: PathLine>(vertical_line: &P, slope: &Slope) -> Point {
    let x = vertical_line.origin().0;
    let y = slope.slope * x + slope.y_intercept;
    (x, y)
}

pub fn from_slope_intersection<P: PathLine + Clone>(
    line_a: &P,
    line_b: &P,
    exclude_tangents: bool,
) -> (Option<Point>, bool) {
    let slope_a = measure::line_slope(line_a);
    let slope_b = measure::line_slope(line_b);

    if measure::is_slope_parallel(&slope_a, &slope_b) {
        if measure::is_slope_equal(&slope_a, &slope_b) {
            return (
                None,
                measure::is_line_overlapping(line_a, line_b, exclude_tangents),
            );
        }
        return (None, false);
    }

    if !slope_a.has_slope {
        (Some(vertical_intersection_point(line_a, &slope_b)), false)
    } else if !slope_b.has_slope {
        (Some(vertical_intersection_point(line_b, &slope_a)), false)
    } else {
        let x = (slope_b.y_intercept - slope_a.y_intercept) / (slope_a.slope - slope_b.slope);
        let y = slope_a.slope * x + slope_a.y_intercept;
        (Some((x, y)), false)
    }
}
