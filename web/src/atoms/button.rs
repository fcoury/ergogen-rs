use stylist::{style, Style};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ButtonProps {
    #[prop_or_default]
    pub onclick: Callback<MouseEvent>,
    #[prop_or_default]
    pub children: Children,
    #[prop_or("large".to_string())]
    pub size: String,
    #[prop_or_default]
    pub class: Classes,
}

fn get_base_button_style() -> Style {
    style!(
        r#"
        display: inline-block;
        border: none;
        margin: 0;
        text-decoration: none;
        background-color: #28a745;
        border-radius: .25rem;
        transition: color .15s ease-in-out,
        background-color .15s ease-in-out,
        border-color .15s ease-in-out,
        box-shadow .15s ease-in-out;
        color: #ffffff;
        font-family: sans-serif;
        cursor: pointer;
        text-align: center;
        -webkit-appearance: none;
        -moz-appearance: none;

        &:hover {
            background-color: #218838;
            border-color: #1e7e34;
        }

        &:active {
            transform: scale(0.98);
            outline: 2px solid #fff;
            outline-offset: -5px;
        }
        "#
    )
    .unwrap()
}

fn get_size_style(size: &str) -> Style {
    match size {
        "small" | "sm" => style!(
            r#"
            padding: 0.4rem 0.8rem;
            font-size: 0.8rem;
            "#
        )
        .unwrap(),
        "medium" | "md" => style!(
            r#"
            padding: 0.7rem 1.4rem;
            font-size: 1rem;
            "#
        )
        .unwrap(),
        _ => style!(
            r#"
            padding: 1rem 2rem;
            font-size: 1.2rem;
            "#
        )
        .unwrap(),
    }
}

#[function_component(Button)]
pub fn button(props: &ButtonProps) -> Html {
    let ButtonProps {
        onclick,
        children,
        size,
        class,
    } = props;

    let mut classes = class.clone();
    classes.extend([get_base_button_style(), get_size_style(size)]);

    html! {
        <button {onclick} class={classes}>
            { for children.iter() }
        </button>
    }
}
