use crate::backend::front_entities::{CustomerBasket, Product};
use chrono::NaiveDateTime;
use dioxus::prelude::*;

// CART DATA accessable through all views
pub static GLOBAL_CART: GlobalSignal<Option<CustomerBasket>> = Global::new(|| None);

pub fn compute_total_qty(basket: &CustomerBasket) -> i32 {
    basket
        .items
        .as_ref()
        .map(|items| items.iter().map(|it| it.quantity).sum())
        .unwrap_or(0)
}

pub fn use_fade_in_image_optional(src: Option<&String>) -> (String, Callback<Event<ImageData>>) {
    if let Some(src) = src {
        use_fade_in_image(src)
    } else {
        ("".to_string(), Callback::new(|_| {}))
    }
}

/*
pub fn plural(length: usize) -> &str {
    if length > 0  {
        "s"
    }

    ""
}
*/

pub fn use_fade_in_image(src: &String) -> (String, Callback<Event<ImageData>>) {
    let mut image_loaded = use_signal(|| false);
    let mut should_animate = use_signal(|| true);
    let mut load_start_time = use_signal(|| js_sys::Date::now());

    // Reset state when src changes
    use_effect(move || {
        image_loaded.set(false);
        should_animate.set(true);
        load_start_time.set(js_sys::Date::now());
    });

    let class_name = format!(
        "fade-in-image {} {}",
        if should_animate() { "" } else { "no-fade" },
        if image_loaded() { "loaded" } else { "" }
    );

    let onload_handler = use_callback(move |_: Event<ImageData>| {
        let load_time = js_sys::Date::now() - load_start_time();

        if load_time < 50.0 {
            should_animate.set(false);
        }

        image_loaded.set(true);
    });

    (class_name, onload_handler)
}

pub fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn format_number(v: f64) -> String {
    format!("{:.2}", v)
}

pub fn format_float(n: Option<f64>) -> String {
    match n {
        Some(value) => {
            let formatted = format!("{:.2}", value);
            formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        }
        None => String::new(),
    }
}

pub fn capitalize_if_alpha(s: &str) -> String {
    // Check if string contains only English alphabet letters
    if s.chars().all(|c| c.is_ascii_alphabetic()) && !s.is_empty() {
        // Convert first char to uppercase, append rest of string
        let mut chars = s.chars();
        chars.next().unwrap().to_uppercase().collect::<String>() + chars.as_str()
    } else {
        // Return original string if conditions not met
        s.to_string()
    }
}

pub fn format_datetime(dt: NaiveDateTime) -> String {
    dt.format("%Y/%m/%d, %H:%M").to_string()
}

pub fn sort_products_by_priority<'a>(products: &'a [Product]) -> Vec<&'a Product> {
    let mut sorted_products: Vec<_> = products.iter().collect();
    sorted_products.sort_by(|a, b| {
        // Helper function to calculate total stock for a product
        let get_total_stock = |product: &Product| -> i32 {
            product
                .variants
                .as_ref()
                .map(|variants| {
                    variants
                        .iter()
                        .map(|variant| variant.calculated_stock_quantity.unwrap_or(0))
                        .sum()
                })
                .unwrap_or(0)
        };

        let a_stock = get_total_stock(a);
        let b_stock = get_total_stock(b);

        // Check if item should be sent to back (no stock AND no priority)
        let a_to_back = a_stock == 0 && a.priority.is_none();
        let b_to_back = b_stock == 0 && b.priority.is_none();

        match (a_to_back, b_to_back) {
            (true, false) => std::cmp::Ordering::Greater, // a goes to back
            (false, true) => std::cmp::Ordering::Less,    // b goes to back
            _ => {
                // Both are in the same group (either both normal or both to back)
                // Sort by priority within the group
                match (a.priority, b.priority) {
                    (Some(a_priority), Some(b_priority)) => b_priority.cmp(&a_priority), // Higher priority first
                    (Some(_), None) => std::cmp::Ordering::Less, // Items with priority come before items without
                    (None, Some(_)) => std::cmp::Ordering::Greater, // Items without priority come after items with
                    (None, None) => std::cmp::Ordering::Equal, // Items without priority maintain their order
                }
            }
        }
    });
    sorted_products
}


pub fn filter_products(products: &[Product], search_q: &str) -> Vec<Product> {
    if search_q.is_empty() {
        return Vec::new();
    }

    products
        .iter()
        .filter(|product| {
            // Check title
            let title_match = product
                .title
                .to_lowercase()
                .contains(&search_q.to_lowercase());
            // Check CAS (Option type)
            let cas_match = if let Some(cas) = &product.cas {
                cas.contains(&search_q)
            } else {
                false
            };
            // Check names (Option type)
            let names_match = if let Some(names) = &product.alternate_names {
                names
                    .iter()
                    .any(|name| name.to_lowercase().contains(&search_q.to_lowercase()))
            } else {
                false
            };
            // Check pubchem (Option type) - exact match
            let pubchem_match = if let Some(pubchem) = &product.pubchem_cid {
                pubchem == &search_q
            } else {
                false
            };
            title_match || cas_match || names_match || pubchem_match
        })
        .cloned()
        .collect()
}
