use std::rc::Rc;
use stylist::{style, Style};
use yew::prelude::*;

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

// State struct to manage component state
#[derive(Clone)]
struct ErgogenState {
    preview_key: String,
    selected_option: Option<ConfigOption>,
    error: Option<String>,
}

#[function_component(Ergogen)]
pub fn ergogen(props: &ErgogenProps) -> Html {
    let state = use_state(|| ErgogenState {
        preview_key: "demo.svg".to_string(),
        selected_option: None,
        error: None,
    });

    // Styles
    let editor_container_style = get_editor_container_style();
    let flex_container_style = get_flex_container_style();
    let split_style = get_split_style();
    let left_pane_style = get_left_split_pane_style();
    let right_pane_style = get_right_split_pane_style();

    html! {
        <div class={flex_container_style}>
            <div class={split_style}>
                <div class={left_pane_style}>
                    <div class={editor_container_style}>
                        // TODO: Add Select component
                        // TODO: Add ConfigEditor
                        // TODO: Add Button
                        // TODO: Add OptionContainer with GenOptions
                        if let Some(error) = &state.error {
                            <div class={get_error_style()}>
                                {error}
                            </div>
                        }
                    </div>
                </div>
                <div class={right_pane_style}>
                    // TODO: Add nested split pane with FilePreview and Downloads
                </div>
            </div>
        </div>
    }
}
