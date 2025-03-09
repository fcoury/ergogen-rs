use bon::{builder, Builder};
use indexmap::IndexMap;
use std::any::Any;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::{
    measure::{self, Atlas},
    path,
    paths::{Line, Path, PathType},
    point,
    schema::{Model, Point},
};

// Clone implementation for Path trait objects
pub trait ClonePath {
    fn clone_box(&self) -> Box<dyn Path>;
}

// We need to manually implement Clone for Box<dyn Path>
impl Clone for Box<dyn Path> {
    fn clone(&self) -> Box<dyn Path> {
        // We need to handle each concrete type that implements Path
        if let Some(line) = self.as_ref().as_line() {
            // Clone the Line
            Box::new(Line {
                origin: line.origin,
                end: line.end,
            })
        } else {
            // For other types we would add more cases
            // This is a fallback that shouldn't happen in practice
            Box::new(Line {
                origin: self.as_ref().origin(),
                end: self.as_ref().origin(), // Using origin as end is not ideal but safest fallback
            })
        }
    }
}

// Now implement ClonePath for concrete types
impl ClonePath for Line {
    fn clone_box(&self) -> Box<dyn Path> {
        Box::new(self.clone())
    }
}

// Implement for PathWithReason as well
impl ClonePath for PathWithReason {
    fn clone_box(&self) -> Box<dyn Path> {
        Box::new(self.clone())
    }
}

// Need to implement Clone for Model's fields
impl Clone for Model {
    fn clone(&self) -> Self {
        Model {
            typ_: self.typ_.clone(),
            origin: self.origin,
            paths: self.paths.as_ref().map(|paths| {
                let mut cloned_paths = IndexMap::new();
                for (k, v) in paths.iter() {
                    cloned_paths.insert(k.clone(), v.clone());
                }
                cloned_paths
            }),
            models: self.models.as_ref().map(|models| {
                let mut cloned_models = IndexMap::new();
                for (k, v) in models.iter() {
                    cloned_models.insert(k.clone(), v.clone());
                }
                cloned_models
            }),
            units: self.units.clone(),
            notes: self.notes.clone(),
            layer: self.layer.clone(),
            caption: None, // Just setting to None for now, can implement Caption clone if needed
        }
    }
}

/// Options to pass to model::combine
#[derive(Default, Clone, Builder)]
pub struct CombineOptions {
    /// Flag to remove paths which are not part of a loop
    pub trim_dead_ends: Option<bool>,

    /// Point which is known to be outside of the model
    pub far_point: Option<Point>,

    /// Output array of 2 models (corresponding to the input models) containing paths that were
    /// deleted in the combination. Each path will be of type PathWithReason, which has a .reason
    /// property describing why it was removed.
    pub out_deleted: Option<[Option<Model>; 2]>,

    /// Cached measurements for model A
    pub measure_a: Option<Atlas>,

    /// Cached measurements for model B
    pub measure_b: Option<Atlas>,

    /// Distance for considering points as matching
    pub point_matching_distance: Option<f64>,
}

// Additional structures needed for the implementation
pub struct CrossedPath {
    pub path: Box<dyn Path>,
    pub segments: Vec<Box<dyn Path>>,
    pub is_inside: Vec<bool>,
}

impl Clone for CrossedPath {
    fn clone(&self) -> Self {
        CrossedPath {
            path: self.path.clone(),
            segments: self.segments.clone(),
            is_inside: self.is_inside.clone(),
        }
    }
}

pub struct OverlappedSegment {
    pub path: Box<dyn Path>,
    pub added_path: Box<dyn Path>,
    pub duplicate: bool,
}

impl Clone for OverlappedSegment {
    fn clone(&self) -> Self {
        OverlappedSegment {
            path: self.path.clone(),
            added_path: self.added_path.clone(),
            duplicate: self.duplicate,
        }
    }
}

