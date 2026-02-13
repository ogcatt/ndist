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
pub fn GroupPage(id: ReadOnlySignal<String>) -> Element {
    let sort_options: [(&str, String); 7] = [
        ("latest", t!("latest")),
        ("oldest", t!("oldest")),
        ("alpha", t!("alpha")),
        ("alpharev", t!("alpharev")),
        ("lowest", t!("lowest")),
        ("highest", t!("highest")),
        ("featured", t!("featured")),
    ];

    let mut group_info = use_signal(|| None::<(String, Option<String>)>); // (title, description)
    let mut not_found = use_signal(|| false);

    let mut sort_option = use_signal(|| String::from("latest"));
    let mut search_query = use_signal(|| String::new());

    // Filter states
    let mut price_min = use_signal(|| 0.0);
    let mut price_max = use_signal(|| 1000.0);
    let mut price_range_min = use_signal(|| 0.0);
    let mut price_range_max = use_signal(|| 1000.0);
    let mut selected_phase: Signal<Option<ProductPhase>> = use_signal(|| None);
    let mut selected_form: Signal<Option<ProductForm>> = use_signal(|| None);
    let mut show_stocked_only = use_signal(|| false);

    // Mobile filter dropdown state
    let mut show_mobile_filters = use_signal(|| false);

    // Get groups data using cache
    let groups_data = use_cached_server(
        "get_user_groups",
        || server_functions::get_user_groups(),
        Duration::from_secs(180),
    );

    // Get all products data using cache
    let all_products_data = use_cached_server(
        "get_products",
        || server_functions::get_products(),
        Duration::from_secs(180),
    );

    // Set group info when groups data loads
    use_effect(move || {
        let current_id = id();

        if let Some(Ok(groups)) = &*groups_data.read() {
            if let Some(group) = groups.iter().find(|g| g.id == current_id) {
                group_info.set(Some((group.name.clone(), group.description.clone())));
                not_found.set(false);
            } else {
                group_info.set(None);
                not_found.set(true);
            }
        }
    });

    // Calculate price range when products data changes
    use_effect(move || {
        if let Some(Ok(data)) = &*all_products_data.read() {
            let current_id = id();

            // Filter products by group access
            let group_products: Vec<_> = data
                .iter()
                .filter(|p| {
                    p.access_groups
                        .as_ref()
                        .map(|groups| groups.contains(&current_id))
                        .unwrap_or(false)
                })
                .collect();

            let mut min_price = f64::MAX;
            let mut max_price = f64::MIN;

            for product in group_products {
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

    // Filter and sort products
    let products = use_memo(move || {
        let mut search_q = search_query();
        search_q = search_q.trim().to_string();
        let current_sort = sort_option();
        let min_price = price_range_min();
        let max_price = price_range_max();
        let stocked_only = show_stocked_only();
        let current_id = id();

        if let Some(Ok(data)) = &*all_products_data.read() {
            // Filter products by group access
            let mut filtered_products: Vec<_> = data
                .iter()
                .filter(|p| {
                    p.access_groups
                        .as_ref()
                        .map(|groups| groups.contains(&current_id))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            // Apply search filter
            if !search_q.is_empty() {
                filtered_products = filtered_products
                    .into_iter()
                    .filter(|product| {
                        // Check title
                        let title_match = product
                            .title
                            .to_lowercase()
                            .contains(&search_q.to_lowercase());

                        // Check CAS
                        let cas_match = if let Some(cas) = &product.cas {
                            cas.contains(&search_q)
                        } else {
                            false
                        };

                        // Check names
                        let names_match = if let Some(names) = &product.alternate_names {
                            names
                                .iter()
                                .any(|name| name.to_lowercase().contains(&search_q.to_lowercase()))
                        } else {
                            false
                        };

                        // Check pubchem - exact match
                        let pubchem_match = if let Some(pubchem) = &product.pubchem_cid {
                            pubchem == &search_q
                        } else {
                            false
                        };

                        title_match || cas_match || names_match || pubchem_match
                    })
                    .collect::<Vec<_>>();
            }

            // Apply phase filter
            if let Some(phase) = selected_phase() {
                filtered_products = filtered_products
                    .into_iter()
                    .filter(|product| product.phase == phase)
                    .collect();
            }

            // Apply form filter
            if let Some(form) = selected_form() {
                filtered_products = filtered_products
                    .into_iter()
                    .filter(|product| product.product_form == form)
                    .collect();
            }

            // Apply price range filter
            filtered_products = filtered_products
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
                filtered_products = filtered_products
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
                    filtered_products.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                }
                "oldest" => {
                    filtered_products.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                }
                "alpha" => {
                    filtered_products.sort_by(|a, b| a.title.cmp(&b.title));
                }
                "alpharev" => {
                    filtered_products.sort_by(|a, b| b.title.cmp(&a.title));
                }
                "featured" => {
                    filtered_products = sort_products_by_priority(&filtered_products)
                        .into_iter()
                        .cloned()
                        .collect();
                }
                "lowest" => {
                    filtered_products.sort_by(|a, b| {
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
                    filtered_products.sort_by(|a, b| {
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
                    filtered_products.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                }
            }

            Some(filtered_products)
        } else {
            None
        }
    });

    rsx! {
        div {
            class: "content-container py-6 md:py-12",

            {if let Some((title, description)) = group_info() {
                rsx! {
                    document::Title { { format!("{} - {}", title, t!("brand")) } }

                    // Group title and description
                    div {
                        class: "mb-6 md:mb-8",
                        h2 {
                            class: "mb-2",
                            "{title}"
                        }
                        {if let Some(desc) = description {
                            rsx! {
                                p {
                                    class: "text-ui-fg-subtle",
                                    "{desc}"
                                }
                            }
                        } else {
                            rsx! {}
                        }}
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
                                            key: "{key}",
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
                            div {
                                class: "flex items-center mb-6",
                                label {
                                    class: "flex items-center cursor-pointer",
                                    input {
                                        r#type: "checkbox",
                                        checked: show_stocked_only(),
                                        class: "sr-only peer",
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
                                        p {
                                            class: "text-ui-fg-subtle",
                                            "No products found in this group."
                                        }
                                    } else {
                                        ul {
                                            class: "grid md:grid-cols-3 lg:grid-cols-4 grid-cols-2 gap-x-4 md:gap-x-6 md:gap-y-20 gap-y-10",

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
                            "Group not found or you don't have access to this group."
                        }
                    }
                }
            } else {
                rsx! {
                    // Loading state
                    div {
                        class: "text-center py-8",
                        p {
                            class: "text-ui-fg-subtle",
                            "Loading..."
                        }
                    }
                }
            }}
        }
    }
}
