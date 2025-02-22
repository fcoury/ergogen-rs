use stylist::{style, Style};
use yew::prelude::*;

use crate::{atoms::DownloadRow, context::ergogen::use_ergogen_context};

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
    let context = use_ergogen_context();

    let mut downloads = Vec::new();
    if let Some(context) = context {
        if let Some(results) = context.results {
            // Raw config
            downloads.push(DownloadItem {
                file_name: "raw".to_string(),
                extension: "txt".to_string(),
                content: context.config_input,
                preview: None,
            });

            // Points data
            if let Some(points) = results.points {
                downloads.push(DownloadItem {
                    file_name: "points".to_string(),
                    extension: "yaml".to_string(),
                    content: serde_yaml::to_string(&points).unwrap_or_default(),
                    preview: None,
                });
            }

            // Outlines
            for (name, outline) in results.outlines {
                if name.starts_with('_') {
                    continue;
                }
                if let Some(dxf) = outline.get("dxf") {
                    downloads.push(DownloadItem {
                        file_name: name.clone(),
                        extension: "dxf".to_string(),
                        content: dxf.as_str().unwrap_or_default().to_string(),
                        preview: Some(format!("{}.svg", name)),
                    });
                }
                if let Some(svg) = outline.get("svg") {
                    downloads.push(DownloadItem {
                        file_name: name.clone(),
                        extension: "svg".to_string(),
                        content: svg.as_str().unwrap_or_default().to_string(),
                        preview: Some(format!("{}.svg", name)),
                    });
                }
            }

            // Cases
            for (name, case) in results.cases {
                if name.starts_with('_') {
                    continue;
                }
                if let Some(jscad) = case.get("jscad") {
                    downloads.push(DownloadItem {
                        file_name: name.clone(),
                        extension: "jscad".to_string(),
                        content: jscad.as_str().unwrap_or_default().to_string(),
                        preview: Some(format!("{}.jscad", name)),
                    });
                }
            }

            // PCBs
            for (name, pcb) in results.pcbs {
                if name.starts_with('_') {
                    continue;
                }
                downloads.push(DownloadItem {
                    file_name: name,
                    extension: "kicad_pcb".to_string(),
                    content: pcb,
                    preview: None,
                });
            }
        }
    }

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
