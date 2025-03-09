use crate::{
    measure::Measure,
    paths::{Path, PathType},
    schema::{Model, Point},
};

use super::Atlas;

/// Calculate the extents of a model and store them in the provided Atlas
pub fn model_extents(model_to_measure: &Model, atlas: &mut Atlas) -> Option<MeasureWithCenter> {
    // Helper function to increase parent model measurements based on child measurements
    fn increase_parent_model(
        child_route: &[String],
        child_measurement: &Measure,
        atlas: &mut Atlas,
    ) {
        if child_route.len() < 2 {
            return;
        }
        // To get the parent route, just traverse backwards 2 to remove id and 'paths'/'models'
        let parent_route = &child_route[0..child_route.len() - 2];
        let parent_route_key = create_route_key(parent_route);
        if !atlas.model_map.contains_key(&parent_route_key) {
            // Just start with the known size
            atlas
                .model_map
                .insert(parent_route_key.clone(), child_measurement.clone());
        } else {
            // Increase the existing measurement
            let parent_measure = atlas.model_map.get_mut(&parent_route_key).unwrap();
            increase_measure(parent_measure, child_measurement);
        }
    }

    // Creating a function to handle path operations
    fn handle_path(atlas: &mut Atlas, walked_path: &WalkPath) {
        // Trust that the path measurement is good
        if !atlas.path_map.contains_key(&walked_path.route_key) {
            let path_measure = path_extents(walked_path.path_context, walked_path.offset);
            atlas
                .path_map
                .insert(walked_path.route_key.clone(), path_measure);
        }
        // Fix for the second error: clone the path measurement before passing it
        let path_measure = atlas.path_map.get(&walked_path.route_key).unwrap().clone();
        // Increase parent model based on this path's measurement
        increase_parent_model(&walked_path.route, &path_measure, atlas);
    }

    // Creating a function to handle model operations
    fn handle_model(atlas: &mut Atlas, walked_model: &WalkModel) {
        // Fix for the third error: clone the model measurement before passing it
        if let Some(model_measure) = atlas.model_map.get(&walked_model.route_key) {
            let model_measure_clone = model_measure.clone();
            increase_parent_model(&walked_model.route, &model_measure_clone, atlas);
        }
    }

    // Walk the model with separate functions instead of closures that capture the same variable
    walk_model_with_atlas(model_to_measure, atlas, handle_path, handle_model);

    // Mark that models have been measured
    atlas.models_measured = true;

    // Return the root model measurement with center
    if let Some(measure) = atlas.model_map.get("") {
        let measure_with_center = MeasureWithCenter {
            low: measure.low,
            high: measure.high,
            center: (
                (measure.low.0 + measure.high.0) / 2.0,
                (measure.low.1 + measure.high.1) / 2.0,
            ),
        };
        Some(measure_with_center)
    } else {
        None
    }
}

fn walk_model_with_atlas(
    model: &Model,
    atlas: &mut Atlas,
    path_handler: fn(&mut Atlas, &WalkPath),
    model_handler: fn(&mut Atlas, &WalkModel),
) {
    walk_model_with_options_and_atlas(model, &[], None, atlas, path_handler, model_handler);
}

fn walk_model_with_options_and_atlas(
    model: &Model,
    route: &[String],
    offset: Option<Point>,
    atlas: &mut Atlas,
    path_handler: fn(&mut Atlas, &WalkPath),
    model_handler: fn(&mut Atlas, &WalkModel),
) {
    // Process paths in this model
    if let Some(paths) = &model.paths {
        for (path_id, path) in paths.iter() {
            let mut path_route = route.to_vec();
            path_route.push("paths".to_string());
            path_route.push(path_id.clone());
            let route_key = create_route_key(&path_route);
            let walked_path = WalkPath {
                route: path_route,
                route_key,
                path_context: path.as_ref(),
                offset,
            };
            path_handler(atlas, &walked_path);
        }
    }
    // Process sub-models
    if let Some(models) = &model.models {
        for (model_id, sub_model) in models.iter() {
            let mut model_route = route.to_vec();
            model_route.push("models".to_string());
            model_route.push(model_id.clone());
            // Calculate new offset if needed
            let new_offset = match offset {
                Some(parent_offset) => Some((
                    parent_offset.0 + sub_model.origin.0,
                    parent_offset.1 + sub_model.origin.1,
                )),
                None => {
                    if sub_model.origin.0 != 0.0 || sub_model.origin.1 != 0.0 {
                        Some(sub_model.origin)
                    } else {
                        None
                    }
                }
            };
            // Recursively walk the sub-model
            walk_model_with_options_and_atlas(
                sub_model,
                &model_route,
                new_offset,
                atlas,
                path_handler,
                model_handler,
            );
            // Call model_handler for this sub-model
            let route_key = create_route_key(&model_route);
            let walked_model = WalkModel {
                route: model_route,
                route_key,
                model_context: sub_model,
            };
            model_handler(atlas, &walked_model);
        }
    }
}

/// Create a route key from a route array
fn create_route_key(route: &[String]) -> String {
    route.join("/")
}

/// Calculate the extents of a path
fn path_extents(path: &dyn Path, offset: Option<Point>) -> Measure {
    let mut measure = Measure {
        low: (f64::MAX, f64::MAX),
        high: (f64::MIN, f64::MIN),
    };

    // Get the path's origin
    let (origin_x, origin_y) = path.origin();

    // Apply offset if provided
    let (x_offset, y_offset) = match &offset {
        Some((x, y)) => (*x, *y),
        None => (0.0, 0.0),
    };

    // Measure based on path type
    match path.typ_() {
        PathType::Line => {
            if let Some(line) = path.as_line() {
                // Measure line's origin
                let x1 = origin_x + x_offset;
                let y1 = origin_y + y_offset;
                measure.low = (f64::min(measure.low.0, x1), f64::min(measure.low.1, y1));
                measure.high = (f64::max(measure.high.0, x1), f64::max(measure.high.1, y1));

                // Measure line's end
                let x2 = line.end.0 + x_offset;
                let y2 = line.end.1 + y_offset;
                measure.low = (f64::min(measure.low.0, x2), f64::min(measure.low.1, y2));
                measure.high = (f64::max(measure.high.0, x2), f64::max(measure.high.1, y2));
            }
        } // Implement other path types as needed
          // _ => {
          //     // For other path types, you'd need to implement specific measurement logic
          //     // based on your Path trait implementations
          // }
    }

    measure
}

/// Increase a measure by incorporating another measure
fn increase_measure(measure: &mut Measure, other: &Measure) {
    measure.low.0 = f64::min(measure.low.0, other.low.0);
    measure.low.1 = f64::min(measure.low.1, other.low.1);
    measure.high.0 = f64::max(measure.high.0, other.high.0);
    measure.high.1 = f64::max(measure.high.1, other.high.1);
}

/// Struct for a path during walking
struct WalkPath<'a> {
    route: Vec<String>,
    route_key: String,
    path_context: &'a dyn Path,
    offset: Option<Point>,
}

/// Struct for a model during walking
struct WalkModel<'a> {
    route: Vec<String>,
    route_key: String,
    model_context: &'a Model,
}

/// Struct for a measure with center point
pub struct MeasureWithCenter {
    pub low: Point,
    pub high: Point,
    pub center: Point,
}
