use std::collections::HashMap;

use boa_engine::{
    Context, JsError, JsResult, JsString, JsValue, NativeFunction, Source, js_string,
    object::ObjectInitializer, property::Attribute,
};
use indexmap::IndexMap;
use serde_json::Value as JsonValue;

use crate::js_footprints_shared::{
    next_ref, resolve_designator, resolve_net_name, resolve_param_value,
};
use crate::js_runtime::{JsContext, JsParamSpec, parse_js_params};
use crate::{NetIndex, PcbError, Placement};
use ergogen_parser::Value as ErgogenValue;

#[derive(Debug)]
pub struct JsFootprintModule {
    pub params: IndexMap<String, JsParamSpec>,
    body: JsValue,
    ctx: Context,
}

pub fn load_js_module(source: &str) -> Result<JsFootprintModule, PcbError> {
    let mut ctx = Context::default();
    let wrapped = format!(
        "globalThis.module = {{ exports: {{}} }}; globalThis.exports = module.exports;\n{}\nmodule.exports;",
        source
    );
    let module_val = ctx
        .eval(Source::from_bytes(wrapped.as_bytes()))
        .map_err(|e| PcbError::FootprintSpec(e.to_string()))?;

    let module_obj = module_val
        .as_object()
        .ok_or_else(|| PcbError::FootprintSpec("module.exports must be object".to_string()))?;

    let params_val = module_obj
        .get(js_string!("params"), &mut ctx)
        .map_err(|e| PcbError::FootprintSpec(e.to_string()))?;
    let params_json = js_value_to_json(&params_val, &mut ctx)?;
    let params =
        parse_js_params(&params_json).map_err(|e| PcbError::FootprintSpec(e.to_string()))?;

    let body_val = module_obj
        .get(js_string!("body"), &mut ctx)
        .map_err(|e| PcbError::FootprintSpec(e.to_string()))?;

    Ok(JsFootprintModule {
        params,
        body: body_val,
        ctx,
    })
}

pub fn render_js_footprint(
    module: &mut JsFootprintModule,
    placement: Placement,
    params: &IndexMap<String, ErgogenValue>,
    refs: &mut HashMap<String, usize>,
    nets: &mut NetIndex,
    side: String,
) -> Result<String, PcbError> {
    let designator = resolve_designator_from_module(module, params);
    let ref_str = next_ref(&designator, refs);
    let ctx = &mut module.ctx;
    let mut js_ctx = JsContext::new(placement, ref_str, true, side, nets);
    let p_val = build_p_object(ctx, &mut js_ctx, &module.params, params)?;
    let body_fn = module
        .body
        .as_object()
        .ok_or_else(|| PcbError::FootprintSpec("footprint body must be function".to_string()))?;
    let result = body_fn
        .call(&JsValue::Undefined, &[p_val], ctx)
        .map_err(|e| PcbError::FootprintSpec(e.to_string()))?;
    let rendered = result
        .as_string()
        .ok_or_else(|| PcbError::FootprintSpec("footprint body must return string".to_string()))?
        .to_std_string()
        .map_err(|e| PcbError::FootprintSpec(e.to_string()))?;
    Ok(rendered)
}

fn build_p_object(
    ctx: &mut Context,
    js_ctx: &mut JsContext<'_>,
    module_params: &IndexMap<String, JsParamSpec>,
    params: &IndexMap<String, ErgogenValue>,
) -> Result<JsValue, PcbError> {
    let mut resolved = Vec::with_capacity(module_params.len());
    for (name, spec) in module_params {
        let js_val = if spec.kind == crate::js_runtime::JsParamKind::Net {
            let net_name = resolve_net_name(name, spec, params)?;
            let net = js_ctx.global_net(&net_name);
            net_to_js(ctx, net).map_err(js_err)?
        } else {
            let value = resolve_param_value(name, spec, params)?;
            json_to_js_value(&value, ctx)?
        };
        resolved.push((name, js_val));
    }

    let mut builder = ObjectInitializer::new(ctx);
    builder
        .property(
            js_string!("at"),
            JsString::from(js_ctx.at()),
            Attribute::all(),
        )
        .property(js_string!("r"), JsValue::from(js_ctx.r()), Attribute::all())
        .property(
            js_string!("rot"),
            JsValue::from(js_ctx.rot()),
            Attribute::all(),
        )
        .property(
            js_string!("ref"),
            JsString::from(js_ctx.ref_str()),
            Attribute::all(),
        )
        .property(
            js_string!("ref_hide"),
            JsString::from(js_ctx.ref_hide()),
            Attribute::all(),
        )
        .property(
            js_string!("side"),
            JsString::from(js_ctx.side()),
            Attribute::all(),
        );

    builder
        .function(make_xy_fn(js_ctx), js_string!("xy"), 2)
        .function(make_eaxy_fn(js_ctx), js_string!("eaxy"), 2)
        .function(make_local_net_fn(js_ctx), js_string!("local_net"), 1)
        .function(make_global_net_fn(js_ctx), js_string!("global_net"), 1);

    for (name, js_val) in resolved {
        builder.property(JsString::from(name.as_str()), js_val, Attribute::all());
    }

    Ok(builder.build().into())
}

