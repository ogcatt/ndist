use crate::Route;
use crate::components::CollectionsGrid;
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn Collections() -> Element {
    rsx! {
        document::Title { { format!("{} - {}", t!("brand"), t!("collections") ) } }
        div {
            class: "pt-8 md:pt-12",
            div {
                class: "content-container py-0 sm:pt-0 sm:pb-12 px-6",
                CollectionsGrid {}
            }
        }
    }
}
