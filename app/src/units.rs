use serde_json::{json, Map, Value};
use std::collections::HashMap;

use crate::{prepare, Error, Result};

// Default units that are available in all configs
const DEFAULT_UNITS: [(&str, f64); 11] = [
    ("U", 19.05),
    ("u", 19.0),
    ("cx", 18.0),
    ("cy", 17.0),
    ("$default_stagger", 0.0),
    ("$default_spread", 19.0), // u
    ("$default_splay", 0.0),
    ("$default_height", 18.0),  // u-1
    ("$default_width", 18.0),   // u-1
    ("$default_padding", 19.0), // u
    ("$default_autobind", 10.0),
];

/// Parse and calculate units from the config
pub fn parse(config: &Value) -> Result<HashMap<String, f64>> {
    // Create a default units map
    let mut raw_units = HashMap::<String, Value>::from([
        ("U".to_string(), json!(19.05)),
        ("u".to_string(), json!(19.0)),
        ("cx".to_string(), json!(18.0)),
        ("cy".to_string(), json!(17.0)),
        ("$default_stagger".to_string(), json!(0.0)),
        ("$default_spread".to_string(), json!("u")),
        ("$default_splay".to_string(), json!(0.0)),
        ("$default_height".to_string(), json!("u-1")),
        ("$default_width".to_string(), json!("u-1")),
        ("$default_padding".to_string(), json!("u")),
        ("$default_autobind".to_string(), json!(10.0)),
    ]);

    // Extend with units from config
    if let Some(config_units) = config.get("units").and_then(|u| u.as_object()) {
        for (key, val) in config_units {
            raw_units.insert(key.clone(), val.clone());
        }
    }

    // Extend with variables from config
    if let Some(config_vars) = config.get("variables").and_then(|v| v.as_object()) {
        for (key, val) in config_vars {
            raw_units.insert(key.clone(), val.clone());
        }
    }

    tracing::info!("Units: {:#?}", raw_units);

    // Calculate final units
    let mut units = HashMap::<String, f64>::new();

    // Iterate fixed values
    let (fixed, mut calculated): (Vec<_>, Vec<_>) =
        raw_units.iter().partition(|(_, v)| v.is_number());

    for (key, val) in fixed {
        tracing::info!("Key: {}, Val: {:?}", key, val);
        if let Some(f) = val.as_f64() {
            units.insert(key.clone(), f);
        }
    }

    loop {
        let mut failed_keys = Vec::new();

        for (key, val) in calculated.iter() {
            if failed_keys.contains(key) {
                continue;
            }

            match evaluate_mathnum(val, &units) {
                Ok(f) => {
                    tracing::info!("Key: {}, Val: {:?}", key, val);
                    units.insert(key.to_string(), f);
                }
                Err(e) => {
                    tracing::error!("Failed to evaluate unit '{}': {}", key, e);
                    failed_keys.push(key);
                }
            }
        }

        if failed_keys.is_empty() {
            break;
        } else if failed_keys.len() == calculated.len() {
            return Err(Error::ValueError(format!(
                "Failed to evaluate units: {:?}",
                failed_keys
            )));
        }
    }

    Ok(units)
}

/// Evaluate a mathematical expression or number using the units map
pub fn evaluate_mathnum(val: &Value, units: &HashMap<String, f64>) -> Result<f64> {
    match val {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(f)
            } else {
                Err(Error::ValueError(format!(
                    "Could not convert number to f64: {}",
                    n
                )))
            }
        }
        Value::String(s) => {
            // Parse a mathematical expression like "u-1" or "2*cx"
            let mut expr = s.clone();

            // Replace unit variables with their values
            for (unit_name, unit_value) in units {
                expr = expr.replace(unit_name, &unit_value.to_string());
            }

            // Use a math expression evaluator
            match meval::eval_str(&expr) {
                Ok(result) => Ok(result),
                Err(e) => Err(Error::ValueError(format!(
                    "Failed to evaluate expression '{}': {}",
                    s, e
                ))),
            }
        }
        _ => Err(Error::TypeError {
            field: "mathnum".to_string(),
            expected: "number or string".to_string(),
        }),
    }
}
