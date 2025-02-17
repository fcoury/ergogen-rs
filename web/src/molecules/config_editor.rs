use monaco::{
    api::{CodeEditorOptions, TextModel},
    sys::editor::BuiltinTheme,
    yew::CodeEditor,
};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ConfigEditorProps {
    #[prop_or_default]
    pub class: Classes,
    pub config_input: String,
    pub on_change: Callback<String>,
}

// Inner editor component that handles the Monaco instance
#[derive(Properties, PartialEq)]
struct MonacoEditorProps {
    text_model: TextModel,
    on_change: Callback<String>,
}

#[function_component(MonacoEditor)]
fn monaco_editor(props: &MonacoEditorProps) -> Html {
    let options = CodeEditorOptions::default()
        .with_language("json".to_owned())
        .with_builtin_theme(BuiltinTheme::VsDark)
        .with_automatic_layout(true);

    let on_change = {
        let callback = props.on_change.clone();
        Callback::from(move |_: String| {
            // Use the text model to get the current value
            let current_value = props.text_model.get_value();
            callback.emit(current_value);
        })
    };

    html! {
        <CodeEditor
            classes={"full-height"}
            options={options.to_sys_options()}
            model={props.text_model.clone()}
            on_change={on_change}
        />
    }
}

#[function_component(ConfigEditor)]
pub fn config_editor(props: &ConfigEditorProps) -> Html {
    // Create and manage TextModel with use_state_eq
    let text_model = use_state_eq(|| {
        TextModel::create(&props.config_input, Some("json"), None)
            .expect("Failed to create text model")
    });

    // Update text model when config_input changes
    let text_model_clone = text_model.clone();
    use_effect_with(props.config_input.clone(), move |config_input| {
        text_model_clone.set_value(config_input);
        || ()
    });

    html! {
        <div class={props.class.clone()}>
            <div style="height: 70vh;">
                <MonacoEditor
                    text_model={(*text_model).clone()}
                    on_change={props.on_change.clone()}
                />
            </div>
        </div>
    }
}
