use crate::Route;
use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::backend::server_functions;

#[component]
pub fn Policies() -> Element {
    let policies = use_resource(move || async move {
        server_functions::get_policies().await
    });

    rsx! {
        document::Title { { format!("{} - {}", t!("brand"), t!("policies") ) } }

        div { class: "pt-3 pb-6 md:pt-9 md:pb-12 flex justify-center",
            div {
                class: "max-w-[1000px] w-full px-5",
                {match &*policies.read() {
                    Some(Ok((tos, _))) => rsx! {
                        p {
                            dangerous_inner_html: tos.to_string()
                        }
                    },
                    _ => rsx! {}
                }}
            }
        }
    }
}
