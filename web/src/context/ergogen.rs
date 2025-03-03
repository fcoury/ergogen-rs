use serde_json::Value;
use std::collections::HashMap;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::console;
use yew::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub struct ErgogenResults {
    pub points: Option<Value>,
    pub outlines: HashMap<String, Value>,
    pub cases: HashMap<String, Value>,
    pub pcbs: HashMap<String, String>,
}

#[derive(Clone, PartialEq)]
pub struct ErgogenContext {
    pub config_input: String,
    pub results: Option<ErgogenResults>,
    pub error: Option<String>,
    pub debug: bool,
    pub auto_gen: bool,
    pub auto_gen_3d: bool,
    pub set_config_input: Callback<String>,
    pub set_error: Callback<Option<String>>,
    pub set_results: Callback<Option<ErgogenResults>>,
    pub set_debug: Callback<bool>,
    pub set_auto_gen: Callback<bool>,
    pub set_auto_gen_3d: Callback<bool>,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn ergogen(config: &str, debug: bool, logger: &Closure<dyn FnMut(String)>) -> js_sys::Promise;
}

impl ErgogenContext {
    pub async fn process_input(&self, input: &str) -> Option<String> {
        // Reset error state
        self.set_error.emit(None);

        // Create logger closure
        let logger = Closure::wrap(Box::new(|msg: String| {
            web_sys::console::log_1(&msg.into());
        }) as Box<dyn FnMut(String)>);

        // Process the configuration and await the Promise
        let promise = ergogen(input, self.debug, &logger);
        let result = match JsFuture::from(promise).await {
            Ok(val) => val,
            Err(e) => {
                console::log_1(&e);
                let error = format!("Error: {e:?}");
                self.set_error.emit(Some(error.clone()));
                return Some(error);
            }
        };

        // Convert result to ErgogenResults
        if let Ok(obj) = result.dyn_into::<js_sys::Object>() {
            // console::log_1(&format!("Got ergogen result: {:#?}", obj).into());
            if let Ok(value) = serde_wasm_bindgen::from_value::<Value>(obj.into()) {
                // console::log_1(&format!("{:#?}", value).into());
                let results = ErgogenResults {
                    points: value.get("points").cloned(),
                    outlines: value
                        .get("outlines")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default(),
                    cases: value
                        .get("cases")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default(),
                    pcbs: value
                        .get("pcbs")
                        .and_then(|v| v.as_object())
                        .map(|obj| {
                            obj.iter()
                                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                                .collect()
                        })
                        .unwrap_or_default(),
                };
                // console::log_1(&format!("result: {:#?}", results).into());
                self.set_results.emit(Some(results));
                None
            } else {
                console::log_1(&"Failed to convert ergogen result".into());
                let error = "Failed to convert ergogen result".to_string();
                self.set_error.emit(Some(error.clone()));
                Some(error)
            }
        } else {
            let error = "Failed to get ergogen result".to_string();
            self.set_error.emit(Some(error.clone()));
            Some(error)
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct ProviderProps {
    pub children: Children,
    pub initial_input: String,
}

#[function_component(ErgogenProvider)]
pub fn ergogen_provider(props: &ProviderProps) -> Html {
    let config_input = use_state(|| props.initial_input.clone());
    let error = use_state(|| None);
    let results = use_state(|| None);
    let debug = use_state(|| true);
    let auto_gen = use_state(|| true);
    let auto_gen_3d = use_state(|| false);

    let set_config_input = {
        let config_input = config_input.clone();
        Callback::from(move |input| config_input.set(input))
    };

    let set_error = {
        let error = error.clone();
        Callback::from(move |e| error.set(e))
    };

    let set_results = {
        let results = results.clone();
        Callback::from(move |r| results.set(r))
    };

    let set_debug = {
        let debug = debug.clone();
        Callback::from(move |d| debug.set(d))
    };

    let set_auto_gen = {
        let auto_gen = auto_gen.clone();
        Callback::from(move |a| auto_gen.set(a))
    };

    let set_auto_gen_3d = {
        let auto_gen_3d = auto_gen_3d.clone();
        Callback::from(move |a| auto_gen_3d.set(a))
    };

    let context = ErgogenContext {
        config_input: (*config_input).clone(),
        results: (*results).clone(),
        error: (*error).clone(),
        debug: *debug,
        auto_gen: *auto_gen,
        auto_gen_3d: *auto_gen_3d,
        set_config_input,
        set_error,
        set_results,
        set_debug,
        set_auto_gen,
        set_auto_gen_3d,
    };

    html! {
        <ContextProvider<ErgogenContext> context={context}>
            { for props.children.iter() }
        </ContextProvider<ErgogenContext>>
    }
}

#[hook]
pub fn use_ergogen_context() -> Option<ErgogenContext> {
    use_context::<ErgogenContext>()
}
