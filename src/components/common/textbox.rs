#![allow(non_snake_case)] // Allow non-snake_case identifiers

use dioxus::prelude::*;

#[derive(Default, PartialEq, Props, Clone)]
pub struct TextBoxProps {
    #[props(extends = input, extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    #[props(optional)]
    value: String,
    #[props(into)]
    input_type: Option<String>,

    #[props(optional)]
    onkeypress: EventHandler<KeyboardEvent>,
    #[props(optional)]
    onblur: EventHandler<FocusEvent>,
    #[props(optional)]
    oninput: EventHandler<FormEvent>,
    #[props(optional)]
    onmounted: EventHandler<Event<MountedData>>,

    #[props(into)]
    pub label: Option<String>,
    #[props(into)]
    pub placeholder: Option<String>,
    #[props(into)]
    pub prefix: Option<String>,
    #[props(into)]
    pub suffix: Option<String>,
    #[props(default = false)]
    pub is_number: bool,
    #[props(default = 0.01f64)]
    pub step: f64,

    #[props(default = false)]
    pub large: bool,

    #[props(default = false)]
    pub inside_label: bool,

    #[props(default = true)]
    pub optional: bool,
}

#[component]
pub fn CTextBox(props: TextBoxProps) -> Element {
    let mut is_focused = use_signal(|| false);
    let has_content = !props.value.is_empty();
    let should_float_label = is_focused() || has_content;

    let onkeypress = move |event| props.onkeypress.call(event);

    let onblur = move |event| {
        is_focused.set(false);
        props.onblur.call(event);
    };

    let oninput = move |event| props.oninput.call(event);
    let onmounted = move |event: Event<MountedData>| props.onmounted.call(event);

    let onfocus = move |_| {
        is_focused.set(true);
    };

    // Helper function to determine input border radius classes
    let get_input_radius_class = || {
        let has_prefix = props.prefix.is_some();
        let has_suffix = props.suffix.is_some();

        match (has_prefix, has_suffix) {
            (true, true) => "rounded-none",  // prefix and suffix
            (true, false) => "rounded-r-md", // only prefix
            (false, true) => "rounded-l-md", // only suffix
            (false, false) => "rounded-md",  // neither
        }
    };

    let get_button_style = || {
        if props.large {
            "pr-3 text-bbase pl-[16px] py-[11px]"
        } else {
            "text-sm py-1"
        }
    };

    rsx! {
        if props.inside_label {
            // Inside label mode - suffix/prefix not implemented for this mode
            div {
                class: "relative",
                input {
                    class: "w-full pr-3 pt-[18px] pl-[11px] pb-[5px] text-bbase border border-typical rounded-md appearance-none focus:outline-none focus:ring-2 focus:ring-blue-500 peer",
                    placeholder: "",
                    r#type: { if props.is_number { String::from("number") } else if let Some(typ) = props.input_type { typ } else { String::from("text") } },
                    step: if props.is_number { props.step.to_string() } else { "any".to_string() },
                    onfocus,
                    onmounted,
                    onkeypress,
                    onblur,
                    oninput,
                    value: props.value,
                    ..props.attributes,
                }
                if let Some(ref label) = props.label {
                    label {
                        class: format!(
                            "absolute transition-all duration-200 pointer-events-none text-gray-500 {} {}",
                            if should_float_label | !props.value.trim().is_empty() {
                                "text-xs left-[11px] top-1.5"
                            } else {
                                "text-bbase left-4 top-3"
                            },
                            if is_focused() { "text-blue-500" } else { "" }
                        ),
                        "{label}"
                        if !props.optional {
                            span {
                                class: "text-rose-500 ml-[2px]",
                                "*"
                            }
                        }
                    }
                }
            }
        } else {
            // Regular mode
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
                div {
                    class: "w-full flex border border-gray-300 rounded-md overflow-hidden",
                    if let Some(ref prefix) = props.prefix {
                        div {
                            class: "bg-gray-200 text-sm px-1.5 py-1 text-gray-700 border-r border-gray-300 rounded-l-md",
                            p {
                                "{prefix}"
                            }
                        }
                    },
                    // Main input element
                    input {
                        class: format!("w-full px-2.5 bg-white {} {}",
                            get_input_radius_class(),
                            get_button_style()
                        ),
                        r#type: { if props.is_number { String::from("number") } else if let Some(typ) = props.input_type { typ } else { String::from("text") } },
                        step: if props.is_number { props.step.to_string() } else { "any".to_string() },
                        placeholder: props.placeholder.as_deref().unwrap_or(""),
                        onmounted,
                        onkeypress,
                        onblur,
                        oninput,
                        value: props.value,
                        ..props.attributes,
                    }
                    if let Some(ref suffix) = props.suffix {
                        div {
                            class: "bg-gray-200 text-sm px-1.5 py-1 text-gray-700 border-l border-gray-300 rounded-r-md",
                            p {
                                "{suffix}"
                            }
                        }
                    },
                }
            }
        }
    }
}
