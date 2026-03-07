#![allow(non_snake_case)]

use dioxus::prelude::*;
use crate::Route;
use crate::backend::server_functions::{
    admin_upload_private_thumbnails, admin_get_stock_items, admin_get_pre_or_back_order_reduces,
    admin_get_stock_locations, admin_adjust_stock_quantity, admin_toggle_stock_location,
    admin_get_stock_adjustment_history,
    CreateStockItemRequest, EditStockItemRequest,
    admin_create_stock_item, admin_edit_stock_item,
};
use std::collections::HashMap;
use crate::backend::front_entities::*;
use crate::components::*;

#[derive(PartialEq, Props, Clone)]
pub struct AdminStockItemProps {
    pub id: Option<ReadSignal<String>>,
}

#[component]
pub fn AdminStockItem(props: AdminStockItemProps) -> Element {
    let is_edit_mode = props.id.is_some();
    let mut stock_item_id = use_signal(|| String::new());
    let props_id = props.id;

    use_effect(move || {
        if let Some(id) = props_id {
            stock_item_id.set(id());
        }
    });

    let mut stock_item_not_found = use_signal(|| false);
    let mut loading = use_signal(|| is_edit_mode);

    // Basic stock item info
    let mut name = use_signal(|| String::new());
    // Stores the part after the "NDI" prefix
    let mut pbi_sku_suffix = use_signal(|| String::new());
    let mut description = use_signal(|| String::new());
    let mut thumbnail_ref = use_signal(|| Option::<String>::None);

    // Stock item settings
    let mut assembly_minutes = use_signal(|| 0i32);
    let mut default_shipping_days = use_signal(|| 0i32);
    let mut default_cost = use_signal(|| 0.0f64);
    let mut warning_quantity = use_signal(|| 0i32);

    // UI states
    let mut uploading = use_signal(|| false);
    let mut upload_dots = use_signal(|| 0u8);
    let mut saving = use_signal(|| false);

    // Animate dots while uploading
    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(1000).await;
            if uploading() {
                upload_dots.set((upload_dots() + 1) % 3);
            }
        }
    });

    // Notification signals
    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new());
    let mut show_notification = use_signal(|| false);

    // Edit mode specific
    let mut back_order_reduces: Signal<Vec<BackOrPreOrderActiveReduce>> = use_signal(|| vec![]);
    let mut pre_order_reduces: Signal<Vec<BackOrPreOrderActiveReduce>> = use_signal(|| vec![]);
    let mut flatten_pre_or_back_reduces = use_signal(|| false);

    // Per-location adjustment state
    let mut adj_deltas: Signal<HashMap<String, String>> = use_signal(HashMap::new);
    let mut adj_notes: Signal<HashMap<String, String>> = use_signal(HashMap::new);
    let mut adj_applying: Signal<Option<String>> = use_signal(|| None);
    let mut toggling: Signal<Option<String>> = use_signal(|| None);

    // History expansion: loc_id -> expanded bool; cached data: slq_id -> history vec
    let mut expanded_loc: Signal<Option<String>> = use_signal(|| None);
    let mut history_cache: Signal<HashMap<String, Vec<StockQuantityAdjustment>>> = use_signal(HashMap::new);
    let mut history_loading: Signal<Option<String>> = use_signal(|| None);

    let mut existing_stock_items = use_resource(move || async move {
        admin_get_stock_items().await
    });

    let locations_resource = use_resource(move || async move {
        admin_get_stock_locations().await
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

            if let Some(Ok((back_orders, pre_orders))) = pre_back_order_reduces.read().as_ref() {
                back_order_reduces.set(back_orders.clone());
                pre_order_reduces.set(pre_orders.clone());
            }

            if let Some(Ok(stock_items)) = existing_stock_items.read().as_ref() {
                if let Some(stock_item) = stock_items.iter().find(|item| item.id == current_id) {
                    name.set(stock_item.name.clone());
                    // Strip "NDI" prefix for display in the suffix input
                    let sku = &stock_item.pbi_sku;
                    pbi_sku_suffix.set(sku.strip_prefix("NDI").unwrap_or(sku).to_string());
                    description.set(stock_item.description.clone().unwrap_or_default());
                    thumbnail_ref.set(stock_item.thumbnail_ref.clone());
                    assembly_minutes.set(stock_item.assembly_minutes.unwrap_or(0));
                    default_shipping_days.set(stock_item.default_shipping_days.unwrap_or(0));
                    default_cost.set(stock_item.default_cost.unwrap_or(0.0));
                    warning_quantity.set(stock_item.warning_quantity.unwrap_or(0));
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
                                    }
                                },
                                Err(e) => {
                                    tracing::error!("Upload error: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("File read error: {}", e);
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
            // Full SKU is "NDI" + suffix
            let full_sku = format!("NDI{}", pbi_sku_suffix());

            if is_edit_mode {
                let request = EditStockItemRequest {
                    id: stock_item_id(),
                    name: name(),
                    pbi_sku: full_sku,
                    description: if description().trim().is_empty() { None } else { Some(description()) },
                    thumbnail_ref: thumbnail_ref(),
                    assembly_minutes: if assembly_minutes() == 0 { None } else { Some(assembly_minutes()) },
                    default_shipping_days: if default_shipping_days() == 0 { None } else { Some(default_shipping_days()) },
                    default_cost: if default_cost() == 0.0 { None } else { Some(default_cost()) },
                    warning_quantity: if warning_quantity() == 0 { None } else { Some(warning_quantity()) },
                    flatten_pre_or_back_reduces: flatten_pre_or_back_reduces(),
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

                if pbi_sku_suffix().trim().is_empty() {
                    notification_message.set("SKU is required".to_string());
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                    saving.set(false);
                    return;
                }

                let request = CreateStockItemRequest {
                    name: name(),
                    pbi_sku: full_sku,
                    description: if description().trim().is_empty() { None } else { Some(description()) },
                    thumbnail_ref: thumbnail_ref(),
                    assembly_minutes: if assembly_minutes() == 0 { None } else { Some(assembly_minutes()) },
                    default_shipping_days: if default_shipping_days() == 0 { None } else { Some(default_shipping_days()) },
                    default_cost: if default_cost() == 0.0 { None } else { Some(default_cost()) },
                    warning_quantity: if warning_quantity() == 0 { None } else { Some(warning_quantity()) },
                };

                match admin_create_stock_item(request).await {
                    Ok(response) => {
                        if response.success {
                            notification_message.set("Stock item created successfully!".to_string());
                            notification_type.set("success".to_string());
                            show_notification.set(true);
                            name.set(String::new());
                            pbi_sku_suffix.set(String::new());
                            description.set(String::new());
                            thumbnail_ref.set(None);
                            assembly_minutes.set(0);
                            default_shipping_days.set(0);
                            default_cost.set(0.0);
                            warning_quantity.set(0);
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

    if loading() {
        return rsx! {
            div { class: "p-8 text-center text-gray-500", "Loading..." }
        };
    }

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

            // LEFT COLUMN
            div {
                class: "flex w-full flex-col gap-2",

                // Basic Info
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4",
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
                                label: "SKU",
                                value: "{pbi_sku_suffix}",
                                placeholder: "0001",
                                prefix: "NDI",
                                optional: false,
                                oninput: move |event: FormEvent| pbi_sku_suffix.set(event.value())
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

                // Stock Quantities (edit mode only)
                if is_edit_mode {
                    {
                        let item_id = stock_item_id();
                        // qty_map: location_id -> (slq_id, quantity, enabled)
                        let qty_map: HashMap<String, (String, i32, bool)> = if let Some(Ok(items)) = existing_stock_items.read().as_ref() {
                            if let Some(item) = items.iter().find(|i| i.id == item_id) {
                                item.location_quantities.as_deref().unwrap_or(&[])
                                    .iter()
                                    .map(|lq| (lq.stock_location_id.clone(), (lq.id.clone(), lq.quantity, lq.enabled)))
                                    .collect()
                            } else { HashMap::new() }
                        } else { HashMap::new() };

                        rsx! {
                            div {
                                class: "bg-white border rounded-md border-gray-200 p-4",
                                div { class: "flex justify-between items-center mb-3",
                                    h2 { class: "text-lg font-medium", "Stock Quantities" }
                                    Link {
                                        to: Route::AdminStockLocations {},
                                        span { class: "text-xs text-gray-500 hover:text-gray-700 underline cursor-pointer", "Manage Locations" }
                                    }
                                }

                                match locations_resource.read().as_ref() {
                                    Some(Ok(locations)) if !locations.is_empty() => rsx! {
                                        div { class: "flex flex-col gap-2",
                                            for loc in locations.iter() {
                                                {
                                                    let loc_id = loc.id.clone();
                                                    let loc_id_toggle = loc.id.clone();
                                                    let loc_id_apply = loc.id.clone();
                                                    let loc_id_hist = loc.id.clone();
                                                    let loc_name = loc.name.clone();
                                                    let item_id2 = item_id.clone();
                                                    let entry = qty_map.get(&loc.id).cloned();
                                                    let is_enabled = entry.as_ref().map(|(_, _, e)| *e).unwrap_or(false);
                                                    let current_qty = entry.as_ref().map(|(_, q, _)| *q).unwrap_or(0);
                                                    let slq_id = entry.as_ref().map(|(id, _, _)| id.clone()).unwrap_or_default();
                                                    let delta_val = adj_deltas.read().get(&loc.id).cloned().unwrap_or_default();
                                                    let note_val = adj_notes.read().get(&loc.id).cloned().unwrap_or_default();
                                                    let is_applying = adj_applying().as_deref() == Some(&loc.id);
                                                    let is_toggling = toggling().as_deref() == Some(&loc.id);
                                                    let is_expanded = expanded_loc().as_deref() == Some(&loc.id);
                                                    let hist_loading = history_loading().as_deref() == Some(&loc.id);
                                                    let cached_history = slq_id.clone();
                                                    let has_history = !history_cache.read().get(&slq_id).unwrap_or(&vec![]).is_empty();
                                                    let history_entries = history_cache.read().get(&slq_id).cloned().unwrap_or_default();

                                                    // Gray out if disabled but has quantity
                                                    let card_class = if !is_enabled && current_qty > 0 {
                                                        "border border-gray-200 rounded-md p-3 opacity-60 bg-gray-50"
                                                    } else if is_enabled {
                                                        "border border-gray-200 rounded-md p-3"
                                                    } else {
                                                        "border border-dashed border-gray-200 rounded-md p-3 bg-gray-50"
                                                    };

                                                    rsx! {
                                                        div {
                                                            key: "{loc_id}",
                                                            class: "{card_class}",

                                                            // Header row: name, qty badge, toggle
                                                            div { class: "flex justify-between items-center",
                                                                span { class: "text-sm font-medium text-gray-800", "{loc_name}" }
                                                                div { class: "flex items-center gap-2",
                                                                    if is_enabled || current_qty > 0 {
                                                                        span {
                                                                            class: "text-xs px-2 py-0.5 rounded-full bg-gray-100 text-gray-700 font-mono",
                                                                            "{current_qty} units"
                                                                        }
                                                                    }
                                                                    {
                                                                        let item_id3 = item_id2.clone();
                                                                        rsx! {
                                                                            div {
                                                                                class: if is_toggling { "opacity-50 pointer-events-none" } else { "" },
                                                                                CToggle {
                                                                                    checked: is_enabled,
                                                                                    onclick: move |_| {
                                                                                        let loc_id_t = loc_id_toggle.clone();
                                                                                        let item_id_t = item_id3.clone();
                                                                                        let new_state = !is_enabled;
                                                                                        spawn(async move {
                                                                                            toggling.set(Some(loc_id_t.clone()));
                                                                                            match admin_toggle_stock_location(item_id_t, loc_id_t.clone(), new_state).await {
                                                                                                Ok(()) => { existing_stock_items.restart(); }
                                                                                                Err(e) => {
                                                                                                    notification_message.set(format!("{e}"));
                                                                                                    notification_type.set("error".to_string());
                                                                                                    show_notification.set(true);
                                                                                                }
                                                                                            }
                                                                                            toggling.set(None);
                                                                                        });
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }

                                                            // Adjustment controls (enabled only)
                                                            if is_enabled {
                                                                div { class: "mt-2 flex gap-2",
                                                                    input {
                                                                        class: "w-24 border border-gray-300 rounded px-2 py-1 text-sm font-mono focus:outline-none focus:ring-1 focus:ring-gray-400",
                                                                        r#type: "number",
                                                                        placeholder: "+/- qty",
                                                                        value: "{delta_val}",
                                                                        oninput: move |e| {
                                                                            adj_deltas.write().insert(loc_id.clone(), e.value());
                                                                        },
                                                                    }
                                                                    input {
                                                                        class: "flex-1 border border-gray-300 rounded px-2 py-1 text-sm focus:outline-none focus:ring-1 focus:ring-gray-400",
                                                                        placeholder: "Reason / note (required)",
                                                                        value: "{note_val}",
                                                                        oninput: move |e| {
                                                                            adj_notes.write().insert(loc_id_apply.clone(), e.value());
                                                                        },
                                                                    }
                                                                }
                                                                div { class: "mt-2 flex gap-2",
                                                                    button {
                                                                        class: format!("flex-1 text-xs px-3 py-1.5 rounded transition-colors {}",
                                                                            if is_applying { "bg-gray-200 text-gray-400 cursor-not-allowed" }
                                                                            else { "bg-gray-800 text-white hover:bg-gray-700" }
                                                                        ),
                                                                        disabled: is_applying,
                                                                        onclick: {
                                                                            let item_id4 = item_id2.clone();
                                                                            let loc_id5 = loc.id.clone();
                                                                            move |_| {
                                                                                let delta_str = adj_deltas.read().get(&loc_id5).cloned().unwrap_or_default();
                                                                                let note = adj_notes.read().get(&loc_id5).cloned().unwrap_or_default();
                                                                                let delta: i32 = match delta_str.trim().parse() {
                                                                                    Ok(d) => d,
                                                                                    Err(_) => {
                                                                                        notification_message.set("Delta must be a whole number".to_string());
                                                                                        notification_type.set("error".to_string());
                                                                                        show_notification.set(true);
                                                                                        return;
                                                                                    }
                                                                                };
                                                                                if delta == 0 { return; }
                                                                                let loc_id6 = loc_id5.clone();
                                                                                let item_id5 = item_id4.clone();
                                                                                spawn(async move {
                                                                                    adj_applying.set(Some(loc_id6.clone()));
                                                                                    match admin_adjust_stock_quantity(item_id5, loc_id6.clone(), delta, note).await {
                                                                                        Ok(new_qty) => {
                                                                                            adj_deltas.write().remove(&loc_id6);
                                                                                            adj_notes.write().remove(&loc_id6);
                                                                                            // Invalidate history cache for this location
                                                                                            history_cache.write().remove(&loc_id6);
                                                                                            notification_message.set(format!("Quantity updated to {new_qty}"));
                                                                                            notification_type.set("success".to_string());
                                                                                            show_notification.set(true);
                                                                                            existing_stock_items.restart();
                                                                                        }
                                                                                        Err(e) => {
                                                                                            notification_message.set(format!("{e}"));
                                                                                            notification_type.set("error".to_string());
                                                                                            show_notification.set(true);
                                                                                        }
                                                                                    }
                                                                                    adj_applying.set(None);
                                                                                });
                                                                            }
                                                                        },
                                                                        if is_applying { "Applying..." } else { "Apply Adjustment" }
                                                                    }
                                                                    // History expand button (only if there's a record)
                                                                    if !slq_id.is_empty() {
                                                                        button {
                                                                            class: format!("text-xs px-3 py-1.5 rounded border transition-colors {}",
                                                                                if is_expanded { "border-gray-400 bg-gray-100 text-gray-700" }
                                                                                else { "border-gray-300 text-gray-500 hover:bg-gray-50" }
                                                                            ),
                                                                            onclick: {
                                                                                let slq = cached_history.clone();
                                                                                let loc_id7 = loc_id_hist.clone();
                                                                                move |_| {
                                                                                    if is_expanded {
                                                                                        expanded_loc.set(None);
                                                                                    } else {
                                                                                        expanded_loc.set(Some(loc_id7.clone()));
                                                                                        // Fetch if not cached
                                                                                        if !history_cache.read().contains_key(&slq) {
                                                                                            let slq2 = slq.clone();
                                                                                            let loc_id8 = loc_id7.clone();
                                                                                            spawn(async move {
                                                                                                history_loading.set(Some(loc_id8.clone()));
                                                                                                if let Ok(history) = admin_get_stock_adjustment_history(slq2.clone()).await {
                                                                                                    history_cache.write().insert(slq2, history);
                                                                                                }
                                                                                                history_loading.set(None);
                                                                                            });
                                                                                        }
                                                                                    }
                                                                                }
                                                                            },
                                                                            if hist_loading { "Loading..." }
                                                                            else if is_expanded { "Hide History" }
                                                                            else { "History" }
                                                                        }
                                                                    }
                                                                }

                                                                // History panel
                                                                if is_expanded {
                                                                    div {
                                                                        class: "mt-2 max-h-48 overflow-y-auto border border-gray-200 rounded-md bg-gray-50",
                                                                        if history_entries.is_empty() && !hist_loading {
                                                                            p { class: "text-xs text-gray-400 p-3 text-center", "No adjustments recorded yet" }
                                                                        } else {
                                                                            div { class: "divide-y divide-gray-200",
                                                                                for adj in history_entries.iter() {
                                                                                    {
                                                                                        let delta_display = if adj.delta >= 0 {
                                                                                            format!("+{}", adj.delta)
                                                                                        } else {
                                                                                            format!("{}", adj.delta)
                                                                                        };
                                                                                        let delta_color = if adj.delta >= 0 { "text-green-700" } else { "text-red-700" };
                                                                                        let date_str = adj.created_at.format("%Y-%m-%d %H:%M").to_string();
                                                                                        rsx! {
                                                                                            div { class: "px-3 py-2 flex justify-between items-start gap-2",
                                                                                                div { class: "flex-1 min-w-0",
                                                                                                    p { class: "text-xs text-gray-700 truncate", "{adj.note}" }
                                                                                                    p { class: "text-xs text-gray-400", "{date_str}" }
                                                                                                }
                                                                                                span {
                                                                                                    class: "text-xs font-mono font-semibold shrink-0 {delta_color}",
                                                                                                    "{delta_display}"
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

                                                            // Disabled but has qty: show note
                                                            if !is_enabled && current_qty > 0 {
                                                                p { class: "mt-1 text-xs text-gray-400 italic", "Disabled — {current_qty} units held" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    Some(Ok(_)) => rsx! {
                                        p { class: "text-sm text-gray-500",
                                            "No stock locations exist. "
                                            Link { to: Route::AdminStockLocations {}, span { class: "underline hover:text-gray-700", "Create one first." } }
                                        }
                                    },
                                    Some(Err(e)) => rsx! { p { class: "text-sm text-red-500", "Error: {e}" } },
                                    None => rsx! { p { class: "text-sm text-gray-400", "Loading locations..." } },
                                }
                            }
                        }
                    }
                }
            }

            // RIGHT SIDEBAR
            div {
                class: "md:w-[38%] w-full min-w-0",

                // Thumbnail (now at top of sidebar)
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 mb-2",
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
                                        span {
                                            class: "text-sm text-gray-600",
                                            { format!("Converting file{}", ".".repeat(1 + upload_dots() as usize)) }
                                        }
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

                // Settings
                div {
                    class: "bg-white border flex flex-col gap-4 rounded-md border-gray-200 p-4 mb-2",
                    h2 { class: "text-lg font-medium mb-2", "Settings" }

                    CTextBox {
                        label: "Warning Quantity",
                        value: if warning_quantity() == 0 { String::new() } else { format!("{}", warning_quantity()) },
                        placeholder: "0", suffix: "units", is_number: true, step: 1f64, optional: true,
                        oninput: move |event: FormEvent| {
                            if event.value().is_empty() { warning_quantity.set(0); }
                            else if let Ok(qty) = event.value().parse::<i32>() { warning_quantity.set(qty); }
                        }
                    }

                    if is_edit_mode {
                        div {
                            class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                            div {
                                p { class: "text-sm text-gray-700 pt-[2px]", "Flatten pre/back-order reduces" }
                                p { class: "text-xs text-gray-500", "Sets all active reduces to inactive" }
                            }
                            CToggle {
                                checked: flatten_pre_or_back_reduces(),
                                onclick: move |_| flatten_pre_or_back_reduces.toggle()
                            }
                        }
                    }
                }

                // Cost and Time Settings
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 mb-2",
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

                // Active Reduces summary (edit mode only)
                if is_edit_mode {
                    {
                        let back_count = back_order_reduces.read().iter().filter(|r| r.active).count();
                        let pre_count = pre_order_reduces.read().iter().filter(|r| r.active).count();
                        rsx! {
                            div {
                                class: "bg-white border rounded-md border-gray-200 p-4 mb-2",
                                h2 { class: "text-lg font-medium mb-2", "Active Reduces" }
                                div { class: "flex flex-col gap-1 text-sm text-gray-700",
                                    div { "Back-order reduces: {back_count} active" }
                                    div { "Pre-order reduces: {pre_count} active" }
                                }
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
