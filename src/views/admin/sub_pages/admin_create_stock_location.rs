#![allow(non_snake_case)]

use dioxus::prelude::*;
use crate::Route;
use crate::backend::server_functions::{
    admin_get_stock_locations,
    admin_create_stock_location, admin_edit_stock_location, admin_delete_stock_location,
    CreateStockLocationRequest, EditStockLocationRequest,
};
use crate::backend::front_entities::*;

#[component]
pub fn AdminStockLocations() -> Element {
    let mut locations = use_resource(move || async move {
        admin_get_stock_locations().await
    });

    // Form state – shared between create and edit
    let mut editing_id: Signal<Option<String>> = use_signal(|| None);
    let mut form_name = use_signal(|| String::new());
    let mut form_description = use_signal(|| String::new());
    let mut form_shipping_method = use_signal(|| StockLocationShippingMethod::Manual);
    let mut form_flat_rate = use_signal(|| String::new());
    let mut form_country = use_signal(|| String::new());
    let mut saving = use_signal(|| false);
    let mut deleting: Signal<Option<String>> = use_signal(|| None);
    let mut error_msg = use_signal(|| String::new());
    let mut success_msg = use_signal(|| String::new());

    let mut clear_form = move || {
        editing_id.set(None);
        form_name.set(String::new());
        form_description.set(String::new());
        form_shipping_method.set(StockLocationShippingMethod::Manual);
        form_flat_rate.set(String::new());
        form_country.set(String::new());
        error_msg.set(String::new());
    };

    let mut fill_form_for_edit = move |loc: &StockLocation| {
        editing_id.set(Some(loc.id.clone()));
        form_name.set(loc.name.clone());
        form_description.set(loc.description.clone().unwrap_or_default());
        form_shipping_method.set(loc.shipping_method.clone());
        form_flat_rate.set(loc.flat_rate_usd.map(|v| format!("{:.2}", v)).unwrap_or_default());
        form_country.set(loc.country.clone().unwrap_or_default());
        error_msg.set(String::new());
    };

    let handle_submit = move |_| {
        let name = form_name().trim().to_string();
        if name.is_empty() {
            error_msg.set("Name is required".to_string());
            return;
        }
        let description = form_description().trim().to_string();
        let flat_rate_usd = form_flat_rate().trim().parse::<f64>().ok().filter(|&v| v > 0.0);
        let country = {
            let c = form_country().trim().to_string();
            if c.is_empty() { None } else { Some(c) }
        };
        let description_opt = if description.is_empty() { None } else { Some(description) };
        let shipping_method = form_shipping_method();
        let id = editing_id();

        spawn(async move {
            saving.set(true);
            error_msg.set(String::new());

            let result = if let Some(edit_id) = id {
                admin_edit_stock_location(EditStockLocationRequest {
                    id: edit_id,
                    name,
                    description: description_opt,
                    shipping_method,
                    flat_rate_usd,
                    country,
                }).await
            } else {
                admin_create_stock_location(CreateStockLocationRequest {
                    name,
                    description: description_opt,
                    shipping_method,
                    flat_rate_usd,
                    country,
                }).await
            };

            match result {
                Ok(()) => {
                    success_msg.set(if editing_id().is_some() {
                        "Location updated".to_string()
                    } else {
                        "Location created".to_string()
                    });
                    clear_form();
                    locations.restart();
                }
                Err(e) => error_msg.set(format!("{}", e)),
            }
            saving.set(false);
        });
    };

    let handle_delete = move |id: String| {
        spawn(async move {
            deleting.set(Some(id.clone()));
            match admin_delete_stock_location(id).await {
                Ok(()) => {
                    success_msg.set("Location deleted".to_string());
                    locations.restart();
                }
                Err(e) => error_msg.set(format!("{}", e)),
            }
            deleting.set(None);
        });
    };

    rsx! {
        // Notifications
        if !success_msg().is_empty() {
            div {
                class: "fixed top-4 right-4 z-50 p-4 rounded-md shadow-lg text-white bg-green-500 flex items-center gap-3",
                span { "{success_msg()}" }
                button { class: "ml-2 hover:text-gray-200", onclick: move |_| success_msg.set(String::new()), "×" }
            }
        }

        div {
            class: "w-full flex flex-col gap-3",

            // Header
            div {
                class: "bg-white border rounded-md border-gray-200 p-4 h-16 flex justify-between items-center",
                div { class: "text-lg font-medium", "Stock Locations" }
                Link {
                    to: Route::AdminInventory {},
                    button {
                        class: "text-sm bg-gray-500 px-3 py-2 text-white rounded hover:bg-gray-600 transition-colors",
                        "Back to Inventory"
                    }
                }
            }

            div {
                class: "flex flex-col lg:flex-row gap-3",

                // Form panel
                div {
                    class: "lg:w-96 w-full bg-white border rounded-md border-gray-200 p-4 flex flex-col gap-3",

                    h2 {
                        class: "text-base font-medium",
                        if editing_id().is_some() { "Edit Location" } else { "New Location" }
                    }

                    // Name
                    div {
                        label { class: "text-sm font-medium text-gray-700 block mb-1", "Name" }
                        input {
                            class: "w-full border border-gray-300 rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400",
                            placeholder: "e.g. Warehouse A",
                            value: "{form_name}",
                            oninput: move |e| form_name.set(e.value()),
                        }
                    }

                    // Description
                    div {
                        label { class: "text-sm font-medium text-gray-700 block mb-1", "Description (optional)" }
                        textarea {
                            class: "w-full border border-gray-300 rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400 resize-none",
                            rows: 2,
                            placeholder: "Notes about this location...",
                            value: "{form_description}",
                            oninput: move |e| form_description.set(e.value()),
                        }
                    }

                    // Shipping method
                    div {
                        label { class: "text-sm font-medium text-gray-700 block mb-1", "Fulfilment Method" }
                        select {
                            class: "w-full border border-gray-300 rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400 bg-white",
                            value: if form_shipping_method() == StockLocationShippingMethod::Manual { "Manual" } else { "FlatRate" },
                            onchange: move |e| {
                                form_shipping_method.set(match e.value().as_str() {
                                    "FlatRate" => StockLocationShippingMethod::FlatRate,
                                    _ => StockLocationShippingMethod::Manual,
                                });
                            },
                            option { value: "Manual", "Manual" }
                            option { value: "FlatRate", "Flat Rate" }
                        }
                    }

                    // Flat rate (shown only when FlatRate is selected)
                    if form_shipping_method() == StockLocationShippingMethod::FlatRate {
                        div {
                            label { class: "text-sm font-medium text-gray-700 block mb-1", "Flat Rate (USD)" }
                            input {
                                class: "w-full border border-gray-300 rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400",
                                r#type: "number", min: "0", step: "0.01",
                                placeholder: "0.00",
                                value: "{form_flat_rate}",
                                oninput: move |e| form_flat_rate.set(e.value()),
                            }
                        }
                    }

                    // Country
                    div {
                        label { class: "text-sm font-medium text-gray-700 block mb-1", "Country (optional)" }
                        input {
                            class: "w-full border border-gray-300 rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400",
                            placeholder: "e.g. US",
                            value: "{form_country}",
                            oninput: move |e| form_country.set(e.value()),
                        }
                    }

                    if !error_msg().is_empty() {
                        div { class: "text-sm text-red-600", "{error_msg()}" }
                    }

                    div { class: "flex gap-2",
                        button {
                            class: format!("flex-1 text-sm px-3 py-2 text-white rounded transition-colors {}",
                                if saving() { "bg-gray-400 cursor-not-allowed" } else { "bg-gray-900 hover:bg-gray-800" }
                            ),
                            disabled: saving(),
                            onclick: handle_submit,
                            if saving() { "Saving..." } else if editing_id().is_some() { "Save Changes" } else { "Create Location" }
                        }
                        if editing_id().is_some() {
                            button {
                                class: "text-sm px-3 py-2 text-gray-700 border border-gray-300 rounded hover:bg-gray-50 transition-colors",
                                onclick: move |_| clear_form(),
                                "Cancel"
                            }
                        }
                    }
                }

                // Locations table
                div {
                    class: "flex-1 bg-white border rounded-md border-gray-200 overflow-hidden",
                    div {
                        class: "grid grid-cols-12 gap-2 px-4 py-3 bg-gray-50 border-b border-gray-200 text-xs font-medium text-gray-500 uppercase tracking-wide",
                        div { class: "col-span-3", "Name" }
                        div { class: "col-span-3", "Fulfilment" }
                        div { class: "col-span-3", "Country" }
                        div { class: "col-span-3", "" }
                    }
                    div {
                        class: "divide-y divide-gray-200",
                        match &*locations.read() {
                            Some(Ok(locs)) => {
                                if locs.is_empty() {
                                    rsx! {
                                        div { class: "p-8 text-center text-gray-400 text-sm", "No stock locations yet. Create one to the left." }
                                    }
                                } else {
                                    rsx! {
                                        for loc in locs.iter() {
                                            {
                                                let loc = loc.clone();
                                                let loc_for_edit = loc.clone();
                                                let loc_id_del = loc.id.clone();
                                                let is_deleting = deleting().as_deref() == Some(&loc.id);
                                                rsx! {
                                                    div {
                                                        key: "{loc.id}",
                                                        class: "grid grid-cols-12 gap-2 px-4 py-3 items-center hover:bg-gray-50 transition-colors",
                                                        div { class: "col-span-3",
                                                            p { class: "text-sm font-medium text-gray-900", "{loc.name}" }
                                                            if let Some(d) = &loc.description {
                                                                p { class: "text-xs text-gray-500 truncate", "{d}" }
                                                            }
                                                        }
                                                        div { class: "col-span-3",
                                                            span {
                                                                class: "text-xs px-2 py-1 rounded-full bg-gray-100 text-gray-700",
                                                                "{loc.shipping_method}"
                                                                if let Some(rate) = loc.flat_rate_usd {
                                                                    " (${rate:.2})"
                                                                }
                                                            }
                                                        }
                                                        div { class: "col-span-3 text-sm text-gray-600",
                                                            {loc.country.as_deref().unwrap_or("—")}
                                                        }
                                                        div { class: "col-span-3 flex gap-2 justify-end",
                                                            button {
                                                                class: "text-xs px-2 py-1 text-gray-700 border border-gray-300 rounded hover:bg-gray-50 transition-colors",
                                                                onclick: move |_| fill_form_for_edit(&loc_for_edit),
                                                                "Edit"
                                                            }
                                                            button {
                                                                class: format!("text-xs px-2 py-1 rounded transition-colors {}",
                                                                    if is_deleting { "bg-gray-200 text-gray-400 cursor-not-allowed" }
                                                                    else { "bg-red-50 text-red-600 border border-red-200 hover:bg-red-100" }
                                                                ),
                                                                disabled: is_deleting,
                                                                onclick: move |_| handle_delete(loc_id_del.clone()),
                                                                if is_deleting { "..." } else { "Delete" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => rsx! {
                                div { class: "p-6 text-center text-red-500 text-sm", "Error: {e}" }
                            },
                            None => rsx! {
                                div { class: "p-6 text-center text-gray-400 text-sm", "Loading..." }
                            },
                        }
                    }
                }
            }
        }
    }
}
