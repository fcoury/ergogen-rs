use serde_yaml::{Mapping, Value};

use crate::{Error, Result};

pub fn preprocess(yaml: serde_yaml::Value) -> Result<serde_yaml::Value> {
    match yaml {
        Value::Mapping(map) => {
            let mut new_map = Mapping::new();

            for (key, value) in map {
                if let Value::String(key_str) = &key {
                    if key_str.contains('.') {
                        // This is a dotted key that needs expansion
                        let parts: Vec<&str> = key_str.split('.').collect();
                        let processed_value = preprocess(value)?;

                        // Insert the value at the leaf of the nested structure
                        insert_nested(&mut new_map, &parts, processed_value)?;
                    } else {
                        // Regular key, just process its value recursively
                        let processed_value = preprocess(value)?;
                        new_map.insert(key, processed_value);
                    }
                } else {
                    // Non-string key, just process its value recursively
                    let processed_value = preprocess(value)?;
                    new_map.insert(key, processed_value);
                }
            }

            Ok(Value::Mapping(new_map))
        }
        Value::Sequence(seq) => {
            let mut new_seq = Vec::new();

            for item in seq {
                let processed_item = preprocess(item)?;
                new_seq.push(processed_item);
            }

            Ok(Value::Sequence(new_seq))
        }
        // TODO: proper handle of $unset
        // Value::String(s) => {
        //     if s == "$unset" {
        //         Ok(Value::Null)
        //     } else {
        //         Ok(Value::String(s))
        //     }
        // }
        // For other scalar values, just return as is
        _ => Ok(yaml),
    }
}

// Helper function to insert a value into a nested map structure
fn insert_nested(map: &mut Mapping, path: &[&str], value: Value) -> Result<()> {
    if path.is_empty() {
        return Ok(());
    }

    if path.len() == 1 {
        // We've reached the final part of the path
        map.insert(Value::String(path[0].to_string()), value);
        return Ok(());
    }

    let current_key = Value::String(path[0].to_string());

    // Get or create the nested map for this part of the path
    let nested_map = match map.get_mut(&current_key) {
        Some(existing) => {
            match existing {
                Value::Mapping(m) => m,
                _ => {
                    // Convert to a mapping if it's not already one
                    let mut new_map = Mapping::new();
                    *existing = Value::Mapping(new_map);
                    match existing {
                        Value::Mapping(m) => m,
                        _ => {
                            return Err(Error::ValueError(format!(
                                "Failed to convert value to mapping for key '{}'",
                                path[0]
                            )))
                        }
                    }
                }
            }
        }
        None => {
            // Create a new mapping and insert it
            let new_map = Mapping::new();
            map.insert(current_key.clone(), Value::Mapping(new_map));
            match map.get_mut(&current_key) {
                Some(Value::Mapping(m)) => m,
                _ => {
                    return Err(Error::ValueError(format!(
                        "Failed to create nested mapping for key '{}'",
                        path[0]
                    )))
                }
            }
        }
    };

    // Continue with the rest of the path
    insert_nested(nested_map, &path[1..], value)
}

// Example usage:
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess() {
        let yaml_str = r#"
        points.zones:
          none:
            key.autobind: 0
            columns:
              a:
              b:
        "#;

        let value: Value = serde_yaml::from_str(yaml_str).unwrap();
        let processed = preprocess(value).unwrap();

        let expected_str = r#"
        points:
          zones:
            none:
              key:
                autobind: 0
              columns:
                a:
                b:
        "#;

        let expected: Value = serde_yaml::from_str(expected_str).unwrap();
        assert_eq!(processed, expected);
    }
}