fn json_to_js_value(value: &JsonValue, ctx: &mut Context) -> Result<JsValue, PcbError> {
    JsValue::from_json(value, ctx).map_err(js_err)
}

fn js_value_to_json(value: &JsValue, ctx: &mut Context) -> Result<JsonValue, PcbError> {
    if value.is_null() || value.is_undefined() {
        return Ok(JsonValue::Null);
    }
    value.to_json(ctx).map_err(js_err)
}

fn make_xy_fn(js_ctx: &mut JsContext<'_>) -> NativeFunction {
    let ptr = js_ctx as *mut JsContext as *mut JsContext<'static>;
    NativeFunction::from_copy_closure(move |_, args, _ctx| {
        let x = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0);
        let y = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0);
        let ctx = unsafe { &mut *ptr };
        Ok(JsValue::from(JsString::from(ctx.xy(x, y))))
    })
}

fn make_eaxy_fn(js_ctx: &mut JsContext<'_>) -> NativeFunction {
    let ptr = js_ctx as *mut JsContext as *mut JsContext<'static>;
    NativeFunction::from_copy_closure(move |_, args, _ctx| {
        let x = args.get(0).and_then(|v| v.as_number()).unwrap_or(0.0);
        let y = args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0);
        let ctx = unsafe { &mut *ptr };
        Ok(JsValue::from(JsString::from(ctx.eaxy(x, y))))
    })
}

fn make_local_net_fn(js_ctx: &mut JsContext<'_>) -> NativeFunction {
    let ptr = js_ctx as *mut JsContext as *mut JsContext<'static>;
    NativeFunction::from_copy_closure(move |_, args, ctx| {
        let name = args
            .get(0)
            .and_then(|v| v.as_string())
            .map(|s| s.to_std_string().unwrap_or_default())
            .unwrap_or_default();
        let js_ctx = unsafe { &mut *ptr };
        let net = js_ctx.local_net(&name);
        net_to_js(ctx, net)
    })
}

fn make_global_net_fn(js_ctx: &mut JsContext<'_>) -> NativeFunction {
    let ptr = js_ctx as *mut JsContext as *mut JsContext<'static>;
    NativeFunction::from_copy_closure(move |_, args, ctx| {
        let name = args
            .get(0)
            .and_then(|v| v.as_string())
            .map(|s| s.to_std_string().unwrap_or_default())
            .unwrap_or_default();
        let js_ctx = unsafe { &mut *ptr };
        let net = js_ctx.global_net(&name);
        net_to_js(ctx, net)
    })
}

fn net_to_js(ctx: &mut Context, net: crate::js_runtime::JsNet) -> JsResult<JsValue> {
    let to_string = NativeFunction::from_copy_closure(|this, _args, ctx| {
        if let Some(obj) = this.as_object() {
            return obj.get(js_string!("str"), ctx);
        }
        Ok(JsValue::from(js_string!("")))
    });
    let mut builder = ObjectInitializer::new(ctx);
    builder
        .property(
            js_string!("name"),
            JsString::from(net.name.as_str()),
            Attribute::all(),
        )
        .property(
            js_string!("index"),
            JsValue::from(net.index as f64),
            Attribute::all(),
        )
        .property(
            js_string!("str"),
            JsString::from(net.str.as_str()),
            Attribute::all(),
        )
        .function(to_string, js_string!("toString"), 0);
    Ok(builder.build().into())
}

fn js_err(e: JsError) -> PcbError {
    PcbError::FootprintSpec(e.to_string())
}

fn resolve_designator_from_module(
    module: &JsFootprintModule,
    params: &IndexMap<String, ErgogenValue>,
) -> String {
    resolve_designator(&module.params, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use std::collections::HashMap;

    #[test]
    fn renders_js_footprint_with_nets_and_helpers() {
        let source = r#"
module.exports = {
  params: {
    net_param: { type: "net", value: "GND" },
    num: { type: "number", value: 2 }
  },
  body: p => `${p.at} ${p.xy(1, 2)} ${p.local_net("A")} ${p.global_net("B")} ${p.net_param} ${p.num}`
};
"#;
        let mut module = load_js_module(source).unwrap();
        let placement = Placement {
            x: 0.0,
            y: 0.0,
            r: 0.0,
            mirrored: false,
        };
        let params = IndexMap::new();
        let mut refs = HashMap::new();
        let mut nets = NetIndex::default();
        let rendered = render_js_footprint(
            &mut module,
            placement,
            &params,
            &mut refs,
            &mut nets,
            "F".to_string(),
        )
        .unwrap();
        assert!(rendered.contains("(at "));
        assert!(rendered.contains("1 2"));
        assert!(rendered.contains("\"FP1_A\""));
        assert!(rendered.contains("\"B\""));
        assert!(rendered.contains("\"GND\""));
    }
}
