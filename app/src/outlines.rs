// use serde_json::{json, Map, Value};
// use std::collections::HashMap;
//
// use crate::{anchor, filter, operation, point::Point, units, utils, Error, Result};
//
// /// Apply binding to a base outline using a bounding box and point metadata
// fn binding(
//     base: Value,
//     bbox: &utils::BBox,
//     point: &Point,
//     units: &HashMap<String, f64>,
// ) -> Result<Value> {
//     // Get bind values from point metadata (top, right, bottom, left)
//     let bind = match point.meta.get("bind") {
//         Some(Value::Array(arr)) if arr.len() >= 4 => {
//             let mut values = [0.0; 4];
//             for (i, val) in arr.iter().enumerate().take(4) {
//                 values[i] = match val {
//                     Value::Number(n) => n.as_f64().unwrap_or(0.0),
//                     Value::String(s) => units::evaluate_mathnum(&Value::String(s.clone()), units)?,
//                     _ => 0.0,
//                 };
//             }
//             values
//         }
//         _ => [0.0; 4],
//     };
//
//     // If it's a mirrored key, swap left and right bind values
//     let bind = if point
//         .meta
//         .get("mirrored")
//         .map_or(false, |v| v.as_bool().unwrap_or(false))
//     {
//         [bind[0], bind[3], bind[2], bind[1]]
//     } else {
//         bind
//     };
//
//     // Calculate binding dimensions
//     let bt = bbox.high[1].max(0.0) + bind[0].max(0.0);
//     let br = bbox.high[0].max(0.0) + bind[1].max(0.0);
//     let bd = bbox.low[1].min(0.0) - bind[2].max(0.0);
//     let bl = bbox.low[0].min(0.0) - bind[3].max(0.0);
//
//     let mut result = base;
//
//     // Apply binding rectangles
//     if bind[0] != 0.0 || bind[1] != 0.0 {
//         let rect = utils::rect(br, bt, None);
//         result = utils::union(&result, &json!(rect))?;
//     }
//
//     if bind[1] != 0.0 || bind[2] != 0.0 {
//         let rect = utils::rect(br, -bd, Some([0.0, bd]));
//         result = utils::union(&result, &json!(rect))?;
//     }
//
//     if bind[2] != 0.0 || bind[3] != 0.0 {
//         let rect = utils::rect(-bl, -bd, Some([bl, bd]));
//         result = utils::union(&result, &json!(rect))?;
//     }
//
//     if bind[3] != 0.0 || bind[0] != 0.0 {
//         let rect = utils::rect(-bl, bt, Some([bl, 0.0]));
//         result = utils::union(&result, &json!(rect))?;
//     }
//
//     Ok(result)
// }
//
// /// Create a rectangle outline
// fn rectangle(
//     config: &Value,
//     name: &str,
//     points: &HashMap<String, Point>,
//     outlines: &HashMap<String, Value>,
//     units: &HashMap<String, f64>,
// ) -> Result<(
//     Box<dyn Fn(&Point) -> (Value, utils::BBox)>,
//     HashMap<String, f64>,
// )> {
//     // Parameter validation
//     if !config.is_object() {
//         return Err(Error::TypeError {
//             field: name.to_string(),
//             expected: "object".to_string(),
//         });
//     }
//
//     let config_obj = config.as_object().unwrap();
//
//     // Check for unexpected keys
//     for key in config_obj.keys() {
//         if !["size", "corner", "bevel"].contains(&key.as_str()) {
//             return Err(Error::Config(format!(
//                 "Unexpected key \"{}\" in \"{}\"",
//                 key, name
//             )));
//         }
//     }
//
//     // Get size parameter
//     let size_val = config_obj
//         .get("size")
//         .ok_or_else(|| Error::MissingField(format!("{}.size", name)))?;
//     let size = match size_val {
//         Value::Array(arr) if arr.len() >= 2 => [
//             units::evaluate_mathnum(&arr[0], units)?,
//             units::evaluate_mathnum(&arr[1], units)?,
//         ],
//         Value::Number(n) => {
//             let val = n.as_f64().unwrap_or(0.0);
//             [val, val]
//         }
//         Value::String(s) => {
//             let val = units::evaluate_mathnum(&Value::String(s.clone()), units)?;
//             [val, val]
//         }
//         _ => {
//             return Err(Error::TypeError {
//                 field: format!("{}.size", name),
//                 expected: "array, number, or string".to_string(),
//             });
//         }
//     };
//
//     // Create rectangle-specific units
//     let mut rec_units = units.clone();
//     rec_units.insert("sx".to_string(), size[0]);
//     rec_units.insert("sy".to_string(), size[1]);
//
//     // Get corner parameter
//     let corner = match config_obj.get("corner") {
//         Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
//         Some(Value::String(s)) => units::evaluate_mathnum(&Value::String(s.clone()), &rec_units)?,
//         None => 0.0,
//         _ => {
//             return Err(Error::TypeError {
//                 field: format!("{}.corner", name),
//                 expected: "number or string".to_string(),
//             });
//         }
//     };
//
//     // Get bevel parameter
//     let bevel = match config_obj.get("bevel") {
//         Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
//         Some(Value::String(s)) => units::evaluate_mathnum(&Value::String(s.clone()), &rec_units)?,
//         None => 0.0,
//         _ => {
//             return Err(Error::TypeError {
//                 field: format!("{}.bevel", name),
//                 expected: "number or string".to_string(),
//             });
//         }
//     };
//
//     // Return shape function and its units
//     Ok((
//         Box::new(move |_point: &Point| {
//             let w = size[0];
//             let h = size[1];
//             let mod_val = 2.0 * (corner + bevel);
//             let cw = w - mod_val;
//             let ch = h - mod_val;
//
//             // Validate dimensions
//             if cw < 0.0 {
//                 panic!("Rectangle for \"{}\" isn't wide enough for its corner and bevel ({} - 2 * {} - 2 * {} <= 0)!",
//                        name, w, corner, bevel);
//             }
//
//             if ch < 0.0 {
//                 panic!("Rectangle for \"{}\" isn't tall enough for its corner and bevel ({} - 2 * {} - 2 * {} <= 0)!",
//                        name, h, corner, bevel);
//             }
//
//             // Create basic rectangle
//             let mut rect = if bevel > 0.0 {
//                 // Create beveled rectangle
//                 let poly_points = vec![
//                     [-bevel, 0.0],
//                     [-bevel, ch],
//                     [0.0, ch + bevel],
//                     [cw, ch + bevel],
//                     [cw + bevel, ch],
//                     [cw + bevel, 0.0],
//                     [cw, -bevel],
//                     [0.0, -bevel],
//                 ];
//                 utils::poly(&poly_points)
//             } else {
//                 // Create regular rectangle
//                 json!({
//                     "width": cw,
//                     "height": ch,
//                 })
//             };
//
//             // Apply corner rounding if needed
//             if corner > 0.0 {
//                 rect = utils::outline(&rect, corner, 0, false)?;
//             }
//
//             // Center the rectangle
//             // In a real implementation, this would translate the rectangle
//
//             // Calculate bounding box
//             let bbox = utils::BBox {
//                 low: [-w / 2.0, -h / 2.0],
//                 high: [w / 2.0, h / 2.0],
//             };
//
//             Ok((rect, bbox))
//         }) as Box<dyn Fn(&Point) -> (Value, utils::BBox)>,
//         rec_units,
//     ))
// }
//
// /// Create a circle outline
// fn circle(
//     config: &Value,
//     name: &str,
//     points: &HashMap<String, Point>,
//     outlines: &HashMap<String, Value>,
//     units: &HashMap<String, f64>,
// ) -> Result<(
//     Box<dyn Fn(&Point) -> (Value, utils::BBox)>,
//     HashMap<String, f64>,
// )> {
//     // Parameter validation
//     if !config.is_object() {
//         return Err(Error::TypeError {
//             field: name.to_string(),
//             expected: "object".to_string(),
//         });
//     }
//
//     let config_obj = config.as_object().unwrap();
//
//     // Check for unexpected keys
//     for key in config_obj.keys() {
//         if key != "radius" {
//             return Err(Error::Config(format!(
//                 "Unexpected key \"{}\" in \"{}\"",
//                 key, name
//             )));
//         }
//     }
//
//     // Get radius parameter
//     let radius_val = config_obj
//         .get("radius")
//         .ok_or_else(|| Error::MissingField(format!("{}.radius", name)))?;
//     let radius = match radius_val {
//         Value::Number(n) => n.as_f64().unwrap_or(0.0),
//         Value::String(s) => units::evaluate_mathnum(&Value::String(s.clone()), units)?,
//         _ => {
//             return Err(Error::TypeError {
//                 field: format!("{}.radius", name),
//                 expected: "number or string".to_string(),
//             });
//         }
//     };
//
//     // Create circle-specific units
//     let mut circ_units = units.clone();
//     circ_units.insert("r".to_string(), radius);
//
//     // Return shape function and its units
//     Ok((
//         Box::new(move |_point: &Point| {
//             // Create circle
//             let circle = utils::circle([0.0, 0.0], radius);
//
//             // Calculate bounding box
//             let bbox = utils::BBox {
//                 low: [-radius, -radius],
//                 high: [radius, radius],
//             };
//
//             (json!(circle), bbox)
//         }),
//         circ_units,
//     ))
// }
//
// /// Create a polygon outline
// fn polygon(
//     config: &Value,
//     name: &str,
//     points: &HashMap<String, Point>,
//     outlines: &HashMap<String, Value>,
//     units: &HashMap<String, f64>,
// ) -> Result<(
//     Box<dyn Fn(&Point) -> (Value, utils::BBox)>,
//     HashMap<String, f64>,
// )> {
//     // Parameter validation
//     if !config.is_object() {
//         return Err(Error::TypeError {
//             field: name.to_string(),
//             expected: "object".to_string(),
//         });
//     }
//
//     let config_obj = config.as_object().unwrap();
//
//     // Check for unexpected keys
//     for key in config_obj.keys() {
//         if key != "points" {
//             return Err(Error::Config(format!(
//                 "Unexpected key \"{}\" in \"{}\"",
//                 key, name
//             )));
//         }
//     }
//
//     // Get points parameter
//     let poly_points = match config_obj.get("points") {
//         Some(Value::Array(arr)) => arr.clone(),
//         _ => {
//             return Err(Error::TypeError {
//                 field: format!("{}.points", name),
//                 expected: "array".to_string(),
//             });
//         }
//     };
//
//     // Return shape function and its units
//     Ok((
//         Box::new(move |point: &Point| {
//             let mut parsed_points = Vec::new();
//             // The poly starts at [0, 0] as it will be positioned later
//             let mut last_anchor = Point::new(0.0, 0.0, 0.0, point.meta.clone());
//
//             for (i, poly_point) in poly_points.iter().enumerate() {
//                 let poly_name = format!("{}.points[{}]", name, i);
//                 last_anchor = anchor::parse(
//                     poly_point,
//                     &poly_name,
//                     points,
//                     Some(&last_anchor),
//                     false,
//                     units,
//                 )
//                 .unwrap();
//                 parsed_points.push(last_anchor.p());
//             }
//
//             let poly = utils::poly(&parsed_points);
//             let bbox = utils::bbox(&parsed_points);
//
//             (json!(poly), bbox)
//         }),
//         units.clone(),
//     ))
// }
//
// /// Use an existing outline
// fn outline(
//     config: &Value,
//     name: &str,
//     points: &HashMap<String, Point>,
//     outlines: &HashMap<String, Value>,
//     units: &HashMap<String, f64>,
// ) -> Result<(
//     Box<dyn Fn(&Point) -> (Value, utils::BBox)>,
//     HashMap<String, f64>,
// )> {
//     // Parameter validation
//     if !config.is_object() {
//         return Err(Error::TypeError {
//             field: name.to_string(),
//             expected: "object".to_string(),
//         });
//     }
//
//     let config_obj = config.as_object().unwrap();
//
//     // Check for unexpected keys
//     for key in config_obj.keys() {
//         if !["name", "origin"].contains(&key.as_str()) {
//             return Err(Error::Config(format!(
//                 "Unexpected key \"{}\" in \"{}\"",
//                 key, name
//             )));
//         }
//     }
//
//     // Get referenced outline name
//     let outline_name = match config_obj.get("name") {
//         Some(Value::String(s)) => s.clone(),
//         _ => {
//             return Err(Error::TypeError {
//                 field: format!("{}.name", name),
//                 expected: "string".to_string(),
//             });
//         }
//     };
//
//     // Ensure the referenced outline exists
//     if !outlines.contains_key(&outline_name) {
//         return Err(Error::Config(format!(
//             "Field \"{}.name\" does not name an existing outline!",
//             name
//         )));
//     }
//
//     // Get origin
//     let origin_val = config_obj
//         .get("origin")
//         .unwrap_or(&Value::Object(Map::new()));
//
//     // Create closure to capture the referenced outline
//     let outline_ref = outlines[&outline_name].clone();
//
//     // Return shape function and its units
//     Ok((
//         Box::new(move |point: &Point| {
//             // Parse origin anchor
//             let origin = anchor::parse(
//                 origin_val,
//                 &format!("{}.origin", name),
//                 points,
//                 Some(point),
//                 false,
//                 units,
//             )
//             .unwrap();
//
//             // Unposition the outline based on origin
//             // In a real implementation, this would transform the outline
//             let o = outline_ref.clone();
//
//             // Calculate bounding box (in a real implementation, this would measure the outline)
//             let bbox = utils::BBox {
//                 low: [-10.0, -10.0], // Placeholder
//                 high: [10.0, 10.0],  // Placeholder
//             };
//
//             (o, bbox)
//         }),
//         units.clone(),
//     ))
// }
//
// /// Available outline creation methods
// type OutlineGeneratorMap = HashMap<
//     &'static str,
//     fn(
//         config: &Value,
//         name: &str,
//         points: &HashMap<String, Point>,
//         outlines: &HashMap<String, Value>,
//         units: &HashMap<String, f64>,
//     ) -> Result<(
//         Box<dyn Fn(&Point) -> (Value, utils::BBox)>,
//         HashMap<String, f64>,
//     )>,
// >;
//
// /// Expand shorthand notation for outline expansions
// fn expand_shorthand(config: &mut Value, name: &str, units: &HashMap<String, f64>) -> Result<()> {
//     if let Some(Value::String(expand)) = config.get("expand") {
//         if expand.len() > 1 {
//             let prefix = &expand[0..expand.len() - 1];
//             let suffix = &expand[expand.len() - 1..];
//
//             let valid_suffixes = [")", ">", "]"];
//             if !valid_suffixes.contains(&suffix) {
//                 return Err(Error::Config(format!(
//                     "If field \"{}\" is a string, it should end with one of [{}]!",
//                     name,
//                     valid_suffixes.join(", ")
//                 )));
//             }
//
//             // Replace expand with just the prefix
//             if let Some(obj) = config.as_object_mut() {
//                 obj.insert("expand".to_string(), Value::String(prefix.to_string()));
//
//                 // Set joints based on suffix if not already set
//                 if !obj.contains_key("joints") {
//                     let joint_index = valid_suffixes
//                         .iter()
//                         .position(|&s| s == suffix)
//                         .unwrap_or(0);
//                     obj.insert("joints".to_string(), Value::Number(joint_index.into()));
//                 }
//             }
//         }
//     }
//
//     // Convert string joints to numeric values
//     if let Some(Value::String(joints)) = config.get("joints") {
//         let joint_value = match joints.as_str() {
//             "round" => 0,
//             "pointy" => 1,
//             "beveled" => 2,
//             _ => return Err(Error::Config(format!("Invalid joints value: {}", joints))),
//         };
//
//         if let Some(obj) = config.as_object_mut() {
//             obj.insert("joints".to_string(), Value::Number(joint_value.into()));
//         }
//     }
//
//     Ok(())
// }
//
// /// Parse outlines configuration and generate outlines
// pub fn parse(
//     config: &Value,
//     points: &HashMap<String, Point>,
//     units: &HashMap<String, f64>,
// ) -> Result<HashMap<String, Value>> {
//     // Available outline generators
//     let outline_generators: OutlineGeneratorMap = [
//         ("rectangle", rectangle as fn(_, _, _, _, _) -> _),
//         ("circle", circle as fn(_, _, _, _, _) -> _),
//         ("polygon", polygon as fn(_, _, _, _, _) -> _),
//         ("outline", outline as fn(_, _, _, _, _) -> _),
//     ]
//     .iter()
//     .cloned()
//     .collect();
//
//     // Output outlines will be collected here
//     let mut outlines = HashMap::new();
//
//     // The config must be an actual object
//     if !config.is_object() {
//         return Err(Error::TypeError {
//             field: "outlines".to_string(),
//             expected: "object".to_string(),
//         });
//     }
//
//     // Process each outline
//     for (outline_name, parts) in config.as_object().unwrap() {
//         // Create a placeholder for the current outline
//         outlines.insert(
//             outline_name.clone(),
//             json!({
//                 "models": {}
//             }),
//         );
//
//         // Convert array to object if needed
//         let parts_obj = match parts {
//             Value::Array(arr) => {
//                 let mut obj = Map::new();
//                 for (i, part) in arr.iter().enumerate() {
//                     obj.insert(i.to_string(), part.clone());
//                 }
//                 Value::Object(obj)
//             }
//             Value::Object(_) => parts.clone(),
//             _ => {
//                 return Err(Error::TypeError {
//                     field: format!("outlines.{}", outline_name),
//                     expected: "object or array".to_string(),
//                 });
//             }
//         };
//
//         // Process each part
//         for (part_name, mut part) in parts_obj.as_object().unwrap().clone() {
//             let name = format!("outlines.{}.{}", outline_name, part_name);
//
//             // String part-shortcuts are expanded first
//             if let Value::String(s) = &part {
//                 let op_result = operation::operation(
//                     s,
//                     &maplit::hashmap! {
//                         "outline".to_string() => outlines.keys().cloned().collect::<Vec<String>>()
//                     },
//                     None,
//                 )?;
//
//                 part = json!({
//                     "operation": op_result.operation,
//                     "what": "outline",
//                     "name": op_result.name
//                 });
//             }
//
//             if !part.is_object() {
//                 return Err(Error::TypeError {
//                     field: name.clone(),
//                     expected: "object or string".to_string(),
//                 });
//             }
//
//             let part_obj = part.as_object().unwrap().clone();
//
//             // Process keys that are common to all part declarations
//             let operation_type = part_obj
//                 .get("operation")
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("add");
//
//             let what = part_obj
//                 .get("what")
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("outline");
//
//             let bound = part_obj
//                 .get("bound")
//                 .and_then(|v| v.as_bool())
//                 .unwrap_or(false);
//
//             let asym = part_obj
//                 .get("asym")
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("source");
//
//             // `where` is delayed until we have all, potentially what-dependent units
//             let original_where = part_obj.get("where").cloned();
//
//             let original_adjust = part_obj.get("adjust").cloned();
//
//             let fillet = match part_obj.get("fillet") {
//                 Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
//                 Some(Value::String(s)) => {
//                     units::evaluate_mathnum(&Value::String(s.clone()), units)?
//                 }
//                 _ => 0.0,
//             };
//
//             // Make a mutable copy of the part for expand_shorthand
//             let mut mutable_part = part.clone();
//             expand_shorthand(&mut mutable_part, &format!("{}.expand", name), units)?;
//
//             // Get expansion parameters
//             let expand = match mutable_part.get("expand") {
//                 Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
//                 Some(Value::String(s)) => {
//                     units::evaluate_mathnum(&Value::String(s.clone()), units)?
//                 }
//                 _ => 0.0,
//             };
//
//             let joints = match mutable_part.get("joints") {
//                 Some(Value::Number(n)) => n.as_u64().unwrap_or(0) as usize,
//                 _ => 0,
//             };
//
//             let scale = match part_obj.get("scale") {
//                 Some(Value::Number(n)) => n.as_f64().unwrap_or(1.0),
//                 Some(Value::String(s)) => {
//                     units::evaluate_mathnum(&Value::String(s.clone()), units)?
//                 }
//                 _ => 1.0,
//             };
//
//             // Create a filtered copy of the part with only keys specific to the outline type
//             let mut filtered_part = Map::new();
//             for (k, v) in part_obj {
//                 if ![
//                     "operation",
//                     "what",
//                     "bound",
//                     "asym",
//                     "where",
//                     "adjust",
//                     "fillet",
//                     "expand",
//                     "joints",
//                     "scale",
//                 ]
//                 .contains(&k.as_str())
//                 {
//                     filtered_part.insert(k.clone(), v.clone());
//                 }
//             }
//
//             // Get the shape maker function for this outline type
//             let generator = outline_generators.get(what).ok_or_else(|| {
//                 Error::Config(format!("Unknown outline type '{}' for '{}'", what, name))
//             })?;
//
//             let (shape_maker, shape_units) = generator(
//                 &Value::Object(filtered_part),
//                 &name,
//                 points,
//                 &outlines,
//                 units,
//             )?;
//
//             // Create a function to adjust a point
//             let adjust_fn = move |start: &Point| -> Point {
//                 anchor::parse(
//                     &original_adjust.clone().unwrap_or(Value::Object(Map::new())),
//                     &format!("{}.adjust", name),
//                     points,
//                     Some(start),
//                     false,
//                     &shape_units,
//                 )
//                 .unwrap_or_else(|_| start.clone())
//             };
//
//             // Filter points based on where clause
//             let where_points = filter::parse(
//                 &original_where.unwrap_or(Value::Null),
//                 &format!("{}.where", name),
//                 points,
//                 &shape_units,
//                 asym,
//             )?;
//
//             // Process each filtered point
//             for w in where_points {
//                 let point = adjust_fn(&w);
//                 let (mut shape, bbox) = shape_maker(&point);
//
//                 // Apply binding if requested
//                 if bound {
//                     shape = binding(shape, &bbox, &point, &shape_units)?;
//                 }
//
//                 // Position the shape according to the point
//                 // In a real implementation, this would transform the shape
//
//                 // Apply operation to combine with existing outline
//                 let current_outline = outlines.get_mut(outline_name).unwrap();
//
//                 match operation_type {
//                     "add" => *current_outline = utils::union(current_outline, &shape)?,
//                     "subtract" => *current_outline = utils::subtract(current_outline, &shape)?,
//                     "intersect" => *current_outline = utils::intersect(current_outline, &shape)?,
//                     "stack" => *current_outline = utils::stack(current_outline, &shape)?,
//                     _ => {
//                         return Err(Error::Config(format!(
//                             "Unknown operation type '{}' for '{}'",
//                             operation_type, name
//                         )));
//                     }
//                 }
//             }
//
//             // Apply scaling if needed
//             if scale != 1.0 {
//                 // In a real implementation, this would scale the outline
//             }
//
//             // Apply expansion if needed
//             if expand != 0.0 {
//                 // In a real implementation, this would expand/contract the outline
//             }
//
//             // Apply filleting if needed
//             if fillet != 0.0 {
//                 // In a real implementation, this would fillet the corners
//             }
//         }
//
//         // Final adjustments (originate and simplify)
//         // In a real implementation, these would transform the outline
//     }
//
//     Ok(outlines)
// }
