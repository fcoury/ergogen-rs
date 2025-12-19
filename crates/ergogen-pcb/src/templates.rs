use ergogen_parser::Value;
use indexmap::IndexMap;

pub const KICAD5_HEADER: &str = include_str!("../templates/kicad5_header.tpl");
pub const KICAD8_HEADER: &str = include_str!("../templates/kicad8_header.tpl");

pub fn mx_template(params: &IndexMap<String, Value>) -> &'static str {
    let keycaps = param_bool(params, "keycaps");
    let reverse = param_bool(params, "reverse");
    let hotswap = param_bool(params, "hotswap");
    match (keycaps, reverse, hotswap) {
        (false, false, false) => include_str!("../templates/footprints/mx/base.tpl"),
        (true, false, false) => include_str!("../templates/footprints/mx/keycaps.tpl"),
        (false, true, false) => include_str!("../templates/footprints/mx/reverse.tpl"),
        (false, false, true) => include_str!("../templates/footprints/mx/hotswap.tpl"),
        (false, true, true) => include_str!("../templates/footprints/mx/reverse_hotswap.tpl"),
        (true, true, true) => include_str!("../templates/footprints/mx/keycaps_reverse_hotswap.tpl"),
        _ => include_str!("../templates/footprints/mx/base.tpl"),
    }
}

pub fn choc_template(params: &IndexMap<String, Value>) -> &'static str {
    let keycaps = param_bool(params, "keycaps");
    let reverse = param_bool(params, "reverse");
    let hotswap = param_bool(params, "hotswap");
    match (keycaps, reverse, hotswap) {
        (false, false, false) => include_str!("../templates/footprints/choc/base.tpl"),
        (true, false, false) => include_str!("../templates/footprints/choc/keycaps.tpl"),
        (false, true, false) => include_str!("../templates/footprints/choc/reverse.tpl"),
        (false, false, true) => include_str!("../templates/footprints/choc/hotswap.tpl"),
        (false, true, true) => include_str!("../templates/footprints/choc/reverse_hotswap.tpl"),
        (true, true, true) => include_str!("../templates/footprints/choc/keycaps_reverse_hotswap.tpl"),
        _ => include_str!("../templates/footprints/choc/base.tpl"),
    }
}

pub fn chocmini_template(params: &IndexMap<String, Value>) -> &'static str {
    let keycaps = param_bool(params, "keycaps");
    let reverse = param_bool(params, "reverse");
    match (keycaps, reverse) {
        (false, false) => include_str!("../templates/footprints/chocmini/base.tpl"),
        (true, false) => include_str!("../templates/footprints/chocmini/keycaps.tpl"),
        (false, true) => include_str!("../templates/footprints/chocmini/reverse.tpl"),
        (true, true) => include_str!("../templates/footprints/chocmini/keycaps_reverse.tpl"),
    }
}

pub fn diode_template() -> &'static str {
    include_str!("../templates/footprints/diode/base.tpl")
}

pub fn button_template(params: &IndexMap<String, Value>) -> &'static str {
    match param_str(params, "side").as_deref() {
        Some("B") => include_str!("../templates/footprints/button/back.tpl"),
        _ => include_str!("../templates/footprints/button/front.tpl"),
    }
}

pub fn pad_template(params: &IndexMap<String, Value>) -> &'static str {
    let align = param_str(params, "align").unwrap_or_default();
    let mirrored = param_bool(params, "mirrored");
    let front = params
        .get("front")
        .and_then(param_bool_opt)
        .unwrap_or(true);
    let has_text = params.get("text").is_some();

    if !front {
        return include_str!("../templates/footprints/pad/up_back.tpl");
    }
    if align == "right" && has_text {
        return include_str!("../templates/footprints/pad/right_text.tpl");
    }
    if mirrored && align == "down" {
        return include_str!("../templates/footprints/pad/down_mirrored.tpl");
    }
    if mirrored && align == "right" {
        return include_str!("../templates/footprints/pad/right_mirrored.tpl");
    }
    if mirrored && align == "left" {
        return include_str!("../templates/footprints/pad/left_mirrored.tpl");
    }
    include_str!("../templates/footprints/pad/base.tpl")
}

