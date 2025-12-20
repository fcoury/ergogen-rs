use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Seq(Vec<Value>),
    Map(IndexMap<String, Value>),
}

impl Value {
    #[must_use]
    pub fn as_map(&self) -> Option<&IndexMap<String, Value>> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_map_mut(&mut self) -> Option<&mut IndexMap<String, Value>> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }

    #[must_use]
    pub fn get_path(&self, path: &str) -> Option<&Value> {
        let mut current = self;
        for seg in path.split('.').filter(|s| !s.is_empty()) {
            let Value::Map(m) = current else {
                return None;
            };
            current = m.get(seg)?;
        }
        Some(current)
    }

    pub fn set_path(&mut self, path: &str, value: Value) -> Result<(), Error> {
        let segments: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
        if segments.is_empty() {
            *self = value;
            return Ok(());
        }

        let mut current = self;
        for seg in &segments[..segments.len() - 1] {
            match current {
                Value::Map(m) => {
                    if !m.contains_key(*seg) {
                        m.insert((*seg).to_string(), Value::Map(IndexMap::new()));
                    }
                    current = m.get_mut(*seg).ok_or_else(|| Error::InvalidPath {
                        path: path.to_string(),
                        segment: (*seg).to_string(),
                    })?;
                }
                _ => {
                    return Err(Error::InvalidPath {
                        path: path.to_string(),
                        segment: (*seg).to_string(),
                    });
                }
            }
        }

        let last = *segments.last().unwrap();
        let Value::Map(m) = current else {
            return Err(Error::InvalidPath {
                path: path.to_string(),
                segment: last.to_string(),
            });
        };
        m.insert(last.to_string(), value);
        Ok(())
    }

    #[must_use]
    pub fn to_json_compact_string(&self) -> String {
        // The JSON produced here is used for snapshot tests and is expected to be stable and
        // order-preserving (IndexMap serialization preserves insertion order).
        serde_json::to_string(self).expect("Value must be JSON-serializable")
    }

    pub fn from_yaml_str(yaml: &str) -> Result<Self, Error> {
        let normalized =
            normalize_yaml_flow_sequence_holes(&normalize_yaml_flow_sequence_expressions(yaml));
        let v: serde_yaml::Value = serde_yaml::from_str(&normalized)?;
        Self::try_from_yaml_value(&v)
    }

    fn try_from_yaml_value(v: &serde_yaml::Value) -> Result<Self, Error> {
        Ok(match v {
            serde_yaml::Value::Null => Value::Null,
            serde_yaml::Value::Bool(b) => Value::Bool(*b),
            serde_yaml::Value::Number(n) => Value::Number(
                n.as_f64()
                    .or_else(|| n.as_i64().map(|i| i as f64))
                    .or_else(|| n.as_u64().map(|u| u as f64))
                    .ok_or(Error::YamlNumber)?,
            ),
            serde_yaml::Value::String(s) => Value::String(s.clone()),
            serde_yaml::Value::Sequence(seq) => Value::Seq(
                seq.iter()
                    .map(Self::try_from_yaml_value)
                    .collect::<Result<_, _>>()?,
            ),
            serde_yaml::Value::Mapping(map) => {
                let mut out = IndexMap::new();
                for (k, vv) in map {
                    let serde_yaml::Value::String(key) = k else {
                        return Err(Error::NonStringKey);
                    };
                    out.insert(key.clone(), Self::try_from_yaml_value(vv)?);
                }
                Value::Map(out)
            }
            // serde_yaml supports tags; treat unknown as error for now to keep IR deterministic.
            _ => return Err(Error::UnsupportedYamlValue),
        })
    }

    pub fn try_from_json_str(s: &str) -> Result<Self, Error> {
        let v: serde_json::Value =
            serde_json::from_str(s).map_err(|e| Error::Json(e.to_string()))?;
        Ok(Self::from_json_value(&v))
    }

    fn from_json_value(v: &serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Array(a) => {
                Value::Seq(a.iter().map(Self::from_json_value).collect())
            }
            serde_json::Value::Object(o) => {
                let mut m = IndexMap::new();
                for (k, v) in o {
                    m.insert(k.clone(), Self::from_json_value(v));
                }
                Value::Map(m)
            }
        }
    }
}

