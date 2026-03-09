#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use dioxus::prelude::*;
use chrono::{Local, Datelike};

#[component]
pub fn CollectionsGrid() -> Element {
    let categories = vec![
        (asset!("/assets/images/categories/All.avif"), "All Products", "all"),
        (asset!("/assets/images/categories/Chondrogenic.avif"), "Chondrogenic", "chondrogenic"),
        (asset!("/assets/images/categories/Osteogenic.avif"), "Osteogenic", "osteogenic"),
        (asset!("/assets/images/categories/Protective.avif"), "Protective", "protective"),
        (asset!("/assets/images/categories/Nootropic.avif"), "Nootropic", "nootropic"),
        (asset!("/assets/images/categories/Other.avif"), "Other", "other"),
    ]; // update other in footer if edited
    
    rsx! {
        div { class: "flex justify-center",
            div { class: "grid grid-cols-2 sm:grid-cols-3 gap-x-2 gap-y-2",
                for category in categories {
                    Link { 
                        to: Route::Collection { codename: category.2.to_string() },
                        div { class: "overflow-hidden rounded-md",
                            img {
                                class: "cimg w-64",
                                src: category.0,
                                title: "Visit {category.1}",
                                alt: "{category.1}"
                            }
                        }
                    }
                }
            }
        }
    }
}
