#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use dioxus::prelude::*;
use chrono::{Local, Datelike};

#[component]
pub fn CollectionsGrid() -> Element {
    let categories = vec![
        (asset!("/assets/images/categories/All.avif"), "All Products", "all"),
        (asset!("/assets/images/categories/PBIOs.avif"), "PBIOs", "pbios"),
        (asset!("/assets/images/categories/Nootropics.avif"), "Nootropics", "nootropics"),
        (asset!("/assets/images/categories/Peptides.avif"), "Peptides & Longevity", "peptides-and-longevity"),
        (asset!("/assets/images/categories/Natural.avif"), "Natural", "natural"),
        (asset!("/assets/images/categories/Physical.avif"), "SARMS & Physical", "sarms-and-physical")
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
