use indexmap::IndexMap;

use crate::error::Error;
use crate::value::Value;

pub fn unnest(config: &Value) -> Result<Value, Error> {
    match config {
        Value::Map(m) => {
            let out = IndexMap::new();
            let mut out_v = Value::Map(out);
            for (k, v) in m {
                let nested = unnest(v)?;
                out_v.set_path(k, nested)?;
            }
            Ok(out_v)
        }
        Value::Seq(seq) => Ok(Value::Seq(
            seq.iter().map(unnest).collect::<Result<_, _>>()?,
        )),
        other => Ok(other.clone()),
    }
}

/// Ergogen-style deep merge.
///
/// - `from == "$unset"` removes keys from objects.
/// - Type mismatches replace `to` with `from`.
/// - Arrays merge by index.
#[must_use]
pub fn extend_all(values: &[Value]) -> Value {
    let mut res = values.first().cloned().unwrap_or(Value::Null);
    for v in values.iter().skip(1) {
        res = extend_one(res, v.clone()).unwrap_or(Value::Null);
    }
    res
}

fn extend_one(to: Value, from: Value) -> Option<Value> {
    match &from {
        Value::Null => return Some(to),
        Value::String(s) if s == "$unset" => return None,
        _ => {}
    }

    match (to, from) {
        (Value::Map(to_map), Value::Map(from_map)) => {
            let mut res = to_map;
            for (k, v) in from_map {
                let next = extend_one(res.get(&k).cloned().unwrap_or(Value::Null), v);
                if let Some(val) = next {
                    res.insert(k, val);
                } else {
                    res.shift_remove(&k);
                }
            }
            Some(Value::Map(res))
        }
        (Value::Seq(mut to_seq), Value::Seq(from_seq)) => {
            for (i, v) in from_seq.into_iter().enumerate() {
                let existing = to_seq.get(i).cloned().unwrap_or(Value::Null);
                let merged = extend_one(existing, v);
                if i >= to_seq.len() {
                    to_seq.resize(i + 1, Value::Null);
                }
                to_seq[i] = merged.unwrap_or(Value::Null);
            }
            Some(Value::Seq(to_seq))
        }
        (_, from_other) => Some(from_other),
    }
}

pub fn inherit(config: &Value) -> Result<Value, Error> {
    inherit_with_root(config, config, &mut Vec::new())
}

fn inherit_with_root(
    config: &Value,
    root: &Value,
    breadcrumbs: &mut Vec<String>,
) -> Result<Value, Error> {
    match config {
        Value::Map(m) => {
            let mut out = IndexMap::new();
            for (k, v) in m {
                breadcrumbs.push(k.clone());
                let mut next = inherit_with_root(v, root, breadcrumbs)?;
                if let Value::Map(ref mut obj) = next
                    && let Some(extends) = obj.get("$extends").cloned()
                {
                    let from_path = breadcrumbs.join(".");
                    next = apply_extends(&from_path, obj.clone(), extends, root)?;
                }
                out.insert(k.clone(), next);
                breadcrumbs.pop();
            }
            Ok(Value::Map(out))
        }
        Value::Seq(seq) => Ok(Value::Seq(
            seq.iter()
                .enumerate()
                .map(|(i, v)| {
                    breadcrumbs.push(format!("[{i}]"));
                    let res = inherit_with_root(v, root, breadcrumbs);
                    breadcrumbs.pop();
                    res
                })
                .collect::<Result<_, _>>()?,
        )),
        other => Ok(other.clone()),
    }
}

