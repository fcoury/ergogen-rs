use regex::Regex;
use serde_json::{Map, Value};

pub fn process_templates(key_dict: &Map<String, Value>) -> Map<String, Value> {
    let mut result = key_dict.clone();
    let re = Regex::new(r"\{\{([^{}]+)\}\}").unwrap();
    let max_iterations = 10; // Prevent infinite loops with circular references
    let mut changes_made = true;

    // Keep processing until no more changes are made or max iterations reached
    for _ in 0..max_iterations {
        if !changes_made {
            break;
        }
        changes_made = false;

        // Create a copy of the current state to read from while modifying result
        let current_state = result.clone();

        result = process_pass(&current_state, &re, &mut changes_made);
    }

    result
}

fn process_pass(
    current: &Map<String, Value>,
    re: &Regex,
    changes_made: &mut bool,
) -> Map<String, Value> {
    let mut result = Map::new();

    for (key, value) in current.iter() {
        result.insert(
            key.clone(),
            process_value(value, "", current, re, changes_made),
        );
    }

    result
}

fn process_value(
    value: &Value,
    path: &str,
    dict: &Map<String, Value>,
    re: &Regex,
    changes_made: &mut bool,
) -> Value {
    match value {
        Value::String(s) => {
            // Process string for templates
            let mut processed = s.clone();
            let original = processed.clone();

            for cap in re.captures_iter(s) {
                let template_path = cap[1].trim();

                // Try to find the value at the path
                if let Some(replacement) = find_value_at_path(dict, template_path) {
                    // Convert the replacement to string
                    let replacement_str = match replacement {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "null".to_string(),
                        _ => format!("{:?}", replacement),
                    };

                    // Replace the template with the found value
                    processed =
                        processed.replace(&format!("{{{{{}}}}}", template_path), &replacement_str);
                }
            }

            // Check if any changes were made
            if processed != original {
                *changes_made = true;
            }

            Value::String(processed)
        }
        Value::Array(arr) => {
            // Process each element of the array
            let processed_arr = arr
                .iter()
                .enumerate()
                .map(|(i, v)| process_value(v, &format!("{}[{}]", path, i), dict, re, changes_made))
                .collect();

            Value::Array(processed_arr)
        }
        Value::Object(obj) => {
            // Process each field of the object
            let mut processed_obj = Map::new();
            for (k, v) in obj {
                let new_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", path, k)
                };
                processed_obj.insert(
                    k.clone(),
                    process_value(v, &new_path, dict, re, changes_made),
                );
            }

            Value::Object(processed_obj)
        }
        // Other value types don't need processing
        _ => value.clone(),
    }
}

