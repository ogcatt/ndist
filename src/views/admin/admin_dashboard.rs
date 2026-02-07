use dioxus::prelude::*;
use crate::backend::server_functions::*;

#[component]
pub fn Dashboard() -> Element {

    rsx! {
        p { "admin dashboard (put useful utilities in this default section)" }
    }
}