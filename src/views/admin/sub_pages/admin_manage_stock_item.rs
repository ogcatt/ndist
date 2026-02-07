#![allow(non_snake_case)]

use crate::Route;
use crate::backend::front_entities::*;
use crate::backend::server_functions::{
    EditStockBatchRequest, EditStockItemRequest, EditStockItemResponse, UploadResponse,
    admin_edit_stock_item, admin_get_pre_or_back_order_reduces, admin_get_stock_batches,
    admin_get_stock_item_relations, admin_get_stock_items, admin_upload_private_thumbnails,
};
use crate::components::*;
use chrono::{NaiveDateTime, Utc};
use dioxus::prelude::*;
use strum::IntoEnumIterator;

// Unique entry for each batch of stock
#[derive(PartialEq, Clone, Debug)]
pub struct EditStockBatch {
    pub id: Option<String>,
    pub stock_batch_code: String,
    pub comment: Option<String>,
    pub supplier: Option<String>,
    pub original_quantity: StockUnitQuantity,
    pub live_quantity: StockUnitQuantity,
    pub stock_unit_on_creation: StockUnit,
    pub cost_usd: Option<f64>,
    pub arrival_date: Option<NaiveDateTime>,
    pub warehouse_location: StockBatchLocation,
    pub tracking_url: Option<String>,
    pub assembled: bool,
    pub status: StockBatchStatus,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[component]
pub fn AdminEditStockItem(id: ReadOnlySignal<String>) -> Element {
    let mut current_stock_item = use_signal(|| None::<StockItem>);
    let mut stock_item_not_found = use_signal(|| false);
    let mut loading = use_signal(|| true);

    // Form state signals
    let mut name = use_signal(|| String::new());
    let mut pbi_sku = use_signal(|| String::new());
    let mut description = use_signal(|| String::new());
    let mut thumbnail_ref = use_signal(|| Option::<String>::None);
    let mut unit = use_signal(|| StockUnit::Multiples);

    // Stock item settings
    let mut assembly_minutes = use_signal(|| 0i32);
    let mut default_shipping_days = use_signal(|| 0i32);
    let mut default_cost = use_signal(|| 0.0);
    let mut is_container = use_signal(|| false);
    let mut warning_quantity = use_signal(|| 0.0);

    // Stock Item Batches
    let mut batches: Signal<Vec<EditStockBatch>> = use_signal(|| vec![]);
    // Stock Item Relations
    let mut stock_item_relations: Signal<Vec<StockItemRelation>> = use_signal(|| vec![]);

    // Pre/Back Order Reduces
    let mut back_order_reduces: Signal<Vec<BackOrPreOrderActiveReduce>> = use_signal(|| vec![]);
    let mut pre_order_reduces: Signal<Vec<BackOrPreOrderActiveReduce>> = use_signal(|| vec![]);
    let mut show_reduces_dropdown = use_signal(|| false);

    let mut flatten_pre_or_back_reduces = use_signal(|| false);

    // UI states
    let mut uploading = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut show_add_stock_relation_modal = use_signal(|| false);

    // Create notification signals
    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new()); // "success" or "error"
    let mut show_notification = use_signal(|| false);

    // States for adding new stock item relations
    let mut selected_stock_item_id = use_signal(|| String::new());
    let mut stock_quantity = use_signal(|| String::new());

    let mut existing_stock_items = use_resource(move || async move {
        tracing::info!("Getting stock items (admin-side)");
        admin_get_stock_items().await
    });

    let mut existing_stock_batches = use_resource(move || async move {
        tracing::info!("Getting stock batches (admin-side)");
        admin_get_stock_batches().await
    });

    let mut existing_stock_item_relations = use_resource(move || async move {
        tracing::info!("Getting stock items (admin-side)");
        admin_get_stock_item_relations().await
    });

    let mut pre_back_order_reduces = use_resource(move || async move {
        tracing::info!("Getting pre/back order reduces (admin-side)");
        admin_get_pre_or_back_order_reduces(id()).await
    });

    let has_relation = move |relations: &[StockItemRelation], child_id: &str| -> bool {
        relations
            .iter()
            .any(|r| r.parent_stock_item_id == id() && r.child_stock_item_id == child_id)
    };

    // Initialize pre/back order reduces
    use_effect(move || {
        if let Some(Ok((back_orders, pre_orders))) = pre_back_order_reduces.read().as_ref() {
            back_order_reduces.set(back_orders.clone());
            pre_order_reduces.set(pre_orders.clone());
        }
    });

    // Initialize stock item relations
    use_effect(move || {
        if let Some(Ok(relations)) = existing_stock_item_relations.read().as_ref() {
            //let current_relations = stock_item_relations.read();

            // Filter relations to only include those with matching variant IDs
            let filtered_relations: Vec<StockItemRelation> = relations
                .iter()
                .filter(|relation| *id() == relation.parent_stock_item_id)
                .cloned()
                .collect();

            stock_item_relations.set(filtered_relations);
        }
    });

    // Initialize batches when data loads
    use_effect(move || {
        if let Some(Ok(stock_batches)) = existing_stock_batches.read().as_ref() {
            let edit_stock_batches = stock_batches
                .iter()
                .filter(|b| b.stock_item_id == id()) // Only include batches matching the current stock item
                .map(|b| EditStockBatch {
                    id: Some(b.id.clone()),
                    stock_batch_code: b.stock_batch_code.clone(),
                    comment: b.comment.clone(),
                    supplier: b.supplier.clone(),
                    original_quantity: b.original_quantity.clone(),
                    live_quantity: b.live_quantity.clone(),
                    stock_unit_on_creation: b.stock_unit_on_creation.clone(),
                    cost_usd: b.cost_usd.clone(),
                    arrival_date: b.arrival_date.clone(),
                    warehouse_location: b.warehouse_location.clone(),
                    tracking_url: b.tracking_url.clone(),
                    assembled: b.assembled,
                    status: b.status.clone(),
                    created_at: Some(b.created_at.clone()),
                    updated_at: Some(b.updated_at.clone()),
                })
                .collect();

            batches.set(edit_stock_batches);
        }
    });

