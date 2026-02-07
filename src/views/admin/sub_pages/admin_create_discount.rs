#![allow(non_snake_case)]

use crate::Route;
use crate::backend::front_entities::*;
use crate::backend::server_functions::{
    CreateDiscountRequest, CreateDiscountResponse, admin_create_discount,
};
use crate::components::*;
use crate::utils::countries::*;
use chrono::NaiveDateTime;
use dioxus::prelude::*;
use strum::IntoEnumIterator;

#[component]
pub fn CDatePicker(
    label: String,
    value: Option<NaiveDateTime>,
    optional: bool,
    oninput: EventHandler<Option<NaiveDateTime>>,
) -> Element {
    let date_string = if let Some(date) = value {
        date.format("%Y-%m-%d").to_string()
    } else {
        String::new()
    };

    rsx! {
        div {
            class: "flex flex-col gap-1",
            label {
                class: format!("text-sm font-medium text-gray-700 {}",
                    if !optional { "after:content-['*'] after:text-red-500 after:ml-1" } else { "" }
                ),
                "{label}"
            }
            input {
                r#type: "date",
                class: "px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500 focus:border-blue-500",
                value: "{date_string}",
                oninput: move |evt: FormEvent| {
                    let value = evt.value();
                    if value.is_empty() {
                        oninput.call(None);
                    } else if let Ok(parsed_date) = chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
                        // Set time to end of day (23:59:59)
                        let datetime = parsed_date.and_hms_opt(23, 59, 59).unwrap_or_else(|| {
                            parsed_date.and_hms_opt(0, 0, 0).unwrap()
                        });
                        oninput.call(Some(datetime));
                    }
                }
            }
        }
    }
}

