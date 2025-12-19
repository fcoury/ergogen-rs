use ergogen_parser::Value;
use indexmap::IndexMap;

#[derive(Debug, thiserror::Error)]
pub enum FootprintSpecError {
    #[error("invalid footprint spec: {0}")]
    Invalid(&'static str),
    #[error("invalid number at {0}")]
    InvalidNumber(&'static str),
    #[error("invalid vector at {0}")]
    InvalidVector(&'static str),
    #[error("invalid string at {0}")]
    InvalidString(&'static str),
    #[error("invalid bool at {0}")]
    InvalidBool(&'static str),
    #[error("missing param {0}")]
    MissingParam(String),
    #[error("invalid param type for {0}")]
    InvalidParamType(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FootprintSpec {
    pub name: String,
    pub params: IndexMap<String, ParamSpec>,
    pub primitives: Vec<Primitive>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamSpec {
    pub kind: ParamKind,
    pub required: bool,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Net,
    Boolean,
    Number,
    String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarSpec {
    Number(f64),
    Template(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Pad {
        at: [ScalarSpec; 2],
        size: [ScalarSpec; 2],
        layers: Vec<String>,
        net: String,
    },
    PadThru {
        at: [ScalarSpec; 2],
        size: [ScalarSpec; 2],
        drill: ScalarSpec,
        layers: Vec<String>,
        net: String,
        shape: Option<String>,
    },
    Circle {
        center: [ScalarSpec; 2],
        radius: ScalarSpec,
        layer: String,
        width: ScalarSpec,
    },
    Line {
        start: [ScalarSpec; 2],
        end: [ScalarSpec; 2],
        layer: String,
        width: ScalarSpec,
    },
    Arc {
        center: [ScalarSpec; 2],
        radius: ScalarSpec,
        start_angle: ScalarSpec,
        angle: ScalarSpec,
        layer: String,
        width: ScalarSpec,
    },
    Rect {
        center: [ScalarSpec; 2],
        size: [ScalarSpec; 2],
        layer: String,
        width: ScalarSpec,
    },
    Text {
        at: [ScalarSpec; 2],
        text: String,
        layer: String,
        size: [ScalarSpec; 2],
        thickness: ScalarSpec,
        rotation: ScalarSpec,
        justify: Option<String>,
        hide: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFootprint {
    pub name: String,
    pub params: IndexMap<String, Value>,
    pub primitives: Vec<ResolvedPrimitive>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedPrimitive {
    Pad {
        at: [f64; 2],
        size: [f64; 2],
        layers: Vec<String>,
        net: String,
    },
    PadThru {
        at: [f64; 2],
        size: [f64; 2],
        drill: f64,
        layers: Vec<String>,
        net: String,
        shape: Option<String>,
    },
    Circle {
        center: [f64; 2],
        radius: f64,
        layer: String,
        width: f64,
    },
    Line {
        start: [f64; 2],
        end: [f64; 2],
        layer: String,
        width: f64,
    },
    Arc {
        center: [f64; 2],
        radius: f64,
        start_angle: f64,
        angle: f64,
        layer: String,
        width: f64,
    },
    Rect {
        center: [f64; 2],
        size: [f64; 2],
        layer: String,
        width: f64,
    },
    Text {
        at: [f64; 2],
        text: String,
        layer: String,
        size: [f64; 2],
        thickness: f64,
        rotation: f64,
        justify: Option<String>,
        hide: bool,
    },
}

pub fn parse_footprint_spec(yaml: &str) -> Result<FootprintSpec, FootprintSpecError> {
    let value = Value::from_yaml_str(yaml).map_err(|_| FootprintSpecError::Invalid("yaml"))?;
    parse_footprint_value(&value)
}

pub fn resolve_footprint_spec(
    spec: &FootprintSpec,
    params: &IndexMap<String, Value>,
) -> Result<ResolvedFootprint, FootprintSpecError> {
    let resolved_params = resolve_params(spec, params)?;
    let vars = params_to_vars(&resolved_params)?;
    let mut primitives = Vec::new();
    for primitive in &spec.primitives {
        match primitive {
            Primitive::Pad {
                at,
                size,
                layers,
                net,
            } => {
                let at = resolve_vec2(at, &vars)?;
                let size = resolve_vec2(size, &vars)?;
                let net = interpolate(net, &vars)?;
                let mut resolved_layers = Vec::new();
                for layer in layers {
                    resolved_layers.push(interpolate(layer, &vars)?);
                }
                primitives.push(ResolvedPrimitive::Pad {
                    at,
                    size,
                    layers: resolved_layers,
                    net,
                });
            }
            Primitive::PadThru {
                at,
                size,
                drill,
                layers,
                net,
                shape,
            } => {
                let at = resolve_vec2(at, &vars)?;
                let size = resolve_vec2(size, &vars)?;
                let drill = resolve_scalar(drill, &vars)?;
                let net = interpolate(net, &vars)?;
                let mut resolved_layers = Vec::new();
                for layer in layers {
                    resolved_layers.push(interpolate(layer, &vars)?);
                }
                let shape = match shape {
                    Some(shape) => Some(interpolate(shape, &vars)?),
                    None => None,
                };
                primitives.push(ResolvedPrimitive::PadThru {
                    at,
                    size,
                    drill,
                    layers: resolved_layers,
                    net,
                    shape,
                });
            }
            Primitive::Circle {
                center,
                radius,
                layer,
                width,
            } => {
                let center = resolve_vec2(center, &vars)?;
                let radius = resolve_scalar(radius, &vars)?;
                let layer = interpolate(layer, &vars)?;
                let width = resolve_scalar(width, &vars)?;
                primitives.push(ResolvedPrimitive::Circle {
                    center,
                    radius,
                    layer,
                    width,
                });
            }
            Primitive::Line {
                start,
                end,
                layer,
                width,
            } => {
                let start = resolve_vec2(start, &vars)?;
                let end = resolve_vec2(end, &vars)?;
                let layer = interpolate(layer, &vars)?;
                let width = resolve_scalar(width, &vars)?;
                primitives.push(ResolvedPrimitive::Line {
                    start,
                    end,
                    layer,
                    width,
                });
            }
            Primitive::Arc {
                center,
                radius,
                start_angle,
                angle,
                layer,
                width,
            } => {
                let center = resolve_vec2(center, &vars)?;
                let radius = resolve_scalar(radius, &vars)?;
                let start_angle = resolve_scalar(start_angle, &vars)?;
                let angle = resolve_scalar(angle, &vars)?;
                let layer = interpolate(layer, &vars)?;
                let width = resolve_scalar(width, &vars)?;
                primitives.push(ResolvedPrimitive::Arc {
                    center,
                    radius,
                    start_angle,
                    angle,
                    layer,
                    width,
                });
            }
            Primitive::Rect {
                center,
                size,
                layer,
                width,
            } => {
                let center = resolve_vec2(center, &vars)?;
                let size = resolve_vec2(size, &vars)?;
                let layer = interpolate(layer, &vars)?;
                let width = resolve_scalar(width, &vars)?;
                primitives.push(ResolvedPrimitive::Rect {
                    center,
                    size,
                    layer,
                    width,
                });
            }
            Primitive::Text {
                at,
                text,
                layer,
                size,
                thickness,
                rotation,
                justify,
                hide,
            } => {
                let at = resolve_vec2(at, &vars)?;
                let text = interpolate(text, &vars)?;
                let layer = interpolate(layer, &vars)?;
                let size = resolve_vec2(size, &vars)?;
                let thickness = resolve_scalar(thickness, &vars)?;
                let rotation = resolve_scalar(rotation, &vars)?;
                let justify = match justify {
                    Some(justify) => Some(interpolate(justify, &vars)?),
                    None => None,
                };
                primitives.push(ResolvedPrimitive::Text {
                    at,
                    text,
                    layer,
                    size,
                    thickness,
                    rotation,
                    justify,
                    hide: *hide,
                });
            }
        }
    }
    Ok(ResolvedFootprint {
        name: spec.name.clone(),
        params: resolved_params,
        primitives,
    })
}

fn parse_footprint_value(value: &Value) -> Result<FootprintSpec, FootprintSpecError> {
    let Value::Map(map) = value else {
        return Err(FootprintSpecError::Invalid("root must be map"));
    };
    let name = map
        .get("name")
        .and_then(value_as_str)
        .ok_or(FootprintSpecError::Invalid("name"))?
        .to_string();
    let params = match map.get("params") {
        Some(Value::Map(param_map)) => parse_params(param_map)?,
        Some(Value::Null) | None => IndexMap::new(),
        _ => return Err(FootprintSpecError::Invalid("params")),
    };
    let primitives = match map.get("primitives") {
        Some(Value::Seq(seq)) => parse_primitives(seq)?,
        _ => return Err(FootprintSpecError::Invalid("primitives")),
    };

    Ok(FootprintSpec {
        name,
        params,
        primitives,
    })
}

fn parse_params(
    map: &IndexMap<String, Value>,
) -> Result<IndexMap<String, ParamSpec>, FootprintSpecError> {
    let mut out = IndexMap::new();
    for (name, v) in map {
        let Value::Map(param) = v else {
            return Err(FootprintSpecError::Invalid("params.<name>"));
        };
        let kind = param
            .get("type")
            .and_then(value_as_str)
            .ok_or(FootprintSpecError::Invalid("params.<name>.type"))?;
        let kind = match kind {
            "net" => ParamKind::Net,
            "boolean" => ParamKind::Boolean,
            "number" => ParamKind::Number,
            "string" => ParamKind::String,
            _ => return Err(FootprintSpecError::Invalid("params.<name>.type")),
        };
        let required = param
            .get("required")
            .map(|v| value_as_bool(v, "params.<name>.required"))
            .transpose()?
            .unwrap_or(false);
        let default = param.get("default").cloned();
        out.insert(
            name.clone(),
            ParamSpec {
                kind,
                required,
                default,
            },
        );
    }
    Ok(out)
}

fn parse_primitives(seq: &[Value]) -> Result<Vec<Primitive>, FootprintSpecError> {
    let mut out = Vec::new();
    for v in seq {
        let Value::Map(map) = v else {
            return Err(FootprintSpecError::Invalid("primitives.<item>"));
        };
        let kind = map
            .get("type")
            .and_then(value_as_str)
            .ok_or(FootprintSpecError::Invalid("primitives.<item>.type"))?;
        match kind {
            "pad" => {
                let at = parse_vec2(map.get("at"), "primitives.pad.at")?;
                let size = parse_vec2(map.get("size"), "primitives.pad.size")?;
                let layers = parse_str_list(map.get("layers"), "primitives.pad.layers")?;
                let net = map
                    .get("net")
                    .and_then(value_as_str)
                    .ok_or(FootprintSpecError::Invalid("primitives.pad.net"))?
                    .to_string();
                out.push(Primitive::Pad {
                    at,
                    size,
                    layers,
                    net,
                });
            }
            "pad_thru" => {
                let at = parse_vec2(map.get("at"), "primitives.pad_thru.at")?;
                let size = parse_vec2(map.get("size"), "primitives.pad_thru.size")?;
                let drill = parse_scalar(
                    map.get("drill").ok_or(FootprintSpecError::Invalid(
                        "primitives.pad_thru.drill",
                    ))?,
                    "primitives.pad_thru.drill",
                )?;
                let layers =
                    parse_str_list(map.get("layers"), "primitives.pad_thru.layers")?;
                let net = map
                    .get("net")
                    .and_then(value_as_str)
                    .ok_or(FootprintSpecError::Invalid("primitives.pad_thru.net"))?
                    .to_string();
                let shape = map.get("shape").and_then(value_as_str).map(|s| s.to_string());
                out.push(Primitive::PadThru {
                    at,
                    size,
                    drill,
                    layers,
                    net,
                    shape,
                });
            }
            "circle" => {
                let center = parse_vec2(map.get("center"), "primitives.circle.center")?;
                let radius = parse_scalar(
                    map.get("radius")
                        .ok_or(FootprintSpecError::Invalid("primitives.circle.radius"))?,
                    "primitives.circle.radius",
                )?;
                let layer = map
                    .get("layer")
                    .and_then(value_as_str)
                    .ok_or(FootprintSpecError::Invalid("primitives.circle.layer"))?
                    .to_string();
                let width = parse_scalar(
                    map.get("width")
                        .ok_or(FootprintSpecError::Invalid("primitives.circle.width"))?,
                    "primitives.circle.width",
                )?;
                out.push(Primitive::Circle {
                    center,
                    radius,
                    layer,
                    width,
                });
            }
            "line" => {
                let start = parse_vec2(map.get("start"), "primitives.line.start")?;
                let end = parse_vec2(map.get("end"), "primitives.line.end")?;
                let layer = map
                    .get("layer")
                    .and_then(value_as_str)
                    .ok_or(FootprintSpecError::Invalid("primitives.line.layer"))?
                    .to_string();
                let width = parse_scalar(
                    map.get("width")
                        .ok_or(FootprintSpecError::Invalid("primitives.line.width"))?,
                    "primitives.line.width",
                )?;
                out.push(Primitive::Line {
                    start,
                    end,
                    layer,
                    width,
                });
            }
            "arc" => {
                let center = parse_vec2(map.get("center"), "primitives.arc.center")?;
                let radius = parse_scalar(
                    map.get("radius")
                        .ok_or(FootprintSpecError::Invalid("primitives.arc.radius"))?,
                    "primitives.arc.radius",
                )?;
                let start_angle = parse_scalar(
                    map.get("start_angle")
                        .ok_or(FootprintSpecError::Invalid("primitives.arc.start_angle"))?,
                    "primitives.arc.start_angle",
                )?;
                let angle = parse_scalar(
                    map.get("angle")
                        .ok_or(FootprintSpecError::Invalid("primitives.arc.angle"))?,
                    "primitives.arc.angle",
                )?;
                let layer = map
                    .get("layer")
                    .and_then(value_as_str)
                    .ok_or(FootprintSpecError::Invalid("primitives.arc.layer"))?
                    .to_string();
                let width = parse_scalar(
                    map.get("width")
                        .ok_or(FootprintSpecError::Invalid("primitives.arc.width"))?,
                    "primitives.arc.width",
                )?;
                out.push(Primitive::Arc {
                    center,
                    radius,
                    start_angle,
                    angle,
                    layer,
                    width,
                });
            }
            "rect" => {
                let center = parse_vec2(map.get("center"), "primitives.rect.center")?;
                let size = parse_vec2(map.get("size"), "primitives.rect.size")?;
                let layer = map
                    .get("layer")
                    .and_then(value_as_str)
                    .ok_or(FootprintSpecError::Invalid("primitives.rect.layer"))?
                    .to_string();
                let width = parse_scalar(
                    map.get("width")
                        .ok_or(FootprintSpecError::Invalid("primitives.rect.width"))?,
                    "primitives.rect.width",
                )?;
                out.push(Primitive::Rect {
                    center,
                    size,
                    layer,
                    width,
                });
            }
            "text" => {
                let at = parse_vec2(map.get("at"), "primitives.text.at")?;
                let text = map
                    .get("text")
                    .and_then(value_as_str)
                    .ok_or(FootprintSpecError::Invalid("primitives.text.text"))?
                    .to_string();
                let layer = map
                    .get("layer")
                    .and_then(value_as_str)
                    .ok_or(FootprintSpecError::Invalid("primitives.text.layer"))?
                    .to_string();
                let size = parse_vec2_opt(map.get("size"), "primitives.text.size", [1.0, 1.0])?;
                let thickness = parse_scalar_opt(
                    map.get("thickness"),
                    "primitives.text.thickness",
                    0.15,
                )?;
                let rotation = parse_scalar_opt(
                    map.get("rotation"),
                    "primitives.text.rotation",
                    0.0,
                )?;
                let justify = map.get("justify").and_then(value_as_str).map(|s| s.to_string());
                let hide = parse_bool_opt(map.get("hide"), "primitives.text.hide", false)?;
                out.push(Primitive::Text {
                    at,
                    text,
                    layer,
                    size,
                    thickness,
                    rotation,
                    justify,
                    hide,
                });
            }
            _ => return Err(FootprintSpecError::Invalid("primitives.<item>.type")),
        }
    }
    Ok(out)
}

fn parse_vec2(
    v: Option<&Value>,
    at: &'static str,
) -> Result<[ScalarSpec; 2], FootprintSpecError> {
    let Some(v) = v else {
        return Err(FootprintSpecError::InvalidVector(at));
    };
    match v {
        Value::Seq(seq) if seq.len() == 2 => {
            let x = parse_scalar(&seq[0], at)?;
            let y = parse_scalar(&seq[1], at)?;
            Ok([x, y])
        }
        _ => Err(FootprintSpecError::InvalidVector(at)),
    }
}

fn parse_vec2_opt(
    v: Option<&Value>,
    at: &'static str,
    default: [f64; 2],
) -> Result<[ScalarSpec; 2], FootprintSpecError> {
    match v {
        Some(v) => parse_vec2(Some(v), at),
        None => Ok([ScalarSpec::Number(default[0]), ScalarSpec::Number(default[1])]),
    }
}

fn parse_scalar_opt(
    v: Option<&Value>,
    at: &'static str,
    default: f64,
) -> Result<ScalarSpec, FootprintSpecError> {
    match v {
        Some(v) => parse_scalar(v, at),
        None => Ok(ScalarSpec::Number(default)),
    }
}

fn parse_bool_opt(
    v: Option<&Value>,
    at: &'static str,
    default: bool,
) -> Result<bool, FootprintSpecError> {
    match v {
        Some(v) => value_as_bool(v, at),
        None => Ok(default),
    }
}

fn parse_str_list(v: Option<&Value>, at: &'static str) -> Result<Vec<String>, FootprintSpecError> {
    let Some(v) = v else {
        return Err(FootprintSpecError::InvalidString(at));
    };
    match v {
        Value::Seq(seq) => seq
            .iter()
            .map(|v| {
                value_as_str(v)
                    .map(|s| s.to_string())
                    .ok_or(FootprintSpecError::InvalidString(at))
            })
            .collect(),
        _ => Err(FootprintSpecError::InvalidString(at)),
    }
}

fn value_as_str(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn value_as_bool(v: &Value, at: &'static str) -> Result<bool, FootprintSpecError> {
    match v {
        Value::Bool(b) => Ok(*b),
        _ => Err(FootprintSpecError::InvalidBool(at)),
    }
}

fn parse_scalar(v: &Value, at: &'static str) -> Result<ScalarSpec, FootprintSpecError> {
    match v {
        Value::Number(n) => Ok(ScalarSpec::Number(*n)),
        Value::String(s) => Ok(ScalarSpec::Template(s.clone())),
        _ => Err(FootprintSpecError::InvalidNumber(at)),
    }
}

fn resolve_params(
    spec: &FootprintSpec,
    params: &IndexMap<String, Value>,
) -> Result<IndexMap<String, Value>, FootprintSpecError> {
    let mut out = IndexMap::new();
    for (name, param) in &spec.params {
        if let Some(value) = params.get(name) {
            validate_param_kind(name, param.kind, value)?;
            out.insert(name.clone(), value.clone());
        } else if let Some(default) = &param.default {
            validate_param_kind(name, param.kind, default)?;
            out.insert(name.clone(), default.clone());
        } else if param.required {
            return Err(FootprintSpecError::MissingParam(name.clone()));
        }
    }
    Ok(out)
}

fn validate_param_kind(
    name: &str,
    kind: ParamKind,
    value: &Value,
) -> Result<(), FootprintSpecError> {
    match kind {
        ParamKind::Net | ParamKind::String => {
            if matches!(value, Value::String(_)) {
                Ok(())
            } else {
                Err(FootprintSpecError::InvalidParamType(name.to_string()))
            }
        }
        ParamKind::Boolean => {
            if matches!(value, Value::Bool(_)) {
                Ok(())
            } else {
                Err(FootprintSpecError::InvalidParamType(name.to_string()))
            }
        }
        ParamKind::Number => {
            if matches!(value, Value::Number(_)) {
                Ok(())
            } else {
                Err(FootprintSpecError::InvalidParamType(name.to_string()))
            }
        }
    }
}

fn params_to_vars(
    params: &IndexMap<String, Value>,
) -> Result<IndexMap<String, String>, FootprintSpecError> {
    let mut out = IndexMap::new();
    for (name, value) in params {
        out.insert(name.clone(), value_to_string(name, value)?);
    }
    Ok(out)
}

fn value_to_string(name: &str, value: &Value) -> Result<String, FootprintSpecError> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(format!("{}", n)),
        Value::Bool(b) => Ok(if *b { "true".to_string() } else { "false".to_string() }),
        _ => Err(FootprintSpecError::InvalidParamType(name.to_string())),
    }
}

fn interpolate(
    raw: &str,
    vars: &IndexMap<String, String>,
) -> Result<String, FootprintSpecError> {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let key = after[..end].trim();
            if let Some(val) = vars.get(key) {
                out.push_str(val);
            } else {
                return Err(FootprintSpecError::MissingParam(key.to_string()));
            }
            rest = &after[end + 2..];
        } else {
            out.push_str(rest);
            return Ok(out);
        }
    }
    out.push_str(rest);
    Ok(out)
}

fn resolve_vec2(
    v: &[ScalarSpec; 2],
    vars: &IndexMap<String, String>,
) -> Result<[f64; 2], FootprintSpecError> {
    Ok([
        resolve_scalar(&v[0], vars)?,
        resolve_scalar(&v[1], vars)?,
    ])
}

fn resolve_scalar(
    v: &ScalarSpec,
    vars: &IndexMap<String, String>,
) -> Result<f64, FootprintSpecError> {
    match v {
        ScalarSpec::Number(n) => Ok(*n),
        ScalarSpec::Template(raw) => {
            let rendered = interpolate(raw, vars)?;
            rendered
                .trim()
                .parse::<f64>()
                .map_err(|_| FootprintSpecError::InvalidNumber("template"))
        }
    }
}
