#![allow(non_snake_case)] // Allow non-snake_case identifiers

use chrono::{NaiveDateTime, Utc};
use dioxus::prelude::*;
use std::time::Duration;

use crate::Route;
use crate::backend::cache::use_cached_server;
use crate::backend::front_entities::*;
use crate::backend::server_functions;

// Helper function to check if a discount is valid
fn is_discount_valid(discount: &Discount) -> bool {
    // Must be active
    if !discount.active {
        return false;
    }

    // Check expiration date
    if let Some(expire_at) = discount.expire_at {
        let now = Utc::now().naive_utc();
        if expire_at < now {
            return false;
        }
    }

    // Check maximum uses
    if let Some(maximum_uses) = discount.maximum_uses {
        if discount.discount_used >= maximum_uses {
            return false;
        }
    }

    // Check fixed amount usage
    if matches!(
        discount.discount_type,
        DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping
    ) {
        if let (Some(discount_amount), Some(amount_used)) =
            (discount.discount_amount, discount.amount_used)
        {
            if amount_used >= discount_amount {
                return false;
            }
        }
    }

    true
}

// Helper function to format discount value display
fn format_discount_value(discount: &Discount) -> String {
    match discount.discount_type {
        DiscountType::Percentage => {
            if let Some(percentage) = discount.discount_percentage {
                format!("{}%", percentage)
            } else {
                "N/A".to_string()
            }
        }
        DiscountType::PercentageOnShipping => {
            if let Some(percentage) = discount.discount_percentage {
                format!("{}% (shipping)", percentage)
            } else {
                "N/A".to_string()
            }
        }
        DiscountType::FixedAmount => {
            if let Some(amount) = discount.discount_amount {
                format!("${:.2}", amount)
            } else {
                "N/A".to_string()
            }
        }
        DiscountType::FixedAmountOnShipping => {
            if let Some(amount) = discount.discount_amount {
                format!("${:.2} (shipping)", amount)
            } else {
                "N/A".to_string()
            }
        }
    }
}

#[component]
pub fn AdminDiscounts() -> Element {
    // Use our caching hook
    let discounts_req = use_cached_server(
        "discounts_list", // Unique key for this server function
        || server_functions::admin_get_discounts(),
        Duration::from_secs(15), // Cache for 15 seconds
    );

    use_effect(move || {
        println!("{:#?}", discounts_req);
    });

    rsx! {
        div {
            div {
                class: "border rounded-md border-gray-200 w-full mb-4 flex justify-between items-center p-2 pl-4",
                div {
                    class: "text-lg font-medium",
                    "Discount Codes"
                }
                Link {
                    to: Route::AdminCreateDiscount {},
                    button {
                        class: "text-sm bg-gray-900 px-3 py-2 text-white rounded hover:bg-gray-800 transition-colors",
                        "Create Discount"
                    }
                }
            }

            div {
                class: "w-full",
                {match &*discounts_req.read() {
                    Some(Ok(discounts)) => {
                        // Sort discounts alphabetically by code
                        let mut sorted_discounts = discounts.clone();
                        sorted_discounts.sort_by(|a, b| a.code.to_lowercase().cmp(&b.code.to_lowercase()));

                        rsx! {
                            if sorted_discounts.len() == 0 {
                                div {
                                    class: "mt-12 text-center",
                                    "No discount codes created yet"
                                }
                            } else {
                                for discount in sorted_discounts.iter() {
                                    {
                                        let is_valid = is_discount_valid(discount);
                                        let discount_value = format_discount_value(discount);
                                        let uses_display = if let Some(max_uses) = discount.maximum_uses {
                                            format!("{}/{}", discount.discount_used, max_uses)
                                        } else {
                                            format!("{}", discount.discount_used)
                                        };

                                        rsx! {
                                            div {
                                                class: "bg-white w-full min-h-12 border rounded-md border-gray-200 p-4 mb-4",
                                                div {
                                                    class: "flex items-center gap-4",
                                                    // Status indicator
                                                    div {
                                                        class: format!(
                                                            "w-1 h-12 rounded {}",
                                                            if is_valid { "bg-green-500" } else { "bg-red-500" }
                                                        )
                                                    }

                                                    div {
                                                        class: "flex-1",
                                                        div {
                                                            class: "flex items-center justify-between",
                                                            div {
                                                                class: "flex items-center gap-6",
                                                                // Discount Code
                                                                div {
                                                                    class: "min-w-0",
                                                                    h3 {
                                                                        class: "text-lg font-medium font-mono",
                                                                        "{discount.code.to_uppercase()}"
                                                                    }
                                                                    div {
                                                                        class: "text-sm text-gray-600",
                                                                        "{discount.discount_type}"
                                                                    }
                                                                }

                                                                // Discount Value
                                                                div {
                                                                    class: "text-center",
                                                                    div {
                                                                        class: "text-lg font-semibold text-green-600",
                                                                        "{discount_value}"
                                                                    }
                                                                    div {
                                                                        class: "text-xs text-gray-500",
                                                                        "Value"
                                                                    }
                                                                }

                                                                // Usage Stats
                                                                div {
                                                                    class: "text-center",
                                                                    div {
                                                                        class: "text-lg font-semibold",
                                                                        "{uses_display}"
                                                                    }
                                                                    div {
                                                                        class: "text-xs text-gray-500",
                                                                        "Uses"
                                                                    }
                                                                }

                                                                // Status badges
                                                                div {
                                                                    class: "flex flex-col gap-1",
                                                                    if !discount.active {
                                                                        span {
                                                                            class: "px-2 py-1 bg-red-100 text-red-800 rounded text-xs",
                                                                            "Inactive"
                                                                        }
                                                                    }
                                                                    if discount.auto_apply {
                                                                        span {
                                                                            class: "px-2 py-1 bg-blue-100 text-blue-800 rounded text-xs",
                                                                            "Auto-apply"
                                                                        }
                                                                    }
                                                                    if discount.expire_at.is_some() {
                                                                        span {
                                                                            class: "px-2 py-1 bg-yellow-100 text-yellow-800 rounded text-xs",
                                                                            "Expires"
                                                                        }
                                                                    }
                                                                }
                                                            }

                                                            // Edit link on the right side
                                                            Link {
                                                                to: Route::AdminEditDiscount { id: discount.id.clone() },
                                                                title: "Edit discount",
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
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(_)) => rsx! {
                        p { "Error loading discount codes" }
                    },
                    None => rsx! {
                        p { "Loading discount codes..." }
                    }
                }}
            }
        }
    }
}
