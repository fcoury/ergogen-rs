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

// pub struct ConfigEditor {
//     options: CodeEditorOptions,
//     editor_link: CodeEditorLink,
// }
//
// impl Component for ConfigEditor {
//     type Message = ();
//     type Properties = ConfigEditorProps;
//
//     fn create(ctx: &Context<Self>) -> Self {
//         let options = CodeEditorOptions::default()
//             .with_language("yaml".to_owned())
//             .with_value(ctx.props().initial_value.clone())
//             .with_builtin_theme(BuiltinTheme::VsDark)
//             .with_automatic_layout(true);
//
//         Self {
//             options,
//             editor_link: CodeEditorLink::new(),
//         }
//     }
//
//     fn view(&self, ctx: &Context<Self>) -> Html {
//         let editor_container_style = style!(
//             r#"
//             height: 100%;
//             min-height: 500px;
//             width: 100%;
//             display: flex;
//             flex-direction: column;
//
//             & > div {
//                 flex-grow: 1;
//                 height: 100%;
//             }
//             "#
//         )
//         .unwrap();
//         let on_editor_created = {
//             let on_change = ctx.props().on_change.clone();
//             let editor_link = self.editor_link.clone();
//
//             Callback::from(move |_: CodeEditorLink| {
//                 editor_link.with_editor(|editor| {
//                     if let Some(model) = editor.get_model() {
//                         let model = model.clone();
//                         let on_change = on_change.clone();
//
//                         let model_clone = model.clone();
//                         _ = model.on_did_change_content(Box::new(move |_event| {
//                             let content = model_clone.get_value();
//                             on_change.emit(content);
//                         }));
//                     }
//                 });
//             })
//         };
//
//         let ConfigEditorProps { text_model, .. } = props;
//
//         html! {
//             <div class={editor_container_style}>
//                 <CodeEditor
//                     classes={"full-height"}
//                     options={Some(self.options.to_sys_options())}
//             model{
//                     on_editor_created={on_editor_created}
//                     link={self.editor_link.clone()}
//                 />
//             </div>
//         }
//     }
// }
