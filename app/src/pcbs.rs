// use serde_json::{json, Map, Value};
// use std::collections::HashMap;
//
// use crate::{anchor, filter, point::Point, utils, Error, Result};
//
// // Type definitions for PCB-related functionality
// type NetIndexer = Box<dyn Fn(&str) -> usize>;
// type ComponentIndexer = Box<dyn Fn(&str) -> String>;
// type FootprintFn = Box<dyn Fn(&Value, &str, &Point) -> Value>;
//
// // Available footprint types and PCB templates
// // In a real implementation, these would be imported from separate modules
// lazy_static::lazy_static! {
//     static ref FOOTPRINT_TYPES: HashMap<String, Footprint> = HashMap::new();
//     static ref TEMPLATE_TYPES: HashMap<String, Template> = HashMap::new();
// }
//
// /// A footprint definition
// struct Footprint {
//     params: HashMap<String, Value>,
//     body: Box<dyn Fn(&HashMap<String, Value>) -> Value>,
// }
//
// /// A PCB template definition
// struct Template {
//     convert_outline: Box<dyn Fn(&Value, &str) -> Value>,
//     body: Box<dyn Fn(&TemplateParams) -> String>,
// }
//
// /// Parameters for PCB template rendering
// struct TemplateParams<'a> {
//     name: &'a str,
//     version: &'a str,
//     author: &'a str,
//     nets: &'a [NetObject],
//     footprints: &'a [Value],
//     outlines: &'a HashMap<String, Value>,
//     custom: Option<&'a Value>,
// }
//
// /// A net object for PCB generation
// #[derive(Clone, Debug)]
// struct NetObject {
//     name: String,
//     index: usize,
// }
//
// impl NetObject {
//     fn new(name: String, index: usize) -> Self {
//         Self { name, index }
//     }
//
//     fn to_string(&self) -> String {
//         format!("(net {} \"{}\")", self.index, self.name)
//     }
// }
//
// /// An xy coordinate object for PCB generation
// #[derive(Clone, Debug)]
// struct XyObject {
//     x: f64,
//     y: f64,
// }
//
// impl XyObject {
//     fn new(x: f64, y: f64) -> Self {
//         Self { x, y }
//     }
//
//     fn to_string(&self) -> String {
//         format!("{} {}", self.x, self.y)
//     }
// }
//
// /// Inject a custom footprint
// pub fn inject_footprint(name: &str, fp: Footprint) {
//     // In a real implementation, this would add to FOOTPRINT_TYPES
// }
//
// /// Inject a custom template
// pub fn inject_template(name: &str, t: Template) {
//     // In a real implementation, this would add to TEMPLATE_TYPES
// }
//
// /// Create a footprint generation function
// fn footprint(
//     points: &HashMap<String, Point>,
//     net_indexer: NetIndexer,
//     component_indexer: ComponentIndexer,
//     units: &HashMap<String, f64>,
//     extra: &ExtraParams,
// ) -> FootprintFn {
//     Box::new(move |config, name, point| {
//         // Config sanitization
//         if !config.is_object() {
//             return json!({
//                 "error": format!("Expected object for '{}'", name)
//             });
//         }
//
//         let config_obj = config.as_object().unwrap();
//
//         // Check for unexpected keys
//         for key in config_obj.keys() {
//             if !["what", "params"].contains(&key.as_str()) {
//                 return json!({
//                     "error": format!("Unexpected key '{}' in '{}'", key, name)
//                 });
//             }
//         }
//
//         // Get footprint type
//         let what = match config_obj.get("what") {
//             Some(Value::String(s)) => s.as_str(),
//             _ => {
//                 return json!({
//                     "error": format!("Field '{}.what' must be a string", name)
//                 });
//             }
//         };
//
//         // Check if footprint type exists
//         if !FOOTPRINT_TYPES.contains_key(what) {
//             return json!({
//                 "error": format!("Unknown footprint type '{}' in '{}'", what, name)
//             });
//         }
//
//         let fp = &FOOTPRINT_TYPES[what];
//
//         // Get and sanitize parameters
//         let original_params = config_obj
//             .get("params")
//             .map(|p| {
//                 if let Value::Object(o) = p {
//                     o.clone()
//                 } else {
//                     Map::new()
//                 }
//             })
//             .unwrap_or_else(Map::new);
//
//         // Make a copy of parameters
//         let mut params = original_params.clone();
//
//         // Remove mirror config as it would be an unexpected field
//         params.remove("mirror");
//
//         // Override with mirror parameters when applicable
//         if point
//             .meta
//             .get("mirrored")
//             .map_or(false, |v| v.as_bool().unwrap_or(false))
//         {
//             if let Some(Value::Object(mirror_overrides)) = original_params.get("mirror") {
//                 for (k, v) in mirror_overrides {
//                     params.insert(k.clone(), v.clone());
//                 }
//             }
//         }
//
//         // Check for unexpected parameter keys
//         for key in params.keys() {
//             if !fp.params.contains_key(key) {
//                 return json!({
//                     "error": format!("Unexpected parameter '{}' in '{}.params'", key, name)
//                 });
//             }
//         }
//
//         // Parse parameters
//         let mut parsed_params = HashMap::new();
//
//         for (param_name, param_def) in &fp.params {
//             // Expand parameter definition shorthand
//             let (param_type, param_default) = match param_def {
//                 Value::String(s) => ("string", Value::String(s.clone())),
//                 Value::Number(n) => ("number", Value::Number(n.clone())),
//                 Value::Bool(b) => ("boolean", Value::Bool(*b)),
//                 Value::Array(a) => ("array", Value::Array(a.clone())),
//                 Value::Object(o) => {
//                     let keys: Vec<&String> = o.keys().collect();
//                     if keys.len() == 2
//                         && keys.contains(&&"type".to_string())
//                         && keys.contains(&&"value".to_string())
//                     {
//                         // Already expanded
//                         (o["type"].as_str().unwrap_or("string"), o["value"].clone())
//                     } else {
//                         // Arbitrary object
//                         ("object", Value::Object(o.clone()))
//                     }
//                 }
//                 _ => ("net", Value::Null),
//             };
//
//             // Combine default value with user override
//             let mut value = params.get(param_name).cloned().unwrap_or(param_default);
//
//             // Handle templating for string values
//             if let Value::String(s) = &value {
//                 let templated = utils::template(s, &point.meta);
//
//                 // Convert back to appropriate type
//                 match param_type {
//                     "string" => {
//                         value = Value::String(templated);
//                     }
//                     "number" => match templated.parse::<f64>() {
//                         Ok(n) => value = json!(n),
//                         Err(_) => {
//                             return json!({
//                                 "error": format!(
//                                     "Could not parse '{}' as a number for '{}.params.{}'",
//                                     templated, name, param_name
//                                 )
//                             });
//                         }
//                     },
//                     "boolean" => {
//                         value = json!(templated == "true" || templated == "1");
//                     }
//                     "array" => match serde_json::from_str::<Vec<Value>>(&templated) {
//                         Ok(arr) => value = Value::Array(arr),
//                         Err(_) => {
//                             return json!({
//                                 "error": format!(
//                                     "Could not parse '{}' as an array for '{}.params.{}'",
//                                     templated, name, param_name
//                                 )
//                             });
//                         }
//                     },
//                     "object" => match serde_json::from_str::<Map<String, Value>>(&templated) {
//                         Ok(obj) => value = Value::Object(obj),
//                         Err(_) => {
//                             return json!({
//                                 "error": format!(
//                                     "Could not parse '{}' as an object for '{}.params.{}'",
//                                     templated, name, param_name
//                                 )
//                             });
//                         }
//                     },
//                     "net" => {
//                         // Handle as a string, will process as a net later
//                         value = Value::String(templated);
//                     }
//                     "anchor" => match serde_json::from_str::<Value>(&templated) {
//                         Ok(anchor_val) => value = anchor_val,
//                         Err(_) => {
//                             return json!({
//                                 "error": format!(
//                                     "Could not parse '{}' as an anchor for '{}.params.{}'",
//                                     templated, name, param_name
//                                 )
//                             });
//                         }
//                     },
//                     _ => {
//                         return json!({
//                             "error": format!(
//                                 "Unknown parameter type '{}' for '{}.params.{}'",
//                                 param_type, name, param_name
//                             )
//                         });
//                     }
//                 }
//             }
//
//             // Type-specific processing
//             match param_type {
//                 "string" | "number" | "boolean" | "array" | "object" => {
//                     parsed_params.insert(param_name.clone(), value);
//                 }
//                 "net" => {
//                     let net = match value {
//                         Value::String(s) => s,
//                         _ => {
//                             return json!({
//                                 "error": format!(
//                                     "Expected string for net parameter '{}.params.{}'",
//                                     name, param_name
//                                 )
//                             });
//                         }
//                     };
//
//                     let index = net_indexer(&net);
//                     let net_obj = NetObject::new(net, index);
//                     parsed_params.insert(
//                         param_name.clone(),
//                         json!({
//                             "name": net_obj.name,
//                             "index": net_obj.index,
//                             "str": net_obj.to_string()
//                         }),
//                     );
//                 }
//                 "anchor" => {
//                     let anchor_point = match anchor::parse(
//                         &value,
//                         &format!("{}.params.{}", name, param_name),
//                         points,
//                         Some(point),
//                         false,
//                         units,
//                     ) {
//                         Ok(p) => p,
//                         Err(e) => {
//                             return json!({
//                                 "error": format!(
//                                     "Failed to parse anchor for '{}.params.{}': {}",
//                                     name, param_name, e
//                                 )
//                             });
//                         }
//                     };
//
//                     // Apply Kicad's Y-mirror
//                     let y = -anchor_point.y;
//
//                     parsed_params.insert(
//                         param_name.clone(),
//                         json!({
//                             "x": anchor_point.x,
//                             "y": y,
//                             "r": anchor_point.r
//                         }),
//                     );
//                 }
//                 _ => {
//                     return json!({
//                         "error": format!(
//                             "Unknown parameter type '{}' for '{}.params.{}'",
//                             param_type, name, param_name
//                         )
//                     });
//                 }
//             }
//         }
//
//         // Generate component reference
//         let designator = parsed_params
//             .get("designator")
//             .and_then(|v| v.as_str())
//             .unwrap_or("_");
//
//         let component_ref = component_indexer(designator);
//         parsed_params.insert("ref".to_string(), json!(component_ref));
//         parsed_params.insert(
//             "ref_hide".to_string(),
//             json!(if extra.references { "" } else { "hide" }),
//         );
//
//         // Add footprint positioning info
//         parsed_params.insert(
//             "point".to_string(),
//             json!({
//                 "x": point.x,
//                 "y": point.y,
//                 "r": point.r
//             }),
//         );
//         parsed_params.insert("x".to_string(), json!(point.x));
//         parsed_params.insert("y".to_string(), json!(-point.y)); // Kicad Y-mirror
//         parsed_params.insert("r".to_string(), json!(point.r));
//         parsed_params.insert("rot".to_string(), json!(point.r)); // To be deprecated
//         parsed_params.insert("xy".to_string(), json!(format!("{} {}", point.x, -point.y)));
//         parsed_params.insert(
//             "at".to_string(),
//             json!(format!("(at {} {} {})", point.x, -point.y, point.r)),
//         );
//
//         // Add functions for internal coordinate operations
//         let mirrored = point
//             .meta
//             .get("mirrored")
//             .map_or(false, |v| v.as_bool().unwrap_or(false));
//
//         // Internal coordinate functions would be implemented in a real library
//         // For this prototype, we'll just record the fact they were provided
//         parsed_params.insert("isxy".to_string(), json!("function_provided"));
//         parsed_params.insert("iaxy".to_string(), json!("function_provided"));
//         parsed_params.insert("esxy".to_string(), json!("function_provided"));
//         parsed_params.insert("eaxy".to_string(), json!("function_provided"));
//
//         // Function for local nets
//         parsed_params.insert("local_net".to_string(), json!("function_provided"));
//
//         // Generate the footprint
//         (fp.body)(&parsed_params)
//     })
// }
//
// /// Extra parameters for footprint generation
// struct ExtraParams {
//     references: bool,
// }
//
// /// Parse PCB configuration and generate PCB layouts
// pub fn parse(
//     config: &Value,
//     points: &HashMap<String, Point>,
//     outlines: &HashMap<String, Value>,
//     units: &HashMap<String, f64>,
// ) -> Result<HashMap<String, String>> {
//     // Validate input
//     if !config.is_object() {
//         return Err(Error::TypeError {
//             field: "config".to_string(),
//             expected: "object".to_string(),
//         });
//     }
//
//     // Get PCBs configuration
//     let pcbs = match config.get("pcbs") {
//         Some(Value::Object(p)) => p,
//         _ => {
//             // Default to empty object
//             return Ok(HashMap::new());
//         }
//     };
//
//     let mut results = HashMap::new();
//
//     // Process each PCB
//     for (pcb_name, pcb_config) in pcbs {
//         // Validate PCB config
//         if !pcb_config.is_object() {
//             return Err(Error::TypeError {
//                 field: format!("pcbs.{}", pcb_name),
//                 expected: "object".to_string(),
//             });
//         }
//
//         let pcb_obj = pcb_config.as_object().unwrap();
//
//         // Check for unexpected keys
//         for key in pcb_obj.keys() {
//             if !["outlines", "footprints", "references", "template", "params"]
//                 .contains(&key.as_str())
//             {
//                 return Err(Error::Config(format!(
//                     "Unexpected key '{}' in 'pcbs.{}'",
//                     key, pcb_name
//                 )));
//             }
//         }
//
//         // Get references flag
//         let references = pcb_obj
//             .get("references")
//             .and_then(|v| v.as_bool())
//             .unwrap_or(false);
//
//         // Get template
//         let template_name = pcb_obj
//             .get("template")
//             .and_then(|v| v.as_str())
//             .unwrap_or("kicad5");
//
//         if !TEMPLATE_TYPES.contains_key(template_name) {
//             return Err(Error::Config(format!(
//                 "Unknown PCB template '{}' in 'pcbs.{}'",
//                 template_name, pcb_name
//             )));
//         }
//
//         let template = &TEMPLATE_TYPES[template_name];
//
//         // Convert outlines
//         let mut kicad_outlines = HashMap::new();
//
//         let outlines_config = match pcb_obj.get("outlines") {
//             Some(Value::Object(o)) => o,
//             Some(Value::Array(arr)) => {
//                 // Convert array to object
//                 let mut obj = Map::new();
//                 for (i, outline) in arr.iter().enumerate() {
//                     obj.insert(i.to_string(), outline.clone());
//                 }
//                 &obj
//             }
//             _ => {
//                 // Default to empty object
//                 &Map::new()
//             }
//         };
//
//         for (outline_name, outline_config) in outlines_config {
//             // Validate outline config
//             if !outline_config.is_object() {
//                 return Err(Error::TypeError {
//                     field: format!("pcbs.{}.outlines.{}", pcb_name, outline_name),
//                     expected: "object".to_string(),
//                 });
//             }
//
//             let outline_obj = outline_config.as_object().unwrap();
//
//             // Get outline reference
//             let ref_name = match outline_obj.get("outline") {
//                 Some(Value::String(s)) => s,
//                 _ => {
//                     return Err(Error::MissingField(format!(
//                         "pcbs.{}.outlines.{}.outline",
//                         pcb_name, outline_name
//                     )));
//                 }
//             };
//
//             // Check if outline exists
//             if !outlines.contains_key(ref_name) {
//                 return Err(Error::Config(format!(
//                     "Unknown outline '{}' referenced in 'pcbs.{}.outlines.{}'",
//                     ref_name, pcb_name, outline_name
//                 )));
//             }
//
//             // Get layer
//             let layer = outline_obj
//                 .get("layer")
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("Edge.Cuts");
//
//             // Convert outline to KiCad format
//             kicad_outlines.insert(
//                 outline_name.clone(),
//                 (template.convert_outline)(&outlines[ref_name], layer),
//             );
//         }
//
//         // Make a global net index registry
//         let mut nets = HashMap::new();
//         nets.insert("".to_string(), 0);
//
//         let net_indexer: NetIndexer = Box::new(move |net: &str| {
//             let mut nets_ref = nets.clone();
//             if !nets_ref.contains_key(net) {
//                 let index = nets_ref.len();
//                 nets_ref.insert(net.to_string(), index);
//             }
//             nets_ref[net]
//         });
//
//         // Make a component indexer
//         let mut component_registry = HashMap::new();
//
//         let component_indexer: ComponentIndexer = Box::new(move |class: &str| {
//             let mut registry_ref = component_registry.clone();
//             let count = registry_ref.entry(class.to_string()).or_insert(0);
//             *count += 1;
//             format!("{}{}", class, count)
//         });
//
//         // Generate footprints
//         let mut footprints = Vec::new();
//         let footprint_factory = footprint(
//             points,
//             net_indexer,
//             component_indexer,
//             units,
//             &ExtraParams { references },
//         );
//
//         let footprints_config = match pcb_obj.get("footprints") {
//             Some(Value::Object(f)) => f,
//             Some(Value::Array(arr)) => {
//                 // Convert array to object
//                 let mut obj = Map::new();
//                 for (i, fp) in arr.iter().enumerate() {
//                     obj.insert(i.to_string(), fp.clone());
//                 }
//                 &obj
//             }
//             _ => {
//                 // Default to empty object
//                 &Map::new()
//             }
//         };
//
//         for (f_name, f_config) in footprints_config {
//             // Validate footprint config
//             if !f_config.is_object() {
//                 return Err(Error::TypeError {
//                     field: format!("pcbs.{}.footprints.{}", pcb_name, f_name),
//                     expected: "object".to_string(),
//                 });
//             }
//
//             let f_obj = f_config.as_object().unwrap();
//             let name = format!("pcbs.{}.footprints.{}", pcb_name, f_name);
//
//             // Get asym value
//             let asym = f_obj
//                 .get("asym")
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("source");
//
//             // Get where filter
//             let where_val = f_obj.get("where").cloned();
//
//             // Get adjust value
//             let adjust_val = f_obj.get("adjust").cloned();
//
//             // Create a filtered copy of the footprint config with only relevant keys
//             let mut filtered_config = Map::new();
//             for (k, v) in f_obj {
//                 if !["asym", "where", "adjust"].contains(&k.as_str()) {
//                     filtered_config.insert(k.clone(), v.clone());
//                 }
//             }
//
//             // Filter points based on where clause
//             let where_points = filter::parse(
//                 &where_val.unwrap_or(Value::Null),
//                 &format!("{}.where", name),
//                 points,
//                 units,
//                 asym,
//             )?;
//
//             // Apply adjust function to points
//             for w in where_points {
//                 let adjusted_point = anchor::parse(
//                     &adjust_val.unwrap_or(Value::Object(Map::new())),
//                     &format!("{}.adjust", name),
//                     points,
//                     Some(&w),
//                     false,
//                     units,
//                 )?;
//
//                 // Create footprint and add to list
//                 footprints.push(footprint_factory(
//                     &Value::Object(filtered_config.clone()),
//                     &name,
//                     &adjusted_point,
//                 ));
//             }
//         }
//
//         // Finalize nets
//         let nets_arr: Vec<NetObject> = nets
//             .iter()
//             .map(|(net, index)| NetObject::new(net.clone(), *index))
//             .collect();
//
//         // Generate PCB
//         let template_params = TemplateParams {
//             name: pcb_name,
//             version: config
//                 .get("meta")
//                 .and_then(|m| m.get("version"))
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("v1.0.0"),
//             author: config
//                 .get("meta")
//                 .and_then(|m| m.get("author"))
//                 .and_then(|a| a.as_str())
//                 .unwrap_or("Unknown"),
//             nets: &nets_arr,
//             footprints: &footprints,
//             outlines: &kicad_outlines,
//             custom: pcb_obj.get("params"),
//         };
//
//         results.insert(pcb_name.clone(), (template.body)(&template_params));
//     }
//
//     Ok(results)
// }
