use dioxus::prelude::*;
use dioxus_core::AttributeValue;

#[derive(Default, Clone, PartialEq, Props)]
pub struct ToggleProps {
    #[props(extends = button, extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    #[props(optional)]
    checked: Option<bool>,

    #[props(optional)]
    onclick: EventHandler<MouseEvent>,
}

// Specifically stylised input type checkbox
// The input use the tailwind peer class, you can use at your advantage to style the children
// eg peer-disabled:font-mute will change children text-color when the input is disabled (Label component already does this by default)
#[component]
pub fn CToggle(mut props: ToggleProps) -> Element {

    let mut interior_sig = use_signal(|| props.checked.unwrap_or_default());

    let onclick = move |event| {
        interior_sig.toggle();
        props.onclick.call(event);
    };

    rsx! {
        button {
            class: "peer relative bg-input rounded-full focus:outline-hidden focus:ring-2 focus:ring-black focus:ring-offset-2 data-[state=active]:after:translate-x-full data-[state=active]:after:border-white after:content-[''] after:absolute after:bg-white after:border-input after:border after:rounded-full disabled:opacity-40   w-11 h-6 after:top-[2px] after:start-[2px] after:h-5 after:w-5   after:transition-all transition-colors duration-200   data-[state=active]:bg-blue-500 data-[state=inactive]:bg-gray-300",
            "data-state": match interior_sig() {
                true => AttributeValue::Text("active".to_string()),
                false => AttributeValue::Text("inactive".to_string()),
            },
            r#type: "button",
            onclick,
            ..props.attributes,
        }
    }
}