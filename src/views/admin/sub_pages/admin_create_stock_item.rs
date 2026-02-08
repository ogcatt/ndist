#![allow(non_snake_case)]

use dioxus::prelude::*;
use strum::IntoEnumIterator;
use chrono::NaiveDateTime;
use crate::Route;
use crate::backend::server_functions::{
    admin_upload_private_thumbnails, admin_get_stock_items, admin_get_stock_batches,
    admin_get_stock_item_relations, admin_get_pre_or_back_order_reduces,
    CreateStockItemRequest, EditStockItemRequest, EditStockBatchRequest,
    admin_create_stock_item, admin_edit_stock_item,
};
use crate::backend::front_entities::*;
use crate::components::*;

// Unique entry for each batch of stock (used in edit mode)
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

#[derive(PartialEq, Props, Clone)]
pub struct AdminStockItemProps {
    pub id: Option<ReadSignal<String>>,
}

#[component]
pub fn AdminStockItem(props: AdminStockItemProps) -> Element {
    let is_edit_mode = props.id.is_some();
    // Use a Signal for internal state, but read from props.id in edit mode
    let mut stock_item_id = use_signal(|| String::new());
    let props_id = props.id;

    use_effect(move || {
        if let Some(id) = props_id {
            stock_item_id.set(id());
        }
    });

    let mut current_stock_item = use_signal(|| None::<StockItem>);
    let mut stock_item_not_found = use_signal(|| false);
    let mut loading = use_signal(|| !is_edit_mode);

    // Basic stock item info
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

    // UI states
    let mut uploading = use_signal(|| false);
    let mut saving = use_signal(|| false);

    // Create notification signals
    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new());
    let mut show_notification = use_signal(|| false);

    // Edit mode specific: Stock Item Batches
    let mut batches: Signal<Vec<EditStockBatch>> = use_signal(|| vec![]);
    let mut stock_item_relations: Signal<Vec<StockItemRelation>> = use_signal(|| vec![]);
    let mut back_order_reduces: Signal<Vec<BackOrPreOrderActiveReduce>> = use_signal(|| vec![]);
    let mut pre_order_reduces: Signal<Vec<BackOrPreOrderActiveReduce>> = use_signal(|| vec![]);
    let mut show_reduces_dropdown = use_signal(|| false);
    let mut flatten_pre_or_back_reduces = use_signal(|| false);

    // Edit mode specific: Existing data for dropdowns
    let mut existing_stock_items = use_resource(move || async move {
        admin_get_stock_items().await
    });

    let mut existing_stock_batches = use_resource(move || async move {
        admin_get_stock_batches().await
    });

    let mut existing_stock_item_relations = use_resource(move || async move {
        admin_get_stock_item_relations().await
    });

    let mut pre_back_order_reduces = use_resource(move || async move {
        if is_edit_mode {
            admin_get_pre_or_back_order_reduces(stock_item_id()).await
        } else {
            Ok((vec![], vec![]))
        }
    });

    // Initialize edit mode data
    use_effect(move || {
        if is_edit_mode {
            let current_id = stock_item_id();

            // Initialize pre/back order reduces
            if let Some(Ok((back_orders, pre_orders))) = pre_back_order_reduces.read().as_ref() {
                back_order_reduces.set(back_orders.clone());
                pre_order_reduces.set(pre_orders.clone());
            }

            // Initialize stock item relations
            if let Some(Ok(relations)) = existing_stock_item_relations.read().as_ref() {
                let filtered_relations: Vec<StockItemRelation> = relations
                    .iter()
                    .filter(|relation| current_id == relation.parent_stock_item_id)
                    .cloned()
                    .collect();
                stock_item_relations.set(filtered_relations);
            }

            // Initialize batches
            if let Some(Ok(stock_batches)) = existing_stock_batches.read().as_ref() {
                let edit_stock_batches = stock_batches
                    .iter()
                    .filter(|b| b.stock_item_id == current_id)
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

            // Initialize stock item data
            if let Some(Ok(stock_items)) = existing_stock_items.read().as_ref() {
                if let Some(stock_item) = stock_items.iter().find(|item| item.id == current_id) {
                    current_stock_item.set(Some(stock_item.clone()));
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
        }
    });

    let handle_thumbnail_upload = move |evt: FormEvent| {
        let mut thumbnail_ref = thumbnail_ref.clone();
        let mut uploading = uploading.clone();

        spawn(async move {
            let files = evt.files();
            if !files.is_empty() {
                if let Some(file_data) = files.get(0) {
                    uploading.set(true);
                    let file_name = file_data.name();

                    let file_contents = file_data.read_bytes().await;
                    let content_type = if file_name.ends_with(".jpg") || file_name.ends_with(".jpeg") {
                        "image/jpeg"
                    } else if file_name.ends_with(".png") {
                        "image/png"
                    } else if file_name.ends_with(".webp") {
                        "image/webp"
                    } else if file_name.ends_with(".avif") {
                        "image/avif"
                    } else {
                        "image/jpeg"
                    };

                    match file_contents {
                        Ok(bytes) => {
                            match admin_upload_private_thumbnails(
                                bytes.to_vec(),
                                file_name.clone(),
                                content_type.to_string()
                            ).await {
                                Ok(response) => {
                                    if response.success {
                                        thumbnail_ref.set(response.url);
                                    } else {
                                        println!("Upload failed: {}", response.message);
                                    }
                                },
                                Err(e) => {
                                    println!("Upload error: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("File read error: {}", e);
                        }
                    }

                    uploading.set(false);
                }
            }
        });
    };

    let handle_submit = move |_| {
        spawn(async move {
            saving.set(true);

            if is_edit_mode {
                let request = EditStockItemRequest {
                    id: stock_item_id(),
                    name: name(),
                    pbi_sku: pbi_sku(),
                    description: if description().trim().is_empty() { None } else { Some(description()) },
                    thumbnail_ref: thumbnail_ref(),
                    unit: unit(),
                    assembly_minutes: if assembly_minutes() == 0 { None } else { Some(assembly_minutes()) },
                    default_shipping_days: if default_shipping_days() == 0 { None } else { Some(default_shipping_days()) },
                    default_cost: if default_cost() == 0.0 { None } else { Some(default_cost()) },
                    warning_quantity: if warning_quantity() == 0.0 { None } else { Some(warning_quantity()) },
                    is_container: is_container(),
                    flatten_pre_or_back_reduces: flatten_pre_or_back_reduces(),
                    batches: if batches().is_empty() { None } else {
                        Some(batches().iter().filter_map(|b| {
                            Some(EditStockBatchRequest {
                                id: b.id.clone(),
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
                                created_at: b.created_at.clone(),
                                updated_at: b.updated_at.clone(),
                            })
                        }).collect())
                    },
                    stock_item_relations: if stock_item_relations().is_empty() { None } else { Some(stock_item_relations()) },
                };

                match admin_edit_stock_item(request).await {
                    Ok(response) => {
                        notification_message.set(response.message);
                        notification_type.set(if response.success { "success".to_string() } else { "error".to_string() });
                        show_notification.set(true);
                    }
                    Err(e) => {
                        notification_message.set(format!("Error: {}", e));
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
            } else {
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

                let request = CreateStockItemRequest {
                    name: name(),
                    pbi_sku: pbi_sku(),
                    description: if description().trim().is_empty() { None } else { Some(description()) },
                    thumbnail_ref: thumbnail_ref(),
                    unit: unit(),
                    assembly_minutes: if assembly_minutes() == 0 { None } else { Some(assembly_minutes()) },
                    default_shipping_days: if default_shipping_days() == 0 { None } else { Some(default_shipping_days()) },
                    default_cost: if default_cost() == 0.0 { None } else { Some(default_cost()) },
                    warning_quantity: if warning_quantity() == 0.0 { None } else { Some(warning_quantity()) },
                    is_container: is_container(),
                };

                match admin_create_stock_item(request).await {
                    Ok(response) => {
                        if response.success {
                            notification_message.set("Stock item created successfully!".to_string());
                            notification_type.set("success".to_string());
                            show_notification.set(true);
                            name.set(String::new());
                            pbi_sku.set(String::new());
                            description.set(String::new());
                            thumbnail_ref.set(None);
                            unit.set(StockUnit::Multiples);
                            assembly_minutes.set(0);
                            default_shipping_days.set(0);
                            default_cost.set(0.0);
                            is_container.set(false);
                        } else {
                            notification_message.set(response.message);
                            notification_type.set("error".to_string());
                            show_notification.set(true);
                        }
                    }
                    Err(e) => {
                        notification_message.set(format!("Error: {}", e));
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
            }

            saving.set(false);
        });
    };

    // Show loading state
    if loading() {
        return rsx! {
            div { class: "p-8 text-center text-gray-500", "Loading..." }
        };
    }

    // Show not found in edit mode
    if is_edit_mode && stock_item_not_found() {
        return rsx! {
            div { class: "p-8 text-center text-red-500", "Stock item not found" }
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
                class: "text-lg font-medium",
                if is_edit_mode { "Edit Stock Item" } else { "Create New Stock Item" }
            }
            if is_edit_mode {
                Link {
                    to: Route::AdminInventory {},
                    button {
                        class: "text-sm bg-gray-500 px-3 py-2 text-white rounded hover:bg-gray-600 transition-colors mr-2",
                        "Back to Inventory"
                    }
                }
            }
            button {
                class: format!("text-sm px-3 py-2 text-white rounded transition-colors {}",
                    if saving() { "bg-gray-500 cursor-not-allowed" } else { "bg-gray-900 hover:bg-gray-800" }
                ),
                disabled: saving(),
                onclick: handle_submit,
                if saving() {
                    if is_edit_mode { "Saving..." } else { "Creating..." }
                } else {
                    if is_edit_mode { "Save Changes" } else { "Create" }
                }
            }
        }

        div {
            class: "flex flex-col md:flex-row w-full gap-2",
            div {
                class: "flex w-full flex-col gap-2",

                // Basic Info Section
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                    h2 { class: "text-lg font-medium mb-4", "Basic Information" }
                    div {
                        class: "flex gap-4 w-full mb-4",
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "Name",
                                value: "{name}",
                                placeholder: "Stock Item Name",
                                optional: false,
                                oninput: move |event: FormEvent| name.set(event.value())
                            }
                        },
                        div {
                            class: "w-full",
                            CTextBox {
                                label: "PBI/PBX SKU",
                                value: "{pbi_sku}",
                                placeholder: "PBI0001 or PBX0001",
                                optional: false,
                                oninput: move |event: FormEvent| {
                                    let mut value = event.value();
                                    if (!value.starts_with("P") && !value.starts_with("PB")) && !value.is_empty() {
                                        value = format!("PBI{}", value);
                                    }
                                    pbi_sku.set(value);
                                }
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

                // Thumbnail Section
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                    h2 { class: "text-lg font-medium mb-4", "Thumbnail" }
                    div {
                        class: "w-full max-w-xs mx-auto",
                        div {
                            class: "aspect-square w-full border-2 border-dashed border-gray-300 rounded-lg hover:border-gray-400 transition-colors cursor-pointer bg-gray-50 hover:bg-gray-100 flex flex-col items-center justify-center relative overflow-hidden",

                            if let Some(url) = thumbnail_ref() {
                                img {
                                    src: "{url}",
                                    class: "w-full h-full object-cover rounded-lg",
                                    alt: "Stock item thumbnail"
                                }
                                button {
                                    class: "absolute top-2 right-2 bg-red-500 text-white rounded-full w-6 h-6 flex items-center justify-center text-sm hover:bg-red-600",
                                    onclick: move |_| thumbnail_ref.set(None),
                                    "×"
                                }
                            } else {
                                div {
                                    class: "text-center p-4",
                                    if uploading() {
                                        div { class: "animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900 mx-auto mb-2" }
                                        span { class: "text-sm text-gray-600", "Uploading..." }
                                    } else {
                                        svg {
                                            class: "mx-auto h-8 w-8 text-gray-400 mb-2",
                                            stroke: "currentColor", fill: "none", view_box: "0 0 48 48",
                                            path {
                                                d: "M28 8H12a4 4 0 00-4 4v20m32-12v8m0 0v8a4 4 0 01-4 4H12a4 4 0 01-4-4v-4m32-4l-3.172-3.172a4 4 0 00-5.656 0L28 28M8 32l9.172-9.172a4 4 0 015.656 0L28 28m0 0l4 4m4-24h8m-4-4v8m-12 4h.02",
                                                stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                                            }
                                        }
                                        span { class: "text-sm text-gray-600 font-medium", "Drop image" }
                                        p { class: "text-xs text-gray-500 mt-1", "or click to browse" }
                                    }
                                }
                            }

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

            // Right sidebar
            div {
                class: "md:w-[38%] w-full min-w-0",

                // Unit and Container Settings
                div {
                    class: "bg-white border flex flex-col gap-4 rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 { class: "text-lg font-medium mb-4", "Settings" }

                    CSelectGroup {
                        label: "Unit Type",
                        optional: false,
                        oninput: move |event: FormEvent| {
                            if let Ok(stock_unit) = event.value().parse::<StockUnit>() {
                                unit.set(stock_unit);
                            }
                        },
                        for unit_type in StockUnit::iter() {
                            CSelectItem {
                                selected: if unit_type == StockUnit::Multiples { true } else { false },
                                key: "{unit_type:?}",
                                value: "{unit_type}",
                                "{unit_type.to_string()}"
                            }
                        }
                    }

                    div {
                        class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                        p { class: "text-sm text-gray-700 pt-[2px]", "Contains other items" }
                        CToggle { checked: is_container(), onclick: move |_| is_container.toggle() }
                    }
                }

                // Cost and Time Settings
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 { class: "text-lg font-medium mb-4", "Cost & Time" }
                    div { class: "flex flex-col gap-4",
                        CTextBox {
                            label: "Default Cost (per unit)",
                            value: if default_cost() == 0.0 { String::new() } else { format!("{}", default_cost()) },
                            placeholder: "0", prefix: "$", is_number: true, step: 1f64, optional: true,
                            oninput: move |event: FormEvent| {
                                if event.value().is_empty() { default_cost.set(0.0); }
                                else if let Ok(cost) = event.value().parse::<f64>() { default_cost.set(cost); }
                            }
                        }
                        CTextBox {
                            label: "Assembly Time",
                            value: if assembly_minutes() == 0 { String::new() } else { format!("{}", assembly_minutes()) },
                            placeholder: "0", suffix: "min", is_number: true, step: 1f64, optional: true,
                            oninput: move |event: FormEvent| {
                                if event.value().is_empty() { assembly_minutes.set(0); }
                                else if let Ok(minutes) = event.value().parse::<i32>() { assembly_minutes.set(minutes); }
                            }
                        }
                        CTextBox {
                            label: "Default Shipping Days",
                            value: if default_shipping_days() == 0 { String::new() } else { format!("{}", default_shipping_days()) },
                            placeholder: "0", suffix: "days", is_number: true, step: 1f64, optional: true,
                            oninput: move |event: FormEvent| {
                                if event.value().is_empty() { default_shipping_days.set(0); }
                                else if let Ok(days) = event.value().parse::<i32>() { default_shipping_days.set(days); }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Backward-compatible aliases
#[component]
pub fn AdminCreateStockItem() -> Element {
    AdminStockItem(AdminStockItemProps { id: None })
}

#[component]
pub fn AdminEditStockItem(id: ReadSignal<String>) -> Element {
    AdminStockItem(AdminStockItemProps { id: Some(id) })
}