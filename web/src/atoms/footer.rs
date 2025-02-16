use stylist::{style, Style};
use wasm_bindgen::prelude::*;
use web_sys::window;
use yew::prelude::*;

// Define the ergogen window binding
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    pub static ERGOGEN: JsValue;
}

fn get_footer_container_style() -> Style {
    style!(
        r#"
        display: flex;
        height: 3rem;
        width: 100%;
        align-items: center;
        justify-content: space-between;
        padding: 0 1rem 0.5rem 1rem;
        margin-top: auto;
        color: #FFFFFF;

        a {
            color: #28a745;
            text-decoration: none;
        }

        a:hover {
            color: #FFF;
        }
        "#
    )
    .unwrap()
}

#[function_component(Footer)]
pub fn footer() -> Html {
    let footer_style = get_footer_container_style();

    // Get the ergogen version from window
    let version = use_state(|| String::new());

    // Effect to get the version on component mount
    let cloned_version = version.clone();
    use_effect_with((), move |_| {
        if let Some(window) = window() {
            if let Ok(ergogen) = js_sys::Reflect::get(&window, &JsValue::from_str("ergogen")) {
                if let Ok(version_val) =
                    js_sys::Reflect::get(&ergogen, &JsValue::from_str("version"))
                {
                    if let Some(version_str) = version_val.as_string() {
                        cloned_version.set(version_str);
                    }
                }
            }
        }
        || ()
    });

    html! {
        <div class={footer_style}>
            <div>
                <a href="https://www.github.com/ergogen/ergogen" target="_blank" rel="noreferrer">
                    {"Ergogen by MrZealot"}
                </a>
            </div>
            <div>
                {"v"}{&*version}
            </div>
            <div>
                {"Powering the "}<a href="https://zealot.hu/absolem" target="_blank" rel="noreferrer">
                    {"Absolem"}
                </a>
            </div>
        </div>
    }
}