fn find_value_at_path<'a>(dict: &'a Map<String, Value>, path: &str) -> Option<&'a Value> {
    // Check if path contains array notation
    if path.contains('[') {
        // Handle array access like items[0]
        let re = Regex::new(r"^([^\[]+)\[(\d+)\]$").unwrap();
        if let Some(caps) = re.captures(path) {
            let array_name = caps.get(1).unwrap().as_str();
            let index: usize = caps.get(2).unwrap().as_str().parse().unwrap();

            // Get the array
            if let Some(Value::Array(arr)) = dict.get(array_name) {
                return arr.get(index);
            } else {
                return None;
            }
        }
    }

    // Handle regular dot notation (person.name)
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = dict.get(parts[0])?;

    for &part in parts.iter().skip(1) {
        // Check if this part contains array indexing
        if part.contains('[') && part.ends_with(']') {
            let array_part: Vec<&str> = part.split('[').collect();
            let obj_key = array_part[0];
            let idx_str = array_part[1].trim_end_matches(']');

            // Navigate to the object first
            match current {
                Value::Object(obj) => {
                    current = obj.get(obj_key)?;
                }
                _ => return None,
            }

            // Then access the array element
            if let Ok(idx) = idx_str.parse::<usize>() {
                match current {
                    Value::Array(arr) => {
                        current = arr.get(idx)?;
                    }
                    _ => return None,
                }
            } else {
                return None;
            }
        } else {
            // Regular object property access
            match current {
                Value::Object(obj) => {
                    current = obj.get(part)?;
                }
                _ => return None,
            }
        }
    }

    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_json_diff::assert_json_include;
    use serde_json::{json, Map, Value};

    // Helper function to convert json! macro output to Map<String, Value>
    fn json_to_map(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(map) => map,
            _ => panic!("Expected a JSON object"),
        }
    }

    #[test]
    fn test_simple_templates() {
        let input = json_to_map(json!({
            "name": "John",
            "greeting": "Hello {{name}}!"
        }));

        let expected = json_to_map(json!({
            "name": "John",
            "greeting": "Hello John!"
        }));

        let result = process_templates(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_nested_path_templates() {
        let input = json_to_map(json!({
            "person": {
                "name": "Alice",
                "age": 30
            },
            "message": "{{person.name}} is {{person.age}} years old."
        }));

        let expected = json_to_map(json!({
            "person": {
                "name": "Alice",
                "age": 30
            },
            "message": "Alice is 30 years old."
        }));

        let result = process_templates(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_array_templates() {
        let input = json_to_map(json!({
            "items": ["apple", "banana", "cherry"],
            "first_item": "My favorite fruit is {{items[0]}}."
        }));

        let expected = json_to_map(json!({
            "items": ["apple", "banana", "cherry"],
            "first_item": "My favorite fruit is apple."
        }));

        let result = process_templates(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_nested_templates() {
        let input = json_to_map(json!({
            "user": {
                "profile": {
                    "firstName": "Bob",
                    "lastName": "Smith"
                }
            },
            "greeting": "Welcome, {{user.profile.firstName}} {{user.profile.lastName}}!"
        }));

        let expected = json_to_map(json!({
            "user": {
                "profile": {
                    "firstName": "Bob",
                    "lastName": "Smith"
                }
            },
            "greeting": "Welcome, Bob Smith!"
        }));

        let result = process_templates(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_multiple_templates_in_string() {
        let input = json_to_map(json!({
            "first": "Hello",
            "last": "World",
            "message": "{{first}}, {{last}}!"
        }));

        let expected = json_to_map(json!({
            "first": "Hello",
            "last": "World",
            "message": "Hello, World!"
        }));

        let result = process_templates(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_non_existent_path() {
        let input = json_to_map(json!({
            "message": "Hello {{missing.path}}!"
        }));

        // Template not replaced because path doesn't exist
        let expected = json_to_map(json!({
            "message": "Hello {{missing.path}}!"
        }));

        let result = process_templates(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_deep_nested_structure() {
        let input = json_to_map(json!({
            "company": {
                "name": "Acme Inc",
                "departments": {
                    "engineering": {
                        "employees": [
                            {"name": "Dave", "role": "Developer"}
                        ]
                    }
                }
            },
            "template": "{{company.name}} - {{company.departments.engineering.employees[0].role}}"
        }));

        let expected = json_to_map(json!({
            "company": {
                "name": "Acme Inc",
                "departments": {
                    "engineering": {
                        "employees": [
                            {"name": "Dave", "role": "Developer"}
                        ]
                    }
                }
            },
            "template": "Acme Inc - Developer"
        }));

        let result = process_templates(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_different_value_types() {
        let input = json_to_map(json!({
            "number": 42,
            "boolean": true,
            "null_value": null,
            "template": "Number: {{number}}, Boolean: {{boolean}}, Null: {{null_value}}"
        }));

        let expected = json_to_map(json!({
            "number": 42,
            "boolean": true,
            "null_value": null,
            "template": "Number: 42, Boolean: true, Null: null"
        }));

        let result = process_templates(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_partial_template() {
        let input = json_to_map(json!({
            "name": "Alice {{info.color}}",
            "info": {
                "color": "Gray",
                "age": 19,
            },
            "message": "Hello {{name}}_{{info.age}}_{{info.other}}! How are you?"
        }));

        let expected = json_to_map(json!({
            "name": "Alice Gray",
            "info": {
                "color": "Gray",
                "age": 19,
            },
            "message": "Hello Alice Gray_19_{{info.other}}! How are you?"
        }));

        let result = process_templates(&input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_real_world_example() {
        let input = json_to_map(json!({
            "col":  {
                "name": "right",
            },
            "col_name": "right",
            "colrow": "{{col.name}}_{{row}}",
            "name": "{{zone.name}}_{{colrow}}",
            "row": "top",
        }));

        let expected = json_to_map(json!({
            "col":  {
                "name": "right",
            },
            "col_name": "right",
            "colrow": "right_top",
            "name": "{{zone.name}}_right_top",
            "row": "top",
        }));

        let result = process_templates(&input);
        println!("{:#?}", result);

        assert_json_include!(actual: result, expected: expected);
    }
}
