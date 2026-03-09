#![allow(non_snake_case)]

use crate::Route;
use crate::backend::front_entities::*;
use crate::backend::server_functions::{
    CreateDiscountRequest, DeleteDiscountRequest, GetDiscountRequest, GetDiscountResponse, UpdateDiscountRequest,
    admin_create_discount, admin_delete_discount, admin_get_discount, admin_update_discount,
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

#[derive(PartialEq, Props, Clone)]
pub struct AdminDiscountProps {
    pub id: Option<Signal<String>>,
}

#[component]
pub fn AdminDiscount(props: AdminDiscountProps) -> Element {
    let is_edit_mode = props.id.is_some();
    let mut discount_id = use_signal(|| String::new());
    let props_id = props.id;

    use_effect(move || {
        if let Some(id) = props_id {
            discount_id.set(id());
        }
    });

    let mut code = use_signal(|| String::new());
    let mut discount_type = use_signal(|| DiscountType::Percentage);
    let mut discount_percentage = use_signal(|| Option::<f64>::None);
    let mut discount_amount = use_signal(|| Option::<f64>::None);
    let mut active = use_signal(|| true);
    let mut maximum_uses = use_signal(|| Option::<i32>::None);
    let mut valid_countries = use_signal(|| Vec::<String>::new());
    let mut valid_after_x_products = use_signal(|| Option::<i32>::None);
    let mut valid_after_x_total = use_signal(|| Option::<f64>::None);
    let mut auto_apply = use_signal(|| false);
    let mut expire_at = use_signal(|| Option::<NaiveDateTime>::None);

    let mut saving = use_signal(|| false);
    let mut deleting = use_signal(|| false);
    let mut is_deleted = use_signal(|| false);
    let mut loaded = use_signal(|| false);

    let mut notification_message = use_signal(|| String::new());
    let mut notification_type = use_signal(|| String::new());
    let mut show_notification = use_signal(|| false);

    let discount_resource = use_resource(move || async move {
        if is_edit_mode && !discount_id().is_empty() {
            admin_get_discount(GetDiscountRequest { id: discount_id() }).await
        } else {
            Err(ServerFnError::new("Not in edit mode"))
        }
    });

    use_effect(move || {
        if !is_edit_mode {
            loaded.set(false);
            return;
        }

        if let Some(res) = discount_resource() {
            match res {
                Ok(resp) => {
                    if resp.success {
                        if let Some(d) = resp.discount.clone() {
                            if !*loaded.peek() {
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

    let handle_save_discount = move |_: Event<MouseData>| {
        let discount_id = discount_id();
        let code = code();
        let discount_type = discount_type();
        let discount_percentage = discount_percentage();
        let discount_amount = discount_amount();
        let active = active();
        let maximum_uses = maximum_uses();
        let valid_countries = valid_countries();
        let valid_after_x_products = valid_after_x_products();
        let valid_after_x_total = valid_after_x_total();
        let auto_apply = auto_apply();
        let expire_at = expire_at();
        let is_edit_mode = props.id.is_some();
        spawn(async move {
            saving.set(true);

            if code.trim().is_empty() {
                notification_message.set("Discount code is required".to_string());
                notification_type.set("error".to_string());
                show_notification.set(true);
                saving.set(false);
                return;
            }

            match discount_type {
                DiscountType::Percentage | DiscountType::PercentageOnShipping => {
                    if discount_percentage.is_none() || discount_percentage.unwrap_or(0.0) <= 0.0 {
                        notification_message.set(
                            "Discount percentage is required and must be greater than 0".to_string(),
                        );
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                        saving.set(false);
                        return;
                    }
                    if discount_percentage.unwrap_or(0.0) > 100.0 {
                        notification_message.set("Discount percentage cannot exceed 100%".to_string());
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                        saving.set(false);
                        return;
                    }
                }
                DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => {
                    if discount_amount.is_none() || discount_amount.unwrap_or(0.0) <= 0.0 {
                        notification_message.set(
                            "Discount amount is required and must be greater than 0".to_string(),
                        );
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                        saving.set(false);
                        return;
                    }
                }
            }

            let save_result = if is_edit_mode {
                admin_update_discount(UpdateDiscountRequest {
                    id: discount_id,
                    code: code.to_uppercase(),
                    discount_percentage,
                    discount_amount,
                    active,
                    maximum_uses,
                    valid_countries: if valid_countries.is_empty() { None } else { Some(valid_countries) },
                    valid_after_x_products,
                    valid_after_x_total,
                    auto_apply,
                    expire_at,
                }).await.map(|r| r.success)
            } else {
                admin_create_discount(CreateDiscountRequest {
                    code: code.to_uppercase(),
                    discount_type,
                    discount_percentage,
                    discount_amount,
                    active,
                    maximum_uses,
                    valid_countries: if valid_countries.is_empty() { None } else { Some(valid_countries) },
                    valid_after_x_products,
                    valid_after_x_total,
                    auto_apply,
                    expire_at,
                }).await.map(|r| r.success)
            };

            match save_result {
                Ok(success) => {
                    if success {
                        notification_message.set(
                            if is_edit_mode {
                                "Discount updated successfully!".to_string()
                            } else {
                                "Discount created successfully!".to_string()
                            }
                        );
                        notification_type.set("success".to_string());
                        show_notification.set(true);
                    } else {
                        notification_message.set("Operation failed".to_string());
                        notification_type.set("error".to_string());
                        show_notification.set(true);
                    }
                }
                Err(e) => {
                    notification_message.set(format!("Error saving discount: {}", e));
                    notification_type.set("error".to_string());
                    show_notification.set(true);
                }
            }

            saving.set(false);
        });
    };

    let handle_delete_discount = move |_: Event<MouseData>| {
        let discount_id = discount_id();
        spawn(async move {
            deleting.set(true);

            match admin_delete_discount(DeleteDiscountRequest { id: discount_id }).await {
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
    };

    let handle_discount_type_change = move |evt: FormEvent| {
        if let Ok(new_type) = evt.value().parse::<DiscountType>() {
            discount_type.set(new_type);
            discount_percentage.set(None);
            discount_amount.set(None);
        }
    };

    if is_edit_mode && !loaded() {
        if discount_resource().is_some_and(|r| r.is_err())
            || discount_resource().is_some_and(|r| r.as_ref().is_ok_and(|resp| !resp.success))
        {
            return rsx! {
                div { class: "text-red-500", "Error loading discount." }
            };
        } else {
            return rsx! { div { "Loading..." } };
        }
    }

    rsx! {
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
            div { class: "text-green-500 text-lg font-medium", "Discount deleted successfully." }
        } else {
            div {
                class: "border rounded-md border-gray-200 w-full mb-4 flex justify-between items-center p-2 pl-4",
                div {
                    class: "text-lg font-medium",
                    if is_edit_mode { "Edit Discount" } else { "Create New Discount" }
                }
                div {
                    class: "flex items-center gap-2",
                    if is_edit_mode {
                        button {
                            class: format!("text-red-500 hover:text-red-700 {}",
                                if deleting() { "cursor-not-allowed opacity-50" } else { "" }
                            ),
                            disabled: deleting(),
                            onclick: handle_delete_discount,
                            svg { xmlns: "http://www.w3.org/2000/svg", width: "20", height: "20", fill: "none",
                                path { stroke: "currentColor", stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "1.5", d: "m12.283 7.5-.288 7.5m-3.99 0-.288-7.5m8.306-2.675c.285.043.569.09.852.138m-.852-.137-.89 11.568a1.875 1.875 0 0 1-1.87 1.73H6.737a1.875 1.875 0 0 1-1.87-1.73l-.89-11.569m12.046 0a40.08 40.08 0 0 0-2.898-.33m-10 .467c.283-.049.567-.095.852-.137m0 0a40.091 40.09 0 0 1 2.898-.33m6.25 0V3.73c0-.984-.758-1.804-1.742-1.834a43.3 43.3 0 0 0-2.766 0c-.984.03-1.742.851-1.742 1.834v.763m6.25 0c-2.08-.16-4.17-.16-6.25 0" }
                            }
                        }
                    }
                    button {
                        class: format!("text-sm px-3 py-2 text-white rounded transition-colors {}",
                            if saving() { "bg-gray-500 cursor-not-allowed" } else {
                                if is_edit_mode { "bg-zinc-600 hover:bg-zinc-500" } else { "bg-zinc-600 hover:bg-zinc-500" }
                            }
                        ),
                        disabled: saving(),
                        onclick: handle_save_discount,
                        if saving() {
                            if is_edit_mode { "Updating..." } else { "Creating..." }
                        } else {
                            if is_edit_mode { "Update" } else { "Create" }
                        }
                    }
                }
            }

            div {
                class: "flex flex-col md:flex-row w-full gap-2",
                div {
                    class: "flex w-full flex-col gap-2",
                    div {
                        class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                        h2 { class: "text-lg font-medium mb-4", "Basic Information" }
                        div {
                            class: "flex gap-4 w-full mb-4",
                            div { class: "w-full",
                                CTextBox {
                                    label: "Discount Code",
                                    value: "{code}",
                                    placeholder: "SAVE10",
                                    optional: false,
                                    oninput: move |event: FormEvent| code.set(event.value().to_uppercase())
                                }
                            },
                            div { class: "w-full",
                                if is_edit_mode {
                                    div {
                                        class: "w-full px-3 py-2 border border-gray-200 rounded-md text-gray-700",
                                        label { class: "text-sm font-medium text-gray-700 block mb-1", "Discount Type" },
                                        "{discount_type().to_string()}"
                                    }
                                } else {
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
                        }
                        div {
                            class: "flex gap-4 w-full",
                            match discount_type() {
                                DiscountType::Percentage | DiscountType::PercentageOnShipping => rsx! {
                                    div { class: "w-full",
                                        CTextBox {
                                            label: "Discount Percentage",
                                            value: if let Some(perc) = discount_percentage() { format!("{}", perc) } else { String::new() },
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
                                    div { class: "w-full",
                                        CTextBox {
                                            label: "Discount Amount",
                                            value: if let Some(amount) = discount_amount() { format!("{}", amount) } else { String::new() },
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
                    div {
                        class: "flex-grow bg-white border rounded-md border-gray-200 p-4 min-h-36",
                        h2 { class: "text-lg font-medium mb-4", "Validity Conditions" }
                        div { class: "flex flex-col gap-4",
                            div {
                                class: "flex gap-4 w-full",
                                div { class: "w-full",
                                    CTextBox {
                                        label: "Valid After X Products",
                                        value: if let Some(products) = valid_after_x_products() { format!("{}", products) } else { String::new() },
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
                                div { class: "w-full",
                                    CTextBox {
                                        label: "Valid After Cart Total",
                                        value: if let Some(total) = valid_after_x_total() { format!("{}", total) } else { String::new() },
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
                            div {
                                class: "flex flex-col gap-2",
                                label { class: "text-sm font-medium text-gray-700", "Valid Countries (leave empty for all countries)" }
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
                                    CSelectItem { selected: true, value: "", "Select a country..." }
                                    for country_code in allowed_countries() {
                                        CSelectItem {
                                            selected: false,
                                            key: "{country_code}",
                                            value: "{country_code}",
                                            "{country_display_name_from_iso(&country_code)}"
                                        }
                                    }
                                }
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
                div { class: "md:w-[38%] w-full min-w-0",
                    div {
                        class: "bg-white border flex flex-col gap-4 rounded-md border-gray-200 p-4 min-h-36 mb-2",
                        h2 { class: "text-lg font-medium mb-4", "Settings" }
                        div {
                            class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                            p { class: "text-sm text-gray-700 pt-[2px]", "Active" }
                            CToggle { checked: active(), onclick: move |_| active.toggle() }
                        }
                        div {
                            class: "w-full flex justify-between px-3.5 py-2 border border-gray-200 rounded-md",
                            p { class: "text-sm text-gray-700 pt-[2px]", "Auto-apply discount" }
                            CToggle { checked: auto_apply(), onclick: move |_| auto_apply.toggle() }
                        }
                    }
                    div {
                        class: "bg-white border rounded-md border-gray-200 p-4 min-h-36 mb-2",
                        h2 { class: "text-lg font-medium mb-4", "Usage Limits" }
                        div { class: "flex flex-col gap-4",
                            CTextBox {
                                label: "Maximum Uses",
                                value: if let Some(uses) = maximum_uses() { format!("{}", uses) } else { String::new() },
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

#[component]
pub fn AdminCreateDiscount() -> Element {
    AdminDiscount(AdminDiscountProps { id: None })
}

#[component]
pub fn AdminEditDiscount(id: String) -> Element {
    let id_signal = use_signal(|| id);
    AdminDiscount(AdminDiscountProps { id: Some(id_signal) })
}
