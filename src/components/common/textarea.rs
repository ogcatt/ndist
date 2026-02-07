#![allow(non_snake_case)] // Allow non-snake_case identifiers

use dioxus::prelude::*;

#[derive(Default, Clone, PartialEq, Props)]
pub struct TextAreaProps {
    #[props(extends = textarea, extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    #[props(optional)]
    value: String,

    #[props(optional)]
    oninput: EventHandler<FormEvent>,
    #[props(optional)]
    onmounted: EventHandler<Event<MountedData>>,

    #[props(into)]
    pub label: Option<String>,
    #[props(into)]
    pub placeholder: Option<String>,
    #[props(default = true)]
    pub optional: bool,
}

#[component]
pub fn CTextArea(props: TextAreaProps) -> Element {
    let oninput = move |event| props.oninput.call(event);
    let onmounted = move |event: Event<MountedData>| props.onmounted.call(event);

    rsx! {

        div {
            // Label (shown above text element)
            if let Some(label) = props.label {
                div {
                    class: "text-xs font-medium text-gray-700 pb-1",
                    "{label}"
                    if !props.optional {
                        span {
                            class: "text-rose-500 ml-[2px]",
                            "*"
                        }
                    }
                }
            }

            textarea {
                class: "text-sm w-full rounded-md px-2.5 py-1 border border-gray-300 bg-white",
                placeholder: props.placeholder.as_deref().unwrap_or(""),
                rows: "5",
                onmounted,
                oninput,
                value: props.value,
                ..props.attributes.clone(),
            }

        }
    }
}