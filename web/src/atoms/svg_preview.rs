use stylist::{style, Style};
use web_sys::console;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SvgPreviewProps {
    pub svg: String,
    #[prop_or("100%".to_string())]
    pub width: String,
    #[prop_or("100%".to_string())]
    pub height: String,
}

fn get_inverted_image_style() -> Style {
    style!(
        r#"
        filter: invert();
        -webkit-user-drag: none;
        -khtml-user-drag: none;
        -moz-user-drag: none;
        -o-user-drag: none;
        user-drag: none;
        "#
    )
    .unwrap()
}

fn get_container_style() -> Style {
    style!(
        r#"
        overflow: hidden;
        height: 100%;

        &:focus-visible {
            outline: none;
        }
        "#
    )
    .unwrap()
}

#[function_component(SvgPreview)]
pub fn svg_preview(props: &SvgPreviewProps) -> Html {
    let SvgPreviewProps { svg, width, height } = props;
    let image_ref = use_node_ref();

    console::log_1(&format!(" *** SVG: {}", svg).into());

    // Create data URL for SVG
    let src = format!("data:image/svg+xml;utf8,{}", urlencoding::encode(svg));

    html! {
        <div class={get_container_style()}>
            <img
                ref={image_ref}
                class={get_inverted_image_style()}
                width={width.clone()}
                height={height.clone()}
                src={src}
                alt="Ergogen SVG Output preview"
            />
        </div>
    }
}
