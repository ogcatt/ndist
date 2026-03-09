#![allow(non_snake_case)] // Allow non-snake_case identifiers

use dioxus::prelude::*;
use std::time::Duration;

use crate::backend::server_functions;
use crate::Route;
use crate::backend::front_entities::*;
use crate::backend::cache::use_cached_server;

// Helper function to get the thumbnail URL from a product
fn get_product_thumbnail_url(product: &Product) -> Option<&String> {
    if let Some(variants) = &product.variants {
        if let Some(default_variant_id) = &product.default_variant_id {
            // Find the default variant first
            variants.iter()
                .find(|v| &v.id == default_variant_id)
                .and_then(|v| v.thumbnail_url.as_ref())
                .or_else(|| {
                    // If default variant not found or has no thumbnail, use first variant
                    variants.first().and_then(|v| v.thumbnail_url.as_ref())
                })
        } else {
            // No default variant set, use first available variant
            variants.first().and_then(|v| v.thumbnail_url.as_ref())
        }
    } else {
        None
    }
}

#[component]
pub fn AdminProducts() -> Element {
    // Use our caching hook
    let products_req = use_cached_server(
        "product_count", // Unique key for this server function
        || server_functions::admin_get_products(true /* convert markdown */),
        Duration::from_secs(15), // Cache for 15 seconds
    );

    use_effect(move || {
        // When we read count, it becomes a dependency of the effect
       // let current_count = products_req();
        // Whenever count changes, the effect will rerun
        println!("{:#?}", products_req);
    });

    rsx! {
        div {
            div {
                class: "border rounded-md border-gray-200 w-full mb-4 flex justify-between items-center p-2 pl-4",
                div {
                    class: "text-lg font-medium",
                    "Products"
                }
                Link {
                    to: Route::AdminCreateProduct {},
                    button {
                        class: "text-sm bg-zinc-600 px-3 py-2 text-white rounded hover:bg-zinc-500 transition-colors",
                        "Create Product"
                    }
                }
            }
            
            div {
                class: "w-full",
                {match &*products_req.read() {
                    Some(Ok(products)) => rsx! {
                        if products.len() == 0 {
                            div {
                                class: "mt-12 text-center",
                                "No products created yet"
                            }
                        } else {
                            for product in products.iter() {
                                {
                                    let thumbnail_url = get_product_thumbnail_url(product);
                                    rsx! {
                                        div {
                                            class: "bg-white w-full min-h-12 border rounded-md border-gray-200 p-4 mb-4",
                                            div {
                                                class: "flex items-center gap-4",
                                                // Thumbnail image
                                                if let Some(url) = thumbnail_url {
                                                    div {
                                                        class: "relative w-16 h-16 group cursor-pointer",
                                                        // Background image
                                                        img {
                                                            class: "w-16 h-16 object-cover rounded border group-hover:opacity-50 transition-opacity duration-200",
                                                            src: "{url}"
                                                        }
                                                        // Overlay with link icon (hidden by default, shown on hover)
                                                        a {
                                                            href: format!("/products/{}", product.handle),
                                                            target: "_blank",
                                                            class: "absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded",
                                                            img {
                                                                class: "w-6 h-6",
                                                                src: asset!("/assets/icons/link-outline.svg")
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    div {
                                                        class: "w-16 h-16 bg-gray-200 rounded border flex items-center justify-center",
                                                        span {
                                                            class: "text-gray-500 text-xs",
                                                            "No Image"
                                                        }
                                                    }
                                                }
                                                
                                                div {
                                                    class: "flex-1",
                                                    h3 {
                                                        class: "text-lg mb-2",
                                                        "{product.title}"
                                                    }
                                                    div {
                                                        class: "flex gap-6 text-sm text-gray-600",
                                                        div {
                                                            class: "flex items-center gap-1",
                                                            span {
                                                                class: "font-medium text-gray-700",
                                                                "Form:"
                                                            }
                                                            span {
                                                                class: "px-2 py-1 bg-blue-100 text-blue-800 rounded text-xs",
                                                                "{product.product_form}"
                                                            }
                                                        }
                                                        div {
                                                            class: "flex items-center gap-1",
                                                            span {
                                                                class: "font-medium text-gray-700",
                                                                "Visibility:"
                                                            }
                                                            span {
                                                                class: match product.visibility {
                                                                    ProductVisibility::Public => "px-2 py-1 bg-green-100 text-green-800 rounded text-xs",
                                                                    ProductVisibility::Private => "px-2 py-1 bg-red-100 text-red-800 rounded text-xs",
                                                                    ProductVisibility::Unlisted => "px-2 py-1 bg-yellow-100 text-yellow-800 rounded text-xs",
                                                                },
                                                                "{product.visibility}"
                                                            }
                                                        }
                                                    }
                                                }
                                                
                                                // Edit link on the right side
                                                Link {
                                                    to: Route::AdminEditProduct { id: format!("{}", product.id) },
                                                    title: "Edit product",
                                                    class: "flex items-center justify-center w-8 h-8 rounded hover:bg-gray-100 transition-colors",
                                                    img {
                                                        class: "w-5 h-5",
                                                        src: asset!("/assets/icons/create-outline.svg")
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(_)) => rsx! {
                        p { "Error loading products" }
                    },
                    None => rsx! {
                        p { "Loading products..." }
                    }
                }}
            }
        }
    }
}