use crate::Error;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Deeply copy a value that can be serialized and deserialized
pub fn deepcopy<T: Serialize + for<'a> Deserialize<'a>>(value: &T) -> T {
    let json = serde_json::to_value(value).unwrap();
    serde_json::from_value(json).unwrap()
}

/// Get a deeply nested value from an object or set a new value
pub fn deep<'a, T: Serialize + for<'de> Deserialize<'de>>(
    obj: &'a mut T,
    key: &str,
    val: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let levels: Vec<&str> = key.split('.').collect();
    let last = levels.last().unwrap();

    let mut json = serde_json::to_value(&obj).unwrap();
    let mut step = &mut json;

    for level in levels.iter().take(levels.len() - 1) {
        if !step.is_object() {
            *step = serde_json::Value::Object(serde_json::Map::new());
        }

        let map = step.as_object_mut().unwrap();
        if !map.contains_key(*level) {
            map.insert(
                level.to_string(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
        }

        step = map.get_mut(*level).unwrap();
    }

    if let Some(value) = val {
        if let Some(obj_map) = step.as_object_mut() {
            obj_map.insert(last.to_string(), value.clone());
            // Update the original object
            *obj = serde_json::from_value(json).unwrap();
            None
        } else {
            None
        }
    } else {
        // Return the value
        if let Some(obj_map) = step.as_object() {
            obj_map.get(*last).map(|v| v.clone())
        } else {
            None
        }
    }
}

/// Template a string, replacing variables in the format {{variable}} with values from a map
pub fn template(template_str: &str, values: &HashMap<String, serde_json::Value>) -> String {
    let mut result = template_str.to_string();
    let regex = regex::Regex::new(r"\{\{([^}]*)\}\}").unwrap();

    for captures in regex.captures_iter(template_str) {
        let key = captures.get(1).unwrap().as_str();
        let replacement = match deep_get_value(values, key) {
            Some(val) => val.to_string().trim_matches('"').to_string(),
            None => "".to_string(),
        };

        result = result.replace(&format!("{{{{{}}}}}", key), &replacement);
    }

    result
}

/// Helper function to get a deeply nested value from a JSON Value
fn deep_get_value<'a>(
    values: &'a HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = values.get(parts[0])?;

    for part in parts.iter().skip(1) {
        current = current.get(part)?;
    }

    Some(current)
}

/// Check if two points are equal
pub fn eq(a: &[f64; 2], b: &[f64; 2]) -> bool {
    a[0] == b[0] && a[1] == b[1]
}

/// Create a line from two points
pub fn line(a: [f64; 2], b: [f64; 2]) -> Line {
    Line { origin: a, end: b }
}

/// Create a circle at a point with a radius
pub fn circle(p: [f64; 2], r: f64) -> Circle {
    Circle {
        center: p,
        radius: r,
    }
}

/// Create a rectangle with width, height, and optional origin
pub fn rect(w: f64, h: f64, o: Option<[f64; 2]>) -> Rect {
    let origin = o.unwrap_or([0.0, 0.0]);

    Rect {
        top: line([origin[0], origin[1] + h], [origin[0] + w, origin[1] + h]),
        right: line([origin[0] + w, origin[1] + h], [origin[0] + w, origin[1]]),
        bottom: line([origin[0] + w, origin[1]], [origin[0], origin[1]]),
        left: line([origin[0], origin[1]], [origin[0], origin[1] + h]),
    }
}

/// Create a polygon from an array of points
pub fn poly(arr: &[[f64; 2]]) -> Poly {
    let mut paths = Vec::new();
    let mut prev = arr.last().unwrap();

    for p in arr {
        if eq(prev, p) {
            continue;
        }

        paths.push(line(*prev, *p));
        prev = p;
    }

    Poly { paths }
}

/// Calculate the bounding box of an array of points
pub fn bbox(arr: &[[f64; 2]]) -> BBox {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for p in arr {
        min_x = min_x.min(p[0]);
        min_y = min_y.min(p[1]);
        max_x = max_x.max(p[0]);
        max_y = max_y.max(p[1]);
    }

    BBox {
        low: [min_x, min_y],
        high: [max_x, max_y],
    }
}

// A distant point used for boolean operations
pub const FAR_POINT: [f64; 2] = [1234.1234, 2143.56789];

/// Parse a semantic version string
pub fn semver(str: &str, name: &str) -> Result<Version, Error> {
    let mut main = str.split('-').next().unwrap();

    if main.starts_with('v') {
        main = &main[1..];
    }

    // Ensure three version components (major.minor.patch)
    let parts: Vec<&str> = main.split('.').collect();
    let mut version_str = main.to_string();

    match parts.len() {
        1 => version_str.push_str(".0.0"),
        2 => version_str.push_str(".0"),
        _ => {}
    }

    match Version::parse(&version_str) {
        Ok(v) => Ok(v),
        Err(_) => Err(Error::Version(format!(
            "Invalid semver \"{}\" at {}!",
            str, name
        ))),
    }
}

/// Check if a version satisfies the required version
pub fn satisfies(current: &Version, expected: &Version) -> bool {
    // Major version must be the same
    if current.major != expected.major {
        return false;
    }

    // Minor version must be greater or equal
    if current.minor > expected.minor {
        return true;
    }

    if current.minor == expected.minor {
        // Patch version must be greater or equal
        return current.patch >= expected.patch;
    }

    false
}

// The following are simplified representations of geometric entities
// In a full implementation, these would be linked to a proper CAD library

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub origin: [f64; 2],
    pub end: [f64; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Circle {
    pub center: [f64; 2],
    pub radius: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub top: Line,
    pub right: Line,
    pub bottom: Line,
    pub left: Line,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Poly {
    pub paths: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub low: [f64; 2],
    pub high: [f64; 2],
}

// Boolean operations (union, subtract, intersect) would typically require a CAD library
// For now, we'll define these as trait methods that would be implemented later

pub trait CombineOperation {
    fn add(&self, other: &Self) -> Self;
    fn subtract(&self, other: &Self) -> Self;
    fn intersect(&self, other: &Self) -> Self;
    fn stack(&self, other: &Self) -> Self;
}
