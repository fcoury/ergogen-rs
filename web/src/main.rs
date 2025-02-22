mod atoms;
mod ergogen;
mod molecules;

use atoms::{Footer, Header};
use ergogen::Ergogen;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use stylist::{style, Style};
use web_sys::console;
use yew::functional::use_effect_with;
use yew::platform::spawn_local;
use yew::prelude::*;

fn get_app_container_style() -> Style {
    style!(
        r#"
        display: flex;
        flex-direction: column;
        color: #FFFFFF;
        height: 100vh;
        width: 100%;
        overflow: hidden;
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        "#,
    )
    .unwrap()
}

const CONFIG_LOCAL_STORAGE_KEY: &str = "LOCAL_STORAGE_CONFIG";

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Results {
    #[serde(flatten)]
    data: HashMap<String, serde_json::Value>,
}

#[derive(Clone, PartialEq)]
pub struct ConfigContext {
    pub config_input: String,
    pub error: Option<String>,
    pub results: Option<Results>,
    pub debug: bool,
    pub auto_gen: bool,
    pub auto_gen_3d: bool,
    pub set_config_input: Callback<String>,
    pub set_error: Callback<Option<String>>,
    pub set_results: Callback<Option<Results>>,
    pub set_debug: Callback<bool>,
    pub set_auto_gen: Callback<bool>,
    pub set_auto_gen_3d: Callback<bool>,
}

impl ConfigContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config_input: String,
        error: Option<String>,
        results: Option<Results>,
        debug: bool,
        auto_gen: bool,
        auto_gen_3d: bool,
        set_config_input: Callback<String>,
        set_error: Callback<Option<String>>,
        set_results: Callback<Option<Results>>,
        set_debug: Callback<bool>,
        set_auto_gen: Callback<bool>,
        set_auto_gen_3d: Callback<bool>,
    ) -> Self {
        Self {
            config_input,
            error,
            results,
            debug,
            auto_gen,
            auto_gen_3d,
            set_config_input,
            set_error,
            set_results,
            set_debug,
            set_auto_gen,
            set_auto_gen_3d,
        }
    }

    pub fn process_input(&self, input: &str, points_only: bool) -> Option<String> {
        let config_str = input.trim();
        if config_str.is_empty() {
            return Some("Empty configuration".into());
        }

        // Reset error state
        self.set_error.emit(None);

        // Parse config (try JSON first, then YAML)
        let parsed_config = match serde_json::from_str(config_str) {
            Ok(json) => json,
            Err(_) => match serde_yaml::from_str(config_str) {
                Ok(yaml) => yaml,
                Err(e) => {
                    return Some(format!("Invalid configuration: {}", e));
                }
            },
        };

        // Process the configuration
        self.process_config(parsed_config, points_only)
    }

    fn process_config(&self, config: serde_json::Value, points_only: bool) -> Option<String> {
        // Extract only needed fields if points_only is true
        let processed_config = if points_only {
            let mut filtered = serde_json::Map::new();
            if let Some(obj) = config.as_object() {
                for key in ["points", "units", "variables", "outlines"] {
                    if let Some(value) = obj.get(key) {
                        filtered.insert(key.to_string(), value.clone());
                    }
                }
            }
            serde_json::Value::Object(filtered)
        } else {
            config
        };

        // TODO: Call the actual ergogen processing function
        // For now, just store the processed config as results
        match processed_config.as_object() {
            Some(obj) => {
                let results = Results {
                    data: obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                };
                self.set_results.emit(Some(results));
                None
            }
            None => Some("Invalid configuration: not a valid object".into()),
        }
    }
}

#[function_component]
fn App() -> Html {
    // Create the style at runtime
    let style = get_app_container_style();

    // Load initial input from Absolem example
    let initial_input = include_str!("examples/absolem.yaml").to_string();

    // Initialize state
    let stored_config =
        LocalStorage::get(CONFIG_LOCAL_STORAGE_KEY).unwrap_or(initial_input.clone());
    let config_input = use_state(|| stored_config);
    let error = use_state(|| None);
    let results = use_state(|| None);
    let debug = use_state(|| true);
    let auto_gen = use_state(|| true);
    let auto_gen_3d = use_state(|| false);

    // Create callbacks
    let set_config_input = {
        let config_input = config_input.clone();
        Callback::from(move |input: String| {
            if let Err(e) = LocalStorage::set(CONFIG_LOCAL_STORAGE_KEY, &input) {
                console::log_1(&format!("Failed to save to local storage: {}", e).into());
            }
            config_input.set(input);
        })
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

    // Create the context
    let config_context = ConfigContext::new(
        (*config_input).clone(),
        (*error).clone(),
        (*results).clone(),
        *debug,
        *auto_gen,
        *auto_gen_3d,
        set_config_input,
        set_error,
        set_results,
        set_debug,
        set_auto_gen,
        set_auto_gen_3d,
    );

    // Set up effect for auto processing
    {
        let config = config_context.clone();
        let input = (*config_input).clone();
        use_effect_with(input, move |input| {
            let config = config.clone();
            let input = input.clone();
            if config.auto_gen {
                spawn_local(async move {
                    if let Some(err) = config.process_input(&input, !config.auto_gen_3d) {
                        config.set_error.emit(Some(err));
                    }
                });
            }
            || ()
        });
    }

    html! {
        <div class={style}>
            <Header />
            <ContextProvider<ConfigContext> context={config_context}>
                <Ergogen />
            </ContextProvider<ConfigContext>>
            <Footer />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
