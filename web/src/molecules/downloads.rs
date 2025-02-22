use stylist::{style, Style};
use yew::prelude::*;

use crate::atoms::DownloadRow;

#[derive(Properties, PartialEq)]
pub struct DownloadsProps {
    pub set_preview: Callback<String>,
}

fn get_downloads_style() -> Style {
    style!(
        r#"
        display: flex;
        flex-direction: column;
        flex-grow: 1;
        padding: 1rem;
        "#
    )
    .unwrap()
}

#[derive(Clone)]
struct DownloadItem {
    file_name: String,
    extension: String,
    content: String,
    preview: Option<String>,
}

#[function_component(Downloads)]
pub fn downloads(props: &DownloadsProps) -> Html {
    let DownloadsProps { set_preview } = props;

    // TODO: Replace with actual results from ergogen processing
    let downloads = vec![
        DownloadItem {
            file_name: "demo".to_string(),
            extension: "svg".to_string(),
            content: "<svg width=\"100\" height=\"100\"><circle cx=\"50\" cy=\"50\" r=\"40\" stroke=\"black\" stroke-width=\"3\" fill=\"red\"/></svg>".to_string(),
            preview: Some("demo.svg".to_string()),
        },
        DownloadItem {
            file_name: "points".to_string(),
            extension: "yaml".to_string(),
            content: "points:\n  key1:\n    x: 0\n    y: 0".to_string(),
            preview: None,
        },
    ];

    html! {
        <div class={get_downloads_style()}>
            <h3>{"Downloads"}</h3>
            {
                downloads.into_iter().map(|download| {
                    html! {
                        <DownloadRow
                            file_name={download.file_name}
                            extension={download.extension}
                            content={download.content}
                            preview={download.preview}
                            set_preview={set_preview.clone()}
                        />
                    }
                }).collect::<Html>()
            }
        </div>
    }
}
