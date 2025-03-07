use crate::{
    paths::{Line, PathLine},
    schema::Point,
    Slope,
};

pub fn point_distance(a: Point, b: Point) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;

    (dx.powi(2) + dy.powi(2)).sqrt()
}

pub fn line_slope<P: PathLine + Clone>(line: &P) -> Slope {
    let dx = line.end().0 - line.origin().0;

    // Check if the line is vertical (or nearly so)
    if dx.abs() < f64::EPSILON {
        let line = Line {
            origin: line.origin(),
            end: line.end(),
        };
        return Slope {
            line,
            has_slope: false,
            slope: 0.0,
            y_intercept: 0.0,
        };
    }

    let dy = line.end().1 - line.origin().1;
    let slope = dy / dx;
    let y_intercept = line.origin().1 - slope * line.origin().0;

    let line = Line {
        origin: line.origin(),
        end: line.end(),
    };
    Slope {
        line,
        has_slope: true,
        slope,
        y_intercept,
    }
}

pub fn is_slope_parallel(a: &Slope, b: &Slope) -> bool {
    a.is_parallel(b)
}

pub fn is_slope_equal(a: &Slope, b: &Slope) -> bool {
    a == b
}

pub fn is_line_overlapping<P: PathLine>(line_a: &P, line_b: &P, exclude_tangents: bool) -> bool {
    fn check_points<P: PathLine>(a: &P, b: &P, exclude_tangents: bool) -> bool {
        // Check if either endpoint of line b is on line a
        is_between_points(&b.origin(), a, exclude_tangents)
            || is_between_points(&b.end(), a, exclude_tangents)
    }

    // Check if any endpoint of line_b is on line_a OR
    // if any endpoint of line_a is on line_b
    check_points(line_a, line_b, exclude_tangents) || check_points(line_b, line_a, exclude_tangents)
}

pub fn is_between_points<P: PathLine>(point: &Point, line: &P, exclusive: bool) -> bool {
    let mut one_dimension = false;

    // Assuming origin() and end() return (f64, f64) tuples
    let origin = line.origin();
    let end = line.end();

    // Iterate through x and y coordinates (indices 0 and 1)
    for i in (0..2).rev() {
        let origin_value = if i == 0 { origin.0 } else { origin.1 };
        let end_value = if i == 0 { end.0 } else { end.1 };
        let point_value = if i == 0 { point.0 } else { point.1 };

        if round_approx(origin_value - end_value, 0.000001) == 0.0 {
            if one_dimension {
                return false;
            }
            one_dimension = true;
            continue;
        }

        let rounded_origin = round_approx(origin_value, 1.0);
        let rounded_end = round_approx(end_value, 1.0);
        let rounded_point = round_approx(point_value, 1.0);

        if !is_between(rounded_point, rounded_origin, rounded_end, exclusive) {
            return false;
        }
    }

    true
}

fn is_between(value_in_question: f64, limit_a: f64, limit_b: f64, exclusive: bool) -> bool {
    if exclusive {
        limit_a.min(limit_b) < value_in_question && value_in_question < limit_a.max(limit_b)
    } else {
        limit_a.min(limit_b) <= value_in_question && value_in_question <= limit_a.max(limit_b)
    }
}

fn round_approx(value: f64, precision: f64) -> f64 {
    (value / precision).round() * precision
}
