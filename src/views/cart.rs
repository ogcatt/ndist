use crate::Route;
use dioxus::prelude::*;
use dioxus_i18n::t;
use std::time::Duration;

use crate::backend::cache::use_stale_while_revalidate_with_callback;
use crate::backend::front_entities::*;
use crate::backend::server_functions;
use crate::utils::GLOBAL_CART;
use crate::utils::countries::*;
use crate::utils::*;

use crate::components::SmilesViewer;

// Local helper to map currency code -> symbol
fn currency_code_symbol(_code: Option<&str>) -> &'static str {
    "$"
}

// Joins basket items to product + variant so we can display full detail
#[derive(Clone, PartialEq)]
struct CartLine {
    // Display data
    product: Product,
    variant: ProductVariants,
    // Basket data
    basket_item_id: String,
    quantity: i32,
}

#[component]
pub fn Cart() -> Element {
    // UI state
    let mut open_discount = use_signal(|| false);
    let mut country = use_signal(|| String::new());
    let mut discount_code_temp = use_signal(|| String::new());
    let mut discount_error_line = use_signal(|| String::new());
    let mut discount_added = use_signal(|| false);
    let max_range = 12i32;

    let countries = available_countries_display();

    // Use the stale-while-revalidate hook for basket data

    let basket_signal = use_stale_while_revalidate_with_callback(
        "get_basket",
        || server_functions::get_basket(),
        Duration::from_secs(180),
        |basket| {
            GLOBAL_CART.with_mut(|c| *c = Some(basket.clone()));
            //tracing::info!("Basket temp log for cart: {:#?}", basket.clone());
        },
    );

    /*
    let mut basket_signal = use_resource(move || async move {
        tracing::info!("Getting basket (admin-side)");
        server_functions::get_basket().await
    });

    use_effect(move || {
        if let Some(cart_res) = basket_signal() {
            match cart_res {
                Ok(basket) => {
                    GLOBAL_CART.with_mut(|c| *c = Some(basket.clone()));
                }
                Err(e) => {
                    GLOBAL_CART.with_mut(|c| *c = None);
                    tracing::error!("Failed to load basket: {:?}", e);
                }
            }
        }
    });
    */

    // Products resource (needed for names, thumbnails, prices)
    let mut products_res = use_resource(|| async move { server_functions::get_products().await });

    // Get current country from basket, default to empty string (reactive)
    let current_country = use_memo({
        let basket_signal = basket_signal.clone();
        move || {
            basket_signal()
                .and_then(|basket| basket.country_code.clone())
                .unwrap_or_default()
        }
    });

    // Check if basket is locked
    let is_basket_locked = use_memo({
        let basket_signal = basket_signal.clone();
        move || basket_signal().map(|basket| basket.locked).unwrap_or(false)
    });

    // Computed: join basket items to products/variants
    let mut cart_lines = use_memo({
        let basket_signal = basket_signal.clone();
        let products_res = products_res.clone();
        move || -> Vec<CartLine> {
            let mut lines = Vec::new();

            let basket_opt = basket_signal();

            let products_ref = products_res.read();

            let products_opt = &*products_ref;

            let (Some(basket), Some(products_res)) = (basket_opt.as_ref(), products_ref.as_ref())
            else {
                return lines;
            };

            let Ok(products) = products_res else {
                return lines;
            };

            let items = basket.items.as_ref().map(|v| v.as_slice()).unwrap_or(&[]);

            for bi in items {
                // Find variant and product
                if let Some((product, variant)) =
                    find_product_variant(products, &bi.product_variant_id)
                {
                    lines.push(CartLine {
                        product: product.clone(),
                        variant: variant.clone(),
                        basket_item_id: bi.id.clone(),
                        quantity: bi.quantity,
                    });
                }
            }

            // Sort like Svelte: by "product_title + variant_title"
            lines.sort_by(|a, b| {
                let aa = format!("{}{}", a.product.title, a.variant.variant_name);
                let bb = format!("{}{}", b.product.title, b.variant.variant_name);
                aa.to_lowercase().cmp(&bb.to_lowercase())
            });

            lines
        }
    });

    // Totals
    let mut subtotal = use_memo({
        let cart_lines = cart_lines.clone();
        move || -> f64 {
            let mut sum = cart_lines
                .read()
                .iter()
                .map(|l| l.variant.price_standard_usd * (l.quantity as f64))
                .sum();
            sum
        }
    });

    let mut taxes = use_memo({
        move || -> f64 {
            // Placeholder 0.0 unless you have a tax rule
            0.0
        }
    });

    // Calculate shipping cost and discount in one memo
    let mut shipping_info = use_memo({
        let basket = basket_signal.clone();
        move || -> (f64, Option<f64>) {
            let mut shipping_calc = if let Some(basket) = basket() {
                if let Some(results) = basket.shipping_results {
                    results
                        .into_iter()
                        .map(|so| so.cost_usd)
                        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap_or(0.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let mut shipping_discount = None;

            if let Some(basket) = basket() {
                if let Some(discount_data) = basket.discount {
                    match discount_data.discount_type {
                        DiscountType::PercentageOnShipping => {
                            let shipping_off_pre = (shipping_calc
                                * (discount_data
                                    .discount_percentage
                                    .expect("Could not match percentage shipping type")
                                    / 100.0));
                            shipping_calc = shipping_calc - shipping_off_pre;
                            shipping_discount = Some(shipping_off_pre);
                        }
                        DiscountType::FixedAmountOnShipping => {
                            let shipping_off_pre = discount_data
                                .discount_amount_left
                                .map(|amount| amount.min(shipping_calc))
                                .expect("Could not match amount left shipping type");

                            shipping_calc = shipping_calc - shipping_off_pre;
                            shipping_discount = Some(shipping_off_pre);
                        }
                        _ => {}
                    }
                }
            }

            (shipping_calc, shipping_discount)
        }
    });

    // Extract shipping cost and discount from the combined memo
    let shipping_cost = use_memo(move || shipping_info.read().0);
    let shipping_discount_off = use_memo(move || shipping_info.read().1);

    // Calculate total and discount in one memo
    let mut total_info = use_memo({
        let basket = basket_signal.clone();
        let subtotal = subtotal.clone();
        let taxes = taxes.clone();
        move || -> (f64, Option<f64>) {
            // If there is no basket pricing cannot be calculated
            if basket().is_none() {
                return (0.0, None);
            }
            let mut s = *subtotal.read();
            let t = *taxes.read();
            let mut discount_off = None;

            if let Some(discount_data) = basket().unwrap().discount {
                match discount_data.discount_type {
                    DiscountType::Percentage => {
                        let discount_off_pre = (s
                            * (discount_data
                                .discount_percentage
                                .expect("Could not match percentage type")
                                / 100.0));
                        s = s - discount_off_pre;
                        discount_off = Some(discount_off_pre);
                    }
                    DiscountType::FixedAmount => {
                        let discount_off_pre = discount_data
                            .discount_amount_left
                            .map(|amount| amount.min(s))
                            .expect("Could not match amount left type");

                        s = s - discount_off_pre;
                        discount_off = Some(discount_off_pre);
                    }
                    _ => {}
                }
            }

            (s + t, discount_off)
        }
    });

    // Extract total and discount from the combined memo
    let total = use_memo(move || total_info.read().0);
    let discount_off = use_memo(move || total_info.read().1);

    // Check for pre-order and backorder warnings
    let mut order_warnings = use_memo({
        let cart_lines = cart_lines.clone();
        move || -> (bool, bool) {
            let lines = cart_lines.read();
            let mut has_preorder = false;
            let mut has_backorder = false;

            for line in lines.iter() {
                // Check for pre-order
                if line.product.pre_order {
                    has_preorder = true;
                }

                // Check for backorder
                if line.product.back_order {
                    if let Some(stock_qty) = line.variant.calculated_stock_quantity {
                        if line.quantity > stock_qty {
                            has_backorder = true;
                        }
                    }
                }
            }

            (has_preorder, has_backorder)
        }
    });

    // Find currency (placeholder None -> "$")
    let currency_code: Option<String> = None;

    // Loading state: only false when both resources are Some(Ok(_))
    let loading = {
        // basket_signal returns Option<CustomerBasket>, so None means loading
        let basket_loaded = basket_signal().is_some();

        // products_res is still a Resource, so check if it's loaded and successful
        let products_loaded = {
            let p = products_res.read();
            matches!(p.as_ref(), Some(Ok(_)))
        };

        !(basket_loaded && products_loaded)
    };

    // Actions
    let modify_cart_item = {
        let mut basket_res = basket_signal.clone();
        move |variant_id: String, quantity: i32| async move {
            // Absolute-set quantity, including 0 for removal
            let _ = server_functions::set_basket_item_quantity(variant_id, quantity).await;
            let new_basket = server_functions::get_basket().await;

            match new_basket {
                Ok(b) => {
                    basket_res.set(Some(b.clone()));
                    GLOBAL_CART.with_mut(|c| *c = Some(b.clone()));
                }
                Err(e) => {
                    tracing::info!("Could not restart basket data: {:?}", e);
                }
            }
        }
    };

    let update_country = {
        let mut basket_res = basket_signal.clone();
        move |country_code: String| async move {
            match server_functions::update_basket_country(country_code).await {
                Ok(updated_basket) => {
                    basket_res.set(Some(updated_basket.clone()));
                    GLOBAL_CART.with_mut(|c| *c = Some(updated_basket.clone()));
                }
                Err(e) => {
                    tracing::error!("Failed to update basket country: {:?}", e);
                }
            }
        }
    };

    let update_discount = {
        let mut basket_res = basket_signal.clone();
        let discount_code_temp = discount_code_temp();
        move || async move {
            match server_functions::update_basket_discount(discount_code_temp).await {
                Ok(result) => {
                    // Update the basket regardless
                    basket_res.set(Some(result.basket.clone()));
                    GLOBAL_CART.with_mut(|c| *c = Some(result.basket.clone()));

                    // Check if there was a discount validation error
                    if let Some(discount_error) = result.discount_error {
                        // There was a discount validation error
                        discount_error_line.set(discount_error.to_string());
                        discount_added.set(false);
                    } else {
                        // No error, discount was applied successfully
                        discount_error_line.set("".to_string());
                        discount_added.set(true);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to update basket discount code: {:?}", e);
                    discount_added.set(false);
                    // Handle server errors (database issues, etc.)
                    discount_error_line.set(e.to_string());
                }
            }
        }
    };

    let remove_cart_item = {
        let modify_cart_item = modify_cart_item.clone();
        move |line_id: String| async move {
            modify_cart_item(line_id, 0).await;
        }
    };

    // Skeleton row component
    let skeleton_row = || {
        rsx! {
            tr {
                class: "h-24 md:h-32 bg-ui-bg-base hover:bg-ui-bg-base-hover border-ui-border-base transition-fg border-b [&_td:last-child]:pr-8 [&_th:last-child]:pr-8 [&_td:first-child]:pl-8 [&_th:first-child]:pl-8 w-full",
                // Skeleton thumbnail
                td { class: "h-12 !pl-0 p-4 md:pr-6 md:w-24 w-12",
                    div { class: "flex md:w-24 w-12",
                        div { class: "skeleton-image boop rounded-md relative w-full overflow-hidden p-4 bg-ui-bg-subtle shadow-elevation-card-rest rounded-large aspect-[1/1] border-ui-border-base border" }
                    }
                }
                // Skeleton titles
                td { class: "h-12 pr-3 text-left",
                    div { class: "skeleton mb-2", style: "width: 70%; height: 20px;" }
                    div { class: "skeleton", style: "width: 50%; height: 16px;" }
                }
                // Skeleton quantity
                td { class: "h-12 pr-3",
                    div { class: "flex gap-2 items-center w-28",
                        div { class: "skeleton", style: "width: 24px; height: 24px;" }
                        div { class: "skeleton", style: "width: 60px; height: 40px;" }
                    }
                }
                // Skeleton unit price
                td { class: "h-12 pr-3 hidden md:table-cell",
                    div { class: "skeleton", style: "width: 60px; height: 20px;" }
                }
                // Skeleton line total
                td { class: "h-12 pr-3 !pr-0",
                    div { class: "flex justify-end",
                        div { class: "skeleton", style: "width: 80px; height: 20px;" }
                    }
                }
            }
        }
    };

    rsx! {
        document::Title { { format!("{} - {}", t!("brand"), t!("cart") ) } }
        div {
            // Header
            div {
                class: "w-full cart-head py-6 md:py-10",
                style: "background-color: rgb(247, 247, 247)",
                div { class: "content-container",
                    div { class: "flex mr-auto justify-self-end justify-end",
                        nav { class: "flex absolute h-7", "aria-label": "Breadcrumb",
                            ol { class: "inline-flex items-center space-x-1 md:space-x-2 rtl:space-x-reverse",
                                li { class: "inline-flex items-center",
                                    Link {
                                        to: Route::Home {},
                                        class: "inline-flex items-center text-sm text-ui-fg-muted hover:underline hover:text-gray-950",
                                        svg { class: "w-3 h-3 me-2.5", "aria-hidden": "true", xmlns: "http://www.w3.org/2000/svg", fill: "var(--text-ui-fg-muted)", view_box: "0 0 20 20",
                                            path { d: "m19.707 9.293-2-2-7-7a1 1 0 0 0-1.414 0l-7 7-2 2a1 1 0 0 0 1.414 1.414L2 10.414V18a2 2 0 0 0 2 2h3a1 1 0 0 0 1-1v-4a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v4a1 1 0 0 0 1 1h3a2 2 0 0 0 2-2v-7.586l.293.293a1 1 0 0 0 1.414-1.414Z" }
                                        }
                                        {t!("home")}
                                    }
                                }
                                li {
                                    div { class: "flex items-center",
                                        svg { class: "rtl:rotate-180 w-3 h-3 text-gray-400 mx-1", "aria-hidden": "true", xmlns: "http://www.w3.org/2000/svg", fill: "none", view_box: "0 0 6 10",
                                            path { stroke: "var(--text-ui-fg-muted)", stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "m1 9 4-4-4-4" }
                                        }
                                        div { class: "ms-1 text-sm text-ui-fg-muted md:ms-2",
                                            {t!("cart")}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex justify-start",
                        h1 { class: "text-2xl md:text-3xl font-medium", {t!("cart")} }
                    }
                }
            }

            // Body
            div { class: "content-container mt-8 lg:flex block",
                // Main render decision:
                if loading {
                    // Skeleton UI - render cart structure with skeleton elements
                    // Left: skeleton items
                    div { class: "lg:w-[66%] w-full",
                        table { class: "text-ui-fg-subtle txt-compact-small w-full",
                            thead { class: "border-ui-border-base txt-compact-small-plus [&_tr:hover]:bg-ui-bg-base border-y border-t-0",
                                tr { class: "bg-ui-bg-base hover:bg-ui-bg-base-hover border-ui-border-base transition-fg border-b [&_td:last-child]:pr-8 [&_th:last-child]:pr-8 [&_td:first-child]:pl-8 [&_th:first-child]:pl-8 text-ui-fg-subtle text-bbase",
                                    th { class: "h-12 pr-3 text-left !pl-0", {t!("item")} }
                                    th { class: "h-12 pr-3 text-left" }
                                    th { class: "h-12 pr-3 text-left", {t!("quantity")} }
                                    th { class: "h-12 pr-3 text-left hidden md:table-cell", {t!("price")} }
                                    th { class: "h-12 pr-3 !pr-0 text-right", {t!("total")} }
                                }
                            }
                            tbody { class: "border-ui-border-base border-b",
                                // Render 2 skeleton rows
                                {skeleton_row()}
                                {skeleton_row()}
                                //{skeleton_row()}
                            }
                        }
                    }

                    // Right: skeleton summary
                    div { class: "lg:flex-grow mt-8 lg:mt-0 lg:ml-20",
                        h1 { class: "text-2xl md:text-2xl font-medium", {t!("summary")} }
                        div { class: "skeleton mt-3", style: "width: 60%; height: 16px;" }
                        div { class: "h-px w-full border-b border-gray-200 mt-4 mb-4" }
                        div {
                            div { class: "flex flex-col gap-y-2 text-bbase text-ui-fg-subtle",
                                // Skeleton subtotal
                                div { class: "flex items-center justify-between",
                                    span { {t!("subtotal")} }
                                    div { class: "skeleton", style: "width: 80px; height: 20px;" }
                                }
                                // Skeleton shipping
                                div { class: "flex items-center justify-between",
                                    span { {t!("shipping")} }
                                    div { class: "skeleton", style: "width: 80px; height: 20px;" }
                                }
                                /*
                                // Skeleton taxes
                                div { class: "flex justify-between",
                                    span { {t!("taxes")} }
                                    div { class: "skeleton", style: "width: 60px; height: 20px;" }
                                }
                                */
                            }
                            div { class: "h-px w-full border-b border-gray-200 my-4" }
                            div { class: "flex items-center justify-between text-ui-fg-base mb-2 txt-medium",
                                span { {t!("total")} }
                                div { class: "skeleton", style: "width: 100px; height: 24px;" }
                            }
                            div { class: "h-px w-full border-b border-gray-200 mt-4" }
                        }
                        // Skeleton checkout button
                        div { class: "skeleton mt-4", style: "width: 100%; height: 40px; border-radius: 6px;" }
                    }
                } else if !cart_lines.read().is_empty() {
                    // Left: items
                    div {
                        class: format!("lg:w-[66%] w-full {}", if is_basket_locked() { "relative" } else { "" }),

                        // Locked basket overlay for items table
                        if is_basket_locked() {
                            div {
                                class: "absolute inset-0 bg-white bg-opacity-75 z-10 flex items-center justify-center",
                                div {
                                    class: "text-center p-4 bg-white border rounded-md shadow-md max-w-sm",
                                    p {
                                        class: "text-sm text-gray-600",
                                        {t!("open-payment-warning")}
                                    }
                                }
                            }
                        }

                        table {
                            class: format!("text-ui-fg-subtle txt-compact-small w-full {}", if is_basket_locked() { "opacity-50" } else { "" }),
                            thead { class: "border-ui-border-base txt-compact-small-plus [&_tr:hover]:bg-ui-bg-base border-y border-t-0",
                                tr { class: "bg-ui-bg-base hover:bg-ui-bg-base-hover border-ui-border-base transition-fg border-b [&_td:last-child]:pr-8 [&_th:last-child]:pr-8 [&_td:first-child]:pl-8 [&_th:first-child]:pl-8 text-ui-fg-subtle text-bbase",
                                    th { class: "h-12 pr-3 text-left !pl-0", {t!("item")} }
                                    th { class: "h-12 pr-3 text-left" }
                                    th { class: "h-12 pr-3 text-left", {t!("quantity")} }
                                    th { class: "h-12 pr-3 text-left hidden md:table-cell", {t!("price")} }
                                    th { class: "h-12 pr-3 !pr-0 text-right", {t!("total")} }
                                }
                            }
                            tbody { class: "border-ui-border-base border-b",
                                {
                                    let lines = cart_lines.read().clone();
                                    let is_locked = is_basket_locked();
                                    rsx! {
                                        for item in lines.into_iter() {
                                            tr {
                                                key: "{item.basket_item_id.clone()}",
                                                class: "h-24 md:h-32 bg-ui-bg-base hover:bg-ui-bg-base-hover border-ui-border-base transition-fg border-b [&_td:last-child]:pr-8 [&_th:last-child]:pr-8 [&_td:first-child]:pl-8 [&_th:first-child]:pl-8 w-full",
                                                // Thumb
                                                td { class: "h-12 !pl-0 p-4 md:pr-6 md:w-24 w-12",
                                                    Link {
                                                        title: format!("{} {}", t!("visit"), item.product.title),
                                                        class: "flex md:w-24 w-12",
                                                        to: Route::ProductPage { handle: item.product.handle.clone() },
                                                        div { class: "boop rounded-md relative w-full overflow-hidden p-4 bg-ui-bg-subtle shadow-elevation-card-rest rounded-large group-hover:shadow-elevation-card-hover transition-shadow ease-in-out duration-150 aspect-[1/1] border-ui-border-base border",
                                                            if let Some(ref url) = item.variant.thumbnail_url {
                                                                img {
                                                                    alt: "Thumbnail",
                                                                    draggable: "false",
                                                                    loading: "lazy",
                                                                    decoding: "async",
                                                                    class: "absolute inset-0 object-cover object-center w-full",
                                                                    style: "position: absolute; height: 100%; width: 100%; inset: 0px; color: transparent;",
                                                                    src: url.clone(),
                                                                }
                                                            } else if let Some(ref smiles) = item.product.smiles {
                                                                div {
                                                                    class: "absolute inset-0 flex items-center justify-center",
                                                                    style: "overflow: hidden;",

                                                                    div {
                                                                        class: "w-full h-full flex items-center justify-center",
                                                                        style: "max-width: 100%; max-height: 100%;",

                                                                        SmilesViewer {
                                                                            smiles: smiles.clone()
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                // Titles
                                                td { class: "h-12 pr-3 text-left",
                                                    Link {
                                                        class: "hover:underline",
                                                        to: Route::ProductPage { handle: item.product.handle.clone() },
                                                        p { class: "font-normal font-sans txt-medium txt-medium-plus text-ui-fg-base",
                                                            {item.product.title.clone()}
                                                            if !item.product.title.contains("(") {
                                                                " "
                                                                span { { format!("({})", item.product.product_form) } }
                                                            }
                                                        }
                                                    }
                                                    p { class: "font-normal font-sans txt-medium inline-block txt-medium text-ui-fg-subtle w-full overflow-hidden text-ellipsis text-sm",
                                                        if item.variant.variant_name != "Default option value" {
                                                            {format!("{}: {}", t!("variant"), item.variant.variant_name)}
                                                        } else {
                                                            if let Some(ref sub) = item.product.subtitle {
                                                                {sub.clone()}
                                                            }
                                                        }
                                                    }
                                                }
                                                // Quantity select + remove
                                                td { class: "h-12 pr-3",
                                                    div { class: "flex gap-2 items-center w-28",
                                                        div { class: "flex items-center justify-between text-small-regular",
                                                            button {
                                                                title: t!("remove-from-cart"),
                                                                class: format!("flex gap-x-1 text-ui-fg-subtle hover:text-ui-fg-base {}", if is_locked { "cursor-not-allowed opacity-50" } else { "cursor-pointer" }),
                                                                disabled: is_locked,
                                                                onclick: {
                                                                    let variant_id = item.variant.id.clone();
                                                                    move |_| {
                                                                        if !is_locked {
                                                                            spawn({
                                                                                let modify_cart_item = modify_cart_item.clone();
                                                                                let variant_id = variant_id.clone();
                                                                                async move { modify_cart_item(variant_id, 0).await; }
                                                                            });
                                                                        }
                                                                    }
                                                                },
                                                                svg { xmlns: "http://www.w3.org/2000/svg", width: "20", height: "20", fill: "none",
                                                                    path { stroke: "currentColor", stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "1.5", d: "m12.283 7.5-.288 7.5m-3.99 0-.288-7.5m8.306-2.675c.285.043.569.09.852.138m-.852-.137-.89 11.568a1.875 1.875 0 0 1-1.87 1.73H6.737a1.875 1.875 0 0 1-1.87-1.73l-.89-11.569m12.046 0a40.08 40.08 0 0 0-2.898-.33m-10 .467c.283-.049.567-.095.852-.137m0 0a40.091 40.091 0 0 1 2.898-.33m6.25 0V3.73c0-.984-.758-1.804-1.742-1.834a43.3 43.3 0 0 0-2.766 0c-.984.03-1.742.851-1.742 1.834v.763m6.25 0c-2.08-.160-4.17-.160-6.25 0" }
                                                                }
                                                                span {}
                                                            }
                                                        }
                                                        div {
                                                            span { class: format!("w-16 lg:w-16 bg-ui-tag-neutral-bg [&_svg]:text-ui-tag-neutral-icon border-ui-tag-neutral-border justify-center overflow-hidden rounded-md relative flex items-center text-sm border text-ui-fg-base group h-10 {}", if is_locked { "opacity-50" } else { "" }),
                                                                select {
                                                                    class: "hover:bg-ui-bg-field-hover w-full appearance-none bg-transparent border-none transition-colors duration-150 focus:border-gray-700 outline-none h-16 items-center justify-center p-4",
                                                                    value: item.quantity.to_string(),
                                                                    disabled: is_locked,
                                                                    oninput: {
                                                                        let variant_id = item.variant.id.clone();
                                                                        move |evt: FormEvent| {
                                                                            if !is_locked {
                                                                                if let Ok(v) = evt.value().parse::<i32>() {
                                                                                    spawn({
                                                                                        let modify_cart_item = modify_cart_item.clone();
                                                                                        let variant_id = variant_id.clone();
                                                                                        async move { modify_cart_item(variant_id, v).await; }
                                                                                    });
                                                                                }
                                                                            }
                                                                        }
                                                                    },
                                                                    for num in 1..=max_range {
                                                                        option {
                                                                            value: num.to_string(),
                                                                            selected: item.quantity == num,
                                                                            {num.to_string()}
                                                                        }
                                                                    }
                                                                }

                                                                span { class: "absolute flex pointer-events-none justify-end w-full pr-2 group-hover:animate-pulse",
                                                                    svg { width: "16", height: "16", view_box: "0 0 16 16", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                                                                        path { d: "M4 6L8 10L12 6", stroke: "currentColor", stroke_width: "1.5", stroke_linecap: "round", stroke_linejoin: "round" }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                // Unit price
                                                td { class: "h-12 pr-3 hidden md:table-cell",
                                                    div { title: t!("price-per-item"), class: "flex flex-col text-ui-fg-muted justify-center h-full",
                                                        span { class: "text-base-regular",
                                                            {format!("{}{}", currency_code_symbol(currency_code.as_deref()), format_number(item.variant.price_standard_usd))}
                                                        }
                                                    }
                                                }
                                                // Line total
                                                td { class: "h-12 pr-3 !pr-0",
                                                    span {
                                                        div { class: "flex flex-col gap-x-2 text-ui-fg-subtle items-end",
                                                            div { class: "text-left",
                                                                span { class: "text-base-regular",
                                                                    {
                                                                        let line_total = item.variant.price_standard_usd * (item.quantity as f64);
                                                                        format!("{}{}", currency_code_symbol(currency_code.as_deref()), format_number(line_total))
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
                        }
                    }

                    // Right: summary
                    div {
                        class: format!("lg:w-[34%] mt-8 lg:mt-0 lg:ml-20 {}", if is_basket_locked() { "relative" } else { "" }),

                        // Locked basket overlay for summary
                        if is_basket_locked() {
                            div {
                                class: "absolute inset-0 bg-white bg-opacity-75 z-10 flex items-start justify-center pt-20",
                                div {
                                    class: "text-center p-4 bg-white border rounded-md shadow-md max-w-sm",
                                    p {
                                        class: "text-sm text-gray-600",
                                        {t!("open-payment-warning")}
                                    }
                                }
                            }
                        }

                        div {
                            class: if is_basket_locked() { "opacity-50 pointer-events-none" } else { "" },
                            h1 { class: "text-2xl md:text-2xl font-medium", {t!("summary")} }

                            // Only show discount section if basket is not locked
                            if !is_basket_locked() {
                                p {
                                    class: "text-sm text-ui-fg-interactive my-3 hover:underline cursor-pointer",
                                    onclick: move |_| { open_discount.set(!open_discount()); },
                                    {t!("add-discount-or-gift")}
                                }
                                if open_discount() {
                                    div { class: "my-1",
                                        div {
                                            label { class: "sr-only", r#for: "discount", {t!("enter-code")} }
                                            input {
                                                id: "discount",
                                                class: "w-full border rounded-md p-2",
                                                placeholder: t!("enter-code"),
                                                value: "{discount_code_temp}",
                                                oninput: move |event: FormEvent| discount_code_temp.set(event.value().to_uppercase())
                                            }
                                            if !discount_error_line().is_empty() {
                                                p {
                                                    class: "text-red-600 text-sm mt-2",
                                                    { discount_error_line() }
                                                }
                                            } else if discount_added() {
                                                p {
                                                    class: "text-green-600 text-sm mt-2",
                                                    { t!("discount-added") }
                                                }
                                            }
                                            button {
                                                id: "discount_submit_button",
                                                onclick: move |_| {
                                                    spawn({
                                                        let update_discount = update_discount.clone();
                                                        async move {update_discount().await;}
                                                    });
                                                },
                                                class: "mt-4 hover:bg-zinc-800 bg-blend transition-fg relative inline-flex items-center justify-center overflow-hidden rounded-md outline-none disabled:bg-ui-bg-disabled disabled:border-ui-border-base disabled:text-ui-fg-disabled disabled:shadow-buttons-neutral disabled:after:hidden after:transition-fg after:absolute after:inset-0 after:content-[''] shadow-buttons-inverted text-ui-fg-on-inverted bg-ui-button-inverted after:button-inverted-gradient hover:bg-ui-button-inverted-hover hover:after:button-inverted-hover-gradient active:bg-ui-button-inverted-pressed active:after:button-inverted-pressed-gradient focus:!shadow-buttons-inverted-focus txt-compact-small-plus gap-x-1.5 px-3 py-1.5 w-full h-10",
                                                {t!("submit")}
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "h-px w-full border-b border-gray-200 mt-4 mb-4" }
                            div {
                                div { class: "flex flex-col gap-y-2 text-bbase text-ui-fg-subtle",
                                    div { class: "flex items-center justify-between",
                                        span { class: "flex gap-x-1 items-center",
                                            {t!("subtotal")}
                                            div { title: t!("total-before-shipping"),
                                                svg { xmlns: "http://www.w3.org/2000/svg", width: "20", height: "20", fill: "none", "data-state": "closed",
                                                    path { fill: "gray", fill_rule: "evenodd", d: "M18 10a8 8 0 1 1-16.001 0A8 8 0 0 1 18 10Zm-7-4a1 1 0 1 1-2 0 1 1 0 0 1 2 0ZM9 9a.75.75 0 0 0 0 1.5h.253a.25.25 0 0 1 .244.304l-.459 2.066A1.75 1.75 0 0 0 10.747 15H11a.75.75 0 1 0 0-1.5h-.253a.25.25 0 0 1-.244-.304l.459-2.066A1.75 1.75 0 0 0 9.253 9H9Z", clip_rule: "evenodd" }
                                                }
                                            }
                                        }
                                        span { {format!("{}{}", currency_code_symbol(currency_code.as_deref()), format_number(*subtotal.read()))} }
                                    }
                                    div { class: "flex items-center justify-between",
                                        {
                                            if let Some(basket) = basket_signal() {
                                                if let Some(discount_data) = basket.discount {
                                                    if let Some(discount_code) = basket.discount_code {
                                                        match discount_data.discount_type {
                                                            DiscountType::PercentageOnShipping
                                                            | DiscountType::FixedAmountOnShipping => {
                                                                rsx! {
                                                                    span { {
                                                                        t!("shipping-discount-code", code: discount_code)
                                                                    } }
                                                                }
                                                            },
                                                            _ => {
                                                                rsx! {
                                                                    span { {t!("shipping")} }
                                                                }
                                                            }
                                                        }

                                                    } else {
                                                        rsx! {
                                                            span { {t!("shipping")} }
                                                        }
                                                    }
                                                } else {
                                                    rsx! {
                                                        span { {t!("shipping")} }
                                                    }
                                                }

                                            } else {
                                                rsx! {
                                                    span { {t!("shipping")} }
                                                }
                                            }
                                        }
                                        {
                                            if let Some(basket) = basket_signal() {
                                                if let Some(results) = basket.shipping_results {
                                                    rsx! {
                                                        span { title: t!("to-be-definite"),
                                                            {
                                                                format!(
                                                                    "~{}{}{}",
                                                                    currency_code_symbol(currency_code.as_deref()),
                                                                    format_number(shipping_cost()),
                                                                    if let Some(discount_o) = basket.discount {
                                                                        match discount_o.discount_type {
                                                                            DiscountType::PercentageOnShipping => {
                                                                                format!(" ({})",
                                                                                    t!("num-off",
                                                                                        num: format!(
                                                                                            "{}%",
                                                                                            format_float(discount_o.discount_percentage)
                                                                                        )
                                                                                    )
                                                                                )
                                                                            },
                                                                            DiscountType::FixedAmountOnShipping => {
                                                                                format!(" (-{}{})", currency_code_symbol(currency_code.as_deref()), format_number(shipping_discount_off().unwrap_or(0.0)))
                                                                            },
                                                                            _ => { "".to_string() },
                                                                        }
                                                                    } else {
                                                                        "".to_string()
                                                                    }
                                                                )
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    rsx! {
                                                        span { title: t!("to-be-determined"), "TBD" }
                                                    }
                                                }
                                            } else {
                                                rsx! {
                                                    span { title: t!("to-be-determined"), "TBD" }
                                                }
                                            }
                                        }

                                    }
                                    {
                                        if let Some(basket) = basket_signal() {
                                            if let Some(discount_o) = basket.discount {
                                                if let Some(discount_off) = discount_off() && (discount_o.discount_type == DiscountType::Percentage || discount_o.discount_type == DiscountType::FixedAmount ) {
                                                    rsx! {
                                                        div { class: "flex items-center justify-between",
                                                            span { {t!("discount-code", code: basket.discount_code.unwrap_or("?".to_string()))} }

                                                            span {
                                                                class: "text-blue-600",
                                                                {
                                                                    format!(
                                                                        "-{}{:.2}{}",
                                                                        currency_code_symbol(currency_code.as_deref()),
                                                                        discount_off,
                                                                        match discount_o.discount_type {
                                                                            DiscountType::Percentage => {
                                                                                format!(" ({})",
                                                                                    t!("num-off",
                                                                                        num: format!(
                                                                                            "{}%",
                                                                                            format_float(discount_o.discount_percentage)
                                                                                        )
                                                                                    )
                                                                                )
                                                                            },
                                                                            DiscountType::FixedAmount => { "".to_string() },
                                                                            _ => { "".to_string() },
                                                                            DiscountType::FixedAmountOnShipping => { "".to_string() }
                                                                        }
                                                                    )
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
                                    {
                                        if let Some(basket) = basket_signal() {
                                            if let Some(discount_o) = basket.discount {
                                                if discount_o.discount_auto_apply {
                                                    rsx ! {
                                                        p {
                                                            class: "text-[10px] text-gray-500",
                                                            { t!("discount-applied-auto") }
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
                                    /*
                                    div { class: "flex justify-between",
                                        span { class: "flex gap-x-1 items-center", {t!("taxes")} }
                                        span { {format!("{}{}", currency_code_symbol(currency_code.as_deref()), format_number(*taxes.read()))} }
                                    }
                                    */
                                }
                                div { class: "h-px w-full border-b border-gray-200 my-4" }
                                div { class: "flex items-center justify-between text-ui-fg-base mb-2 txt-medium",
                                    span { {t!("total")} }
                                    span {
                                        class: "txt-xlarge-plus",
                                        {
                                            let base_total = *total.read();
                                            let shipping_cost = *shipping_cost.read();

                                            let final_total = base_total + shipping_cost;

                                            format!("~{}{}",
                                                currency_code_symbol(currency_code.as_deref()),
                                                format_number(final_total)
                                            )
                                        }
                                    }
                                }
                                div { class: "h-px w-full border-b border-gray-200 mt-4" }
                            }

                            // Country select - disabled if locked
                            div { class: "flex flex-col col-span-2 w-full",
                                        div {
                                            class: format!("mt-4 relative flex items-center text-base-regular border border-typical bg-ui-bg-subtle rounded-md hover:bg-ui-bg-field-hover {}", if is_basket_locked() { "opacity-50" } else { "" }),
                                            select {
                                                class: "appearance-none flex-1 bg-transparent border-none px-3 py-1.5 transition-colors duration-150 outline-none",
                                                value: current_country.read().clone(),
                                                disabled: is_basket_locked(),
                                                onchange: move |event: FormEvent| {
                                                    if !is_basket_locked() {
                                                        let new_country = event.value();
                                                        spawn({
                                                            let update_country = update_country.clone();
                                                            async move {
                                                                update_country(new_country).await;
                                                            }
                                                        });
                                                    }
                                                },
                                                // Add a default empty option
                                                option {
                                                    value: "",
                                                    selected: current_country.read().is_empty(),
                                                    "Select country to continue..."
                                                }
                                                for (iso, display) in countries.iter() {
                                                    option {
                                                        value: iso.clone(),
                                                        selected: *current_country.read() == *iso,
                                                        key: "{iso}",
                                                        { display.clone() }
                                                    }
                                                }
                                            }
                                            span { class: "absolute flex pointer-events-none justify-end w-full pr-2 group-hover:animate-pulse",
                                                svg { width: "16", height: "16", view_box: "0 0 16 16", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                                                    path { d: "M4 6L8 10L12 6", stroke: "currentColor", stroke_width: "1.5", stroke_linecap: "round", stroke_linejoin: "round" }
                                                }
                                            }
                                        }
                                    }
                        }

                        if current_country() == "US" {
                            p {
                                class: "text-[12px] text-gray-500 mt-2",
                                { t!("us-orders-discount-notice") }
                            }
                        }

                        // Warning messages for pre-orders and backorders
                        {
                            let (has_preorder, has_backorder) = *order_warnings.read();
                            rsx! {
                                if has_preorder {
                                    div {
                                        class: "mt-4 p-3 border border-blue-300 bg-blue-50 text-blue-800 rounded-md text-sm",
                                        { t!("contains-preorder") }
                                    }
                                }
                                if has_backorder {
                                    div {
                                        class: "mt-4 p-3 border border-blue-300 bg-blue-50 text-blue-800 rounded-md text-sm",
                                        { t!("contains-preorder") }
                                    }
                                }
                            }
                        }

                        // Checkout button - different text and behavior when locked
                        if is_basket_locked() {
                            Link { to: Route::Checkout {},
                                button {
                                    id: "cart_mod_button",
                                    class: "mt-4 hover:bg-zinc-800 z-20 bg-blend transition-fg relative inline-flex items-center justify-center overflow-hidden rounded-md outline-none disabled:bg-ui-bg-disabled disabled:border-ui-border-base disabled:text-ui-fg-disabled disabled:shadow-buttons-neutral disabled:after:hidden after:transition-fg after:absolute after:inset-0 after:content-[''] shadow-buttons-inverted text-ui-fg-on-inverted bg-ui-button-inverted after:button-inverted-gradient hover:bg-ui-button-inverted-hover hover:after:button-inverted-hover-gradient active:bg-ui-button-inverted-pressed active:after:button-inverted-pressed-gradient focus:!shadow-buttons-inverted-focus txt-compact-small-plus gap-x-1.5 px-3 py-1.5 w-full h-10",
                                    {t!("visit-payment")}
                                }
                            }
                        } else {
                            Link { to: Route::Checkout {},
                                button {
                                    id: "cart_mod_button",
                                    class: "mt-4 hover:bg-zinc-800 bg-blend transition-fg relative inline-flex items-center justify-center overflow-hidden rounded-md outline-none disabled:bg-ui-bg-disabled disabled:border-ui-border-base disabled:text-ui-fg-disabled disabled:shadow-buttons-neutral disabled:after:hidden after:transition-fg after:absolute after:inset-0 after:content-[''] shadow-buttons-inverted text-ui-fg-on-inverted bg-ui-button-inverted after:button-inverted-gradient hover:bg-ui-button-inverted-hover hover:after:button-inverted-hover-gradient active:bg-ui-button-inverted-pressed active:after:button-inverted-pressed-gradient focus:!shadow-buttons-inverted-focus txt-compact-small-plus gap-x-1.5 px-3 py-1.5 w-full h-10",
                                    {t!("checkout")}
                                }
                            }
                        }
                    }
                } else {
                    // Empty cart (only after both resources finished and joined lines are empty)
                    div { class: "",
                        /*h1 { class: "text-2xl md:text-3xl font-medium", {t!("cart")} }*/
                        p { class: "text-ui-fg-subtle mt-4", {t!("cart-empty-msg")} }
                        Link { class: "mt-4 text-bbase flex gap-x-1 items-center group", to: Route::Collection { codename: String::from("all") },
                            p { class: "font-normal font-sans txt-medium text-ui-fg-interactive", {t!("explore-products")} }
                            svg { xmlns: "http://www.w3.org/2000/svg", width: "20", height: "20", fill: "none", class: "group-hover:rotate-45 ease-in-out duration-200",
                                path { stroke: "var(--text-ui-fg-interactive)", stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "1.5", d: "m5.75 14.25 8.5-8.5m0 0h-7.5m7.5 0v7.5" }
                            }
                        }
                    }
                }
            }
        }

        // style { [include_str!("cart.css")] }
    }
}

// Helpers

fn find_product_variant<'a>(
    products: &'a Vec<Product>,
    variant_id: &str,
) -> Option<(&'a Product, &'a ProductVariants)> {
    for p in products {
        if let Some(ref vars) = p.variants {
            if let Some(v) = vars.iter().find(|v| v.id == variant_id) {
                return Some((p, v));
            }
        }
    }
    None
}
