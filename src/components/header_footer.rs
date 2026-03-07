#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use dioxus::prelude::*;
use std::time::Duration;

use crate::components::{Header, Footer};
use crate::backend::server_functions::get_store_settings;
use crate::backend::cache::use_stale_while_revalidate;

#[component]
pub fn HeaderFooter() -> Element {
    let settings_signal = use_stale_while_revalidate(
        "store_settings",
        || async { get_store_settings().await },
        Duration::from_secs(30),
    );

    let locked = settings_signal.read().as_ref().map(|s| s.lock_store).unwrap_or(false);
    let lock_comment = settings_signal.read().as_ref().and_then(|s| s.lock_comment.clone());

    if locked {
        return rsx! {
            div {
                class: "min-h-screen flex flex-col items-center justify-center bg-white px-4",
                img {
                    src: asset!("/assets/images/header.avif"),
                    class: "h-14 mb-6",
                    alt: "Store logo"
                }
                p {
                    class: "text-gray-800 text-lg font-medium mb-2",
                    "The store is currently unavailable."
                }
                if let Some(comment) = lock_comment {
                    p {
                        class: "text-gray-500 text-sm",
                        "{comment}"
                    }
                }
            }
        };
    }

    rsx! {
        Header {},
        div {
            style: "min-height: 75vh;",
            Outlet::<Route> {}
        },
        Footer {}
    }
}
