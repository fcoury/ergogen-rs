use serde_json::{Map, Value};

use crate::{utils, Result};

/// Deeply extend a base object with properties from another object
pub fn extend(to: &Value, from: &Value) -> Result<Value> {
    if from.is_null() {
        return Ok(to.clone());
    }

    if from.as_str().map_or(false, |s| s == "$unset") {
        return Ok(Value::Null);
    }

    // If types don't match, use the 'from' value
    if to.is_object() != from.is_object() || to.is_array() != from.is_array() {
        return Ok(from.clone());
    }

    // Handle objects
    if to.is_object() && from.is_object() {
        let mut result = to.clone();
        let result_obj = result.as_object_mut().unwrap();

        for (key, value) in from.as_object().unwrap() {
            let extended = if result_obj.contains_key(key) {
                extend(&result_obj[key], value)?
            } else {
                value.clone()
            };

            if extended.is_null() {
                result_obj.remove(key);
            } else {
                result_obj.insert(key.clone(), extended);
            }
        }

        return Ok(result);
    }

    // Handle arrays
    if to.is_array() && from.is_array() {
        let mut result = to.clone();
        let result_arr = result.as_array_mut().unwrap();

        for (i, value) in from.as_array().unwrap().iter().enumerate() {
            if i < result_arr.len() {
                result_arr[i] = extend(&result_arr[i], value)?;
            } else {
                result_arr.push(value.clone());
            }
        }

        return Ok(result);
    }

    // Default case: just use the 'from' value
    Ok(from.clone())
}

/// Extend an object with multiple other objects
pub fn extend_multiple(args: &[Value]) -> Result<Value> {
    if args.is_empty() {
        return Ok(Value::Null);
    }

    let mut res = args[0].clone();

    for arg in args.iter().skip(1) {
        if arg == &res {
            continue;
        }

        res = extend(&res, arg)?;
    }

    Ok(res)
}

/// Operation to be applied during traversal
type TraverseOp = fn(&mut Map<String, Value>, &str, Value, &Value, &Vec<String>) -> ();

/// Traverse a configuration object and apply an operation to each node
pub fn traverse(
    config: &Value,
    root: &Value,
    breadcrumbs: &mut Vec<String>,
    op: TraverseOp,
) -> Value {
    match config {
        Value::Object(obj) => {
            let mut result = Map::new();

            for (key, val) in obj {
                breadcrumbs.push(key.clone());
                let traversed = traverse(val, root, breadcrumbs, op);
                op(&mut result, key, traversed, root, breadcrumbs);
                breadcrumbs.pop();
            }

            Value::Object(result)
        }
        Value::Array(arr) => {
            let mut dummy = Map::new();
            let mut result = Vec::new();

            for (index, val) in arr.iter().enumerate() {
                breadcrumbs.push(format!("[{}]", index));
                let traversed = traverse(val, root, breadcrumbs, op);
                op(&mut dummy, "dummykey", traversed, root, breadcrumbs);

                if let Some(val) = dummy.get("dummykey") {
                    result.push(val.clone());
                }

                breadcrumbs.pop();
            }

            Value::Array(result)
        }
        _ => config.clone(),
    }
}

/// Unnest a configuration object by flattening nested properties
pub fn unnest(config: &Value) -> Value {
    fn unnest_op(
        target: &mut Map<String, Value>,
        key: &str,
        val: Value,
        _root: &Value,
        _breadcrumbs: &Vec<String>,
    ) {
        target.insert(key.to_string(), val);
    }

    traverse(config, config, &mut Vec::new(), unnest_op)
}

/// Process inheritance in a configuration by resolving $extends references
pub fn inherit(config: &Value) -> Result<Value> {
    fn inherit_op(
        target: &mut Map<String, Value>,
        key: &str,
        mut val: Value,
        root: &Value,
        breadcrumbs: &Vec<String>,
    ) {
        if val.is_object() && val.get("$extends").is_some() {
            let mut candidates = match val["$extends"].clone() {
                Value::Array(arr) => arr,
                other => vec![other],
            };

            let mut list = vec![val.clone()];
            let mut index = 0;

            while index < candidates.len() {
                let path_value = candidates[index].clone();
                index += 1;

                if let Some(path) = path_value.as_str() {
                    if let Some(other) = utils::deep(&mut root.clone(), path, None) {
                        let mut parents = Vec::new();
                        if let Some(extends) = other.get("$extends") {
                            match extends {
                                Value::Array(arr) => parents = arr.clone(),
                                other => parents.push(other.clone()),
                            }
                        }

                        candidates.extend(parents);

                        // Detect circular dependencies
                        if list.contains(&other) {
                            panic!(
                                "\"{}\" (reached from \"{}\") leads to a circular dependency!",
                                path,
                                breadcrumbs.join(".")
                            );
                        }

                        list.insert(0, other.clone());
                    } else {
                        panic!("\"{}\" (reached from \"{}\") does not name a valid inheritance target!",
                                   path, breadcrumbs.join("."));
                    }
                }
            }

            // Combine all objects in the inheritance chain
            val = list
                .iter()
                .try_fold(Value::Null, |acc, item| extend(&acc, item))
                .expect("Error extending objects in inheritance chain");

            // Remove the $extends field
            if let Some(obj) = val.as_object_mut() {
                obj.remove("$extends");
            }
        }

        target.insert(key.to_string(), val);
    }

    Ok(traverse(config, config, &mut Vec::new(), inherit_op))
}

// pub(crate) fn parameterize(config: _) -> _ {
//     todo!()
// }
