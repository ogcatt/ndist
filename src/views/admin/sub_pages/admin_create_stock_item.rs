#![allow(non_snake_case)]

use dioxus::prelude::*;
use strum::IntoEnumIterator;
use crate::Route;
use crate::backend::server_functions::{
    admin_upload_private_thumbnails, UploadResponse, 
    admin_create_stock_item, admin_get_stock_items,
    CreateStockItemRequest, CreateStockItemResponse
};
use crate::backend::front_entities::*;
use crate::components::*;

#[component]
pub fn AdminCreateStockItem() -> Element {
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
    let mut creating = use_signal(|| false);

    // Create notification signals
    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new()); // "success" or "error"
    let mut show_notification = use_signal(|| false);

    let handle_create_stock_item = move |_| {
        spawn(async move {
            creating.set(true);
            
            // Validate required fields
            if name().trim().is_empty() {
                notification_message.set("Name is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                creating.set(false);
                return;
            }

            if pbi_sku().trim().is_empty() {
                notification_message.set("PBI SKU is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                creating.set(false);
                return;
            }

            // Prepare request data
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
                //assembled: assembled(),
            };

            // Call server function
            match admin_create_stock_item(request).await {
                Ok(response) => {
                    if response.success {
                        notification_message.set("Stock item created successfully!".to_string());
                        notification_type.set("success".to_string());
                        show_notification.set(true);
                        
                        // Reset form
                        name.set(String::new());
                        pbi_sku.set(String::new());
                        description.set(String::new());
                        thumbnail_ref.set(None);
                        unit.set(StockUnit::Multiples);
                        assembly_minutes.set(0);
                        default_shipping_days.set(0);
                        default_cost.set(0.0);
                        is_container.set(false);
                        //assembled.set(None);
                    } else {
                        notification_message.set(response.message);
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                },
                Err(e) => {
                    notification_message.set(format!("Error creating stock item: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }
            
            creating.set(false);
        });
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
                        let content_type = if file_name.ends_with(".jpg") || file_name.ends_with(".jpeg") {
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
                    
                    uploading.set(false);
                }
            }
        });
    };

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
                "Create New Stock Item"
            }
            button {
                class: format!("text-sm px-3 py-2 text-white rounded transition-colors {}",
                    if creating() { "bg-gray-500 cursor-not-allowed" } else { "bg-gray-900 hover:bg-gray-800" }
                ),
                disabled: creating(),
                onclick: handle_create_stock_item,
                if creating() {
                    "Creating..."
                } else {
                    "Create"
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
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Basic Information"
                    }
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
                                    // Ensure it starts with PBI if user doesn't type it
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

            // Right sidebar
            div {
                class: "md:w-[38%] w-full min-w-0",
                
                // Unit and Container Settings
                div {
                    class: "bg-white border flex flex-col gap-4 rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Settings"
                    }
                    
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
                        p {
                            class: "text-sm text-gray-700 pt-[2px]",
                            "Contains other items"
                        }
                        CToggle {
                            checked: is_container(),
                            onclick: move |_| is_container.toggle()
                        }
                    }

                    /*
                    if is_container() {
                        CSelectGroup {
                            label: "Assembled Status (DEPRECATE)",
                            optional: true,
                            oninput: move |event: FormEvent| {
                                match event.value().as_str() {
                                    "true" => assembled.set(Some(true)),
                                    "false" => assembled.set(Some(false)),
                                    _ => assembled.set(None),
                                }
                            },
                            CSelectItem {
                                selected: assembled().is_none(),
                                value: "",
                                "Not Set"
                            }
                            CSelectItem {
                                selected: assembled() == Some(true),
                                value: "true",
                                "Assembled"
                            }
                            CSelectItem {
                                selected: assembled() == Some(false),
                                value: "false",
                                "Not Assembled"
                            }
                        }
                    }
                    */
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
                }
            }
        }
    }
}