#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::Route;
use chrono::{Datelike, Local};
use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::components::{SmilesViewer};
use crate::backend::server_functions::get_or_create_basket;
use crate::backend::front_entities::Product;

#[component]
pub fn SearchResults(
    products: Vec<Product>,
    search_query: String,
    on_product_click: EventHandler<()>,
) -> Element {
    if search_query.is_empty() || products.is_empty() {
        return rsx! {
            div {
                class: "absolute top-36 z-50 left-0 w-full bg-white shadow-lg border-t border-gray-200 h-48 flex items-center justify-center text-gray-500 animate-slide-down",
                if search_query.trim().is_empty() {
                    { t!("start-typing-to-search") }
                } else {
                    { t!("no-products-found") }
                }
            }
        };
    }

    let total_results = products.len();
    let display_products = products.iter().take(5).collect::<Vec<_>>();
    let has_more_results = total_results > 5;

    // Helper function to format price without unnecessary .00
    let format_price = |price: f64| -> String {
        let formatted = format!("${:.2}", price);
        if formatted.ends_with(".00") {
            formatted.trim_end_matches(".00").to_string()
        } else {
            formatted
        }
    };

    rsx! {
        div {
            class: "absolute top-36 left-0 w-full bg-white shadow-lg border-t border-gray-200 max-h-96 overflow-y-auto animate-slide-down z-50",

            div {
                class: "p-4",

                for product in display_products { // Limit to 5 results
                    Link {
                        to: Route::ProductPage { handle: product.handle.clone() },
                        class: "flex items-center gap-4 p-3 hover:bg-gray-50 rounded-lg transition-colors duration-150 border-b border-gray-100 last:border-b-0",
                        onclick: move |_| {
                            on_product_click.call(()); // Call the callback when clicked
                        },

                        // Small image/SMILES viewer
                        div {
                            class: "w-16 h-16 bg-gray-100 rounded-lg overflow-hidden flex-shrink-0 border border-gray-200",

                            // Get thumbnail from variants (same logic as ProductCard)
                            if let Some(variants) = &product.variants {
                                if let Some(thumbnail_url) = {
                                    if let Some(default_variant_id) = &product.default_variant_id {
                                        variants
                                            .iter()
                                            .find(|v| &v.id == default_variant_id)
                                            .and_then(|v| v.thumbnail_url.as_ref())
                                            .or_else(|| variants.first().and_then(|v| v.thumbnail_url.as_ref()))
                                    } else {
                                        variants.first().and_then(|v| v.thumbnail_url.as_ref())
                                    }
                                } {
                                    img {
                                        src: "{thumbnail_url}",
                                        alt: "{product.title}",
                                        class: "w-full h-full object-cover object-center",
                                        loading: "lazy"
                                    }
                                } else if let Some(smiles) = &product.smiles {
                                    div {
                                        class: "w-full h-full flex items-center justify-center",
                                        SmilesViewer {
                                            smiles: smiles.clone()
                                        }
                                    }
                                } else {
                                    div {
                                        class: "w-full h-full flex items-center justify-center text-gray-400 text-xs",
                                        "No Image"
                                    }
                                }
                            } else if let Some(smiles) = &product.smiles {
                                div {
                                    class: "w-full h-full flex items-center justify-center",
                                    SmilesViewer {
                                        smiles: smiles.clone()
                                    }
                                }
                            } else {
                                div {
                                    class: "w-full h-full flex items-center justify-center text-gray-400 text-xs",
                                    "No Image"
                                }
                            }
                        }

                        // Product info
                        div {
                            class: "flex-1 min-w-0", // min-w-0 for text truncation

                            // Name - Form, variants
                            div {
                                class: "font-medium text-gray-900 truncate",
                                // Format title with product form
                                {
                                    let display_title = if product.title.contains("(") {
                                        product.title.clone()
                                    } else {
                                        format!(
                                            "{} ({})",
                                            product.title,
                                            product.product_form.to_frontend_string()
                                        )
                                    };

                                    // Add variant names
                                    if let Some(variants) = &product.variants {
                                        if !variants.is_empty() {
                                            let variant_names: Vec<String> = variants
                                                .iter()
                                                .map(|v| v.variant_name.clone())
                                                .collect();
                                            format!("{}, {}", display_title, variant_names.join("/"))
                                        } else {
                                            display_title
                                        }
                                    } else {
                                        display_title
                                    }
                                }
                            }

                            // Subtitle
                            if let Some(subtitle) = &product.subtitle {
                                div {
                                    class: "text-sm text-gray-500 truncate mt-1",
                                    "{subtitle}"
                                }
                            }
                        }

                        // Price
                        div {
                            class: "text-right flex-shrink-0 ml-4",

                            div {
                                class: "font-medium text-gray-900",
                                // Calculate price range from variants (same logic as ProductCard)
                                {
                                    if let Some(variants) = &product.variants {
                                        if variants.is_empty() {
                                            "N/A".to_string()
                                        } else {
                                            let prices: Vec<f64> = variants.iter().map(|v| v.price_standard_usd).collect();
                                            let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                                            let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

                                            if (min_price - max_price).abs() < 0.01 {
                                                // Same price
                                                format_price(min_price)
                                            } else {
                                                format!("{} — {}", format_price(min_price), format_price(max_price))
                                            }
                                        }
                                    } else {
                                        "N/A".to_string()
                                    }
                                }
                            }
                        }
                    }
                }

                // Show "Show All Results" link if there are more than 5 results

                /*
                if has_more_results {
                    div {
                        class: "border-t border-gray-200 mt-2 pt-3",
                        Link {
                            to: Route::SearchPage { query: search_query.clone() }, // Adjust this route to your search page
                            class: "flex items-center justify-center w-full py-2 text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded-lg transition-colors duration-150 text-sm font-medium",
                            "Show All Results ({total_results})"
                        }
                    }
                }
                */
            }
        }
    }
}
