// use indexmap::IndexMap;
// use nalgebra::{Point2, Rotation2};
// use serde_json::{json, Map, Value};
// use std::f64::consts::PI;
//
// use crate::json::{expect_keys, expect_type, get_object, JsonType};
// use crate::point::Point;
// use crate::{anchor, units, utils, Error, Result};
//
// /// Render a zone of keys
// pub fn render_zone(
//     zone_name: &str,
//     zone: &Value,
//     anchor: &Point,
//     global_key: &Value,
//     units: &IndexMap<String, f64>,
// ) -> Result<IndexMap<String, Point>> {
//     expect_keys(zone, &format!("points.zones.{}", zone_name), &["columns", "rows", "key"])?;
//
//     let cols = get_object(cols, "columns", &format!("points.zones.{}.columns", zone_name))?;
//     zone.insert("columns".to_string(), cols);
//
//     todo!()
// }
//
// /// Parse an axis value for mirroring
// pub fn parse_axis(
//     config: &Value,
//     name: &str,
//     points: &IndexMap<String, Point>,
//     units: &IndexMap<String, f64>,
// ) -> Result<Option<f64>> {
//     match config {
//         Value::Object(obj) => {
//             let distance = if let Some(dist) = obj.get("distance") {
//                 match dist {
//                     Value::Number(n) => n.as_f64().unwrap_or(0.0),
//                     Value::String(s) => units::evaluate_mathnum(&Value::String(s.clone()), units)?,
//                     _ => 0.0,
//                 }
//             } else {
//                 0.0
//             };
//
//             // Create a copy of the object without the distance field
//             let mut obj_copy = obj.clone();
//             obj_copy.remove("distance");
//
//             // Parse as an anchor
//             let axis_point =
//                 anchor::parse(&Value::Object(obj_copy), name, points, None, false, units)?;
//
//             Ok(Some(axis_point.x + distance / 2.0))
//         }
//         Value::Number(n) => Ok(Some(n.as_f64().unwrap_or(0.0))),
//         Value::String(s) => {
//             let val = units::evaluate_mathnum(&Value::String(s.clone()), units)?;
//             Ok(Some(val))
//         }
//         Value::Null => Ok(None),
//         _ => Err(Error::TypeError {
//             field: name.to_string(),
//             expected: "number, object, or null".to_string(),
//         }),
//     }
// }
//
// /// Perform mirroring of a point around an axis
// pub fn perform_mirror(point: &Point, axis: f64) -> (Option<String>, Option<Point>) {
//     // Skip if point is already marked as mirrored or is asym=source
//     if point
//         .meta
//         .get("mirrored")
//         .is_some_and(|v| v.as_bool().unwrap_or(false))
//     {
//         return (None, None);
//     }
//
//     if point
//         .meta
//         .get("asym")
//         .is_some_and(|v| v.as_str().unwrap_or("") == "source")
//     {
//         return (None, None);
//     }
//
//     // Create a mirrored point
//     let mut mp = point.clone();
//     mp.mirror(axis);
//
//     // Update metadata
//     let mirrored_name = format!(
//         "mirror_{}",
//         point
//             .meta
//             .get("name")
//             .map_or("unknown", |v| v.as_str().unwrap_or("unknown"))
//     );
//     mp.meta.insert("mirrored".to_string(), Value::Bool(true));
//     mp.meta
//         .insert("name".to_string(), Value::String(mirrored_name.clone()));
//
//     if let Some(colrow) = point.meta.get("colrow").and_then(|v| v.as_str()) {
//         mp.meta.insert(
//             "colrow".to_string(),
//             Value::String(format!("mirror_{}", colrow)),
//         );
//     }
//
//     // If original point is asym=clone, mark it to be skipped
//     if point
//         .meta
//         .get("asym")
//         .is_some_and(|v| v.as_str().unwrap_or("") == "clone")
//     {
//         // We can't modify the original point here, so we return this info
//         return (Some(mirrored_name), Some(mp));
//     }
//
//     (Some(mirrored_name), Some(mp))
// }
//
// /// Apply autobind to points based on their positions
// pub fn perform_autobind(
//     points: &mut IndexMap<String, Point>,
//     units: &IndexMap<String, f64>,
// ) -> Result<()> {
//     // Group points by zone and column
//     let mut bounds = IndexMap::new();
//     let mut col_lists: indexmap::IndexMap<String, Vec<String>> = IndexMap::new();
//
//     // Helper to get the zone name with mirror prefix if needed
//     let mirrorzone = |p: &Point| -> String {
//         let mut zone = p
//             .meta
//             .get("zone")
//             .and_then(|z| z.get("name"))
//             .and_then(|n| n.as_str())
//             .unwrap_or("unknown")
//             .to_string();
//
//         if p.meta
//             .get("mirrored")
//             .is_some_and(|m| m.as_bool().unwrap_or(false))
//         {
//             zone = format!("mirror_{}", zone);
//         }
//
//         zone
//     };
//
//     // Round one: get column upper/lower bounds and per-zone column lists
//     for p in points.values() {
//         let zone = mirrorzone(p);
//         let col = p
//             .meta
//             .get("col")
//             .and_then(|c| c.get("name"))
//             .and_then(|n| n.as_str())
//             .unwrap_or("unknown")
//             .to_string();
//
//         // Initialize zone and column if needed
//         if !bounds.contains_key(&zone) {
//             bounds.insert(zone.clone(), IndexMap::new());
//         }
//
//         if !bounds[&zone].contains_key(&col) {
//             bounds
//                 .get_mut(&zone)
//                 .unwrap()
//                 .insert(col.clone(), (f64::MAX, f64::MIN));
//         }
//
//         println!("p is: {:#?}", p);
//         let columns = p
//             .meta
//             .get("zone")
//             .and_then(|z| z.get("columns"))
//             .and_then(|c| c.as_object())
//             .map(|obj| obj.keys().cloned().collect::<Vec<String>>())
//             .unwrap_or_default();
//
//         println!("col_lists before: {:?}", col_lists);
//         println!("columns: {:?}", columns);
//         if !col_lists.contains_key(&zone) {
//             col_lists.insert(zone.clone(), columns);
//         }
//
//         // Update bounds
//         let (min, max) = bounds.get_mut(&zone).unwrap().get_mut(&col).unwrap();
//         *min = min.min(p.y);
//         *max = max.max(p.y);
//     }
//
//     println!("col_lists1: {:?}", col_lists);
//
//     // Round two: apply autobind as appropriate
//     for p in points.values_mut() {
//         let autobind = match p.meta.get("autobind") {
//             Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
//             Some(Value::String(s)) => units::evaluate_mathnum(&Value::String(s.clone()), units)?,
//             _ => continue,
//         };
//
//         if autobind == 0.0 {
//             continue;
//         }
//
//         let zone = mirrorzone(p);
//         let col = p
//             .meta
//             .get("col")
//             .and_then(|c| c.get("name"))
//             .and_then(|n| n.as_str())
//             .unwrap_or("unknown")
//             .to_string();
//
//         // Get column list and bounds
//         let col_list = match col_lists.get(&zone) {
//             Some(list) => list,
//             None => continue,
//         };
//
//         let (col_min, col_max) = match bounds.get(&zone).and_then(|b| b.get(&col)) {
//             Some(&(min, max)) => (min, max),
//             None => continue,
//         };
//
//         println!("zone: {}", zone);
//         println!("col: {}", col);
//         println!("col_list: {:?}", col_list);
//         println!("col_min: {col_min} col_max: {col_max}");
//
//         // Get or initialize bind
//         let mut bind = match p.meta.get("bind") {
//             Some(Value::Array(arr)) => {
//                 let mut bind = vec![-1.0, -1.0, -1.0, -1.0];
//                 for (i, val) in arr.iter().enumerate().take(4) {
//                     if let Some(n) = val.as_f64() {
//                         bind[i] = n;
//                     } else if let Some(s) = val.as_str() {
//                         bind[i] = units::evaluate_mathnum(&Value::String(s.to_string()), units)?;
//                     }
//                 }
//                 bind
//             }
//             _ => vec![-1.0, -1.0, -1.0, -1.0],
//         };
//
//         // Up
//         println!("0: {}", bind[0]);
//         if bind[0] == -1.0 {
//             println!(" bind 0 p.y: {} col_max: {}", p.y, col_max);
//             bind[0] = if p.y < col_max { autobind } else { 0.0 };
//         }
//
//         // Down
//         if bind[2] == -1.0 {
//             bind[2] = if p.y > col_min { autobind } else { 0.0 };
//         }
//
//         // Left
//         if bind[3] == -1.0 {
//             bind[3] = 0.0;
//             let col_index = col_list.iter().position(|c| c == &col);
//             if let Some(index) = col_index {
//                 if index > 0 {
//                     let left_col = &col_list[index - 1];
//                     if let Some(&(left_min, left_max)) =
//                         bounds.get(&zone).and_then(|b| b.get(left_col))
//                     {
//                         if p.y >= left_min && p.y <= left_max {
//                             bind[3] = autobind;
//                         }
//                     }
//                 }
//             }
//         }
//
//         // Right
//         if bind[1] == -1.0 {
//             bind[1] = 0.0;
//             let col_index = col_list.iter().position(|c| c == &col);
//             if let Some(index) = col_index {
//                 if index < col_list.len() - 1 {
//                     let right_col = &col_list[index + 1];
//                     if let Some(&(right_min, right_max)) =
//                         bounds.get(&zone).and_then(|b| b.get(right_col))
//                     {
//                         if p.y >= right_min && p.y <= right_max {
//                             bind[1] = autobind;
//                         }
//                     }
//                 }
//             }
//         }
//
//         println!("{:?}", bind);
//
//         // Update the bind in metadata
//         p.meta.insert("bind".to_string(), json!(bind));
//     }
//
//     Ok(())
// }
//
// /// Parse the points configuration and generate key positions
// pub fn parse(config: &Value, units: &IndexMap<String, f64>) -> Result<IndexMap<String, Point>> {
//     // Config sanitization
//     if !config.is_object() {
//         return Err(Error::TypeError {
//             field: "points".to_string(),
//             expected: "object".to_string(),
//         });
//     }
//
//     let config_obj = config.as_object().unwrap();
//
//     // Extract zones
//     let zones = match config_obj.get("zones") {
//         Some(Value::Object(z)) => z,
//         _ => {
//             return Err(Error::MissingField("points.zones".to_string()));
//         }
//     };
//
//     // Extract global key config
//     let global_key = match config_obj.get("key") {
//         Some(Value::Object(k)) => Value::Object(k.clone()),
//         _ => Value::Object(Map::new()),
//     };
//
//     // Extract global rotate
//     let global_rotate = match config_obj.get("rotate") {
//         Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
//         Some(Value::String(s)) => units::evaluate_mathnum(&Value::String(s.clone()), units)?,
//         _ => 0.0,
//     };
//
//     // Extract global mirror
//     let global_mirror = config_obj.get("mirror");
//
//     // Collect all points
//     let mut points = IndexMap::new();
//
//     // Render zones
//     for (zone_name, zone_value) in zones.iter() {
//         // Zone sanitization
//         let zone = match zone_value {
//             Value::Object(z) => Value::Object(z.clone()),
//             _ => Value::Object(Map::new()),
//         };
//         let zone_obj = zone.as_object().unwrap();
//
//         // Extract keys that are handled here, not at the zone render level
//         let empty = Value::Object(Map::new());
//         let anchor_value = zone_obj.get("anchor").unwrap_or(&empty);
//         let anchor = anchor::parse(
//             anchor_value,
//             &format!("points.zones.{}.anchor", zone_name),
//             &points,
//             None,
//             false,
//             units,
//         )?;
//
//         let zone_rotate = match zone_obj.get("rotate") {
//             Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
//             Some(Value::String(s)) => units::evaluate_mathnum(&Value::String(s.clone()), units)?,
//             _ => 0.0,
//         };
//
//         let zone_mirror = zone_obj.get("mirror");
//
//         // Create a copy of the zone without these special keys
//         let mut zone_copy = zone_obj.clone();
//         zone_copy.remove("anchor");
//         zone_copy.remove("rotate");
//         zone_copy.remove("mirror");
//
//         // Creating new points
//         let mut new_points = render_zone(
//             zone_name,
//             &Value::Object(zone_copy),
//             &anchor,
//             &global_key,
//             units,
//         )?;
//
//         // Simplify the names in individual point "zones" and single-key columns
//         let mut keys_to_rename = Vec::new();
//         for key in new_points.keys() {
//             if key.ends_with("_default") {
//                 keys_to_rename.push(key.clone());
//             }
//         }
//
//         for key in keys_to_rename {
//             let new_key = key[0..key.len() - 8].to_string();
//             if let Some(mut point) = new_points.shift_remove(&key) {
//                 point
//                     .meta
//                     .insert("name".to_string(), Value::String(new_key.clone()));
//                 new_points.insert(new_key, point);
//             }
//         }
//
//         // Adjust new points (per-zone rotation)
//         for point in new_points.values_mut() {
//             if zone_rotate != 0.0 {
//                 point.rotate(zone_rotate, None, false);
//             }
//         }
//
//         // Add new points so they can be referenced
//         for (name, point) in &new_points {
//             if points.contains_key(name) {
//                 return Err(Error::Config(format!(
//                     "Key \"{}\" defined more than once!",
//                     name
//                 )));
//             }
//             points.insert(name.clone(), point.clone());
//         }
//
//         // Per-zone mirroring for new keys
//         if let Some(axis) = parse_axis(
//             zone_mirror.unwrap_or(&Value::Null),
//             &format!("points.zones.{}.mirror", zone_name),
//             &points,
//             units,
//         )? {
//             let mut mirrored_points = IndexMap::new();
//
//             for point in new_points.values() {
//                 let (mname, mp) = perform_mirror(point, axis);
//                 if let (Some(name), Some(point)) = (mname, mp) {
//                     mirrored_points.insert(name, point);
//                 }
//             }
//
//             // Add mirrored points to the collection
//             for (name, point) in mirrored_points {
//                 points.insert(name, point);
//             }
//         }
//     }
//
//     // Apply global rotation
//     if global_rotate != 0.0 {
//         for point in points.values_mut() {
//             point.rotate(global_rotate, None, false);
//         }
//     }
//
//     // Global mirroring for points that haven't been mirrored yet
//     if let Some(axis) = parse_axis(
//         global_mirror.unwrap_or(&Value::Null),
//         "points.mirror",
//         &points,
//         units,
//     )? {
//         let mut mirrored_points = IndexMap::new();
//
//         for point in points.values() {
//             if point.meta.get("mirrored").is_none() {
//                 let (mname, mp) = perform_mirror(point, axis);
//                 if let (Some(name), Some(point)) = (mname, mp) {
//                     mirrored_points.insert(name, point);
//                 }
//             }
//         }
//
//         // Add global mirrored points
//         for (name, point) in mirrored_points {
//             points.insert(name, point);
//         }
//     }
//
//     // Remove temporary points
//     let mut filtered = IndexMap::new();
//     for (name, point) in &points {
//         if point
//             .meta
//             .get("skip")
//             .is_some_and(|v| v.as_bool().unwrap_or(false))
//         {
//             continue;
//         }
//         filtered.insert(name.clone(), point.clone());
//     }
//
//     // Apply autobind
//     perform_autobind(&mut filtered, units)?;
//
//     Ok(filtered)
// }
//
// /// Generate a visual representation of the points
// pub fn visualize(points: &IndexMap<String, Point>, units: &IndexMap<String, f64>) -> Value {
//     let mut models = Map::new();
//
//     for (name, point) in points {
//         let width = point
//             .meta
//             .get("width")
//             .and_then(|w| w.as_f64())
//             .unwrap_or(18.0);
//
//         let height = point
//             .meta
//             .get("height")
//             .and_then(|h| h.as_f64())
//             .unwrap_or(18.0);
//
//         // Create a rectangle centered at the point
//         let rect = utils::rect(width, height, Some([-width / 2.0, -height / 2.0]));
//
//         // Position the rectangle at the point (in a real implementation, this would apply rotation)
//         // Here we just create a JSON representation
//         models.insert(
//             name.clone(),
//             json!({
//                 "x": point.x,
//                 "y": point.y,
//                 "r": point.r,
//                 "width": width,
//                 "height": height
//             }),
//         );
//     }
//
//     json!({ "models": models })
// }
//
// /// Add a rotation to a list of rotations
// pub fn push_rotation(list: &mut Vec<RotationStep>, angle: f64, origin: [f64; 2], _resist: bool) {
//     let mut candidate = origin;
//
//     for r in list.iter() {
//         // Apply all previous rotations to the origin
//         let rot = Rotation2::new(r.angle * PI / 180.0);
//         let origin_point = Point2::new(candidate[0], candidate[1]);
//         let r_origin_point = Point2::new(r.origin[0], r.origin[1]);
//
//         let translated = origin_point - r_origin_point.coords;
//         let rotated = rot * translated;
//         let result_x = r_origin_point.x + rotated.x;
//         let result_y = r_origin_point.y + rotated.y;
//         let result = Point2::new(result_x, result_y);
//
//         candidate = [result.x, result.y];
//     }
//
//     list.push(RotationStep {
//         angle,
//         origin: candidate,
//     });
// }
//
// /// A step in a series of rotations
// #[derive(Debug, Clone)]
// pub struct RotationStep {
//     pub angle: f64,
//     pub origin: [f64; 2],
// }
//
// #[cfg(test)]
// mod tests {
//     use assert_json_diff::assert_json_include;
//     use utils::convert_numbers_to_f64;
//
//     use super::*;
//
//     #[test]
//     fn point_fixtures() {
//         let config = include_str!("../tests/points/basic_2x2.yaml");
//         let config = serde_yaml::from_str::<Value>(config).unwrap();
//         let points = config.get("points").unwrap();
//         let points = parse(points, &IndexMap::new()).unwrap();
//         let points = json!(points);
//
//         let expected = include_str!("../tests/points/basic_2x2___points.json");
//         let expected = convert_numbers_to_f64(serde_json::from_str::<Value>(expected).unwrap());
//         assert_json_include!(actual: points, expected: expected);
//     }
// }