    // Initialize stock items when data loads
    use_effect(move || {
        if let Some(Ok(stock_items)) = existing_stock_items.read().as_ref() {
            let stock_item_id = id();
            if let Some(stock_item) = stock_items.iter().find(|item| item.id == stock_item_id) {
                current_stock_item.set(Some(stock_item.clone()));

                // Set form values
                name.set(stock_item.name.clone());
                pbi_sku.set(stock_item.pbi_sku.clone());
                description.set(stock_item.description.clone().unwrap_or_default());
                thumbnail_ref.set(stock_item.thumbnail_ref.clone());
                unit.set(stock_item.unit.clone());
                assembly_minutes.set(stock_item.assembly_minutes.unwrap_or(0));
                default_shipping_days.set(stock_item.default_shipping_days.unwrap_or(0));
                default_cost.set(stock_item.default_cost.unwrap_or(0.0));
                warning_quantity.set(stock_item.warning_quantity.unwrap_or(0.0));
                is_container.set(stock_item.is_container);

                loading.set(false);
            } else {
                stock_item_not_found.set(true);
                loading.set(false);
            }
        }
    });

    let mut add_stock_item_relation = move |stock_item_id: String, quantity: f64| {
        let new_relation = StockItemRelation {
            parent_stock_item_id: id().clone(),
            child_stock_item_id: stock_item_id.clone(),
            quantity,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        stock_item_relations.with_mut(|relations| {
            // Check if relation already exists
            if !relations.iter().any(|r| {
                r.parent_stock_item_id == new_relation.parent_stock_item_id
                    && r.child_stock_item_id == new_relation.child_stock_item_id
            }) {
                relations.push(new_relation);
            }
        });
    };

    let mut remove_stock_item_relation = move |parent_id: &str, child_id: &str| {
        stock_item_relations.with_mut(|relations| {
            relations.retain(|r| {
                r.parent_stock_item_id != parent_id || r.child_stock_item_id != child_id
            });
        });
    };

    let mut create_batch = move || {
        let mut current_batches = batches();
        current_batches.push(EditStockBatch {
            id: None,
            stock_batch_code: String::from(""),
            comment: None,
            supplier: None,
            original_quantity: unit().into(),
            live_quantity: unit().into(),
            stock_unit_on_creation: unit(),
            cost_usd: None,
            arrival_date: None,
            warehouse_location: StockBatchLocation::EU,
            tracking_url: None,
            assembled: false,
            status: StockBatchStatus::Draft,
            created_at: Some(Utc::now().naive_utc()),
            updated_at: Some(Utc::now().naive_utc()),
        });
        batches.set(current_batches);
    };

    let mut remove_batch = move |index: usize| {
        batches.with_mut(|v| {
            if index < v.len() {
                v.remove(index);
            }
        });
    };

    let handle_save_stock_item = move |_| {
        spawn(async move {
            saving.set(true);

            // Validate required fields
            if name().trim().is_empty() {
                notification_message.set("Name is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                saving.set(false);
                return;
            }

            if pbi_sku().trim().is_empty() {
                notification_message.set("PBI SKU is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                saving.set(false);
                return;
            }

            // Prepare request data
            let request = EditStockItemRequest {
                id: id(),
                name: name(),
                pbi_sku: pbi_sku(),
                description: if description().trim().is_empty() {
                    None
                } else {
                    Some(description())
                },
                thumbnail_ref: thumbnail_ref(),
                unit: unit(),
                assembly_minutes: if assembly_minutes() == 0 {
                    None
                } else {
                    Some(assembly_minutes())
                },
                default_shipping_days: if default_shipping_days() == 0 {
                    None
                } else {
                    Some(default_shipping_days())
                },
                default_cost: if default_cost() == 0.0 {
                    None
                } else {
                    Some(default_cost())
                },
                warning_quantity: if warning_quantity() == 0.0 {
                    None
                } else {
                    Some(warning_quantity())
                },
                is_container: is_container(),
                stock_item_relations: Some(stock_item_relations()),
                flatten_pre_or_back_reduces: flatten_pre_or_back_reduces(),
                batches: Some(
                    batches()
                        .into_iter()
                        .map(|b| EditStockBatchRequest {
                            id: b.id,
                            stock_batch_code: b.stock_batch_code,
                            comment: b.comment,
                            supplier: b.supplier,
                            original_quantity: b.original_quantity,
                            live_quantity: b.live_quantity,
                            stock_unit_on_creation: b.stock_unit_on_creation,
                            cost_usd: b.cost_usd,
                            arrival_date: b.arrival_date,
                            warehouse_location: b.warehouse_location,
                            tracking_url: b.tracking_url,
                            assembled: b.assembled,
                            status: b.status,
                            created_at: b.created_at,
                            updated_at: b.updated_at,
                        })
                        .collect(),
                ),
            };

            // Call server function
            match admin_edit_stock_item(request).await {
                Ok(response) => {
                    if response.success {
                        notification_message.set("Stock item updated successfully!".to_string());
                        notification_type.set("success".to_string());
                        show_notification.set(true);

                        // Refresh the stock items data
                        existing_stock_items.restart();
                    } else {
                        notification_message.set(response.message);
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error updating stock item: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }

            saving.set(false);
        });
    };

    let get_available_stock_items = move || -> Vec<StockItem> {
        let current_relations = stock_item_relations();
        let used_stock_item_ids: std::collections::HashSet<String> = current_relations
            .iter()
            .map(|r| r.child_stock_item_id.clone())
            .collect();

        if let Some(Ok(all_stock_items)) = existing_stock_items.read().as_ref() {
            all_stock_items
                .iter()
                .filter(|item| !used_stock_item_ids.contains(&item.id) && item.id != id()) // Exclude current item and already used items
                .cloned()
                .collect()
        } else {
            vec![]
        }
    };

    let mut handle_add_stock_relation = move || {
        if selected_stock_item_id().is_empty() || stock_quantity().is_empty() {
            return;
        }

        if let Ok(quantity) = stock_quantity().parse::<f64>() {
            if quantity > 0.0 {
                add_stock_item_relation(selected_stock_item_id(), quantity);

                // Reset form
                selected_stock_item_id.set(String::new());
                stock_quantity.set(String::new());
                show_add_stock_relation_modal.set(false);
            }
        }
    };

    let handle_thumbnail_upload = move |evt: FormEvent| {
        let mut thumbnail_ref = thumbnail_ref.clone();
        let mut uploading = uploading.clone();

        spawn(async move {
            if let Some(file_engine) = evt.files() {
                let files = file_engine.files();
                if let Some(file_name) = files.get(0) {
                    uploading.set(true);

                    if let Some(file_data) = file_engine.read_file(file_name).await {
                        // Determine content type based on file extension
                        let content_type =
                            if file_name.ends_with(".jpg") || file_name.ends_with(".jpeg") {
                                "image/jpeg"
                            } else if file_name.ends_with(".png") {
                                "image/png"
                            } else if file_name.ends_with(".webp") {
                                "image/webp"
                            } else if file_name.ends_with(".avif") {
                                "image/avif"
                            } else {
                                "image/jpeg" // default
                            };

                        match admin_upload_private_thumbnails(
                            file_data,
                            file_name.clone(),
                            content_type.to_string(),
                        )
                        .await
                        {
                            Ok(response) => {
                                if response.success {
                                    thumbnail_ref.set(response.url);
                                } else {
                                    println!("Upload failed: {}", response.message);
                                }
                            }
                            Err(e) => {
                                println!("Upload error: {}", e);
                            }
                        }
                    }

                    uploading.set(false);
                }
            }
        });
    };

    // Helper functions for reduces counts
    let get_reduce_counts = move || -> (usize, usize, usize, usize) {
        let back_orders = back_order_reduces();
        let pre_orders = pre_order_reduces();

        let active_back_orders = back_orders.iter().filter(|r| r.active).count();
        let inactive_back_orders = back_orders.iter().filter(|r| !r.active).count();
        let active_pre_orders = pre_orders.iter().filter(|r| r.active).count();
        let inactive_pre_orders = pre_orders.iter().filter(|r| !r.active).count();

        (
            active_back_orders,
            inactive_back_orders,
            active_pre_orders,
            inactive_pre_orders,
        )
    };

    let has_any_reduces = move || -> bool {
        let (active_back, inactive_back, active_pre, inactive_pre) = get_reduce_counts();
        active_back > 0 || inactive_back > 0 || active_pre > 0 || inactive_pre > 0
    };

    if loading() {
        return rsx! {
            div {
                "Loading stock item..."
            }
        };
    }

    if stock_item_not_found() {
        return rsx! {
            div {
                class: "text-center py-8",
                h2 {
                    class: "text-xl font-medium text-gray-900 mb-2",
                    "Stock Item Not Found"
                }
                p {
                    class: "text-gray-600 mb-4",
                    "The stock item you're looking for could not be found."
                }
                Link {
                    to: Route::AdminInventory {},
                    class: "text-blue-600 hover:text-blue-500",
                    "← Back to Stock Items"
                }
            }
        };
    }

    rsx! {
        // Notification
        if show_notification() {
            div {
                class: format!("fixed top-4 right-4 z-50 p-4 rounded-md shadow-lg text-white {}",
                    if notification_type() == "success" { "bg-green-500" } else { "bg-red-500" }
                ),
                div {
                    class: "flex justify-between items-center",
                    span { "{notification_message()}" }
                    button {
                        class: "ml-4 text-white hover:text-gray-200",
                        onclick: move |_| show_notification.set(false),
                        "×"
                    }
                }
            }
        }

        div {
            class: "border rounded-md border-gray-200 w-full mb-4 flex justify-between items-center p-2 pl-4",
            div {
                class: "flex items-center gap-3",
                div {
                    class: "text-lg font-medium",
                    "Edit Stock Item"
                }
            }
            div {
                class: "flex",
                if has_any_reduces() {
                    div {
                        class: "flex items-center gap-2 mr-4",
                        label {
                            class: "text-sm font-medium text-gray-700 cursor-pointer",
                            r#for: "flatten-checkbox",
                            "Flatten"
                        }
                        input {
                            id: "flatten-checkbox",
                            r#type: "checkbox",
                            class: "h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded",
                            checked: flatten_pre_or_back_reduces(),
                            onchange: move |event| {
                                flatten_pre_or_back_reduces.set(event.value().parse().unwrap_or(false));
                            }
                        }
                    }
                }
                button {
                    class: format!("text-sm px-3 py-2 text-white rounded transition-colors {}",
                        if saving() { "bg-gray-500 cursor-not-allowed" } else { "bg-blue-600 hover:bg-blue-700" }
                    ),
                    disabled: saving(),
                    onclick: handle_save_stock_item,
                    if saving() {
                        "Saving..."
                    } else {
                        "Save Changes"
                    }
                }
            }
        }

        // Pre/Back Order Reduces Section
        {
            if has_any_reduces() {
                let (active_back, inactive_back, active_pre, inactive_pre) = get_reduce_counts();
                rsx! {
                    div {
                        class: "border rounded-md border-gray-200 w-full mb-4",

                        // Header with counts
                        div {
                            class: "flex justify-between items-center p-4 cursor-pointer hover:bg-gray-50",
                            onclick: move |_| show_reduces_dropdown.toggle(),

                            div {
                                class: "flex items-center gap-4",
                                h3 {
                                    class: "text-lg font-medium",
                                    "Order Reduces"
                                }
                                div {
                                    class: "flex gap-3 text-sm",
                                    if active_back > 0 || inactive_back > 0 {
                                        div {
                                            class: "flex items-center gap-1",
                                            span {
                                                class: "font-medium text-red-600",
                                                "Backorder: {active_back}"
                                            }
                                            if inactive_back > 0 {
                                                span {
                                                    class: "text-gray-500",
                                                    "({inactive_back})"
                                                }
                                            }
                                        }
                                    }
                                    if active_pre > 0 || inactive_pre > 0 {
                                        div {
                                            class: "flex items-center gap-1",
                                            span {
                                                class: "font-medium text-blue-600",
                                                "Preorder: {active_pre}"
                                            }
                                            if inactive_pre > 0 {
                                                span {
                                                    class: "text-gray-500",
                                                    "({inactive_pre})"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div {
                                class: format!("transform transition-transform {}",
                                    if show_reduces_dropdown() { "rotate-180" } else { "rotate-0" }
                                ),
                                svg {
                                    class: "w-5 h-5 text-gray-400",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M19 9l-7 7-7-7"
                                    }
                                }
                            }
                        }

                        // Dropdown content
                        if show_reduces_dropdown() {
                            div {
                                class: "border-t border-gray-200 p-4",

                                // Active reduces section
                                if active_back > 0 || active_pre > 0 {
                                    div {
                                        class: "mb-6",
                                        h4 {
                                            class: "text-sm font-medium text-gray-900 mb-3",
                                            "Active Reduces"
                                        }
                                        div {
                                            class: "space-y-2",

                                            // Active backorder reduces
                                            {
                                                let active_back_reduces: Vec<_> = back_order_reduces()
                                                    .into_iter()
                                                    .filter(|r| r.active)
                                                    .collect();

                                                rsx! {
                                                    for reduce in active_back_reduces {
                                                        div {
                                                            class: "flex justify-between items-center bg-red-50 border border-red-200 rounded-md p-3",
                                                            div {
                                                                div {
                                                                    class: "text-sm font-medium text-red-800",
                                                                    "Backorder Reduce"
                                                                }
                                                                div {
                                                                    class: "text-xs text-red-600",
                                                                    "Order: {reduce.order_id} • Qty: {reduce.reduction_quantity} {reduce.stock_unit}"
                                                                }
                                                            }
                                                            div {
                                                                class: "text-xs text-red-500",
                                                                { reduce.created_at.format("%Y-%m-%d %H:%M").to_string() }
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            // Active preorder reduces
                                            {
                                                let active_pre_reduces: Vec<_> = pre_order_reduces()
                                                    .into_iter()
                                                    .filter(|r| r.active)
                                                    .collect();

                                                rsx! {
                                                    for reduce in active_pre_reduces {
                                                        div {
                                                            class: "flex justify-between items-center bg-blue-50 border border-blue-200 rounded-md p-3",
                                                            div {
                                                                div {
                                                                    class: "text-sm font-medium text-blue-800",
                                                                    "Preorder Reduce"
                                                                }
                                                                div {
                                                                    class: "text-xs text-blue-600",
                                                                    "Order: {reduce.order_id} • Qty: {reduce.reduction_quantity} {reduce.stock_unit}"
                                                                }
                                                            }
                                                            div {
                                                                class: "text-xs text-blue-500",
                                                                { reduce.created_at.format("%Y-%m-%d %H:%M").to_string() }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Unconfirmed reduces section
                                if inactive_back > 0 || inactive_pre > 0 {
                                    div {
                                        h4 {
                                            class: "text-sm font-medium text-gray-900 mb-3",
                                            "Unconfirmed Reduces"
                                        }
                                        div {
                                            class: "space-y-2",

                                            // Inactive backorder reduces
                                            {
                                                let inactive_back_reduces: Vec<_> = back_order_reduces()
                                                    .into_iter()
                                                    .filter(|r| !r.active)
                                                    .collect();

                                                rsx! {
                                                    for reduce in inactive_back_reduces {
                                                        div {
                                                            class: "flex justify-between items-center bg-gray-50 border border-gray-200 rounded-md p-3",
                                                            div {
                                                                div {
                                                                    class: "text-sm font-medium text-gray-700",
                                                                    "Backorder Reduce (Unconfirmed)"
                                                                }
                                                                div {
                                                                    class: "text-xs text-gray-500",
                                                                    "Order: {reduce.order_id} • Qty: {reduce.reduction_quantity} {reduce.stock_unit}"
                                                                }
                                                            }
                                                            div {
                                                                class: "text-xs text-gray-400",
                                                                { reduce.created_at.format("%Y-%m-%d %H:%M").to_string() }
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            // Inactive preorder reduces
                                            {
                                                let inactive_pre_reduces: Vec<_> = pre_order_reduces()
                                                    .into_iter()
                                                    .filter(|r| !r.active)
                                                    .collect();

                                                rsx! {
                                                    for reduce in inactive_pre_reduces {
                                                        div {
                                                            class: "flex justify-between items-center bg-gray-50 border border-gray-200 rounded-md p-3",
                                                            div {
                                                                div {
                                                                    class: "text-sm font-medium text-gray-700",
                                                                    "Preorder Reduce (Unconfirmed)"
                                                                }
                                                                div {
                                                                    class: "text-xs text-gray-500",
                                                                    "Order: {reduce.order_id} • Qty: {reduce.reduction_quantity} {reduce.stock_unit}"
                                                                }
                                                            }
                                                            div {
                                                                class: "text-xs text-gray-400",
                                                                { reduce.created_at.format("%Y-%m-%d %H:%M").to_string() }
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
            } else {
                rsx! { }
            }
        }

        div {
            class: "flex flex-col md:flex-row w-full gap-2",
            div {
                class: "flex w-full flex-col gap-2",

                // Basic Info Section
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                    div {
                        class: "flex gap-4 w-full mb-4",
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "PBI/PBX SKU",
                                value: "{pbi_sku}",
                                placeholder: "PBI0001 or PBX0001",
                                optional: false,
                                oninput: move |event: FormEvent| {
                                    let mut value = event.value();
                                    // Ensure it starts with PBI if user doesn't type it
                                    if (!value.starts_with("P") && !value.starts_with("PB")) && !value.is_empty() {
                                        value = format!("PBI{}", value);
                                    }
                                    pbi_sku.set(value);
                                }
                            }
                        },
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Name",
                                value: "{name}",
                                placeholder: "Stock Item Name",
                                optional: false,
                                oninput: move |event: FormEvent| name.set(event.value())
                            }
                        }
                    }
                    CTextArea {
                        label: "Description",
                        placeholder: "Optional description for this stock item...",
                        value: "{description}",
                        oninput: move |event: FormEvent| description.set(event.value())
                    }
                },

                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 min-h-36",
                    div {
                        class: "flex justify-between border-b border-gray-300",
                        h2 {
                            class: "text-lg pl-4 pt-3.5 pb-2",
                            "Stock Item Relations"
                        },
                        div {
                            button {
                                class: "text-sm bg-gray-900 px-3 py-2 text-white rounded hover:bg-gray-800 transition-colors m-2",
                                onclick: move |_| show_add_stock_relation_modal.set(true),
                                "Add Relation"
                            }
                        }
                    },
                    {
                        let current_relations = stock_item_relations();
                        let filtered_relations: Vec<_> = current_relations.iter()
                            .filter(|r| r.parent_stock_item_id == id())
                            .cloned()
                            .collect();
                        if filtered_relations.is_empty() {
                            rsx! {
                                div {
                                    class: "text-gray-500 w-full py-8 text-center text-sm",
                                    "No relations"
                                }
                            }
                        } else {
                            rsx! {
                                div {
                                    {filtered_relations.into_iter().map(|relation| {
                                        let parent_id = relation.parent_stock_item_id.clone();
                                        let child_id = relation.child_stock_item_id.clone();

                                        // Clone the values specifically for the closure
                                        let parent_id_for_closure = parent_id.clone();
                                        let child_id_for_closure = child_id.clone();

                                        rsx! {
                                            div {
                                                key: "{parent_id}_{child_id}",
                                                class: "flex items-center justify-between bg-white rounded border p-2 m-2",
                                                div {
                                                    class: "flex-1",
                                                    {
                                                        if let Some(Ok(all_stock_items)) = existing_stock_items.read().as_ref() {
                                                            if let Some(stock_item) = all_stock_items.iter().find(|item| item.id == child_id) {
                                                                rsx! {
                                                                    div {
                                                                        class: "text-xs font-medium text-gray-900",
                                                                        "{stock_item.name}"
                                                                    }
                                                                    div {
                                                                        class: "text-xs text-gray-500",
                                                                        "SKU: {stock_item.pbi_sku} • Qty: {relation.quantity} {stock_item.unit}"
                                                                    }
                                                                }
                                                            } else {
                                                                rsx! {
                                                                    div { "Unknown item" }
                                                                }
                                                            }
                                                        } else {
                                                            rsx! {
                                                                div { "Loading..." }
                                                            }
                                                        }
                                                    }
                                                },
                                                button {
                                                    class: "text-red-600 hover:text-red-800 text-xs",
                                                    onclick: move |_| {
                                                        remove_stock_item_relation(&parent_id_for_closure, &child_id_for_closure)
                                                    },
                                                    "Remove"
                                                }
                                            }
                                        }
                                    })}
                                }
                            }
                        }
                    }
                }

                // Batches Section
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 min-h-36",
                    div {
                        class: "flex justify-between border-b border-gray-300",
                        h2 {
                            class: "text-lg pl-4 pt-3.5 pb-2",
                            "Batches"
                        },
                        div {
                            button {
                                class: "text-sm bg-gray-900 px-3 py-2 text-white rounded hover:bg-gray-800 transition-colors m-2",
                                onclick: move |_| create_batch(),
                                "Add Batch"
                            }
                        }
                    },

                    if batches.read().len() == 0 {
                        div {
                            class: "text-gray-500 w-full py-8 text-center text-sm",
                            "No batches created yet"
                        }
                    } else {
                        div {
                            class: "",
                            for (index, batch) in batches.read().iter().enumerate() {
                                div {
                                    class: "p-4 border-t border-gray-300 first:border-t-0",

                                    div {
                                        class: "flex justify-between items-center mb-4",
                                        h3 {
                                            class: "text-xs text-uppercase text-gray-700 font-bold",
                                            "BATCH {index + 1}"
                                        },
                                        button {
                                            class: "text-red-600 hover:text-red-800 text-sm",
                                            onclick: move |_| remove_batch(index),
                                            "Remove"
                                        }
                                    },

                                    div {
                                        class: "flex flex-col gap-4",

                                        // First row - Basic batch info including status
                                        div {
                                            class: "grid grid-cols-1 md:grid-cols-4 gap-4",

                                            div {
                                                CTextBox {
                                                    label: "Batch Code",
                                                    value: batch.stock_batch_code.clone(),
                                                    prefix: "BATCH",
                                                    placeholder: "0001",
                                                    optional: false,
                                                    oninput: move |event: FormEvent| {
                                                        batches.with_mut(|v| {
                                                            if let Some(batch) = v.get_mut(index) {
                                                                batch.stock_batch_code = event.value();
                                                            }
                                                        });
                                                    }
                                                }
                                            },

                                            div {
                                                CTextBox {
                                                    label: "Supplier",
                                                    value: batch.supplier.clone().unwrap_or_default(),
                                                    placeholder: "Supplier name",
                                                    optional: true,
                                                    oninput: move |event: FormEvent| {
                                                        batches.with_mut(|v| {
                                                            if let Some(batch) = v.get_mut(index) {
                                                                batch.supplier = if event.value().is_empty() { None } else { Some(event.value()) };
                                                            }
                                                        });
                                                    }
                                                }
                                            },

                                            div {
                                                CSelectGroup {
                                                    label: "Status",
                                                    optional: false,
                                                    oninput: move |event: FormEvent| {
                                                        if let Ok(status) = event.value().parse::<StockBatchStatus>() {
                                                            batches.with_mut(|v| {
                                                                if let Some(batch) = v.get_mut(index) {
                                                                    batch.status = status;
                                                                }
                                                            });
                                                        }
                                                    },
                                                    for status in StockBatchStatus::iter() {
                                                        CSelectItem {
                                                            selected: status == batch.status,
                                                            key: "{status:?}",
                                                            value: "{status}",
                                                            "{status.to_string()}"
                                                        }
                                                    }
                                                }
                                            },

                                            div {
                                                CSelectGroup {
                                                    label: "Location",
                                                    optional: false,
                                                    oninput: move |event: FormEvent| {
                                                        if let Ok(location) = event.value().parse::<StockBatchLocation>() {
                                                            batches.with_mut(|v| {
                                                                if let Some(batch) = v.get_mut(index) {
                                                                    batch.warehouse_location = location;
                                                                }
                                                            });
                                                        }
                                                    },
                                                    for location in StockBatchLocation::iter() {
                                                        CSelectItem {
                                                            selected: location == batch.warehouse_location,
                                                            key: "{location:?}",
                                                            value: "{location}",
                                                            "{location.to_string()}"
                                                        }
                                                    }
                                                }
                                            }
                                        },

                                        // Second row - Quantities and cost
                                        div {
                                            class: "grid grid-cols-1 md:grid-cols-3 gap-4",

                                            div {
                                                CTextBox {
                                                    label: "Original Quantity",
                                                    value: match &batch.original_quantity {
                                                        StockUnitQuantity::Multiples(qty) => format!("{}", qty),
                                                        StockUnitQuantity::Grams(qty) => format!("{}", qty),
                                                        StockUnitQuantity::Milliliters(qty) => format!("{}", qty),
                                                    },
                                                    suffix: match &batch.original_quantity {
                                                        StockUnitQuantity::Multiples(_) => "qty",
                                                        StockUnitQuantity::Grams(_) => "g",
                                                        StockUnitQuantity::Milliliters(_) => "ml",
                                                    },
                                                    placeholder: "0",
                                                    optional: false,
                                                    is_number: true,
                                                    step: match &batch.original_quantity {
                                                        StockUnitQuantity::Multiples(_) => 1f64,
                                                        StockUnitQuantity::Grams(_) => 0.001f64,
                                                        StockUnitQuantity::Milliliters(_) => 0.001f64,
                                                    },
                                                    oninput: move |event: FormEvent| {
                                                        if event.value().parse::<f64>().unwrap_or(0.0) >= 0.0 {
                                                        batches.with_mut(|v| {
                                                            if let Some(batch) = v.get_mut(index) {
                                                                match &batch.original_quantity {
                                                                    StockUnitQuantity::Multiples(_) => {
                                                                        if let Ok(qty) = event.value().parse::<i32>() {
                                                                            batch.original_quantity = StockUnitQuantity::Multiples(qty);
                                                                        }
                                                                    },
                                                                    StockUnitQuantity::Grams(_) => {
                                                                        if let Ok(qty) = event.value().parse::<f64>() {
                                                                            batch.original_quantity = StockUnitQuantity::Grams(qty);
                                                                        }
                                                                    },
                                                                    StockUnitQuantity::Milliliters(_) => {
                                                                        if let Ok(qty) = event.value().parse::<f64>() {
                                                                            batch.original_quantity = StockUnitQuantity::Milliliters(qty);
                                                                        }
                                                                    },
                                                                }
                                                            }
                                                        });
                                                    };
                                                    }
                                                }
                                            },

                                            div {
                                                CTextBox {
                                                    label: "Live Quantity",
                                                    value: match &batch.live_quantity {
                                                        StockUnitQuantity::Multiples(qty) => format!("{}", qty),
                                                        StockUnitQuantity::Grams(qty) => format!("{}", qty),
                                                        StockUnitQuantity::Milliliters(qty) => format!("{}", qty),
                                                    },
                                                    suffix: match &batch.live_quantity {
                                                        StockUnitQuantity::Multiples(_) => "qty",
                                                        StockUnitQuantity::Grams(_) => "g",
                                                        StockUnitQuantity::Milliliters(_) => "ml",
                                                    },
                                                    placeholder: "0",
                                                    optional: false,
                                                    is_number: true,
                                                    step: match &batch.live_quantity {
                                                        StockUnitQuantity::Multiples(_) => 1f64,
                                                        StockUnitQuantity::Grams(_) => 0.001f64,
                                                        StockUnitQuantity::Milliliters(_) => 0.001f64,
                                                    },
                                                    oninput: move |event: FormEvent| {
                                                        batches.with_mut(|v| {
                                                            if let Some(batch) = v.get_mut(index) {
                                                                match &batch.live_quantity {
                                                                    StockUnitQuantity::Multiples(_) => {
                                                                        if let Ok(qty) = event.value().parse::<i32>() {
                                                                            batch.live_quantity = StockUnitQuantity::Multiples(qty);
                                                                        }
                                                                    },
                                                                    StockUnitQuantity::Grams(_) => {
                                                                        if let Ok(qty) = event.value().parse::<f64>() {
                                                                            batch.live_quantity = StockUnitQuantity::Grams(qty);
                                                                        }
                                                                    },
                                                                    StockUnitQuantity::Milliliters(_) => {
                                                                        if let Ok(qty) = event.value().parse::<f64>() {
                                                                            batch.live_quantity = StockUnitQuantity::Milliliters(qty);
                                                                        }
                                                                    },
                                                                }
                                                            }
                                                        });
                                                    }
                                                }
                                            },

                                            div {
                                                CTextBox {
                                                    label: "Cost (USD)",
                                                    value: batch.cost_usd.map_or(String::new(), |cost| format!("{:.2}", cost)),
                                                    placeholder: "0.00",
                                                    optional: true,
                                                    prefix: "$",
                                                    is_number: true,
                                                    step: 0.01,
                                                    oninput: move |event: FormEvent| {
                                                        batches.with_mut(|v| {
                                                            if let Some(batch) = v.get_mut(index) {
                                                                batch.cost_usd = if event.value().is_empty() {
                                                                    None
                                                                } else {
                                                                    event.value().parse::<f64>().ok()
                                                                };
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                        },

                                        // Third row - Comment and tracking
                                        div {
                                            class: "grid grid-cols-1 md:grid-cols-2 gap-4",

                                            div {
                                                CTextBox {
                                                    label: "Tracking URL",
                                                    value: batch.tracking_url.clone().unwrap_or_default(),
                                                    placeholder: "https://tracking.example.com/123456",
                                                    optional: true,
                                                    oninput: move |event: FormEvent| {
                                                        batches.with_mut(|v| {
                                                            if let Some(batch) = v.get_mut(index) {
                                                                batch.tracking_url = if event.value().is_empty() { None } else { Some(event.value()) };
                                                            }
                                                        });
                                                    }
                                                }
                                            },

                                            div {
                                                class: "flex items-end",
                                                div {
                                                    class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                                                    p {
                                                        class: "text-sm text-gray-700 pt-[2px]",
                                                        "Assembled"
                                                    }
                                                    CToggle {
                                                        checked: batch.assembled,
                                                        onclick: move |_| {
                                                            batches.with_mut(|v| {
                                                                if let Some(batch) = v.get_mut(index) {
                                                                    batch.assembled = !batch.assembled;
                                                                }
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        },

                                        // Comment field
                                        div {
                                            CTextArea {
                                                label: "Comment",
                                                placeholder: "Optional notes about this batch...",
                                                value: batch.comment.clone().unwrap_or_default(),
                                                oninput: move |event: FormEvent| {
                                                    batches.with_mut(|v| {
                                                        if let Some(batch) = v.get_mut(index) {
                                                            batch.comment = if event.value().is_empty() { None } else { Some(event.value()) };
                                                        }
                                                    });
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

            if show_add_stock_relation_modal() {
                div {
                    class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
                    onclick: move |_| show_add_stock_relation_modal.set(false),

                    div {
                        class: "bg-white rounded-lg p-6 max-w-md w-full mx-4",
                        onclick: move |e| e.stop_propagation(),

                        h3 {
                            class: "text-lg font-medium text-gray-900 mb-4",
                            "Add Child Stock Item"
                        },

                        div {
                            class: "space-y-4",

                            div {
                                label {
                                    class: "block text-sm font-medium text-gray-700 mb-1",
                                    "Stock Item"
                                },
                                select {
                                    class: "w-full border border-gray-300 rounded-md px-3 py-2 text-sm",
                                    value: selected_stock_item_id(),
                                    onchange: move |e| selected_stock_item_id.set(e.value()),

                                    option {
                                        value: "",
                                        "Select a stock item"
                                    },

                                    for stock_item in get_available_stock_items().iter() {
                                        option {
                                            value: stock_item.id.clone(),
                                            "{stock_item.name} ({stock_item.pbi_sku})"
                                        }
                                    }
                                }
                            },

                            if !selected_stock_item_id().is_empty() {
                                if let Some(Ok(all_stock_items)) = existing_stock_items.read().as_ref() {
                                    if let Some(selected_item) = all_stock_items.iter().find(|item| item.id == selected_stock_item_id()) {
                                        div {
                                            label {
                                                class: "block text-sm font-medium text-gray-700 mb-1",
                                                "Quantity ({selected_item.unit})"
                                            },
                                            input {
                                                r#type: "number",
                                                step: if selected_item.unit == StockUnit::Multiples { "1" } else { "0.01" },
                                                min: if selected_item.unit == StockUnit::Multiples { "1" } else { "0.01" },
                                                class: "w-full border border-gray-300 rounded-md px-3 py-2 text-sm",
                                                value: stock_quantity(),
                                                placeholder: if selected_item.unit == StockUnit::Multiples { "1" } else { "1.00" },
                                                oninput: move |e| stock_quantity.set(e.value())
                                            }
                                        }
                                    }
                                }
                            }
                        },

                        div {
                            class: "flex justify-end gap-2 mt-6",
                            button {
                                class: "px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors",
                                onclick: move |_| {
                                    show_add_stock_relation_modal.set(false);
                                    selected_stock_item_id.set(String::new());
                                    stock_quantity.set(String::new());
                                },
                                "Cancel"
                            },
                            button {
                                class: "px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md transition-colors",
                                disabled: selected_stock_item_id().is_empty() || stock_quantity().is_empty(),
                                onclick: move |_| handle_add_stock_relation(),
                                "Add"
                            }
                        }
                    }
                }
            }

            // Right sidebar
            div {
                class: "md:w-[38%] w-full min-w-0",

                // Unit and Container Settings
                div {
                    class: "bg-white border flex flex-col gap-4 rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 {
                        class: "text-lg font-medium",
                        "Settings"
                    }

                    CSelectGroup {
                        label: "Unit Type",
                        disabled: true,
                        //optional: false,
                        /*
                        oninput: move |event: FormEvent| {
                            if let Ok(stock_unit) = event.value().parse::<StockUnit>() {
                                unit.set(stock_unit);
                            }
                        },
                        */
                        for unit_type in StockUnit::iter() {
                            CSelectItem {
                                selected: unit_type == unit(),
                                key: "{unit_type:?}",
                                value: "{unit_type}",
                                "{unit_type.to_string()}"
                            }
                        }
                    }

                    div {
                        class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                        p {
                            class: "text-sm text-gray-700 pt-[2px]",
                            "Contains other items"
                        }
                        CToggle {
                            checked: is_container(),
                            onclick: move |_| is_container.toggle()
                        }
                    }
                }

                // Cost and Time Settings
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Cost & Time"
                    }
                    div {
                        class: "flex flex-col gap-4",

                        CTextBox {
                            label: "Default Cost (per unit)",
                            value: if default_cost() == 0.0 { String::new() } else { format!("{}", default_cost()) },
                            placeholder: "0",
                            prefix: "$",
                            is_number: true,
                            step: 1f64,
                            optional: true,
                            oninput: move |event: FormEvent| {
                                if event.value().is_empty() {
                                    default_cost.set(0.0);
                                } else if let Ok(cost) = event.value().parse::<f64>() {
                                    default_cost.set(cost);
                                }
                            }
                        }

                        CTextBox {
                            label: "Assembly Time",
                            value: if assembly_minutes() == 0 { String::new() } else { format!("{}", assembly_minutes()) },
                            placeholder: "0",
                            suffix: "min",
                            is_number: true,
                            step: 1f64,
                            optional: true,
                            oninput: move |event: FormEvent| {
                                if event.value().is_empty() {
                                    assembly_minutes.set(0);
                                } else if let Ok(minutes) = event.value().parse::<i32>() {
                                    assembly_minutes.set(minutes);
                                }
                            }
                        }

                        CTextBox {
                            label: "Default Shipping Days",
                            value: if default_shipping_days() == 0 { String::new() } else { format!("{}", default_shipping_days()) },
                            placeholder: "0",
                            suffix: "days",
                            is_number: true,
                            step: 1f64,
                            optional: true,
                            oninput: move |event: FormEvent| {
                                if event.value().is_empty() {
                                    default_shipping_days.set(0);
                                } else if let Ok(days) = event.value().parse::<i32>() {
                                    default_shipping_days.set(days);
                                }
                            }
                        }
                    }
                },

                // Thumbnail Section
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 min-h-36",
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Thumbnail"
                    }
                    div {
                        class: "w-full max-w-xs mx-auto",
                        div {
                            class: "aspect-square w-full border-2 border-dashed border-gray-300 rounded-lg hover:border-gray-400 transition-colors cursor-pointer bg-gray-50 hover:bg-gray-100 flex flex-col items-center justify-center relative overflow-hidden",

                            if let Some(url) = thumbnail_ref() {
                                // Display uploaded image
                                img {
                                    src: "{url}",
                                    class: "w-full h-full object-cover rounded-lg",
                                    alt: "Stock item thumbnail"
                                }
                                // Remove button overlay
                                button {
                                    class: "absolute top-2 right-2 bg-red-500 text-white rounded-full w-6 h-6 flex items-center justify-center text-sm hover:bg-red-600",
                                    onclick: move |_| thumbnail_ref.set(None),
                                    "×"
                                }
                            } else {
                                // Upload prompt
                                div {
                                    class: "text-center p-4",
                                    if uploading() {
                                        div {
                                            class: "animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900 mx-auto mb-2"
                                        }
                                        span {
                                            class: "text-sm text-gray-600",
                                            "Uploading..."
                                        }
                                    } else {
                                        svg {
                                            class: "mx-auto h-8 w-8 text-gray-400 mb-2",
                                            stroke: "currentColor",
                                            fill: "none",
                                            view_box: "0 0 48 48",
                                            path {
                                                d: "M28 8H12a4 4 0 00-4 4v20m32-12v8m0 0v8a4 4 0 01-4 4H12a4 4 0 01-4-4v-4m32-4l-3.172-3.172a4 4 0 00-5.656 0L28 28M8 32l9.172-9.172a4 4 0 015.656 0L28 28m0 0l4 4m4-24h8m-4-4v8m-12 4h.02",
                                                stroke_width: "2",
                                                stroke_linecap: "round",
                                                stroke_linejoin: "round",
                                            }
                                        }
                                        span {
                                            class: "text-sm text-gray-600 font-medium",
                                            "Drop image"
                                        }
                                        p {
                                            class: "text-xs text-gray-500 mt-1",
                                            "or click to browse"
                                        }
                                    }
                                }
                            }

                            // Hidden file input
                            input {
                                r#type: "file",
                                accept: "image/*",
                                class: "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
                                onchange: handle_thumbnail_upload,
                                disabled: uploading()
                            }
                        }
                    }
                }
            }
        }
    }
}
