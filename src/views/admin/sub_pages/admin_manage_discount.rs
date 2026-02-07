#![allow(non_snake_case)]
use crate::Route;
use crate::backend::front_entities::*;
use crate::backend::server_functions::{
    DeleteDiscountRequest, DeleteDiscountResponse, GetDiscountRequest, GetDiscountResponse,
    UpdateDiscountRequest, UpdateDiscountResponse, admin_delete_discount, admin_get_discount,
    admin_update_discount,
};
use crate::components::*;
use crate::utils::countries::*;
use chrono::NaiveDateTime;
use dioxus::prelude::*;
use strum::IntoEnumIterator;

#[component]
pub fn AdminEditDiscount(id: String) -> Element {
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
    let mut updating = use_signal(|| false);
    let mut deleting = use_signal(|| false);
    let mut is_deleted = use_signal(|| false);
    let mut loaded = use_signal(|| false);
    // Create notification signals
    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new()); // "success" or "error"
    let mut show_notification = use_signal(|| false);

    let discount_resource = use_resource({
        let id = id.clone();
        move || {
            let id_clone = id.clone();
            async move { admin_get_discount(GetDiscountRequest { id: id_clone }).await }
        }
    });

    use_effect(move || {
        if let Some(res) = discount_resource() {
            match res {
                Ok(resp) => {
                    if resp.success {
                        if let Some(d) = resp.discount.clone() {
                            if !*loaded.peek() {
                                // Changed from !loaded.peek() to !*loaded.peek()
                                code.set(d.code);
                                discount_type.set(d.discount_type);
                                discount_percentage.set(d.discount_percentage);
                                discount_amount.set(d.discount_amount);
                                active.set(d.active);
                                maximum_uses.set(d.maximum_uses);
                                valid_countries.set(d.valid_countries.unwrap_or_default());
                                valid_after_x_products.set(d.valid_after_x_products);
                                valid_after_x_total.set(d.valid_after_x_total);
                                auto_apply.set(d.auto_apply);
                                expire_at.set(d.expire_at);
                                loaded.set(true);
                            }
                        } else {
                            notification_message.set("Discount not found".to_string());
                            notification_type.set("error".to_string());
                            show_notification.set(true);
                        }
                    } else {
                        notification_message.set(resp.message.clone());
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error loading discount: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }
        }
    });

    let handle_update_discount = {
        let id = id.clone();
        move |_| {
            let id_clone = id.clone(); // Clone here to avoid moving the captured id
            spawn(async move {
                updating.set(true);
                // Validate required fields
                if code().trim().is_empty() {
                    notification_message.set("Discount code is required".to_string());
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                    updating.set(false);
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
                            updating.set(false);
                            return;
                        }
                        if discount_percentage().unwrap_or(0.0) > 100.0 {
                            notification_message
                                .set("Discount percentage cannot exceed 100%".to_string());
                            notification_type.set("error".to_string());
                            show_notification.set(true);
                            updating.set(false);
                            return;
                        }
                    }
                    DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => {
                        if discount_amount().is_none() || discount_amount().unwrap_or(0.0) <= 0.0 {
                            notification_message.set(
                                "Discount amount is required and must be greater than 0"
                                    .to_string(),
                            );
                            notification_type.set("error".to_string());
                            show_notification.set(true);
                            updating.set(false);
                            return;
                        }
                    }
                }
                // Prepare request data
                let request = UpdateDiscountRequest {
                    id: id_clone, // Use the clone (no further .clone() needed)
                    code: code().to_uppercase(),
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
                match admin_update_discount(request).await {
                    Ok(response) => {
                        if response.success {
                            notification_message.set("Discount updated successfully!".to_string());
                            notification_type.set("success".to_string());
                            show_notification.set(true);
                        } else {
                            notification_message.set(response.message);
                            notification_type.set("error".to_string());
                            show_notification.set(true);
                        }
                    }
                    Err(e) => {
                        notification_message.set(format!("Error updating discount: {}", e));
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                updating.set(false);
            });
        }
    };

    let handle_delete_discount = {
        let id = id.clone();
        move |_| {
            let id_clone = id.clone(); // Clone here to avoid moving the captured id
            spawn(async move {
                deleting.set(true);
                // Call server function
                match admin_delete_discount(DeleteDiscountRequest { id: id_clone }).await {
                    // Use the clone (no further .clone() needed)
                    Ok(response) => {
                        if response.success {
                            notification_message.set("Discount deleted successfully!".to_string());
                            notification_type.set("success".to_string());
                            show_notification.set(true);
                            is_deleted.set(true);
                        } else {
                            notification_message.set(response.message);
                            notification_type.set("error".to_string());
                            show_notification.set(true);
                        }
                    }
                    Err(e) => {
                        notification_message.set(format!("Error deleting discount: {}", e));
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                deleting.set(false);
            });
        }
    };

    if !loaded() {
        if discount_resource().is_some_and(|r| r.is_err())
            || (discount_resource().is_some_and(|r| r.is_ok_and(|resp| !resp.success)))
        {
            return rsx! {
                div {
                    class: "text-red-500",
                    "Error loading discount."
                }
            };
        } else {
            return rsx! {
                div {
                    "Loading..."
                }
            };
        }
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
        if is_deleted() {
            div {
                class: "text-green-500 text-lg font-medium",
                "Discount deleted successfully."
            }
        } else {
            div {
                class: "border rounded-md border-gray-200 w-full mb-4 flex justify-between items-center p-2 pl-4",
                div {
                    class: "text-lg font-medium",
                    "Edit Discount"
                }
                div {
                    class: "flex items-center gap-2",
                    button {
                        class: format!("text-red-500 hover:text-red-700 {}",
                            if deleting() { "cursor-not-allowed opacity-50" } else { "" }
                        ),
                        disabled: deleting(),
                        onclick: handle_delete_discount,
                        svg { xmlns: "http://www.w3.org/2000/svg", width: "20", height: "20", fill: "none",
                            path { stroke: "currentColor", stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "1.5", d: "m12.283 7.5-.288 7.5m-3.99 0-.288-7.5m8.306-2.675c.285.043.569.09.852.138m-.852-.137-.89 11.568a1.875 1.875 0 0 1-1.87 1.73H6.737a1.875 1.875 0 0 1-1.87-1.73l-.89-11.569m12.046 0a40.08 40.08 0 0 0-2.898-.33m-10 .467c.283-.049.567-.095.852-.137m0 0a40.091 40.091 0 0 1 2.898-.33m6.25 0V3.73c0-.984-.758-1.804-1.742-1.834a43.3 43.3 0 0 0-2.766 0c-.984.03-1.742.851-1.742 1.834v.763m6.25 0c-2.08-.160-4.17-.160-6.25 0" }
                        }
                    }
                    button {
                        class: format!("text-sm px-3 py-2 text-white rounded transition-colors {}",
                            if updating() { "bg-gray-500 cursor-not-allowed" } else { "bg-gray-900 hover:bg-gray-800" }
                        ),
                        disabled: updating(),
                        onclick: handle_update_discount,
                        if updating() {
                            "Updating..."
                        } else {
                            "Update"
                        }
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
                                label {
                                    class: "text-sm font-medium text-gray-700 block mb-1",
                                    "Discount Type"
                                }
                                div {
                                    class: "w-full px-3 py-2 border border-gray-200 rounded-md text-gray-700",
                                    "{discount_type().to_string()}"
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
}
