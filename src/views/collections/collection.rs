use dioxus::prelude::*;
use dioxus_i18n::t;
use std::time::Duration;
use strum::IntoEnumIterator;

use crate::Route;
use crate::backend::cache::use_cached_server;
use crate::backend::front_entities::*;
use crate::backend::server_functions;
use crate::components::*;
use crate::utils::sort_products_by_priority;

#[component]
pub fn Collection(codename: ReadOnlySignal<String>) -> Element {
    let sort_options: [(&str, String); 7] = [
        ("latest", t!("latest")),
        ("oldest", t!("oldest")),
        ("alpha", t!("alpha")),
        ("alpharev", t!("alpharev")),
        ("lowest", t!("lowest")),
        ("highest", t!("highest")),
        ("featured", t!("featured")),
    ];

    let mut collection_enum = use_signal(|| None);
    let mut not_found = use_signal(|| false);
    let mut title = use_signal(|| String::new());
    let mut show_content = use_signal(|| false);

    let mut sort_option = use_signal(|| String::from("latest"));
    let mut search_query = use_signal(|| String::new());

    // New filter states
    let mut price_min = use_signal(|| 0.0);
    let mut price_max = use_signal(|| 1000.0);
    let mut price_range_min = use_signal(|| 0.0);
    let mut price_range_max = use_signal(|| 1000.0);
    let mut selected_phase: Signal<Option<ProductPhase>> = use_signal(|| None);
    let mut selected_form: Signal<Option<ProductForm>> = use_signal(|| None);
    let mut show_stocked_only = use_signal(|| false);

    // Mobile filter dropdown state
    let mut show_mobile_filters = use_signal(|| false);

    let mut original_products: Signal<Option<Vec<Product>>> = use_signal(|| None);

    let all_products_data = use_cached_server(
        "get_products", // Unique key for this server function
        || server_functions::get_products(),
        Duration::from_secs(180), // Cache
    );

    use_effect(move || {
        let current_codename = codename();

        // If appropriate set current enum
        if current_codename == "all" {
            collection_enum.set(None);
            not_found.set(false);
        } else if let Some(current_collection) = Category::from_key(&current_codename) {
            collection_enum.set(Some(current_collection));
            not_found.set(false);
        } else {
            collection_enum.set(None);
            not_found.set(true);
        }

        // Check if "All" category to make an exception and not use the enum
        title.set(if codename() == "all" {
            t!("all-products")
        } else if let Some(collection) = &*collection_enum.read() {
            collection.to_string()
        } else {
            String::new()
        });

        show_content.set(codename() == "all" || collection_enum.read().is_some());
    });

    // Calculate price range when products data changes
    use_effect(move || {
        if let Some(Ok(data)) = &*all_products_data.read() {
            let mut min_price = f64::MAX;
            let mut max_price = f64::MIN;

            for product in data {
                if let Some(variants) = &product.variants {
                    for variant in variants {
                        min_price = min_price.min(variant.price_standard_usd);
                        max_price = max_price.max(variant.price_standard_usd);
                    }
                }
            }

            if min_price != f64::MAX && max_price != f64::MIN {
                price_min.set(min_price);
                price_max.set(max_price);
                // Only set range if they haven't been modified
                if price_range_min() == 0.0 && price_range_max() == 1000.0 {
                    price_range_min.set(min_price);
                    price_range_max.set(max_price);
                }
            }
        }
    });

    use_effect(move || {
        if let Some(Ok(data)) = &*all_products_data.read() {
            let filtered_products = if codename() == "all" {
                data.clone()
            } else {
                data.iter()
                    .filter_map(|p| {
                        p.collections.as_ref().and_then(|colls| {
                            if colls.iter().any(|coll| coll == &codename()) {
                                Some(p.clone())
                            } else {
                                None
                            }
                        })
                    })
                    .collect::<Vec<_>>()
            };

            original_products.set(Some(filtered_products));
        }
    });

    // Changed from use_effect to use_memo to fix the read-write cycle warning
    let products = use_memo(move || {
        let mut search_q = search_query();
        search_q = search_q.trim().to_string();
        let current_sort = sort_option();
        let min_price = price_range_min();
        let max_price = price_range_max();
        let stocked_only = show_stocked_only();

        if let Some(Ok(data)) = &*all_products_data.read() {
            let filtered_products = if codename() == "all" {
                data.clone()
            } else {
                data.iter()
                    .filter_map(|p| {
                        p.collections.as_ref().and_then(|colls| {
                            if colls.iter().any(|coll| coll == &codename()) {
                                Some(p.clone())
                            } else {
                                None
                            }
                        })
                    })
                    .collect::<Vec<_>>()
            };

            // Apply search filter
            let mut search_filtered = if !search_q.is_empty() {
                filtered_products
                    .into_iter()
                    .filter(|product| {
                        // Check title
                        let title_match = product
                            .title
                            .to_lowercase()
                            .contains(&search_q.to_lowercase());

                        // Check CAS (Option type)
                        let cas_match = if let Some(cas) = &product.cas {
                            cas.contains(&search_q)
                        } else {
                            false
                        };

                        // Check names (Option type)
                        let names_match = if let Some(names) = &product.alternate_names {
                            names
                                .iter()
                                .any(|name| name.to_lowercase().contains(&search_q.to_lowercase()))
                        } else {
                            false
                        };

                        // Check pubchem (Option type) - exact match
                        let pubchem_match = if let Some(pubchem) = &product.pubchem_cid {
                            pubchem == &search_q
                        } else {
                            false
                        };

                        title_match || cas_match || names_match || pubchem_match
                    })
                    .collect::<Vec<_>>()
            } else {
                filtered_products
            };

            // Apply phase filter
            if let Some(phase) = selected_phase() {
                search_filtered = search_filtered
                    .into_iter()
                    .filter(|product| product.phase == phase)
                    .collect();
            }

            // Apply form filter
            if let Some(form) = selected_form() {
                search_filtered = search_filtered
                    .into_iter()
                    .filter(|product| product.product_form == form)
                    .collect();
            }

            // Apply price range filter
            search_filtered = search_filtered
                .into_iter()
                .filter(|product| {
                    if let Some(variants) = &product.variants {
                        variants.iter().any(|variant| {
                            variant.price_standard_usd >= min_price
                                && variant.price_standard_usd <= max_price
                        })
                    } else {
                        false
                    }
                })
                .collect();

            // Apply stock filter
            if stocked_only {
                search_filtered = search_filtered
                    .into_iter()
                    .filter(|product| {
                        if let Some(variants) = &product.variants {
                            variants.iter().any(|variant| {
                                if let Some(stock) = variant.calculated_stock_quantity {
                                    stock > 0
                                } else {
                                    false
                                }
                            })
                        } else {
                            false
                        }
                    })
                    .collect();
            }

            // Apply sorting
            match current_sort.as_str() {
                "latest" => {
                    search_filtered.sort_by(|a, b| {
                        // Compare NaiveDateTime directly (newer first)
                        b.created_at.cmp(&a.created_at)
                    });
                }
                "oldest" => {
                    search_filtered.sort_by(|a, b| {
                        // Compare NaiveDateTime directly (older first)
                        a.created_at.cmp(&b.created_at)
                    });
                }
                "alpha" => {
                    search_filtered.sort_by(|a, b| a.title.cmp(&b.title));
                }
                "alpharev" => {
                    search_filtered.sort_by(|a, b| b.title.cmp(&a.title));
                }
                "featured" => {
                    search_filtered = sort_products_by_priority(&search_filtered)
                        .into_iter()
                        .cloned()
                        .collect();
                }
                "lowest" => {
                    search_filtered.sort_by(|a, b| {
                        let min_price_a = a
                            .variants
                            .as_ref()
                            .and_then(|variants| {
                                variants
                                    .iter()
                                    .map(|v| v.price_standard_usd)
                                    .reduce(f64::min)
                            })
                            .unwrap_or(0.0);
                        let min_price_b = b
                            .variants
                            .as_ref()
                            .and_then(|variants| {
                                variants
                                    .iter()
                                    .map(|v| v.price_standard_usd)
                                    .reduce(f64::min)
                            })
                            .unwrap_or(0.0);
                        min_price_a
                            .partial_cmp(&min_price_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                "highest" => {
                    search_filtered.sort_by(|a, b| {
                        let max_price_a = a
                            .variants
                            .as_ref()
                            .and_then(|variants| {
                                variants
                                    .iter()
                                    .map(|v| v.price_standard_usd)
                                    .reduce(f64::max)
                            })
                            .unwrap_or(0.0);
                        let max_price_b = b
                            .variants
                            .as_ref()
                            .and_then(|variants| {
                                variants
                                    .iter()
                                    .map(|v| v.price_standard_usd)
                                    .reduce(f64::max)
                            })
                            .unwrap_or(0.0);
                        max_price_b
                            .partial_cmp(&max_price_a)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                _ => {
                    // Default to latest if unknown sort option
                    search_filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                }
            }

            Some(search_filtered)
        } else {
            None
        }
    });

    rsx! {
        div {
            class: "content-container py-6 md:py-12",

            {if show_content() {
                rsx! {
                    document::Title { { format!("{} - {}", t!("brand"), title ) } }

                    {
                        { rsx! {} }
                    }

                    h2 {
                        class: "mb-2 flex",
                        "{title}"
                    }

                    // Mobile filter toggle button
                    div {
                        class: "md:hidden my-3",
                        button {
                            class: "w-full py-2 px-4 border-typical border rounded-md flex items-center justify-between relative group",
                            onclick: move |_| show_mobile_filters.toggle(),
                            span { { t!("filters") } }
                            span {
                                class: if show_mobile_filters() {
                                    "flex pointer-events-none rotate-180 transition-transform duration-200 group-hover:animate-pulse"
                                } else {
                                    "flex pointer-events-none transition-transform duration-200 group-hover:animate-pulse"
                                },
                                svg {
                                    width: "16",
                                    height: "16",
                                    view_box: "0 0 16 16",
                                    fill: "none",
                                    xmlns: "http://www.w3.org/2000/svg",
                                    path {
                                        d: "M4 6L8 10L12 6",
                                        stroke: "currentColor",
                                        stroke_width: "1.5",
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round"
                                    }
                                }
                            }
                        }
                    }

                    div {
                        class: "md:flex mt-6",
                        div {
                            class: "w-full mb-6 md:mb-0 md:w-1/4 md:pr-6",
                            // Show filters on mobile only when toggled
                            class: if show_mobile_filters() { "block" } else { "hidden md:block" },

                            // Search
                            label {
                                class: "block mb-3 text-bbase text-ui-fg-subtle",
                                { t!("search") }
                            }

                            div {
                                class: "flex col-span-2 md:col-span-1 flex-col w-full mb-6",
                                CTextBox {
                                    value: "{search_query}",
                                    placeholder: t!("name-cas-etc"),
                                    optional: false,
                                    large: true,
                                    oninput: move |event: FormEvent| search_query.set(event.value())
                                }
                            },

                            // Sort By
                            label {
                                class: "block mb-3 text-bbase text-ui-fg-subtle",
                                { t!("sort-by") }
                            }
                            div {
                                class: "w-full mb-6",
                                CSelectGroup {
                                    large: true,
                                    oninput: move |event: FormEvent| {
                                        sort_option.set(event.value());
                                    },
                                    for (key, title) in sort_options {
                                        CSelectItem {
                                            value: "{key}",
                                            selected: key == &*sort_option.read(),
                                            key: "{key}", // Add a key for each item
                                            "{title}"
                                        }
                                    }
                                },
                            }

                            // Product Form Filter
                            label {
                                class: "block mb-3 text-bbase text-ui-fg-subtle",
                                "Product Form"
                            }
                            div {
                                class: "w-full mb-6",
                                CSelectGroup {
                                    large: true,
                                    oninput: move |event: FormEvent| {
                                        let value = event.value();
                                        if value == "all" {
                                            selected_form.set(None);
                                        } else {
                                            // Find the matching enum variant
                                            for form in ProductForm::iter() {
                                                if form.to_string() == value {
                                                    selected_form.set(Some(form));
                                                    break;
                                                }
                                            }
                                        }
                                    },
                                    CSelectItem {
                                        value: "all",
                                        selected: selected_form().is_none(),
                                        key: "{\"all\"}",
                                        "All Forms"
                                    }
                                    for form in ProductForm::iter() {
                                        CSelectItem {
                                            value: "{form.to_string()}",
                                            selected: selected_form().map_or(false, |f| f == form),
                                            key: "{form.to_string()}",
                                            "{form.to_string()}"
                                        }
                                    }
                                },
                            }

                            // Product Phase Filter
                            label {
                                class: "block mb-3 text-bbase text-ui-fg-subtle",
                                "Product Phase"
                            }
                            div {
                                class: "w-full mb-6",
                                CSelectGroup {
                                    large: true,
                                    oninput: move |event: FormEvent| {
                                        let value = event.value();
                                        if value == "all" {
                                            selected_phase.set(None);
                                        } else {
                                            // Find the matching enum variant
                                            for phase in ProductPhase::iter() {
                                                if phase.to_string() == value {
                                                    selected_phase.set(Some(phase));
                                                    break;
                                                }
                                            }
                                        }
                                    },
                                    CSelectItem {
                                        value: "all",
                                        selected: selected_phase().is_none(),
                                        key: "{\"all\"}",
                                        "All Phases"
                                    }
                                    for phase in ProductPhase::iter() {
                                        CSelectItem {
                                            value: "{phase.to_string()}",
                                            selected: selected_phase().map_or(false, |p| p == phase),
                                            key: "{phase.to_string()}",
                                            "{phase.to_string()}"
                                        }
                                    }
                                },
                            }

                            // Stock Filter Toggle
                            // Stock Filter Toggle
                            div {
                                class: "flex items-center mb-6",
                                label {
                                    class: "flex items-center cursor-pointer",
                                    input {
                                        r#type: "checkbox",
                                        checked: show_stocked_only(),
                                        class: "sr-only peer",  // Added 'peer' class here
                                        onchange: move |evt| show_stocked_only.set(evt.checked())
                                    }
                                    div {
                                        class: if show_stocked_only() {
                                            "relative w-11 h-6 bg-blue-600 focus:outline-none focus:ring-4 focus:ring-blue-300 rounded-full after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all after:translate-x-full"
                                        } else {
                                            "relative w-11 h-6 bg-gray-200 focus:outline-none focus:ring-4 focus:ring-blue-300 rounded-full after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all"
                                        }
                                    }
                                    span {
                                        class: "ml-3 text-sm font-medium text-ui-fg-subtle",
                                        "In Stock Only"
                                    }
                                }
                            }

                            // Price Range Filter

                            /*
                            {if let Some(products_list) = original_products() {
                                if products_list.len() > 2 {
                                    rsx! {
                                        label {
                                            class: "block mb-3 text-bbase text-ui-fg-subtle",
                                            "Price Range"
                                        }

                                        div {
                                            class: "mb-6",
                                            div {
                                                class: "flex justify-between text-sm text-ui-fg-muted mb-2",
                                                span { "${price_range_min():.2}" }
                                                span { "${price_range_max():.2}" }
                                            }

                                            div {
                                                class: "relative h-2",
                                                // Background track (non-interactive)
                                                div {
                                                    class: "absolute w-full h-2 bg-gray-200 rounded-lg",
                                                    style: "top: 0; z-index: 1;"
                                                }

                                                // Filled track between sliders
                                                div {
                                                    class: "absolute h-2 bg-blue-400 rounded-lg",
                                                    style: "left: {((price_range_min() - price_min()) / (price_max() - price_min()) * 100.0)}%; right: {100.0 - ((price_range_max() - price_min()) / (price_max() - price_min()) * 100.0)}%; z-index: 2;"
                                                }

                                                // Min range slider (thumb only, transparent track)
                                                input {
                                                    r#type: "range",
                                                    min: "{price_min()}",
                                                    max: "{price_max()}",
                                                    value: "{price_range_min()}",
                                                    step: "0.01",
                                                    class: "absolute w-full h-2 appearance-none cursor-pointer range-slider-min",
                                                    style: "z-index: 3; background: transparent;",
                                                    oninput: move |evt| {
                                                        let val: f64 = evt.value().parse().unwrap_or(price_min());
                                                        if val <= price_range_max() {
                                                            price_range_min.set(val);
                                                        } else {
                                                            // If trying to set min above max, set both to the same value
                                                            price_range_min.set(price_range_max());
                                                            price_range_max.set(price_range_max());
                                                        }
                                                    }
                                                }

                                                // Max range slider (thumb only, transparent track)
                                                input {
                                                    r#type: "range",
                                                    min: "{price_min()}",
                                                    max: "{price_max()}",
                                                    value: "{price_range_max()}",
                                                    step: "0.01",
                                                    class: "absolute w-full h-2 appearance-none cursor-pointer range-slider-max",
                                                    style: "z-index: 4; background: transparent;",
                                                    oninput: move |evt| {
                                                        let val: f64 = evt.value().parse().unwrap_or(price_max());
                                                        if val >= price_range_min() {
                                                            price_range_max.set(val);
                                                        } else {
                                                            // If trying to set max below min, set both to the same value
                                                            price_range_max.set(price_range_min());
                                                            price_range_min.set(price_range_min());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            } else {
                                rsx! {}
                            }}
                            */

                        }

                        div {
                            class: "w-full md:w-3/4",
                            div {
                                class: "flex justify-end mb-3",
                                p {
                                    class: "text-ui-fg-muted text-sm right-0",
                                    {
                                        format!("{}.",
                                            if let Some(products) = products() {
                                                if products.len() == 1 {
                                                    t!("num-result-found", num: 1)
                                                } else {
                                                    t!("num-results-found", num: products.len())
                                                }
                                            } else {
                                                t!("num-results-found", num: 0)
                                            }
                                        )
                                    },
                                }
                            }

                            {match products() {
                                Some(products) => rsx! {
                                    if products.is_empty() {
                                        /* IF NO RESULTS */
                                        p {
                                            class: "text-ui-fg-subtle",
                                            { t!("no-products-found-etc")},
                                            " ",
                                            Link {
                                                to: Route::Collections { },
                                                class: "a",
                                                { t!("view-all-collections") }
                                            },
                                            "."
                                        }
                                    } else {
                                        // IF PRODUCTS FOUND
                                        ul {
                                            class: "grid md:grid-cols-3 lg:grid-cols-4 grid-cols-2 gap-x-4 md:gap-x-6 md:gap-y-20 gap-y-10",
                                            // PRODUCT CARDS GO HERE INSIDE LI

                                            for product in products.iter() {
                                                li {
                                                    class: "",
                                                    ProductCard {
                                                        key: "{product.id}",
                                                        product: product.clone()
                                                    }
                                                }
                                            }

                                        }
                                    }
                                },
                                None => rsx! {
                                    if not_found() {
                                        /* IF NO RESULTS */
                                        p {
                                            class: "text-ui-fg-subtle",
                                            { t!("no-products-found-etc")},
                                            " ",
                                            Link {
                                                to: Route::Collections { },
                                                class: "a",
                                                { t!("view-all-collections") }
                                            },
                                            "."
                                        }
                                    } else {
                                        ul {
                                            class: "grid md:grid-cols-3 lg:grid-cols-4 grid-cols-2 gap-x-4 md:gap-x-6 md:gap-y-20 gap-y-10",
                                            for _num in 0..4 {
                                                li {
                                                    class: "",
                                                    ProductCard {
                                                        loading: true
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }}

                        }
                    }
                }
            } else if not_found() {
                rsx! {
                    div {
                        class: "text-center py-8",
                        p {
                            class: "text-lg",
                            "Could not find category '{codename()}'"
                        },
                        div {
                            class: "mt-3",
                            Link {
                                to: Route::Collections {},
                                class: "a",
                                { t!("view-all-collections") }
                            }
                        }
                    }
                }
            } else {
                rsx!()
            }}
        }
    }
}
