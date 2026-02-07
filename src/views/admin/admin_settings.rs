#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn AdminSettings() -> Element {
    rsx! {
        div {
            "settings"
        }
    }
}