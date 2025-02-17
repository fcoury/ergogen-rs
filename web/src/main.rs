mod atoms;
mod ergogen;
mod molecules;

use atoms::{Footer, Header};
use ergogen::Ergogen;
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

#[derive(Clone, PartialEq)]
pub struct Config {
    input: String,
}

#[derive(Clone, PartialEq)]
pub struct ConfigContext {
    config: Config,
}

impl ConfigContext {
    pub fn new(initial_input: String) -> Self {
        Self {
            config: Config {
                input: initial_input,
            },
        }
    }
}

#[function_component]
fn App() -> Html {
    // Create the style at runtime
    let style = get_app_container_style();

    // Assuming Absolem.value is defined somewhere
    let initial_input = String::from("absolem_value_here");
    let config_context = use_state(|| ConfigContext::new(initial_input));

    html! {
        <div class={style}>
            <Header />
            <ContextProvider<ConfigContext> context={(*config_context).clone()}>
                <Ergogen />
            </ContextProvider<ConfigContext>>
            <Footer />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
