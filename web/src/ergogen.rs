use stylist::{style, Style};
use web_sys::console;
use yew::prelude::*;

use crate::{atoms::Split, molecules::MonacoWrapper};

// First, let's define our styles
fn get_editor_container_style() -> Style {
    style!(
        r#"
        position: relative;
        height: 80%;
        display: flex;
        flex-direction: column;
        width: 100%;
        flex-grow: 1;
        "#
    )
    .unwrap()
}

fn get_flex_container_style() -> Style {
    style!(
        r#"
        display: flex;
        flex-flow: wrap;
        "#
    )
    .unwrap()
}

fn get_error_style() -> Style {
    style!(
        r#"
        background: #ff6d6d;
        color: #a31111;
        padding: 1em;
        margin: 0.5em 0 0.5em 0;
        border: 1px solid #a31111;
        border-radius: 0.3em;
        "#
    )
    .unwrap()
}

fn get_split_style() -> Style {
    style!(
        r#"
        width: 100%;
        height: 100%;
        display: flex;
        padding: 1rem;

        & .gutter {
            background-color: #878787;
            border-radius: 0.15rem;
            background-repeat: no-repeat;
            background-position: 50%;
        }

        & .gutter:hover {
            background-color: #a0a0a0;
        }
        "#
    )
    .unwrap()
}

fn get_left_split_pane_style() -> Style {
    style!(
        r#"
        padding-right: 1rem;
        position: relative;
        "#
    )
    .unwrap()
}

fn get_right_split_pane_style() -> Style {
    style!(
        r#"
        padding-left: 1rem;
        position: relative;
        "#
    )
    .unwrap()
}

// Properties for our component
#[derive(Properties, PartialEq)]
pub struct ErgogenProps {
    #[prop_or_default]
    pub children: Children,
}

// Define the ConfigOption type
#[derive(Clone, PartialEq)]
pub struct ConfigOption {
    pub label: String,
    pub value: String,
}

#[function_component(Ergogen)]
pub fn ergogen() -> Html {
    let preview_key = use_state(|| "demo.svg".to_string());
    let selected_option = use_state(|| None::<ConfigOption>);
    let error = use_state(|| None::<String>);

    let flex_container_style = get_flex_container_style();
    let editor_container_style = get_editor_container_style();

    let code = "test";

    html! {
        <div class={flex_container_style}>
            <Split direction="horizontal" sizes={vec![30.0, 70.0]} min_size={Some(100.0)} gutter_size={Some(10.0)} snap_offset={Some(0.0)}>
                <div>
                    <div class={editor_container_style}>
                        <MonacoWrapper
                            initial_value={code}
                        />
                        // TODO: Add Select component
                        // TODO: Add ConfigEditor
                        // TODO: Add Button
                        // TODO: Add OptionContainer with GenOptions
                        if let Some(error) = (*error).as_ref() {
                            <div class={get_error_style()}>
                                {error}
                            </div>
                        }
                    </div>
                </div>
                <div>
                    <Split direction="horizontal" sizes={vec![70.0, 30.0]} min_size={Some(100.0)} gutter_size={Some(10.0)} snap_offset={Some(0.0)}>
                        <div>
    <p>{"Preview Area - Current key: "}{(*preview_key).clone()}</p>
                            // TODO: Add FilePreview
                        </div>
                        <div>
    <p>{"Downloads Area"}</p>
                            // TODO: Add Downloads
                        </div>
                    </Split>
                </div>
            </Split>
        </div>
    }
}
