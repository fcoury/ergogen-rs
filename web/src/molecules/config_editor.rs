use monaco::yew::{CodeEditorLink, CodeEditorProps};
use monaco::{api::CodeEditorOptions, sys::editor::BuiltinTheme, yew::CodeEditor};
use std::rc::Rc;
use stylist::style;
use yew::{html, Callback, Component, Context, Html, Properties};

#[derive(Properties, PartialEq)]
pub struct MonacoProps {
    #[prop_or_default]
    pub on_change: Callback<String>,
    #[prop_or_default]
    pub initial_value: String,
}

pub struct MonacoWrapper {
    options: CodeEditorOptions,
    editor_link: CodeEditorLink,
}

impl Component for MonacoWrapper {
    type Message = ();
    type Properties = MonacoProps;

    fn create(ctx: &Context<Self>) -> Self {
        let options = CodeEditorOptions::default()
            .with_language("rust".to_owned())
            .with_value(ctx.props().initial_value.clone())
            .with_builtin_theme(BuiltinTheme::VsDark)
            .with_automatic_layout(true);

        Self {
            options,
            editor_link: CodeEditorLink::new(),
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let editor_container_style = style!(
            r#"
            height: 100%;
            min-height: 500px;
            width: 100%;
            display: flex;
            flex-direction: column;

            & > div {
                flex-grow: 1;
                height: 100%;
            }
            "#
        )
        .unwrap();
        let on_editor_created = {
            let on_change = ctx.props().on_change.clone();
            let editor_link = self.editor_link.clone();

            Callback::from(move |_: CodeEditorLink| {
                editor_link.with_editor(|editor| {
                    if let Some(model) = editor.get_model() {
                        let model = model.clone();
                        let on_change = on_change.clone();

                        let model_clone = model.clone();
                        model.on_did_change_content(Box::new(move |_event| {
                            let content = model_clone.get_value();
                            on_change.emit(content);
                        }));
                    }
                });
            })
        };

        html! {
            <div class={editor_container_style}>
                <CodeEditor
                    classes={"full-height"}
                    options={Some(self.options.to_sys_options())}
                    on_editor_created={on_editor_created}
                    link={self.editor_link.clone()}
                />
            </div>
        }
    }
}
