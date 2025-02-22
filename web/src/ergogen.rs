use monaco::api::TextModel;
use stylist::{style, Style};
use web_sys::{console, HtmlSelectElement};
use yew::prelude::*;

use crate::{atoms::Split, molecules::ConfigEditor};

fn get_editor_container_style() -> Style {
    style!(
        r#"
        position: relative;
        display: flex;
        flex-direction: column;
        width: 100%;
        height: calc(100vh - 4rem);
        overflow: hidden;

        & > select {
            margin-bottom: 1rem;
        }

        & > div {
            flex: 1;
            min-height: 0;
        }
        "#
    )
    .unwrap()
}

fn get_flex_container_style() -> Style {
    style!(
        r#"
        display: flex;
        flex-flow: wrap;
        width: 100%;
        height: 100vh;
        overflow: hidden;
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

#[derive(Clone, PartialEq)]
pub struct GroupedOption {
    pub label: String,
    pub options: Vec<ConfigOption>,
}

#[derive(Clone, PartialEq)]
pub struct ConfigOption {
    pub label: String,
    pub value: String,
}

#[function_component(Ergogen)]
pub fn ergogen() -> Html {
    let preview_key = use_state(|| "demo.svg".to_string());
    let error = use_state(|| None::<String>);

    let example_options = [
        GroupedOption {
            label: "Simple (points only)".to_string(),
            options: vec![ConfigOption {
                label: "Absolem (simplified)".to_string(),
                value: include_str!("examples/absolem.yaml").to_string(),
            }],
        },
        GroupedOption {
            label: "Empty configurations".to_string(),
            options: vec![ConfigOption {
                label: "Empty YAML configuration".to_string(),
                value: include_str!("examples/empty.yaml").to_string(),
            }],
        },
        GroupedOption {
            label: "Complete (with pcb)".to_string(),
            options: vec![],
        },
        GroupedOption {
            label: "Miscellaneous".to_string(),
            options: vec![],
        },
    ];

    // Get the first example option
    let first_example = example_options
        .iter()
        .flat_map(|group| &group.options)
        .next()
        .cloned()
        .unwrap_or(ConfigOption {
            label: "Empty YAML configuration".to_string(),
            value: include_str!("examples/empty.yaml").to_string(),
        });

    // Initialize states with the first example
    let selected_option = use_state(|| Some(first_example.clone()));
    let content =
        use_state_eq(|| TextModel::create(&first_example.value, Some("yaml"), None).unwrap());

    let flex_container_style = get_flex_container_style();
    let editor_container_style = get_editor_container_style();

    let example_options_cloned = example_options.clone();
    let content_cloned = content.clone();
    let selected_option_cloned = selected_option.clone();
    html! {
        <div class={flex_container_style}>
            <Split direction="horizontal" sizes={vec![30.0, 70.0]} min_size={Some(100.0)} gutter_size={Some(10.0)} snap_offset={Some(0.0)}>
                <div style="height: 100%; display: flex;">
                    <div class={editor_container_style} style="flex: 1;">
                        <select
                            placeholder="Paste your config below, or select an example to get started"
                            value={first_example.label.clone()}
                            onchange={Callback::from(move |e: Event| {
                                let input: HtmlSelectElement = e.target_unchecked_into();
                                let value = input.value();
                                if let Some(option) = example_options_cloned.iter().flat_map(|group| &group.options).find(|opt| opt.label == value) {
                                    selected_option_cloned.set(Some(option.clone()));
                                    content_cloned.set(TextModel::create(&option.value, Some("yaml"), None).unwrap());
                                    console::log_1(&format!("Selected option: {}", option.label).into());
                                    console::log_1(&format!("Selected value: {}", option.value).into());
                                } else {
                                    selected_option_cloned.set(None);
                                }
                            })}>
                            { for example_options.iter().map(|group| {
                                html! {
                                    <optgroup label={group.label.clone()}>
                                        { for group.options.iter().map(|option| {
                                            html! {
                                                <option value={option.label.clone()} selected={option.label == first_example.label}>
                                                    {&option.label}
                                                </option>
                                            }
                                        })}
                                    </optgroup>
                                }
                            }) }
                        </select>

                        // ConfigEditor
                        <ConfigEditor
                            text_model={(*content).clone()}
                        />
                        // TODO: Add Button
                        // TODO: Add OptionContainer with GenOptions
                        if let Some(error) = (*error).as_ref() {
                            <div class={get_error_style()}>
                                {error}
                            </div>
                        }
                    </div>
                </div>
                <div style="height: 100%; display: flex;">
                    <div style="flex: 1; height: 100%;">
                        <Split direction="horizontal" sizes={vec![70.0, 30.0]} min_size={Some(100.0)} gutter_size={Some(10.0)} snap_offset={Some(0.0)}>
                        <div style="height: 100%; display: flex; flex-direction: column; flex: 1;">
                            <p>{"Preview Area - Current key: "}{(*preview_key).clone()}</p>
                            // TODO: Add FilePreview
                        </div>
                        <div style="height: 100%; display: flex; flex-direction: column; flex: 1;">
                            <p>{"Downloads Area"}</p>
                            // TODO: Add Downloads
                        </div>
                        </Split>
                    </div>
                </div>
            </Split>
        </div>
    }
}