fn apply_extends(
    from_path: &str,
    val_obj: IndexMap<String, Value>,
    extends: Value,
    root: &Value,
) -> Result<Value, Error> {
    let mut candidates = match extends {
        Value::Seq(a) => a,
        other => vec![other],
    };

    let mut chain: Vec<(String, IndexMap<String, Value>)> = Vec::new();
    chain.push((from_path.to_string(), val_obj.clone()));
    let mut seen_paths: Vec<String> = vec![from_path.to_string()];

    while let Some(path_v) = candidates.first().cloned() {
        candidates.remove(0);
        let Value::String(path) = path_v else {
            return Err(Error::Parameterize {
                at: format!("{from_path}.$extends"),
                message: "$extends must be a string or array of strings".to_string(),
            });
        };

        let other = root
            .get_path(&path)
            .ok_or_else(|| Error::ExtendsTargetMissing {
                from: format!("{from_path}.$extends"),
                path: path.clone(),
            })?;
        let Value::Map(other_map) = other.clone() else {
            return Err(Error::ExtendsTargetMissing {
                from: format!("{from_path}.$extends"),
                path: path.clone(),
            });
        };

        if seen_paths.contains(&path) {
            let mut cycle = seen_paths.clone();
            cycle.push(path.clone());
            return Err(Error::ExtendsCycle {
                cycle: cycle.join(" -> "),
            });
        }
        seen_paths.push(path.clone());

        if let Some(parents) = other_map.get("$extends") {
            match parents {
                Value::Seq(arr) => candidates.extend(arr.clone()),
                v => candidates.push(v.clone()),
            }
        }

        chain.push((path, other_map));
    }

    // Merge so the earliest ancestor is first, and `val` is last.
    let mut merged_list: Vec<Value> = chain.into_iter().map(|(_, m)| Value::Map(m)).collect();
    // chain is [self, parent1, parent2, ...], but we want [root-most parent, ..., self]
    merged_list.reverse();
    let mut merged = extend_all(&merged_list);
    if let Value::Map(ref mut m) = merged {
        m.shift_remove("$extends");
    }
    Ok(merged)
}

pub fn parameterize(config: &Value) -> Result<Value, Error> {
    parameterize_with_root(config, &mut Vec::new())
}

fn parameterize_with_root(config: &Value, breadcrumbs: &mut Vec<String>) -> Result<Value, Error> {
    match config {
        Value::Map(m) => {
            let mut out = IndexMap::new();
            for (k, v) in m {
                breadcrumbs.push(k.clone());
                let mut next = parameterize_with_root(v, breadcrumbs)?;
                if let Value::Map(obj) = &next
                    && obj.contains_key("$skip")
                    && matches!(obj.get("$skip"), Some(Value::Bool(true)))
                {
                    breadcrumbs.pop();
                    continue;
                }
                if let Value::Map(obj) = next.clone() {
                    let params = obj.get("$params").cloned();
                    let args = obj.get("$args").cloned();
                    let at = breadcrumbs.join(".");

                    match (params, args) {
                        (None, None) => {}
                        (Some(_), None) => {
                            breadcrumbs.pop();
                            continue;
                        }
                        (None, Some(_)) => {
                            return Err(Error::Parameterize {
                                at,
                                message: "found $args but missing $params".to_string(),
                            });
                        }
                        (Some(params), Some(args)) => {
                            next = apply_parameterize(&at, Value::Map(obj), params, args)?;
                        }
                    }
                }

                out.insert(k.clone(), next);
                breadcrumbs.pop();
            }
            Ok(Value::Map(out))
        }
        Value::Seq(seq) => Ok(Value::Seq(
            seq.iter()
                .enumerate()
                .map(|(i, v)| {
                    breadcrumbs.push(format!("[{i}]"));
                    let res = parameterize_with_root(v, breadcrumbs);
                    breadcrumbs.pop();
                    res
                })
                .collect::<Result<_, _>>()?,
        )),
        other => Ok(other.clone()),
    }
}

fn apply_parameterize(at: &str, val: Value, params: Value, args: Value) -> Result<Value, Error> {
    let Value::Seq(params) = params else {
        return Err(Error::Parameterize {
            at: format!("{at}.$params"),
            message: "$params must be an array of strings".to_string(),
        });
    };
    let Value::Seq(args) = args else {
        return Err(Error::Parameterize {
            at: format!("{at}.$args"),
            message: "$args must be an array".to_string(),
        });
    };

    if params.len() != args.len() {
        return Err(Error::Parameterize {
            at: at.to_string(),
            message: "the number of $params and $args doesn't match".to_string(),
        });
    }

    let mut params: Vec<String> = params
        .into_iter()
        .map(|v| match v {
            Value::String(s) => Ok(s),
            _ => Err(Error::Parameterize {
                at: format!("{at}.$params"),
                message: "$params must contain strings".to_string(),
            }),
        })
        .collect::<Result<_, _>>()?;

    let mut arg_strs: Vec<String> = Vec::new();
    for v in args {
        arg_strs.push(arg_to_replacement_string(&v));
    }

    // Mirror the JS behavior: JSON.stringify(val) then global regex replacements then JSON.parse.
    let mut json = val.to_json_compact_string();
    for (par, arg) in params.drain(..).zip(arg_strs.drain(..)) {
        let re = regex::Regex::new(&par).map_err(|e| Error::Parameterize {
            at: format!("{at}.$params"),
            message: format!("invalid regex \"{par}\": {e}"),
        })?;
        json = re.replace_all(&json, arg.as_str()).to_string();
    }

    let mut reparsed = Value::try_from_json_str(&json).map_err(|e| Error::Parameterize {
        at: at.to_string(),
        message: format!("replacements didn't lead to valid JSON: {e}"),
    })?;

    if let Value::Map(ref mut m) = reparsed {
        m.shift_remove("$params");
        m.shift_remove("$args");
        m.shift_remove("$skip");
    }
    Ok(reparsed)
}

