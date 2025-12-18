use std::collections::HashMap;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Units {
    variables: HashMap<String, f64>,
}

impl Default for Units {
    fn default() -> Self {
        let mut variables = HashMap::new();
        variables.insert("u".to_string(), 19.05);
        variables.insert("U".to_string(), 19.0);
        variables.insert("cx".to_string(), 18.0);
        variables.insert("cy".to_string(), 17.0);
        Self { variables }
    }
}

impl Units {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, name: &str, value: f64) {
        self.variables.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<f64> {
        self.variables.get(name).cloned()
    }

    pub fn parse(&self, input: &Value) -> Result<f64> {
        match input {
            Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0)),
            Value::String(s) => {
                // First try to parse as a direct number
                if let Ok(val) = s.parse::<f64>() {
                    return Ok(val);
                }

                // If not a number, try to evaluate as an expression using meval
                let mut context = meval::Context::new();
                for (name, value) in &self.variables {
                    context.var(name, *value);
                }

                meval::eval_str_with_context(s, &context)
                    .map_err(|e| anyhow!("Failed to evaluate expression '{}': {}", s, e))
            }
            _ => Err(anyhow!("Cannot parse non-numeric/string value as unit: {:?}", input)),
        }
    }
}

/// Flattens a nested JSON/YAML object using dot notation for keys.
pub fn unnest(value: &Value) -> Value {
    let mut flat = serde_json::Map::new();
    unnest_recursive(value, "", &mut flat);
    Value::Object(flat)
}

fn unnest_recursive(value: &Value, prefix: &str, flat: &mut serde_json::Map<String, Value>) {
    if let Value::Object(obj) = value {
        for (key, val) in obj {
            let new_prefix = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", prefix, key)
            };
            unnest_recursive(val, &new_prefix, flat);
        }
    } else {
        flat.insert(prefix.to_string(), value.clone());
    }
}

/// Deep merges two JSON values.
pub fn merge(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                merge(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_unnest() {
        let input = json!({
            "a": {
                "b": 1,
                "c": {
                    "d": 2
                }
            },
            "e": 3
        });
        let expected = json!({
            "a.b": 1,
            "a.c.d": 2,
            "e": 3
        });
        assert_eq!(unnest(&input), expected);
    }

    #[test]
    fn test_merge() {
        let mut a = json!({
            "a": 1,
            "b": {
                "c": 2
            }
        });
        let b = json!({
            "b": {
                "d": 3
            },
            "e": 4
        });
        let expected = json!({
            "a": 1,
            "b": {
                "c": 2,
                "d": 3
            },
            "e": 4
        });
        merge(&mut a, &b);
        assert_eq!(a, expected);
    }
}