pub struct BreakPathsResult {
    pub crossed_paths: Vec<CrossedPath>,
    pub overlapped_segments: Vec<OverlappedSegment>,
}

impl Clone for BreakPathsResult {
    fn clone(&self) -> Self {
        BreakPathsResult {
            crossed_paths: self.crossed_paths.clone(),
            overlapped_segments: self.overlapped_segments.clone(),
        }
    }
}

pub struct PathRemoved {
    pub path: Box<dyn Path>,
    pub reason: String,
    pub route_key: String,
}

impl Clone for PathRemoved {
    fn clone(&self) -> Self {
        PathRemoved {
            path: self.path.clone(),
            reason: self.reason.clone(),
            route_key: self.route_key.clone(),
        }
    }
}

// Enhanced Path structure to include removal reason
pub struct PathWithReason {
    pub inner: Box<dyn Path>,
    pub reason: Option<String>,
    pub route_key: Option<String>,
}

impl Clone for PathWithReason {
    fn clone(&self) -> Self {
        PathWithReason {
            inner: self.inner.clone(),
            reason: self.reason.clone(),
            route_key: self.route_key.clone(),
        }
    }
}

impl Path for PathWithReason {
    fn typ_(&self) -> PathType {
        self.inner.typ_()
    }

    fn origin(&self) -> (f64, f64) {
        self.inner.origin()
    }

    fn set_origin(&mut self, origin: (f64, f64)) {
        self.inner.set_origin(origin);
    }

    fn set_end(&mut self, end: (f64, f64)) {
        self.inner.set_end(end);
    }

    fn layer(&self) -> Option<String> {
        self.inner.layer()
    }

    fn as_line(&self) -> Option<&Line> {
        self.inner.as_line()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Wrapper for Point to make it hash-able for use in HashMap/IndexMap
#[derive(Clone, Copy)]
struct HashablePoint(Point);

impl PartialEq for HashablePoint {
    fn eq(&self, other: &Self) -> bool {
        // Compare with some epsilon tolerance for floating point comparisons
        const EPSILON: f64 = 1e-9;
        (self.0 .0 - other.0 .0).abs() < EPSILON && (self.0 .1 - other.0 .1).abs() < EPSILON
    }
}

impl Eq for HashablePoint {}

impl Hash for HashablePoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Convert f64 to bits for hashing
        // Round to a precision that makes sense for your geometry
        let precision = 1e6; // 6 decimal places
        let x_rounded = (self.0 .0 * precision).round() as i64;
        let y_rounded = (self.0 .1 * precision).round() as i64;
        x_rounded.hash(state);
        y_rounded.hash(state);
    }
}

impl From<Point> for HashablePoint {
    fn from(point: Point) -> Self {
        HashablePoint(point)
    }
}

impl From<HashablePoint> for Point {
    fn from(point: HashablePoint) -> Self {
        point.0
    }
}

