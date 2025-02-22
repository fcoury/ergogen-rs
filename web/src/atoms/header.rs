use stylist::{style, Style};
use yew::prelude::*;

fn get_header_container_style() -> Style {
    style!(
        r#"
        width: 100%;
        height: 4em;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0 1rem 0 1rem;
        "#
    )
    .unwrap()
}

fn get_link_container_style() -> Style {
    style!(
        r#"
        a {
            color: white;
            text-decoration: none;
            display: inline-block;
            margin-right: 2em;
        }
        a:last-of-type {
            margin-right: 0;
        }
        "#
    )
    .unwrap()
}

#[function_component(Header)]
pub fn header() -> Html {
    html! {
        <div class={get_header_container_style()}>
            <div>
                <h2>{"Ergogen"}</h2>
            </div>

            <div class={get_link_container_style()}>
                <a href="https://docs.ergogen.xyz/" target="_blank">
                    {"Docs"}
                </a>
                <a href="https://discord.gg/nbKcAZB" target="_blank">
                    {"Discord"}
                </a>
            </div>
        </div>
    }
}
