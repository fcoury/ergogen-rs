use stylist::{style, Style};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct JscadPreviewProps {
    pub preview_content: String,
}

fn get_container_style() -> Style {
    style!(
        r#"
        width: 100%;
        height: 400px;
        display: flex;
        align-items: center;
        justify-content: center;
        background-color: #2d2d2d;
        color: #ffffff;
        "#
    )
    .unwrap()
}

#[function_component(JscadPreview)]
pub fn jscad_preview(_props: &JscadPreviewProps) -> Html {
    html! {
        <div class={get_container_style()}>
            // TODO: Implement actual JSCAD preview
            {"3D Preview (Coming Soon)"}
        </div>
    }
}
