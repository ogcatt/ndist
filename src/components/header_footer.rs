#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use dioxus::prelude::*;
use chrono::{Local, Datelike};

use crate::components::{Header, Footer};

#[component]
pub fn HeaderFooter() -> Element {
    
    rsx! {
        Header {},
        // Main site body (below navbar) - Add some top padding if navbar is sticky/fixed
        // Note: The Outlet should ideally be wrapped in a main tag or similar semantic element
        // outside the Navbar component itself, but keeping it here as per original structure.
        div {
            style: "min-height: 75vh;",
            // Add styling here
            Outlet::<Route> {}
        },
        // Main site footer
        Footer {}
    }
}