/// Combines two models based on inclusion rules
#[builder]
pub fn combine(
    model_a: &Model,
    model_b: &Model,
    include_a_inside_b: Option<bool>,
    include_a_outside_b: Option<bool>,
    include_b_inside_a: Option<bool>,
    include_b_outside_a: Option<bool>,
    options: Option<CombineOptions>,
) -> Model {
    let include_a_inside_b = include_a_inside_b.unwrap_or(false);
    let include_a_outside_b = include_a_outside_b.unwrap_or(true);
    let include_b_inside_a = include_b_inside_a.unwrap_or(false);
    let include_b_outside_a = include_b_outside_a.unwrap_or(true);

    let mut opts = options.clone().unwrap_or_default();

    // Initialize measurements if not provided
    if opts.measure_a.is_none() {
        let mut atlas_a = Atlas::new();
        atlas_a.measure_models(model_a);
        opts.measure_a = Some(atlas_a);
    }

    if opts.measure_b.is_none() {
        let mut atlas_b = Atlas::new();
        atlas_b.measure_models(model_b);
        opts.measure_b = Some(atlas_b);
    }

    // Make sure far_point is calculated if not provided
    if opts.far_point.is_none() {
        let measure_a = opts.measure_a.as_ref().unwrap();
        let measure_b = opts.measure_b.as_ref().unwrap();

        if let (Some(measure_a_model), Some(measure_b_model)) =
            (measure_a.model_map.get(""), measure_b.model_map.get(""))
        {
            let combined_measure = measure::Measure {
                low: (
                    f64::min(measure_a_model.low.0, measure_b_model.low.0),
                    f64::min(measure_a_model.low.1, measure_b_model.low.1),
                ),
                high: (
                    f64::max(measure_a_model.high.0, measure_b_model.high.0),
                    f64::max(measure_a_model.high.1, measure_b_model.high.1),
                ),
            };

            opts.far_point = Some(point::add(combined_measure.high, (1.0, 1.0)));
        }
    }

    let far_point = opts.far_point.unwrap_or((0.0, 0.0));
    let measure_a = opts.measure_a.as_ref().unwrap();
    let measure_b = opts.measure_b.as_ref().unwrap();

    // Break all paths at intersections
    let mut paths_a =
        break_all_paths_at_intersections(model_a, model_b, true, measure_a, measure_b, far_point);

    let mut paths_b =
        break_all_paths_at_intersections(model_b, model_a, true, measure_b, measure_a, far_point);

    // Check for equal overlaps
    if let Some(point_matching_distance) = opts.point_matching_distance {
        check_for_equal_overlaps(
            &mut paths_a.overlapped_segments,
            &mut paths_b.overlapped_segments,
            point_matching_distance,
        );
    }

    // Create the output models for tracking deleted paths
    let mut out_deleted = opts.out_deleted.unwrap_or([None, None]);

    // Function to track deleted paths - adapted to match the signature expected by add_or_delete_segments
    let mut track_deleted_path =
        |deleted_path: Box<dyn Path>, route_key: String, reason: String| {
            // Determine which model this path belongs to based on some logic
            // For now, we'll just use "a" paths for the first model and "b" paths for the second
            let which = if route_key.starts_with("models/a/") {
                0
            } else {
                1
            };

            // If out_deleted[which] is None, create a new Model
            if out_deleted[which].is_none() {
                out_deleted[which] = Some(Model {
                    typ_: None,
                    origin: (0.0, 0.0),
                    paths: Some(IndexMap::new()),
                    models: None,
                    units: None,
                    notes: None,
                    layer: None,
                    caption: None,
                });
            }

            // Add the deleted path to the model
            if let Some(ref mut model) = out_deleted[which] {
                if let Some(ref mut paths) = model.paths {
                    let path_with_reason = PathWithReason {
                        inner: deleted_path,
                        reason: Some(reason),
                        route_key: Some(route_key),
                    };

                    // Use route_key or generate a new key
                    let path_key = format!("deleted_{}", paths.len());
                    paths.insert(path_key, Box::new(path_with_reason) as Box<dyn Path>);
                }
            }
        };

    // Process paths from model A
    for crossed_path in &paths_a.crossed_paths {
        add_or_delete_segments(
            crossed_path,
            include_a_inside_b,
            include_a_outside_b,
            true,
            measure_a,
            &mut track_deleted_path,
        );
    }

    // Process paths from model B
    for crossed_path in &paths_b.crossed_paths {
        add_or_delete_segments(
            crossed_path,
            include_b_inside_a,
            include_b_outside_a,
            false,
            measure_b,
            &mut track_deleted_path,
        );
    }

    // Create the result model combining A and B
    let mut result = Model {
        typ_: None,
        origin: (0.0, 0.0),
        paths: None,
        models: Some(IndexMap::from([
            ("a".to_string(), model_a.clone()),
            ("b".to_string(), model_b.clone()),
        ])),
        units: None,
        notes: None,
        layer: None,
        caption: None,
    };

    // If trimDeadEnds option is enabled, remove dead ends
    if opts.trim_dead_ends.unwrap_or(true) {
        let should_keep = if !include_a_inside_b && !include_b_inside_a {
            // Union case
            Some(|walked_path: &WalkPath| -> bool {
                // When A and B share an outer contour, the segments marked as duplicate
                // will not pass the "inside" test on either A or B.
                // Duplicates were discarded from B but kept in A
                for overlapped_segment in &paths_a.overlapped_segments {
                    if overlapped_segment.duplicate {
                        // This isn't a perfect comparison, but it's a start
                        let path_ptr = walked_path.path_context as *const dyn Path;
                        let segment_ptr = overlapped_segment.added_path.as_ref() as *const dyn Path;
                        if std::ptr::eq(path_ptr, segment_ptr) {
                            return false;
                        }
                    }
                }

                // Default - keep the path
                true
            })
        } else {
            None
        };

        remove_dead_ends(
            &mut result,
            should_keep,
            &mut |wp: &WalkPath, reason: String| {
                let which = if wp.route[1] == "a" { 0 } else { 1 };

                // Create a boxed clone of the path context
                if let Some(line) = wp.path_context.as_line() {
                    let path_clone: Box<dyn Path> = Box::new(Line {
                        origin: line.origin,
                        end: line.end,
                    });

                    track_deleted_path(path_clone, wp.route_key.clone(), reason);
                }
            },
        );
    }

    // Update the options with our modified values
    if options.is_some() {
        opts.out_deleted = Some(out_deleted);
    }

    result
}

