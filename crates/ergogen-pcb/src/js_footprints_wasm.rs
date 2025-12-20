use std::collections::HashMap;

use indexmap::IndexMap;
use js_sys::{Function, Object, Reflect};
use serde_json::Value as JsonValue;
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};

use crate::js_footprints_shared::{
    next_ref, resolve_designator, resolve_net_name, resolve_param_value,
};
use crate::js_runtime::{JsNet, JsParamSpec, parse_js_params};
use crate::vfs;
use crate::{NetIndex, PcbError, Placement, escape_kicad_text, fmt_num, rotate_ccw, to_kicad_xy};
use ergogen_parser::Value as ErgogenValue;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = ergogenRenderJsFootprint)]
    fn render_js_footprint(source: &str, p: JsValue) -> String;

    #[wasm_bindgen(js_name = ergogenJsFootprintParams)]
    fn js_footprint_params(source: &str) -> JsValue;

    #[wasm_bindgen(js_name = ergogenLoadJsFootprintSource)]
    fn load_js_footprint_source(path: &str) -> String;
}

pub fn load_js_source(path: &std::path::Path) -> Result<String, PcbError> {
    let path_str = path.to_string_lossy();
    if let Some(source) = vfs::read(&path_str) {
        if source.trim().is_empty() {
            return Err(PcbError::FootprintSpec(format!(
                "empty JS footprint source for {path_str}"
            )));
        }
        return Ok(source);
    }
    let source = load_js_footprint_source(&path_str);
    if source.trim().is_empty() {
        return Err(PcbError::FootprintSpec(format!(
            "empty JS footprint source for {path_str}"
        )));
    }
    Ok(source)
}

pub fn render_js_footprint_wasm(
    source: &str,
    placement: Placement,
    params: &IndexMap<String, ErgogenValue>,
    refs: &mut HashMap<String, usize>,
    nets: &mut NetIndex,
    side: String,
) -> Result<String, PcbError> {
    let params_val = js_footprint_params(source);
    let params_json: JsonValue = serde_wasm_bindgen::from_value(params_val)
        .map_err(|e| PcbError::FootprintSpec(e.to_string()))?;
    let params_spec =
        parse_js_params(&params_json).map_err(|e| PcbError::FootprintSpec(e.to_string()))?;

    let designator = resolve_designator(&params_spec, params);
    let ref_str = next_ref(&designator, refs);

    let p = build_p_object(placement, &ref_str, &side, &params_spec, params, nets)?;

    Ok(render_js_footprint(source, p))
}

fn build_p_object(
    placement: Placement,
    ref_str: &str,
    side: &str,
    params_spec: &IndexMap<String, JsParamSpec>,
    params: &IndexMap<String, ErgogenValue>,
    nets: &mut NetIndex,
) -> Result<JsValue, PcbError> {
    let (at_x, at_y) = to_kicad_xy(placement.x, placement.y);
    let at = format!(
        "(at {} {} {})",
        fmt_num(at_x),
        fmt_num(at_y),
        fmt_num(placement.r)
    );
    let ref_hide = "hide";
    let r = placement.r;
    let rot = placement.r;

    let obj = Object::new();
    Reflect::set(&obj, &JsValue::from_str("at"), &JsValue::from_str(&at)).map_err(js_err)?;
    Reflect::set(&obj, &JsValue::from_str("r"), &JsValue::from_f64(r)).map_err(js_err)?;
    Reflect::set(&obj, &JsValue::from_str("rot"), &JsValue::from_f64(rot)).map_err(js_err)?;
    Reflect::set(&obj, &JsValue::from_str("ref"), &JsValue::from_str(ref_str)).map_err(js_err)?;
    Reflect::set(
        &obj,
        &JsValue::from_str("ref_hide"),
        &JsValue::from_str(ref_hide),
    )
    .map_err(js_err)?;
    Reflect::set(&obj, &JsValue::from_str("side"), &JsValue::from_str(side)).map_err(js_err)?;

    let xy_fn = make_xy_fn(at_x, at_y, r);
    Reflect::set(
        &obj,
        &JsValue::from_str("xy"),
        xy_fn.as_ref().unchecked_ref(),
    )
    .map_err(js_err)?;
    let eaxy_fn = make_eaxy_fn(at_x, at_y, r);
    Reflect::set(
        &obj,
        &JsValue::from_str("eaxy"),
        eaxy_fn.as_ref().unchecked_ref(),
    )
    .map_err(js_err)?;

    let nets_ptr = nets as *mut NetIndex as *mut NetIndex;
    let ref_str = ref_str.to_string();
    let local_net_fn = make_local_net_fn(nets_ptr, ref_str.clone());
    Reflect::set(
        &obj,
        &JsValue::from_str("local_net"),
        local_net_fn.as_ref().unchecked_ref(),
    )
    .map_err(js_err)?;
    let global_net_fn = make_global_net_fn(nets_ptr);
    Reflect::set(
        &obj,
        &JsValue::from_str("global_net"),
        global_net_fn.as_ref().unchecked_ref(),
    )
    .map_err(js_err)?;

    let mut resolved = Vec::with_capacity(params_spec.len());
    for (name, spec) in params_spec {
        let value = if spec.kind == crate::js_runtime::JsParamKind::Net {
            let net_name = resolve_net_name(name, spec, params)?;
            let net = net_from_name(nets, net_name);
            net_to_js(net)?
        } else {
            let value = resolve_param_value(name, spec, params)?;
            serde_wasm_bindgen::to_value(&value)
                .map_err(|e| PcbError::FootprintSpec(e.to_string()))?
        };
        resolved.push((name, value));
    }

    for (name, value) in resolved {
        Reflect::set(&obj, &JsValue::from_str(name.as_str()), &value).map_err(js_err)?;
    }

    xy_fn.forget();
    eaxy_fn.forget();
    local_net_fn.forget();
    global_net_fn.forget();
    Ok(obj.into())
}

