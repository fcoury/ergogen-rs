use serde_yaml::Value;

use crate::{Error, Result};

pub fn preprocess_extends(yaml_str: String) -> Result<String> {
    // Parse the YAML string into a Value
    let yaml_value: Value = serde_yaml::from_str(&yaml_str)?;

    // Make a clone first so we can use it as root
    let root = yaml_value.clone();

    // Process the $extends directives
    let mut yaml_value = yaml_value;
    process_extends_recursively(&mut yaml_value, &root)?;

    // Serialize back to string
    let processed_yaml = serde_yaml::to_string(&yaml_value)?;

    Ok(processed_yaml)
}

fn process_extends_recursively(node: &mut Value, root: &Value) -> Result<()> {
    match node {
        Value::Mapping(mapping) => {
            // First, check if this mapping has an $extends directive
            let mut has_extends = false;
            let mut extend_path = String::new();

            if let Some(Value::String(path)) = mapping.get(Value::String("$extends".to_string())) {
                has_extends = true;
                extend_path = path.clone();
            }

            if has_extends {
                // Parse the path (e.g., "presets.pg1316s")
                let parts: Vec<&str> = extend_path.split('.').collect();

                // Find the referenced object
                let mut target = root;
                for part in &parts {
                    match target {
                        Value::Mapping(m) => {
                            if let Some(next) = m.get(Value::String(part.to_string())) {
                                target = next;
                            } else {
                                return Err(Error::Config(format!(
                                    "Path part '{}' not found",
                                    part
                                )));
                            }
                        }
                        _ => {
                            return Err(Error::Config(format!(
                                "Expected mapping at path part '{}'",
                                part
                            )))
                        }
                    }
                }

                // If target is a mapping, extend the current node with its contents
                if let Value::Mapping(target_mapping) = target {
                    // Create a clone to avoid borrowing issues
                    let target_clone = target_mapping.clone();

                    // Remove the $extends key
                    mapping.remove(Value::String("$extends".to_string()));

                    // Add all key-value pairs from the target
                    for (k, v) in target_clone {
                        if !mapping.contains_key(&k) {
                            mapping.insert(k, v.clone());
                        }
                    }

                    // Here's the critical change - we need to re-process this node after extension
                    // to handle any $extends directives that were brought in
                    return process_extends_recursively(node, root);
                } else {
                    return Err(Error::Config(format!(
                        "Target of $extends is not a mapping: {}",
                        extend_path
                    )));
                }
            }

            // Process recursively for all values in the mapping (if we didn't return above)
            let keys: Vec<Value> = mapping.keys().cloned().collect();
            for key in keys {
                if let Some(value) = mapping.get_mut(&key) {
                    process_extends_recursively(value, root)?;
                }
            }
        }
        Value::Sequence(seq) => {
            // Process each item in the sequence
            for item in seq {
                process_extends_recursively(item, root)?;
            }
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    #[test]
    fn test_preprocess_real() -> Result<()> {
        let yaml_str = include_str!("../../fixtures/zeph.yaml");
        let processed = preprocess_extends(yaml_str.to_string())?;
        println!("{}", processed);

        Ok(())
    }

    #[test]
    fn test_preprocess_extends() -> Result<()> {
        let yaml_str = r#"
meta:
  engine: 4.1.0
  name: zeph
  version: 0.2
  ref: &kb_ref "Zeph v0.1"
  author: ceoloide
  url: &kb_url https://github.com/ceoloide/zeph
  footprint: &switch_footprint "pg1316s_reversible"
  # footprint: &switch_footprint "ceoloide/switch_mx"
  # footprint: &switch_footprint "ceoloide/switch_choc_v1_v2"
  switch:
    $extends: presets.pg1316s

presets:
  # These presets provide different layout options
  # Select a preset in the `units` section below
  # Note: The appropriate switch footprint will still need to be set in the `pcb` section
  pg1316s:
    # Key and keycap measures
    kx: 13.5 # spacing between key centers (X-axis)
    ky: 13 # spacing between key centers (Y-axis)
    ks: 18.5 # horizontal space between columns (default: 19)
    kp: 18.5 # vertical padding between keys (deafult: 19)
    kpx: ks * 0.5 # x padding for the outline
    kpy: kp * 0.5 # y padding for the outline
    kcow: 13.8 # key cutout hole width (cherry, choc v2: 14, choc v1: 13.8)
    kcoh: 13.8 # key cutout hole height (cherry, choc v2: 14, choc v1: 13.8)
    keycw: 17.5 # keycap width (cherry: 18, choc: 17.5)
    keych: 16.5 # keycap height (cherry: 18, choc: 16.5)
    led_pos_x: 0 # Led X position relative to the switch center
    led_pos_y: 4.7 # Led Y position relative to the switch center
    led_rotation: 0 # Led rotation
    vertical_underglow_shift: -kp + 7.8 # How much to shift underglow leds tied to keys
    vertical_diode_shift: 2.5 # How much to shift to avoid overlap
    horizontal_diode_shift: -0.5
    diode_rotation: -90 # Diode rotation
    switch_rotation: 0
"#;

        let processed = preprocess_extends(yaml_str.to_string())?;

        // Parse the processed yaml to verify structure
        let processed_value: Value = serde_yaml::from_str(&processed)?;

        // Check that meta.switch now contains all the keys from presets.pg1316s
        if let Value::Mapping(meta) = &processed_value["meta"] {
            if let Value::Mapping(switch) = &meta["switch"] {
                // Verify a few key properties were copied over
                assert!(switch.contains_key(Value::String("kx".to_string())));
                assert!(switch.contains_key(Value::String("ky".to_string())));
                assert!(switch.contains_key(Value::String("ks".to_string())));
                assert!(switch.contains_key(Value::String("kp".to_string())));

                // Verify the $extends key was removed
                assert!(!switch.contains_key(Value::String("$extends".to_string())));

                // Check a specific value
                if let Value::Number(kx) = &switch["kx"] {
                    assert_eq!(kx.as_f64().unwrap(), 13.5);
                } else {
                    panic!("kx is not a number");
                }
            } else {
                panic!("switch is not a mapping");
            }
        } else {
            panic!("meta is not a mapping");
        }

        Ok(())
    }

    #[test]
    fn test_extends_with_existing_values() -> Result<()> {
        let yaml_str = r#"
object:
  $extends: template
  existing_key: "This should be preserved"

template:
  key1: "value1"
  key2: "value2"
  existing_key: "This should not override"
"#;

        let processed = preprocess_extends(yaml_str.to_string())?;
        let processed_value: Value = serde_yaml::from_str(&processed)?;

        if let Value::Mapping(object) = &processed_value["object"] {
            // Check that existing_key was preserved
            if let Value::String(existing) = &object["existing_key"] {
                assert_eq!(existing, "This should be preserved");
            } else {
                panic!("existing_key is not a string");
            }

            // Check that new keys were added
            if let Value::String(key1) = &object["key1"] {
                assert_eq!(key1, "value1");
            } else {
                panic!("key1 is not a string");
            }

            // Check that $extends was removed
            assert!(!object.contains_key(Value::String("$extends".to_string())));
        } else {
            panic!("object is not a mapping");
        }

        Ok(())
    }

    #[test]
    fn test_nested_extends() -> Result<()> {
        let yaml_str = r#"
main:
  nested:
    $extends: templates.level1

templates:
  level1:
    key1: "value1"
    deep:
      $extends: templates.level2
  level2:
    key2: "value2"
"#;

        let processed = preprocess_extends(yaml_str.to_string())?;
        let processed_value: Value = serde_yaml::from_str(&processed)?;

        // Check first level extension
        if let Value::Mapping(main) = &processed_value["main"] {
            if let Value::Mapping(nested) = &main["nested"] {
                assert!(nested.contains_key(Value::String("key1".to_string())));

                // Check nested extension
                if let Value::Mapping(deep) = &nested["deep"] {
                    assert!(deep.contains_key(Value::String("key2".to_string())));
                } else {
                    panic!("deep is not a mapping");
                }
            } else {
                panic!("nested is not a mapping");
            }
        } else {
            panic!("main is not a mapping");
        }

        Ok(())
    }
}
