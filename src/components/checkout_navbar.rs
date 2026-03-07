#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use dioxus::prelude::*;
use dioxus_i18n::{prelude::*, t};
use chrono::{Local, Datelike};

use crate::utils::GLOBAL_CART;
use crate::backend::server_functions::get_store_settings;
use crate::backend::cache::use_stale_while_revalidate;
use std::time::Duration;

// Define the navbar CSS asset
const NAVBAR_CSS: Asset = asset!("/assets/styling/navbar.css");

#[component]
pub fn CheckoutHeader() -> Element {

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

    // Updated date checks to match svelte version
    let now = Local::now();
    let is_christmas = now.month() == 12 && (now.day() >= 20 && now.day() <= 31);
    let is_new_year = now.month() == 1 && now.day() <= 3;
    let is_ind_day = now.month() == 7 && now.day() == 4;

    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS },

        div {
            class: "sticky top-0 inset-x-0 z-50", // Removed `group`
            header {
                class: "relative h-18 mx-auto border-b duration-200 bg-white border-ui-border-base",
                nav {
                    class: "content-container txt-xsmall-plus text-ui-fg-subtle flex items-center justify-between w-full h-full text-smm md:text-sm",
                    // Left section: Mobile toggle and desktop menus
                    div {
                        class: "flex-1 basis-0 h-full flex items-center",
                        
                        // Desktop navigation - fixed container structure
                        div {
                            class: "h-full hidden md:flex", // Changed to flex to ensure horizontal layout
                            // Products dropdown
                            // 2. Added `group` here
                            div {
                                class: "h-full relative group",
                                Link {
                                    to: Route::Cart { },
                                    class: "h-full",
                                    // 5. Ensured transition classes are present (they were)
                                    button {
                                        class: "mr-5 text-sm relative text-nowrap h-full flex items-center transition-all ease-out duration-200 hover:text-ui-fg-base",
                                        { t!("back-to-cart") }
                                        
                                    }
                                },
                                
                            },
                        }
                    },
                    // Center section: Logo
                    div {
                        class: "flex items-center h-full",
                        Link {
                            to: Route::Home {},
                            class: "pl-2 fadeyy", // fadeyy might handle opacity/filter, not necessarily background
                            if is_ind_day {
                                img {
                                    src: asset!("/assets/images/header-4th.avif"),
                                    alt: t!("brand").to_uppercase(),
                                    title: t!("independence-day"),
                                    class: "h-8 md:h-12"
                                }
                            } else if is_christmas {
                                img {
                                    src: asset!("/assets/images/header-christmas.avif"),
                                    alt: t!("brand").to_uppercase(),
                                    title: t!("merry-christmas"),
                                    class: "h-8 md:h-12"
                                }
                            } else if is_new_year {
                                img {
                                    src: asset!("/assets/images/header.avif"), // Placeholder, replace if you have a new year image
                                    alt: t!("brand").to_uppercase(),
                                    title: t!("new-year"),
                                    class: "h-10 md:h-11 lg:h-12"
                                }
                            } else {
                                img {
                                    src: asset!("/assets/images/header.avif"),
                                    alt: t!("brand").to_uppercase(),
                                    class: "h-10 md:h-11 lg:h-12"
                                }
                            }
                        }
                    },
                    div {
                        class: "flex items-center gap-x-6 h-full flex-1 basis-0 justify-end",
                        div {
                            // Keep this
                        }
                    }
                }
            },
        },

        div {
            Outlet::<Route> {}
        }
    }
}