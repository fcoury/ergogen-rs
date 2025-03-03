use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{expr::evaluate_expression, Error, Result};

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

    // Calculate final units
    let mut units = HashMap::<String, f64>::new();

    // Iterate fixed values
    let (fixed, calculated): (Vec<_>, Vec<_>) = raw_units.iter().partition(|(_, v)| v.is_number());

    for (key, val) in fixed {
        if let Some(f) = val.as_f64() {
            units.insert(key.clone(), f);
        }
    }

    let mut last_failed_keys = Vec::new();
    loop {
        let mut failed_keys = Vec::new();

        for (key, val) in calculated.iter() {
            if failed_keys.contains(key) {
                continue;
            }

            match evaluate_mathnum(val, &units) {
                Ok(f) => {
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
        } else if last_failed_keys == failed_keys {
            return Err(Error::ValueError(format!(
                "Failed to evaluate units: {:?}",
                failed_keys
            )));
        }

        last_failed_keys = failed_keys.clone();
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
        Value::String(s) => Ok(evaluate_expression(s, units.clone())?),
        _ => Err(Error::TypeError {
            field: "mathnum".to_string(),
            expected: "number or string".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_default_config() {
        // Test with an empty config
        let config = json!({});
        let result = parse(&config);
        assert!(result.is_ok());

        let units = result.unwrap();
        // Check that default values are present and correct
        assert_eq!(units.get("U").unwrap(), &19.05);
        assert_eq!(units.get("u").unwrap(), &19.0);
        assert_eq!(units.get("cx").unwrap(), &18.0);
        assert_eq!(units.get("cy").unwrap(), &17.0);

        // Check that expressions were evaluated
        assert_eq!(units.get("$default_height").unwrap(), &18.0); // u-1 = 19-1 = 18
    }

    #[test]
    fn test_parse_with_custom_units() {
        // Test with custom units in config
        let config = json!({
            "units": {
                "U": 20.0,
                "custom_unit": 25.5,
                "derived_unit": "custom_unit+U"
            }
        });

        let result = parse(&config);
        assert!(result.is_ok());

        let units = result.unwrap();
        // Check that default values were overridden
        assert_eq!(units.get("U").unwrap(), &20.0);
        // Check that new units were added
        assert_eq!(units.get("custom_unit").unwrap(), &25.5);
        // Check that expressions were evaluated
        assert_eq!(units.get("derived_unit").unwrap(), &45.5); // custom_unit+U = 25.5+20.0 = 45.5
    }

    #[test]
    fn test_parse_with_custom_notation() {
        // Test with custom notation in config
        let config = json!({
            "units": {
                "U": 20.0,
                "derived_unit": "0.5px",
                "px": "2U",
            }
        });

        let result = parse(&config);
        assert!(result.is_ok());

        let units = result.unwrap();
        assert_eq!(units.get("U").unwrap(), &20.0);
        assert_eq!(units.get("px").unwrap(), &40.0); // 2U = 2*20 = 40
                                                     // Should be 2 * 20 * 0.5 = 20
        assert_eq!(units.get("derived_unit").unwrap(), &20.0);
    }

    #[test]
    fn test_parse_with_variables() {
        // Test with variables in config
        let config = json!({
            "variables": {
                "var1": 10.0,
                "var2": "var1*2",
                "var3": "var2+u"
            }
        });

        let result = parse(&config);
        assert!(result.is_ok());

        let units = result.unwrap();
        // Check that variables were added
        assert_eq!(units.get("var1").unwrap(), &10.0);
        // Check that expressions were evaluated
        assert_eq!(units.get("var2").unwrap(), &20.0); // var1*2 = 10*2 = 20
        assert_eq!(units.get("var3").unwrap(), &39.0); // var2+u = 20+19 = 39
    }

    #[test]
    fn test_parse_with_circular_references() {
        // Test with circular references that should fail
        let config = json!({
            "variables": {
                "circular1": "circular2+1",
                "circular2": "circular1+1"
            }
        });

        let result = parse(&config);
        assert!(result.is_err());
        match result {
            Err(Error::ValueError(_)) => {
                // Expected error type
            }
            _ => panic!("Expected ValueError for circular references"),
        }
    }

    #[test]
    fn test_parse_with_complex_dependency_chain() {
        // Test with a complex but valid dependency chain
        let config = json!({
            "variables": {
                "base": 5.0,
                "level1": "base*2",
                "level2": "level1+10",
                "level3": "level2*2-level1"
            }
        });

        let result = parse(&config);
        assert!(result.is_ok());

        let units = result.unwrap();
        // Check that the dependency chain was resolved correctly
        assert_eq!(units.get("base").unwrap(), &5.0);
        assert_eq!(units.get("level1").unwrap(), &10.0); // base*2 = 5*2 = 10
        assert_eq!(units.get("level2").unwrap(), &20.0); // level1+10 = 10+10 = 20
        assert_eq!(units.get("level3").unwrap(), &30.0); // level2*2-level1 = 20*2-10 = 30
    }

    #[test]
    fn test_parse_with_invalid_expression() {
        // Test with an invalid expression
        let config = json!({
            "variables": {
                "invalid": "this is not a valid expression"
            }
        });

        let result = parse(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_with_non_object_units() {
        // Test with non-object units
        let config = json!({
            "units": "not an object"
        });

        let result = parse(&config);
        assert!(result.is_ok()); // Should ignore invalid units and use defaults
    }

    #[test]
    fn test_parse_with_non_object_variables() {
        // Test with non-object variables
        let config = json!({
            "variables": "not an object"
        });

        let result = parse(&config);
        assert!(result.is_ok()); // Should ignore invalid variables and use defaults
    }
}
