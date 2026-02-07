use dioxus::prelude::*;

#[derive(Clone, PartialEq, Props)]
pub struct SelectGroupProps {
    #[props(extends = select, extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    #[props(optional)]
    oninput: EventHandler<FormEvent>,
    #[props(into)]
    pub label: Option<String>,
    #[props(default = true)]
    pub optional: bool,
    #[props(default = false)]
    pub large: bool,
    #[props(default = false)]
    pub disabled: bool,
    children: Element,
}

impl std::default::Default for SelectGroupProps {
    fn default() -> Self {
        Self {
            attributes: Vec::<Attribute>::default(),
            oninput: EventHandler::<FormEvent>::default(),
            label: None,
            optional: true,
            large: false,
            disabled: false,
            children: rsx! {},
        }
    }
}

#[component]
pub fn CSelectGroup(mut props: SelectGroupProps) -> Element {
    let oninput = move |event| props.oninput.call(event);

    rsx! {
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

        if props.large {
            // Large select with custom styling similar to your example
            div {
                class: "relative flex items-center text-base-regular border border-typical bg-ui-bg-subtle rounded-md hover:bg-ui-bg-field-hover group",
                select {
                    class: format!(
                        "appearance-none flex-1 bg-transparent border-none px-4 py-2.5 transition-colors duration-150 outline-none{}",
                        if props.disabled { " cursor-not-allowed text-gray-400" } else { "" }
                    ),
                    disabled: props.disabled,
                    oninput,
                    ..props.attributes,
                    {props.children}
                }
                span {
                    class: "absolute flex pointer-events-none justify-end w-full pr-2 group-hover:animate-pulse",
                    svg {
                        width: "16",
                        height: "16",
                        view_box: "0 0 16 16",
                        fill: "none",
                        xmlns: "http://www.w3.org/2000/svg",
                        path {
                            d: "M4 6L8 10L12 6",
                            stroke: "currentColor",
                            stroke_width: "1.5",
                            stroke_linecap: "round",
                            stroke_linejoin: "round"
                        }
                    }
                }
            }
        } else {
            // Default small select styling
            div {
                select {
                    class: format!(
                        "text-sm w-full px-2.5 pt-[5px] pb-[4px] bg-white border border-gray-300 rounded-md{}",
                        if props.disabled { " cursor-not-allowed text-gray-400" } else { "" }
                    ),
                    disabled: props.disabled,
                    oninput,
                    ..props.attributes,
                    {props.children}
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Props)]
pub struct SelectPlaceholderProps {
    #[props(extends = option, extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
}

impl std::default::Default for SelectPlaceholderProps {
    fn default() -> Self {
        Self {
            attributes: Vec::<Attribute>::default(),
            children: rsx! {},
        }
    }
}

#[component]
pub fn SelectPlaceholder(mut props: SelectPlaceholderProps) -> Element {
    rsx! {
        option { disabled: true, selected: true, value: r#"{""}"#, {props.children} }
    }
}

#[derive(Default, Clone, PartialEq, Props)]
pub struct SelectLabelProps {
    #[props(extends = optgroup, extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
}

#[component]
pub fn SelectLabel(mut props: SelectLabelProps) -> Element {
    rsx! {
        optgroup { ..props.attributes }
    }
}

#[derive(Clone, PartialEq, Props)]
pub struct SelectItemProps {
    #[props(extends = option, extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    #[props(optional, default = None)]
    selected: Option<bool>,
    children: Element,
}

impl std::default::Default for SelectItemProps {
    fn default() -> Self {
        Self {
            attributes: Vec::<Attribute>::default(),
            selected: None,
            children: rsx! {},
        }
    }
}

#[component]
pub fn CSelectItem(mut props: SelectItemProps) -> Element {
    if let Some(selected) = props.selected {
        rsx! {
            option { selected, ..props.attributes, {props.children} }
        }
    } else {
        rsx! {
            option { ..props.attributes, {props.children} }
        }
    }
}
