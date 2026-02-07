use crate::Route;
use dioxus::prelude::*;
use dioxus_i18n::t;

use crate::backend::front_entities::*;
use crate::backend::server_functions;
use crate::utils::{countries::country_display_name_from_iso, format_datetime};

#[component]
pub fn OrderStatus(order_id: String) -> Element {
    let order_info = use_resource({
        let order_id = order_id.clone();
        move || {
            let order_id = order_id.clone();
            async move { server_functions::get_short_order(order_id.clone()).await }
        }
    });

    use_effect(move || {
        let order_info = order_info();

        tracing::info!("User-displaying order info: {:?}", order_info);
    });

    rsx! {
        document::Title { { format!("{} - {}", t!("brand"), t!("order-status") ) } }

        div {
            class: "min-h-screen py-2 md:py-8",
            div {
                class: "max-w-2xl mx-auto px-4 sm:px-6 lg:px-8",
                {
                    match &*order_info.read_unchecked() {
                        Some(Ok(order_info)) => rsx! {
                            div {

                                // Header Card
                                div {
                                    class: "mb-2",
                                    div {
                                        class: "py-5 flex justify-between items-start",
                                        div {
                                            h1 {
                                                class: "text-2xl text-gray-900",
                                                { t!("your-order") }
                                            }
                                            h2 {
                                                class: "text-base mt-1 text-gray-700 text-semibold text-gray-900",
                                                { t!("order-id-ref", ref: order_info.ref_code.clone() ) }
                                            }
                                        }
                                        div {
                                            if order_info.fulfilled_at.is_some() {
                                                span {
                                                    class: "px-3 py-1 bg-green-100 text-green-800 rounded-full text-sm font-medium",
                                                    { t!("os-complete") }
                                                }
                                            } else if order_info.paid_at.is_some() {
                                                span {
                                                    class: "px-3 py-1 bg-blue-100 text-blue-800 rounded-full text-sm font-medium",
                                                    { t!("os-paid") }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Tracking Link Card
                                {
                                    if let Some(tracking_url) = &order_info.tracking_url {
                                        if order_info.fulfilled_at.is_some() {
                                            rsx! {
                                                div {
                                                    class: "bg-blue-50 border border-blue-200 rounded-lg p-4 mb-4",
                                                    div {
                                                        class: "flex items-center justify-between",
                                                        div {
                                                            h3 {
                                                                class: "text-lg font-semibold text-blue-900",
                                                                { t!("tracking-information") }
                                                            }
                                                            p {
                                                                class: "text-sm text-blue-700",
                                                                { t!("track-your-package") }
                                                            }
                                                        }
                                                        a {
                                                            href: tracking_url.clone(),
                                                            target: "_blank",
                                                            rel: "noopener noreferrer",
                                                            class: "px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors font-medium",
                                                            { t!("track-package") }
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
                                }

                                // Progress Card
                                div {
                                    class: "bg-white py-5 mb-4 rounded-lg border border-gray-200 overflow-hidden flex justify-center",
                                    div {
                                        class: "flex flex-col w-[450px]",

                                        // Icons container
                                        div {
                                            class: "flex px-6 justify-between relative",
                                            img {
                                                src: asset!("/assets/icons/clipboard.png"),
                                                class: format_args!("w-10 h-10 object-contain {}", if order_info.created_at.is_some() { "" } else { "opacity-40" })
                                            }
                                            img {
                                                src: asset!("/assets/icons/coin.png"),
                                                class: format_args!("w-10 h-10 object-contain {}", if order_info.paid_at.is_some() { "" } else { "opacity-40" })
                                            }
                                            img {
                                                src: asset!("/assets/icons/open-box.png"),
                                                class: format_args!("w-10 h-10 object-contain {}", if order_info.prepared_at.is_some() { "" } else { "opacity-40" })
                                            }
                                            img {
                                                src: if order_info.fulfilled_at.is_some() {
                                                    asset!("/assets/icons/delivery.gif")
                                                } else {
                                                    asset!("/assets/icons/delivery.png") // Assuming you have a static PNG version
                                                },
                                                class: format_args!("w-10 h-10 object-contain {}", if order_info.fulfilled_at.is_some() { "" } else { "opacity-40" })
                                            }
                                        }
                                        div {
                                            class: "relative mx-5 mt-5",
                                            // Background line
                                            div {
                                                class: "absolute top-1/2 left-2 right-2 h-2 bg-gray-200 -translate-y-1/2"
                                            }
                                            // Progress line (sky fill)
                                            div {
                                                class: "absolute top-1/2 left-2 h-2 bg-blue-500 -translate-y-1/2",
                                                style: format_args!("width: calc({}% - 0px)", {
                                                    let progress = if order_info.fulfilled_at.is_some() {
                                                        100.0
                                                    } else if order_info.prepared_at.is_some() {
                                                        62.7
                                                    } else if order_info.paid_at.is_some() {
                                                        33.3
                                                    } else if order_info.created_at.is_some() {
                                                        0.0
                                                    } else {
                                                        0.0
                                                    };
                                                    progress
                                                })
                                            }
                                        }
                                        div {
                                            class: "flex justify-between relative ml-[-15px] mr-[-15px] mt-5 text-center",
                                            div {
                                                p {
                                                    class: "text-[10px] pb-2 text-blue-500",
                                                    {
                                                        if let Some(created_at) = order_info.created_at {
                                                            format_datetime(created_at)
                                                        } else {
                                                            "\u{00A0}".to_string()
                                                        }
                                                    }
                                                }
                                                p {
                                                    class: format!("text-xs font-semibold {}", if order_info.created_at.is_some() { "text-blue-500" } else { "text-gray-500" }),
                                                    style: "word-spacing: 100vw;",
                                                    { t!("registered-order") }
                                                }
                                            }
                                            div {
                                                p {
                                                    class: "text-[10px] pb-2 text-blue-500",
                                                    {
                                                        if let Some(paid_at) = order_info.paid_at {
                                                            format_datetime(paid_at)
                                                        } else {
                                                            "\u{00A0}".to_string()
                                                        }
                                                    }
                                                }
                                                p {
                                                    class: format!("text-xs font-semibold {}", if order_info.paid_at.is_some() { "text-blue-500" } else { "text-gray-500" }),
                                                    style: "word-spacing: 100vw;",
                                                    { t!("approved-payment") }
                                                }
                                            }
                                            div {
                                                p {
                                                    class: "text-[10px] pb-2 text-blue-500",
                                                    {
                                                        if let Some(prepared_at) = order_info.prepared_at {
                                                            format_datetime(prepared_at)
                                                        } else {
                                                            "\u{00A0}".to_string()
                                                        }
                                                    }
                                                }
                                                p {
                                                    class: format!("text-xs font-semibold {}", if order_info.prepared_at.is_some() { "text-blue-500" } else { "text-gray-500" }),
                                                    style: "word-spacing: 100vw;",
                                                    { t!("order-prepared") }
                                                }
                                            }
                                            div {
                                                p {
                                                    class: "text-[10px] pb-2 text-blue-500",
                                                    {
                                                        if let Some(fulfilled_at) = order_info.fulfilled_at {
                                                            format_datetime(fulfilled_at)
                                                        } else {
                                                            "\u{00A0}".to_string()
                                                        }
                                                    }
                                                }
                                                p {
                                                    class: format!("text-xs font-semibold {}", if order_info.fulfilled_at.is_some() { "text-blue-500" } else { "text-gray-500" }),
                                                    style: "word-spacing: 100vw;",
                                                    { t!("order-dispatched") }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Order Details Card
                                div {
                                    class: "bg-white rounded-lg border border-gray-200 p-6 mb-4",
                                    h3 {
                                        class: "text-lg font-semibold text-gray-900 mb-4",
                                        { t!("os-order-details") }
                                    }
                                    div {
                                        class: "grid grid-cols-2 gap-4 text-sm mb-4",
                                        div {
                                            span {
                                                class: "font-medium text-gray-500",
                                                { t!("os-shipping-option") }
                                            }
                                            p {
                                                class: "text-gray-900",
                                                { format!("{:?}", order_info.shipping_option) }
                                            }
                                        }
                                        div {
                                            span {
                                                class: "font-medium text-gray-500",
                                                { t!("os-billing-country") }
                                            }
                                            p {
                                                class: "text-gray-900",
                                                { country_display_name_from_iso(&order_info.billing_country.clone()) }
                                            }
                                        }
                                    }
                                    div {
                                        class: "pt-4 border-t border-gray-200",
                                        div {
                                            class: "flex justify-between items-center text-lg font-semibold",
                                            span { { t!("os-total-amount") } }
                                            span { { format!("${:.2}", order_info.total_amount_usd) } }
                                        }
                                    }
                                }

                                // Items Card
                                div {
                                    class: "bg-white rounded-lg border border-gray-200 p-6 mb-4",
                                    h3 {
                                        class: "text-lg font-semibold text-gray-900 mb-4",
                                        { t!("os-order-items") }
                                    }
                                    div {
                                        class: "space-y-3",
                                        for item in &order_info.items {
                                            {
                                                let item_class = if item.pre_order_on_purchase {
                                                    "bg-blue-50 border-2 border-blue-200 rounded-md p-3"
                                                } else {
                                                    "bg-white border rounded-md border-gray-200 p-3"
                                                };

                                                // Find matching pre-order for this item
                                                let matching_pre_order = order_info.pre_orders.iter()
                                                    .find(|po| po.order_item_id == item.id);

                                                rsx! {
                                                    div {
                                                        class: "{item_class}",
                                                        div {
                                                            class: "font-medium",
                                                            "{item.quantity}x {item.product_title}"
                                                            if !item.variant_name.is_empty() {
                                                                span { class: "text-gray-600", " ({item.variant_name})" }
                                                            }
                                                            if item.pre_order_on_purchase {
                                                                span {
                                                                    class: "ml-2 px-2 py-1 bg-blue-100 text-blue-800 rounded text-xs font-medium",
                                                                    { t!("os-pre-order") }
                                                                }
                                                            }
                                                        }
                                                        div {
                                                            class: "text-sm text-gray-600",
                                                            { format!("{}: ${:.2}, {}: ${:.2}", t!("os-price-per-item"), item.price_usd, t!("os-total"), item.price_usd * item.quantity as f64) }
                                                        }

                                                        // Pre-order details if this item has matching pre-order
                                                        {
                                                            if let Some(pre_order) = matching_pre_order {
                                                                rsx! {
                                                                    div {
                                                                        class: "mt-3 pt-3 border-t border-gray-200",
                                                                        div {
                                                                            class: "text-sm text-gray-700",
                                                                            div {
                                                                                class: "grid grid-cols-1 gap-1",

                                                                                // Prepared date
                                                                                {
                                                                                    if let Some(prepared_at) = pre_order.prepared_at {
                                                                                        rsx! {
                                                                                            div {
                                                                                                span { class: "font-medium", "Prepared: " }
                                                                                                span { { format_datetime(prepared_at) } }
                                                                                            }
                                                                                        }
                                                                                    } else {
                                                                                        rsx! { }
                                                                                    }
                                                                                }

                                                                                // Fulfilled date
                                                                                {
                                                                                    if let Some(fulfilled_at) = pre_order.fulfilled_at {
                                                                                        rsx! {
                                                                                            div {
                                                                                                span { class: "font-medium", "Fulfilled: " }
                                                                                                span { { format_datetime(fulfilled_at) } }
                                                                                            }
                                                                                        }
                                                                                    } else {
                                                                                        rsx! { }
                                                                                    }
                                                                                }

                                                                                // Tracking link
                                                                                {
                                                                                    if let Some(tracking_url) = &pre_order.tracking_url {
                                                                                        rsx! {
                                                                                            div {
                                                                                                span { class: "font-medium", "Tracking Link: " }
                                                                                                a {
                                                                                                    href: tracking_url.clone(),
                                                                                                    target: "_blank",
                                                                                                    rel: "noopener noreferrer",
                                                                                                    class: "text-blue-600 hover:text-blue-800 underline",
                                                                                                    "Track Package"
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    } else {
                                                                                        rsx! { }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            } else {
                                                                rsx! { }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Backorder Warning
                                {
                                    if order_info.contains_back_order {
                                        rsx! {
                                            div {
                                                class: "bg-orange-50 border-2 border-orange-200 rounded-lg p-4",
                                                div {
                                                    class: "flex items-start",
                                                    div {
                                                        class: "flex-shrink-0",
                                                        span {
                                                            class: "text-orange-500 text-xl",
                                                            "⚠"
                                                        }
                                                    }
                                                    div {
                                                        class: "ml-3",
                                                        h4 {
                                                            class: "text-sm font-semibold text-orange-800",
                                                            { t!("os-backorder-notice") }
                                                        }
                                                        p {
                                                            class: "text-sm text-orange-700 mt-1",
                                                            { t!("os-backorder-message") }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! { }
                                    }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! { "Could not find order" },
                        None => rsx! {
                            p {
                                class: "text-gray-700",
                                { t!("loading-your-order") }
                            }
                        }
                    }
                }
            }
        }
    }
}
