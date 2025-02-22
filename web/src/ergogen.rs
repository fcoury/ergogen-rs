use monaco::api::TextModel;
use stylist::{style, Style};
use wasm_bindgen_futures::spawn_local;
use web_sys::{console, HtmlSelectElement};
use yew::prelude::*;

use crate::{
    atoms::{Button, Split},
    context::ergogen::use_ergogen_context,
    molecules::{ConfigEditor, Downloads, FilePreview},
};

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
    let context = use_ergogen_context();

    let preview_content = if let Some(context) = &context {
        if let Some(results) = &context.results {
            // Try to find the preview content in the results
            let mut content = String::new();
            let parts: Vec<&str> = preview_key.split('.').collect();
            if parts.len() > 1 {
                let name = parts[0];
                let ext = parts[1];
                match ext {
                    "svg" => {
                        if let Some(outline) = results.outlines.get(name) {
                            if let Some(svg) = outline.get("svg") {
                                content = svg.as_str().unwrap_or_default().to_string();
                            }
                        }
                    }
                    "jscad" => {
                        if let Some(case) = results.cases.get(name) {
                            if let Some(jscad) = case.get("jscad") {
                                content = jscad.as_str().unwrap_or_default().to_string();
                            }
                        }
                    }
                    _ => {}
                }
            }
            content
        } else {
            String::new()
        }
    } else {
        String::new()
    };

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
    let first_example_label = first_example.label.clone();
    html! {
        <div class={flex_container_style}>
            <Split direction="horizontal" sizes={vec![30.0, 70.0]} min_size={Some(100.0)} gutter_size={Some(10.0)} snap_offset={Some(0.0)}>
                <div style="height: 100%; display: flex;">
                    <div class={editor_container_style} style="flex: 1;">
                        <select
                            placeholder="Paste your config below, or select an example to get started"
                            value={first_example.label}
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
                                    <optgroup label={group.label.to_string()}>
                                        { for group.options.iter().map(|option| {
                                            let label = option.label.to_string();
                                            html! {
                                                <option value={label.clone()} selected={option.label == first_example_label}>
                                                    {label}
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

                        // Generate button and options
                        if let Some(context) = &context {
                            <div>
                                <Button onclick={
                                    let context = context.clone();
                                    let content = (*content).clone();
                                    Callback::from(move |_| {
                                        let input = content.get_value();
                                        let context = context.clone();
                                        spawn_local(async move {
                                            context.process_input(&input, false).await;
                                        });
                                    })
                                }>{"Generate"}</Button>

                                <div style="display: flex; justify-content: space-between; margin-top: 1rem;">
                                    <label>
                                        <input
                                            type="checkbox"
                                            checked={context.auto_gen}
                                            onchange={
                                                let set_auto_gen = context.set_auto_gen.clone();
                                                Callback::from(move |e: Event| {
                                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                    set_auto_gen.emit(input.checked());
                                                })
                                            }
                                        />
                                        {"Auto-generate"}
                                    </label>

                                    <label>
                                        <input
                                            type="checkbox"
                                            checked={context.debug}
                                            onchange={
                                                let set_debug = context.set_debug.clone();
                                                Callback::from(move |e: Event| {
                                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                    set_debug.emit(input.checked());
                                                })
                                            }
                                        />
                                        {"Debug"}
                                    </label>

                                    <label>
                                        <input
                                            type="checkbox"
                                            checked={context.auto_gen_3d}
                                            onchange={
                                                let set_auto_gen_3d = context.set_auto_gen_3d.clone();
                                                Callback::from(move |e: Event| {
                                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                                    set_auto_gen_3d.emit(input.checked());
                                                })
                                            }
                                        />
                                        {"Auto-gen 3D "}
                                        <small>{"(slow)"}</small>
                                    </label>
                                </div>
                            </div>
                        }

                        if let Some(context) = &context {
                            if let Some(error) = &context.error {
                                <div class={get_error_style()}>
                                    {error}
                                </div>
                            }
                        }
                    </div>
                </div>
                <div style="height: 100%; display: flex;">
                    <div style="flex: 1; height: 100%;">
                        <Split direction="horizontal" sizes={vec![70.0, 30.0]} min_size={Some(100.0)} gutter_size={Some(10.0)} snap_offset={Some(0.0)}>
                        <div style="height: 100%; display: flex; flex-direction: column; flex: 1;">
                            <FilePreview
                                preview_key={(*preview_key).clone()}
                                preview_content={(*preview_content).to_string()}
                            />
                        </div>
                        <div style="height: 100%; display: flex; flex-direction: column; flex: 1;">
                            <Downloads
                                set_preview={Callback::from(move |key: String| preview_key.set(key))}
                            />
                        </div>
                        </Split>
                    </div>
                </div>
            </Split>
        </div>
    }
}