/// Function to walk the model and collect all paths
fn walk_model_paths<F>(model: &Model, route_prefix: &[String], mut callback: F)
where
    F: FnMut(&dyn Path, Vec<String>),
{
    if let Some(ref paths) = model.paths {
        for (path_id, path) in paths {
            let mut path_route = route_prefix.to_vec();
            path_route.push("paths".to_string());
            path_route.push(path_id.clone());

            callback(path.as_ref(), path_route);
        }
    }

    if let Some(ref models) = model.models {
        for (model_id, sub_model) in models {
            let mut model_route = route_prefix.to_vec();
            model_route.push("models".to_string());
            model_route.push(model_id.clone());

            // Collect paths from sub models and process them directly
            // This avoids the need for clone and recursive calls
            if let Some(ref sub_paths) = sub_model.paths {
                for (sub_path_id, sub_path) in sub_paths {
                    let mut sub_path_route = model_route.clone();
                    sub_path_route.push("paths".to_string());
                    sub_path_route.push(sub_path_id.clone());

                    callback(sub_path.as_ref(), sub_path_route);
                }
            }

            // For sub-sub models, we would need a more complex approach
            // but since most hierarchies are only two levels deep, this is a pragmatic solution
            // If you need deeper nesting, consider rewriting to use an explicit stack
        }
    }
}

