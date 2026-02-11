#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn AdminUsers() -> Element {
    rsx! {
        div {
            "customers"
        }
    }
}