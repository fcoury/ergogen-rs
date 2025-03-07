use crate::{
    measure,
    paths::{Line, Path, PathLine, PathType},
    point::{self},
    schema::Point,
};

pub fn rotate<P: Path + Clone>(path: &P, angle: f64, rotation: Option<Point>) -> P {
    let point = point::rotate(path.origin(), angle, rotation);
    let mut result_path = path.clone();
    result_path.set_origin(point);

    match path.typ_() {
        PathType::Line => {
            let line = path.as_line().unwrap().clone();
            let end = point::rotate(line.end, angle, rotation);
            result_path.set_end(end);
        }
    };

    result_path
}

#[derive(Debug, Clone, Default)]
pub struct PathIntersectionOptions {
    path1_offset: Option<Point>,
    path2_offset: Option<Point>,
    exclude_tangents: Option<bool>,
}

pub fn intersection(
    path1: &impl Path,
    path2: &impl Path,
    options: Option<PathIntersectionOptions>,
) -> Vec<Point> {
    match path1.typ_() {
        crate::paths::PathType::Line => {
            line_path_intersection(path1.as_line().unwrap(), path2, options)
        }
    }
}

fn line_path_intersection(
    line1: &Line,
    path2: &impl Path,
    options: Option<PathIntersectionOptions>,
) -> Vec<Point> {
    match path2.typ_() {
        crate::paths::PathType::Line => {
            let line2 = path2.as_line().unwrap();
            line_line_intersection(line1, line2, options, false)
        }
    }
}

fn line_line_intersection(
    line1: &Line,
    line2: &Line,
    options: Option<PathIntersectionOptions>,
    swap_offsets: bool,
) -> Vec<Point> {
    let mut result = Vec::new();

    // Handle offsets
    let options = options.unwrap_or_default();

    let (path1_offset, path2_offset) = if swap_offsets {
        (options.path2_offset, options.path1_offset)
    } else {
        (options.path1_offset, options.path2_offset)
    };

    // Apply offsets if needed
    let line1 = if let Some(offset) = path1_offset {
        move_relative(line1, offset)
    } else {
        line1.clone()
    };

    let line2 = if let Some(offset) = path2_offset {
        move_relative(line2, offset)
    } else {
        line2.clone()
    };

    // Find the point of intersection between the two lines
    let exclude_tangents = options.exclude_tangents.unwrap_or(false);
    let (intersection_point, exclude_tangents) =
        crate::point::from_slope_intersection(&line1, &line2, exclude_tangents);

    if let Some(point) = intersection_point {
        // Check if the intersection point is between both line segments
        if measure::is_between_points(&point, &line1, exclude_tangents)
            && measure::is_between_points(&point, &line2, exclude_tangents)
        {
            result.push(point);
        }
    }

    result
}

pub fn move_relative<P: Path + Clone>(path: &P, delta: Point) -> P {
    let mut path = path.clone();
    path.set_origin(point::add(path.origin(), delta));

    match path.typ_() {
        PathType::Line => {
            let line = path.as_line().unwrap().clone();
            path.set_end(point::add(line.end, delta));
        }
    }

    path
}
