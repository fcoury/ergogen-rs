// use serde_json::{json, Map, Value};
// use std::collections::{HashMap, HashSet};
//
// use crate::{operation, Error, Result};
//
// /// Information about a case
// struct CaseInfo {
//     body: String,
//     case_dependencies: Vec<String>,
//     outline_dependencies: Vec<String>,
// }
//
// /// Parse the cases configuration and generate JSCAD scripts
// pub fn parse(
//     config: &Value,
//     outlines: &HashMap<String, Value>,
//     units: &HashMap<String, f64>,
// ) -> Result<HashMap<String, String>> {
//     // Validate input
//     if !config.is_object() {
//         return Err(Error::TypeError {
//             field: "cases".to_string(),
//             expected: "object".to_string(),
//         });
//     }
//
//     // Prepare output containers
//     let mut scripts = HashMap::new();
//     let mut cases = HashMap::new();
//     let mut results = HashMap::new();
//
//     // Resolve dependencies for a case
//     fn resolve(
//         case_name: &str,
//         cases: &HashMap<String, CaseInfo>,
//         scripts: &HashMap<String, String>,
//         resolved_scripts: &mut HashSet<String>,
//         resolved_cases: &mut HashSet<String>,
//     ) -> String {
//         // Get outline dependencies
//         for o in &cases[case_name].outline_dependencies {
//             resolved_scripts.insert(o.clone());
//         }
//
//         // Get case dependencies
//         for c in &cases[case_name].case_dependencies {
//             resolved_cases.insert(c.clone());
//             resolve(c, cases, scripts, resolved_scripts, resolved_cases);
//         }
//
//         // Build the result script
//         let mut result = String::new();
//
//         // Add dependent scripts
//         for o in resolved_scripts {
//             result.push_str(&scripts[o]);
//             result.push_str("\n\n");
//         }
//
//         // Add dependent cases
//         for c in resolved_cases {
//             result.push_str(&cases[c].body);
//         }
//
//         // Add this case
//         result.push_str(&cases[case_name].body);
//
//         // Add main function
//         result.push_str(&format!(
//             r#"
//
//             function main() {{
//                 return {}_case_fn();
//             }}
//
//         "#,
//             case_name
//         ));
//
//         result
//     }
//
//     // Process each case
//     for (case_name, case_config) in config.as_object().unwrap() {
//         // Convert array to object if needed
//         let parts_obj = match case_config {
//             Value::Array(arr) => {
//                 let mut obj = Map::new();
//                 for (i, part) in arr.iter().enumerate() {
//                     obj.insert(i.to_string(), part.clone());
//                 }
//                 Value::Object(obj)
//             }
//             Value::Object(_) => case_config.clone(),
//             _ => {
//                 return Err(Error::TypeError {
//                     field: format!("cases.{}", case_name),
//                     expected: "object or array".to_string(),
//                 });
//             }
//         };
//
//         let mut body = Vec::new();
//         let mut case_dependencies = Vec::new();
//         let mut outline_dependencies = Vec::new();
//         let mut first = true;
//
//         // Process each part
//         for (part_name, mut part) in parts_obj.as_object().unwrap().clone() {
//             // String shortcuts are expanded first
//             if let Value::String(s) = &part {
//                 let op_result = operation::operation(
//                     s,
//                     &maplit::hashmap! {
//                         "outline".to_string() => outlines.keys().cloned().collect(),
//                         "case".to_string() => cases.keys().cloned().collect(),
//                     },
//                     Some(vec!["case".to_string(), "outline".to_string()]),
//                 )
//                 .map_err(|e| Error::Config(e))?;
//
//                 part = json!({
//                     "what": op_result.what.unwrap_or_else(|| "outline".to_string()),
//                     "name": op_result.name,
//                     "operation": op_result.operation
//                 });
//             }
//
//             // Validate part
//             if !part.is_object() {
//                 return Err(Error::TypeError {
//                     field: format!("cases.{}.{}", case_name, part_name),
//                     expected: "object or string".to_string(),
//                 });
//             }
//
//             let part_obj = part.as_object().unwrap();
//             let part_qname = format!("cases.{}.{}", case_name, part_name);
//             let part_var = format!("{}__part_{}", case_name, part_name);
//
//             // Check for unexpected keys
//             for key in part_obj.keys() {
//                 if !["what", "name", "extrude", "shift", "rotate", "operation"]
//                     .contains(&key.as_str())
//                 {
//                     return Err(Error::Config(format!(
//                         "Unexpected key \"{}\" in \"{}\"",
//                         key, part_qname
//                     )));
//                 }
//             }
//
//             // Get part parameters
//             let what = part_obj
//                 .get("what")
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("outline");
//             if !["outline", "case"].contains(&what) {
//                 return Err(Error::Config(format!(
//                     "Field \"{}.what\" must be one of ['outline', 'case']",
//                     part_qname
//                 )));
//             }
//
//             let name = part_obj
//                 .get("name")
//                 .and_then(|v| v.as_str())
//                 .ok_or_else(|| Error::MissingField(format!("{}.name", part_qname)))?;
//
//             // Handle shift
//             let shift = match part_obj.get("shift") {
//                 Some(Value::Array(arr)) if arr.len() >= 3 => {
//                     let mut vals = [0.0, 0.0, 0.0];
//                     for (i, val) in arr.iter().enumerate().take(3) {
//                         vals[i] = match val {
//                             Value::Number(n) => n.as_f64().unwrap_or(0.0),
//                             Value::String(s) => match s.parse::<f64>() {
//                                 Ok(v) => v,
//                                 Err(_) => {
//                                     return Err(Error::Config(format!(
//                                         "Could not parse '{}' as a number in {}.shift[{}]",
//                                         s, part_qname, i
//                                     )));
//                                 }
//                             },
//                             _ => 0.0,
//                         };
//                     }
//                     vals
//                 }
//                 _ => [0.0, 0.0, 0.0],
//             };
//
//             // Handle rotate
//             let rotate = match part_obj.get("rotate") {
//                 Some(Value::Array(arr)) if arr.len() >= 3 => {
//                     let mut vals = [0.0, 0.0, 0.0];
//                     for (i, val) in arr.iter().enumerate().take(3) {
//                         vals[i] = match val {
//                             Value::Number(n) => n.as_f64().unwrap_or(0.0),
//                             Value::String(s) => match s.parse::<f64>() {
//                                 Ok(v) => v,
//                                 Err(_) => {
//                                     return Err(Error::Config(format!(
//                                         "Could not parse '{}' as a number in {}.rotate[{}]",
//                                         s, part_qname, i
//                                     )));
//                                 }
//                             },
//                             _ => 0.0,
//                         };
//                     }
//                     vals
//                 }
//                 _ => [0.0, 0.0, 0.0],
//             };
//
//             // Handle operation
//             let operation = part_obj
//                 .get("operation")
//                 .and_then(|v| v.as_str())
//                 .unwrap_or("add");
//             if !["add", "subtract", "intersect"].contains(&operation) {
//                 return Err(Error::Config(format!(
//                     "Field \"{}.operation\" must be one of ['add', 'subtract', 'intersect']",
//                     part_qname
//                 )));
//             }
//
//             // Generate the appropriate base code depending on part type
//             let base = if what == "outline" {
//                 // Check if outline exists
//                 if !outlines.contains_key(name) {
//                     return Err(Error::Config(format!(
//                         "Field \"{}.name\" does not name a valid outline!",
//                         part_qname
//                     )));
//                 }
//
//                 // Get extrude parameter
//                 let extrude = match part_obj.get("extrude") {
//                     Some(Value::Number(n)) => n.as_f64().unwrap_or(1.0),
//                     Some(Value::String(s)) => match s.parse::<f64>() {
//                         Ok(v) => v,
//                         Err(_) => {
//                             return Err(Error::Config(format!(
//                                 "Could not parse '{}' as a number in {}.extrude",
//                                 s, part_qname
//                             )));
//                         }
//                     },
//                     None => 1.0,
//                     _ => {
//                         return Err(Error::TypeError {
//                             field: format!("{}.extrude", part_qname),
//                             expected: "number or string".to_string(),
//                         });
//                     }
//                 };
//
//                 // Create a unique name for the extruded outline
//                 let extruded_name =
//                     format!("{}_extrude_{}", name, extrude.to_string().replace(".", "_"));
//
//                 // Add to outline dependencies if not already there
//                 if !scripts.contains_key(&extruded_name) {
//                     // Generate the actual JSCAD script from outline data
//                     // Match the JavaScript version that uses m.exporter.toJscadScript
//                     let outline = &outlines[name];
//                     let script = generate_jscad_script(outline, &extruded_name, extrude);
//                     scripts.insert(extruded_name.clone(), script);
//                 }
//
//                 outline_dependencies.push(extruded_name.clone());
//                 format!("{}_outline_fn()", extruded_name)
//             } else {
//                 // It's a case reference
//                 if part_obj.get("extrude").is_some() {
//                     return Err(Error::Config(format!(
//                         "Field \"{}.extrude\" should not be used when what=case!",
//                         part_qname
//                     )));
//                 }
//
//                 // Check if the case exists
//                 if !cases.contains_key(name) {
//                     return Err(Error::Config(format!(
//                         "Field \"{}.name\" does not name a valid case!",
//                         part_qname
//                     )));
//                 }
//
//                 case_dependencies.push(name.to_string());
//                 format!("{}_case_fn()", name)
//             };
//
//             // Map operation type to JSCAD function
//             let op = match operation {
//                 "subtract" => "subtract",
//                 "intersect" => "intersect",
//                 _ => "union",
//             };
//
//             // Generate the operation statement
//             let op_statement = if first {
//                 format!("let result = {};", part_var)
//             } else {
//                 format!("result = result.{}({});", op, part_var)
//             };
//
//             // No longer the first part
//             first = false;
//
//             // Add the part code to the body
//             body.push(format!(
//                 r#"
//
//                 // creating part {} of case {}
//                 let {} = {};
//
//                 // make sure that rotations are relative
//                 let {}_bounds = {}.getBounds();
//                 let {}_x = {}_bounds[0].x + ({}_bounds[1].x - {}_bounds[0].x) / 2
//                 let {}_y = {}_bounds[0].y + ({}_bounds[1].y - {}_bounds[0].y) / 2
//                 {} = translate([-{}_x, -{}_y, 0], {});
//                 {} = rotate({}, {});
//                 {} = translate([{}_x, {}_y, 0], {});
//
//                 {} = translate({}, {});
//                 {};
//
//             "#,
//                 part_name,
//                 case_name,
//                 part_var,
//                 base,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 json!(rotate).to_string(),
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 part_var,
//                 json!(shift).to_string(),
//                 part_var,
//                 op_statement
//             ));
//         }
//
//         // Create the case function
//         let case_body = format!(
//             r#"
//
//                 function {}_case_fn() {{
//                     {}
//                     return result;
//                 }}
//
//             "#,
//             case_name,
//             body.join("")
//         );
//
//         // Store the case
//         cases.insert(
//             case_name.to_string(),
//             CaseInfo {
//                 body: case_body,
//                 case_dependencies,
//                 outline_dependencies,
//             },
//         );
//
//         // Resolve the complete case script
//         let mut resolved_scripts = HashSet::new();
//         let mut resolved_cases = HashSet::new();
//         results.insert(
//             case_name.to_string(),
//             resolve(
//                 case_name,
//                 &cases,
//                 &scripts,
//                 &mut resolved_scripts,
//                 &mut resolved_cases,
//             ),
//         );
//     }
//
//     Ok(results)
// }
//
// // Helper function to generate JSCAD script from outline data
// // This replicates the m.exporter.toJscadScript function in JavaScript
// fn generate_jscad_script(outline: &Value, function_name: &str, extrude: f64) -> String {
//     // In a real implementation, this would generate actual JSCAD code
//     // based on the outline data, similar to m.exporter.toJscadScript
//     format!(
//         r#"function {}_outline_fn() {{
//     // Actual implementation would use the outline data to generate JSCAD
//     // This is where you'd use the equivalent of m.exporter.toJscadScript
//
//     // Create the outline path
//     const path = new CSG.Path2D([[0, 0], [10, 0], [10, 10], [0, 10]], true);
//
//     // Extrude to the specified height
//     const shape = path.rectangularExtrude({}, 1, 1, 16);
//
//     return shape;
// }}"#,
//         function_name, extrude
//     )
// }
