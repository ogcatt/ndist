//use crate::Route;
use crate::backend::cache::use_cached_server;
use crate::backend::server_functions;
use dioxus::prelude::*;

#[component]
pub fn GroupPage(id: ReadOnlySignal<String>) -> Element {
    rsx! {
        "groups"
    }
}
