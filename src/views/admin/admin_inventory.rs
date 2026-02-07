#![allow(non_snake_case)]

use crate::Route;
use crate::backend::front_entities::*;
use crate::backend::server_functions;
use crate::utils::*;
use dioxus::prelude::*;

#[component]
pub fn AdminInventory() -> Element {
    let stock_items = use_resource(move || async move {
        tracing::info!("Getting stock items for admin inventory management");
        server_functions::admin_get_stock_items().await
    });

    let stock_batches = use_resource(move || async move {
        tracing::info!("Getting stock batches (admin-side)");
        server_functions::admin_get_stock_batches().await
    });

    rsx! {
        div {
            class: "w-full",

            // Header
            div {
                class: "bg-white border rounded-md border-gray-200 p-4 mb-4 h-20 flex justify-between items-center",
                div {
                    class: "text-lg font-medium",
                    "Inventory"
                }
                Link {
                    to: Route::AdminCreateStockItem {},
                    button {
                        class: "text-sm bg-gray-900 px-3 py-2 text-white rounded hover:bg-gray-800 transition-colors",
                        "Create Item"
                    }
                }
            }

            // Inventory table container
            div {
                class: "bg-white border rounded-md border-gray-200",

                // Column headers
                div {
                    class: "grid grid-cols-12 gap-4 px-4 py-3 bg-gray-50 border-b border-gray-200 text-xs font-medium text-gray-500 uppercase tracking-wide",
                    div { class: "col-span-1", "" } // Thumbnail
                    div { class: "col-span-3", "Item" }
                    div { class: "col-span-2", "Ready Stock" }
                    div { class: "col-span-2", "Unmade Stock" }
                    div { class: "col-span-2", "Total Stock" }
                    div { class: "col-span-1", "Batches" }
                    div { class: "col-span-1", "" } // Actions
                }

                // Items list
                div {
                    class: "divide-y divide-gray-200",
                    {match (&*stock_items.read(), &*stock_batches.read()) {
                        (Some(Ok(stock_items)), Some(Ok(stock_batches))) => {
                            if stock_items.len() == 0 {
                                rsx! {
                                    div {
                                        class: "p-8 text-center text-gray-500",
                                        "No inventory items created yet"
                                    }
                                }
                            } else {
                                rsx! {
                                    for stock_item in stock_items.iter() {
                                        {
                                            let stock_item = stock_item.clone();
                                            let stock_batches = stock_batches.clone();
                                            rsx! {
                                                StockItemRow {
                                                    stock_item: stock_item,
                                                    stock_batches: stock_batches
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        (Some(Err(e)), _) => rsx! {
                            div {
                                class: "p-8 text-center text-red-500",
                                { format!("Error loading stock items: {:?}", e) }
                            }
                        },
                        (_, Some(Err(e))) => rsx! {
                            div {
                                class: "p-8 text-center text-red-500",
                                { format!("Error loading stock batches: {:?}", e) }
                            }
                        },
                        _ => rsx! {
                            div {
                                class: "p-8 text-center text-gray-500",
                                "Loading stock data..."
                            }
                        }
                    }}
                }
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct StockItemRowProps {
    stock_item: StockItem,
    stock_batches: Vec<StockBatch>,
}

#[component]
fn StockItemRow(props: StockItemRowProps) -> Element {
    let mut batches_expanded = use_signal(|| false);

    // Filter stock batches for this item
    let item_batches: Vec<StockBatch> = props
        .stock_batches
        .iter()
        .filter(|batch| batch.stock_item_id == props.stock_item.id)
        .cloned()
        .collect();

    // Count batches with paid/complete status and live_quantity > 0
    let active_batch_count = item_batches
        .iter()
        .filter(|batch| {
            matches!(
                batch.status,
                StockBatchStatus::Paid | StockBatchStatus::Complete
            ) && !batch.live_quantity.is_zero()
        })
        .count();

    // Sort batches by created_at desc and take 5 most recent
    let recent_batches = {
        let mut sorted_batches = item_batches.clone();
        sorted_batches.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        sorted_batches.truncate(5);
        sorted_batches
    };

    let get_unit_color = |unit: &StockUnit| -> &'static str {
        match unit {
            StockUnit::Multiples => "bg-blue-100 text-blue-800",
            StockUnit::Grams => "bg-green-100 text-green-800",
            StockUnit::Milliliters => "bg-purple-100 text-purple-800",
        }
    };

    let format_quantity = |qty: &StockUnitQuantity| -> String {
        match qty {
            StockUnitQuantity::Multiples(n) => n.to_string(),
            StockUnitQuantity::Grams(g) => format!("{:.3}", g),
            StockUnitQuantity::Milliliters(ml) => format!("{:.3}", ml),
        }
    };

    let get_unit_suffix = |unit: &StockUnit| -> &'static str {
        match unit {
            StockUnit::Multiples => "",
            StockUnit::Grams => "g",
            StockUnit::Milliliters => "ml",
        }
    };

    let border_class = {
        if let Some(stock_quantities) = &props.stock_item.stock_quantities {
            if stock_quantities.stock_too_low {
                "border-l-4 border-l-red-500"
            } else {
                ""
            }
        } else {
            ""
        }
    };

    rsx! {
        div {
            class: "flex flex-col",

            // Main stock item row
            div {
                class: "grid grid-cols-12 gap-4 px-4 py-4 hover:bg-gray-50 transition-colors {border_class}",

                // Thumbnail
                div {
                    class: "col-span-1 flex items-center",
                    {
                        if let Some(url) = &props.stock_item.thumbnail_ref {
                            rsx! {
                                img {
                                    class: "w-12 h-12 object-cover rounded border",
                                    src: "https://{url}"
                                }
                            }
                        } else {
                            rsx! {
                                div {
                                    class: "w-12 h-12 bg-gray-200 rounded border flex items-center justify-center",
                                    span {
                                        class: "text-gray-500 text-xs",
                                        "N/A"
                                    }
                                }
                            }
                        }
                    }
                }

                // Item info
                div {
                    class: "col-span-3",
                    div {
                        class: "flex items-center gap-2 mb-1",
                        Link {
                            to: Route::AdminEditStockItem { id: format!("{}", props.stock_item.id) },
                            title: "Edit stock item",
                            class: "font-medium text-gray-900 hover:underline",
                            "{props.stock_item.pbi_sku}"
                        }
                        span {
                            class: "px-2 py-0.5 rounded-full text-xs font-medium {get_unit_color(&props.stock_item.unit)}",
                            "{props.stock_item.unit}"
                        }
                        {
                            if let Some(stock_quantities) = &props.stock_item.stock_quantities {
                                if stock_quantities.stock_too_low {
                                    rsx! {
                                        span {
                                            class: "px-2 py-0.5 bg-red-700 text-white text-xs font-semibold rounded-full",
                                            "⚠️ Replace"
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            } else {
                                rsx! {}
                            }
                        }
                    }
                    div {
                        class: "text-sm text-gray-700",
                        "{props.stock_item.name}"
                    }
                    {
                        if let Some(description) = &props.stock_item.description {
                            rsx! {
                                div {
                                    class: "text-xs text-gray-500 mt-1 truncate",
                                    "{description}"
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }

                // Ready stock
                div {
                    class: "col-span-2 flex items-center",
                    {
                        if let Some(stock_quantities) = &props.stock_item.stock_quantities {
                            rsx! {
                                span {
                                    class: "text-green-600 font-medium",
                                    "{format_quantity(&stock_quantities.ready_stock_quantity)}{get_unit_suffix(&props.stock_item.unit)}"
                                }
                            }
                        } else {
                            rsx! {
                                span { class: "text-gray-400", "—" }
                            }
                        }
                    }
                }

                // Unmade stock
                div {
                    class: "col-span-2 flex items-center",
                    {
                        if let Some(stock_quantities) = &props.stock_item.stock_quantities {
                            rsx! {
                                span {
                                    class: "text-orange-600 font-medium",
                                    "{format_quantity(&stock_quantities.unready_stock_quantity)}{get_unit_suffix(&props.stock_item.unit)}"
                                }
                            }
                        } else {
                            rsx! {
                                span { class: "text-gray-400", "—" }
                            }
                        }
                    }
                }

                // Total stock
                div {
                    class: "col-span-2 flex items-center gap-2",
                    {
                        if let Some(stock_quantities) = &props.stock_item.stock_quantities {
                            rsx! {
                                span {
                                    class: "font-semibold text-gray-900",
                                    "{format_quantity(&stock_quantities.total_stock_quantity)}{get_unit_suffix(&props.stock_item.unit)}"
                                }
                                {
                                    if stock_quantities.total_child_stock_items > 0 {
                                        rsx! {
                                            span {
                                                class: "text-xs bg-gray-100 text-gray-600 px-1.5 py-0.5 rounded",
                                                "{stock_quantities.total_child_stock_items} ↓"
                                            }
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }
                        } else {
                            rsx! {
                                span { class: "text-gray-400", "—" }
                            }
                        }
                    }
                }

                // Batches
                div {
                    class: "col-span-1 flex items-center justify-center",
                    button {
                        class: "flex items-center gap-1 px-2 py-1 rounded hover:bg-gray-100 transition-colors",
                        onclick: move |_| batches_expanded.set(!batches_expanded()),
                        {
                            if active_batch_count > 0 {
                                rsx! {
                                    span {
                                        class: "bg-blue-500 text-white text-xs font-bold rounded-full w-5 h-5 flex items-center justify-center",
                                        "{active_batch_count}"
                                    }
                                }
                            } else {
                                rsx! {
                                    span { class: "text-gray-400 text-xs", "—" }
                                }
                            }
                        }
                        span {
                            class: {
                                if batches_expanded() {
                                    "text-xs text-gray-400 transform transition-transform rotate-180 ml-1"
                                } else {
                                    "text-xs text-gray-400 transform transition-transform ml-1"
                                }
                            },
                            "▼"
                        }
                    }
                }

                // Actions
                div {
                    class: "col-span-1 flex justify-center",
                    Link {
                        to: Route::AdminEditStockItem { id: format!("{}", props.stock_item.id) },
                        title: "Edit stock item",
                        class: "flex items-center justify-center w-8 h-8 rounded hover:bg-gray-100 transition-colors",
                        img {
                            class: "w-5 h-5",
                            src: asset!("/assets/icons/create-outline.svg")
                        }
                    }
                }
            }

            // Expanded batches section
            {
                if batches_expanded() {
                    rsx! {
                        div {
                            class: "border-t border-gray-200 bg-gray-50",
                            {
                                if recent_batches.is_empty() {
                                    rsx! {
                                        div {
                                            class: "p-4 text-center text-gray-500",
                                            "No stock batches found"
                                        }
                                    }
                                } else {
                                    rsx! {
                                        div {
                                            class: "p-2",
                                            for (i, batch) in recent_batches.iter().enumerate() {
                                                {
                                                    let batch_clone = batch.clone();
                                                    let stock_unit_clone = props.stock_item.unit.clone();
                                                    rsx! {
                                                        StockBatchRow {
                                                            key: "{i}",
                                                            batch: batch_clone,
                                                            stock_unit: stock_unit_clone
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
                } else {
                    rsx! {}
                }
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct StockBatchRowProps {
    batch: StockBatch,
    stock_unit: StockUnit,
}

#[component]
fn StockBatchRow(props: StockBatchRowProps) -> Element {
    let mut batch_expanded = use_signal(|| false);

    let get_status_color = |status: &StockBatchStatus| -> &'static str {
        match status {
            StockBatchStatus::Draft => "bg-gray-100 text-gray-800 border-l-gray-400",
            StockBatchStatus::Paid => "bg-blue-100 text-blue-800 border-l-blue-400",
            StockBatchStatus::Complete => "bg-green-100 text-green-800 border-l-green-400",
            StockBatchStatus::Issue => "bg-red-100 text-red-800 border-l-red-400",
        }
    };

    let format_batch_quantity =
        |original: &StockUnitQuantity, live: &StockUnitQuantity| -> String {
            match (&props.stock_unit, original, live) {
                (
                    StockUnit::Multiples,
                    StockUnitQuantity::Multiples(orig),
                    StockUnitQuantity::Multiples(current),
                ) => {
                    format!("{}/{} left", current, orig)
                }
                (
                    StockUnit::Grams,
                    StockUnitQuantity::Grams(orig),
                    StockUnitQuantity::Grams(current),
                ) => {
                    format!("{:.3}g/{:.3}g left", current, orig)
                }
                (
                    StockUnit::Milliliters,
                    StockUnitQuantity::Milliliters(orig),
                    StockUnitQuantity::Milliliters(current),
                ) => {
                    format!("{:.3}ml/{:.3}ml left", current, orig)
                }
                _ => format!("Unit mismatch"),
            }
        };

    let format_date = |date: &Option<chrono::NaiveDateTime>| -> String {
        match date {
            Some(d) => d.format("%Y-%m-%d").to_string(),
            None => "".to_string(),
        }
    };

    rsx! {
        div {
            class: "mb-2 border-l-4 {get_status_color(&props.batch.status)}",
            // Compact row
            div {
                class: "bg-white p-3 cursor-pointer hover:bg-gray-50 transition-colors",
                onclick: move |_| batch_expanded.set(!batch_expanded()),
                div {
                    class: "flex items-center justify-between",
                    div {
                        class: "flex items-center gap-3",
                        span {
                            class: "font-mono text-sm font-medium",
                            "BATCH{props.batch.stock_batch_code}"
                        }
                        span {
                            class: "text-xs px-2 py-1 rounded {get_status_color(&props.batch.status)}",
                            "{props.batch.status}"
                        }
                        span {
                            class: "text-sm text-gray-600",
                            "{format_batch_quantity(&props.batch.original_quantity, &props.batch.live_quantity)}"
                        }
                        {
                            if let Some(cost) = props.batch.cost_usd {
                                rsx! {
                                    span {
                                        class: "text-sm text-green-600 font-medium",
                                        "${cost:.2}"
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }
                    }
                    span {
                        class: {
                            if batch_expanded() {
                                "text-xs text-gray-400 transform transition-transform rotate-180"
                            } else {
                                "text-xs text-gray-400 transform transition-transform"
                            }
                        },
                        "▼"
                    }
                }
            }

            // Expanded details
            {
                if batch_expanded() {
                    rsx! {
                        div {
                            class: "bg-white border-t border-gray-200 p-4 relative",
                            // Warehouse location (top right)
                            div {
                                class: "absolute top-2 right-2 text-xs bg-gray-100 px-2 py-1 rounded",
                                "{props.batch.warehouse_location}"
                            }

                            div {
                                class: "grid grid-cols-1 md:grid-cols-2 gap-4 pt-4",
                                // Left column
                                div {
                                    class: "space-y-2",
                                    {
                                        if let Some(supplier) = &props.batch.supplier {
                                            rsx! {
                                                div {
                                                    span { class: "font-medium", "Supplier: " }
                                                    span { "{supplier}" }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }
                                    {
                                        if let Some(comment) = &props.batch.comment {
                                            rsx! {
                                                div {
                                                    span { class: "font-medium", "Comment: " }
                                                    span { "{uppercase_first_letter(comment)}" }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }
                                }

                                // Right column
                                div {
                                    class: "space-y-2",
                                    // Arrival date
                                    div {
                                        span { class: "font-medium", "Arrival: " }
                                        {
                                            if let Some(arrival) = &props.batch.arrival_date {
                                                rsx! {
                                                    span { "{format_date(&Some(*arrival))}" }
                                                }
                                            } else if matches!(props.batch.status, StockBatchStatus::Complete) {
                                                rsx! {
                                                    span { class: "text-gray-600 italic", "Arrived" }
                                                }
                                            } else {
                                                rsx! {
                                                    span { class: "text-gray-400", "Pending" }
                                                }
                                            }
                                        }
                                    }

                                    // Tracking URL
                                    div {
                                        span { class: "font-medium", "Tracking: " }
                                        {
                                            if let Some(tracking_url) = &props.batch.tracking_url {
                                                rsx! {
                                                    a {
                                                        href: "{tracking_url}",
                                                        target: "_blank",
                                                        class: "text-blue-600 hover:text-blue-800 underline",
                                                        "View ↗"
                                                    }
                                                }
                                            } else {
                                                rsx! {
                                                    span {
                                                        class: "font-medium text-gray-400",
                                                        "None"
                                                    }
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
            }
        }
    }
}
