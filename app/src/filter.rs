// use regex::Regex;
// use serde_json::Value;
// use std::collections::HashMap;
//
// use crate::{anchor, point::Point, utils, Error, Result};
//
// /// A filter function that takes a point and returns whether it matches
// type FilterFn = Box<dyn Fn(&Point) -> bool>;
//
// /// Create a filter function that always returns true
// fn always_true() -> FilterFn {
//     Box::new(|_| true)
// }
//
// /// Create a filter function that always returns false
// fn always_false() -> FilterFn {
//     Box::new(|_| false)
// }
//
// /// Create a filter function that combines multiple filters with AND
// fn and_filter(filters: Vec<FilterFn>) -> FilterFn {
//     Box::new(move |point| filters.iter().all(|filter| filter(point)))
// }
//
// /// Create a filter function that combines multiple filters with OR
// fn or_filter(filters: Vec<FilterFn>) -> FilterFn {
//     Box::new(move |point| filters.iter().any(|filter| filter(point)))
// }
//
// /// Create a filter function that checks if a point's metadata matches a value
// fn similar_filter(keys: Vec<&str>, reference: &str, name: &str) -> FilterFn {
//     // Check if this is a negated filter
//     let (is_negated, reference) = if reference.starts_with('-') {
//         (true, &reference[1..])
//     } else {
//         (false, reference)
//     };
//
//     // Determine if this is a regex or exact match
//     let matcher: Box<dyn Fn(&str) -> bool> = if reference.starts_with('/') {
//         // Create a regex matcher
//         let parts: Vec<&str> = reference.splitn(3, '/').collect();
//         if parts.len() < 3 {
//             // Invalid regex format
//             return Box::new(move |_| false);
//         }
//
//         let pattern = parts[1];
//         let flags = parts[2];
//
//         match Regex::new(&format!("(?{}){}", flags, pattern)) {
//             Ok(regex) => Box::new(move |val| regex.is_match(val)),
//             Err(_) => {
//                 // Invalid regex
//                 Box::new(move |_| false)
//             }
//         }
//     } else {
//         // Create an exact matcher
//         let reference = reference.to_string();
//         Box::new(move |val| val == reference)
//     };
//
//     // Create the filter function
//     Box::new(move |point| {
//         let result = keys.iter().any(|key| {
//             let value = match *key {
//                 "meta.name" => point
//                     .meta
//                     .get("name")
//                     .and_then(|v| v.as_str())
//                     .unwrap_or(""),
//                 "meta.tags" => {
//                     if let Some(Value::Array(tags)) = point.meta.get("tags") {
//                         // Check if any tag matches
//                         return tags
//                             .iter()
//                             .any(|tag| tag.as_str().map_or(false, |t| matcher(t)));
//                     }
//                     // No tags
//                     ""
//                 }
//                 _ => {
//                     // Get a value from a nested path
//                     let parts: Vec<&str> = key.split('.').collect();
//                     let mut current = &Value::Object(serde_json::Map::from_iter(
//                         point.meta.iter().map(|(k, v)| (k.clone(), v.clone())),
//                     ));
//
//                     for part in parts {
//                         current = match current.get(part) {
//                             Some(val) => val,
//                             None => return false,
//                         };
//                     }
//
//                     current.as_str().unwrap_or("")
//                 }
//             };
//
//             matcher(value)
//         });
//
//         if is_negated {
//             !result
//         } else {
//             result
//         }
//     })
// }
//
// /// Parse a simple filter expression
// fn simple_filter(exp: &str, name: &str) -> FilterFn {
//     let mut keys = vec!["meta.name", "meta.tags"];
//     let mut op = "~";
//     let mut value = exp.to_string();
//
//     let parts: Vec<&str> = exp.split_whitespace().collect();
//
//     // Parse the expression based on its format
//     if parts.len() >= 2 && ["~"].contains(&parts[1]) {
//         // Format: "key1,key2 ~ value"
//         keys = parts[0].split(',').collect();
//         op = parts[1];
//         value = parts[2..].join(" ");
//     } else if parts.len() >= 1 && ["~"].contains(&parts[0]) {
//         // Format: "~ value"
//         op = parts[0];
//         value = parts[1..].join(" ");
//     }
//
//     // Currently only the "~" (similar) operator is supported
//     match op {
//         "~" => similar_filter(keys, &value, name),
//         _ => always_false(),
//     }
// }
//
// /// Parse a complex filter expression
// fn complex_filter(
//     config: &Value,
//     name: &str,
//     units: &HashMap<String, f64>,
//     aggregator: fn(Vec<FilterFn>) -> FilterFn,
// ) -> FilterFn {
//     match config {
//         Value::Bool(b) => {
//             if *b {
//                 always_true()
//             } else {
//                 always_false()
//             }
//         }
//         Value::String(s) => simple_filter(s, name),
//         Value::Array(arr) => {
//             // Alternate between AND and OR for nested arrays
//             let alternate = if aggregator as usize == and_filter as usize {
//                 or_filter
//             } else {
//                 and_filter
//             };
//
//             let filters = arr
//                 .iter()
//                 .map(|elem| complex_filter(elem, name, units, alternate))
//                 .collect();
//
//             aggregator(filters)
//         }
//         _ => {
//             // Unexpected type
//             always_false()
//         }
//     }
// }
//
// /// Check if a value contains any objects (recursively)
// fn contains_object(val: &Value) -> bool {
//     match val {
//         Value::Object(_) => true,
//         Value::Array(arr) => arr.iter().any(contains_object),
//         _ => false,
//     }
// }
//
// /// Parse a filter configuration and return matching points
// pub fn parse(
//     config: &Value,
//     name: &str,
//     points: &HashMap<String, Point>,
//     units: &HashMap<String, f64>,
//     asym: &str,
// ) -> Result<Vec<Point>> {
//     let mut result = Vec::new();
//
//     // If config is undefined, return a default point at [0, 0]
//     if config.is_null() {
//         result.push(Point::default());
//     }
//     // If config contains an object, it's an anchor
//     else if contains_object(config) {
//         if ["source", "both"].contains(&asym) {
//             let point = anchor::parse(config, name, points, None, false, units)?;
//             result.push(point);
//         }
//
//         if ["clone", "both"].contains(&asym) {
//             let clone = anchor::parse(config, name, points, None, true, units)?;
//
//             // Check for duplicates
//             if !result.iter().any(|p| p.equals(&clone)) {
//                 result.push(clone);
//             }
//         }
//     }
//     // Otherwise, it's a filter condition
//     else {
//         let filter = complex_filter(config, name, units, or_filter);
//
//         // Apply the filter to all points
//         let filtered: Vec<Point> = points.values().filter(|p| filter(p)).cloned().collect();
//
//         if ["source", "both"].contains(&asym) {
//             result.extend(filtered.clone());
//         }
//
//         if ["clone", "both"].contains(&asym) {
//             // Find the mirrored versions of the filtered points
//             for point in filtered {
//                 if let Some(name) = point.meta.get("name").and_then(|n| n.as_str()) {
//                     let mirrored_name = anchor::mirror_ref(name, true);
//                     if let Some(mirrored) = points.get(&mirrored_name) {
//                         if !result.iter().any(|p| {
//                             p.meta.get("name").and_then(|n| n.as_str()) == Some(&mirrored_name)
//                         }) {
//                             result.push(mirrored.clone());
//                         }
//                     }
//                 }
//             }
//         }
//     }
//
//     Ok(result)
// }