fn normalize_yaml_flow_sequence_holes(input: &str) -> String {
    // Upstream Ergogen fixtures use flow sequences with "holes", e.g. `[,10,,]`, which some YAML
    // parsers accept as nulls but `serde_yaml` rejects. Normalize these cases by inserting `null`
    // for empty items inside `[...]` when they are separated by commas.
    //
    // This is a conservative state machine that only operates inside flow sequences and only when
    // we're not inside single or double quotes.
    #[derive(Debug, Clone, Copy)]
    struct Frame {
        expect_value: bool,
        saw_any_element: bool,
        last_sep_was_hole: bool,
    }

    let mut out = String::with_capacity(input.len());
    let mut frames: Vec<Frame> = Vec::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut prev_was_escape = false;

    let chars = input.chars();
    for ch in chars {
        if frames.is_empty() {
            // Not inside a flow sequence.
            if ch == '[' {
                frames.push(Frame {
                    expect_value: true,
                    saw_any_element: false,
                    last_sep_was_hole: false,
                });
            }
            out.push(ch);
            continue;
        }

        // Inside at least one `[...]`.
        if in_double && !prev_was_escape {
            prev_was_escape = ch == '\\';
        } else {
            prev_was_escape = false;
        }

        if !in_double && ch == '\'' {
            in_single = !in_single;
            if in_single
                && let Some(cur) = frames.last_mut()
                && cur.expect_value
            {
                cur.expect_value = false;
                cur.saw_any_element = true;
            }
            out.push(ch);
            continue;
        }
        if !in_single && ch == '"' && !prev_was_escape {
            in_double = !in_double;
            if in_double
                && let Some(cur) = frames.last_mut()
                && cur.expect_value
            {
                cur.expect_value = false;
                cur.saw_any_element = true;
            }
            out.push(ch);
            continue;
        }

        if in_single || in_double {
            out.push(ch);
            continue;
        }

        // Helper: access current frame.
        let mut cur = *frames.last().expect("non-empty");

        match ch {
            '[' => {
                // Starting a nested flow sequence counts as providing a value for the parent.
                if cur.expect_value {
                    cur.expect_value = false;
                    cur.saw_any_element = true;
                    *frames.last_mut().unwrap() = cur;
                }
                frames.push(Frame {
                    expect_value: true,
                    saw_any_element: false,
                    last_sep_was_hole: false,
                });
                out.push(ch);
            }
            ',' => {
                let was_hole = cur.expect_value;
                if was_hole {
                    out.push_str("null");
                    cur.saw_any_element = true;
                }
                out.push(ch);
                cur.expect_value = true;
                cur.last_sep_was_hole = was_hole;
                *frames.last_mut().unwrap() = cur;
            }
            ']' => {
                if cur.expect_value && cur.saw_any_element {
                    if cur.last_sep_was_hole {
                        out.push_str("null");
                    } else {
                        // Upstream configs sometimes include trailing commas in flow sequences, e.g.
                        // `[1, 2,]` or with whitespace/newlines after the comma. `serde_yaml`
                        // rejects these, so we conservatively remove the trailing comma.
                        let trimmed_len = out.trim_end_matches(char::is_whitespace).len();
                        if trimmed_len > 0 && out.as_bytes()[trimmed_len - 1] == b',' {
                            out.remove(trimmed_len - 1);
                        }
                    }
                }
                out.push(ch);
                frames.pop();
                if let Some(parent) = frames.last_mut() {
                    parent.expect_value = false;
                    parent.saw_any_element = true;
                }
            }
            c if c.is_whitespace() => {
                out.push(c);
            }
            other => {
                if cur.expect_value {
                    cur.expect_value = false;
                    cur.saw_any_element = true;
                    *frames.last_mut().unwrap() = cur;
                }
                out.push(other);
            }
        }
    }

    out
}

fn normalize_yaml_flow_sequence_expressions(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_single = false;
    let mut in_double = false;
    let mut prev_was_escape = false;
    let mut idx = 0;

    while idx < input.len() {
        let ch = input[idx..].chars().next().expect("valid char");
        let ch_len = ch.len_utf8();

        if in_double && !prev_was_escape {
            prev_was_escape = ch == '\\';
        } else {
            prev_was_escape = false;
        }

        if !in_double && ch == '\'' {
            in_single = !in_single;
            out.push(ch);
            idx += ch_len;
            continue;
        }
        if !in_single && ch == '"' && !prev_was_escape {
            in_double = !in_double;
            out.push(ch);
            idx += ch_len;
            continue;
        }

        if !in_single && !in_double && ch == '[' {
            let (seq, consumed) = extract_flow_sequence(&input[idx..]);
            out.push_str(&normalize_flow_sequence(&seq));
            idx += consumed;
            continue;
        }

        out.push(ch);
        idx += ch_len;
    }

    out
}

fn extract_flow_sequence(input: &str) -> (String, usize) {
    let mut depth = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut prev_was_escape = false;

    for (offset, ch) in input.char_indices() {
        if in_double && !prev_was_escape {
            prev_was_escape = ch == '\\';
        } else {
            prev_was_escape = false;
        }

        if !in_double && ch == '\'' {
            in_single = !in_single;
        } else if !in_single && ch == '"' && !prev_was_escape {
            in_double = !in_double;
        }

        if in_single || in_double {
            continue;
        }

        match ch {
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = offset + ch.len_utf8();
                    return (input[..end].to_string(), end);
                }
            }
            _ => {}
        }
    }

    (input.to_string(), input.len())
}

fn normalize_flow_sequence(seq: &str) -> String {
    if !seq.starts_with('[') || !seq.ends_with(']') || seq.len() < 2 {
        return seq.to_string();
    }
    let inner = &seq[1..seq.len() - 1];
    let normalized = normalize_flow_sequence_content(inner);
    format!("[{}]", normalized)
}

