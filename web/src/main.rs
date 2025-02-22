mod atoms;
mod context;
mod ergogen;
mod molecules;

use atoms::{Footer, Header};
use context::ergogen::ErgogenProvider;
use ergogen::Ergogen;
use gloo_storage::{LocalStorage, Storage};
use stylist::{style, Style};
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

#[function_component]
fn App() -> Html {
    // Load initial input from Absolem example
    let initial_input = include_str!("examples/absolem.yaml").to_string();

    // Get stored config or use initial input
    let stored_config =
        LocalStorage::get(CONFIG_LOCAL_STORAGE_KEY).unwrap_or(initial_input.clone());

    html! {
        <div class={get_app_container_style()}>
            <Header />
            <ErgogenProvider initial_input={stored_config}>
                <Ergogen />
            </ErgogenProvider>
            <Footer />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
