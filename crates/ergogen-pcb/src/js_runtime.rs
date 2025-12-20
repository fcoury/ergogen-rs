use indexmap::IndexMap;
use serde_json::Value as JsonValue;

use crate::{NetIndex, Placement, escape_kicad_text, fmt_num, rotate_ccw, to_kicad_xy};

#[derive(Debug, thiserror::Error)]
pub enum JsRuntimeError {
    #[error("invalid js params: expected object")]
    ParamsNotObject,
    #[error("invalid js param `{0}`: {1}")]
    ParamInvalid(String, &'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsParamKind {
    Net,
    Number,
    String,
    Boolean,
    Array,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsParamSpec {
    pub kind: JsParamKind,
    pub required: bool,
    pub default: Option<JsonValue>,
}

pub fn parse_js_params(value: &JsonValue) -> Result<IndexMap<String, JsParamSpec>, JsRuntimeError> {
    let obj = value.as_object().ok_or(JsRuntimeError::ParamsNotObject)?;
    let mut out = IndexMap::new();
    for (name, val) in obj {
        let spec = parse_js_param_value(name, val)?;
        out.insert(name.clone(), spec);
    }
    Ok(out)
}

fn parse_js_param_value(name: &str, value: &JsonValue) -> Result<JsParamSpec, JsRuntimeError> {
    if let Some(obj) = value.as_object() {
        if let Some(kind_val) = obj.get("type") {
            let kind_str = kind_val.as_str().ok_or_else(|| {
                JsRuntimeError::ParamInvalid(name.to_string(), "type must be string")
            })?;
            let kind = match kind_str {
                "net" => JsParamKind::Net,
                "number" => JsParamKind::Number,
                "string" => JsParamKind::String,
                "boolean" => JsParamKind::Boolean,
                "array" => JsParamKind::Array,
                _ => {
                    return Err(JsRuntimeError::ParamInvalid(
                        name.to_string(),
                        "unknown type",
                    ));
                }
            };
            let value_field = obj.get("value");
            let required = value_field.is_none() || value_field.is_some_and(|v| v.is_null());
            let default = value_field.cloned().filter(|v| !v.is_null());
            if let Some(default) = &default
                && !default_matches_kind(kind, default)
            {
                return Err(JsRuntimeError::ParamInvalid(
                    name.to_string(),
                    "default type mismatch",
                ));
            }
            Ok(JsParamSpec {
                kind,
                required,
                default,
            })
        } else {
            Err(JsRuntimeError::ParamInvalid(
                name.to_string(),
                "missing type",
            ))
        }
    } else {
        let kind = match value {
            JsonValue::String(_) => JsParamKind::String,
            JsonValue::Number(_) => JsParamKind::Number,
            JsonValue::Bool(_) => JsParamKind::Boolean,
            JsonValue::Array(_) => JsParamKind::Array,
            JsonValue::Null => {
                return Ok(JsParamSpec {
                    kind: JsParamKind::Net,
                    required: false,
                    default: None,
                });
            }
            _ => {
                return Err(JsRuntimeError::ParamInvalid(
                    name.to_string(),
                    "unsupported default type",
                ));
            }
        };
        Ok(JsParamSpec {
            kind,
            required: false,
            default: Some(value.clone()),
        })
    }
}

fn default_matches_kind(kind: JsParamKind, value: &JsonValue) -> bool {
    match kind {
        JsParamKind::Net | JsParamKind::String => value.is_string(),
        JsParamKind::Number => value.is_number(),
        JsParamKind::Boolean => value.is_boolean(),
        JsParamKind::Array => value.is_array(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JsNet {
    pub name: String,
    pub index: usize,
    pub str: String,
}

pub(crate) struct JsContext<'a> {
    at: String,
    r: f64,
    rot: f64,
    ref_str: String,
    ref_hide: String,
    side: String,
    at_x: f64,
    at_y: f64,
    nets: &'a mut NetIndex,
}

impl<'a> JsContext<'a> {
    pub(crate) fn new(
        placement: Placement,
        ref_str: String,
        ref_hide: bool,
        side: String,
        nets: &'a mut NetIndex,
    ) -> Self {
        let (at_x, at_y) = to_kicad_xy(placement.x, placement.y);
        let at = format!(
            "(at {} {} {})",
            fmt_num(at_x),
            fmt_num(at_y),
            fmt_num(placement.r)
        );
        Self {
            at,
            r: placement.r,
            rot: placement.r,
            ref_str,
            ref_hide: if ref_hide {
                "hide".to_string()
            } else {
                "".to_string()
            },
            side,
            at_x,
            at_y,
            nets,
        }
    }

    pub(crate) fn at(&self) -> &str {
        &self.at
    }

    pub(crate) fn r(&self) -> f64 {
        self.r
    }

    pub(crate) fn rot(&self) -> f64 {
        self.rot
    }

    pub(crate) fn ref_str(&self) -> &str {
        &self.ref_str
    }

    pub(crate) fn ref_hide(&self) -> &str {
        &self.ref_hide
    }

    pub(crate) fn side(&self) -> &str {
        &self.side
    }

    pub(crate) fn xy(&self, x: f64, y: f64) -> String {
        let (dx, dy) = rotate_ccw((x, y), -self.r);
        let nx = self.at_x + dx;
        let ny = self.at_y + dy;
        format!("{} {}", fmt_num(nx), fmt_num(ny))
    }

    pub(crate) fn eaxy(&self, x: f64, y: f64) -> String {
        self.xy(x, y)
    }

    pub(crate) fn local_net(&mut self, id: &str) -> JsNet {
        let name = format!("{}_{}", self.ref_str, id);
        self.net_from_name(name)
    }

    pub(crate) fn global_net(&mut self, name: &str) -> JsNet {
        self.net_from_name(name.to_string())
    }

    fn net_from_name(&mut self, name: String) -> JsNet {
        if name.is_empty() {
            return JsNet {
                name,
                index: 0,
                str: "(net 0 \"\")".to_string(),
            };
        }
        let index = self.nets.ensure(&name);
        let safe = escape_kicad_text(&name);
        JsNet {
            name,
            index,
            str: format!("(net {} \"{}\")", index, safe),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_js_param_defaults_and_types() {
        let params = json!({
            "designator": "MH",
            "side": "F",
            "hole_size": "2.2",
            "hole_drill": "2.2",
            "from": { "type": "net", "value": null },
            "width": { "type": "number", "value": 0.25 },
            "flag": { "type": "boolean", "value": true },
            "coords": { "type": "array", "value": [0, 1, 2] }
        });

        let specs = parse_js_params(&params).unwrap();
        assert_eq!(specs["designator"].kind, JsParamKind::String);
        assert_eq!(specs["hole_size"].kind, JsParamKind::String);
        assert_eq!(specs["width"].kind, JsParamKind::Number);
        assert_eq!(specs["flag"].kind, JsParamKind::Boolean);
        assert_eq!(specs["coords"].kind, JsParamKind::Array);

        let net_spec = &specs["from"];
        assert_eq!(net_spec.kind, JsParamKind::Net);
        assert!(net_spec.required);
        assert!(net_spec.default.is_none());
    }

    #[test]
    fn xy_and_eaxy_apply_rotation_and_translation() {
        let placement = Placement {
            x: 10.0,
            y: 20.0,
            r: 90.0,
            mirrored: false,
        };
        let mut nets = NetIndex::default();
        let ctx = JsContext::new(
            placement,
            "U1".to_string(),
            true,
            "F".to_string(),
            &mut nets,
        );

        assert_eq!(ctx.at(), "(at 10 -20 90)");
        assert_eq!(ctx.r(), 90.0);
        assert_eq!(ctx.rot(), 90.0);
        assert_eq!(ctx.ref_str(), "U1");
        assert_eq!(ctx.ref_hide(), "hide");
        assert_eq!(ctx.side(), "F");

        // rotate (1,2) by -90 => (2, -1), then translate by (10, -20)
        assert_eq!(ctx.xy(1.0, 2.0), "12 -21");
        assert_eq!(ctx.eaxy(1.0, 2.0), "12 -21");
    }

    #[test]
    fn local_and_global_net_naming_and_indices() {
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            r: 0.0,
            mirrored: false,
        };
        let mut nets = NetIndex::default();
        let mut ctx = JsContext::new(
            placement,
            "MCU1".to_string(),
            false,
            "F".to_string(),
            &mut nets,
        );

        let local = ctx.local_net("24");
        assert_eq!(local.name, "MCU1_24");
        assert_eq!(local.index, 1);
        assert_eq!(local.str, "(net 1 \"MCU1_24\")");

        let gnd = ctx.global_net("GND");
        assert_eq!(gnd.name, "GND");
        assert_eq!(gnd.index, 2);
        assert_eq!(gnd.str, "(net 2 \"GND\")");

        let gnd_again = ctx.global_net("GND");
        assert_eq!(gnd_again.index, 2);
    }
}