fn arg_to_replacement_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Number(n) => {
            if n.is_finite()
                && n.fract() == 0.0
                && *n <= (i64::MAX as f64)
                && *n >= (i64::MIN as f64)
            {
                format!("{}", *n as i64)
            } else {
                // Fall back to a compact float representation.
                let mut buf = ryu::Buffer::new();
                buf.format(*n).to_string()
            }
        }
        Value::String(s) => s.clone(),
        // Upstream JS would use `[object Object]` for objects; keep this explicit so
        // parameterization failures are easier to understand.
        Value::Seq(_) | Value::Map(_) => v.to_json_compact_string(),
    }
}

#[derive(Debug, Clone)]
pub struct PreparedIr {
    pub canonical: Value,
}

impl PreparedIr {
    pub fn from_yaml_str(yaml: &str) -> Result<Self, Error> {
        let raw = Value::from_yaml_str(yaml)?;
        let canonical = parameterize(&inherit(&unnest(&raw)?)?)?;
        Ok(Self { canonical })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unnest_converts_dotted_keys() {
        let raw = Value::Map(IndexMap::from([
            ("a.b".to_string(), Value::Number(1.0)),
            ("a.c".to_string(), Value::Number(2.0)),
        ]));
        let got = unnest(&raw).unwrap();
        let want = Value::Map(IndexMap::from([(
            "a".to_string(),
            Value::Map(IndexMap::from([
                ("b".to_string(), Value::Number(1.0)),
                ("c".to_string(), Value::Number(2.0)),
            ])),
        )]));
        assert_eq!(got, want);
    }

    #[test]
    fn inherit_merges_and_unsets() {
        let raw = Value::Map(IndexMap::from([
            (
                "templates".to_string(),
                Value::Map(IndexMap::from([(
                    "base".to_string(),
                    Value::Map(IndexMap::from([
                        ("x".to_string(), Value::Number(1.0)),
                        ("y".to_string(), Value::Number(2.0)),
                    ])),
                )])),
            ),
            (
                "thing".to_string(),
                Value::Map(IndexMap::from([
                    (
                        "$extends".to_string(),
                        Value::String("templates.base".to_string()),
                    ),
                    ("y".to_string(), Value::String("$unset".to_string())),
                    ("z".to_string(), Value::Number(3.0)),
                ])),
            ),
        ]));

        let got = inherit(&raw).unwrap();
        let thing = got.get_path("thing").unwrap();
        let Value::Map(m) = thing else {
            panic!("thing should be map")
        };
        assert_eq!(m.get("x"), Some(&Value::Number(1.0)));
        assert!(!m.contains_key("y"));
        assert_eq!(m.get("z"), Some(&Value::Number(3.0)));
        assert!(!m.contains_key("$extends"));
    }

    #[test]
    fn parameterize_replaces_and_removes_params_and_args() {
        let raw = Value::Map(IndexMap::from([(
            "t".to_string(),
            Value::Map(IndexMap::from([
                (
                    "$params".to_string(),
                    Value::Seq(vec![Value::String("AAA".to_string())]),
                ),
                ("$args".to_string(), Value::Seq(vec![Value::Number(42.0)])),
                ("name".to_string(), Value::String("AAA".to_string())),
                ("x".to_string(), Value::String("AAA + 1".to_string())),
            ])),
        )]));

        let got = parameterize(&raw).unwrap();
        let t = got.get_path("t").unwrap();
        let Value::Map(m) = t else {
            panic!("t should be map")
        };
        assert_eq!(m.get("name"), Some(&Value::String("42".to_string())));
        assert_eq!(m.get("x"), Some(&Value::String("42 + 1".to_string())));
        assert!(!m.contains_key("$params"));
        assert!(!m.contains_key("$args"));
    }
}