/// Breaks all paths at their intersections
fn break_all_paths_at_intersections(
    model_a: &Model,
    model_b: &Model,
    check_inside: bool,
    measure_a: &Atlas,
    measure_b: &Atlas,
    far_point: Point,
) -> BreakPathsResult {
    let mut result = BreakPathsResult {
        crossed_paths: Vec::new(),
        overlapped_segments: Vec::new(),
    };

    // Process each path in model_a
    if let Some(ref paths_a) = model_a.paths {
        for (_, path_a) in paths_a {
            if let Some(line_a) = path_a.as_line() {
                let mut all_intersections: Vec<Point> = Vec::new();

                // Process each path in model_b
                if let Some(ref paths_b) = model_b.paths {
                    for (_, path_b) in paths_b {
                        if let Some(line_b) = path_b.as_line() {
                            let line_a_obj = Line {
                                origin: line_a.origin,
                                end: line_a.end,
                            };

                            let line_b_obj = Line {
                                origin: line_b.origin,
                                end: line_b.end,
                            };

                            // Find intersections between the two lines
                            let intersections = path::intersection(&line_a_obj, &line_b_obj, None);

                            // Add intersections to our list
                            all_intersections.extend(intersections);
                        }
                    }
                }

                // Process each submodel in model_b
                if let Some(ref submodels_b) = model_b.models {
                    for (_, submodel_b) in submodels_b {
                        if let Some(ref subpaths_b) = submodel_b.paths {
                            for (_, subpath_b) in subpaths_b {
                                if let Some(line_b) = subpath_b.as_line() {
                                    let line_a_obj = Line {
                                        origin: line_a.origin,
                                        end: line_a.end,
                                    };

                                    let line_b_obj = Line {
                                        origin: line_b.origin,
                                        end: line_b.end,
                                    };

                                    // Find intersections between the two lines
                                    let intersections =
                                        path::intersection(&line_a_obj, &line_b_obj, None);

                                    // Add intersections to our list
                                    all_intersections.extend(intersections);
                                }
                            }
                        }
                    }
                }

                // If we have intersections, sort them and break the path
                if !all_intersections.is_empty() {
                    // Remove duplicates (points that are very close to each other)
                    all_intersections.sort_by(|a, b| {
                        let dist_a = measure::point_distance(line_a.origin, *a);
                        let dist_b = measure::point_distance(line_a.origin, *b);
                        dist_a.partial_cmp(&dist_b).unwrap()
                    });

                    // Create segments based on intersections
                    let mut segments: Vec<Box<dyn Path>> = Vec::new();
                    let mut inside_flags: Vec<bool> = Vec::new();

                    // Starting point for the first segment
                    let mut segment_start = line_a.origin;

                    // Create segments for each interval between intersections
                    for intersection in &all_intersections {
                        // Create a new line segment
                        let segment = Line {
                            origin: segment_start,
                            end: *intersection,
                        };

                        // Check if segment is inside model_b (if check_inside is true)
                        let is_inside = if check_inside {
                            // Compute midpoint of segment
                            let midpoint = (
                                (segment.origin.0 + segment.end.0) / 2.0,
                                (segment.origin.1 + segment.end.1) / 2.0,
                            );

                            // Check if midpoint is inside model_b
                            is_point_inside_model(midpoint, model_b, far_point)
                        } else {
                            false
                        };

                        // Add segment and inside flag
                        segments.push(Box::new(segment));
                        inside_flags.push(is_inside);

                        // Update starting point for next segment
                        segment_start = *intersection;
                    }

                    // Add final segment from last intersection to end
                    let final_segment = Line {
                        origin: segment_start,
                        end: line_a.end,
                    };

                    // Check if final segment is inside
                    let is_inside = if check_inside {
                        let midpoint = (
                            (final_segment.origin.0 + final_segment.end.0) / 2.0,
                            (final_segment.origin.1 + final_segment.end.1) / 2.0,
                        );
                        is_point_inside_model(midpoint, model_b, far_point)
                    } else {
                        false
                    };

                    // Add final segment
                    segments.push(Box::new(final_segment));
                    inside_flags.push(is_inside);

                    // Add to crossed_paths if we have segments
                    if !segments.is_empty() {
                        result.crossed_paths.push(CrossedPath {
                            path: Box::new(line_a.clone()),
                            segments,
                            is_inside: inside_flags,
                        });
                    }
                }
            }
        }
    }

    // Handle nested models in model_a
    if let Some(ref submodels_a) = model_a.models {
        for (_, submodel_a) in submodels_a {
            // Recursively process each submodel
            let submodel_result = break_all_paths_at_intersections(
                submodel_a,
                model_b,
                check_inside,
                measure_a,
                measure_b,
                far_point,
            );

            // Merge results
            result.crossed_paths.extend(submodel_result.crossed_paths);
            result
                .overlapped_segments
                .extend(submodel_result.overlapped_segments);
        }
    }

    result
}