fn make_xy_fn(at_x: f64, at_y: f64, r: f64) -> Closure<dyn FnMut(f64, f64) -> JsValue> {
    Closure::wrap(Box::new(move |x: f64, y: f64| -> JsValue {
        let (dx, dy) = rotate_ccw((x, y), -r);
        let nx = at_x + dx;
        let ny = at_y + dy;
        JsValue::from_str(&format!("{} {}", fmt_num(nx), fmt_num(ny)))
    }))
}

fn make_eaxy_fn(at_x: f64, at_y: f64, r: f64) -> Closure<dyn FnMut(f64, f64) -> JsValue> {
    make_xy_fn(at_x, at_y, r)
}

fn make_local_net_fn(
    nets_ptr: *mut NetIndex,
    ref_str: String,
) -> Closure<dyn FnMut(JsValue) -> JsValue> {
    Closure::wrap(Box::new(move |id: JsValue| -> JsValue {
        let raw = js_value_to_string(id);
        let name = format!("{}_{}", ref_str, raw);
        let net = unsafe { net_from_name(&mut *nets_ptr, name) };
        net_to_js(net).unwrap_or_else(|_| JsValue::NULL)
    }))
}

fn make_global_net_fn(nets_ptr: *mut NetIndex) -> Closure<dyn FnMut(JsValue) -> JsValue> {
    Closure::wrap(Box::new(move |name: JsValue| -> JsValue {
        let raw = js_value_to_string(name);
        let net = unsafe { net_from_name(&mut *nets_ptr, raw) };
        // See Boa implementation: `global_net` is most often used where KiCad expects a numeric id.
        JsValue::from_f64(net.index as f64)
    }))
}

fn js_value_to_string(value: JsValue) -> String {
    if let Some(s) = value.as_string() {
        return s;
    }
    if let Some(n) = value.as_f64() {
        return format!("{}", n);
    }
    if let Some(b) = value.as_bool() {
        return b.to_string();
    }
    String::new()
}

fn net_from_name(nets: &mut NetIndex, name: String) -> JsNet {
    if name.is_empty() {
        return JsNet {
            name,
            index: 0,
            str: "(net 0 \"\")".to_string(),
        };
    }
    let index = nets.ensure(&name);
    let safe = escape_kicad_text(&name);
    JsNet {
        name,
        index,
        str: format!("(net {} \"{}\")", index, safe),
    }
}

fn net_to_js(net: JsNet) -> Result<JsValue, PcbError> {
    let obj = Object::new();
    Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &JsValue::from_str(&net.name),
    )
    .map_err(js_err)?;
    Reflect::set(
        &obj,
        &JsValue::from_str("index"),
        &JsValue::from_f64(net.index as f64),
    )
    .map_err(js_err)?;
    Reflect::set(
        &obj,
        &JsValue::from_str("str"),
        &JsValue::from_str(&net.str),
    )
    .map_err(js_err)?;
    let to_string = Function::new_no_args("return this.str;");
    Reflect::set(&obj, &JsValue::from_str("toString"), &to_string.into()).map_err(js_err)?;
    Ok(obj.into())
}

fn js_err(err: JsValue) -> PcbError {
    PcbError::FootprintSpec(format!("js bridge error: {err:?}"))
}
