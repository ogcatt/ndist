use crate::Route;
use dioxus::prelude::*;
use dioxus_i18n::t;
use lazy_regex::regex;
use std::time::Duration;

// If you keep these components in your crate:
use crate::components::{CSelectGroup, CSelectItem, CTextBox, ShippingMap};

use crate::backend::cache::use_stale_while_revalidate_with_callback;
use crate::backend::front_entities::*;
use crate::backend::payments;
use crate::backend::server_functions;
use crate::utils::GLOBAL_CART;
use crate::utils::countries::*;
use crate::utils::*;

use crate::components::SmilesViewer;

// Helper: currency symbol placeholder
fn currency_code_symbol(_code: Option<&str>) -> &'static str {
    "$"
}

fn format_number(v: f64) -> String {
    format!("{:.2}", v)
}

fn find_product_variant<'a>(
    products: &'a Vec<Product>,
    variant_id: &str,
) -> Option<(&'a Product, &'a ProductVariants)> {
    for p in products {
        if let Some(ref vars) = p.variants {
            if let Some(v) = vars.iter().find(|v| v.id == variant_id) {
                return Some((p, v));
            }
        }
    }
    None
}

// Safe label for ProductForm using its Display implementation
fn product_form_label(form: &ProductForm) -> Option<String> {
    // If you want to hide "Other", return None for it, else show all:
    match form {
        ProductForm::Other => None,
        _ => Some(form.to_string()),
    }
}

// Join basket item to product + variant for display
#[derive(Clone, PartialEq)]
struct CartLine {
    product: Product,
    variant: ProductVariants,
    basket_item_id: String,
    quantity: i32,
}

