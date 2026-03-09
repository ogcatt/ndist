#![allow(non_snake_case)] // Allow non-snake_case identifiers

use chrono::{Local, NaiveDateTime, Utc};
use dioxus::prelude::*;
use js_sys::eval;
use std::time::Duration;

use crate::Route;
use crate::backend::cache::use_cached_server;
use crate::backend::front_entities::*;
use crate::backend::server_functions;
use crate::utils;

#[derive(Debug, Clone, PartialEq)]
enum OrderTab {
    All,
    Unfulfilled,
    PreOrders,
    Packaged,
    Fulfilled,
}

impl std::fmt::Display for OrderTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderTab::All => write!(f, "All"),
            OrderTab::Unfulfilled => write!(f, "Unfulfilled"),
            OrderTab::PreOrders => write!(f, "Pre-Orders"),
            OrderTab::Packaged => write!(f, "Packaged"),
            OrderTab::Fulfilled => write!(f, "Fulfilled"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum TimeSpan {
    Days7,
    Days30,
    Days60,
    Days90,
    Days180,
    Days360,
}

impl TimeSpan {
    fn days(&self) -> i64 {
        match self {
            TimeSpan::Days7 => 7,
            TimeSpan::Days30 => 30,
            TimeSpan::Days60 => 60,
            TimeSpan::Days90 => 90,
            TimeSpan::Days180 => 180,
            TimeSpan::Days360 => 360,
        }
    }
}

impl std::fmt::Display for TimeSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.days())
    }
}

// Helper functions
fn format_date(date: &NaiveDateTime) -> String {
    let local_date = Local::now().naive_local().date();
    let order_date = date.date();

    if order_date == local_date {
        "Today".to_string()
    } else if order_date == local_date - chrono::Duration::days(1) {
        "Yesterday".to_string()
    } else {
        date.format("%a %d %b %Y").to_string()
    }
}

fn has_backorders(order: &OrderInfo) -> bool {
    !order.backorder_reduces.is_empty()
}

fn has_preorders(order: &OrderInfo) -> bool {
    order.items.iter().any(|item| item.pre_order_on_purchase)
}

fn get_order_status_display(order: &OrderInfo) -> String {
    if has_backorders(order) {
        "Backordered".to_string()
    } else {
        order.status.to_string()
    }
}

fn get_order_status_class(order: &OrderInfo) -> &'static str {
    if has_backorders(order) {
        "px-2 py-1 bg-orange-100 text-orange-800 rounded-full text-xs font-medium"
    } else {
        match order.status {
            OrderStatus::Pending | OrderStatus::Processing => {
                "px-2 py-1 bg-yellow-100 text-yellow-800 rounded-full text-xs font-medium"
            }
            OrderStatus::Paid => {
                "px-2 py-1 bg-yellow-100 text-yellow-800 rounded-full text-xs font-medium"
            }
            OrderStatus::Fulfilled => {
                "px-2 py-1 bg-green-100 text-green-800 rounded-full text-xs font-medium"
            }
            OrderStatus::Cancelled => {
                "px-2 py-1 bg-red-100 text-red-800 rounded-full text-xs font-medium"
            }
            OrderStatus::Refunded => {
                "px-2 py-1 bg-gray-100 text-gray-800 rounded-full text-xs font-medium"
            }
        }
    }
}

fn is_item_backordered(
    item: &OrderShortItem,
    backorder_reduces: &[BackOrPreOrderActiveReduce],
) -> bool {
    // Check if this item has any backorder reduces by matching order_item_id
    backorder_reduces
        .iter()
        .any(|reduce| reduce.order_item_id == item.id)
}

fn is_item_preordered(
    item: &OrderShortItem,
    preorder_reduces: &[BackOrPreOrderActiveReduce],
) -> bool {
    // Check if this item has any preorder reduces by matching order_item_id
    preorder_reduces
        .iter()
        .any(|reduce| reduce.order_item_id == item.id)
}

fn get_preorder_status(
    item: &OrderShortItem,
    preorder_reduces: &[BackOrPreOrderActiveReduce],
    pre_orders: &[PreOrder],
) -> String {
    // Find the matching pre_order entry for this item
    if let Some(pre_order) = pre_orders.iter().find(|po| po.order_item_id == item.id) {
        if pre_order.fulfilled_at.is_some() {
            // Check if it has tracking - for both express and non-express
            if pre_order.tracking_url.is_none()
                || pre_order
                    .tracking_url
                    .as_ref()
                    .map_or(false, |url| url.trim().is_empty())
            {
                "Untracked".to_string()
            } else {
                "Fulfilled".to_string()
            }
        } else if pre_order.tracking_url.is_some()
            && pre_order
                .tracking_url
                .as_ref()
                .map_or(false, |url| !url.trim().is_empty())
        {
            "Packaged".to_string()
        } else {
            "To Fulfill".to_string()
        }
    } else if is_item_preordered(item, preorder_reduces) {
        "Unstocked".to_string()
    } else {
        "To Fulfill".to_string()
    }
}

fn order_matches_tab(order: &OrderInfo, tab: &OrderTab) -> bool {
    match tab {
        OrderTab::All => !has_only_preorders(order),
        OrderTab::Unfulfilled => {
            // Paid orders count as unfulfilled
            let is_paid = order
                .payments
                .iter()
                .any(|p| matches!(p.status, PaymentStatus::Paid));
            is_paid && !matches!(order.status, OrderStatus::Fulfilled) && !has_only_preorders(order)
        }
        OrderTab::PreOrders => has_preorders(order),
        OrderTab::Packaged => {
            // Items that have prepared_at date
            order.prepared_at.is_some()
                && !matches!(order.status, OrderStatus::Fulfilled)
                && !has_only_preorders(order)
        }
        OrderTab::Fulfilled => {
            matches!(order.status, OrderStatus::Fulfilled) && !has_only_preorders(order)
        }
    }
}

fn has_only_preorders(order: &OrderInfo) -> bool {
    !order.items.is_empty() && order.items.iter().all(|item| item.pre_order_on_purchase)
}

fn filter_orders_by_timespan<'a>(
    orders: &'a [OrderInfo],
    timespan: &TimeSpan,
) -> Vec<&'a OrderInfo> {
    let cutoff_date = Utc::now().naive_utc() - chrono::Duration::days(timespan.days());
    orders
        .iter()
        .filter(|order| order.created_at >= cutoff_date)
        .collect()
}

fn calculate_stats(orders: &[OrderInfo], timespan: &TimeSpan) -> (usize, usize, usize, f64) {
    let filtered_orders = filter_orders_by_timespan(orders, timespan);

    let total_orders = filtered_orders.len();
    // Sum the quantities of each item instead of just counting items
    let total_items: usize = filtered_orders
        .iter()
        .map(|o| {
            o.items
                .iter()
                .map(|item| item.quantity as usize)
                .sum::<usize>()
        })
        .sum();
    let fulfilled_orders = filtered_orders
        .iter()
        .filter(|o| matches!(o.status, OrderStatus::Fulfilled))
        .count();

    // Calculate average time to fulfill
    let fulfill_times: Vec<i64> = filtered_orders
        .iter()
        .filter_map(|order| {
            if let (Some(fulfilled_at), Some(paid_at)) = (
                &order.fulfilled_at,
                order.payments.iter().find_map(|p| p.paid_at.as_ref()),
            ) {
                Some((*fulfilled_at - *paid_at).num_days())
            } else {
                None
            }
        })
        .collect();

    let avg_time_to_fulfill = if fulfill_times.is_empty() {
        0.0
    } else {
        fulfill_times.iter().sum::<i64>() as f64 / fulfill_times.len() as f64
    };

    (
        total_orders,
        total_items,
        fulfilled_orders,
        avg_time_to_fulfill,
    )
}

