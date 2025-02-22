use yew::prelude::*;

use crate::atoms::{JscadPreview, SvgPreview};

#[derive(Properties, PartialEq)]
pub struct FilePreviewProps {
    pub preview_key: String,
    pub preview_content: String,
}

#[function_component(FilePreview)]
pub fn file_preview(props: &FilePreviewProps) -> Html {
    let preview_ext = props.preview_key.split('.').last().unwrap_or("");

    match preview_ext {
        "svg" => html! {
            <SvgPreview svg={props.preview_content.clone()} />
        },
        "jscad" => html! {
            <JscadPreview preview_content={props.preview_content.clone()} />
        },
        _ => html! {},
    }
}