#[component]
pub fn Checkout() -> Element {
    // Form state
    let mut email = use_signal(|| String::new());
    let mut phone = use_signal(|| String::new());
    let mut email_list = use_signal(|| true);

    //let mut country = use_signal(|| String::new());
    let mut first_name = use_signal(|| String::new());
    let mut last_name = use_signal(|| String::new());
    let mut company = use_signal(|| String::new());
    let mut address_line_1 = use_signal(|| String::new());
    let mut address_line_2 = use_signal(|| String::new());
    let mut post_code = use_signal(|| String::new());
    let mut province = use_signal(|| String::new());
    let mut city = use_signal(|| String::new());

    let mut loading_option_signal = use_signal(|| None::<ShippingOption>);
    let mut email_error = use_signal(|| None::<String>);
    let mut phone_error = use_signal(|| None::<String>);

    let mut payment_agreed = use_signal(|| false);
    let mut payment_loading = use_signal(|| false);
    let mut payment_error = use_signal(|| None::<String>);

    // Auth check: user must be signed in to access checkout
    let mut auth_loading = use_signal(|| true);
    let mut email_locked = use_signal(|| false);

    let session_resource = use_resource(move || async move {
        server_functions::get_session_info().await
    });

    use_effect(move || {
        if let Some(Ok(info)) = session_resource.read().as_ref() {
            auth_loading.set(false);
            if !info.authenticated {
                spawn(async move {
                    gloo_timers::future::TimeoutFuture::new(100).await;
                    let _ = web_sys::window()
                        .unwrap()
                        .location()
                        .set_href("/");
                });
            } else {
                email.set(info.email.clone());
                email_locked.set(true);
            }
        }
    });

    // Country list from allowed list helper
    let countries = available_countries_display();

    // Use the stale-while-revalidate hook for basket data
    let basket_signal = use_stale_while_revalidate_with_callback(
        "get_basket",
        || server_functions::get_basket(),
        Duration::from_secs(180),
        |basket| {
            GLOBAL_CART.with_mut(|c| *c = Some(basket.clone()));
        },
    );

    let navigator = use_navigator();

    // Check for open payment, redirect if found
    use_effect(move || {
        let basket_ref = basket_signal().clone();

        if let Some(basket) = basket_ref {
            if let Some(payment_id) = basket.payment_id {
                navigator.push(Route::CheckoutPayment {
                    payment_id: payment_id,
                });
            }
        }
    });

    // Add this after basket_signal:
    let current_country = use_memo({
        let basket_signal = basket_signal.clone();
        move || {
            basket_signal()
                .and_then(|basket| basket.country_code.clone())
                .unwrap_or_default()
        }
    });

    let info_passes = use_memo({
        let email = email.clone();
        let current_country = current_country.clone();
        let first_name = first_name.clone();
        let last_name = last_name.clone();
        let address_line_1 = address_line_1.clone();
        let post_code = post_code.clone();
        let city = city.clone();
        move || -> bool {
            !email().trim().is_empty()
                && !current_country.read().trim().is_empty()
                && !first_name().trim().is_empty()
                && !last_name().trim().is_empty()
                && !address_line_1().trim().is_empty()
                && !post_code().trim().is_empty()
                && !city().trim().is_empty()
        }
    });

    let form_validation_passes = use_memo({
        let mut email = email.clone();
        let mut phone = phone.clone();
        let mut basket_signal = basket_signal.clone();
        let mut email_error = email_error.clone();
        let mut phone_error = phone_error.clone();

        move || -> bool {
            let mut is_valid = true;

            // Email validation
            let email_regex = regex!(r"^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$");
            if email().trim().is_empty() || !email_regex.is_match(&email().to_uppercase()) {
                email_error.set(Some(t!("email-invalid")));
                is_valid = false;
            } else {
                email_error.set(None);
            }

            // Phone validation
            let phone_regex =
                regex!(r"^\+?(\d{1,3})?[-.\s]?(\(?\d{3}\)?[-.\s]?)?(\d[-.\s]?){6,9}\d$");
            let mut phone_value = phone().clone();
            phone_value = phone_value.trim().to_string();
            let countries_requiring_phone: Vec<&str> = vec![]; // Empty for now as requested

            let phone_required = if let Some(basket) = basket_signal() {
                // Required for Express shipping
                basket.shipping_option == Some(ShippingOption::Express) ||
                // Required for specific countries (empty vec for now)
                basket.country_code.as_ref().map_or(false, |country| countries_requiring_phone.contains(&country.as_str()))
            } else {
                false
            };

            if phone_required && phone_value.is_empty() {
                if basket_signal().and_then(|b| b.shipping_option) == Some(ShippingOption::Express)
                {
                    phone_error.set(Some(t!("phone-required-express")));
                } else {
                    phone_error.set(Some(t!("phone-required-country")));
                }
                is_valid = false;
            } else if !phone_value.is_empty() && !phone_regex.is_match(&phone_value) {
                phone_error.set(Some(t!("phone-invalid")));
                is_valid = false;
            } else {
                phone_error.set(None);
            }

            is_valid
        }
    });

    let update_country = {
        let mut basket_res = basket_signal.clone();
        move |country_code: String| async move {
            match server_functions::update_basket_country(country_code).await {
                Ok(updated_basket) => {
                    basket_res.set(Some(updated_basket.clone()));
                    GLOBAL_CART.with_mut(|c| *c = Some(updated_basket.clone()));
                }
                Err(e) => {
                    tracing::error!("Failed to update basket country: {:?}", e);
                }
            }
        }
    };

    // Use regular resource for products (you can change this to use_stale_while_revalidate too if needed)
    let mut products_res = use_resource(|| async move { server_functions::get_products().await });

    // Join to cart lines
    let mut cart_lines = use_memo({
        let basket_signal = basket_signal.clone();
        let products_res = products_res.clone();
        move || -> Vec<CartLine> {
            let mut lines = Vec::new();

            // Get basket data from signal (it's Option<CustomerBasket>)
            let basket_opt = basket_signal();

            // Get products data from resource (it's Option<Result<Vec<Product>, ServerFnError>>)
            let products_ref = products_res.read();

            let (Some(basket), Some(products_res)) = (basket_opt.as_ref(), products_ref.as_ref())
            else {
                return lines;
            };

            let Ok(products) = products_res else {
                return lines;
            };

            let items = basket.items.as_ref().map(|v| v.as_slice()).unwrap_or(&[]);
            for bi in items {
                if let Some((product, variant)) =
                    find_product_variant(products, &bi.product_variant_id)
                {
                    lines.push(CartLine {
                        product: product.clone(),
                        variant: variant.clone(),
                        basket_item_id: bi.id.clone(),
                        quantity: bi.quantity,
                    });
                }
            }

            // Sort like Svelte by product_title + variant_title
            lines.sort_by(|a, b| {
                let aa = format!("{}{}", a.product.title, a.variant.variant_name);
                let bb = format!("{}{}", b.product.title, b.variant.variant_name);
                aa.to_lowercase().cmp(&bb.to_lowercase())
            });

            lines
        }
    });

    // Totals from joined lines (placeholder shipping/tax as in Svelte)
    let currency_code: Option<String> = None;

    let mut shipping_discount_off: Signal<Option<f64>> = use_signal(|| None);
    let mut discount_off: Signal<Option<f64>> = use_signal(|| None);

    // 2. Update your shipping calculation to include discount logic:
    // Replace your existing shipping logic in the totals section with this:

    let mut shipping_cost = use_memo({
        let basket = basket_signal.clone();
        move || -> f64 {
            let mut shipping_calc = if let Some(basket) = basket() {
                if let Some(results) = basket.shipping_results
                    && basket.shipping_option.is_some()
                {
                    results
                        .into_iter()
                        .find(|so| so.option == basket.shipping_option.unwrap())
                        .map(|to| to.cost_usd)
                        .unwrap_or(0.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };

            if let Some(basket) = basket() {
                if let Some(discount_data) = basket.discount {
                    match discount_data.discount_type {
                        DiscountType::PercentageOnShipping => {
                            let shipping_off_pre = (shipping_calc
                                * (discount_data
                                    .discount_percentage
                                    .expect("Could not match percentage shipping type")
                                    / 100.0));
                            shipping_calc = shipping_calc - shipping_off_pre;
                            shipping_discount_off.set(Some(shipping_off_pre));
                        }
                        DiscountType::FixedAmountOnShipping => {
                            let shipping_off_pre = discount_data
                                .discount_amount_left
                                .map(|amount| amount.min(shipping_calc))
                                .expect("Could not match amount left shipping type");

                            shipping_calc = shipping_calc - shipping_off_pre;
                            shipping_discount_off.set(Some(shipping_off_pre));
                        }
                        _ => {}
                    }
                }
            }

            shipping_calc
        }
    });

    let mut subtotal = use_memo({
        let cart_lines = cart_lines.clone();
        move || -> f64 {
            cart_lines
                .read()
                .iter()
                .map(|l| l.variant.price_standard_usd * (l.quantity as f64))
                .sum()
        }
    });
    let mut taxes = use_memo({ move || -> f64 { 0.0 } });

    let mut total = use_memo({
        let basket = basket_signal.clone();
        let subtotal = subtotal.clone();
        let taxes = taxes.clone();
        move || {
            if basket().is_none() {
                return 0.0;
            }
            let mut s = *subtotal.read();
            let t = *taxes.read();

            if let Some(discount_data) = basket().unwrap().discount {
                match discount_data.discount_type {
                    DiscountType::Percentage => {
                        let discount_off_pre = (s
                            * (discount_data
                                .discount_percentage
                                .expect("Could not match percentage type")
                                / 100.0));
                        s = s - discount_off_pre;
                        discount_off.set(Some(discount_off_pre));
                    }
                    DiscountType::FixedAmount => {
                        let discount_off_pre = discount_data
                            .discount_amount_left
                            .map(|amount| amount.min(s))
                            .expect("Could not match amount left type");

                        s = s - discount_off_pre;
                        discount_off.set(Some(discount_off_pre));
                    }
                    _ => {}
                }
            }

            s + t
        }
    });

    let handle_payment_initiation = {
        let email = email.clone();
        let phone = phone.clone();
        let email_list = email_list.clone();
        let current_country = current_country.clone();
        let first_name = first_name.clone();
        let last_name = last_name.clone();
        let company = company.clone();
        let address_line_1 = address_line_1.clone();
        let address_line_2 = address_line_2.clone();
        let post_code = post_code.clone();
        let province = province.clone();
        let city = city.clone();
        let mut payment_loading = payment_loading.clone();
        let mut payment_error = payment_error.clone();
        let navigator = use_navigator();

        move |_: MouseEvent| {
            let email_val = email();
            let phone_val = phone();
            let email_list_val = email_list();
            let current_country_val = current_country.read().clone();
            let first_name_val = first_name();
            let last_name_val = last_name();
            let company_val = company();
            let address_line_1_val = address_line_1();
            let address_line_2_val = address_line_2();
            let post_code_val = post_code();
            let province_val = province();
            let city_val = city();
            let mut payment_loading = payment_loading.clone();
            let mut payment_error = payment_error.clone();
            let navigator = navigator.clone();

            spawn(async move {
                payment_loading.set(true);
                payment_error.set(None);

                // Prepare the request
                let shipping_info = CustomerShippingInfo {
                    phone: if phone_val.trim().is_empty() {
                        None
                    } else {
                        Some(phone_val)
                    },
                    email_list: email_list_val,
                    first_name: first_name_val,
                    last_name: last_name_val,
                    company: if company_val.trim().is_empty() {
                        None
                    } else {
                        Some(company_val)
                    },
                    address_line_1: address_line_1_val,
                    address_line_2: if address_line_2_val.trim().is_empty() {
                        None
                    } else {
                        Some(address_line_2_val)
                    },
                    post_code: post_code_val,
                    province: if province_val.trim().is_empty() {
                        None
                    } else {
                        Some(province_val)
                    },
                    city: city_val,
                    country: if current_country_val.trim().is_empty() {
                        None
                    } else {
                        Some(current_country_val)
                    },
                };

                let request = payments::InitPaymentOrderRequest {
                    email: email_val,
                    shipping_info,
                };

                // Call the server function
                match payments::init_payment_and_order(request).await {
                    Ok(payment_id) => {
                        // Success - redirect to checkout payment page
                        navigator.push(Route::CheckoutPayment {
                            payment_id: payment_id,
                        });
                    }
                    Err(e) => {
                        // Error - show error message
                        payment_error.set(Some(format!("Payment initialization failed: {}", e)));
                        payment_loading.set(false);
                    }
                }
            });
        }
    };

    // Loading determination for the right panel
    let loading = {
        // basket_signal returns Option<CustomerBasket>, so None means loading
        let basket_loaded = basket_signal().is_some();

        // products_res is still a Resource, so check if it's loaded and successful
        let products_loaded = {
            let p = products_res.read();
            matches!(p.as_ref(), Some(Ok(_)))
        };

        !(basket_loaded && products_loaded)
    };

    // Helper to render the quantity bubble (no let bindings in attributes)
    fn qty_bubble(qty: i32) -> Element {
        let (class, left) = if qty < 10 {
            (
                "hover:bg-gray-700 duration-500 cursor-default ml-9 mt-[-4px] w-[18px] h-[18px] absolute z-10 rounded-full bg-gray-800 text-smm text-white justify-center text-center",
                "left-[5.5px]",
            )
        } else {
            (
                "hover:bg-gray-700 duration-500 cursor-default ml-8 mt-[-4px] w-[24px] h-[18px] absolute z-10 rounded-full bg-gray-800 text-smm text-white justify-center text-center",
                "left-[4.5px]",
            )
        };
        rsx! {
            div {
                title: t!("x-quantity-of-this-variant", qty: qty),
                class: "{class}",
                div { class: "absolute {left} top-[-1px]", "{qty}" }
            }
        }
    }

    rsx! {
        document::Title { { format!("{} - {}", t!("brand"), t!("checkout") ) } }
        document::Script {
            src: "https://challenges.cloudflare.com/turnstile/v0/api.js"
        }

        if auth_loading() {
            div { class: "max-w-[1100px] w-full mx-auto px-5 md:px-6 pt-20 flex justify-center",
                div { class: "text-gray-500 text-base", "Loading..." }
            }
        }

        if !auth_loading() {
        div { class: "max-w-[1100px] w-full mx-auto px-5 md:px-6 pt-6 md:pt-10 pb-8 md:pb-16",
            div { class: "lg:flex block",
                // Left column (63%)
                div { class: "lg:w-[63%] w-full md:mt-8",
                    // Contact Details header
                    div { class: "flex justify-start mb-6",
                        h1 { class: "text-2xl md:text-2xl font-medium", {t!("contact-details")} }
                    }
                    // Contact details grid
                    div { class: "grid grid-cols-2 gap-3",
                        // Email
                        div { class: "flex col-span-2 md:col-span-1 flex-col w-full",
                            CTextBox {
                                label: t!("email"),
                                value: "{email}",
                                optional: false,
                                inside_label: true,
                                input_type: "email".to_string(),
                                disabled: email_locked(),
                                oninput: move |event: FormEvent| {
                                    if !email_locked() {
                                        email.set(event.value());
                                    }
                                },
                            }
                        }
                        // Phone
                        div { class: "flex col-span-2 md:col-span-1 flex-col w-full",
                            CTextBox {
                                label: t!("phone-number"),
                                value: "{phone}",
                                optional: true,
                                inside_label: true,
                                oninput: move |event: FormEvent| {
                                    phone.set(event.value());
                                }
                            }
                        }
                    }
                    // Email updates checkbox
                    div { class: "mt-3",
                        label { class: "inline-flex items-center gap-2 cursor-pointer text-ui-fg-subtle",
                            input {
                                r#type: "checkbox",
                                checked: "{email_list}",
                                onchange: move |_ev| {
                                    email_list.set(!email_list());
                                }
                            }
                            span { {t!("email-news-updates")} }
                        }
                    }

                    // Delivery header
                    div { class: "flex justify-start mb-6 mt-8",
                        h1 { class: "text-2xl md:text-2xl font-medium", {t!("delivery")} }
                    }

                    // Delivery fields
                    div { class: "grid grid-cols-2 gap-3",
                        // Country select (full width) - FIXED VERSION
                        div { class: "flex flex-col col-span-2 w-full",
                            div {
                                class: "relative flex items-center text-base-regular border border-typical bg-ui-bg-subtle rounded-md hover:bg-ui-bg-field-hover",
                                select {
                                    class: "appearance-none flex-1 bg-transparent border-none px-4 py-2.5 transition-colors duration-150 outline-none",
                                    value: current_country.read().clone(),
                                    onchange: move |event: FormEvent| {
                                        let new_country = event.value();
                                        spawn({
                                            let update_country = update_country.clone();
                                            async move {
                                                update_country(new_country).await;
                                            }
                                        });
                                    },
                                    // Add a default empty option
                                    option {
                                        value: "",
                                        selected: current_country.read().is_empty(),
                                        "Select a country..."
                                    }
                                    for (iso, display) in countries.iter() {
                                        option {
                                            value: iso.clone(),
                                            selected: *current_country.read() == *iso,
                                            key: "{iso}",
                                            { display.clone() }
                                        }
                                    }
                                }
                                span { class: "absolute flex pointer-events-none justify-end w-full pr-2 group-hover:animate-pulse",
                                    svg { width: "16", height: "16", view_box: "0 0 16 16", fill: "none", xmlns: "http://www.w3.org/2000/svg",
                                        path { d: "M4 6L8 10L12 6", stroke: "currentColor", stroke_width: "1.5", stroke_linecap: "round", stroke_linejoin: "round" }
                                    }
                                }
                            }
                        }

                        // First name
                        div { class: "flex col-span-2 md:col-span-1 flex-col w-full",
                            CTextBox {
                                label: t!("first-name"),
                                value: "{first_name}",
                                optional: false,
                                inside_label: true,
                                oninput: move |event: FormEvent| first_name.set(event.value()),
                            }
                        }
                        // Last name
                        div { class: "flex col-span-2 md:col-span-1 flex-col w-full",
                            CTextBox {
                                label: t!("last-name"),
                                value: "{last_name}",
                                optional: false,
                                inside_label: true,
                                oninput: move |event: FormEvent| last_name.set(event.value()),
                            }
                        }
                        // Company
                        div { class: "flex col-span-2 flex-col w-full",
                            CTextBox {
                                label: t!("company"),
                                value: "{company}",
                                optional: true,
                                inside_label: true,
                                oninput: move |event: FormEvent| company.set(event.value()),
                            }
                        }
                        // Address line 1
                        div { class: "flex col-span-2 flex-col w-full",
                            CTextBox {
                                label: t!("address"),
                                value: "{address_line_1}",
                                optional: false,
                                inside_label: true,
                                oninput: move |event: FormEvent| address_line_1.set(event.value()),
                            }
                        }
                        // Address line 2
                        div { class: "flex col-span-2 flex-col w-full",
                            CTextBox {
                                label: t!("apartment-suite-etc"),
                                value: "{address_line_2}",
                                optional: true,
                                inside_label: true,
                                oninput: move |event: FormEvent| address_line_2.set(event.value()),
                            }
                        }

                        // City / Province / Post code (3-col stack on md+)
                        div { class: "flex flex-col col-span-2 w-full",
                            div { class: "grid grid-cols-3 gap-3 w-full",
                                // City
                                div { class: "flex col-span-3 md:col-span-1 flex-col w-full",
                                    CTextBox {
                                        label: t!("city-town"),
                                        value: "{city}",
                                        optional: false,
                                        inside_label: true,
                                        oninput: move |event: FormEvent| city.set(event.value()),
                                    }
                                }
                                // Province
                                div { class: "flex col-span-3 md:col-span-1 flex-col w-full",
                                    CTextBox {
                                        label: t!("state-province"),
                                        value: "{province}",
                                        optional: true,
                                        inside_label: true,
                                        oninput: move |event: FormEvent| province.set(event.value()),
                                    }
                                }
                                // Post code
                                div { class: "flex col-span-3 md:col-span-1 flex-col w-full",
                                    CTextBox {
                                        label: t!("zip-postal-code"),
                                        value: "{post_code}",
                                        optional: false,
                                        inside_label: true,
                                        oninput: move |event: FormEvent| post_code.set(event.value()),
                                    }
                                }
                            }
                        }
                    }

                    // Select shipping method (placeholder)
                    div { class: "flex w-full justify-start mb-4 mt-4",
                        h1 { class: "text-xl md:text-xl font-medium", {t!("select-shipping-method")} }
                    }

                    /*                    if !*info_passes.read() {
                        div { class: "bg-gray-200 rounded-md p-5 pl-4 text-bbase",
                            p { class: "text-ui-fg-muted", {t!("enter-shipping-to-see-options")} }
                        }
                    } else {
                        p { class: "text-ui-fg-subtle", "..." }
                    }
                    */

                    if !current_country.read().is_empty() && !email().is_empty() && !first_name().is_empty() && !last_name().is_empty() && !address_line_1().is_empty() && !post_code().is_empty() && !city().is_empty() {
                        div {
                            class: "border border-typical rounded-md pt-1",
                            div {
                                class: "border-b overflow-hidden border-typical p-2 flex items-center justify-center",
                                ShippingMap {
                                    country: current_country.read().clone(),
                                    canvas_width: 400,
                                }
                            }
                            div {
                                class: "p-3 space-y-2 px-2",
                                {
                                    if let Some(basket) = basket_signal() {
                                        if let Some(shipping_results) = basket.shipping_results.clone() {
                                            if shipping_results.is_empty() {
                                                rsx! {
                                                    div {
                                                        class: "text-center py-8 text-gray-500",
                                                        p { { t!("no-shipping-options") } }
                                                    }
                                                }
                                            } else {
                                                rsx! {
                                                    for result in {
                                                        let mut sorted_results: Vec<_> = shipping_results.iter().collect();
                                                        sorted_results.sort_by(|a, b| a.cost_usd.partial_cmp(&b.cost_usd).unwrap());
                                                        sorted_results
                                                    } {
                                                        {
                                                            let option = result.option;
                                                            let is_selected = basket.shipping_option == Some(result.option);
                                                            let is_loading = loading_option_signal() == Some(result.option);

                                                            rsx! {
                                                                div {
                                                                    key: "{result.option}",
                                                                    class: {
                                                                        let base_classes = "border rounded-lg p-3 cursor-pointer transition-colors duration-200 hover:bg-gray-50 relative";
                                                                        let border_classes = if is_loading {
                                                                            "border-blue-500 bg-blue-50 shipping-option-loading"
                                                                        } else if is_selected {
                                                                            "border-blue-500 bg-blue-50"
                                                                        } else {
                                                                            "border-gray-200"
                                                                        };
                                                                        format!("{} {}", base_classes, border_classes)
                                                                    },
                                                                    onclick: move |_| {
                                                                        if loading_option_signal().is_none() {
                                                                            loading_option_signal.set(Some(option));
                                                                            let mut basket_res = basket_signal.clone();
                                                                            let mut loading_signal = loading_option_signal.clone();

                                                                            spawn(async move {
                                                                                match server_functions::update_basket_shipping_option(option).await {
                                                                                    Ok(updated_basket) => {
                                                                                        basket_res.set(Some(updated_basket.clone()));
                                                                                        GLOBAL_CART.with_mut(|c| *c = Some(updated_basket.clone()));
                                                                                        loading_signal.set(None);
                                                                                    }
                                                                                    Err(e) => {
                                                                                        tracing::error!("Failed to update shipping option: {:?}", e);
                                                                                        loading_signal.set(None);
                                                                                    }
                                                                                }
                                                                            });
                                                                        }
                                                                    },
                                                                    div {
                                                                        class: "flex justify-between items-start",
                                                                        div {
                                                                            class: "flex items-start space-x-3 flex-1",
                                                                            // Radio button
                                                                            div {
                                                                                class: "mt-1",
                                                                                input {
                                                                                    r#type: "radio",
                                                                                    name: "shipping-option",
                                                                                    class: "w-4 h-4 text-blue-600 border-gray-300 focus:ring-blue-500 focus:ring-2",
                                                                                    checked: is_selected,
                                                                                    disabled: is_loading,
                                                                                    onchange: |_| {}, // Handled by parent div onclick
                                                                                }
                                                                            }
                                                                            div {
                                                                                class: "flex-1",
                                                                                h4 {
                                                                                    class: "font-medium text-gray-900 mb-1",
                                                                                    "{result.option}"
                                                                                }
                                                                                p {
                                                                                    class: "text-sm text-gray-600 mb-1",
                                                                                    {
                                                                                       result.option.to_description()
                                                                                    }
                                                                                }
                                                                                p {
                                                                                    class: "text-sm text-gray-500",
                                                                                    { t!("num-working-days", num: result.estimated_days.clone()) }
                                                                                }
                                                                            }
                                                                        }
                                                                        div {
                                                                            class: "text-right ml-4 flex flex-col items-end",
                                                                            div {
                                                                                class: "font-semibold text-gray-900",
                                                                                { format!("{}{}", currency_code_symbol(currency_code.as_deref()), format_number(result.cost_usd)) }
                                                                            }
                                                                            if is_loading {
                                                                                div {
                                                                                    class: "text-xs text-blue-600 mt-1 flex items-center space-x-1",
                                                                                    div {
                                                                                        class: "loading-spinner w-3 h-3 border border-blue-600 border-t-transparent rounded-full animate-spin"
                                                                                    }
                                                                                    span { { t!("updating") } }
                                                                                }
                                                                            } else if is_selected {
                                                                                div {
                                                                                    class: "text-xs text-blue-600 mt-1",
                                                                                    { t!("selected") }
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
                                            rsx! {}
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }
                        }

                        if email_error().is_some() || phone_error().is_some() {
                            div { class: "mt-4 space-y-2",
                                if let Some(error) = email_error() {
                                    div { class: "text-red-600 text-sm bg-red-50 p-3 rounded-md border border-red-200",
                                        {error}
                                    }
                                }
                                if let Some(error) = phone_error() {
                                    div { class: "text-red-600 text-sm bg-red-50 p-3 rounded-md border border-red-200",
                                        {error}
                                    }
                                }
                            }
                        }

                    } else {
                        div { class: "bg-gray-200 rounded-md p-5 pl-4 text-bbase",
                            p { class: "text-ui-fg-muted", {t!("enter-shipping-to-see-options")} }
                        }
                    }

                    // Payment section (non-functional placeholder)

                    // Payment section - Cryptocurrency
                    if *form_validation_passes.read() && *info_passes.read() && basket_signal().and_then(|b| b.shipping_option).is_some() {
                        div { class: "flex justify-start mb-4 mt-8",
                            h1 { class: "text-2xl md:text-2xl font-medium", {t!("payment")} }
                        }

                        div {
                            class: "border border-typical rounded-md pt-1",
                            div {
                                class: "border-b border-typical p-4 flex items-center",
                                img {
                                    src: asset!("/assets/icons/cryptos.avif"),
                                    alt: "Cryptocurrency",
                                    class: "h-6 mr-3"
                                }
                                h3 { class: "text-lg font-medium", {t!("pay-with-cryptocurrency")} }
                            }
                            div {
                                class: "p-4 space-y-3",
                                // Crypto options as list
                                div { class: "bg-gray-50 rounded-md p-1",
                                    p { class: "text-sm font-medium text-gray-700 mb-2", {t!("accepted-cryptocurrencies")} }
                                    div { class: "flex flex-wrap gap-2",
                                        for crypto in ["BTC", "BCH", "LTC", "XMR", "USDT (BEP-20/TRC-20)", "USDC (BEP-20/TRC-20)"] {
                                            span {
                                                key: "{crypto}",
                                                class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800 border border-blue-200",
                                                {crypto}
                                            }
                                        }
                                    }
                                }

                                // Warning message
                                div { class: "bg-yellow-50 border border-yellow-200 rounded-md p-3 mt-4",
                                    p { class: "text-sm text-yellow-800",
                                        {t!("crypto-payment-warning")}
                                    }
                                }

                                // Terms agreement checkbox
                                div { class: "mt-4",
                                    label { class: "inline-flex items-start gap-3 cursor-pointer",
                                        input {
                                            r#type: "checkbox",
                                            checked: "{payment_agreed}",
                                            class: "w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500 focus:ring-2 mt-0.5",
                                            onchange: move |_| {
                                                payment_agreed.set(!payment_agreed());
                                            }
                                        }
                                        span { class: "text-sm text-gray-700",
                                            span {
                                                {format!("{} ",t!("agree-terms-payment-pre"))}
                                            }
                                            Link {
                                                to: Route::Policies {},
                                                class: "a",
                                                new_tab: true,
                                                {t!("terms-of-service")}
                                            }
                                        }
                                    }
                                }

                                // Show error message if there's one
                                {
                                    if let Some(error) = payment_error() {
                                        rsx! {
                                            div { class: "mb-4 p-3 bg-red-50 border border-red-200 rounded-md",
                                                p { class: "text-sm text-red-800", "{error}" }
                                            }
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }

                                // Continue to payment button
                                div { class: "mt-6",
                                    if *payment_agreed.read() {
                                        button {
                                            disabled: *payment_loading.read(),
                                            class: {
                                                if *payment_loading.read() {
                                                    "w-full inline-flex justify-center items-center px-6 py-3 border border-transparent text-base font-medium rounded-md text-white bg-gray-400 cursor-not-allowed"
                                                } else {
                                                    "w-full inline-flex justify-center items-center px-6 py-3 border border-transparent text-base font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 transition-colors duration-200"
                                                }
                                            },
                                            onclick: handle_payment_initiation,
                                            if *payment_loading.read() {
                                                div { class: "flex items-center",
                                                    div { class: "loading-spinner w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin mr-2" }
                                                    span { { t!("processing") } }
                                                }
                                            } else {
                                                { t!("continue-to-payment") }
                                            }
                                        }
                                    } else {
                                        button {
                                            disabled: true,
                                            class: "w-full inline-flex justify-center items-center px-6 py-3 border border-transparent text-base font-medium rounded-md text-white bg-gray-400 cursor-not-allowed",
                                            { t!("continue-to-payment") }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "flex justify-start mb-4 mt-8",
                            h1 { class: "text-2xl md:text-2xl font-medium", {t!("payment")} }
                        }
                        div { class: "bg-gray-200 rounded-md p-5 pl-4 text-bbase",
                            p { class: "text-ui-fg-muted", {t!("complete-form-first")} }
                        }
                    }
                }

                // Right column: In Your Cart section (display only)
                div { class: "w-full lg:w-auto lg:min-w-[26rem] lg:mt-8 pt-16 lg:pt-0 lg:pl-20",
                    h1 { class: "text-2xl md:text-2xl font-medium flex",
                        {t!("in-your-cart")}
                        Link {
                            to: Route::Cart {},
                            title: t!("visit-cart"),
                            img { alt: "[open]", class: "blende ml-2 mt-1 fadey", src: asset!("/assets/icons/open-outline.svg"), style: "height:20px;" }

                        }
                    }
                    div { class: "h-px w-full border-b border-gray-200 mt-4 mb-4" }

                    // Totals like Svelte
                    div {
                        div { class: "flex flex-col gap-y-2 text-bbase text-ui-fg-subtle",
                            div { class: "flex items-center justify-between",
                                span { class: "flex gap-x-1 items-center",
                                    {t!("subtotal")}
                                    svg {
                                        xmlns: "http://www.w3.org/2000/svg", width: "20", height: "20", fill: "none", "data-state": "closed",
                                        path { fill: "gray", fill_rule: "evenodd", d: "M18 10a8 8 0 1 1-16.001 0A8 8 0 0 1 18 10Zm-7-4a1 1 0 1 1-2 0 1 1 0 0 1 2 0ZM9 9a.75.75 0 0 0 0 1.5h.253a.25.25 0 0 1 .244.304l-.459 2.066A1.75 1.75 0 0 0 10.747 15H11a.75.75 0 1 0 0-1.5h-.253a.25.25 0 0 1 .244-.304l.459-2.066A1.75 1.75 0 0 0 9.253 9H9Z", clip_rule: "evenodd" }
                                    }
                                }
                                span { {format!("{}{}", currency_code_symbol(currency_code.as_deref()), format_number(*subtotal.read()))} }
                            }
                            div { class: "flex items-center justify-between",
                                {
                                    if let Some(basket) = basket_signal() {
                                        if let Some(discount_data) = basket.discount {
                                            if let Some(discount_code) = basket.discount_code {
                                                match discount_data.discount_type {
                                                    DiscountType::PercentageOnShipping
                                                    | DiscountType::FixedAmountOnShipping => {
                                                        rsx! {
                                                            span { {
                                                                t!("shipping-discount-code", code: discount_code)
                                                            } }
                                                        }
                                                    },
                                                    _ => {
                                                        rsx! {
                                                            span { {t!("shipping")} }
                                                        }
                                                    }
                                                }
                                            } else {
                                                rsx! {
                                                    span { {t!("shipping")} }
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                span { {t!("shipping")} }
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            span { {t!("shipping")} }
                                        }
                                    }
                                }
                                {
                                    if let Some(basket) = basket_signal() {
                                        if let Some(results) = basket.shipping_results && basket.shipping_option.is_some() {
                                            rsx! {
                                                span { title: t!("to-be-definite"),
                                                    {
                                                        format!(
                                                            "{}{}{}",
                                                            currency_code_symbol(currency_code.as_deref()),
                                                            format_number(*shipping_cost.read()),
                                                            if let Some(discount_o) = basket.discount {
                                                                match discount_o.discount_type {
                                                                    DiscountType::PercentageOnShipping => {
                                                                        format!(" ({})",
                                                                            t!("num-off",
                                                                                num: format!(
                                                                                    "{}%",
                                                                                    format_float(discount_o.discount_percentage)
                                                                                )
                                                                            )
                                                                        )
                                                                    },
                                                                    DiscountType::FixedAmountOnShipping => {
                                                                        format!(" (-{}{})", currency_code_symbol(currency_code.as_deref()), format_number(shipping_discount_off().unwrap_or(0.0)))
                                                                    },
                                                                    _ => { "".to_string() },
                                                                }
                                                            } else {
                                                                "".to_string()
                                                            }
                                                        )
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                span { title: t!("to-be-determined"), "TBD" }
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            span { title: t!("to-be-determined"), "TBD" }
                                        }
                                    }
                                }
                            }

                            {
                                if let Some(basket) = basket_signal() {
                                    if let Some(discount_o) = basket.discount {
                                        if let Some(discount_off) = discount_off() && (discount_o.discount_type == DiscountType::Percentage || discount_o.discount_type == DiscountType::FixedAmount ) {
                                            rsx! {
                                                div { class: "flex items-center justify-between",
                                                    span { {t!("discount-code", code: basket.discount_code.unwrap_or("?".to_string()))} }

                                                    span {
                                                        class: "text-blue-600",
                                                        {
                                                            format!(
                                                                "-{}{:.2}{}",
                                                                currency_code_symbol(currency_code.as_deref()),
                                                                discount_off,
                                                                match discount_o.discount_type {
                                                                    DiscountType::Percentage => {
                                                                        format!(" ({})",
                                                                            t!("num-off",
                                                                                num: format!(
                                                                                    "{}%",
                                                                                    format_float(discount_o.discount_percentage)
                                                                                )
                                                                            )
                                                                        )
                                                                    },
                                                                    DiscountType::FixedAmount => { "".to_string() },
                                                                    _ => { "".to_string() },
                                                                }
                                                            )
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                } else {
                                    rsx! {}
                                }
                            }

                            /*
                            div { class: "flex justify-between",
                                span { class: "flex gap-x-1 items-center", {t!("taxes")} }
                                span { {format!("{}{}", currency_code_symbol(currency_code.as_deref()), format_number(*taxes.read()))} }
                            }
                            */
                        }
                        div { class: "h-px w-full border-b border-gray-200 my-4" }
                        div { class: "flex items-center justify-between text-ui-fg-base mb-2 txt-medium",
                            span { {t!("total")} }
                            span {
                                class: "txt-xlarge-plus",
                                {
                                    let base_total = *total.read();
                                    let shipping_cost = *shipping_cost.read();

                                    let final_total = base_total + shipping_cost;

                                    format!("{}{}",
                                        currency_code_symbol(currency_code.as_deref()),
                                        format_number(final_total)
                                    )
                                }
                            }
                        }
                    }

                    div { class: "h-px w-full border-b border-gray-200 mt-4 mb-4" }

                    // Items list (display only)
                    if loading {
                        div { class: "mt-5 text-ui-fg-subtle", {t!("loading")} }
                    } else if cart_lines.read().is_empty() {
                        div { class: "mt-5 text-ui-fg-subtle", {t!("cart-empty-msg")} }
                    } else {
                        {
                            let lines = cart_lines.read().clone();
                            rsx! {
                                div { class: "mt-5",
                                    for item in lines.into_iter() {
                                        div { class: "flex mb-5 justify-between", key: "{item.basket_item_id.clone()}",
                                            div { class: "flex",
                                                div { class: "w-12 relative",
                                                    // quantity bubble
                                                    { qty_bubble(item.quantity) }
                                                    // image (variant thumbnail or placeholder)
                                                    div { class: "boop rounded-md relative w-full overflow-hidden p-4 bg-ui-bg-subtle shadow-elevation-card-rest rounded-large group-hover:shadow-elevation-card-hover transition-shadow ease-in-out duration-150 aspect-[1/1] border-ui-border-base border",
                                                        if let Some(ref url) = item.variant.thumbnail_url {
                                                            img {
                                                                alt: "Thumbnail", draggable: "false", loading: "lazy", decoding: "async",
                                                                class: "absolute inset-0 object-cover object-center w-full",
                                                                style: "position: absolute; height: 100%; width: 100%; inset: 0px; color: transparent;",
                                                                src: url.to_string()
                                                            }
                                                        } else if let Some(ref smiles) = item.product.smiles {
                                                            div {
                                                                class: "absolute inset-0 flex items-center justify-center",
                                                                style: "overflow: hidden;",

                                                                div {
                                                                    class: "w-full h-full flex items-center justify-center",
                                                                    style: "max-width: 100%; max-height: 100%;",

                                                                    SmilesViewer {
                                                                        smiles: smiles.clone()
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                // Titles
                                                div { class: "ml-6 mt-0",
                                                    p { class: "text-sm",
                                                        if item.product.title.contains("(") {
                                                            {item.product.title.clone()}
                                                        } else {
                                                            {item.product.title.clone()}
                                                            if let Some(label) = product_form_label(&item.product.product_form) {
                                                                " "
                                                                span { { format!("({})", label) } }
                                                            }
                                                        }
                                                    }
                                                    p { class: "text-sm font-normal font-sans txt-medium inline-block txt-medium text-ui-fg-subtle w-full overflow-hidden text-ellipsis",
                                                        if item.variant.variant_name != "Default option value" {
                                                            {item.variant.variant_name.clone()}
                                                        } else {
                                                            if let Some(ref sub) = item.product.subtitle {
                                                                {sub.clone()}
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            // Right: line total only
                                            div { class: "text-right flex flex-col justify-center mt-[-8px]",
                                                p { class: "text-base-regular text-ui-fg-subtle",
                                                    {
                                                        let line_total = item.variant.price_standard_usd * (item.quantity as f64);
                                                        format!("{}{}", currency_code_symbol(currency_code.as_deref()), format_number(line_total))
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
        } // end if !auth_loading
    }
}
