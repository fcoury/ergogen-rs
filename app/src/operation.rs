use std::collections::HashMap;

/// Parse an operation prefix from a string
///
/// Operations can be prefixed with:
/// - '+' for addition (default)
/// - '-' for subtraction
/// - '~' for intersection
/// - '^' for stacking
pub fn op_prefix(s: &str) -> (String, String) {
    if s.is_empty() {
        return (String::from("add"), String::new());
    }

    let first_char = s.chars().next().unwrap();
    let name = s[1..].to_string();

    let operation = match first_char {
        '+' => "add",
        '-' => "subtract",
        '~' => "intersect",
        '^' => "stack",
        _ => "add", // No prefix means the whole string is the name
    };

    if operation == "add" && first_char != '+' {
        // If no recognized prefix, the entire string is the name
        return (operation.to_string(), s.to_string());
    }

    (operation.to_string(), name)
}

/// Parse an operation with its target from a string
pub fn operation(
    s: &str,
    choices: &HashMap<String, Vec<String>>,
    order: Option<Vec<String>>,
) -> Result<OperationResult, String> {
    let (operation, name) = op_prefix(s);

    let order = order.unwrap_or_else(|| choices.keys().cloned().collect());

    for key in order {
        if let Some(values) = choices.get(&key) {
            if values.contains(&name) {
                return Ok(OperationResult {
                    name,
                    operation,
                    what: Some(key),
                });
            }
        }
    }

    Ok(OperationResult {
        name: name.clone(),
        operation,
        what: None,
    })
}

/// Result of parsing an operation
pub struct OperationResult {
    pub name: String,
    pub operation: String,
    pub what: Option<String>,
}