#[component]
pub fn AdminCreateDiscount() -> Element {
    // Basic discount info
    let mut code = use_signal(|| String::new());
    let mut discount_type = use_signal(|| DiscountType::Percentage);
    let mut discount_percentage = use_signal(|| Option::<f64>::None);
    let mut discount_amount = use_signal(|| Option::<f64>::None);
    let mut active = use_signal(|| true);

    // Usage limits
    let mut maximum_uses = use_signal(|| Option::<i32>::None);

    // Validity conditions
    let mut valid_countries = use_signal(|| Vec::<String>::new());
    let mut valid_after_x_products = use_signal(|| Option::<i32>::None);
    let mut valid_after_x_total = use_signal(|| Option::<f64>::None);
    let mut auto_apply = use_signal(|| false);
    let mut expire_at = use_signal(|| Option::<NaiveDateTime>::None);

    // UI states
    let mut creating = use_signal(|| false);

    // Create notification signals
    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new()); // "success" or "error"
    let mut show_notification = use_signal(|| false);

    let handle_create_discount = move |_| {
        spawn(async move {
            creating.set(true);

            // Validate required fields
            if code().trim().is_empty() {
                notification_message.set("Discount code is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                creating.set(false);
                return;
            }

            // Validate discount value based on type
            match discount_type() {
                DiscountType::Percentage | DiscountType::PercentageOnShipping => {
                    if discount_percentage().is_none()
                        || discount_percentage().unwrap_or(0.0) <= 0.0
                    {
                        notification_message.set(
                            "Discount percentage is required and must be greater than 0"
                                .to_string(),
                        );
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                        creating.set(false);
                        return;
                    }
                    if discount_percentage().unwrap_or(0.0) > 100.0 {
                        notification_message
                            .set("Discount percentage cannot exceed 100%".to_string());
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                        creating.set(false);
                        return;
                    }
                }
                DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => {
                    if discount_amount().is_none() || discount_amount().unwrap_or(0.0) <= 0.0 {
                        notification_message.set(
                            "Discount amount is required and must be greater than 0".to_string(),
                        );
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                        creating.set(false);
                        return;
                    }
                }
            }

            // Prepare request data
            let request = CreateDiscountRequest {
                code: code().to_uppercase(),
                discount_type: discount_type(),
                discount_percentage: discount_percentage(),
                discount_amount: discount_amount(),
                active: active(),
                maximum_uses: maximum_uses(),
                valid_countries: if valid_countries().is_empty() {
                    None
                } else {
                    Some(valid_countries())
                },
                valid_after_x_products: valid_after_x_products(),
                valid_after_x_total: valid_after_x_total(),
                auto_apply: auto_apply(),
                expire_at: expire_at(),
            };

            // Call server function
            match admin_create_discount(request).await {
                Ok(response) => {
                    if response.success {
                        notification_message.set("Discount created successfully!".to_string());
                        notification_type.set("success".to_string());
                        show_notification.set(true);

                        // Reset form
                        code.set(String::new());
                        discount_type.set(DiscountType::Percentage);
                        discount_percentage.set(None);
                        discount_amount.set(None);
                        active.set(true);
                        maximum_uses.set(None);
                        valid_countries.set(Vec::new());
                        valid_after_x_products.set(None);
                        valid_after_x_total.set(None);
                        auto_apply.set(false);
                        expire_at.set(None);
                    } else {
                        notification_message.set(response.message);
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error creating discount: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }

            creating.set(false);
        });
    };

    let handle_discount_type_change = move |evt: FormEvent| {
        if let Ok(new_type) = evt.value().parse::<DiscountType>() {
            discount_type.set(new_type);
            // Clear both percentage and amount when type changes
            discount_percentage.set(None);
            discount_amount.set(None);
        }
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
                "Create New Discount"
            }
            button {
                class: format!("text-sm px-3 py-2 text-white rounded transition-colors {}",
                    if creating() { "bg-gray-500 cursor-not-allowed" } else { "bg-gray-900 hover:bg-gray-800" }
                ),
                disabled: creating(),
                onclick: handle_create_discount,
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
                                label: "Discount Code",
                                value: "{code}",
                                placeholder: "SAVE10",
                                optional: false,
                                oninput: move |event: FormEvent| {
                                    code.set(event.value().to_uppercase());
                                }
                            }
                        },
                        div {
                            class: "w-full",
                            CSelectGroup {
                                label: "Discount Type",
                                optional: false,
                                oninput: handle_discount_type_change,
                                for dtype in DiscountType::iter() {
                                    CSelectItem {
                                        selected: if dtype == discount_type() { true } else { false },
                                        key: "{dtype:?}",
                                        value: "{dtype}",
                                        "{dtype.to_string()}"
                                    }
                                }
                            }
                        }
                    }

                    div {
                        class: "flex gap-4 w-full",
                        match discount_type() {
                            DiscountType::Percentage | DiscountType::PercentageOnShipping => rsx! {
                                div {
                                    class: "w-full",
                                    CTextBox {
                                        label: "Discount Percentage",
                                        value: if let Some(perc) = discount_percentage() {
                                            format!("{}", perc)
                                        } else {
                                            String::new()
                                        },
                                        placeholder: "10",
                                        suffix: "%",
                                        is_number: true,
                                        step: 0.01f64,
                                        optional: false,
                                        oninput: move |event: FormEvent| {
                                            if event.value().is_empty() {
                                                discount_percentage.set(None);
                                            } else if let Ok(perc) = event.value().parse::<f64>() {
                                                discount_percentage.set(Some(perc));
                                            }
                                        }
                                    }
                                }
                            },
                            DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => rsx! {
                                div {
                                    class: "w-full",
                                    CTextBox {
                                        label: "Discount Amount",
                                        value: if let Some(amount) = discount_amount() {
                                            format!("{}", amount)
                                        } else {
                                            String::new()
                                        },
                                        placeholder: "25",
                                        prefix: "$",
                                        is_number: true,
                                        step: 0.01f64,
                                        optional: false,
                                        oninput: move |event: FormEvent| {
                                            if event.value().is_empty() {
                                                discount_amount.set(None);
                                            } else if let Ok(amount) = event.value().parse::<f64>() {
                                                discount_amount.set(Some(amount));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },

                // Validity Conditions Section
                div {
                    class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Validity Conditions"
                    }
                    div {
                        class: "flex flex-col gap-4",

                        div {
                            class: "flex gap-4 w-full",
                            div {
                                class: "w-full",
                                CTextBox {
                                    label: "Valid After X Products",
                                    value: if let Some(products) = valid_after_x_products() {
                                        format!("{}", products)
                                    } else {
                                        String::new()
                                    },
                                    placeholder: "0",
                                    suffix: "items",
                                    is_number: true,
                                    step: 1f64,
                                    optional: true,
                                    oninput: move |event: FormEvent| {
                                        if event.value().is_empty() {
                                            valid_after_x_products.set(None);
                                        } else if let Ok(products) = event.value().parse::<i32>() {
                                            valid_after_x_products.set(Some(products));
                                        }
                                    }
                                }
                            },
                            div {
                                class: "w-full",
                                CTextBox {
                                    label: "Valid After Cart Total",
                                    value: if let Some(total) = valid_after_x_total() {
                                        format!("{}", total)
                                    } else {
                                        String::new()
                                    },
                                    placeholder: "0",
                                    prefix: "$",
                                    is_number: true,
                                    step: 0.01f64,
                                    optional: true,
                                    oninput: move |event: FormEvent| {
                                        if event.value().is_empty() {
                                            valid_after_x_total.set(None);
                                        } else if let Ok(total) = event.value().parse::<f64>() {
                                            valid_after_x_total.set(Some(total));
                                        }
                                    }
                                }
                            }
                        }

                        CDatePicker {
                            label: "Expiration Date".to_string(),
                            value: expire_at(),
                            optional: true,
                            oninput: move |date: Option<NaiveDateTime>| expire_at.set(date)
                        }

                        // Country Selection (Multi-select would be ideal, but using a simplified approach)
                        div {
                            class: "flex flex-col gap-2",
                            label {
                                class: "text-sm font-medium text-gray-700",
                                "Valid Countries (leave empty for all countries)"
                            }
                            CSelectGroup {
                                label: "Add Country",
                                optional: true,
                                oninput: move |event: FormEvent| {
                                    let country_code = event.value();
                                    if !country_code.is_empty() && !valid_countries().contains(&country_code) {
                                        let mut countries = valid_countries();
                                        countries.push(country_code);
                                        valid_countries.set(countries);
                                    }
                                },
                                CSelectItem {
                                    selected: true,
                                    value: "",
                                    "Select a country..."
                                }
                                for country_code in allowed_countries() {
                                    CSelectItem {
                                        selected: false,
                                        key: "{country_code}",
                                        value: "{country_code}",
                                        "{country_display_name_from_iso(&country_code)}"
                                    }
                                }
                            }

                            // Display selected countries
                            if !valid_countries().is_empty() {
                                div {
                                    class: "flex flex-wrap gap-2 mt-2",
                                    for country_code in valid_countries() {
                                        div {
                                            key: "{country_code}",
                                            class: "inline-flex items-center px-2 py-1 rounded-md bg-blue-100 text-blue-800 text-sm",
                                            span { "{country_display_name_from_iso(&country_code)}" }
                                            button {
                                                class: "ml-2 text-blue-600 hover:text-blue-800",
                                                onclick: {
                                                    let code = country_code.clone();
                                                    move |_| {
                                                        let mut countries = valid_countries();
                                                        countries.retain(|c| c != &code);
                                                        valid_countries.set(countries);
                                                    }
                                                },
                                                "×"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Right sidebar
            div {
                class: "md:w-[38%] w-full min-w-0",

                // Settings
                div {
                    class: "bg-white border flex flex-col gap-4 rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Settings"
                    }

                    div {
                        class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                        p {
                            class: "text-sm text-gray-700 pt-[2px]",
                            "Active"
                        }
                        CToggle {
                            checked: active(),
                            onclick: move |_| active.toggle()
                        }
                    }

                    div {
                        class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                        p {
                            class: "text-sm text-gray-700 pt-[2px]",
                            "Auto-apply discount"
                        }
                        CToggle {
                            checked: auto_apply(),
                            onclick: move |_| auto_apply.toggle()
                        }
                    }
                }

                // Usage Limits
                div {
                    class: "bg-white border rounded-md border-gray-200 p-4 min-h-36 mb-2",
                    h2 {
                        class: "text-lg font-medium mb-4",
                        "Usage Limits"
                    }
                    div {
                        class: "flex flex-col gap-4",

                        CTextBox {
                            label: "Maximum Uses",
                            value: if let Some(uses) = maximum_uses() {
                                format!("{}", uses)
                            } else {
                                String::new()
                            },
                            placeholder: "Unlimited",
                            suffix: "uses",
                            is_number: true,
                            step: 1f64,
                            optional: true,
                            oninput: move |event: FormEvent| {
                                if event.value().is_empty() {
                                    maximum_uses.set(None);
                                } else if let Ok(uses) = event.value().parse::<i32>() {
                                    maximum_uses.set(Some(uses));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
