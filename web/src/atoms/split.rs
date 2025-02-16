use gloo::events::EventListener;
use stylist::{style, Style};
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement, MouseEvent};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SplitProps {
    pub direction: String,
    pub sizes: Vec<f64>,
    pub min_size: Option<f64>,
    pub gutter_size: Option<f64>,
    pub snap_offset: Option<f64>,
    #[prop_or_default]
    pub children: Children,
}

pub struct Split {
    is_dragging: bool,
    start_position: f64,
    start_sizes: Vec<f64>,
    drag_listener: Option<EventListener>,
    up_listener: Option<EventListener>,
    container_ref: NodeRef,
    current_sizes: Vec<f64>,
}

pub enum SplitMsg {
    StartDrag(f64, Vec<f64>),
    Drag(f64),
    EndDrag,
}

impl Component for Split {
    type Message = SplitMsg;
    type Properties = SplitProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            is_dragging: false,
            start_position: 0.0,
            start_sizes: vec![],
            drag_listener: None,
            up_listener: None,
            container_ref: NodeRef::default(),
            current_sizes: vec![],
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            SplitMsg::StartDrag(position, sizes) => {
                self.is_dragging = true;
                self.start_position = position;
                self.start_sizes = sizes;
                self.setup_drag_listeners(ctx);
                true
            }
            SplitMsg::Drag(current_position) => {
                if !self.is_dragging {
                    return false;
                }

                if let Some(container) = self.container_ref.cast::<HtmlElement>() {
                    let element: &web_sys::Element = container.as_ref();
                    let rect = element.get_bounding_client_rect();
                    let total_size = if ctx.props().direction == "horizontal" {
                        rect.width()
                    } else {
                        rect.height()
                    };

                    let delta = current_position - self.start_position;
                    let delta_percentage = (delta / total_size) * 100.0;

                    let mut new_sizes = self.start_sizes.clone();
                    new_sizes[0] += delta_percentage;
                    new_sizes[1] -= delta_percentage;

                    if let Some(min_size) = ctx.props().min_size {
                        let min_percentage = (min_size / total_size) * 100.0;
                        new_sizes[0] = new_sizes[0].max(min_percentage);
                        new_sizes[1] = new_sizes[1].max(min_percentage);
                    }

                    self.current_sizes = new_sizes;
                    true
                } else {
                    false
                }
            }
            SplitMsg::EndDrag => {
                self.is_dragging = false;
                self.cleanup_listeners();
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let direction = &ctx.props().direction;
        let gutter_size = ctx.props().gutter_size.unwrap_or(10.0);

        let flex_direction = if direction == "horizontal" {
            "row"
        } else {
            "column"
        };
        let container_style = style!(
            r#"
            display: flex;
            flex-direction: ${flex_direction};
            width: 100%;
            height: 100%;
            "#,
            flex_direction = flex_direction
        )
        .unwrap();

        let size_prop = if direction == "horizontal" {
            "width"
        } else {
            "height"
        };
        let cursor = if direction == "horizontal" {
            "col-resize"
        } else {
            "row-resize"
        };
        let gutter_style = Style::new(format!(
            r#"
            background-color: #878787;
            border-radius: 0.15rem;
            background-repeat: no-repeat;
            background-position: 50%;
            flex-shrink: 0;
            {size_prop}: {gutter_size}px;
            cursor: {cursor};
            "#,
            size_prop = size_prop,
            gutter_size = gutter_size,
            cursor = cursor
        ))
        .unwrap();

        let children: Vec<_> = ctx.props().children.iter().collect();
        let child_count = children.len();

        html! {
            <div class={container_style} ref={self.container_ref.clone()}>
                {
                    children.into_iter().enumerate().map(|(index, child)| {
                        let size = if index < self.current_sizes.len() {
                            self.current_sizes[index]
                        } else if index < ctx.props().sizes.len() {
                            ctx.props().sizes[index]
                        } else {
                            100.0 / (child_count as f64)
                        };

                        let child_style = style!(
                            r#"
                            flex-grow: 0;
                            flex-shrink: 0;
                            flex-basis: ${size}%;
                            "#,
                            size = size
                        ).unwrap();

                        let child_html = html! {
                            <div class={child_style.clone()}>
                                { child }
                            </div>
                        };

                        if index < child_count - 1 {
                            let link = ctx.link().clone();
                            let direction = direction.clone();
                            let mousedown = Callback::from(move |e: MouseEvent| {
                                e.prevent_default();
                                let position = if direction == "horizontal" {
                                    e.client_x() as f64
                                } else {
                                    e.client_y() as f64
                                };
                                link.send_message(SplitMsg::StartDrag(
                                    position,
                                    vec![size, 100.0 - size]
                                ));
                            });

                            html! {
                                <>
                                    { child_html }
                                    <div class={gutter_style.clone()} onmousedown={mousedown}></div>
                                </>
                            }
                        } else {
                            child_html
                        }
                    }).collect::<Html>()
                }
            </div>
        }
    }
}

impl Split {
    fn setup_drag_listeners(&mut self, ctx: &Context<Self>) {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let link = ctx.link().clone();
        let direction = ctx.props().direction.clone();

        let drag_listener = EventListener::new(&document, "mousemove", move |e| {
            let e = e.dyn_ref::<MouseEvent>().unwrap();
            let position = if direction == "horizontal" {
                e.client_x() as f64
            } else {
                e.client_y() as f64
            };
            link.send_message(SplitMsg::Drag(position));
        });

        let link = ctx.link().clone();
        let up_listener = EventListener::new(&document, "mouseup", move |_| {
            link.send_message(SplitMsg::EndDrag);
        });

        self.drag_listener = Some(drag_listener);
        self.up_listener = Some(up_listener);
    }

    fn cleanup_listeners(&mut self) {
        self.drag_listener = None;
        self.up_listener = None;
    }
}