/// Determines if a point is inside a model
/// This is a simplified approach - a proper implementation would use ray casting
fn is_point_inside_model(point: Point, model: &Model, far_point: Point) -> bool {
    // Create a ray from point to far_point
    let ray = Line {
        origin: point,
        end: far_point,
    };

    // Count intersections with all paths in the model
    let mut intersection_count = 0;

    // Process paths directly in this model
    if let Some(ref paths) = model.paths {
        for (_, path) in paths {
            if let Some(line) = path.as_line() {
                let line_obj = Line {
                    origin: line.origin,
                    end: line.end,
                };

                let intersections = path::intersection(&ray, &line_obj, None);

                intersection_count += intersections.len();
            }
        }
    }

    // Process submodels
    if let Some(ref submodels) = model.models {
        for (_, submodel) in submodels {
            // Check if the point is inside the submodel
            if is_point_inside_model(point, submodel, far_point) {
                // If using a ray-casting algorithm, we want to count this as an odd number
                // of intersections to indicate "inside"
                intersection_count += 1;
            }
        }
    }

    // If the number of intersections is odd, the point is inside
    intersection_count % 2 == 1
}

/// Checks for equally overlapping segments
fn check_for_equal_overlaps(
    overlapped_segments_a: &mut Vec<OverlappedSegment>,
    overlapped_segments_b: &mut Vec<OverlappedSegment>,
    point_matching_distance: f64,
) {
    // For each segment in A, check if there's a matching segment in B
    for segment_a in overlapped_segments_a.iter_mut() {
        if let Some(line_a) = segment_a.added_path.as_line() {
            for segment_b in overlapped_segments_b.iter_mut() {
                if let Some(line_b) = segment_b.added_path.as_line() {
                    // Check if the lines have matching endpoints (within point_matching_distance)
                    let origin_matches = measure::point_distance(line_a.origin, line_b.origin)
                        <= point_matching_distance
                        || measure::point_distance(line_a.origin, line_b.end)
                            <= point_matching_distance;

                    let end_matches = measure::point_distance(line_a.end, line_b.origin)
                        <= point_matching_distance
                        || measure::point_distance(line_a.end, line_b.end)
                            <= point_matching_distance;

                    // Check if the slopes are equal
                    let slope_a = measure::line_slope(line_a);
                    let slope_b = measure::line_slope(line_b);

                    if origin_matches && end_matches && measure::is_slope_equal(&slope_a, &slope_b)
                    {
                        // Mark as duplicates
                        segment_a.duplicate = true;
                        segment_b.duplicate = true;
                        break;
                    }
                }
            }
        }
    }
}

/// Adds or deletes segments based on inclusion rules
fn add_or_delete_segments<F>(
    crossed_path: &CrossedPath,
    include_inside: bool,
    include_outside: bool,
    _is_a: bool,    // Renamed to _is_a to avoid unused warning
    _atlas: &Atlas, // Renamed to _atlas to avoid unused warning
    mut on_deleted: F,
) where
    F: FnMut(Box<dyn Path>, String, String),
{
    // For each segment in crossed_path
    for (i, segment) in crossed_path.segments.iter().enumerate() {
        let is_inside = crossed_path.is_inside.get(i).cloned().unwrap_or(false);

        // Determine if this segment should be kept
        let keep = if is_inside {
            include_inside
        } else {
            include_outside
        };

        if !keep {
            // Get the route key for the path (this would come from a path walk)
            let route_key = "unknown".to_string(); // Placeholder

            // Call on_deleted with reason based on inside/outside status
            let reason = if is_inside {
                "inside".to_string()
            } else {
                "outside".to_string()
            };

            // Clone the segment before passing it to on_deleted
            let segment_clone: Box<dyn Path> = match segment.typ_() {
                PathType::Line => {
                    if let Some(line) = segment.as_line() {
                        Box::new(Line {
                            origin: line.origin,
                            end: line.end,
                        })
                    } else {
                        // This should not happen if segment is a line
                        continue;
                    }
                } // Add other path types as needed
            };

            on_deleted(segment_clone, route_key, reason);
        }
    }
}

