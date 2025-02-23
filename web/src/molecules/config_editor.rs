use monaco::{
    api::{CodeEditorOptions, TextModel},
    sys::editor::BuiltinTheme,
    yew::CodeEditor,
};
use stylist::style;
use yew::{function_component, html, Callback, Html, Properties};

#[derive(Properties, PartialEq)]
pub struct ConfigEditorProps {
    #[prop_or_default]
    pub on_change: Callback<String>,
    #[prop_or_default]
    pub initial_value: String,
    pub text_model: TextModel,
}

fn get_options() -> CodeEditorOptions {
    CodeEditorOptions::default()
        .with_language("yaml".to_owned())
        .with_value("".to_owned())
        .with_builtin_theme(BuiltinTheme::VsDark)
        .with_automatic_layout(true)
}

#[function_component(ConfigEditor)]
pub fn config_editor(props: &ConfigEditorProps) -> Html {
    let ConfigEditorProps { text_model, .. } = props;

    let editor_container_style = style!(
        r#"
        height: 100%;
        width: 100%;
        display: flex;
        flex-direction: column;
        flex: 1;

        & > div {
            flex: 1;
            min-height: 0;
        }
        "#
    )
    .unwrap();

    html! {
        <div class={editor_container_style}>
            <CodeEditor
                classes={"full-height"}
                options={get_options().to_sys_options()}
                model={text_model.clone()} />
        </div>
    }
}
