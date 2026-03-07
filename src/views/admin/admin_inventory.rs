#![allow(non_snake_case)]

use crate::Route;
use crate::backend::front_entities::*;
use crate::backend::server_functions;
use dioxus::prelude::*;

#[component]
pub fn AdminInventory() -> Element {
    let stock_items = use_resource(move || async move {
        tracing::info!("Getting stock items for admin inventory management");
        server_functions::admin_get_stock_items().await
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
                div { class: "flex gap-2",
                    Link {
                        to: Route::AdminStockLocations {},
                        button {
                            class: "text-sm border border-gray-300 px-3 py-2 text-gray-700 rounded hover:bg-gray-50 transition-colors",
                            "Manage Locations"
                        }
                    }
                    Link {
                        to: Route::AdminCreateStockItem {},
                        button {
                            class: "text-sm bg-gray-900 px-3 py-2 text-white rounded hover:bg-gray-800 transition-colors",
                            "Create Item"
                        }
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
                    div { class: "col-span-4", "Item" }
                    div { class: "col-span-4", "Stock (by location)" }
                    div { class: "col-span-2", "Total" }
                    div { class: "col-span-1", "" } // Actions
                }

                // Items list
                div {
                    class: "divide-y divide-gray-200",
                    {match &*stock_items.read() {
                        Some(Ok(stock_items)) => {
                            if stock_items.is_empty() {
                                rsx! {
                                    div {
                                        class: "p-8 text-center text-gray-500",
                                        "No inventory items created yet"
                                    }
                                }
                            } else {
                                rsx! {
                                    for stock_item in stock_items.iter() {
                                        StockItemRow {
                                            stock_item: stock_item.clone()
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            div {
                                class: "p-8 text-center text-red-500",
                                { format!("Error loading stock items: {:?}", e) }
                            }
                        },
                        None => rsx! {
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
}

#[component]
fn StockItemRow(props: StockItemRowProps) -> Element {
    let total_quantity: i32 = props
        .stock_item
        .location_quantities
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|lq| lq.quantity)
        .sum();

    let is_low_stock = props.stock_item.warning_quantity
        .map(|w| total_quantity < w)
        .unwrap_or(false);

    let border_class = if is_low_stock { "border-l-4 border-l-red-500" } else { "" };

    rsx! {
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
                                span { class: "text-gray-500 text-xs", "N/A" }
                            }
                        }
                    }
                }
            }

            // Item info
            div {
                class: "col-span-4",
                div {
                    class: "flex items-center gap-2 mb-1",
                    Link {
                        to: Route::AdminEditStockItem { id: format!("{}", props.stock_item.id) },
                        title: "Edit stock item",
                        class: "font-medium text-gray-900 hover:underline",
                        "{props.stock_item.pbi_sku}"
                    }
                    {
                        if is_low_stock {
                            rsx! {
                                span {
                                    class: "px-2 py-0.5 bg-red-700 text-white text-xs font-semibold rounded-full",
                                    "⚠ Low Stock"
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }
                div { class: "text-sm text-gray-700", "{props.stock_item.name}" }
                {
                    if let Some(desc) = &props.stock_item.description {
                        rsx! {
                            div { class: "text-xs text-gray-500 mt-1 truncate", "{desc}" }
                        }
                    } else {
                        rsx! {}
                    }
                }
            }

            // Location quantities
            div {
                class: "col-span-4 flex items-center",
                {
                    if let Some(loc_qtys) = &props.stock_item.location_quantities {
                        if loc_qtys.is_empty() {
                            rsx! { span { class: "text-gray-400 text-sm", "—" } }
                        } else {
                            rsx! {
                                div {
                                    class: "flex flex-wrap gap-1",
                                    for lq in loc_qtys.iter() {
                                        {
                                            let display_name = lq.stock_location_name
                                                .as_deref()
                                                .unwrap_or(&lq.stock_location_id)
                                                .to_string();
                                            let qty = lq.quantity;
                                            rsx! {
                                                span {
                                                    class: "text-xs px-2 py-0.5 bg-gray-100 rounded text-gray-700",
                                                    "{display_name}: {qty}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        rsx! { span { class: "text-gray-400 text-sm", "—" } }
                    }
                }
            }

            // Total
            div {
                class: "col-span-2 flex items-center",
                span {
                    class: if is_low_stock { "font-semibold text-red-600" } else { "font-semibold text-gray-900" },
                    "{total_quantity}"
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
    }
}
