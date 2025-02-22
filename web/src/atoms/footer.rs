use stylist::{style, Style};
use wasm_bindgen::prelude::*;
use yew::prelude::*;

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
        "#
    )
    .unwrap()
}

fn get_link_style() -> Style {
    style!(
        r#"
        color: #28a745;
        text-decoration: none;
        "#
    )
    .unwrap()
}

fn get_link_hover_style() -> Style {
    style!(
        r#"
        &:hover {
            color: #FFF;
        }
        "#
    )
    .unwrap()
}

#[function_component(Footer)]
pub fn footer() -> Html {
    let version = use_state(|| String::from("unknown"));

    {
        let version = version.clone();
        use_effect_with((), move |_| {
            let version_str = web_sys::window()
                .and_then(|win| js_sys::Reflect::get(&win, &JsValue::from_str("ergogen")).ok())
                .and_then(|ergogen| {
                    js_sys::Reflect::get(&ergogen, &JsValue::from_str("version")).ok()
                })
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "unknown".to_string());
            version.set(version_str);
            || ()
        });
    }

    html! {
        <div class={get_footer_container_style()}>
            <div>
                <a class={classes!(get_link_style(), get_link_hover_style())} href="https://www.github.com/ergogen/ergogen" target="_blank">
                    {"Ergogen by MrZealot"}
                </a>
            </div>
            <div>
                {"v"}{(*version).clone()}
            </div>
            <div>
                {"Powering the "}
                <a class={classes!(get_link_style(), get_link_hover_style())} href="https://zealot.hu/absolem" target="_blank">
                    {"Absolem"}
                </a>
            </div>
        </div>
    }
}