fn copy_to_clipboard(text: String) {
    let _ = eval(&format!(
        r#"navigator.clipboard.writeText('{}').then(() => console.log('Copied to clipboard'))"#,
        text.replace("'", "\\'")
    ));
}

// Check if item has matching pre_order entry
fn has_matching_preorder(item: &OrderShortItem, pre_orders: &[PreOrder]) -> bool {
    pre_orders
        .iter()
        .any(|pre_order| pre_order.order_item_id == item.id)
}

// And define the feedback struct:
#[derive(Clone)]
struct OperationFeedback {
    message: String,
    is_success: bool,
}

#[component]
fn OrderDetailModal(
    order: OrderInfo,
    show: Signal<bool>,
    refresh_trigger: Signal<u32>,
    is_preorder_view: bool,
    selected_preorder_item: Option<OrderShortItem>,
) -> Element {
    let mut dropdown_open = use_signal(|| false);
    let mut tracking_url = use_signal(|| String::new());
    let mut operation_feedback = use_signal(|| None::<OperationFeedback>);
    let mut notes_text = use_signal(|| order.notes.clone().unwrap_or_default());

    let is_backordered = has_backorders(&order);
    let contains_preorders = has_preorders(&order);

    // Clone the order for use in closures
    let order_clone = order.clone();

    // For preorder view, get the specific item and check if it has reduces
    let preorder_item_has_reduces = if let Some(ref item) = selected_preorder_item {
        is_item_preordered(item, &order.preorder_reduces)
    } else {
        false
    };

    // Check if the preorder item has a matching pre_order entry
    let preorder_has_matching_entry = if let Some(ref item) = selected_preorder_item {
        has_matching_preorder(item, &order.pre_orders)
    } else {
        false
    };

    rsx! {
        if *show.read() {
            div {
                class: "fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4",
                onclick: move |_| show.set(false),

                div {
                    class: "bg-white rounded-lg max-w-6xl w-full max-h-[90vh] overflow-y-auto",
                    onclick: |e| e.stop_propagation(),

                    // Header
                    div {
                        class: "flex justify-between items-center p-6 border-b",
                        h2 {
                            class: "text-xl font-semibold",
                            if is_preorder_view {
                                "Pre-Order #{order.ref_code}"
                            } else {
                                "Order #{order.ref_code}"
                            }
                            if is_backordered && !is_preorder_view {
                                span {
                                    class: "ml-2 px-2 py-1 bg-orange-100 text-orange-800 rounded text-sm font-medium",
                                    "BACKORDERED"
                                }
                            }
                        }
                        div {
                            class: "flex items-center gap-4",
                            // Success/Error feedback
                            if let Some(feedback) = operation_feedback() {
                                div {
                                    class: if feedback.is_success { "text-green-600 text-sm font-medium" } else { "text-red-600 text-sm font-medium" },
                                    "{feedback.message}"
                                }
                            }

                            // Pre-order view: different packaging logic
                            if is_preorder_view {
                                if preorder_item_has_reduces {
                                    // Item has reduces - disabled button
                                    div {
                                        class: "relative",
                                        button {
                                            class: "text-gray-400 p-2 rounded-md cursor-not-allowed opacity-50",
                                            disabled: true,
                                            title: "Cannot package pre-order with active reduces",
                                            img {
                                                class: "w-8 opacity-50",
                                                src: asset!("/assets/icons/open-box.png")
                                            }
                                        }
                                        div {
                                            class: "absolute right-0 top-full mt-1 text-xs text-gray-500 whitespace-nowrap",
                                            "Cannot package - unstocked"
                                        }
                                    }
                                } else if preorder_has_matching_entry {
                                    // Has matching pre_order entry - show tracking/fulfill options
                                    if let Some(ref item) = selected_preorder_item {
                                        // Find the matching pre_order
                                        if let Some(matching_preorder) = order.pre_orders.iter().find(|po| po.order_item_id == item.id) {
                                            // Check if already fulfilled
                                            if matching_preorder.fulfilled_at.is_some() {
                                                // Already fulfilled - check if express without tracking and show tracking input
                                                if matching_preorder.shipping_option == ShippingOption::Express && matching_preorder.tracking_url.is_none() {
                                                    div {
                                                        class: "w-full flex gap-2",
                                                        input {
                                                            r#type: "text",
                                                            class: "bg-gray-50 border border-gray-200 rounded-md flex-1 py-2 px-3",
                                                            placeholder: "Add tracking URL",
                                                            oninput: move |event: FormEvent| tracking_url.set(event.value())
                                                        }
                                                        button {
                                                            class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                                                            onclick: {
                                                                let preorder_id = matching_preorder.id.clone();
                                                                let order_id = order.id.clone();
                                                                let mut refresh_trigger = refresh_trigger.clone();
                                                                let mut operation_feedback = operation_feedback.clone();
                                                                let mut tracking_url = tracking_url.clone();
                                                                move |_| {
                                                                    let preorder_id_clone = preorder_id.clone();
                                                                    let order_id = order_id.clone();
                                                                    let tracking_url_str = tracking_url().clone();
                                                                    let mut refresh_trigger = refresh_trigger.clone();
                                                                    let mut operation_feedback = operation_feedback.clone();
                                                                    if !tracking_url_str.is_empty() {
                                                                        // THIS CAN NEVER BE REACHED WHEN .fufilled_at is some but tracking doesn't exist for express orders
                                                                        spawn(async move {
                                                                            match server_functions::admin_express_pre_order_send_tracking(order_id, preorder_id_clone, tracking_url_str).await {
                                                                                Ok(_) => {
                                                                                    operation_feedback.set(Some(OperationFeedback {
                                                                                        message: "Express pre-order tracking sent successfully!".to_string(),
                                                                                        is_success: true,
                                                                                    }));
                                                                                    tracking_url.set("".to_string());
                                                                                    show.set(false);
                                                                                    refresh_trigger.set(refresh_trigger() + 1);
                                                                                },
                                                                                Err(e) => {
                                                                                    operation_feedback.set(Some(OperationFeedback {
                                                                                        message: format!("Failed to send express pre-order tracking: {}", e),
                                                                                        is_success: false,
                                                                                    }));
                                                                                }
                                                                            }
                                                                        });
                                                                    } else {
                                                                        operation_feedback.set(Some(OperationFeedback {
                                                                            message: "Please enter a tracking URL".to_string(),
                                                                            is_success: false,
                                                                        }));
                                                                    }
                                                                }
                                                            },
                                                            "Send Tracking"
                                                        }
                                                    }
                                                } else {
                                                    // Already fulfilled
                                                    div {
                                                        class: "text-green-600 text-sm font-medium",
                                                        "✓ Fulfilledd"
                                                    }
                                                }
                                            } else {
                                                // Not fulfilled yet - show tracking input or express button
                                                div {
                                                    class: "w-full flex gap-2",
                                                    if matching_preorder.shipping_option == ShippingOption::Express {
                                                        // Express shipping - show fulfill button
                                                        button {
                                                            class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                                                            onclick: {
                                                                let preorder_id = matching_preorder.id.clone();
                                                                let order_id = order.id.clone();
                                                                let mut refresh_trigger = refresh_trigger.clone();
                                                                let mut operation_feedback = operation_feedback.clone();
                                                                move |_| {
                                                                    let preorder_id_clone = preorder_id.clone();
                                                                    let order_id = order_id.clone();
                                                                    let mut refresh_trigger = refresh_trigger.clone();
                                                                    let mut operation_feedback = operation_feedback.clone();
                                                                    spawn(async move {
                                                                        match server_functions::admin_express_pre_order_fulfilled_notracking(order_id, preorder_id_clone).await {
                                                                            Ok(_) => {
                                                                                operation_feedback.set(Some(OperationFeedback {
                                                                                    message: "Express pre-order fulfilled successfully!".to_string(),
                                                                                    is_success: true,
                                                                                }));
                                                                                show.set(false);
                                                                                refresh_trigger.set(refresh_trigger() + 1);
                                                                            },
                                                                            Err(e) => {
                                                                                operation_feedback.set(Some(OperationFeedback {
                                                                                    message: format!("Failed to fulfill express pre-order: {}", e),
                                                                                    is_success: false,
                                                                                }));
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            "Fulfill"
                                                        }
                                                    } else {
                                                        // Non-express shipping - show tracking input
                                                        input {
                                                            r#type: "text",
                                                            class: "bg-gray-50 border border-gray-200 rounded-md flex-1 py-2 px-3",
                                                            placeholder: "Tracking URL",
                                                            oninput: move |event: FormEvent| tracking_url.set(event.value())
                                                        }
                                                        button {
                                                            class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700",
                                                            onclick: {
                                                                let preorder_id = matching_preorder.id.clone();
                                                                let order_id = order.id.clone();
                                                                let mut refresh_trigger = refresh_trigger.clone();
                                                                let mut operation_feedback = operation_feedback.clone();
                                                                let mut tracking_url = tracking_url.clone();
                                                                move |_| {
                                                                    let preorder_id_clone = preorder_id.clone();
                                                                    let order_id = order_id.clone();
                                                                    let tracking_url_str = tracking_url().clone();
                                                                    let mut refresh_trigger = refresh_trigger.clone();
                                                                    let mut operation_feedback = operation_feedback.clone();
                                                                    if !tracking_url_str.is_empty() {
                                                                        spawn(async move {
                                                                            match server_functions::admin_set_pre_order_fulfilled(order_id, preorder_id_clone, tracking_url_str).await {
                                                                                Ok(_) => {
                                                                                    operation_feedback.set(Some(OperationFeedback {
                                                                                        message: "Pre-order tracking updated successfully!".to_string(),
                                                                                        is_success: true,
                                                                                    }));
                                                                                    tracking_url.set("".to_string());
                                                                                    show.set(false);
                                                                                    refresh_trigger.set(refresh_trigger() + 1);
                                                                                },
                                                                                Err(e) => {
                                                                                    operation_feedback.set(Some(OperationFeedback {
                                                                                        message: format!("Failed to update pre-order tracking: {}", e),
                                                                                        is_success: false,
                                                                                    }));
                                                                                }
                                                                            }
                                                                        });
                                                                    } else {
                                                                        operation_feedback.set(Some(OperationFeedback {
                                                                            message: "Please enter a tracking URL".to_string(),
                                                                            is_success: false,
                                                                        }));
                                                                    }
                                                                }
                                                            },
                                                            "Update"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Item doesn't have reduces and no matching pre_order - functional button
                                    div {
                                        class: "relative",
                                        button {
                                            class: "text-blue-600 hover:text-blue-800 p-2 rounded-md hover:bg-blue-100",
                                            onclick: {
                                                let selected_preorder_item = selected_preorder_item.clone(); // Clone before the move closure
                                                move |_| {
                                                    if let Some(ref item) = selected_preorder_item {
                                                        let order_item_id = item.id.clone();
                                                        let parent_order_id = order_clone.id.clone();
                                                        let mut refresh_trigger = refresh_trigger.clone();
                                                        let mut operation_feedback = operation_feedback.clone();
                                                        spawn(async move {
                                                            match server_functions::admin_set_preorder_prepared(order_item_id, parent_order_id).await {
                                                                Ok(_) => {
                                                                    operation_feedback.set(Some(OperationFeedback {
                                                                        message: "Pre-order marked as prepared successfully!".to_string(),
                                                                        is_success: true,
                                                                    }));
                                                                    show.set(false);
                                                                    refresh_trigger.set(refresh_trigger() + 1);
                                                                },
                                                                Err(e) => {
                                                                    operation_feedback.set(Some(OperationFeedback {
                                                                        message: "Failed to mark pre-order as prepared".to_string(),
                                                                        is_success: false,
                                                                    }));
                                                                    eprintln!("Error marking pre-order as prepared: {:?}", e);
                                                                }
                                                            }
                                                        });
                                                    }
                                                }
                                            },
                                            img {
                                                class: "w-8 opacity-70",
                                                src: asset!("/assets/icons/open-box.png")
                                            }
                                        }
                                    }
                                }
                            } else {
                                // Regular order view: existing logic
                                if order.prepared_at.is_none() && !is_backordered {
                                    div {
                                        class: "relative",
                                        // Dropdown button
                                        button {
                                            class: "text-gray-600 hover:text-gray-800 p-2 rounded-md hover:bg-gray-100",
                                            onclick: move |_| dropdown_open.set(!dropdown_open()),
                                            img {
                                                class: "w-8 opacity-70",
                                                src: asset!("/assets/icons/open-box.png")
                                            }
                                        }
                                        // Dropdown menu
                                        if dropdown_open() {
                                            div {
                                                class: "absolute right-0 top-full mt-1 w-48 bg-white border border-gray-200 rounded-md shadow-lg z-10",
                                                button {
                                                    class: "w-full text-left px-4 py-2 hover:bg-gray-50 text-sm",
                                                    onclick: move |_| {
                                                        let order_id = order_clone.id.clone();
                                                        let mut refresh_trigger = refresh_trigger.clone();
                                                        let mut operation_feedback = operation_feedback.clone();
                                                        spawn(async move {
                                                            match server_functions::admin_set_order_prepared(order_id).await {
                                                                Ok(_) => {
                                                                    // Success - close dropdown and modal
                                                                    operation_feedback.set(Some(OperationFeedback {
                                                                        message: "Order marked as packaged successfully!".to_string(),
                                                                        is_success: true,
                                                                    }));
                                                                    dropdown_open.set(false);
                                                                    show.set(false);
                                                                    refresh_trigger.set(refresh_trigger() + 1);
                                                                },
                                                                Err(e) => {
                                                                    // Handle error
                                                                    operation_feedback.set(Some(OperationFeedback {
                                                                        message: "Failed to mark order as packaged".to_string(),
                                                                        is_success: false,
                                                                    }));
                                                                    eprintln!("Error marking order as prepared: {:?}", e);
                                                                    dropdown_open.set(false);
                                                                }
                                                            }
                                                        });
                                                    },
                                                    "Mark as packaged"
                                                }
                                            }
                                        }
                                    }
                                } else if is_backordered && order.prepared_at.is_none() {
                                    div {
                                        class: "relative",
                                        button {
                                            class: "text-gray-400 p-2 rounded-md cursor-not-allowed opacity-50",
                                            disabled: true,
                                            title: "Cannot package order with backordered items",
                                            img {
                                                class: "w-8 opacity-50",
                                                src: asset!("/assets/icons/open-box.png")
                                            }
                                        }
                                        div {
                                            class: "absolute right-0 top-full mt-1 text-xs text-gray-500 whitespace-nowrap",
                                            "Cannot package - backordered items"
                                        }
                                    }
                                } else {
                                    div {
                                        class: "w-full flex",
                                        // Only show input and button if order is not fulfilled
                                        if order.status != OrderStatus::Fulfilled {
                                            // For express shipping, only show button (no input)
                                            if order.shipping_option == ShippingOption::Express {
                                                button {
                                                    class: "text-gray-600 hover:text-gray-800 p-2 rounded-md hover:bg-gray-100",
                                                    onclick: move |_| {
                                                        let order_id = order_clone.id.clone();
                                                        let mut refresh_trigger = refresh_trigger.clone();
                                                        let mut operation_feedback = operation_feedback.clone();
                                                        spawn(async move {
                                                            match server_functions::admin_express_fulfilled_notracking(order_id).await {
                                                                Ok(_) => {
                                                                    // Success - close dropdown and modal
                                                                    operation_feedback.set(Some(OperationFeedback {
                                                                        message: "Express order fulfilled successfully!".to_string(),
                                                                        is_success: true,
                                                                    }));
                                                                    dropdown_open.set(false);
                                                                    show.set(false);
                                                                    refresh_trigger.set(refresh_trigger() + 1);
                                                                },
                                                                Err(e) => {
                                                                    // Handle error
                                                                    operation_feedback.set(Some(OperationFeedback {
                                                                        message: "Failed to fulfill express order".to_string(),
                                                                        is_success: false,
                                                                    }));
                                                                    eprintln!("Error marking express order as fulfilled: {:?}", e);
                                                                    dropdown_open.set(false);
                                                                }
                                                            }
                                                        });
                                                    },
                                                    img {
                                                        class: "w-8 opacity-70",
                                                        src: asset!("/assets/icons/checkbox-outline.svg")
                                                    }
                                                }
                                            } else {
                                                // For non-express shipping, show both input and button
                                                input {
                                                    r#type: "text",
                                                    class: "bg-gray-50 border border-gray-200 rounded-md w-full py-1.5 px-3",
                                                    placeholder: "Tracking URL",
                                                    oninput: move |event: FormEvent| tracking_url.set(event.value())
                                                }
                                                button {
                                                    class: "text-gray-600 hover:text-gray-800 p-2 rounded-md hover:bg-gray-100",
                                                    onclick: move |_| {
                                                        let order_id = order_clone.id.clone();
                                                        let tracking_url_str = tracking_url().clone();
                                                        let mut refresh_trigger = refresh_trigger.clone();
                                                        let mut operation_feedback = operation_feedback.clone();
                                                        if tracking_url_str.len() > 0 {
                                                            spawn(async move {
                                                                match server_functions::admin_set_order_fulfilled(order_id, tracking_url_str).await {
                                                                    Ok(_) => {
                                                                        // Success - close dropdown and modal
                                                                        operation_feedback.set(Some(OperationFeedback {
                                                                            message: "Order fulfilled with tracking URL!".to_string(),
                                                                            is_success: true,
                                                                        }));
                                                                        tracking_url.set("".to_string());
                                                                        dropdown_open.set(false);
                                                                        show.set(false);
                                                                        refresh_trigger.set(refresh_trigger() + 1);
                                                                    },
                                                                    Err(e) => {
                                                                        // Handle error
                                                                        operation_feedback.set(Some(OperationFeedback {
                                                                            message: "Failed to fulfill order".to_string(),
                                                                            is_success: false,
                                                                        }));
                                                                        eprintln!("Error marking order as fulfilled: {:?}", e);
                                                                        dropdown_open.set(false);
                                                                    }
                                                                }
                                                            });
                                                        } else {
                                                            operation_feedback.set(Some(OperationFeedback {
                                                                message: "Please enter a tracking URL".to_string(),
                                                                is_success: false,
                                                            }));
                                                        }
                                                    },
                                                    img {
                                                        class: "w-8 opacity-70",
                                                        src: asset!("/assets/icons/checkbox-outline.svg")
                                                    }
                                                }
                                            }
                                        } else {
                                            // Special case: Fulfilled Express order without tracking URL
                                            if order.shipping_option == ShippingOption::Express && order.tracking_url.is_none() {
                                                input {
                                                    r#type: "text",
                                                    class: "bg-gray-50 border border-gray-200 rounded-md w-full py-1.5 px-3",
                                                    placeholder: "Add tracking URL",
                                                    oninput: move |event: FormEvent| tracking_url.set(event.value())
                                                }
                                                button {
                                                    class: "text-gray-600 hover:text-gray-800 p-2 rounded-md hover:bg-gray-100",
                                                    onclick: move |_| {
                                                        let order_id = order_clone.id.clone();
                                                        let tracking_url_str = tracking_url().clone();
                                                        let mut refresh_trigger = refresh_trigger.clone();
                                                        let mut operation_feedback = operation_feedback.clone();
                                                        if tracking_url_str.len() > 0 {
                                                            spawn(async move {
                                                                match server_functions::admin_express_order_send_tracking(order_id, tracking_url_str).await {
                                                                    Ok(_) => {
                                                                        // Success - close modal
                                                                        operation_feedback.set(Some(OperationFeedback {
                                                                            message: "Tracking URL added successfully!".to_string(),
                                                                            is_success: true,
                                                                        }));
                                                                        tracking_url.set("".to_string());
                                                                        show.set(false);
                                                                        refresh_trigger.set(refresh_trigger() + 1);
                                                                    },
                                                                    Err(e) => {
                                                                        // Handle error
                                                                        operation_feedback.set(Some(OperationFeedback {
                                                                            message: "Failed to add tracking URL".to_string(),
                                                                            is_success: false,
                                                                        }));
                                                                        eprintln!("Error adding tracking URL: {:?}", e);
                                                                    }
                                                                }
                                                            });
                                                        } else {
                                                            operation_feedback.set(Some(OperationFeedback {
                                                                message: "Please enter a tracking URL".to_string(),
                                                                is_success: false,
                                                            }));
                                                        }
                                                    },
                                                    img {
                                                        class: "w-8 opacity-70",
                                                        src: asset!("/assets/icons/checkbox-outline.svg")
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Close button
                            button {
                                class: "text-gray-400 hover:text-gray-600 text-2xl",
                                onclick: move |_| show.set(false),
                                "×"
                            }
                        }
                    }

                    // Backorder notice (only for regular order view)
                    if is_backordered && !is_preorder_view {
                        div {
                            class: "bg-orange-50 border-l-4 border-orange-400 p-4 mx-6 mt-2",
                            div {
                                class: "flex",
                                div {
                                    class: "flex-shrink-0",
                                    svg {
                                        class: "h-5 w-5 text-orange-400",
                                        fill: "currentColor",
                                        view_box: "0 0 20 20",
                                        path {
                                            fill_rule: "evenodd",
                                            d: "M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z",
                                            clip_rule: "evenodd"
                                        }
                                    }
                                }
                                div {
                                    class: "ml-3",
                                    h3 {
                                        class: "text-sm font-medium text-orange-800",
                                        "Backordered Items"
                                    }
                                    div {
                                        class: "mt-2 text-sm text-orange-700",
                                        p {
                                            "This order contains items that are currently out of stock. The order cannot be packaged until all items are available."
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Pre-order notice (only for regular order view with preorders)
                    if contains_preorders && !is_preorder_view {
                        div {
                            class: "bg-blue-50 border-l-4 border-blue-400 p-4 mx-6 mt-2",
                            div {
                                class: "flex",
                                div {
                                    class: "flex-shrink-0",
                                    svg {
                                        class: "h-5 w-5 text-blue-400",
                                        fill: "currentColor",
                                        view_box: "0 0 20 20",
                                        path {
                                            fill_rule: "evenodd",
                                            d: "M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z",
                                            clip_rule: "evenodd"
                                        }
                                    }
                                }
                                div {
                                    class: "ml-3",
                                    h3 {
                                        class: "text-sm font-medium text-blue-800",
                                        "Pre-Order Notice"
                                    }
                                    div {
                                        class: "mt-2 text-sm text-blue-700",
                                        p {
                                            "This order contains a pre-order which is available in the Pre-Orders tab."
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Content
                    div {
                        class: "p-6 grid bg-gray-100 grid-cols-1 lg:grid-cols-2 gap-8",

                        // LEFT SIDE
                        div {
                            class: "space-y-6",

                            // Order ref and email
                            div {
                                class: "space-y-3",
                                div {
                                    class: "flex items-center gap-2",
                                    span { class: "text-gray-600", "Email:" }
                                    button {
                                        class: "text-blue-600 hover:text-blue-800 underline",
                                        onclick: {
                                            let email = order.customer_email.clone();
                                            move |_| copy_to_clipboard(email.clone())
                                        },
                                        "{order.customer_email}"
                                    }
                                }
                            }

                            // Order items section
                            div {
                                class: "space-y-3",
                                h3 {
                                    class: "font-semibold text-gray-900",
                                    if is_preorder_view {
                                        "Pre-Order Items"
                                    } else {
                                        "Order Items"
                                    }
                                }
                                div {
                                    class: "space-y-2",
                                    {
                                        // Filter items based on view mode
                                        let items_to_display: Vec<&OrderShortItem> = if is_preorder_view {
                                            if let Some(ref selected_item) = selected_preorder_item {
                                                vec![selected_item]
                                            } else {
                                                order.items.iter().filter(|item| item.pre_order_on_purchase).collect()
                                            }
                                        } else {
                                            order.items.iter().collect()
                                        };

                                        rsx! {
                                            for item in items_to_display {
                                                {
                                                    let item_class = if is_item_backordered(item, &order.backorder_reduces) {
                                                        "bg-orange-50 border-2 border-orange-200 rounded-md p-3"
                                                    } else if item.pre_order_on_purchase && !is_preorder_view {
                                                        "bg-blue-50 border-2 border-blue-200 rounded-md p-3 relative opacity-70"
                                                    } else if item.pre_order_on_purchase && is_preorder_view {
                                                        "bg-blue-50 border-2 border-blue-200 rounded-md p-3"
                                                    } else {
                                                        "bg-white border rounded-md border-gray-200 p-3 rounded"
                                                    };

                                                    rsx! {
                                                        div {
                                                            class: "{item_class}",

                                                            // Pre-order overlay text (only for regular order view)
                                                            if item.pre_order_on_purchase && !is_preorder_view {
                                                                div {
                                                                    class: "absolute top-0 left-0 right-0 bottom-0 flex items-center justify-center bg-blue-100 bg-opacity-75 rounded-md",
                                                                    span {
                                                                        class: "text-blue-800 font-semibold text-sm",
                                                                        "To be fulfilled separately"
                                                                    }
                                                                }
                                                            }

                                                            div {
                                                                class: "font-medium",
                                                                "{item.quantity}x {item.product_title}"
                                                                if !item.variant_name.is_empty() {
                                                                    span { class: "text-gray-600", " ({item.variant_name})" }
                                                                }
                                                                if is_item_backordered(item, &order.backorder_reduces) {
                                                                    span {
                                                                        class: "ml-2 px-2 py-1 bg-orange-100 text-orange-800 rounded text-xs font-medium",
                                                                        "BACKORDERED"
                                                                    }
                                                                } else if item.pre_order_on_purchase {
                                                                    span {
                                                                        class: "ml-2 px-2 py-1 bg-blue-100 text-blue-800 rounded text-xs font-medium",
                                                                        "PRE-ORDER"
                                                                    }
                                                                }
                                                            }
                                                            div {
                                                                class: "text-sm text-gray-600",
                                                                { format!("Price per item: ${:.2}, Total: ${:.2}", item.price_usd, item.price_usd * item.quantity as f64) }
                                                            }
                                                            if is_item_backordered(item, &order.backorder_reduces) {
                                                                div {
                                                                    class: "text-xs text-orange-700 font-medium mt-1",
                                                                    "⚠ This item is currently out of stock"
                                                                }
                                                            } else if item.pre_order_on_purchase && is_preorder_view {
                                                                div {
                                                                    class: "text-xs text-blue-700 font-medium mt-1",
                                                                    if is_item_preordered(item, &order.preorder_reduces) {
                                                                        "⚠ This pre-order is currently unstocked"
                                                                    } else {
                                                                        "✓ This pre-order is ready to fulfill"
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

                            // Order information
                            div {
                                class: "space-y-3",
                                h3 { class: "font-semibold text-gray-900", "Order Information" }
                                div {
                                    class: "space-y-2 text-sm",
                                    div {
                                        class: "flex justify-between",
                                        span { class: "text-gray-600", "Status:" }
                                        span { class: "font-medium", "{get_order_status_display(&order)}" }
                                    }
                                    div {
                                        class: "flex justify-between",
                                        span { class: "text-gray-600", "Created:" }
                                        span { { order.created_at.format("%Y-%m-%d %H:%M").to_string() } }
                                    }
                                    {if let Some(prepared_at) = order.prepared_at {
                                        rsx! {
                                            div {
                                                class: "flex justify-between",
                                                span { class: "text-gray-600", "Prepared:" }
                                                span { { prepared_at.format("%Y-%m-%d %H:%M").to_string() } }
                                            }
                                        }
                                    } else {
                                        rsx! {}
                                    }}
                                    {if let Some(fulfilled_at) = order.fulfilled_at {
                                        rsx! {
                                            div {
                                                class: "flex justify-between",
                                                span { class: "text-gray-600", "Fulfilled:" }
                                                span { { fulfilled_at.format("%Y-%m-%d %H:%M").to_string() } }
                                            }
                                        }
                                    } else {
                                        rsx! {}
                                    }}

                                    // Show tracking URL - conditionally for pre-order view
                                    {
                                        let show_tracking = if is_preorder_view {
                                            if let Some(ref item) = selected_preorder_item {
                                                if let Some(pre_order) = order.pre_orders.iter().find(|po| po.order_item_id == item.id) {
                                                    pre_order.tracking_url.is_some() && pre_order.fulfilled_at.is_some()
                                                } else {
                                                    false
                                                }
                                            } else {
                                                false
                                            }
                                        } else {
                                            order.tracking_url.is_some()
                                        };

                                        if show_tracking {
                                            let tracking_url = if is_preorder_view {
                                                if let Some(ref item) = selected_preorder_item {
                                                    order.pre_orders.iter()
                                                        .find(|po| po.order_item_id == item.id)
                                                        .and_then(|po| po.tracking_url.as_ref())
                                                } else {
                                                    None
                                                }
                                            } else {
                                                order.tracking_url.as_ref()
                                            };

                                            if let Some(url) = tracking_url {
                                                rsx! {
                                                    div {
                                                        class: "flex justify-between",
                                                        span { class: "text-gray-600", "Tracking:" }
                                                        a {
                                                            href: "{url}",
                                                            target: "_blank",
                                                            class: "text-blue-600 hover:text-blue-800 underline",
                                                            "View tracking"
                                                        }
                                                    }
                                                }
                                            } else {
                                                rsx! {}
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }

                                    // Only show pricing for regular order view
                                    if !is_preorder_view {
                                        div {
                                            class: "flex justify-between font-semibold border-t pt-2 mt-2",
                                            span { "Subtotal:" }
                                            span { { format!("${:.2}", order.subtotal_usd) } }
                                        }
                                        div {
                                            class: "flex justify-between font-semibold border-t pt-2 mt-2",
                                            span { "Shipping:" }
                                            span { { format!("${:.2}", order.shipping_usd) } }
                                        }
                                        div {
                                            class: "flex justify-between font-semibold border-t pt-2 mt-2",
                                            span { "Total:" }
                                            span { { format!("${:.2}", order.total_amount_usd) } }
                                        }
                                    }
                                }
                            }

                            // Payments (only for regular order view)
                            if !order.payments.is_empty() && !is_preorder_view {
                                div {
                                    class: "space-y-3",
                                    h3 { class: "font-semibold text-gray-900", "Payments" }
                                    div {
                                        class: "space-y-2",
                                        for payment in &order.payments {
                                            div {
                                                class: "bg-white border rounded-md border-gray-200 p-3 rounded text-sm",
                                                div {
                                                    class: "flex justify-between",
                                                    span { { payment.method.clone() } }
                                                    span { class: "font-medium", { format!("${:.2}", payment.amount_usd) } }
                                                }
                                                div {
                                                    class: "text-gray-600",
                                                    { format!("Status: {}", payment.status) }
                                                }
                                                {if let Some(paid_at) = payment.paid_at {
                                                    rsx! {
                                                        div {
                                                            class: "text-gray-600",
                                                            { format!("Paid: {}", paid_at.format("%Y-%m-%d %H:%M")) }
                                                        }
                                                    }
                                                } else {
                                                    rsx! {}
                                                }}
                                            }
                                        }
                                    }
                                }
                            }

                            // Notes
                            div {
                                class: "space-y-3",
                                h3 { class: "font-semibold text-gray-900", "Notes" }
                                textarea {
                                    class: "w-full bg-white border border-gray-200 rounded-md p-2 text-sm resize-none",
                                    rows: "3",
                                    placeholder: "Add a note...",
                                    oninput: move |event: FormEvent| notes_text.set(event.value()),
                                    "{notes_text}"
                                }
                                button {
                                    class: "px-3 py-1.5 bg-zinc-600 text-white text-sm rounded hover:bg-zinc-500",
                                    onclick: {
                                        let order_id = order.id.clone();
                                        move |_| {
                                            let order_id = order_id.clone();
                                            let notes = notes_text();
                                            let mut operation_feedback = operation_feedback.clone();
                                            spawn(async move {
                                                match server_functions::admin_update_order_notes(order_id, notes).await {
                                                    Ok(_) => {
                                                        operation_feedback.set(Some(OperationFeedback {
                                                            message: "Notes saved".to_string(),
                                                            is_success: true,
                                                        }));
                                                    }
                                                    Err(e) => {
                                                        operation_feedback.set(Some(OperationFeedback {
                                                            message: format!("Failed to save notes: {}", e),
                                                            is_success: false,
                                                        }));
                                                    }
                                                }
                                            });
                                        }
                                    },
                                    "Save"
                                }
                            }
                        }

                        // RIGHT SIDE - Show address for both regular and preorder views
                        div {
                            class: "space-y-6",

                            // Address Info
                            {if let Some(address) = &order.address {
                                rsx! {
                                    div {
                                        class: "space-y-3",
                                        h3 {
                                            class: "font-semibold text-gray-900", {
                                                let mut weight = order.order_weight;
                                                if let Some(ref item) = selected_preorder_item {
                                                    if let Some(matching_preorder) = order.pre_orders.iter().find(|po| po.order_item_id == item.id) {
                                                        weight = matching_preorder.pre_order_weight;
                                                    }
                                                }


                                                format!("Shipping ({} @{:.0}g)",
                                                    order.shipping_option.to_string(),
                                                    weight
                                                )
                                            }
                                        }
                                        div {
                                            class: "space-y-2 text-sm",

                                            // Name
                                            div {
                                                button {
                                                    class: "text-left hover:bg-gray-50 p-2 rounded w-full",
                                                    onclick: {
                                                        let name = format!("{} {}", address.first_name, address.last_name);
                                                        move |_| copy_to_clipboard(name.clone())
                                                    },
                                                    div { class: "font-medium", { format!("{} {}", address.first_name, address.last_name) } }
                                                    div { class: "text-xs text-gray-500", "Click to copy" }
                                                }
                                            }

                                            // Company
                                            {if let Some(company) = &address.company {
                                                rsx! {
                                                    div {
                                                        button {
                                                            class: "text-left hover:bg-gray-50 p-2 rounded w-full",
                                                            onclick: {
                                                                let company = company.clone();
                                                                move |_| copy_to_clipboard(company.clone())
                                                            },
                                                            div { class: "font-medium", { company.clone() } }
                                                            div { class: "text-xs text-gray-500", "Click to copy" }
                                                        }
                                                    }
                                                }
                                            } else {
                                                rsx! {}
                                            }}

                                            // Address line 1
                                            div {
                                                button {
                                                    class: "text-left hover:bg-gray-50 p-2 rounded w-full",
                                                    onclick: {
                                                        let addr = address.address_line_1.clone();
                                                        move |_| copy_to_clipboard(addr.clone())
                                                    },
                                                    div { class: "font-medium", { address.address_line_1.clone() } }
                                                    div { class: "text-xs text-gray-500", "Click to copy" }
                                                }
                                            }

                                            // Address line 2
                                            {if let Some(address_line_2) = &address.address_line_2.clone() {
                                                rsx! {
                                                    div {
                                                        button {
                                                            class: "text-left hover:bg-gray-50 p-2 rounded w-full",
                                                            onclick: {
                                                                let addr = address_line_2.clone();
                                                                move |_| copy_to_clipboard(addr.clone())
                                                            },
                                                            div { class: "font-medium", { address_line_2.clone() } }
                                                            div { class: "text-xs text-gray-500", "Click to copy" }
                                                        }
                                                    }
                                                }
                                            } else {
                                                rsx! {}
                                            }}

                                            // City
                                            div {
                                                button {
                                                    class: "text-left hover:bg-gray-50 p-2 rounded w-full",
                                                    onclick: {
                                                        let city = address.city.clone();
                                                        move |_| copy_to_clipboard(city.clone())
                                                    },
                                                    div { class: "font-medium", { address.city.clone() } }
                                                    div { class: "text-xs text-gray-500", "Click to copy" }
                                                }
                                            }

                                            // Post code
                                            div {
                                                button {
                                                    class: "text-left hover:bg-gray-50 p-2 rounded w-full",
                                                    onclick: {
                                                        let post_code = address.post_code.clone();
                                                        move |_| copy_to_clipboard(post_code.clone())
                                                    },
                                                    div { class: "font-medium", { address.post_code.clone() } }
                                                    div { class: "text-xs text-gray-500", "Post code - Click to copy" }
                                                }
                                            }

                                            // Province
                                            {if let Some(province) = &address.province {
                                                rsx! {
                                                    div {
                                                        button {
                                                            class: "text-left hover:bg-gray-50 p-2 rounded w-full",
                                                            onclick: {
                                                                let province = province.clone();
                                                                move |_| copy_to_clipboard(province.clone())
                                                            },
                                                            div { class: "font-medium", { province.clone() } }
                                                            div { class: "text-xs text-gray-500", "Province - Click to copy" }
                                                        }
                                                    }
                                                }
                                            } else {
                                                rsx! {}
                                            }}

                                            // Country
                                            {if let Some(country_iso) = &address.country {
                                                rsx! {
                                                    div {
                                                        button {
                                                            class: "text-left hover:bg-gray-50 p-2 rounded w-full",
                                                            onclick: {
                                                                let country = utils::countries::country_display_name_from_iso(country_iso);
                                                                move |_| copy_to_clipboard(country.clone())
                                                            },
                                                            div {
                                                                class: "font-medium",
                                                                { format!("{} ({})", country_iso, utils::countries::country_display_name_from_iso(country_iso)) }
                                                            }
                                                            div { class: "text-xs text-gray-500", "Click to copy" }
                                                        }
                                                    }
                                                }
                                            } else {
                                                rsx! {}
                                            }}

                                            // Phone
                                            {if let Some(phone) = &address.phone {
                                                rsx! {
                                                    div {
                                                        button {
                                                            class: "text-left hover:bg-gray-50 p-2 rounded w-full",
                                                            onclick: {
                                                                let phone = phone.clone();
                                                                move |_| copy_to_clipboard(phone.clone())
                                                            },
                                                            div { class: "font-medium", { phone.clone() } }
                                                            div { class: "text-xs text-gray-500", "Phone - Click to copy" }
                                                        }
                                                    }
                                                }
                                            } else {
                                                rsx! {}
                                            }}
                                        }
                                    }
                                }
                            } else {
                                rsx! {}
                            }}
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn AdminOrders() -> Element {
    let mut active_tab = use_signal(|| OrderTab::All);
    let mut selected_timespan = use_signal(|| TimeSpan::Days30);
    let mut show_order_modal = use_signal(|| false);
    let mut selected_order: Signal<Option<OrderInfo>> = use_signal(|| None);
    let mut selected_preorder_item: Signal<Option<OrderShortItem>> = use_signal(|| None);
    let mut refresh_trigger = use_signal(|| 0u32);

    // Use cached server function
    let orders_req = use_resource(move || async move {
        let _ = refresh_trigger();
        server_functions::admin_get_orders(false).await
    });

    rsx! {
        div {
            class: "w-full",

            // Modal
            if let Some(order) = selected_order.read().as_ref() {
                OrderDetailModal {
                    order: order.clone(),
                    show: show_order_modal,
                    refresh_trigger: refresh_trigger,
                    is_preorder_view: *active_tab.read() == OrderTab::PreOrders,
                    selected_preorder_item: selected_preorder_item.read().clone(),
                }

            }

            // Stats bar
            div {
                class: "bg-white border rounded-md border-gray-200 p-4 mb-4 flex flex-wrap gap-3 items-center justify-between",

                // Time selector
                div {
                    class: "flex flex-wrap items-center gap-2",
                    span {
                        class: "text-sm text-gray-600",
                        "{selected_timespan.read().days()}d"
                    }
                    div {
                        class: "flex flex-wrap gap-1",
                        for timespan in [TimeSpan::Days7, TimeSpan::Days30, TimeSpan::Days60, TimeSpan::Days90, TimeSpan::Days180, TimeSpan::Days360] {
                            button {
                                class: if *selected_timespan.read() == timespan {
                                    "px-2 py-1 bg-zinc-600 text-white rounded text-xs"
                                } else {
                                    "px-2 py-1 bg-gray-100 text-gray-700 rounded text-xs hover:bg-gray-200"
                                },
                                onclick: move |_| selected_timespan.set(timespan.clone()),
                                "{timespan}"
                            }
                        }
                    }
                }

                // Stats
                {match &*orders_req.read() {
                    Some(Ok(orders)) => {
                        let (total_orders, total_items, fulfilled_orders, avg_time) =
                            calculate_stats(orders, &selected_timespan.read());

                        rsx! {
                            div {
                                class: "flex flex-wrap gap-4",

                                div {
                                    class: "text-center",
                                    div { class: "text-xs text-gray-500 uppercase tracking-wide", "Orders" }
                                    div { class: "text-xl font-semibold", "{total_orders}" }
                                }

                                div {
                                    class: "text-center",
                                    div { class: "text-xs text-gray-500 uppercase tracking-wide", "Items" }
                                    div { class: "text-xl font-semibold", "{total_items}" }
                                }

                                div {
                                    class: "text-center",
                                    div { class: "text-xs text-gray-500 uppercase tracking-wide", "Fulfilled" }
                                    div { class: "text-xl font-semibold", "{fulfilled_orders}" }
                                }

                                div {
                                    class: "text-center",
                                    div { class: "text-xs text-gray-500 uppercase tracking-wide", "Avg Time" }
                                    div { class: "text-xl font-semibold", "{avg_time:.1}d" }
                                }
                            }
                        }
                    },
                    _ => rsx! {
                        div {
                            class: "flex gap-4",
                            div {
                                class: "text-center",
                                div { class: "text-xs text-gray-500 uppercase tracking-wide", "Loading..." }
                                div { class: "text-xl font-semibold", "—" }
                            }
                        }
                    }
                }}
            }

            // Orders container
            div {
                class: "bg-white border rounded-md border-gray-200",

                // Header with tabs
                div {
                    class: "border-b border-gray-200",
                    div {
                        class: "flex overflow-x-auto",
                        for tab in [OrderTab::All, OrderTab::Unfulfilled, OrderTab::PreOrders, OrderTab::Packaged, OrderTab::Fulfilled] {
                            button {
                                class: if *active_tab.read() == tab {
                                    "px-4 py-3 text-sm font-medium text-gray-900 border-b-2 border-gray-900 whitespace-nowrap shrink-0"
                                } else {
                                    "px-4 py-3 text-sm font-medium text-gray-500 hover:text-gray-700 border-b-2 border-transparent hover:border-gray-300 whitespace-nowrap shrink-0"
                                },
                                onclick: move |_| active_tab.set(tab.clone()),
                                "{tab}"
                                {match &*orders_req.read() {
                                    Some(Ok(orders)) => {
                                        let count = orders.iter().filter(|o| order_matches_tab(o, &tab)).count();
                                        if count > 0 {
                                            rsx! {
                                                span {
                                                    class: "ml-2 px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded-full",
                                                    "{count}"
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    },
                                    _ => rsx! {}
                                }}
                            }
                        }
                    }
                }

                // Column headers (desktop only)
                div {
                    class: "hidden md:grid grid-cols-12 gap-4 px-4 py-3 bg-gray-50 border-b border-gray-200 text-xs font-medium text-gray-500 uppercase tracking-wide",
                    div { class: "col-span-1", "" }
                    div { class: "col-span-2", "Order" }
                    div { class: "col-span-2", "Date" }
                    div { class: "col-span-2", "Total" }
                    div { class: "col-span-2", "Status" }
                    div { class: "col-span-2", "Items" }
                    div { class: "col-span-1", "" }
                }

                // Orders list
                div {
                    class: "divide-y divide-gray-200",
                    {match &*orders_req.read() {
                        Some(Ok(orders)) => {
                            let mut filtered_orders: Vec<&OrderInfo> = orders.iter()
                                .filter(|order| order_matches_tab(order, &active_tab.read()))
                                .collect();
                            filtered_orders.sort_by(|a, b| a.created_at.cmp(&b.created_at));

                            if filtered_orders.is_empty() {
                                rsx! {
                                    div {
                                        class: "p-8 text-center text-gray-500",
                                        "No orders found"
                                    }
                                }
                            } else {
                                rsx! {
                                    for order in filtered_orders.iter().rev() {
                                        {
                                            let contains_preorders = has_preorders(order);
                                            let is_preorder_tab = *active_tab.read() == OrderTab::PreOrders;

                                            if is_preorder_tab {
                                                // Pre-order tab: show only pre-order items
                                                let preorder_items: Vec<&OrderShortItem> = order.items.iter()
                                                    .filter(|item| item.pre_order_on_purchase)
                                                    .collect();

                                                rsx! {
                                                    for item in preorder_items {
                                                        div {
                                                            class: "md:grid md:grid-cols-12 md:gap-4 px-4 py-3 md:py-1 hover:bg-gray-50 transition-colors md:min-h-[40px] md:items-center",

                                                            // Checkbox (desktop only)
                                                            div {
                                                                class: "col-span-1 hidden md:block",
                                                                input {
                                                                    r#type: "checkbox",
                                                                    class: "rounded border-gray-300 text-gray-900 focus:ring-gray-500",
                                                                    onclick: |e| e.stop_propagation(),
                                                                }
                                                            }

                                                            // Order number
                                                            div {
                                                                class: "col-span-2 cursor-pointer",
                                                                onclick: {
                                                                    let order_clone = (*order).clone();
                                                                    let item_clone = (*item).clone();
                                                                    move |_| {
                                                                        selected_order.set(Some(order_clone.clone()));
                                                                        selected_preorder_item.set(Some(item_clone.clone()));
                                                                        show_order_modal.set(true);
                                                                    }
                                                                },
                                                                div {
                                                                    class: "font-medium text-gray-900",
                                                                    "#{order.ref_code}"
                                                                }
                                                                div {
                                                                    class: "text-xs text-gray-500",
                                                                    "{order.customer_email}"
                                                                }
                                                            }

                                                            // Mobile card row: date + status
                                                            div {
                                                                class: "flex md:contents items-center justify-between gap-2 mt-1",
                                                                onclick: {
                                                                    let order_clone = (*order).clone();
                                                                    let item_clone = (*item).clone();
                                                                    move |_| {
                                                                        selected_order.set(Some(order_clone.clone()));
                                                                        selected_preorder_item.set(Some(item_clone.clone()));
                                                                        show_order_modal.set(true);
                                                                    }
                                                                },
                                                                // Date
                                                                span { class: "col-span-2 text-sm text-gray-500", "{format_date(&order.created_at)}" }
                                                                // Status
                                                                {
                                                                    let status = get_preorder_status(item, &order.preorder_reduces, &order.pre_orders);
                                                                    let status_class = if status == "Unstocked" {
                                                                        "col-span-2 px-2 py-1 bg-red-100 text-red-800 rounded-full text-xs font-medium"
                                                                    } else if status == "Packaged" || status == "Fulfilled" {
                                                                        "col-span-2 px-2 py-1 bg-green-100 text-green-800 rounded-full text-xs font-medium"
                                                                    } else if status == "Untracked" {
                                                                        "col-span-2 px-2 py-1 bg-yellow-100 text-yellow-800 rounded-full text-xs font-medium"
                                                                    } else {
                                                                        "col-span-2 px-2 py-1 bg-blue-100 text-blue-800 rounded-full text-xs font-medium"
                                                                    };
                                                                    rsx! { span { class: "{status_class}", "{status}" } }
                                                                }
                                                                // Total placeholder (desktop only)
                                                                span { class: "col-span-2 hidden md:block text-gray-400 text-sm", "—" }
                                                            }

                                                            // Items column
                                                            div {
                                                                class: "col-span-2 text-sm text-gray-900 cursor-pointer mt-1 md:mt-0",
                                                                onclick: {
                                                                    let order_clone = (*order).clone();
                                                                    let item_clone = (*item).clone();
                                                                    move |_| {
                                                                        selected_order.set(Some(order_clone.clone()));
                                                                        selected_preorder_item.set(Some(item_clone.clone()));
                                                                        show_order_modal.set(true);
                                                                    }
                                                                },
                                                                "{item.product_title}"
                                                                if !item.variant_name.is_empty() {
                                                                    div {
                                                                        class: "text-xs text-gray-500 truncate",
                                                                        "({item.variant_name})"
                                                                    }
                                                                }
                                                            }

                                                            // Action button
                                                            div {
                                                                class: "col-span-1 hidden md:flex justify-center",
                                                                a {
                                                                    href: "#",
                                                                    title: "View order",
                                                                    class: "flex items-center justify-center w-8 h-8 rounded hover:bg-gray-100 transition-colors",
                                                                    onclick: |e| e.prevent_default(),
                                                                    img { class: "w-5 h-5", src: asset!("/assets/icons/create-outline.svg") }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                // Other tabs: show regular order rows
                                                rsx! {
                                                    div {
                                                        class: "md:grid md:grid-cols-12 md:gap-4 px-4 py-3 md:py-1 hover:bg-gray-50 transition-colors md:min-h-[40px] md:items-center cursor-pointer",
                                                        onclick: {
                                                            let order_clone = (*order).clone();
                                                            move |_| {
                                                                selected_order.set(Some(order_clone.clone()));
                                                                selected_preorder_item.set(None);
                                                                show_order_modal.set(true);
                                                            }
                                                        },

                                                        // Checkbox (desktop only)
                                                        div {
                                                            class: "col-span-1 hidden md:block",
                                                            input {
                                                                r#type: "checkbox",
                                                                class: "rounded border-gray-300 text-gray-900 focus:ring-gray-500",
                                                                onclick: |e| e.stop_propagation(),
                                                            }
                                                        }

                                                        // Order number (always visible)
                                                        div {
                                                            class: "col-span-2",
                                                            div {
                                                                class: "font-medium text-gray-900",
                                                                "#{order.ref_code}"
                                                            }
                                                            div {
                                                                class: "text-xs text-gray-500",
                                                                "{order.customer_email}"
                                                            }
                                                        }

                                                        // Mobile combined row / desktop grid cells: Date + Total + Status
                                                        div {
                                                            class: "flex md:contents flex-wrap items-center gap-2 mt-1 md:mt-0",
                                                            // Date
                                                            span { class: "col-span-2 text-sm text-gray-500", "{format_date(&order.created_at)}" }
                                                            // Total
                                                            span {
                                                                class: "col-span-2 font-medium text-gray-900 text-sm",
                                                                {
                                                                    if contains_preorders {
                                                                        format!("${:.2} (IPE)", order.total_amount_usd)
                                                                    } else {
                                                                        format!("${:.2}", order.total_amount_usd)
                                                                    }
                                                                }
                                                            }
                                                            // Status
                                                            div {
                                                                class: "col-span-2",
                                                                span {
                                                                    class: "{get_order_status_class(order)}",
                                                                    {
                                                                        if has_backorders(order) {
                                                                            "Backordered".to_string()
                                                                        } else if order.status == OrderStatus::Pending && order.prepared_at.is_some() {
                                                                            "Prepared".to_string()
                                                                        } else if order.status == OrderStatus::Fulfilled && order.tracking_url.is_none() {
                                                                            "Untracked".to_string()
                                                                        } else {
                                                                            order.status.to_string()
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }

                                                        // Items - exclude pre-order items
                                                        div {
                                                            class: "col-span-2 text-sm text-gray-900 mt-1 md:mt-0",
                                                            {
                                                                let non_preorder_items: Vec<&OrderShortItem> = order.items.iter()
                                                                    .filter(|item| !item.pre_order_on_purchase)
                                                                    .collect();

                                                                let total = non_preorder_items.iter().map(|item| item.quantity).sum::<i32>();
                                                                if total == 0 {
                                                                    "—".to_string()
                                                                } else if total == 1 {
                                                                    "1 item".to_string()
                                                                } else {
                                                                    format!("{} items", total)
                                                                }
                                                            }
                                                            div {
                                                                class: "text-xs text-gray-500 truncate",
                                                                {
                                                                    let non_preorder_items: Vec<&OrderShortItem> = order.items.iter()
                                                                        .filter(|item| !item.pre_order_on_purchase)
                                                                        .collect();

                                                                    if non_preorder_items.is_empty() {
                                                                        "".to_string()
                                                                    } else {
                                                                        let item_names: Vec<String> = non_preorder_items.iter()
                                                                            .take(2)
                                                                            .map(|item| item.product_title.clone())
                                                                            .collect();
                                                                        let display_text = if non_preorder_items.len() > 2 {
                                                                            format!("{}, {} +{} more",
                                                                                item_names.get(0).unwrap_or(&"".to_string()),
                                                                                item_names.get(1).unwrap_or(&"".to_string()),
                                                                                non_preorder_items.len() - 2
                                                                            )
                                                                        } else {
                                                                            item_names.join(", ")
                                                                        };
                                                                        display_text
                                                                    }
                                                                }
                                                            }
                                                        }

                                                        // Edit button (desktop only)
                                                        div {
                                                            class: "col-span-1 hidden md:flex justify-center",
                                                            a {
                                                                href: "#",
                                                                title: "Edit order",
                                                                class: "flex items-center justify-center w-8 h-8 rounded hover:bg-gray-100 transition-colors",
                                                                onclick: |e| e.prevent_default(),
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
                                }
                            }
                        },
                        Some(Err(_)) => rsx! {
                            div {
                                class: "p-8 text-center text-red-500",
                                "Error loading orders"
                            }
                        },
                        None => rsx! {
                            div {
                                class: "p-8 text-center text-gray-500",
                                "Loading orders..."
                            }
                        }
                    }}
                }
            }
        }
    }
}
