use crate::Route;
use dioxus::prelude::*;
use dioxus_i18n::t;
use std::time::Duration;
use tracing;
use wasm_bindgen::JsCast;

use crate::backend::cache::*;
use crate::backend::front_entities::*;
use crate::backend::server_functions;
use crate::components::{Meta, ProductCard, SmilesViewer};
use crate::utils::GLOBAL_CART;

#[component]
pub fn ProductPage(handle: ReadOnlySignal<String>) -> Element {
    use_effect(move || {
        let current_handle = handle();
        tracing::info!("Current product handle: {}", current_handle.clone());
    });

    let mut current_product = use_signal(|| None::<Product>);
    let mut product_not_found = use_signal(|| false);
    let mut current_variant = use_signal(|| 0usize);
    let mut current_stock_quantity = use_signal(|| 0i32);
    let mut combined_variant_stock_quantity = use_signal(|| 0i32);
    let mut quantity = use_signal(|| 1i32);
    let mut preview_url = use_signal(|| String::new());
    let mut all_products = use_signal(|| Vec::<Product>::new());
    let mut relevant_products = use_signal(|| Vec::<Product>::new());
    let mut scroll_position = use_signal(|| 0f64);
    let mut sub_name = use_signal(|| String::new());
    let mut sub_email = use_signal(|| String::new());
    let mut sub_complete = use_signal(|| String::new());
    let mut mail_join_open = use_signal(|| false);
    let mut close_stock_updates = use_signal(|| false);
    let mut image_zoomed = use_signal(|| false);
    let mut mouse_x = use_signal(|| 50.0);
    let mut mouse_y = use_signal(|| 50.0);

    // Add-to-cart UI state
    let mut add_btn_text = use_signal(|| "Add To Cart".to_string());
    let mut add_btn_added = use_signal(|| false);
    let mut error_line = use_signal(|| String::new());
    let mut adding = use_signal(|| false);
    let max_per_item = 12i32;

    let mut update_product_data = move |product: Product| {
        current_product.set(Some(product.clone()));
        product_not_found.set(false);
        current_variant.set(0);

        // Update the preview_url based on what's available
        if let Some(ref variants) = product.variants {
            if let Some(variant) = variants.first() {
                if let Some(ref thumbnail) = variant.thumbnail_url {
                    preview_url.set(thumbnail.clone());
                } else if product.smiles.is_some() && product.enable_render_if_smiles {
                    preview_url.set("smiles".to_string());
                } else {
                    preview_url.set(String::new());
                }
            }
        } else {
            preview_url.set(String::new());
        }
    };

    // 1. Try to get cached public products for related products section
    let cached_products: Option<Vec<Product>> = use_existing_cached_server("get_products");

    // 2. Load related products from cache if available
    use_effect(move || {
        if let Some(ref products) = cached_products {
            all_products.set(products.clone());

            let current_handle = handle();
            let rel_products: Vec<Product> = products
                .iter()
                .filter(|p| p.handle != current_handle)
                .take(8)
                .cloned()
                .collect();
            relevant_products.set(rel_products);
        }
    });

    // 3. Fetch the specific product by handle with intelligent caching
    // Uses: 1) Individual product cache, 2) Products list cache, 3) Fresh fetch
    let handle_clone = handle();
    let (product_signal, mut refresh_trigger) = use_product_by_handle(
        handle_clone,
        move || {
            let handle_value = handle();
            async move {
                server_functions::get_product_by_handle(handle_value).await
            }
        },
        Duration::from_secs(300), // Cache for 5 minutes
    );

    // 4. Update current product when the signal updates
    use_effect(move || {
        if let Some(product) = product_signal.read().as_ref() {
            update_product_data(product.clone());
        }
    });

    // 4b. Track loading state and not found
    let mut loading = use_signal(|| true);
    use_effect(move || {
        // Subscribe to product_signal changes
        let product_opt = product_signal.read();
        
        if product_opt.is_some() {
            // Got a product - not loading, not "not found"
            loading.set(false);
            product_not_found.set(false);
        }
        // If None, we don't know yet if it's initial load or actual not found
    });
    
    // Use a timeout to determine when loading is "done"
    // When the product page first loads, we wait a bit then check
    let mut checked_not_found = use_signal(|| false);
    use_effect(move || {
        // Delay the check to allow fetch to complete
        let checked = *checked_not_found.read();
        if !checked && *loading.read() {
            // Small delay then check if still loading (means no product came through)
            // This is a simple heuristic
        }
    });
    
    // Simpler: After a short delay, if still no product, show not found
    // We'll use a deferred check
    let mut has_checked = use_signal(|| false);
    use_effect(move || {
        // This runs once on mount
        let checked = *has_checked.read();
        if !checked {
            has_checked.set(true);
            let mut product_signal = product_signal.clone();
            let mut product_not_found = product_not_found.clone();
            let mut loading = loading.clone();
            
            spawn(async move {
                // Wait for the async fetch to complete using gloo timer (web compatible)
                gloo_timers::future::sleep(std::time::Duration::from_millis(500)).await;
                
                // Check if we got a product
                if product_signal.read().is_none() {
                    // After delay, still no product - it's not found (or no access)
                    product_not_found.set(true);
                }
                loading.set(false);
            });
        }
    });

    // 5. Optionally fetch all public products for related products (in background)
    let public_products_resource = use_resource(move || async move {
        server_functions::get_products().await
    });

    // 6. Update related products when public products load
    use_effect(move || {
        if let Some(Ok(products)) = public_products_resource.read().as_ref() {
            all_products.set(products.clone());

            let current_handle = handle();
            let rel_products: Vec<Product> = products
                .iter()
                .filter(|p| p.handle != current_handle)
                .take(8)
                .cloned()
                .collect();
            relevant_products.set(rel_products);
        }
    });

    // Compute stock for currently selected variant
    use_effect(move || {
        current_stock_quantity.set(if let Some(product) = current_product() {
            if let Some(ref variants) = product.variants {
                let current_variant_idx = *current_variant.read();
                if let Some(current_variant) = variants.get(current_variant_idx) {
                    if let Some(qty) = current_variant.calculated_stock_quantity {
                        if product.back_order || product.pre_order {
                            999
                        } else {
                            qty
                        }
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        })
    });

    // Helper: get current variant live stock
    let calc_current_variant_stock = {
        let current_product = current_product.clone();
        let current_variant = current_variant.clone();
        move || -> i32 {
            if let Some(product) = current_product() {
                if let Some(ref variants) = product.variants {
                    if let Some(v) = variants.get(*current_variant.read()) {
                        if product.back_order || product.pre_order {
                            return 999;
                        } else {
                            return v.calculated_stock_quantity.unwrap_or(0);
                        }
                    }
                }
            }
            0
        }
    };

    use_effect({
        // Only depend on current_product/current_variant changes.
        // Do NOT read quantity here so the effect won't subscribe to it.
        let mut quantity = quantity.clone();
        let current_product = current_product.clone();
        let current_variant = current_variant.clone();
        move || {
            // Recompute stock
            let stock = {
                if let Some(product) = current_product() {
                    if let Some(ref variants) = product.variants {
                        if let Some(v) = variants.get(*current_variant.read()) {
                            if product.back_order || product.pre_order {
                                999
                            } else {
                                v.calculated_stock_quantity.unwrap_or(0)
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else {
                    0
                }
            };

            // Compute a clamped value from the current quantity value,
            // but only set if it would actually change.
            let old_q = *quantity.read();
            let mut new_q = old_q;
            new_q = new_q.clamp(1, max_per_item);
            new_q = new_q.min(stock.max(1)); // keep at least 1 if there is stock

            if new_q != old_q {
                quantity.set(new_q);
            }
        }
    });

    // Helper functions...
    let currency_code_symbol = |_code: &str| -> &str { "$" };
    let validate_email = |email: &str| -> bool { email.contains("@") && email.contains(".") };
    let get_container_material = |product_form: &ProductForm| -> &str {
        match product_form {
            ProductForm::Vial => "Glass Vial",
            ProductForm::Ampoule => "Encapsulated Glass",
            ProductForm::DirectSpray | ProductForm::VerticalSpray => "Glass Bottle, Vertical Spray",
            ProductForm::Solution => "Glass Bottle (With Marked Dropper)",
            ProductForm::Container => "Plastic/Glass Container",
            _ => "Various",
        }
    };
    let scroll_left = move |_| {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(element) = document.get_element_by_id("related-products-scroll") {
                    let current_scroll = element.scroll_left();
                    element.set_scroll_left(current_scroll - 300);
                    scroll_position.set((current_scroll - 300) as f64);
                }
            }
        }
    };
    let scroll_right = move |_| {
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(element) = document.get_element_by_id("related-products-scroll") {
                    let current_scroll = element.scroll_left();
                    element.set_scroll_left(current_scroll + 300);
                    scroll_position.set((current_scroll + 300) as f64);
                }
            }
        }
    };

    rsx! {
        div {
            class: "content-container py-2 sm:py-12",
            div {
                class: "flex justify-center items-start py-6",
                div {
                    class: "flex flex-col md:flex-row gap-8 max-w-6xl w-full px-4 justify-center",

                    if let Some(product) = current_product.read().as_ref() {

                        // Include seo/meta tags
                        {
                            let title = if product.title.contains("(") {
                                format!("{} {}", t!("buy"), product.title)
                            } else {
                                format!("{} {} ({})", t!("buy"), product.title, product.product_form)
                            };

                            let description = format!(
                                "Purchase {}{} at Novel Distributions - Shipping Worldwide{}{}", // ADD MORE HERE
                                product.title,
                                if let Some(alternate_names) = product.alternate_names.clone() {
                                    format!(" ({})",
                                    alternate_names.iter().map(|n| n.to_string())
                                        .collect::<Vec<String>>()
                                        .join("/")
                                    )
                                } else {
                                    "".to_string()
                                },
                                if let Some(purity) = &product.purity { format!(", {}% pure", purity) } else { "".to_string() },
                                if let Some(cas) = &product.cas { format!(", CAS: {}", cas) } else { "".to_string() }
                            );

                            let preview = &*preview_url.read();


                            rsx! {
                                Meta {
                                    title: title,
                                    description: description,
                                    image_url: if !preview.is_empty() && preview != "smiles" { preview.to_string() } else { "".to_string() }
                                }
                            }
                        }

                        // Set meta information

                        // Product Image and Thumbnails Section (NOW ON LEFT)
                        div {
                            class: "flex-grow md:w-[400px]",

                            // Main Image
                            div {
                                class: "relative aspect-square rounded-lg overflow-hidden shadow-lg border-ui-border-base border",

                                {
                                    let preview = &*preview_url.read();
                                    if !preview.is_empty() && preview != "smiles" {
                                        // Add state for zoom
                                        let mut is_zoomed = use_signal(|| false);
                                        let mut transform_style = use_signal(|| String::from("transform: scale(1)"));

                                        rsx! {
                                            div {
                                                class: if *is_zoomed.read() {
                                                    "cursor-zoom-out relative w-full h-full overflow-hidden"
                                                } else {
                                                    "cursor-zoom-in relative w-full h-full overflow-hidden"
                                                },

                                                img {
                                                    id: "zoomable-image",
                                                    alt: format!("{} {}", product.title, t!("thumbnail")),
                                                    src: "{preview}",
                                                    class: if *is_zoomed.read() {
                                                        "absolute object-contain cursor-zoom-out transition-transform duration-300 ease-out select-none"
                                                    } else {
                                                        "w-full h-full object-contain cursor-zoom-in transition-transform duration-300 ease-out select-none"
                                                    },
                                                    style: format!("cursor: {}; {}",
                                                        if *is_zoomed.read() { "zoom-out" } else { "zoom-in" },
                                                        transform_style()
                                                    ),
                                                    loading: "eager",

                                                    onclick: move |evt| {
                                                        if *is_zoomed.read() {
                                                            // Zoom out
                                                            is_zoomed.set(false);
                                                            transform_style.set("transform: scale(1)".to_string());
                                                        } else {
                                                            // Zoom in
                                                            is_zoomed.set(true);

                                                            // Get click coordinates
                                                            let client_x = evt.client_coordinates().x;
                                                            let client_y = evt.client_coordinates().y;

                                                            // Use eval to get the bounding rect and calculate position
                                                            spawn(async move {
                                                                let eval_result = document::eval(&format!(r#"
                                                                    const img = document.getElementById('zoomable-image');
                                                                    const rect = img.getBoundingClientRect();
                                                                    const relX = ({} - rect.left) / rect.width;
                                                                    const relY = ({} - rect.top) / rect.height;

                                                                    // Clamp values between 0 and 1
                                                                    const clampedX = Math.max(0, Math.min(1, relX));
                                                                    const clampedY = Math.max(0, Math.min(1, relY));

                                                                    return {{ x: clampedX, y: clampedY }};
                                                                "#, client_x, client_y)).await;

                                                                if let Ok(result) = eval_result {
                                                                    if let Some(coords) = result.as_object() {
                                                                        let rel_x = coords.get("x").and_then(|v| v.as_f64()).unwrap_or(0.5);
                                                                        let rel_y = coords.get("y").and_then(|v| v.as_f64()).unwrap_or(0.5);

                                                                        // Zoom scale factor (adjust as needed)
                                                                        let zoom_scale = 2.0;

                                                                        // Calculate translation to keep zoomed image within bounds
                                                                        let max_translate = (zoom_scale - 1.0) * 50.0;

                                                                        // Calculate where to position the image based on click position
                                                                        let translate_x: f64 = (0.5 - rel_x) * 100.0 * (zoom_scale - 1.0);
                                                                        let translate_y: f64 = (0.5 - rel_y) * 100.0 * (zoom_scale - 1.0);

                                                                        // Clamp translations to prevent image from going out of bounds
                                                                        let clamped_translate_x = translate_x.max(-max_translate).min(max_translate);
                                                                        let clamped_translate_y = translate_y.max(-max_translate).min(max_translate);

                                                                        // Apply the transformation
                                                                        transform_style.set(format!(
                                                                            "transform: scale({}) translate({}%, {}%)",
                                                                            zoom_scale,
                                                                            clamped_translate_x / zoom_scale,
                                                                            clamped_translate_y / zoom_scale
                                                                        ));
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    },

                                                    // Optional: Allow mouse move while zoomed to pan around
                                                    onmousemove: move |evt| {
                                                        if *is_zoomed.read() {
                                                            let client_x = evt.client_coordinates().x;
                                                            let client_y = evt.client_coordinates().y;

                                                            spawn(async move {
                                                                let eval_result = document::eval(&format!(r#"
                                                                    const img = document.getElementById('zoomable-image');
                                                                    const rect = img.getBoundingClientRect();
                                                                    const relX = ({} - rect.left) / rect.width;
                                                                    const relY = ({} - rect.top) / rect.height;

                                                                    const clampedX = Math.max(0, Math.min(1, relX));
                                                                    const clampedY = Math.max(0, Math.min(1, relY));

                                                                    return {{ x: clampedX, y: clampedY }};
                                                                "#, client_x, client_y)).await;

                                                                if let Ok(result) = eval_result {
                                                                    if let Some(coords) = result.as_object() {
                                                                        let rel_x = coords.get("x").and_then(|v| v.as_f64()).unwrap_or(0.5);
                                                                        let rel_y = coords.get("y").and_then(|v| v.as_f64()).unwrap_or(0.5);

                                                                        let zoom_scale = 2.5;
                                                                        let max_translate = (zoom_scale - 1.0) * 50.0;

                                                                        let translate_x: f64 = (0.5 - rel_x) * 100.0 * (zoom_scale - 1.0);
                                                                        let translate_y: f64 = (0.5 - rel_y) * 100.0 * (zoom_scale - 1.0);

                                                                        let clamped_translate_x = translate_x.max(-max_translate).min(max_translate);
                                                                        let clamped_translate_y = translate_y.max(-max_translate).min(max_translate);

                                                                        transform_style.set(format!(
                                                                            "transform: scale({}) translate({}%, {}%)",
                                                                            zoom_scale,
                                                                            clamped_translate_x / zoom_scale,
                                                                            clamped_translate_y / zoom_scale
                                                                        ));
                                                                    }
                                                                }
                                                            });
                                                        }
                                                    },

                                                    // Reset on mouse leave
                                                    onmouseleave: move |_| {
                                                        if *is_zoomed.read() {
                                                            is_zoomed.set(false);
                                                            transform_style.set("transform: scale(1)".to_string());
                                                        }
                                                    },

                                                    // Prevent drag behavior
                                                    ondragstart: move |evt| evt.prevent_default()
                                                }
                                            }
                                        }
                                    } else if product.smiles.is_some() && product.enable_render_if_smiles {
                                        rsx! {
                                            div {
                                                title: t!("mol-info"),
                                                class: "flex items-center justify-center h-full text-gray-500",

                                                SmilesViewer {
                                                    smiles: product.smiles.clone().unwrap().clone()
                                                }

                                            }
                                        }
                                    } else {
                                        rsx! {
                                            div {
                                                class: "flex items-center justify-center h-full text-gray-500",
                                                { t!("no-thumbnail") }
                                            }
                                        }
                                    }
                                }

                                /*

                                // Mailing List for Out of Stock Products
                                if (product.force_no_stock || current_stock_quantity() == 0) && !*close_stock_updates.read() {
                                    div {
                                        class: "absolute inset-x-0 top-0 w-full pt-4 pb-4 bg-white bg-opacity-60 text-black border-ui-border-base border-b",
                                        div {
                                            class: "left-0 right-0 mx-4",
                                            button {
                                                class: "absolute mr-8 cursor-pointer right-0 bg-transparent border-0 text-black",
                                                onclick: move |_| close_stock_updates.set(true),
                                                "X"
                                            }

                                            if sub_complete.read().is_empty() {
                                                p { class: "text-base mb-2", { t!("receive-stock-updates") } }
                                                if !*mail_join_open.read() {
                                                    p { class: "text-sm text-gray-600", { t!("want-to-receive-updates", title: product.title.clone()) } }
                                                    div { class: "flex mt-4 gap-x-2",
                                                        a {
                                                            target: "_blank",
                                                            href: "https://x.com/",
                                                            button {
                                                                class: "text-sm border border-gray-300 px-5 py-2 rounded-md min-w-24 hover:bg-gray-100 hover:text-black",
                                                                onclick: move |_| close_stock_updates.set(true),
                                                                { t!("follow-on-x") }
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    div { class: "mt-2",
                                                        input {
                                                            r#type: "text",
                                                            placeholder: { t!("your-name-slash-nickname") },
                                                            class: "w-full px-3 py-2 rounded-md text-black mb-2",
                                                            value: "{sub_name}",
                                                            oninput: move |evt| sub_name.set(evt.value())
                                                        }
                                                        input {
                                                            r#type: "email",
                                                            placeholder: { t!("your-email") },
                                                            class: "w-full px-3 py-2 rounded-md text-black mb-2",
                                                            value: "{sub_email}",
                                                            oninput: move |evt| sub_email.set(evt.value())
                                                        }
                                                        p { class: "text-xs text-gray-300 mb-2", { t!("you-can-unsub") } }
                                                        if !sub_name.read().is_empty() && !sub_email.read().is_empty() && validate_email(&sub_email.read()) {
                                                            button {
                                                                class: "text-sm border border-white px-5 py-2 rounded-md min-w-24 hover:bg-gray-100 hover:text-black",
                                                                onclick: move |_| {
                                                                    sub_complete.set("true".to_string());
                                                                },
                                                                { t!("subscribe") }
                                                            }
                                                        }
                                                    }
                                                }
                                            } else if *sub_complete.read() == "true" {
                                                { t!("joined-mailing") }
                                            } else if *sub_complete.read() == "error" {
                                                { t!("failed-mailing") }
                                            }
                                        }
                                    }
                                }

                                */
                            }

                            // Thumbnails Row
                            div {
                                id: "img_row",
                                class: "flex justify-center mt-3.5 overflow-x-auto",

                                // Variant thumbnails
                                {
                                    if let Some(ref variants) = product.variants {
                                        let current_variant_idx = *current_variant.read();
                                        if let Some(current_variant) = variants.get(current_variant_idx) {
                                            let has_main_thumbnail = current_variant.thumbnail_url.is_some();
                                            let has_additional_thumbnails = current_variant.additional_thumbnail_urls
                                                .as_ref()
                                                .map(|thumbs| !thumbs.is_empty())
                                                .unwrap_or(false);

                                            // Only show thumbnails if at least one exists
                                            if has_main_thumbnail || has_additional_thumbnails {
                                                rsx! {
                                                    div {
                                                        class: "flex flex-nowrap mt-4 px-1.5", // Removed gap, will use margins instead

                                                        // Main thumbnail (if exists)
                                                        {
                                                            if let Some(ref thumbnail_url) = current_variant.thumbnail_url {
                                                                let thumbnail_clone = thumbnail_url.clone();
                                                                rsx! {
                                                                    div {
                                                                        key: "{\"main-thumb\"}",
                                                                        class: "flex-shrink-0 cursor-pointer w-32 h-32 rounded-md overflow-hidden border-ui-border-base border mr-1.5",
                                                                        onclick: move |_| preview_url.set(thumbnail_clone.clone()),
                                                                        img {
                                                                            alt: { format!("{} {}", product.title, t!("thumbnail")) },
                                                                            src: "{thumbnail_url}",
                                                                            class: "w-full h-full object-cover",
                                                                            loading: "lazy"
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                rsx! {}
                                                            }
                                                        }

                                                        // Additional thumbnails (if they exist)
                                                        {
                                                            if let Some(ref additional_thumbnails) = current_variant.additional_thumbnail_urls {
                                                                rsx! {
                                                                    {additional_thumbnails.iter().enumerate().map(|(i, thumbnail_url)| {
                                                                        let thumbnail_clone = thumbnail_url.clone();
                                                                        rsx! {
                                                                            div {
                                                                                key: "{i}",
                                                                                class: "flex-shrink-0 cursor-pointer w-32 h-32 rounded-md overflow-hidden border-ui-border-base border mr-1.5",
                                                                                onclick: move |_| preview_url.set(thumbnail_clone.clone()),
                                                                                img {
                                                                                    alt: { format!("{} {} {}", product.title, t!("additional-thumbnail"), i) },
                                                                                    src: "{thumbnail_url}",
                                                                                    class: "w-full h-full object-cover",
                                                                                    loading: "lazy"
                                                                                }
                                                                            }
                                                                        }
                                                                    })}
                                                                }
                                                            } else {
                                                                rsx! {}
                                                            }
                                                        }

                                                        // Molecule thumbnail (if available and has other thumbnails)
                                                        if product.smiles.is_some() && product.enable_render_if_smiles {
                                                            if let Some(ref variants) = product.variants {
                                                                if variants.iter().any(|v| v.thumbnail_url.is_some()) {
                                                                    div {
                                                                        title: { t!("mol-info") },
                                                                        class: "flex-shrink-0 cursor-pointer w-32 h-32 rounded-md border-ui-border-base border overflow-hidden flex items-center justify-center mr-1.5",
                                                                        onclick: move |_| preview_url.set("smiles".to_string()),
                                                                        div {
                                                                            class: "w-full h-full",
                                                                            SmilesViewer {
                                                                                smiles: product.smiles.clone().unwrap().clone()
                                                                            }
                                                                        }
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
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }
                        }

                        // Product Information Section (NOW ON RIGHT)
                        div {
                            class: "md:sticky md:min-w-[49%] md:top-48 md:w-[400px] text-bbase",

                            // Product Title and Subtitle
                            h1 {
                                class: "text-ui-fg-base text-2dot5xl font-normal",
                                if product.title.contains("(") {
                                    "{product.title}"
                                    document::Title { { format!("{} {}", t!("buy"), product.title ) } }
                                } else {
                                    "{product.title} ({product.product_form.to_frontend_string()})"
                                    document::Title { { format!("{} {} ({})", t!("buy"), product.title, product.product_form) } }
                                }
                            }

                            p {
                                class: "font-normal font-sans text-sm text-ui-fg-muted mt-2",
                                if let Some(ref subtitle) = product.subtitle {
                                    "{subtitle}"
                                }

                                if let Some(ref variants) = product.variants {
                                    if variants.len() == 1 {
                                        if let Some(variant) = variants.first() {
                                            if product.subtitle.is_some() {
                                                " — "
                                            }
                                            if !variant.variant_name.is_empty() && variant.variant_name != "Default option value" {
                                                "{variant.variant_name}"
                                            }
                                        }
                                    }
                                }
                            }

                            // Stock Status
                            div {
                                class: "flex mt-3",
                                {
                                    if let Some(ref variants) = product.variants {
                                        let current_variant_idx = *current_variant.read();
                                        if let Some(current_variant) = variants.get(current_variant_idx) {
                                            // Check for pre-order first
                                            if product.pre_order {
                                                rsx! {
                                                    div { class: "blinking-blue mt-[6px] mr-[8px]" }
                                                    p { { t!("pre-order") } }
                                                }
                                            } else if let Some(qty) = current_variant.calculated_stock_quantity {
                                                if product.force_no_stock || qty == 0 {
                                                    if product.back_order {
                                                        rsx ! {
                                                            // CHANGED: From blinking-amber to blinking-yellow
                                                            div { class: "blinking-yellow mt-[6px] mr-[8px]" }
                                                            p { { t!("backorder-oos") } }
                                                        }
                                                    } else {
                                                        rsx! {
                                                            div { class: "blinking-red mt-[6px] mr-[8px]" }
                                                            p { { t!("sold-out") } }
                                                        }
                                                    }
                                                } else if qty < 12 {
                                                    rsx! {
                                                        div { class: "blinking-amber mt-[6px] mr-[8px]" }
                                                        p { class: "effect-shine", { format!("{}{}", t!("qty-in-stock", qty: qty), if product.back_order { format!(" ({})", t!("can-be-backordered")) } else { "".to_string() }) } }
                                                    }
                                                } else {
                                                    rsx! {
                                                        div { class: "blinking-green mt-[6px] mr-[8px]" }
                                                        p { class: "effect-shine", { t!("qty-in-stock", qty: qty) } }
                                                    }
                                                }
                                            } else {
                                                rsx! {}
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }

                            // Product Description
                            if let Some(ref small_desc) = product.small_description_md {
                                div {
                                    class: "mt-3 mb-3",
                                    div {
                                        class: "text-ui-fg-subtle text-sm uldisc",
                                        dangerous_inner_html: "{small_desc}"
                                    }
                                }
                            }

                            // Price
                            div {
                                class: "mb-5",
                                {
                                    let current_variant_idx = *current_variant.read();
                                    if let Some(ref variants) = product.variants {
                                        if current_variant_idx < variants.len() {
                                            if let Some(variant) = variants.get(current_variant_idx) {
                                                rsx! {
                                                    span {
                                                        class: "flex",
                                                        h2 {
                                                            title: { t!("price-per-single-item") },
                                                            class: if variant.price_standard_without_sale.is_some() { "text-xl font-weight-450 text-sale-500" } else { "text-xl font-weight-450" },
                                                            "{currency_code_symbol(\"USD\")}"
                                                            "{variant.price_standard_usd:.2}"
                                                        }
                                                        {
                                                            if let Some(price_standard_without_sale) = variant.price_standard_without_sale {
                                                                rsx! {
                                                                    h2 {
                                                                        class: "text-xl text-gray-700 line-through font-weight-450 ml-3",
                                                                        "{currency_code_symbol(\"USD\")}"
                                                                        "{price_standard_without_sale:.2}"
                                                                        " "
                                                                    }
                                                                }
                                                            } else {
                                                                rsx! {}
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                rsx! { h2 { { t!("price-not-available") } } }
                                            }
                                        } else {
                                            rsx! { h2 { { t!("price-not-available") } } }
                                        }
                                    } else {
                                        rsx! { h2 { { t!("price-not-available") } } }
                                    }
                                }
                            }

                            // Product Details Collapsible Card
                            div {
                                class: "divide-y mb-5 elevation-none border rounded-md",

                                // Product Details Section
                                details {
                                    class: "pr-2 text-slight-black",
                                    summary {
                                        class: "flex-1 px-3 py-3 pt-1.5 min-h-9 unselectable cursor-pointer",
                                        { t!("product-details") }
                                    }
                                    div {
                                        class: "px-3 py-2 text-ui-fg-subtle border-t",
                                        div {
                                            class: "w-full",
                                            div {
                                                class: "text-small-regular text-slight-black",
                                                div {
                                                    class: "grid grid-cols-2 gap-x-8",
                                                    div {
                                                        class: "flex flex-col gap-y-4",
                                                        div {
                                                            span { class: "font-medium", { t!("container-material") } }
                                                            p { "{get_container_material(&product.product_form)}" }
                                                        }
                                                        div {
                                                            span { class: "font-medium", { t!("product-form") } }
                                                            p { "{product.product_form}" }
                                                        }
                                                        div {
                                                            span { class: "font-medium", { t!("physical-description") } }
                                                            p {
                                                                if let Some(ref desc) = product.physical_description {
                                                                    "{desc}"
                                                                } else {
                                                                    "-"
                                                                }
                                                            }
                                                        }
                                                    }
                                                    div {
                                                        class: "flex flex-col gap-y-4",
                                                        div {
                                                            span { class: "font-medium", { t!("weight") } }
                                                            p {
                                                                if let Some(weight) = product.weight {
                                                                    "{weight}g"
                                                                } else {
                                                                    "-"
                                                                }
                                                            }
                                                        }
                                                        div {
                                                            span { class: "font-medium", { t!("dimensions") } }
                                                            p {
                                                                if let (Some(height), Some(width)) = (product.dimensions_height, product.dimensions_width) {
                                                                    "{height:.0}x{width:.0}mm"
                                                                } else {
                                                                    {
                                                                        match product.product_form {
                                                                            ProductForm::DirectSpray | ProductForm::VerticalSpray => "120x30mm",
                                                                            ProductForm::Solution => "100x33mm",
                                                                            _ => "-"
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        div {
                                                            span { class: "font-medium", "SKUs" }
                                                            p {
                                                                if let Some(ref variants) = product.variants {
                                                                    for (i, variant) in variants.iter().enumerate() {
                                                                        if let Some(ref sku) = variant.pbx_sku {
                                                                            "{sku}「{variant.variant_name}」"
                                                                        } else {
                                                                            "None ({variant.variant_name})"
                                                                        }
                                                                        if i != variants.len() - 1 {
                                                                            "  "
                                                                        }
                                                                    }
                                                                } else {
                                                                    { t!("no-variants") }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Shipping Information Section
                                details {
                                    class: "pr-2",
                                    summary {
                                        class: "flex-1 px-3 pt-1.5 min-h-9 text-slight-black unselectable cursor-pointer",
                                        { t!("shipping-information") }
                                    }
                                    div {
                                        class: "px-3 py-2 text-ui-fg-subtle border-t text-sm",
                                        { t!("shipped-bubble-wrap-info") }
                                        {
                                            match product.product_form {
                                                ProductForm::DirectSpray | ProductForm::VerticalSpray => {
                                                    rsx! { { format!(" {}", t!("spray-clips-info")) } }
                                                }
                                                _ => rsx! { }
                                            }
                                        }
                                        { format!(" {} ", t!("data-sheets-info")) }
                                        Link {
                                            class: "a",
                                            to: Route::ShippingPolicy {},
                                            { t!("shipping-page") }
                                        }
                                        "."
                                    }
                                }
                            }

                            // Variant Selection
                            {
                                if let Some(ref variants) = product.variants {
                                    if variants.len() > 1 {
                                        if variants.first().map(|v| &v.variant_name).unwrap_or(&"".to_string()) != "Default option value" {
                                            rsx! {
                                                div {
                                                    class: "mt-2",
                                                    div {
                                                        class: "mt-2 flex flex-wrap justify-between gap-2",
                                                        for (i, variant) in {
                                                                let mut indexed_variants: Vec<_> = variants.iter().enumerate().collect();
                                                                indexed_variants.sort_by(|a, b| {
                                                                    let extract_number = |name: &str| -> f64 {
                                                                        name.chars()
                                                                            .take_while(|c| c.is_ascii_digit() || *c == '.')
                                                                            .collect::<String>()
                                                                            .parse()
                                                                            .unwrap_or(0.0)
                                                                    };

                                                                    let a_num = extract_number(&a.1.variant_name);
                                                                    let b_num = extract_number(&b.1.variant_name);
                                                                    a_num.partial_cmp(&b_num).unwrap_or(std::cmp::Ordering::Equal)
                                                                });
                                                                indexed_variants
                                                            } {
                                                            button {
                                                                key: "{i}",
                                                                class: format!(
                                                                    "max-w-[50%] text-bbase {} flex items-center justify-center bg-ui-bg-subtle border h-10 rounded-md p-2 flex-1 hover:shadow-elevation-card-rest transition-shadow ease-in-out duration-150",
                                                                    if i == *current_variant.read() { "border-interactive" } else { "border-ui-border-base" }
                                                                ),
                                                                onclick: move |_| {
                                                                    current_variant.set(i);
                                                                    add_btn_text.set(t!("add-to-cart"));
                                                                    add_btn_added.set(false);
                                                                },
                                                                div {
                                                                    class: "flex items-center",
                                                                    if product.force_no_stock || variant.calculated_stock_quantity.unwrap_or(0) == 0 {
                                                                        span {
                                                                            // CHANGED: Add condition to check if it's backorder vs out of stock
                                                                            if product.back_order {
                                                                                /*
                                                                                div { class: "blinking-yellow mt-[2px] mr-[6px]" }
                                                                                */
                                                                            } else {
                                                                                div { class: "blinking-red mt-[2px] mr-[6px]" }
                                                                            }
                                                                        }
                                                                    }
                                                                    span { "{variant.variant_name}" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! { }
                                        }
                                    } else {
                                        rsx! { }
                                    }
                                } else {
                                    rsx! { }
                                }
                            }

                            // Add to Cart Section
                            div {
                                class: "mb-4 mt-2",
                                div {
                                    class: "flex justify-center",

                                    // Quantity Selector
                                    div {
                                        title: "Item Quantity",
                                        class: "w-36",
                                        input {
                                            r#type: "number",
                                            class: "w-full border rounded-md px-3 py-2",
                                            value: "{quantity}",
                                            min: "1",
                                            max: "{std::cmp::min(calc_current_variant_stock(), max_per_item)}",
                                            oninput: move |evt| {
                                                if let Ok(val) = evt.value().parse::<i32>() {
                                                    let stock = calc_current_variant_stock();
                                                    let clamped = val.clamp(1, stock.max(0)).clamp(1, max_per_item);
                                                    quantity.set(clamped);
                                                    add_btn_text.set(t!("add-to-cart"));
                                                    add_btn_added.set(false);
                                                }
                                            }
                                        }
                                    }

                                    // Add to Cart Button
                                    {
                                        let stock_now = current_stock_quantity();
                                        let qty_val = *quantity.read();
                                        let disabled = product.force_no_stock || stock_now == 0 || qty_val <= 0;
                                        if disabled {
                                            rsx! {
                                                button {
                                                    title: { t!("product-currently-out-of-stock") },
                                                    class: "ml-2 cursor-not-allowed bg-blend transition-fg relative inline-flex items-center justify-center overflow-hidden rounded-md outline-none disabled:bg-ui-bg-disabled disabled:border-ui-border-base disabled:text-ui-fg-disabled disabled:shadow-buttons-neutral disabled:after:hidden after:transition-fg after:absolute after:inset-0 after:content-[''] shadow-buttons-inverted text-ui-fg-on-inverted bg-zinc-700 after:button-inverted-gradient active:bg-ui-button-inverted-pressed active:after:button-inverted-pressed-gradient focus:!shadow-buttons-inverted-focus txt-compact-small-plus gap-x-1.5 px-3 py-1.5 w-full h-10",
                                                    disabled: true,
                                                    { t!("out-of-stock") }
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                button {
                                                    class: "ml-2 hover:bg-zinc-800 bg-blend transition-fg relative inline-flex items-center justify-center overflow-hidden rounded-md outline-none disabled:bg-ui-bg-disabled disabled:border-ui-border-base disabled:text-ui-fg-disabled disabled:shadow-buttons-neutral disabled:after:hidden after:transition-fg after:absolute after:inset-0 after:content-[''] shadow-buttons-inverted text-ui-fg-on-inverted bg-ui-button-inverted after:button-inverted-gradient hover:bg-ui-button-inverted-hover hover:after:button-inverted-hover-gradient active:bg-ui-button-inverted-pressed active:after:button-inverted-pressed-gradient focus:!shadow-buttons-inverted-focus txt-compact-small-plus gap-x-1.5 px-3 py-1.5 w-full h-10",
                                                    disabled: *adding.read(),
                                                    onclick: {
                                                        let add_btn_text = add_btn_text.clone();
                                                        let add_btn_added = add_btn_added.clone();
                                                        let error_line = error_line.clone();
                                                        let adding = adding.clone();
                                                        let quantity = quantity.clone();
                                                        let mut refresh_trigger = refresh_trigger.clone();
                                                        let current_product_sig = current_product.clone();
                                                        let current_variant_idx = *current_variant.read();
                                                        move |_| {
                                                            if let Some(prod) = current_product_sig() {
                                                                if let Some(ref variants) = prod.variants {
                                                                    if let Some(variant) = variants.get(current_variant_idx) {
                                                                        let variant_id = variant.id.clone();
                                                                        let req_qty = *quantity.read();
                                                                        let clamped_req = req_qty.min(max_per_item).max(1);

                                                                        spawn({
                                                                            let mut add_btn_text = add_btn_text.clone();
                                                                            let mut add_btn_added = add_btn_added.clone();
                                                                            let mut error_line = error_line.clone();
                                                                            let mut adding = adding.clone();
                                                                            let mut quantity = quantity.clone();
                                                                            let mut refresh_trigger = refresh_trigger.clone();
                                                                            async move {
                                                                                adding.set(true);
                                                                                add_btn_text.set(t!("adding-dot-dot-dot"));
                                                                                error_line.set("".to_string());

                                                                                let result = server_functions::add_or_update_basket_item(variant_id.clone(), clamped_req).await;

                                                                                match result {
                                                                                    Ok(resp) => {
                                                                                        // Update global cart
                                                                                        GLOBAL_CART.with_mut(|c| *c = Some(resp.basket.clone()));

                                                                                        match resp.status.as_str() {
                                                                                            "Complete" => {
                                                                                                add_btn_text.set(t!("added"));
                                                                                                add_btn_added.set(true);
                                                                                                quantity.set(1);
                                                                                                error_line.set("".to_string());
                                                                                            }
                                                                                            "Reduced" => {
                                                                                                add_btn_text.set(t!("reduced"));
                                                                                                add_btn_added.set(false);
                                                                                                error_line.set(t!("reduced-info"));
                                                                                                let current = *refresh_trigger.read();
                                                                                                refresh_trigger.set(current + 1);
                                                                                            }
                                                                                            "Removed" => {
                                                                                                add_btn_text.set(t!("unavailable"));
                                                                                                add_btn_added.set(false);
                                                                                                error_line.set(t!("removed-info"));
                                                                                                let current = *refresh_trigger.read();
                                                                                                refresh_trigger.set(current + 1);
                                                                                            }
                                                                                            "NotFound" => {
                                                                                                add_btn_text.set(t!("not-found"));
                                                                                                add_btn_added.set(false);
                                                                                                error_line.set(t!("not-found-info"));
                                                                                                let current = *refresh_trigger.read();
                                                                                                refresh_trigger.set(current + 1);
                                                                                            }
                                                                                            "Invalid" => {
                                                                                                add_btn_text.set(t!("invalid"));
                                                                                                add_btn_added.set(false);
                                                                                                error_line.set(t!("invalid-info"));
                                                                                            }
                                                                                            _ => {
                                                                                                add_btn_text.set(t!("updated"));
                                                                                                add_btn_added.set(true);
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    Err(e) => {
                                                                                        add_btn_text.set(t!("failed-to-add"));
                                                                                        add_btn_added.set(false);
                                                                                        error_line.set(t!("could-not-add-error", error: format!("{:?}", e)));
                                                                                        let current = *refresh_trigger.read();
                                                                                        refresh_trigger.set(current + 1);
                                                                                    }
                                                                                }

                                                                                adding.set(false);
                                                                            }
                                                                        });
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    },
                                                    img {
                                                        style: "height:18px;filter: invert(1)",
                                                        src: if *add_btn_added.read() { asset!("/assets/icons/bag-check-outline.svg") } else { asset!("/assets/icons/bag-add-outline.svg") }
                                                    }
                                                    "{add_btn_text}"
                                                }
                                            }
                                        }
                                    }
                                }

                                if !error_line.read().is_empty() {
                                    p { class: "pt-3 text-orange-700", "{error_line}" }
                                }

                                p {
                                    class: "my-3 text-xs text-ui-fg-subtle",
                                    { format!("{} ", t!("tos-line")) }
                                    Link {
                                        title: { t!("visit-policies-page") },
                                        class: "underline",
                                        to: Route::Policies {},
                                        { t!("terms-and-conditions") }
                                    }
                                    { format!(" {}", t!("prior-to-ordering")) }
                                }
                            }
                        }
                    } else if *product_not_found.read() {
                        div {
                            class: "text-red-500",
                            { t!("product-not-found", handle: handle()) }
                        }
                    }
                }
            }

            // Research and Details Section
            if let Some(product) = current_product.read().as_ref() {
                div {
                    class: "flex justify-center w-full",
                    div {
                        class: "justify-self-start w-full max-w-[1130px]",

                        hr { class: "my-4" }
                        h2 { class: "mb-4 mt-6", { t!("research") } }

                        // Labs button placeholder
                        if let Some(ref plabs_node_id) = product.plabs_node_id {
                            p { class: "text-ui-fg-subtle", { t!("for-more-technical-info") } }
                            div { class: "my-3",
                                a {
                                    target: "_blank",
                                    href: "https://labs.penchant.bio/library/{plabs_node_id}",
                                    class: "inline-flex items-center px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700",
                                    { t!("view-labs-page") }
                                }
                            }
                        }

                        // Main description
                        if let Some(ref main_desc) = product.main_description_md {
                            div {
                                class: "mt-4",
                                div {
                                    dangerous_inner_html: "{main_desc}"
                                }
                            }
                        }

                        // Chemical Information Table
                        div {
                            class: "mt-4 rounded-md border-ui-border-base border",
                            table {
                                class: "table-auto w-full",
                                tbody {
                                    class: "data-bodyy",
                                    tr {
                                        class: "border-b",
                                        td { class: "pl-2 py-1 w-[25%]", { t!("names") } }
                                        td {
                                            class: "px-1",
                                            if let Some(ref alt_names) = product.alternate_names {
                                                "{alt_names.join(\", \")}"
                                            } else {
                                                span { class: "text-gray-400", { t!("no-other-names") } }
                                            }
                                        }
                                    }
                                    tr {
                                        class: "border-b",
                                        td { class: "pl-2 py-1 w-[25%]", { t!("pubchem") } }
                                        td {
                                            class: "px-1",
                                            if let Some(ref pubchem) = product.pubchem_cid {
                                                a {
                                                    class: "new-tab-link",
                                                    target: "_blank",
                                                    href: "https://pubchem.ncbi.nlm.nih.gov/compound/{pubchem}",
                                                    "{pubchem}"
                                                }
                                            } else {
                                                span { class: "text-gray-400", { t!("no-pubchem-page") } }
                                            }
                                        }
                                    }
                                    tr {
                                        class: "border-b",
                                        td { class: "pl-2 py-1 w-[25%]", { t!("cas") } }
                                        td {
                                            class: "hover:bg-gray-100 px-1 rounded",
                                            style: "user-select: all;",
                                            if let Some(ref cas) = product.cas {
                                                "{cas}"
                                            } else {
                                                span { class: "text-gray-400", { t!("no-cas-available") } }
                                            }
                                        }
                                    }
                                    tr {
                                        class: "border-b",
                                        td { class: "pl-2 py-1 w-[25%]", { t!("iupac") } }
                                        td {
                                            class: "break-words hover:bg-gray-100 px-1 rounded",
                                            style: "word-break: break-all; user-select: all;",
                                            if let Some(ref iupac) = product.iupac {
                                                "{iupac}"
                                            } else {
                                                span { class: "text-gray-400", { t!("no-iupac-available") } }
                                            }
                                        }
                                    }
                                    tr {
                                        class: "border-b",
                                        td { class: "pl-2 py-1 w-[25%]", { t!("mol-formula") } }
                                        td {
                                            class: "hover:bg-gray-100 px-1 rounded",
                                            style: "user-select: all;",
                                            if let Some(ref mol_form) = product.mol_form {
                                                "{mol_form}"
                                            } else {
                                                span { class: "text-gray-400", { t!("no-mol-formula") } }
                                            }
                                        }
                                    }
                                    tr {
                                        td { class: "pl-2 py-1 w-[25%]", { t!("smiles") } }
                                        td {
                                            class: "break-words hover:bg-gray-100 px-1 rounded",
                                            style: "word-break: break-all; user-select: all;",
                                            if let Some(ref smiles) = product.smiles {
                                                "{smiles}"
                                            } else {
                                                span { class: "text-gray-400", { t!("no-smiles") } }
                                            }
                                        }
                                    }
                                }
                            }
                        }


                        h2 { class: "mb-4 mt-6 font-normal txt-medium text-xl", { t!("physical-details") } }

                        div {
                            class: "rounded-md border-ui-border-base border",
                            table {
                                class: "table-auto w-full",
                                tbody {
                                    class: "data-bodyy",
                                    tr {
                                        /*class: "border-b",*/
                                        td { class: "pl-2 py-1 w-[25%]", { t!("physical-description") } }
                                        td {
                                            class: "px-1",
                                            if let Some(ref physical_description) = product.physical_description {
                                                "{physical_description}"
                                            } else {
                                                span { class: "text-gray-400", { t!("no-physical-description") } }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Analysis Section
                        h2 { class: "mb-4 mt-6 font-normal txt-medium text-xl", { t!("analysis") } }
                        div {
                            style: "line-height: 1;",
                            class: "flex overflow-x-auto",

                            {
                                let has_analysis = product.analysis_url_qnmr.is_some() ||
                                                 product.analysis_url_hplc.is_some() ||
                                                 product.analysis_url_qh1.is_some();

                                if has_analysis {
                                    rsx! {
                                        if let Some(ref qnmr_url) = product.analysis_url_qnmr {
                                            a {
                                                href: "{qnmr_url}",
                                                target: "_blank",
                                                div {
                                                    title: { t!("open-qnmr") },
                                                    class: "mr-2 cursor-pointer rounded-lg border-ui-border-base border hover:bg-gray-100 w-48 h-[5.2rem] px-3 py-2",
                                                    h2 { class: "text-xl text-black", { t!("results-qnmr") } }
                                                    small { class: "text-xs text-ui-fg-muted", { t!("details-qnmr") } }
                                                }
                                            }
                                        }
                                        if let Some(ref qh1_url) = product.analysis_url_qh1 {
                                            a {
                                                href: "{qh1_url}",
                                                target: "_blank",
                                                div {
                                                    title: { t!("open-h1") },
                                                    class: "mr-2 cursor-pointer rounded-lg border-ui-border-base border hover:bg-gray-100 w-48 h-[5.2rem] px-3 py-2",
                                                    h3 { class: "text-xl text-black", { t!("results-h1") } }
                                                    small { class: "text-xs text-ui-fg-muted", { t!("details-h1") } }
                                                }
                                            }
                                        }
                                        if let Some(ref hplc_url) = product.analysis_url_hplc {
                                            a {
                                                href: "{hplc_url}",
                                                target: "_blank",
                                                div {
                                                    title: { t!("open-hplc") },
                                                    class: "mr-2 cursor-pointer rounded-lg border-ui-border-base border hover:bg-gray-100 w-48 h-[5.2rem] px-3 py-2",
                                                    h3 { class: "text-xl text-black", { t!("results-hplc") } }
                                                    small { class: "text-xs text-ui-fg-muted", { t!("details-hplc") } }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {
                                        p { class: "text-ui-fg-subtle", { t!("no-results-posted") } }
                                    }
                                }
                            }
                        }

                        /*

                        // Solution Details for specific product types
                        if product.product_form == ProductForm::Solution {
                            if let Some(ref subtitle) = product.subtitle {
                                if subtitle.to_lowercase().ends_with("mg/ml") {
                                    hr { class: "mb-4 mt-8" }
                                    h2 { class: "mb-4 mt-6", { t!("physical-details") } }
                                    div {
                                        class: "flex gap-2",
                                        div {
                                            class: "rounded-md flex border-ui-border-base border w-[18rem]",
                                            img {
                                                alt: "dropper",
                                                class: "h-[22rem] pl-6 pt-4 pb-4",
                                                src: asset!("/assets/images/dropper_site.svg")
                                            }
                                            div {
                                                span {
                                                    class: "flex mt-12 ml-[-2rem]",
                                                    p { class: "text-xl", "{subtitle.to_lowercase().split(\"mg/ml\").next().unwrap_or(\"\")}" }
                                                    p { class: "text-xl pl-1 mt-0", "mg/mL" }
                                                }

                                                {
                                                    if let Ok(mg_value) = subtitle.to_lowercase().split("mg/ml").next().unwrap_or("0").parse::<f64>() {
                                                        rsx! {
                                                            p {
                                                                class: "mt-[92px] cursor-help",
                                                                title: "1mL provides {mg_value} mg of this compound",
                                                                "{mg_value} mg"
                                                            }
                                                            p {
                                                                class: "mt-[5.5px] cursor-help",
                                                                "{mg_value * 0.75} mg"
                                                            }
                                                            p {
                                                                class: "mt-[5.5px] cursor-help",
                                                                "{mg_value * 0.5} mg"
                                                            }
                                                            p {
                                                                class: "mt-[5.5px] cursor-help",
                                                                "{mg_value * 0.25} mg"
                                                            }
                                                        }
                                                    } else {
                                                        rsx! {}
                                                    }
                                                }
                                            }
                                        }
                                        /*
                                        div {
                                            class: "h-full flex-col gap-2 flex",
                                            div {
                                                class: "rounded-md border-ui-border-base border w-auto p-4",
                                                h3 { { t!("solvent ")} }
                                                p { class: "text-ui-fg-subtle", "Liquid, solved in AR Grade PEG-400." }
                                            }
                                            /*
                                            div {
                                                class: "rounded-md border-ui-border-base border w-16 p-4",
                                                "Product"
                                            }
                                            */
                                        }
                                        */
                                    }
                                }
                            }
                        }

                        */

                        // Related Products Section
                        hr { class: "mt-8 mb-6" }
                        div {
                            class: "flex justify-between items-center mb-4",
                            h2 { class: "font-normal font-sans txt-medium text-xl", { t!("relevant-products") } }
                            div {
                                class: "flex gap-2",
                                button {
                                    class: "w-8 h-8 rounded-full border border-gray-300 flex items-center justify-center hover:bg-gray-100 disabled:opacity-50",
                                    onclick: scroll_left,
                                    disabled: *scroll_position.read() <= 0.0,
                                    "←"
                                }
                                button {
                                    class: "w-8 h-8 rounded-full border border-gray-300 flex items-center justify-center hover:bg-gray-100",
                                    onclick: scroll_right,
                                    "→"
                                }
                            }
                        }
                        div {
                            id: "related-products-scroll",
                            class: "overflow-x-auto scroll-smooth",
                            style: "scrollbar-width: none; -ms-overflow-style: none;",
                            ul {
                                class: "flex gap-x-4 md:gap-x-6 pb-6",
                                for product in relevant_products.read().iter() {
                                    li {
                                        // FIXED: Set explicit width constraints for consistent sizing
                                        class: "flex-shrink-0 w-48 md:w-64",
                                        ProductCard {
                                            key: "{product.id}",
                                            product: product.clone(),
                                            top_class: "" // Remove size constraints from here
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