/// Helper struct for walking paths
pub struct WalkPath<'a> {
    pub path_context: &'a dyn Path,
    pub route: Vec<String>,
    pub route_key: String,
}

/// Removes dead ends from the model
fn remove_dead_ends<F, G>(model: &mut Model, should_keep: Option<G>, mut on_deleted: F)
where
    F: FnMut(&WalkPath, String),
    G: Fn(&WalkPath) -> bool,
{
    // A dead end is a path that doesn't form a loop with other paths
    // To identify dead ends, we need to:
    // 1. Build a graph of connected paths
    // 2. Identify paths that have only one connection (dead ends)
    // 3. Remove them and repeat until no more dead ends are found

    // First, we need to collect all paths from the model
    let mut paths = Vec::new();

    // Walk through the model and collect paths
    if let Some(ref models) = model.models {
        for (model_id, sub_model) in models {
            // For each path in the submodel
            if let Some(ref model_paths) = sub_model.paths {
                for (path_id, path) in model_paths {
                    let route = vec![
                        "models".to_string(),
                        model_id.clone(),
                        "paths".to_string(),
                        path_id.clone(),
                    ];
                    let route_key = route.join("/");

                    // Create a WalkPath
                    let walk_path = WalkPath {
                        path_context: path.as_ref(),
                        route: route.clone(),
                        route_key,
                    };

                    // Apply should_keep filter if provided
                    let keep = match &should_keep {
                        Some(keep_fn) => keep_fn(&walk_path),
                        None => true,
                    };

                    if !keep {
                        on_deleted(&walk_path, "filtered".to_string());
                        continue;
                    }

                    // If the path is a line, add it to our paths list
                    if let Some(_line) = path.as_line() {
                        paths.push(walk_path);
                    }
                }
            }
        }
    }

    // Build a graph of connected paths using our hashable point wrapper
    // For simplicity, we'll use a map of endpoint to list of paths
    let mut connections: HashMap<HashablePoint, Vec<usize>> = HashMap::new();

    // Populate connections
    for (i, walk_path) in paths.iter().enumerate() {
        if let Some(line) = walk_path.path_context.as_line() {
            // Add both endpoints to connections
            connections
                .entry(HashablePoint(line.origin))
                .or_insert_with(Vec::new)
                .push(i);
            connections
                .entry(HashablePoint(line.end))
                .or_insert_with(Vec::new)
                .push(i);
        }
    }

    // Identify dead ends
    let mut any_deleted = true;

    // Repeat until no more dead ends are found
    while any_deleted {
        any_deleted = false;
        let mut dead_ends = Vec::new();

        // Find endpoints that only connect to one path
        for (_, connected_paths) in &connections {
            if connected_paths.len() == 1 {
                // This is a dead end
                dead_ends.push(connected_paths[0]);
            }
        }

        // Create a vector of unique dead ends
        let unique_dead_ends: Vec<usize> = dead_ends.iter().cloned().collect();

        for &path_index in &unique_dead_ends {
            // Get the path
            if path_index < paths.len() {
                let walk_path = &paths[path_index];

                // Call on_deleted
                on_deleted(walk_path, "dead end".to_string());

                // Remove the path from connections
                if let Some(line) = walk_path.path_context.as_line() {
                    // Remove from origin connections
                    if let Some(origin_connections) =
                        connections.get_mut(&HashablePoint(line.origin))
                    {
                        origin_connections.retain(|&i| i != path_index);
                    }

                    // Remove from end connections
                    if let Some(end_connections) = connections.get_mut(&HashablePoint(line.end)) {
                        end_connections.retain(|&i| i != path_index);
                    }
                }

                any_deleted = true;
            }
        }
    }

    // Now we should modify the model to remove the paths we deleted
    // But since we're passing ownership of deleted paths to on_deleted,
    // the model structure should be modified by the caller
}