fn normalize_flow_sequence_content(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut item = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut prev_was_escape = false;
    let mut brace_depth = 0usize;
    let mut idx = 0;

    while idx < content.len() {
        let ch = content[idx..].chars().next().expect("valid char");
        let ch_len = ch.len_utf8();

        if in_double && !prev_was_escape {
            prev_was_escape = ch == '\\';
        } else {
            prev_was_escape = false;
        }

        if !in_double && ch == '\'' {
            in_single = !in_single;
            item.push(ch);
            idx += ch_len;
            continue;
        }
        if !in_single && ch == '"' && !prev_was_escape {
            in_double = !in_double;
            item.push(ch);
            idx += ch_len;
            continue;
        }

        if in_single || in_double {
            item.push(ch);
            idx += ch_len;
            continue;
        }

        match ch {
            '[' => {
                let (nested, consumed) = extract_flow_sequence(&content[idx..]);
                item.push_str(&normalize_flow_sequence(&nested));
                idx += consumed;
                continue;
            }
            '{' => {
                brace_depth += 1;
                item.push(ch);
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                item.push(ch);
            }
            ',' if brace_depth == 0 => {
                out.push_str(&normalize_flow_item(&item));
                out.push(',');
                item.clear();
            }
            other => item.push(other),
        }

        idx += ch_len;
    }

    out.push_str(&normalize_flow_item(&item));
    out
}

fn normalize_flow_item(item: &str) -> String {
    let trimmed = item.trim();
    if trimmed.is_empty() {
        return item.to_string();
    }

    if trimmed.starts_with('"') || trimmed.starts_with('\'') {
        return item.to_string();
    }
    if trimmed.starts_with('[') || trimmed.starts_with('{') || trimmed.contains(':') {
        return item.to_string();
    }

    let lower = trimmed.to_ascii_lowercase();
    if matches!(lower.as_str(), "true" | "false" | "null" | "~") {
        return item.to_string();
    }
    if trimmed.parse::<f64>().is_ok() {
        return item.to_string();
    }

    let mut should_quote = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() || matches!(ch, '+' | '-' | '*' | '/' | '(' | ')') {
            should_quote = true;
            break;
        }
    }
    if !should_quote {
        return item.to_string();
    }

    let leading_len = item.len() - item.trim_start().len();
    let trailing_len = item.len() - item.trim_end().len();
    let leading = &item[..leading_len];
    let trailing = &item[item.len() - trailing_len..];
    let escaped = trimmed.replace('\'', "''");
    format!("{leading}'{escaped}'{trailing}")
}

#[cfg(test)]
mod yaml_normalize_tests {
    use super::*;

    #[test]
    fn normalizes_flow_sequence_holes_to_nulls() {
        let yaml = "a: [,10,,]\n";
        let v = Value::from_yaml_str(yaml).unwrap();
        let a = v.get_path("a").unwrap();
        let Value::Seq(seq) = a else {
            panic!("a should be seq")
        };
        assert_eq!(
            seq,
            &vec![Value::Null, Value::Number(10.0), Value::Null, Value::Null]
        );
    }

    #[test]
    fn strips_trailing_commas_in_flow_sequences() {
        let yaml = "a: [1, 2,]\n";
        let v = Value::from_yaml_str(yaml).unwrap();
        let a = v.get_path("a").unwrap();
        let Value::Seq(seq) = a else {
            panic!("a should be seq")
        };
        assert_eq!(seq, &vec![Value::Number(1.0), Value::Number(2.0)]);

        let yaml = "a: [\n  1,\n  2,\n]\n";
        let v = Value::from_yaml_str(yaml).unwrap();
        let a = v.get_path("a").unwrap();
        let Value::Seq(seq) = a else {
            panic!("a should be seq")
        };
        assert_eq!(seq, &vec![Value::Number(1.0), Value::Number(2.0)]);
    }

    #[test]
    fn normalizes_flow_sequence_expressions_to_strings() {
        let yaml = "a: [-ks * 0.5, kp * 0.25]\n";
        let v = Value::from_yaml_str(yaml).unwrap();
        let a = v.get_path("a").unwrap();
        let Value::Seq(seq) = a else {
            panic!("a should be seq")
        };
        assert_eq!(
            seq,
            &vec![
                Value::String("-ks * 0.5".to_string()),
                Value::String("kp * 0.25".to_string()),
            ]
        );
    }

    #[test]
    fn keeps_numeric_flow_sequence_values() {
        let yaml = "a: [1, -2.5, 3]\n";
        let v = Value::from_yaml_str(yaml).unwrap();
        let a = v.get_path("a").unwrap();
        let Value::Seq(seq) = a else {
            panic!("a should be seq")
        };
        assert_eq!(
            seq,
            &vec![Value::Number(1.0), Value::Number(-2.5), Value::Number(3.0)]
        );
    }
}
