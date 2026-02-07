use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn NotFound(route: Vec<String>) -> Element {
    //let route_str = route.join("/");

    rsx! {

        document::Title { { format!("{} - 404: {}", t!("brand"), t!("page-not-found") ) } }

        div {

            div {
                class: "content-container py-6 md:py-12 text-center",

                h1 {
                    class: "mb-2 text-4xl font-bold italic",
                    "404"
                }
                h2 {
                    class: "text-xl mb-3",
                    { t!("page-not-found") }
                },
                p {
                    class: "text-gray-700",
                    { t!("page-not-exist", url: format!("{route:?}")) }
                },
                /*
                pre {
                    class: "text-gray-400 text-xs",
                    "Attempted to navigate to: {route:?}"
                }
                */

            }
        }
    }
}
