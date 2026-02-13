#![allow(non_snake_case)]

use crate::Route;
use crate::backend::front_entities::*;
use dioxus::prelude::*;
use dioxus_i18n::t;
use crate::backend::server_functions::get_session_info;
use crate::backend::cache::use_stale_while_revalidate;
use std::time::Duration;

use crate::components::SmilesViewer;

#[component]
pub fn ProductCard(
    #[props(default = None)] product: Option<Product>,
    #[props(default = "")] top_class: &'static str,
    #[props(default = false)] loading: bool,
) -> Element {
    let mut image_loaded = use_signal(|| false);

    // If loading or no product, render skeleton
    if loading || product.is_none() {
        return rsx! {

            div {
                class: "group",

                div {
                    class: "{top_class} boop rounded-lg relative overflow-hidden p-4 bg-ui-bg-subtle shadow-elevation-card-rest rounded-large aspect-square w-full border-gray-200 border",

                    // Skeleton image
                    div {
                        class: "skeleton-image absolute inset-0"
                    }
                }

                div {
                    class: "flex txt-compact-medium mt-4 justify-between",

                    // Skeleton title (60% width)
                    div {
                        class: "skeleton",
                        style: "width: 60%; height: 20px;"
                    }

                    // Skeleton price (15% width)
                    div {
                        class: "skeleton",
                        style: "width: 15%; height: 20px;"
                    }
                }

                // Skeleton subtitle (20% width)
                div {
                    class: "skeleton mt-2",
                    style: "width: 20%; height: 16px;"
                }
            }
        };
    }

    // Unwrap the product since we know it exists at this point
    let product = product.unwrap();

    // Check if product is gated (has access_groups)
    let is_gated = product.access_groups.as_ref().map_or(false, |groups| !groups.is_empty());

    // Use cached session info for faster access checks
    let session_signal = use_stale_while_revalidate(
        "session_info",
        || async { get_session_info().await },
        Duration::from_secs(60),
    );

    let session_state = session_signal.read();

    // Check if user has access based on session group membership
    let user_has_access = if is_gated {
        if let Some(session) = session_state.as_ref() {
            // User has access if any of their group IDs match the product's access groups
            product.access_groups
                .as_ref()
                .map_or(false, |access_groups| {
                    session.group_ids.iter().any(|group_id| access_groups.contains(group_id))
                })
        } else {
            // No session - no access to gated product
            false
        }
    } else {
        // Product is not gated - everyone has access
        true
    };

    // Utility function for currency symbol
    let currency_symbol = |_currency_code: &str| -> &str {
        "$" // Assuming USD since price_standard_usd suggests USD
    };

    // Get thumbnail from variants (prioritize default variant)
    let thumbnail_url = if let Some(variants) = &product.variants {
        if let Some(default_variant_id) = &product.default_variant_id {
            variants
                .iter()
                .find(|v| &v.id == default_variant_id)
                .and_then(|v| v.thumbnail_url.as_ref())
                .or_else(|| variants.first().and_then(|v| v.thumbnail_url.as_ref()))
        } else {
            variants.first().and_then(|v| v.thumbnail_url.as_ref())
        }
    } else {
        None
    };

    // Check if out of stock based on variant stock quantities
    let is_out_of_stock = product.force_no_stock
        || product
            .variants
            .as_ref()
            .map(|variants| {
                variants.is_empty()
                    || variants
                        .iter()
                        .all(|v| v.calculated_stock_quantity.unwrap_or(0) <= 0)
            })
            .unwrap_or(true);

    // Helper function to format price without unnecessary .00
    fn format_price(price: f64) -> String {
        let formatted = format!("${:.2}", price);
        if formatted.ends_with(".00") {
            formatted.trim_end_matches(".00").to_string()
        } else {
            formatted
        }
    }

    // Check if any variant has a discount
    let has_discount = product.variants.as_ref().map_or(false, |variants| {
        variants.iter().any(|v| v.price_standard_without_sale.is_some())
    });

    // Calculate price display info
    let (price_text, original_price_text, is_single_variant) = if let Some(variants) = &product.variants {
        if variants.is_empty() {
            ("N/A".to_string(), None, false)
        } else if variants.len() == 1 {
            let variant = &variants[0];
            let current_price = format_price(variant.price_standard_usd);
            let original_price = variant.price_standard_without_sale.map(format_price);
            (current_price, original_price, true)
        } else {
            let prices: Vec<f64> = variants.iter().map(|v| v.price_standard_usd).collect();
            let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

            let price_range = if (min_price - max_price).abs() < 0.01 {
                format_price(min_price)
            } else {
                format!("{} — {}", format_price(min_price), format_price(max_price))
            };
            (price_range, None, false)
        }
    } else {
        ("N/A".to_string(), None, false)
    };

    // Format title with product form
    let display_title = if product.title.contains("(") {
        product.title.clone()
    } else {
        format!(
            "{} ({})",
            product.title,
            product.product_form.to_frontend_string()
        )
    };

    // Determine if this should be a clickable link or just a div
    let card_content = rsx! {
        div {
            // Apply different classes based on access
            class: if user_has_access { "{top_class} boop rounded-lg relative overflow-hidden p-4 bg-ui-bg-subtle shadow-elevation-card-rest rounded-large group-hover:shadow-elevation-card-hover transition-shadow ease-in-out duration-150 aspect-square w-full border-gray-200 border" } else { "{top_class} rounded-lg relative overflow-hidden p-4 bg-ui-bg-subtle shadow-elevation-card-rest rounded-large aspect-square w-full border-gray-200 border cursor-not-allowed" },

            // Pre-order indicator (shows if pre_order is true, regardless of stock)
            if product.pre_order {
                div {
                    title: "Available for Pre-order",
                    style: "z-index: 3",
                    class: "rounded-md flex py-0.5 px-1 text-xs absolute right-1.5 top-2 bg-white border-ui-border-base border text-black",

                    div {
                        class: "mr-1 bg-[#34c9c9]",
                        style: "border-radius: 50%;height: 12px; width: 12px;margin-top: 2px;"
                    }
                    span {
                        class: "text-ui-fg-subtle",
                        { t!("pre-order") }
                    }
                }
            }

            // Out of stock indicator (only shows if not pre-order, or shows with gray styling if pre-order)
            if is_out_of_stock {
                div {
                    title: "Out of Stock in all locations",
                    style: if product.pre_order { "z-index: 2; top: 44px;" } else { "z-index: 2" },
                    class: "rounded-md flex py-0.5 mt-[-8px] px-1 text-xs absolute right-1.5 bg-white border-typical border text-black",

                    div {
                        class: if product.pre_order { "mr-1 bg-gray-400 mt-[-8px]" } else { "mr-1 bg-orange-400" },
                        style: "border-radius: 50%;height: 12px; width: 12px;margin-top: 2px;"
                    }
                    span {
                        class: "text-ui-fg-subtle",
                        { t!("sold-out") }
                    }
                }
            }

            // Mechanism indicator (shows on hover at bottom right)
            if let Some(mechanism) = &product.mechanism {
                div {
                    title: "Mechanism of Action",
                    class: "rounded-md flex py-0.5 px-1 text-xs absolute right-1.5 bg-white border-ui-border-base border text-black opacity-0 group-hover:opacity-100 transition-opacity duration-200",
                    style: "z-index: 3; bottom: 0.375rem;",

                    span {
                        class: "text-ui-fg-subtle",
                        "{mechanism}"
                    }
                }
            }

            // Members-only overlay for gated products without access
            if is_gated && !user_has_access {
                div {
                    class: "absolute inset-0 bg-white bg-opacity-0 group-hover:bg-opacity-50 transition-all duration-200 flex items-center justify-center",
                    style: "z-index: 4;",

                    div {
                        class: "opacity-0 group-hover:opacity-100 transition-opacity duration-200 text-center px-4",
                        p {
                            class: "text-ui-fg-base font-medium text-sm",
                            "This product is for members-only"
                        }
                    }
                }
            }

            // Product image or SMILES viewer
            if let Some(img_url) = thumbnail_url {
                img {
                    alt: format!("{} {}", product.title, t!("thumbnail")),
                    class: format!("absolute inset-0 object-cover object-center fade-in-image {}",
                        if image_loaded() { "loaded" } else { "" }),
                    loading: "lazy",
                    draggable: "false",
                    crossorigin: "anonymous",
                    decoding: "async",
                    style: "position:absolute;height:100%;width:100%;left:0;top:0;right:0;bottom:0;color:transparent",
                    src: "{img_url}",
                    onload: move |_| {
                        image_loaded.set(true);
                    }
                }
            } else if let Some(smiles) = product.smiles {
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

        div {
            class: "flex txt-compact-medium mt-4 justify-between",

            div {
                class: "flex items-center",

                // Lock icon for gated products
                if is_gated {
                    if user_has_access {
                        img {
                            class: "mr-2",
                            src: asset!("/assets/icons/lock-open-outline.svg"),
                            style: "height:16px;",
                            title: "You have access to this product"
                        }
                    } else {
                        img {
                            class: "mr-2",
                            src: asset!("/assets/icons/lock-closed-outline.svg"),
                            style: "height:16px;",
                            title: "Members-only product"
                        }
                    }
                }

                p {
                    class: "prod-title font-normal font-sans txt-medium text-ui-fg-subtle",
                    "{display_title}"
                }
            }

            div {
                class: "flex items-center gap-x-2",

                // Price display with discount support
                div {
                    class: "flex items-center gap-2",

                    // Current price (red if on sale)
                    span {
                        class: if has_discount { "font-normal font-sans txt-medium text-sale-500 pl-1 whitespace-nowrap" } else { "font-normal font-sans txt-medium text-ui-fg-muted pl-1 whitespace-nowrap" },
                        "{price_text}"
                    }

                    // Original price (strikethrough if on sale and single variant)
                    if is_single_variant && has_discount {
                        if let Some(original_price) = original_price_text {
                            span {
                                class: "font-normal font-sans txt-medium text-ui-fg-muted line-through",
                                "{original_price}"
                            }
                        }
                    }
                }
            }
        }

        // Optional subtitle
        if let Some(subtitle) = &product.subtitle {
            p {
                class: "mt-0.5 md:mt-0 text-sm text-ui-fg-muted",
                "{subtitle}"
            }
        } else if let Some(variants) = &product.variants {
            p {
                class: "mt-0.5 md:mt-0 text-sm text-ui-fg-muted",
                for (i, variant) in variants.iter().enumerate() {
                    if i != 0 {
                        ", "
                    },
                    "{variant.variant_name}",
                }
            }
        }
    };

    rsx! {
        // Conditionally wrap in Link or just a div based on access
        if user_has_access {
            Link {
                to: Route::ProductPage { handle: product.handle.clone() },
                class: "group go",
                {card_content}
            }
        } else {
            div {
                class: "group",
                {card_content}
            }
        }
    }
}

#[component]
pub fn WideProductCard(
    #[props(default = None)] product: Option<Product>,
    #[props(default = "")] top_class: &'static str,
    #[props(default = false)] loading: bool,
    #[props(default = false)] alert: bool,
) -> Element {
    let mut image_loaded = use_signal(|| false);

    // If loading or no product, render skeleton
    if loading || product.is_none() {
        return rsx! {
            div { class: "group",
                // Only wrap in animated border when alert is true
                if alert {
                    div { class: "gradient-border rounded-lg",
                        // Inner card gets the real content, inherits the radius to hide the border behind
                        div {
                            class: "{top_class} relative boop rounded-lg overflow-hidden p-4 bg-ui-bg-subtle shadow-elevation-card-rest border-0 flex flex-col md:flex-row md:h-64",
                            style: "border-radius: inherit;",

                            // Mobile: Image first (top)
                            div { class: "w-full h-48 md:hidden mb-4 flex-shrink-0",
                                div { class: "skeleton-image w-full h-full rounded" }
                            }

                            // Content section
                            div { class: "flex-1 md:pr-4 flex flex-col min-w-0",
                                div { class: "skeleton mb-2", style: "width: 70%; height: 24px;" }
                                div { class: "skeleton mb-4", style: "width: 40%; height: 18px;" }

                                div { class: "flex-1 mb-3",
                                    for i in 0..3 {
                                        div { class: "skeleton mb-2", style: "width: {90 - i * 15}%; height: 16px;" }
                                    }
                                }

                                div { class: "skeleton", style: "width: 25%; height: 20px;" }
                            }

                            // Desktop: Image on right
                            div { class: "hidden md:block w-64 flex-shrink-0", style: "max-height: 250px;",
                                div { class: "skeleton-image w-full h-full rounded" }
                            }
                        }
                    }
                } else {
                    // Non-alert normal card as before (with regular border)
                    div {
                        class: "{top_class} boop rounded-lg overflow-hidden p-4 bg-ui-bg-subtle shadow-elevation-card-rest border-gray-200 border flex flex-col md:flex-row md:h-64",

                        // Mobile: Image first (top)
                        div { class: "w-full h-48 md:hidden mb-4 flex-shrink-0",
                            div { class: "skeleton-image w-full h-full rounded" }
                        }

                        // Content section
                        div { class: "flex-1 md:pr-4 flex flex-col min-w-0",
                            div { class: "skeleton mb-2", style: "width: 70%; height: 24px;" }
                            div { class: "skeleton mb-4", style: "width: 40%; height: 18px;" }

                            div { class: "flex-1 mb-3",
                                for i in 0..3 {
                                    div { class: "skeleton mb-2", style: "width: {90 - i * 15}%; height: 16px;" }
                                }
                            }

                            div { class: "skeleton", style: "width: 25%; height: 20px;" }
                        }

                        // Desktop: Image on right
                        div { class: "hidden md:block w-64 flex-shrink-0", style: "max-height: 250px;",
                            div { class: "skeleton-image w-full h-full rounded" }
                        }
                    }
                }
            }
        };
    }

    // Unwrap the product since we know it exists at this point
    let product = product.unwrap();

    // Check if product is gated (has access_groups)
    let is_gated = product.access_groups.as_ref().map_or(false, |groups| !groups.is_empty());

    // Use cached session info for faster access checks
    let session_signal = use_stale_while_revalidate(
        "session_info",
        || async { get_session_info().await },
        Duration::from_secs(60),
    );

    let session_state = session_signal.read();

    // Check if user has access based on session group membership
    let user_has_access = if is_gated {
        if let Some(session) = session_state.as_ref() {
            // User has access if any of their group IDs match the product's access groups
            product.access_groups
                .as_ref()
                .map_or(false, |access_groups| {
                    session.group_ids.iter().any(|group_id| access_groups.contains(group_id))
                })
        } else {
            // No session - no access to gated product
            false
        }
    } else {
        // Product is not gated - everyone has access
        true
    };

    // Get thumbnail from variants (prioritize default variant)
    let thumbnail_url = if let Some(variants) = &product.variants {
        if let Some(default_variant_id) = &product.default_variant_id {
            variants
                .iter()
                .find(|v| &v.id == default_variant_id)
                .and_then(|v| v.thumbnail_url.as_ref())
                .or_else(|| variants.first().and_then(|v| v.thumbnail_url.as_ref()))
        } else {
            variants.first().and_then(|v| v.thumbnail_url.as_ref())
        }
    } else {
        None
    };

    // Check if out of stock based on variant stock quantities
    let is_out_of_stock = product.force_no_stock
        || product
            .variants
            .as_ref()
            .map(|variants| {
                variants.is_empty()
                    || variants
                        .iter()
                        .all(|v| v.calculated_stock_quantity.unwrap_or(0) <= 0)
            })
            .unwrap_or(true);

    // Helper function to format price without unnecessary .00
    fn format_price(price: f64) -> String {
        let formatted = format!("${:.2}", price);
        if formatted.ends_with(".00") {
            formatted.trim_end_matches(".00").to_string()
        } else {
            formatted
        }
    }

    // Check if any variant has a discount
    let has_discount = product.variants.as_ref().map_or(false, |variants| {
        variants.iter().any(|v| v.price_standard_without_sale.is_some())
    });

    // Calculate price display info
    let (price_text, original_price_text, is_single_variant) = if let Some(variants) = &product.variants {
        if variants.is_empty() {
            ("N/A".to_string(), None, false)
        } else if variants.len() == 1 {
            let variant = &variants[0];
            let current_price = format_price(variant.price_standard_usd);
            let original_price = variant.price_standard_without_sale.map(format_price);
            (current_price, original_price, true)
        } else {
            let prices: Vec<f64> = variants.iter().map(|v| v.price_standard_usd).collect();
            let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

            let price_range = if (min_price - max_price).abs() < 0.01 {
                format_price(min_price)
            } else {
                format!("{} — {}", format_price(min_price), format_price(max_price))
            };
            (price_range, None, false)
        }
    } else {
        ("N/A".to_string(), None, false)
    };

    // Format title with product form
    let display_title = if product.title.contains("(") {
        product.title.clone()
    } else {
        format!(
            "{} ({})",
            product.title,
            product.product_form.to_frontend_string()
        )
    };

    let card_content = |with_alert_styles: bool| rsx! {
        // ALERT: animated white-blue border + white background
        if with_alert_styles {
            div {
                class: if user_has_access { "{top_class} animated-blue-border bg-white relative boop rounded-lg overflow-hidden p-4 bg-white shadow-elevation-card-rest group-hover:shadow-elevation-card-hover transition-shadow ease-in-out duration-150 flex flex-col md:flex-row md:h-64" } else { "{top_class} animated-blue-border bg-white relative rounded-lg overflow-hidden p-4 bg-white shadow-elevation-card-rest flex flex-col md:flex-row md:h-64 cursor-not-allowed" },

                // Status indicators positioned absolutely
                div {
                    class: "absolute top-2 right-2 flex flex-col gap-1",
                    style: "z-index: 3;",

                    if product.pre_order {
                        div {
                            title: "Available for Pre-order",
                            class: "rounded-md flex py-0.5 px-1 text-xs bg-white border-ui-border-base border text-black",
                            div { class: "mr-1 bg-[#34c9c9]", style: "border-radius: 50%;height: 12px; width: 12px;margin-top: 2px;" }
                            span { class: "text-ui-fg-subtle", { t!("pre-order") } }
                        }
                    }

                    if is_out_of_stock {
                        div {
                            title: "Out of Stock in all locations",
                            class: "rounded-md flex py-0.5 px-1 text-xs bg-white border-typical border text-black",
                            div { class: if product.pre_order { "mr-1 bg-gray-400" } else { "mr-1 bg-orange-400" }, style: "border-radius: 50%;height: 12px; width: 12px;margin-top: 2px;" }
                            span { class: "text-ui-fg-subtle", { t!("sold-out") } }
                        }
                    }
                }

                // Members-only overlay for gated products without access
                if is_gated && !user_has_access {
                    div {
                        class: "absolute inset-0 bg-white bg-opacity-0 group-hover:bg-opacity-50 transition-all duration-200 flex items-center justify-center",
                        style: "z-index: 4;",

                        div {
                            class: "opacity-0 group-hover:opacity-100 transition-opacity duration-200 text-center px-4",
                            p {
                                class: "text-ui-fg-base font-medium",
                                "This product is for members-only"
                            }
                        }
                    }
                }

                // Mobile: Image first (top)
                div {
                    class: "w-full h-48 md:hidden mb-4 flex-shrink-0 relative aspect-square",

                    if let Some(img_url) = thumbnail_url {
                        img {
                            alt: format!("{} {}", product.title, t!("thumbnail")),
                            class: format!("w-full h-full object-cover object-center rounded fade-in-image {}",
                                if image_loaded() { "loaded" } else { "" }),
                            loading: "lazy",
                            draggable: "false",
                            crossorigin: "anonymous",
                            decoding: "async",
                            src: "{img_url}",
                            onload: move |_| { image_loaded.set(true); }
                        }
                    } else if let Some(smiles) = &product.smiles {
                        div {
                            class: "w-full h-full flex items-center justify-center bg-gray-50 rounded overflow-hidden",
                            SmilesViewer { smiles: smiles.clone() }
                        }
                    } else {
                        div {
                            class: "w-full h-full bg-gray-100 rounded flex items-center justify-center",
                            span { class: "text-gray-400 text-sm", "No image" }
                        }
                    }
                }

                // Content section
                div {
                    class: "flex-1 md:pr-4 flex flex-col min-w-0",

                    div {
                        class: "flex items-center mb-1",

                        // Lock icon for gated products
                        if is_gated {
                            if user_has_access {
                                img {
                                    class: "mr-1",
                                    src: asset!("/assets/icons/lock-open-outline.svg"),
                                    style: "height:18px;",
                                    title: "You have access to this product"
                                }
                            } else {
                                img {
                                    class: "mr-1",
                                    src: asset!("/assets/icons/lock-closed-outline.svg"),
                                    style: "height:18px;",
                                    title: "Members-only product"
                                }
                            }
                        }

                        h3 {
                            class: "font-normal font-sans txt-large text-ui-fg-base line-clamp-2",
                            "{display_title}"
                        }
                    }

                    // Subtitle
                    div {
                        class: "mb-3",
                        if let Some(subtitle) = &product.subtitle {
                            p { class: "text-sm text-ui-fg-muted line-clamp-1", "{subtitle}" }
                        } else if let Some(variants) = &product.variants {
                            p {
                                class: "text-sm text-ui-fg-muted line-clamp-1",
                                for (i, variant) in variants.iter().enumerate() {
                                    if i != 0 { ", " }
                                    "{variant.variant_name}"
                                }
                            }
                        }
                    }

                    // Small description
                    if let Some(description) = &product.small_description_md {
                        div {
                            class: "flex-1 text-sm text-ui-fg-subtle overflow-hidden mb-3",
                            style: "display: -webkit-box; -webkit-line-clamp: 4; -webkit-box-orient: vertical;",
                            dangerous_inner_html: "{description}"
                        }
                    }

                    // Price at bottom left with discount support
                    div {
                        class: "mt-auto",
                        div {
                            class: "flex items-center gap-2",

                            // Current price (red if on sale)
                            span {
                                class: if has_discount { "font-medium text-sale-500" } else { "font-medium text-ui-fg-base" },
                                "{price_text}"
                            }

                            // Original price (strikethrough if on sale and single variant)
                            if is_single_variant && has_discount {
                                if let Some(original_price) = original_price_text {
                                    span {
                                        class: "font-medium text-ui-fg-muted line-through",
                                        "{original_price}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Desktop: Image on right
                div {
                    class: "hidden md:block w-64 flex-shrink-0 relative",
                    style: "max-height: 250px;",

                    if let Some(img_url) = thumbnail_url {
                        img {
                            alt: format!("{} {}", product.title, t!("thumbnail")),
                            class: format!("w-full aspect-square h-full object-cover object-center rounded fade-in-image {}",
                                if image_loaded() { "loaded" } else { "" }),
                            loading: "lazy",
                            draggable: "false",
                            crossorigin: "anonymous",
                            decoding: "async",
                            style: "max-height: 250px;",
                            src: "{img_url}",
                            onload: move |_| { image_loaded.set(true); }
                        }
                    } else if let Some(smiles) = &product.smiles {
                        div {
                            class: "w-full h-full flex items-center justify-center bg-gray-50 rounded",
                            style: "max-height: 250px; overflow: hidden;",
                            SmilesViewer { smiles: smiles.clone() }
                        }
                    } else {
                        div {
                            class: "w-full h-full bg-gray-100 rounded flex items-center justify-center",
                            style: "max-height: 250px;",
                            span { class: "text-gray-400 text-sm", "No image" }
                        }
                    }
                }
            }
        } else {
            // Non-alert: normal static border
            div {
                class: if user_has_access { "{top_class} relative boop rounded-lg overflow-hidden p-4 bg-white shadow-elevation-card-rest group-hover:shadow-elevation-card-hover transition-shadow ease-in-out duration-150 border-gray-200 border flex flex-col md:flex-row md:h-64" } else { "{top_class} relative rounded-lg overflow-hidden p-4 bg-white shadow-elevation-card-rest border-gray-200 border flex flex-col md:flex-row md:h-64 cursor-not-allowed" },

                // Status indicators positioned absolutely
                div {
                    class: "absolute top-2 right-2 flex flex-col gap-1",
                    style: "z-index: 3;",

                    if product.pre_order {
                        div {
                            title: "Available for Pre-order",
                            class: "rounded-md flex py-0.5 px-1 text-xs bg-white border-ui-border-base border text-black",
                            div { class: "mr-1 bg-[#34c9c9]", style: "border-radius: 50%;height: 12px; width: 12px;margin-top: 2px;" }
                            span { class: "text-ui-fg-subtle", { t!("pre-order") } }
                        }
                    }

                    if is_out_of_stock {
                        div {
                            title: "Out of Stock in all locations",
                            class: "rounded-md flex py-0.5 px-1 text-xs bg-white border-typical border text-black",
                            div { class: if product.pre_order { "mr-1 bg-gray-400" } else { "mr-1 bg-orange-400" }, style: "border-radius: 50%;height: 12px; width: 12px;margin-top: 2px;" }
                            span { class: "text-ui-fg-subtle", { t!("sold-out") } }
                        }
                    }
                }

                // Members-only overlay for gated products without access
                if is_gated && !user_has_access {
                    div {
                        class: "absolute inset-0 bg-white bg-opacity-0 group-hover:bg-opacity-50 transition-all duration-200 flex items-center justify-center",
                        style: "z-index: 4;",

                        div {
                            class: "opacity-0 group-hover:opacity-100 transition-opacity duration-200 text-center px-4",
                            p {
                                class: "text-ui-fg-base font-medium",
                                "This product is for members-only"
                            }
                        }
                    }
                }

                // Mobile: Image first (top)
                div {
                    class: "w-full h-48 md:hidden mb-4 flex-shrink-0 relative",

                    if let Some(img_url) = thumbnail_url {
                        img {
                            alt: format!("{} {}", product.title, t!("thumbnail")),
                            class: format!("w-full h-full object-cover object-center rounded fade-in-image {}",
                                if image_loaded() { "loaded" } else { "" }),
                            loading: "lazy",
                            draggable: "false",
                            crossorigin: "anonymous",
                            decoding: "async",
                            src: "{img_url}",
                            onload: move |_| { image_loaded.set(true); }
                        }
                    } else if let Some(smiles) = &product.smiles {
                        div {
                            class: "w-full h-full flex items-center justify-center bg-gray-50 rounded overflow-hidden",
                            SmilesViewer { smiles: smiles.clone() }
                        }
                    } else {
                        div {
                            class: "w-full h-full bg-gray-100 rounded flex items-center justify-center",
                            span { class: "text-gray-400 text-sm", "No image" }
                        }
                    }
                }

                // Content section
                div {
                    class: "flex-1 md:pr-4 flex flex-col min-w-0",

                    div {
                        class: "flex items-center mb-1",

                        // Lock icon for gated products
                        if is_gated {
                            if user_has_access {
                                img {
                                    class: "mr-2",
                                    src: asset!("/assets/icons/lock-open-outline.svg"),
                                    style: "height:18px;",
                                    title: "You have access to this product"
                                }
                            } else {
                                img {
                                    class: "mr-2",
                                    src: asset!("/assets/icons/lock-closed-outline.svg"),
                                    style: "height:18px;",
                                    title: "Members-only product"
                                }
                            }
                        }

                        h3 {
                            class: "font-normal font-sans txt-large text-ui-fg-base line-clamp-2",
                            "{display_title}"
                        }
                    }

                    div {
                        class: "mb-3",
                        if let Some(subtitle) = &product.subtitle {
                            p { class: "text-sm text-ui-fg-muted line-clamp-1", "{subtitle}" }
                        } else if let Some(variants) = &product.variants {
                            p {
                                class: "text-sm text-ui-fg-muted line-clamp-1",
                                for (i, variant) in variants.iter().enumerate() {
                                    if i != 0 { ", " }
                                    "{variant.variant_name}"
                                }
                            }
                        }
                    }

                    if let Some(description) = &product.small_description_md {
                        div {
                            class: "flex-1 text-sm text-ui-fg-subtle overflow-hidden mb-3",
                            style: "display: -webkit-box; -webkit-line-clamp: 4; -webkit-box-orient: vertical;",
                            dangerous_inner_html: "{description}"
                        }
                    }

                    // Price at bottom left with discount support
                    div {
                        class: "mt-auto",
                        div {
                            class: "flex items-center gap-2",

                            // Current price (red if on sale)
                            span {
                                class: if has_discount { "font-medium text-red-500" } else { "font-medium text-ui-fg-base" },
                                "{price_text}"
                            }

                            // Original price (strikethrough if on sale and single variant)
                            if is_single_variant && has_discount {
                                if let Some(original_price) = original_price_text {
                                    span {
                                        class: "font-medium text-ui-fg-muted line-through",
                                        "{original_price}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Desktop: Image on right
                div {
                    class: "hidden md:block w-64 flex-shrink-0 relative",
                    style: "max-height: 250px;",

                    if let Some(img_url) = thumbnail_url {
                        img {
                            alt: format!("{} {}", product.title, t!("thumbnail")),
                            class: format!("w-full h-full object-cover object-center rounded fade-in-image {}",
                                if image_loaded() { "loaded" } else { "" }),
                            loading: "lazy",
                            draggable: "false",
                            crossorigin: "anonymous",
                            decoding: "async",
                            style: "max-height: 250px;",
                            src: "{img_url}",
                            onload: move |_| { image_loaded.set(true); }
                        }
                    } else if let Some(smiles) = &product.smiles {
                        div {
                            class: "w-full h-full flex items-center justify-center bg-gray-50 rounded",
                            style: "max-height: 250px; overflow: hidden;",
                            SmilesViewer { smiles: smiles.clone() }
                        }
                    } else {
                        div {
                            class: "w-full h-full bg-gray-100 rounded flex items-center justify-center",
                            style: "max-height: 250px;",
                            span { class: "text-gray-400 text-sm", "No image" }
                        }
                    }
                }
            }
        }
    };

    rsx! {
        // Conditionally wrap in Link or just a div based on access
        if user_has_access {
            Link {
                to: Route::ProductPage { handle: product.handle.clone() },
                class: "group go block",
                {card_content(alert)}
            }
        } else {
            div {
                class: "group block",
                {card_content(alert)}
            }
        }
    }
}
