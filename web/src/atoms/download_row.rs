use stylist::{style, Style};
use wasm_bindgen::prelude::*;
use web_sys::Url;
use yew::prelude::*;

use super::Button;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn btoa(s: &str) -> String;
}

#[derive(Properties, PartialEq)]
pub struct DownloadRowProps {
    pub file_name: String,
    pub extension: String,
    pub content: String,
    #[prop_or_default]
    pub preview: Option<String>,
    pub set_preview: Callback<String>,
}

fn get_row_style() -> Style {
    style!(
        r#"
        display: flex;
        justify-content: space-between;
        margin-bottom: 0.5em;
        "#
    )
    .unwrap()
}

fn get_file_name_style() -> Style {
    style!(
        r#"
        overflow: hidden;
        text-overflow: ellipsis;
        "#
    )
    .unwrap()
}

fn get_buttons_style() -> Style {
    style!(
        r#"
        white-space: nowrap;
        "#
    )
    .unwrap()
}

fn get_button_style() -> Style {
    style!(
        r#"
        margin-right: 0.5em;
        "#
    )
    .unwrap()
}

#[function_component(DownloadRow)]
pub fn download_row(props: &DownloadRowProps) -> Html {
    let DownloadRowProps {
        file_name,
        extension,
        content,
        preview,
        set_preview,
    } = props;

    let download_url = {
        let array = js_sys::Array::new();
        array.push(&JsValue::from_str(&content));
        let blob_properties = web_sys::BlobPropertyBag::new();
        let _ = blob_properties.set_type("application/octet-stream");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&array, &blob_properties)
            .expect("Failed to create Blob");
        Url::create_object_url_with_blob(&blob).expect("Failed to create object URL")
    };

    let full_name = format!("{}.{}", file_name, extension);
    let download_name = full_name.clone();

    html! {
        <div class={get_row_style()}>
            <div class={get_file_name_style()}>
                {full_name}
            </div>
            <div class={get_buttons_style()}>
                if let Some(preview_path) = preview {
                    <Button
                        size="small"
                        class={get_button_style()}
                        onclick={
                            let set_preview = set_preview.clone();
                            let preview_path = preview_path.clone();
                            Callback::from(move |_| set_preview.emit(preview_path.clone()))
                        }
                    >
                        {"Preview"}
                    </Button>
                }
                <a
                    href={download_url}
                    download={download_name}
                    target="_blank"
                >
                    <Button size="small">
                        {"Download"}
                    </Button>
                </a>
            </div>
        </div>
    }
}
