mod aggregator;
mod anchor;
mod config;
mod error;
mod expr;
mod outline;
mod point;
mod points;
mod preprocess;
mod server;
mod template;
mod types;
mod units;
mod yaml;
mod zone;

use serde_json::Value;
use std::collections::HashMap;

pub use error::{Error, Result};

pub async fn process(_raw: &str, _debug: bool) -> Result<HashMap<String, Value>> {
    todo!()
}

// /// Process the input configuration and generate keyboard layouts
// #[async_recursion::async_recursion]
// pub async fn process(raw: &str, debug: bool) -> Result<HashMap<String, Value>> {
//     // Interpret input format
//     let (mut config, format) = io::interpret(raw)?;
//     tracing::info!("Interpreting format: {:?}", format);
//
//     // Apply preprocessing
//     tracing::info!("Preprocessing input...");
//     config = prepare::unnest(&config);
//     config = prepare::inherit(&config)?;
//     // TODO: Implement parameterization
//     // config = prepare::parameterize(&config)?;
//
//     // Initialize results object
//     let mut results = HashMap::new();
//
//     // Add debug info if requested
//     if debug {
//         results.insert("raw".to_string(), Value::String(raw.to_string()));
//         results.insert("canonical".to_string(), config.clone());
//     }
//
//     // Check engine compatibility
//     if let Some(meta) = config.get("meta") {
//         if let Some(engine_version) = meta.get("engine").and_then(|v| v.as_str()) {
//             tracing::info!("Checking compatibility...");
//
//             // Get current version
//             let version = env!("CARGO_PKG_VERSION");
//             let current = utils::semver(version, "current")?;
//             let required = utils::semver(engine_version, "config.meta.engine")?;
//
//             if !utils::satisfies(&current, &required) {
//                 return Err(Error::Version(format!(
//                     "Current ergogen version ({}) doesn't satisfy config's engine requirement ({})!",
//                     version, engine_version
//                 )));
//             }
//         }
//     }
//
//     // Calculate variables
//     tracing::info!("Calculating variables...");
//     let units = units::parse(&config)?;
//
//     if debug {
//         results.insert("units".to_string(), serde_json::to_value(&units).unwrap());
//     }
//
//     // Parse points
//     tracing::info!("Parsing points...");
//     let points_config = config
//         .get("points")
//         .ok_or_else(|| Error::MissingField("points".to_string()))?;
//
//     let points = points::parse(points_config, &units)?;
//     println!("{:#?}", points);
//
//     if points.is_empty() {
//         return Err(Error::Config(
//             "Input does not contain any points!".to_string(),
//         ));
//     }
//
//     if debug {
//         results.insert("points".to_string(), serde_json::to_value(&points).unwrap());
//         results.insert("demo".to_string(), points::visualize(&points, &units));
//     }
//
//     // Generate outlines
//     // tracing::info!("Generating outlines...");
//     // let empty = serde_json::json!({});
//     // let outlines_config = config.get("outlines").unwrap_or(&empty);
//     // let outlines = outlines::parse(outlines_config, &points, &units)?;
//     //
//     // let mut output_outlines = HashMap::new();
//     // for (name, outline) in &outlines {
//     //     if !debug && name.starts_with('_') {
//     //         continue;
//     //     }
//     //
//     //     output_outlines.insert(name.clone(), outline.clone());
//     // }
//     //
//     // results.insert(
//     //     "outlines".to_string(),
//     //     serde_json::to_value(&output_outlines).unwrap(),
//     // );
//
//     // Model cases
//     // tracing::info!("Modeling cases...");
//     // let cases_config = config.get("cases").unwrap_or(&empty);
//     // let cases = cases::parse(cases_config, &outlines, &units)?;
//     //
//     // let mut output_cases = HashMap::new();
//     // for (name, case_script) in &cases {
//     //     if !debug && name.starts_with('_') {
//     //         continue;
//     //     }
//     //
//     //     output_cases.insert(
//     //         name.clone(),
//     //         serde_json::json!({
//     //             "jscad": case_script
//     //         }),
//     //     );
//     // }
//     //
//     // results.insert(
//     //     "cases".to_string(),
//     //     serde_json::to_value(&output_cases).unwrap(),
//     // );
//
//     // Scaffold PCBs
//     // tracing::info!("Scaffolding PCBs...");
//     // let pcbs = pcbs::parse(&config, &points, &outlines, &units)?;
//     //
//     // let mut output_pcbs = HashMap::new();
//     // for (name, pcb_text) in &pcbs {
//     //     if !debug && name.starts_with('_') {
//     //         continue;
//     //     }
//     //
//     //     output_pcbs.insert(name.clone(), Value::String(pcb_text.clone()));
//     // }
//     //
//     // results.insert(
//     //     "pcbs".to_string(),
//     //     serde_json::to_value(&output_pcbs).unwrap(),
//     // );
//
//     // If there's no output and debug mode is off, rerun with debug
//     // if !debug && output_outlines.is_empty() && output_cases.is_empty() && output_pcbs.is_empty() {
//     //     tracing::info!("Output would be empty, rerunning in debug mode...");
//     //     return process(raw, true).await;
//     // }
//
//     Ok(results)
// }

pub async fn start_webserver(listen_addr: Option<String>, port: Option<u16>) -> anyhow::Result<()> {
    server::start(listen_addr, port).await
}