pub fn promicro_template(params: &IndexMap<String, Value>) -> &'static str {
    match param_str(params, "orientation").as_deref() {
        Some("up") => include_str!("../templates/footprints/promicro/up.tpl"),
        _ => include_str!("../templates/footprints/promicro/down.tpl"),
    }
}

pub fn trrs_template(params: &IndexMap<String, Value>) -> &'static str {
    let reverse = param_bool(params, "reverse");
    let symmetric = param_bool(params, "symmetric");
    match (reverse, symmetric) {
        (true, true) => include_str!("../templates/footprints/trrs/reverse_symmetric.tpl"),
        (true, false) => include_str!("../templates/footprints/trrs/reverse.tpl"),
        _ => include_str!("../templates/footprints/trrs/base.tpl"),
    }
}

pub fn injected_template() -> &'static str {
    include_str!("../templates/footprints/injected.tpl")
}

pub fn rest_template(what: &str, params: &IndexMap<String, Value>) -> (&'static str, &'static str) {
    match what {
        "alps" => (include_str!("../templates/footprints/rest/alps.tpl"), "S"),
        "jstph" => (include_str!("../templates/footprints/rest/jstph.tpl"), "JST"),
        "jumper" => (include_str!("../templates/footprints/rest/jumper.tpl"), "J"),
        "oled" => (include_str!("../templates/footprints/rest/oled.tpl"), "OLED"),
        "omron" => (include_str!("../templates/footprints/rest/omron.tpl"), "S"),
        "rgb" => (include_str!("../templates/footprints/rest/rgb.tpl"), "LED"),
        "rotary" => (include_str!("../templates/footprints/rest/rotary.tpl"), "ROT"),
        "scrollwheel" => {
            if param_bool(params, "reverse") {
                (include_str!("../templates/footprints/rest/scrollwheel_reverse.tpl"), "REF")
            } else {
                (include_str!("../templates/footprints/rest/scrollwheel.tpl"), "REF")
            }
        }
        "slider" => {
            if param_str(params, "side").as_deref() == Some("B") {
                (include_str!("../templates/footprints/rest/slider_back.tpl"), "T")
            } else {
                (include_str!("../templates/footprints/rest/slider.tpl"), "T")
            }
        }
        "via" => (include_str!("../templates/footprints/rest/via.tpl"), "REF"),
        _ => (include_str!("../templates/footprints/rest/alps.tpl"), "S"),
    }
}

pub fn trace_template(side: &str) -> &'static str {
    match side {
        "B" => include_str!("../templates/footprints/test/trace_b.tpl"),
        _ => include_str!("../templates/footprints/test/trace_f.tpl"),
    }
}

pub fn test_zone_template() -> &'static str {
    include_str!("../templates/footprints/test/zone.tpl")
}

pub fn test_dynamic_net_template() -> &'static str {
    include_str!("../templates/footprints/test/dynamic_net.tpl")
}

pub fn test_anchor_template() -> &'static str {
    include_str!("../templates/footprints/test/anchor.tpl")
}

pub fn test_arrobj_template() -> &'static str {
    include_str!("../templates/footprints/test/arrobj.tpl")
}

fn param_bool(params: &IndexMap<String, Value>, key: &str) -> bool {
    match params.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => *n != 0.0,
        Some(Value::String(s)) => s == "true",
        _ => false,
    }
}

fn param_bool_opt(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => Some(*n != 0.0),
        Value::String(s) => Some(s == "true"),
        _ => None,
    }
}

fn param_str(params: &IndexMap<String, Value>, key: &str) -> Option<String> {
    match params.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}
