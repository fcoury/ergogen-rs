use indexmap::IndexMap;
use serde_json::Value as JsonValue;

use crate::PcbError;
use crate::js_runtime::JsParamSpec;
use ergogen_parser::Value as ErgogenValue;

pub(crate) fn resolve_param_value(
    name: &str,
    spec: &JsParamSpec,
    params: &IndexMap<String, ErgogenValue>,
) -> Result<JsonValue, PcbError> {
    if let Some(value) = params.get(name) {
        return ergogen_value_to_json(value);
    }
    if let Some(default) = &spec.default {
        return Ok(default.clone());
    }
    if spec.required {
        return Err(PcbError::FootprintSpec(format!("missing js param {name}")));
    }
    Ok(JsonValue::Null)
}

pub(crate) fn resolve_net_name(
    name: &str,
    spec: &JsParamSpec,
    params: &IndexMap<String, ErgogenValue>,
) -> Result<String, PcbError> {
    if let Some(value) = params.get(name) {
        return match value {
            ErgogenValue::String(s) => Ok(s.clone()),
            ErgogenValue::Number(n) => Ok(format!("{}", n)),
            ErgogenValue::Bool(b) => Ok(b.to_string()),
            _ => Err(PcbError::FootprintSpec(format!(
                "invalid js net param {name}"
            ))),
        };
    }
    if let Some(default) = &spec.default {
        if let Some(s) = default.as_str() {
            return Ok(s.to_string());
        }
        if let Some(n) = default.as_f64() {
            return Ok(format!("{}", n));
        }
        if let Some(b) = default.as_bool() {
            return Ok(b.to_string());
        }
        return Ok(String::new());
    }
    if spec.required {
        return Err(PcbError::FootprintSpec(format!("missing js param {name}")));
    }
    Ok(String::new())
}

pub(crate) fn ergogen_value_to_json(value: &ErgogenValue) -> Result<JsonValue, PcbError> {
    let s = value.to_json_compact_string();
    serde_json::from_str(&s).map_err(|e| PcbError::FootprintSpec(e.to_string()))
}

pub(crate) fn resolve_designator(
    params_spec: &IndexMap<String, JsParamSpec>,
    params: &IndexMap<String, ErgogenValue>,
) -> String {
    if let Some(ErgogenValue::String(s)) = params.get("designator")
        && !s.is_empty()
    {
        return s.clone();
    }
    if let Some(spec) = params_spec.get("designator")
        && let Some(default) = &spec.default
        && let Some(s) = default.as_str()
        && !s.is_empty()
    {
        return s.to_string();
    }
    "FP".to_string()
}

pub(crate) fn next_ref(
    prefix: &str,
    refs: &mut std::collections::HashMap<String, usize>,
) -> String {
    let entry = refs.entry(prefix.to_string()).or_insert(0);
    *entry += 1;
    format!("{prefix}{}", *entry)
}
