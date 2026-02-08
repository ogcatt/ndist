use super::front_entities::*;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use rand::Rng;
#[cfg(feature = "server")]
use regex::Regex;
#[cfg(feature = "server")]
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
#[cfg(feature = "server")]
use sea_orm::{
    self, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr,
    EntityTrait, IntoActiveModel, ModelTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait, prelude::Expr,
};
#[cfg(feature = "server")]
use std::collections::HashMap;
#[cfg(feature = "server")]
use std::env;
#[cfg(feature = "server")]
use std::pin::Pin;
#[cfg(feature = "server")]
use super::server_functions::DbErrExt;
#[cfg(feature = "server")]
use thiserror::Error;
#[cfg(feature = "server")]
use tokio::join;
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
use super::db::get_db;
#[cfg(feature = "server")]
use super::email::{EmailService, EmailType};
#[cfg(feature = "server")]
use super::entity_conversions;
#[cfg(feature = "server")]
use super::shipping_calculations::{
    calculate_shipping_cost, calculate_shipping_cost_with_preorder_surcharge,
};
#[cfg(feature = "server")]
use crate::backend::server_functions::{
    calculate_stock_quantities, calculate_total_cart_weight, calculate_variant_available_stock,
    check_discount, get_or_create_basket, get_stock_quantities_for_stock_items,
};
#[cfg(feature = "server")]
use crate::utils::{capitalize_if_alpha, countries::allowed_countries};
#[cfg(feature = "server")]
use entity::{
    self, address, basket_items, customer_baskets, discounts, user_sessions, users, order,
    order_item, payment, product_variant_stock_item_relations, product_variants, products,
    sea_orm_active_enums, stock_active_reduce, stock_backorder_active_reduce, stock_batches,
    stock_item_relations, stock_items, stock_preorder_active_reduce,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitPaymentOrderRequest {
    pub email: String,
    pub shipping_info: CustomerShippingInfo,
}

/* This is imported from front entities now:
 * pub struct CustomerShippingInfo {
     // The phone of the customer (added to tracking)
     phone: Option<String>,
     // If the customer joined the email push list
     email_list: bool,
     // Valid first name
     first_name: String,
     // Valid last name
     last_name: String,
     // The company name if exists
     company: Option<String>,
     // The primary address line
     address_line_1: String,
     // The second address line (e.g apt, suite, etc.)
     address_line_2: Option<String>,
     // The postcode for shipment
     post_code: String,
     // The province/state/county of shipment
     province: Option<String>,
     // The city of the shipment address
     city: String,
     // Do not set this on the frontend, this will be implied by the cart automatically
     country: Option<String>
 }
*/

// Custom error enum for validation errors
#[derive(Debug, Serialize, thiserror::Error, PartialEq)]
pub enum ValidationError {
    #[error("Invalid email format")]
    InvalidEmailFormat,
    #[error("Invalid phone number format")]
    InvalidPhoneNumberFormat,
    #[error("Field {0} exceeds 128 characters")]
    FieldTooLong(String),
    #[error("Could not retrieve referenced cart")]
    CartRefFailed,
    #[error("No country on cart")]
    NoCartCountry,
    #[error("Cart country not allowed")]
    InvalidCartCountry,
    #[error("No shipping option set on cart")]
    NoShippingOption,
    #[error("Failed discount get check")]
    DiscountFailed,
    #[error("Discount invalid")]
    DiscountInvalid,
    #[error("Discount lookup failed")]
    DiscountLookFailed,
    #[error("No discount reduction cart model attributes")]
    DiscountAttributeFailed,
    #[error("No basket items")]
    NoBasketItems,
    #[error("Bitcart failed to create invoice")]
    BitCartInitFailed,
    #[error("Failed find product variant")]
    FailedFindProductVariant,

    #[error("Could not assume variable as some even after some check")]
    CouldNotAssumeAsSome,
}

#[derive(Debug, Serialize, thiserror::Error)]
#[error("Validation failed: {0:?}")]
pub struct ValidationErrors(pub Vec<ValidationError>);

// Implement From trait to allow using ? operator with ValidationErrors in server functions
impl From<ValidationErrors> for ServerFnError {
    fn from(err: ValidationErrors) -> Self {
        ServerFnError::new(format!("Validation error: {}", err))
    }
}

// Create the payment and associated order. This is triggered at the end stage of checkout.
#[server]
pub async fn init_payment_and_order(
    request: InitPaymentOrderRequest,
) -> Result<String, ServerFnError> {
    tracing::info!("Attempting creation of payment and order");

    let db = get_db().await; // Await the db once here
    let shipping_info = request.shipping_info;

    // Ensure we have a basket from cookie. This will create one if it does not exist so it should be scanned for validity.
    // This also checks the stock for validity
    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();

    //--- LOCK CART --- THIS EXISTS TO PREVENT THE CART BEING UPDATED AFTER THE CHECKS HAVE COMPLETED

    customer_baskets::Entity::update_many()
        .col_expr(customer_baskets::Column::Locked, Expr::value(true))
        .filter(customer_baskets::Column::Id.eq(&basket_id))
        .exec(db).await.map_db_err()?;

    // Get products with variants at the same time
    let products_fut = products::Entity::find()
        .find_with_related(product_variants::Entity)
        .all(db);

    // Find the basket record in the database
    let basket_fut = customer_baskets::Entity::find()
        .filter(customer_baskets::Column::Id.eq(&basket_id))
        .one(db);

    let basket_items_fut = basket_items::Entity::find()
        .filter(basket_items::Column::BasketId.eq(&basket_id))
        .all(db);

    let discounts_fut = discounts::Entity::find().all(db);

    let stock_batches_fut = stock_batches::Entity::find()
        .filter(stock_batches::Column::LiveQuantity.gt(0.0))
        .filter(stock_batches::Column::Status.eq(sea_orm_active_enums::StockBatchStatus::Complete))
        .order_by_asc(stock_batches::Column::CreatedAt)
        .all(db);

    let stock_relations_fut = stock_item_relations::Entity::find().all(db);

    let variant_stock_relations_fut = product_variant_stock_item_relations::Entity::find().all(db);

    // Add stock quantities future
    let stock_quantities_fut = get_stock_quantities_for_stock_items(None);

    let (
        products_res,
        basket_res,
        basket_items_res,
        discounts_res,
        stock_batches_res,
        stock_relations_res,
        variant_stock_relations_res,
        stock_quantities_res,
    ) = join!(
        products_fut,
        basket_fut,
        basket_items_fut,
        discounts_fut,
        stock_batches_fut,
        stock_relations_fut,
        variant_stock_relations_fut,
        stock_quantities_fut
    );

    let products_with_variants: Vec<(products::Model, Vec<product_variants::Model>)> =
        products_res.map_db_err()?;
    let basket_entity = basket_res.map_db_err()?;
    let basket_items_entity = basket_items_res.map_db_err()?;
    let discounts_entities = discounts_res.map_db_err()?;
    let stock_batches_entities = stock_batches_res.map_db_err()?;
    let stock_relations_entities = stock_relations_res.map_db_err()?;
    let variant_stock_relations_entities = variant_stock_relations_res.map_db_err()?;
    let stock_results_entities = stock_quantities_res?;

    // Extract products and variants from the tuple structure for the stock reduces function
    let products_entities: Vec<products::Model> = products_with_variants
        .iter()
        .map(|(product, _)| product.clone())
        .collect();

    let product_variants_entities: Vec<product_variants::Model> = products_with_variants
        .iter()
        .flat_map(|(_, variants)| variants.clone())
        .collect();

    if basket_entity.is_none() {
        return Err(ValidationErrors(vec![ValidationError::CartRefFailed]).into());
    }

    //--- CHECKS --- CHECK ALL PARAMETERS

    // Definite basket entity
    let basket_mod = basket_entity.unwrap_or_else(|| {
        unlock_cart(&basket_id);
        panic!("Could not unwrap basket even after checking for some...")
    });

    // Check the cart country code for existence and validity
    if let Some(ref country_code) = basket_mod.country_code {
        if !allowed_countries().contains(&country_code.as_str()) {
            unlock_cart(&basket_id);
            return Err(ValidationErrors(vec![ValidationError::NoCartCountry]).into());
        }
    } else {
        unlock_cart(&basket_id);
        return Err(ValidationErrors(vec![ValidationError::InvalidCartCountry]).into());
    }

    // Make sure the basket has a shipping option
    if basket_mod.shipping_option.is_none() {
        unlock_cart(&basket_id);
        return Err(ValidationErrors(vec![ValidationError::NoShippingOption]).into());
    }

    let mut discount_model: Option<discounts::Model> = None;

    if let Some(discount_code) = basket_mod.discount_code.clone() {
        // Handle country code extraction
        let country_code = match basket_mod.country_code {
            Some(ref code) => code,
            None => {
                unlock_cart(&basket_id).await;
                return Err(ValidationErrors(vec![ValidationError::CouldNotAssumeAsSome]).into());
            }
        };

        let variants: Vec<product_variants::Model> = products_with_variants
            .clone()
            .into_iter()
            .flat_map(|(_, variants)| variants)
            .collect();

        // Handle discount check
        let check = match check_discount(
            discount_code,
            Some(country_code.to_string()),
            Some(discounts_entities.clone()),
            basket_items_entity.clone(),
            variants,
        )
        .await
        {
            Ok(check) => check,
            Err(_) => {
                unlock_cart(&basket_id).await;
                return Err(ValidationErrors(vec![ValidationError::DiscountFailed]).into());
            }
        };

        if !check.is_valid {
            unlock_cart(&basket_id).await;
            return Err(ValidationErrors(vec![ValidationError::DiscountInvalid]).into());
        } else {
            discount_model = Some(check.discount.clone());
        }
    }

    let email_regex =
        Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").expect("Email regex failed creation");
    let phone_regex = Regex::new(r"^\+?[1-9]\d{1,14}$").expect("Phone regex failed creation");

    // If this contains any entries after checks then the init should be cancelled
    let mut validation_errors = Vec::new();
    if !email_regex.is_match(&request.email) {
        validation_errors.push(ValidationError::InvalidEmailFormat);
    }

    // Check email length
    if request.email.len() > 128 {
        validation_errors.push(ValidationError::FieldTooLong("email".to_string()));
    }
    if let Some(phone) = &shipping_info.phone {
        if !phone_regex.is_match(phone) {
            validation_errors.push(ValidationError::InvalidPhoneNumberFormat);
        }
        if phone.len() > 128 {
            validation_errors.push(ValidationError::FieldTooLong("phone".to_string()));
        }
    }
    // Validate string field lengths in CustomerShippingInfo
    if shipping_info.first_name.len() > 128 {
        validation_errors.push(ValidationError::FieldTooLong("first_name".to_string()));
    }
    if shipping_info.last_name.len() > 128 {
        validation_errors.push(ValidationError::FieldTooLong("last_name".to_string()));
    }
    if let Some(company) = &shipping_info.company {
        if company.len() > 128 {
            validation_errors.push(ValidationError::FieldTooLong("company".to_string()));
        }
    }
    if shipping_info.address_line_1.len() > 128 {
        validation_errors.push(ValidationError::FieldTooLong("address_line_1".to_string()));
    }
    if let Some(address_line_2) = &shipping_info.address_line_2 {
        if address_line_2.len() > 128 {
            validation_errors.push(ValidationError::FieldTooLong("address_line_2".to_string()));
        }
    }
    if shipping_info.post_code.len() > 128 {
        validation_errors.push(ValidationError::FieldTooLong("post_code".to_string()));
    }
    if let Some(province) = &shipping_info.province {
        if province.len() > 128 {
            validation_errors.push(ValidationError::FieldTooLong("province".to_string()));
        }
    }
    if shipping_info.city.len() > 128 {
        validation_errors.push(ValidationError::FieldTooLong("city".to_string()));
    }
    if let Some(country) = &shipping_info.country {
        if country.len() > 128 {
            validation_errors.push(ValidationError::FieldTooLong("country".to_string()));
        }
    }

    // COST CALCULATIONS

    // The cost including discount, excluding shipping
    let mut subtotal_cost = 0.0;
    // Total shipping cost for the selected option
    let mut shipping_cost = 0.0;
    // This should be positive as it is taken away from the final price
    let mut discount_reduction_cost = 0.0;

    // Calculate subtotal using cart items
    if let Some(ref basket_items) = basket.items {
        if basket_items.len() > 0 {
            for basket_item in basket_items {
                if let Some((product, variant)) =
                    find_product_variant(&products_with_variants, &basket_item.product_variant_id)
                {
                    subtotal_cost += variant.price_standard_usd * basket_item.quantity as f64;
                }
            }
        } else {
            validation_errors.push(ValidationError::NoBasketItems);
        }
    } else {
        validation_errors.push(ValidationError::CouldNotAssumeAsSome);
    }

    // Calculate shipping cost

    // Calculate shipping cost
    let total_basket_weight_grams =
        calculate_total_cart_weight(&basket_items_entity, &products_with_variants, false);

    let total_basket_weight_grams_exc_pre =
        calculate_total_cart_weight(&basket_items_entity, &products_with_variants, true);

    // Calculate basket cost for shipping calculation
    let basket_cost_usd = subtotal_cost;

    // Separate basket items into pre-order and regular items
    let mut preorder_items = Vec::new();
    let mut regular_items = Vec::new();

    for item in &basket_items_entity {
        if let Some((product, _)) = find_product_variant(&products_with_variants, &item.variant_id)
        {
            if product.pre_order {
                preorder_items.push(item);
            } else {
                regular_items.push(item);
            }
        }
    }

    // Calculate shipping costs based on item composition
    match (regular_items.is_empty(), preorder_items.is_empty()) {
        (true, true) => {
            // No items - this should have been caught earlier
            validation_errors.push(ValidationError::NoBasketItems);
        }
        (false, true) => {
            // Only regular items - use regular shipping
            let regular_weight = calculate_total_cart_weight(
                &regular_items.iter().cloned().cloned().collect::<Vec<_>>(),
                &products_with_variants,
                false,
            );

            // Calculate cost for regular items only
            let regular_cost: f64 = regular_items
                .iter()
                .map(|item| {
                    if let Some((_, variant)) =
                        find_product_variant(&products_with_variants, &item.variant_id)
                    {
                        variant.price_standard_usd * item.quantity as f64
                    } else {
                        0.0
                    }
                })
                .sum();

            if let Some(shipping_quote) = calculate_shipping_cost(
                &basket_mod
                    .country_code
                    .clone()
                    .expect("expected country code to exist"),
                regular_weight as u32,
                regular_cost,
            ) {
                for option in shipping_quote.available_options {
                    if option.option == ShippingOption::from(basket.shipping_option.unwrap()) {
                        shipping_cost = option.cost_usd;
                    }
                }
            }
        }
        (true, false) => {
            // Only pre-order items - use regular shipping (no surcharge when alone)
            let preorder_weight = calculate_total_cart_weight(
                &preorder_items.iter().cloned().cloned().collect::<Vec<_>>(),
                &products_with_variants,
                false,
            );

            // Calculate cost for preorder items only
            let preorder_cost: f64 = preorder_items
                .iter()
                .map(|item| {
                    if let Some((_, variant)) =
                        find_product_variant(&products_with_variants, &item.variant_id)
                    {
                        variant.price_standard_usd * item.quantity as f64
                    } else {
                        0.0
                    }
                })
                .sum();

            if let Some(shipping_quote) = calculate_shipping_cost(
                &basket_mod
                    .country_code
                    .clone()
                    .expect("expected country code to exist"),
                preorder_weight as u32,
                preorder_cost,
            ) {
                for option in shipping_quote.available_options {
                    if option.option == ShippingOption::from(basket.shipping_option.unwrap()) {
                        shipping_cost = option.cost_usd;
                    }
                }
            }
        }
        (false, false) => {
            // Both regular and pre-order items - combine shipping costs with surcharge on pre-orders
            let regular_weight = calculate_total_cart_weight(
                &regular_items.iter().cloned().cloned().collect::<Vec<_>>(),
                &products_with_variants,
                false,
            );
            let preorder_weight = calculate_total_cart_weight(
                &preorder_items.iter().cloned().cloned().collect::<Vec<_>>(),
                &products_with_variants,
                false,
            );

            // Calculate costs separately
            let regular_cost: f64 = regular_items
                .iter()
                .map(|item| {
                    if let Some((_, variant)) =
                        find_product_variant(&products_with_variants, &item.variant_id)
                    {
                        variant.price_standard_usd * item.quantity as f64
                    } else {
                        0.0
                    }
                })
                .sum();

            let preorder_cost: f64 = preorder_items
                .iter()
                .map(|item| {
                    if let Some((_, variant)) =
                        find_product_variant(&products_with_variants, &item.variant_id)
                    {
                        variant.price_standard_usd * item.quantity as f64
                    } else {
                        0.0
                    }
                })
                .sum();

            let regular_quote = calculate_shipping_cost(
                &basket_mod
                    .country_code
                    .clone()
                    .expect("expected country code to exist"),
                regular_weight as u32,
                regular_cost,
            );

            let preorder_quote = calculate_shipping_cost_with_preorder_surcharge(
                &basket_mod
                    .country_code
                    .clone()
                    .expect("expected country code to exist"),
                preorder_weight as u32,
                preorder_cost,
            );

            match (regular_quote, preorder_quote) {
                (Some(regular), Some(preorder)) => {
                    let selected_shipping_option =
                        ShippingOption::from(basket.shipping_option.unwrap());

                    let regular_cost = regular
                        .available_options
                        .iter()
                        .find(|opt| opt.option == selected_shipping_option)
                        .map(|opt| opt.cost_usd)
                        .unwrap_or(0.0);

                    let preorder_cost = preorder
                        .available_options
                        .iter()
                        .find(|opt| opt.option == selected_shipping_option)
                        .map(|opt| opt.cost_usd)
                        .unwrap_or(0.0);

                    shipping_cost = regular_cost + preorder_cost;
                }
                _ => {
                    // If shipping calculation fails, set to 0 or handle error
                    shipping_cost = 0.0;
                }
            }
        }
    }

    // Calculate discount reductions

    let mut discount_id: Option<String> = None;

    if let Some(discount_code) = basket_mod.discount_code.clone() {
        let discount_match = discounts_entities.iter().find(|d| d.code == *discount_code);

        match discount_match {
            Some(found_discount) => {
                discount_id = Some(found_discount.id.clone());

                let discount_type = DiscountType::from_seaorm(found_discount.discount_type.clone());
                match discount_type {
                    DiscountType::Percentage => {
                        if let Some(percentage) = found_discount.discount_percentage {
                            discount_reduction_cost = (subtotal_cost * (percentage / 100.0))
                        } else {
                            validation_errors.push(ValidationError::DiscountAttributeFailed);
                        }
                    }
                    DiscountType::FixedAmount => {
                        if let Some(discount_amount) = found_discount.discount_amount {
                            let amount_used = found_discount.amount_used.unwrap_or(0.0);
                            let discount_amount_left = (discount_amount - amount_used).max(0.0);
                            discount_reduction_cost = discount_amount_left.min(subtotal_cost);
                        } else {
                            validation_errors.push(ValidationError::DiscountAttributeFailed);
                        }
                    }
                    DiscountType::PercentageOnShipping => {
                        if let Some(percentage) = found_discount.discount_percentage {
                            discount_reduction_cost = (shipping_cost * (percentage / 100.0))
                        } else {
                            validation_errors.push(ValidationError::DiscountAttributeFailed);
                        }
                    }
                    DiscountType::FixedAmountOnShipping => {
                        if let Some(discount_amount) = found_discount.discount_amount {
                            let amount_used = found_discount.amount_used.unwrap_or(0.0);
                            let discount_amount_left = (discount_amount - amount_used).max(0.0);
                            discount_reduction_cost = discount_amount_left.min(shipping_cost);
                        } else {
                            validation_errors.push(ValidationError::DiscountAttributeFailed);
                        }
                    }
                }
            }
            None => {
                validation_errors.push(ValidationError::DiscountLookFailed);
            }
        }
    }

    // Debug
    tracing::info!("subtotal: {}", subtotal_cost);
    tracing::info!("shipping: {}", shipping_cost);

    // Add the subtotal and shipping cost together for the final cost
    let final_cost = (subtotal_cost + shipping_cost) - discount_reduction_cost;

    tracing::info!("final cost: {}", final_cost);

    // CHECK IF ALL VALIDITY STEPS HAVE PASSED, OTHERWISE ERROR
    if !validation_errors.is_empty() {
        unlock_cart(&basket_id).await;
        return Err(ValidationErrors(validation_errors).into());
    }

    let payment_id = Uuid::new_v4().to_string();

    let (bitcart_invoice_id, bitcart_invoice_url) =
        create_bitcart_payment(final_cost, payment_id.clone())
            .await
            .map_err(|e| {
                unlock_cart(&basket_id);
                tracing::info!("BitCart init error: {:?}", e);
                ValidationErrors(vec![ValidationError::BitCartInitFailed])
            })?;

    // Debug for bitcart
    //tracing::info!("{:?}", bitcart_payment);

    //--- START DATABASE TRANSACTIONS ---
    let txn = db.begin().await.map_db_err()?;

    // Generate unique IDs
    let order_id = Uuid::new_v4().to_string();
    let address_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    // Create PENDING order entity
    let order = order::ActiveModel {
        id: Set(order_id.clone()),
        ref_code: Set(OrderCodeGenerator::generate()),
        customer_id: Set(None), // Don't integrate customer ID's for now
        customer_email: Set(request.email.clone()),
        add_to_email_list: Set(shipping_info.email_list),
        billing_country: Set(basket_mod.country_code.clone().unwrap().clone()),
        shipping_option: Set(basket.shipping_option.unwrap().to_seaorm()),
        subtotal_usd: Set(subtotal_cost),
        shipping_usd: Set(shipping_cost),
        order_weight: Set(total_basket_weight_grams_exc_pre),
        refund_comment: Set(None),
        status: Set(sea_orm_active_enums::OrderStatus::Pending),
        fulfilled_at: Set(None),
        cancelled_at: Set(None),
        refunded_at: Set(None),
        prepared_at: Set(None),
        tracking_url: Set(None),
        total_amount_usd: Set(final_cost), // subtotal + shipping
        discount_id: Set(discount_id),
        notes: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let order_result = order.insert(&txn).await.map_db_err()?;

    // Create order item entities LINKED TO order

    // Collect order items to insert
    let mut order_items = Vec::new();
    let mut order_item_ids = Vec::new();

    for basket_item in basket.items.unwrap() {
        let temp_id = Uuid::new_v4().to_string();
        order_item_ids.push(temp_id.clone()); // Collect the ID

        let (product, variant) =
            match find_product_variant(&products_with_variants, &basket_item.product_variant_id) {
                Some((product, variant)) => (product, variant),
                None => {
                    unlock_cart(&basket_id).await;
                    return Err(
                        ValidationErrors(vec![ValidationError::FailedFindProductVariant]).into(),
                    );
                }
            };

        let order_item = order_item::ActiveModel {
            id: Set(temp_id),
            order_id: Set(order_id.clone()),
            product_variant_id: Set(basket_item.product_variant_id.clone()),
            quantity: Set(basket_item.quantity),
            price_usd: Set(variant.price_standard_usd),
            product_title: Set(product.title.clone()),
            variant_name: Set(variant.variant_name.clone()),
            pre_order_on_purchase: Set(product.pre_order),
        };

        order_items.push(order_item);
    }

    // Insert all order items in the transaction
    for order_item in order_items {
        order_item.insert(&txn).await.map_db_err()?;
    }

    // Create shipping address entity LINKED TO order

    let address = address::ActiveModel {
        id: Set(address_id),
        order_id: Set(order_id.clone()),
        customer_id: Set(None),
        first_name: Set(capitalize_if_alpha(&shipping_info.first_name.clone())),
        last_name: Set(capitalize_if_alpha(&shipping_info.last_name.clone())),
        company: Set(shipping_info.company.clone()),
        address_line_1: Set(shipping_info.address_line_1.clone()),
        address_line_2: Set(shipping_info.address_line_2.clone()),
        city: Set(shipping_info.city.clone()),
        province: Set(shipping_info.province.clone()),
        postal_code: Set(shipping_info.post_code.clone()),
        country: Set(basket_mod.clone().country_code.unwrap().clone()),
        phone: Set(shipping_info.phone.clone()),
        r#type: Set(sea_orm_active_enums::AddressType::Shipping),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let address_result = address.insert(&txn).await.map_db_err()?;

    // Create payment entity LINKED TO order

    let payment = payment::ActiveModel {
        id: Set(payment_id.clone()),
        order_id: Set(Some(order_id.clone())),
        method: Set("Crypto".to_string()),
        processor_ref: Set(Some(bitcart_invoice_id.clone())),
        processor_url: Set(Some(bitcart_invoice_url.clone())),
        status: Set(sea_orm_active_enums::PaymentStatus::Pending),
        amount_usd: Set(final_cost), // Same as order total
        paid_at: Set(None),          // Will be set when payment is completed
        created_at: Set(now),
        updated_at: Set(now),
    };

    let payment_result = payment.insert(&txn).await.map_db_err()?;

    // Link PAYMENT ID to cart:

    let mut basket_active = basket_mod.clone().into_active_model();
    basket_active.payment_id = Set(Some(payment_id.clone()));
    basket_active.update(&txn).await.map_db_err()?;

    // Create active reduce entries LINKED TO order

    let stock_items_entities: Vec<stock_items::Model> = stock_items::Entity::find().all(db).await.map_db_err()?;

    let reduces_to_add = create_stock_reduces(
        basket_items_entity,
        order_item_ids, // Pass the collected order item IDs
        order_id.clone(),
        &stock_batches_entities,
        &stock_items_entities, // Add the stock items entities
        &stock_relations_entities,
        &variant_stock_relations_entities,
        &products_entities,
        &product_variants_entities,
        &stock_results_entities,
    )
    .await?;

    tracing::info!("{:#?}", reduces_to_add);

    // Insert all reduces
    insert_stock_reduces(reduces_to_add, &txn).await?;

    // If discount exists, increase the discount's active_reduce_quantity

    if let Some(discount_code) = basket_mod.clone().discount_code.clone() {
        let discount_match = discounts_entities.iter().find(|d| d.code == *discount_code);

        if let Some(disc) = discount_match {
            let mut discount_active = disc.clone().into_active_model();
            discount_active.active_reduce_quantity = Set(disc.active_reduce_quantity + 1);
            discount_active.update(&txn).await.map_db_err()?;
        }
    }

    // FINALLY EXECUTE THE DATABASE TRANSACTIONS txn.commit().await.map_db_err()?;

    // Temporary return
    tracing::info!("SUCCESS FOR PAYMENT CREATION");
    Ok(payment_id)
}

#[cfg(feature = "server")]
pub async fn cancel_payment(payment_id: &str, expire: bool) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let payment_fut = payment::Entity::find()
        .filter(payment::Column::Id.eq(payment_id))
        .one(db);

    let basket_fut = customer_baskets::Entity::find()
        .filter(customer_baskets::Column::PaymentId.eq(payment_id))
        .one(db);

    let (payment_res, basket_res) = join!(payment_fut, basket_fut);

    let basket_entity = basket_res.map_db_err()?;
    let payment_mod = payment_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get payment by payment id (does it exist?)..."));

    let order_id = payment_mod
        .order_id
        .clone()
        .ok_or_else(|| ServerFnError::new("Payment does not have an associated order"))?;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let (address_res, order_res) = join!(address_fut, order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod = order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));

    let customer_email = order_mod.customer_email.clone();
    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
    };

    // Start seaorm transaction
    let txn = db.begin().await.map_db_err()?;
    let now = Utc::now().naive_utc();

    // Delete stock active reduces
    stock_active_reduce::Entity::delete_many()
        .filter(stock_active_reduce::Column::OrderId.eq(&order_id))
        .exec(&txn).await.map_db_err()?;

    // Delete pre-order active reduces
    stock_preorder_active_reduce::Entity::delete_many()
        .filter(stock_preorder_active_reduce::Column::OrderId.eq(&order_id))
        .exec(&txn).await.map_db_err()?;

    // Delete backorder active reduces
    stock_backorder_active_reduce::Entity::delete_many()
        .filter(stock_backorder_active_reduce::Column::OrderId.eq(&order_id))
        .exec(&txn).await.map_db_err()?;

    // Remove payment id from basket
    if let Some(basket) = basket_entity {
        let mut basket_active = basket.clone().into_active_model();
        basket_active.payment_id = Set(None);
        basket_active.locked = Set(false); // UNLOCK CART
        if expire {
            basket_active.payment_failed_at = Set(Some(now));
        }
        basket_active.update(&txn).await.map_db_err()?;
    } // If it can't find the basket don't modify it (it should be able to however)

    // Update payment entity status to expired
    let mut payment_active = payment_mod.clone().into_active_model();
    payment_active.status = Set(if expire {
        sea_orm_active_enums::PaymentStatus::Expired
    } else {
        sea_orm_active_enums::PaymentStatus::Cancelled
    });
    payment_active.order_id = Set(None);
    payment_active.update(&txn).await.map_db_err()?;

    // If discount exists on order, remove active reduce
    if let Some(ref discount_id) = order_mod.discount_id {
        let discount_entity = discounts::Entity::find()
            .filter(discounts::Column::Id.eq(discount_id))
            .one(db).await.map_db_err()?;

        if let Some(discount) = discount_entity {
            let mut discount_active = discount.clone().into_active_model();
            discount_active.active_reduce_quantity =
                Set((discount.active_reduce_quantity - 1).max(0));
            discount_active.update(&txn).await.map_db_err()?;
        } // If fails fetch ignore (discount was probably deleted)
    }

    let add_to_email_list = order_mod.add_to_email_list;

    // IF CANCEL THEN DELETE ORDER AND ADDRESS/ITEMS, OTHERWISE KEEP THEM JUST IN CASE OF PAYMENT FAILURE
    if !expire {
        // Delete address associated with failed order
        address::Entity::delete_many()
            .filter(address::Column::OrderId.eq(&order_id))
            .exec(&txn).await.map_db_err()?;

        // Delete order items
        order_item::Entity::delete_many()
            .filter(order_item::Column::OrderId.eq(&order_id))
            .exec(&txn).await.map_db_err()?;

        // Delete order
        order_mod
            .delete(&txn) // Delete the order within the transaction
            .await.map_db_err()?;
    } txn.commit().await.map_db_err()?;

    if !expire {
        delete_bitcart_payment(&payment_id).await;
    } else {
        // Send expired email to the user
        let email_service = EmailService::new()?;

        match email_service
            .send_email(
                &customer_email,
                &customer_name,
                EmailType::ExpiredOrder,
                add_to_email_list,
            )
            .await
        {
            Ok(()) => tracing::info!("success sending email"),
            Err(e) => tracing::info!("{e:?}"),
        }
    }

    Ok(())
}

#[cfg(feature = "server")]
pub async fn complete_payment(payment_id: &str) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let payment_fut = payment::Entity::find()
        .filter(payment::Column::Id.eq(payment_id))
        .one(db);

    let basket_fut = customer_baskets::Entity::find()
        .filter(customer_baskets::Column::PaymentId.eq(payment_id))
        .one(db);

    let (payment_res, basket_res) = join!(payment_fut, basket_fut);

    let basket_entity = basket_res.map_db_err()?;
    let payment_mod = payment_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get payment by payment id (does it exist?)..."));

    let order_id = payment_mod
        .order_id
        .clone()
        .ok_or_else(|| ServerFnError::new("Payment does not have an associated order"))?;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let backorder_reduces_fut = stock_backorder_active_reduce::Entity::find()
        .filter(stock_backorder_active_reduce::Column::OrderId.eq(&order_id))
        .all(db);

    let (address_res, order_res, backorder_reduces_res) =
        join!(address_fut, order_fut, backorder_reduces_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod = order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));
    let backorder_reduces = backorder_reduces_res.map_db_err()?;

    let stock_active_reduces_mod = stock_active_reduce::Entity::find()
        .filter(stock_active_reduce::Column::OrderId.eq(&order_id))
        .all(db).await.map_db_err()?;

    // Start seaorm transaction
    let txn = db.begin().await.map_db_err()?;
    let now = Utc::now().naive_utc();

    // UPDATE PAYMENT AS PAID

    let mut payment_active = payment_mod.clone().into_active_model();
    payment_active.status = Set(sea_orm_active_enums::PaymentStatus::Paid);
    payment_active.paid_at = Set(Some(now));
    payment_active.update(&txn).await.map_db_err()?;

    // UPDATE ORDER AS PAID

    let mut order_active = order_mod.clone().into_active_model();
    order_active.status = Set(sea_orm_active_enums::OrderStatus::Paid);
    order_active.update(&txn).await.map_db_err()?;

    // FLATTEN DISCOUNT ACTIVE REDUCE

    if let Some(ref discount_id) = order_mod.discount_id {
        let discount_entity = discounts::Entity::find()
            .filter(discounts::Column::Id.eq(discount_id))
            .one(db).await.map_db_err()?;

        if let Some(discount) = discount_entity {
            let mut discount_active = discount.clone().into_active_model();
            discount_active.active_reduce_quantity =
                Set((discount.active_reduce_quantity - 1).max(0));
            discount_active.discount_used = Set(discount.discount_used + 1);
            discount_active.update(&txn).await.map_db_err()?;
        } // If fails fetch ignore (discount was probably deleted)
    }

    if let Some(basket) = basket_entity {
        // DELETE CART ITEMS

        basket_items::Entity::delete_many()
            .filter(basket_items::Column::BasketId.eq(&basket.id))
            .exec(&txn).await.map_db_err()?;

        // DELETE CART

        customer_baskets::Entity::delete_many()
            .filter(customer_baskets::Column::Id.eq(&basket.id))
            .exec(&txn).await.map_db_err()?;
    }

    // UPDATE STOCK BACKORDER/PREORDER REDUCE ENTRIES AS ACTIVE

    // Update stock_backorder_active_reduce entries
    stock_backorder_active_reduce::Entity::update_many()
        .col_expr(
            stock_backorder_active_reduce::Column::Active,
            Expr::value(true),
        )
        .filter(stock_backorder_active_reduce::Column::OrderId.eq(&order_mod.id))
        .exec(&txn).await.map_db_err()?;

    // Update stock_preorder_active_reduce entries
    stock_preorder_active_reduce::Entity::update_many()
        .col_expr(
            stock_preorder_active_reduce::Column::Active,
            Expr::value(true),
        )
        .filter(stock_preorder_active_reduce::Column::OrderId.eq(&order_mod.id))
        .exec(&txn).await.map_db_err()?; txn.commit().await.map_db_err()?;

    // FLATTEN STOCK ITEM ACTIVE REDUCES

    flatten_stock_reduces(stock_active_reduces_mod).await?;

    // Send order confirmation email
    let email_service = EmailService::new()?;

    let customer_email = order_mod.customer_email.clone();
    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
    };

    let email_type = if backorder_reduces.len() > 0 {
        EmailType::OrderConfirmationWithBackorder {
            order_id: order_mod.id.clone(),
            order_ref: order_mod.ref_code.clone(),
            order_date: now.format("%Y-%m-%d").to_string(),
        }
    } else {
        EmailType::OrderConfirmation {
            order_id: order_mod.id.clone(),
            order_ref: order_mod.ref_code.clone(),
            order_date: now.format("%Y-%m-%d").to_string(),
        }
    };

    match email_service
        .send_email(
            &customer_email,
            &customer_name,
            email_type,
            order_mod.add_to_email_list,
        )
        .await
    {
        Ok(()) => tracing::info!("success sending order confirmation email"),
        Err(e) => tracing::info!("{e:?}"),
    }

    Ok(())
}

// Check payments function (ran with cron)
#[cfg(feature = "server")]
pub async fn check_payments() -> Result<(), ServerFnError> {
    let db = get_db().await;

    // Get all pending payments with their related orders in a single query
    let pending_payments = payment::Entity::find()
        .filter(payment::Column::Status.eq(sea_orm_active_enums::PaymentStatus::Pending))
        .find_also_related(order::Entity)
        .all(db).await.map_db_err()?;

    // Process each pending payment
    for (payment, order_opt) in pending_payments {
        // Skip payments without order_id or processor_ref
        if payment.order_id.is_none() || payment.processor_ref.is_none() {
            continue;
        }

        let processor_ref = payment.processor_ref.as_ref().unwrap();
        let payment_id = &payment.id;

        // Check BitCart status for this payment
        let bitcart_check = get_bitcart_payment_status(processor_ref).await;
        let bitcart_status = match bitcart_check {
            Ok(status) => status,
            Err(e) => {
                tracing::info!(
                    "Could not get bitcart status for payment {}: {:?}",
                    payment_id,
                    e
                );
                // Continue to next payment instead of panicking
                continue;
            }
        };

        // Process based on BitCart status
        match bitcart_status {
            PaymentUStatus::Pending => {
                // No action needed, keep as pending
                continue;
            }
            PaymentUStatus::Invalid => {
                if let Err(e) = cancel_payment(payment_id, false).await {
                    tracing::info!("Failed to cancel invalid payment {}: {:?}", payment_id, e);
                }
            }
            PaymentUStatus::Expired => {
                if let Err(e) = cancel_payment(payment_id, true).await {
                    tracing::info!("Failed to cancel expired payment {}: {:?}", payment_id, e);
                }
            }
            PaymentUStatus::Complete => {
                if let Err(e) = complete_payment(payment_id).await {
                    tracing::info!("Failed to complete payment {}: {:?}", payment_id, e);
                }
            }
        }
    }

    Ok(())
}

#[cfg(feature = "server")]
pub async fn check_payment(payment_id: &str) -> Result<PaymentShortInfo, ServerFnError> {
    let db = get_db().await;

    let payment = payment::Entity::find()
        .filter(payment::Column::Id.eq(payment_id))
        .one(db).await.map_db_err()?
        .expect("Could not get payment model when trying to check payment");

    let order_id_temp = payment.order_id.clone();

    if order_id_temp.is_none() {
        return Ok(PaymentShortInfo {
            status: PaymentStatus::from_seaorm(payment.status),
            order_id: "".to_string(),
            order_ref_code: "NOREFF".to_string(),
            processor_url: "".to_string(),
        });
    }

    let order_id = order_id_temp.unwrap();
    let processor_url = payment.processor_url.clone();

    let order = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db).await.map_db_err()?
        .expect("Could not order related to payment");

    let ref_code = order.ref_code;

    match payment.status {
        sea_orm_active_enums::PaymentStatus::Cancelled
        | sea_orm_active_enums::PaymentStatus::Failed
        | sea_orm_active_enums::PaymentStatus::Paid
        | sea_orm_active_enums::PaymentStatus::Refunded => {
            return Ok(PaymentShortInfo {
                status: PaymentStatus::from_seaorm(payment.status),
                order_id: order_id,
                order_ref_code: ref_code,
                processor_url: processor_url.unwrap_or("".to_string()),
            });
        }
        _ => {}
    }

    let processor_ref = payment.processor_ref.expect("Processor ref should exist");

    let bitcart_check = get_bitcart_payment_status(&processor_ref).await;
    let mut bitcart_status: Option<PaymentUStatus> = None;

    match bitcart_check {
        Ok(bitcart) => {
            bitcart_status = Some(bitcart);
        }
        Err(e) => {
            tracing::info!("Could not get bitcart for id: {:?}", e);
            return Err(ServerFnError::new(format!("BitCart API error: {:?}", e)));
        }
    }

    let mut temp_status: PaymentStatus = PaymentStatus::Pending;

    match bitcart_status.expect("Bitcart status should exist") {
        PaymentUStatus::Pending => {
            temp_status = PaymentStatus::Pending;
        }
        PaymentUStatus::Invalid => {
            let cancel = cancel_payment(&payment_id, false).await?;
            temp_status = PaymentStatus::Failed;
        }
        PaymentUStatus::Expired => {
            let cancel = cancel_payment(&payment_id, true).await?;
            temp_status = PaymentStatus::Expired;
        }
        PaymentUStatus::Complete => {
            // COMPLETE PAYMENT / ORDER HERE
            complete_payment(&payment_id).await?;
            temp_status = PaymentStatus::Paid;
        }
    }

    return Ok(PaymentShortInfo {
        status: temp_status,
        order_id: order_id,
        order_ref_code: ref_code,
        processor_url: processor_url.unwrap_or("".to_string()),
    });
}

#[cfg(feature = "server")]
pub async fn unlock_cart(basket_id: &str) -> Result<(), ServerFnError> {
    let db = get_db().await;

    // Find the basket record in the database
    let basket_entity = customer_baskets::Entity::find()
        .filter(customer_baskets::Column::Id.eq(basket_id))
        .one(db).await.map_db_err()?;

    if let Some(basket_model) = basket_entity.clone() {
        let mut basket_active: customer_baskets::ActiveModel = basket_model.into();
        basket_active.locked = ActiveValue::Set(false);

        // Save the updated basket
        customer_baskets::Entity::update(basket_active)
            .exec(db).await.map_db_err()?;
    } else {
        // Do nothing (soft error if the basket doesn't exist)
    }

    Ok(())
}

#[derive(Debug)]
#[cfg(feature = "server")]
pub struct StockReducesResult {
    pub regular_reduces: Vec<stock_active_reduce::ActiveModel>,
    pub preorder_reduces: Vec<stock_preorder_active_reduce::ActiveModel>,
    pub backorder_reduces: Vec<stock_backorder_active_reduce::ActiveModel>,
}

#[derive(Debug, Clone)]
#[cfg(feature = "server")]
pub enum OrderType {
    Regular,
    PreOrder,
    BackOrder,
}

#[cfg(feature = "server")]
pub async fn create_stock_reduces(
    basket_items: Vec<basket_items::Model>,
    order_item_ids: Vec<String>, // Add this to map basket items to their order item IDs
    order_id: String,
    stock_batches: &Vec<stock_batches::Model>,
    stock_items: &Vec<stock_items::Model>, // Add stock items parameter
    stock_relations: &Vec<stock_item_relations::Model>,
    variant_stock_relations: &Vec<product_variant_stock_item_relations::Model>,
    products: &Vec<products::Model>,
    product_variants: &Vec<product_variants::Model>,
    stock_results: &Vec<StockQuantityResult>,
) -> Result<StockReducesResult, ServerFnError> {
    let mut regular_reduces = Vec::new();
    let mut preorder_reduces = Vec::new();
    let mut backorder_reduces = Vec::new();
    let now = Utc::now().naive_utc();

    // Create lookup maps
    let stock_lookup: HashMap<String, &StockQuantityResult> = stock_results
        .iter()
        .map(|result| (result.stock_item_id.clone(), result))
        .collect();

    // Create stock items lookup map
    let stock_items_lookup: HashMap<String, &stock_items::Model> = stock_items
        .iter()
        .map(|item| (item.id.clone(), item))
        .collect();

    let mut variant_relations_map: HashMap<
        String,
        Vec<&product_variant_stock_item_relations::Model>,
    > = HashMap::new();
    for relation in variant_stock_relations {
        variant_relations_map
            .entry(relation.product_variant_id.clone())
            .or_insert_with(Vec::new)
            .push(relation);
    }

    for (index, basket_item) in basket_items.iter().enumerate() {
        let order_item_id = &order_item_ids[index]; // Get the corresponding order item ID

        // Find the variant and then the product to determine order type
        let variant = product_variants
            .iter()
            .find(|variant| variant.id == basket_item.variant_id);

        let (order_type, available_stock) = if let Some(variant) = variant {
            let product = products
                .iter()
                .find(|product| product.id == variant.product_id);

            if let Some(product) = product {
                // Calculate available stock for this variant
                let available_stock = calculate_variant_available_stock(
                    &basket_item.variant_id,
                    &variant_relations_map,
                    &stock_lookup,
                );

                // Determine order type based on availability and product settings
                let order_type = if product.back_order && basket_item.quantity > available_stock {
                    OrderType::BackOrder
                } else if product.pre_order {
                    OrderType::PreOrder
                } else {
                    OrderType::Regular
                };

                (order_type, available_stock)
            } else {
                (OrderType::Regular, 0)
            }
        } else {
            (OrderType::Regular, 0)
        };

        // Get the stock item relations for this variant from pre-fetched data
        let variant_relations: Vec<&product_variant_stock_item_relations::Model> =
            variant_stock_relations
                .iter()
                .filter(|relation| relation.product_variant_id == basket_item.variant_id)
                .collect();

        for relation in variant_relations {
            let required_quantity = basket_item.quantity as f64 * relation.quantity;

            match order_type {
                OrderType::Regular => {
                    // Regular processing with child stock item support
                    let mut item_reduces = process_stock_item_requirement_with_cache(
                        &relation.stock_item_id,
                        required_quantity,
                        &order_id,
                        now,
                        stock_batches,
                        stock_relations,
                    )?;
                    regular_reduces.append(&mut item_reduces);
                }
                OrderType::PreOrder | OrderType::BackOrder => {
                    // Get the stock item to retrieve its unit
                    if let Some(stock_item) = stock_items_lookup.get(&relation.stock_item_id) {
                        // For preorder/backorder, only create reduces on top-level stock items
                        let top_level_reduces = create_top_level_stock_reduces(
                            &relation.stock_item_id,
                            required_quantity,
                            &order_id,
                            order_item_id, // Pass the order item ID
                            now,
                            &StockUnit::from_seaorm(stock_item.unit.clone()), // Pass the stock unit from the stock item
                            &order_type,
                        )?;

                        match order_type {
                            OrderType::PreOrder => {
                                preorder_reduces.extend(top_level_reduces.preorder_reduces)
                            }
                            OrderType::BackOrder => {
                                backorder_reduces.extend(top_level_reduces.backorder_reduces)
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
    }

    Ok(StockReducesResult {
        regular_reduces,
        preorder_reduces,
        backorder_reduces,
    })
}

/// Creates reduces only on top-level (highest) stock items for preorder/backorder
#[cfg(feature = "server")]
fn create_top_level_stock_reduces(
    stock_item_id: &str,
    required_quantity: f64,
    order_id: &str,
    order_item_id: &str,
    created_at: chrono::NaiveDateTime,
    stock_unit: &StockUnit, // Add stock_unit parameter since we're not using batches
    order_type: &OrderType,
) -> Result<StockReducesResult, ServerFnError> {
    let mut preorder_reduces = Vec::new();
    let mut backorder_reduces = Vec::new();

    // Create a single reduce record for the entire required quantity
    // since we're no longer tracking individual batches
    match order_type {
        OrderType::PreOrder => {
            let reduce = stock_preorder_active_reduce::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                order_id: Set(order_id.to_string()),
                order_item_id: Set(order_item_id.to_string()),
                stock_item_id: Set(stock_item_id.to_string()), // Use stock_item_id instead of stock_batch_id
                stock_unit: Set(stock_unit.clone().to_seaorm()),
                reduction_quantity: Set(required_quantity),
                active: Set(false),
                created_at: Set(created_at),
                updated_at: Set(created_at),
            };
            preorder_reduces.push(reduce);
        }
        OrderType::BackOrder => {
            let reduce = stock_backorder_active_reduce::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                order_id: Set(order_id.to_string()),
                order_item_id: Set(order_item_id.to_string()),
                stock_item_id: Set(stock_item_id.to_string()), // Use stock_item_id instead of stock_batch_id
                stock_unit: Set(stock_unit.clone().to_seaorm()),
                reduction_quantity: Set(required_quantity),
                active: Set(false),
                created_at: Set(created_at),
                updated_at: Set(created_at),
            };
            backorder_reduces.push(reduce);
        }
        _ => unreachable!(),
    }

    Ok(StockReducesResult {
        regular_reduces: Vec::new(),
        preorder_reduces,
        backorder_reduces,
    })
}

/// Original function for regular active reduces (supports child stock items)
#[cfg(feature = "server")]
fn process_stock_item_requirement_with_cache(
    stock_item_id: &str,
    required_quantity: f64,
    order_id: &str,
    created_at: chrono::NaiveDateTime,
    stock_batches: &Vec<stock_batches::Model>,
    stock_relations: &Vec<stock_item_relations::Model>,
) -> Result<Vec<stock_active_reduce::ActiveModel>, ServerFnError> {
    let mut reduces = Vec::new();
    let mut remaining_quantity = required_quantity;

    // Step 1: Try to fulfill from batches (READY stock) - oldest first
    let mut item_batches: Vec<&stock_batches::Model> = stock_batches
        .iter()
        .filter(|batch| batch.stock_item_id == stock_item_id && batch.live_quantity > 0.0)
        .collect();

    // Sort by created_at (oldest first) - they should already be sorted from the query
    item_batches.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    for batch in item_batches {
        if remaining_quantity <= 0.0 {
            break;
        }

        let batch_available = batch.live_quantity;
        let quantity_to_reduce = remaining_quantity.min(batch_available);

        let reduce = stock_active_reduce::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            order_id: Set(order_id.to_string()),
            stock_batch_id: Set(batch.id.clone()),
            stock_unit: Set(batch.stock_unit_on_creation.clone()),
            reduction_quantity: Set(quantity_to_reduce),
            created_at: Set(created_at),
            updated_at: Set(created_at),
        };

        reduces.push(reduce);
        remaining_quantity -= quantity_to_reduce;
    }

    // Step 2: If still need more, try child stock items (UNREADY stock)
    if remaining_quantity > 0.0 {
        let child_relations: Vec<&stock_item_relations::Model> = stock_relations
            .iter()
            .filter(|relation| relation.parent_stock_item_id == stock_item_id)
            .collect();

        if !child_relations.is_empty() {
            // Calculate how many units we can make from child items
            let mut max_units_possible = f64::INFINITY;

            // First pass: determine the limiting factor
            for child_relation in &child_relations {
                let child_batches: Vec<&stock_batches::Model> = stock_batches
                    .iter()
                    .filter(|batch| {
                        batch.stock_item_id == child_relation.child_stock_item_id
                            && batch.live_quantity > 0.0
                    })
                    .collect();

                let total_child_stock: f64 = child_batches.iter().map(|b| b.live_quantity).sum();
                let units_from_this_child = total_child_stock / child_relation.quantity;
                max_units_possible = max_units_possible.min(units_from_this_child);
            }

            let units_to_make = remaining_quantity.min(max_units_possible.floor());

            if units_to_make > 0.0 {
                // Second pass: create reduces for each child item
                for child_relation in child_relations {
                    let child_quantity_needed = units_to_make * child_relation.quantity;

                    // Recursively process child stock item
                    let mut child_reduces = process_stock_item_requirement_with_cache(
                        &child_relation.child_stock_item_id,
                        child_quantity_needed,
                        order_id,
                        created_at,
                        stock_batches,
                        stock_relations,
                    )?;

                    reduces.append(&mut child_reduces);
                }
            }
        }
    }

    Ok(reduces)
}

#[cfg(feature = "server")]
pub async fn insert_stock_reduces(
    reduces_result: StockReducesResult,
    txn: &DatabaseTransaction,
) -> Result<(), ServerFnError> {
    // Insert regular reduces
    for reduce in reduces_result.regular_reduces {
        reduce.insert(txn).await.map_db_err()?;
    }

    // Insert preorder reduces
    for reduce in reduces_result.preorder_reduces {
        reduce.insert(txn).await.map_db_err()?;
    }

    // Insert backorder reduces
    for reduce in reduces_result.backorder_reduces {
        reduce.insert(txn).await.map_db_err()?;
    }

    Ok(())
}

#[cfg(feature = "server")]
pub async fn flatten_stock_reduces(
    reduces: Vec<stock_active_reduce::Model>,
) -> Result<(), ServerFnError> {
    let db = get_db().await;
    let txn = db.begin().await.map_db_err()?;

    // Process each reduce
    for reduce in reduces {
        // Find the corresponding stock batch
        let batch = stock_batches::Entity::find()
            .filter(stock_batches::Column::Id.eq(&reduce.stock_batch_id))
            .one(&txn).await.map_db_err()?;

        if let Some(batch_model) = batch {
            // Update the live quantity
            let new_live_quantity =
                (batch_model.live_quantity - reduce.reduction_quantity).max(0.0);

            let mut batch_active: stock_batches::ActiveModel = batch_model.into();
            batch_active.live_quantity = Set(new_live_quantity);
            batch_active.updated_at = Set(Utc::now().naive_utc());

            // Update the batch
            batch_active.update(&txn).await.map_db_err()?;
        }

        // Delete the reduce entry
        stock_active_reduce::Entity::delete_by_id(&reduce.id)
            .exec(&txn).await.map_db_err()?;
    }

    // Commit the transaction txn.commit().await.map_db_err()?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn flatten_preorder_backorder_reduces(
    stock_item_id: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;
    let txn = db.begin().await.map_db_err()?;

    // Get all active backorder reduces for the specific stock item, ordered by creation date (oldest first)
    let backorder_reduces = stock_backorder_active_reduce::Entity::find()
        .filter(stock_backorder_active_reduce::Column::Active.eq(true))
        .filter(stock_backorder_active_reduce::Column::StockItemId.eq(&stock_item_id))
        .order_by_asc(stock_backorder_active_reduce::Column::CreatedAt)
        .all(&txn).await.map_db_err()?;

    // Get all active preorder reduces for the specific stock item, ordered by creation date (oldest first)
    let preorder_reduces = stock_preorder_active_reduce::Entity::find()
        .filter(stock_preorder_active_reduce::Column::Active.eq(true))
        .filter(stock_preorder_active_reduce::Column::StockItemId.eq(&stock_item_id))
        .order_by_asc(stock_preorder_active_reduce::Column::CreatedAt)
        .all(&txn).await.map_db_err()?;

    // Process backorders first (higher priority)
    for reduce in backorder_reduces {
        let reduce_id = reduce.id.clone();
        if let Err(e) = process_reduce(&txn, reduce, true).await {
            // Log error but continue processing other reduces
            eprintln!("Error processing backorder reduce {}: {}", reduce_id, e);
        }
    }

    // Then process preorders
    for reduce in preorder_reduces {
        let reduce_id = reduce.id.clone();
        if let Err(e) = process_reduce(&txn, reduce, false).await {
            // Log error but continue processing other reduces
            eprintln!("Error processing preorder reduce {}: {}", reduce_id, e);
        }
    }

    // Commit the transaction txn.commit().await.map_db_err()?;
    Ok(())
}

#[cfg(feature = "server")]
async fn process_reduce<C>(
    txn: &C,
    reduce: impl Into<ReduceModel>,
    is_backorder: bool,
) -> Result<(), ServerFnError>
where
    C: ConnectionTrait,
{
    let reduce = reduce.into();

    // Find available batches for this stock item with matching stock unit
    let available_batches = stock_batches::Entity::find()
        .filter(stock_batches::Column::StockItemId.eq(&reduce.stock_item_id))
        .filter(stock_batches::Column::Status.eq(sea_orm_active_enums::StockBatchStatus::Complete))
        .filter(stock_batches::Column::StockUnitOnCreation.eq(reduce.stock_unit.clone()))
        .filter(stock_batches::Column::LiveQuantity.gt(0.0))
        .order_by_asc(stock_batches::Column::CreatedAt) // Process oldest batches first (FIFO)
        .all(txn)
        .await.map_db_err()?;

    // Check if we can fulfill this reduce with available stock
    let total_available: f64 = available_batches
        .iter()
        .map(|batch| batch.live_quantity)
        .sum();

    if total_available < reduce.reduction_quantity {
        // Cannot fulfill this reduce, skip it
        return Ok(());
    }

    // Process the reduction across batches
    let mut remaining_to_reduce = reduce.reduction_quantity;

    for batch in available_batches {
        if remaining_to_reduce <= 0.0 {
            break;
        }

        let reduction_from_this_batch = remaining_to_reduce.min(batch.live_quantity);
        let new_live_quantity = batch.live_quantity - reduction_from_this_batch;

        // Update the batch
        let mut batch_active: stock_batches::ActiveModel = batch.into();
        batch_active.live_quantity = Set(new_live_quantity);
        batch_active.updated_at = Set(Utc::now().naive_utc());
        batch_active.update(txn).await.map_db_err()?;

        remaining_to_reduce -= reduction_from_this_batch;
    }

    // Delete the original reduce entry
    if is_backorder {
        stock_backorder_active_reduce::Entity::delete_by_id(&reduce.id)
            .exec(txn)
            .await.map_db_err()?;
    } else {
        stock_preorder_active_reduce::Entity::delete_by_id(&reduce.id)
            .exec(txn)
            .await.map_db_err()?;
    }

    Ok(())
}

// Helper struct to unify backorder and preorder reduce models
#[derive(Clone, Debug)]
#[cfg(feature = "server")]
struct ReduceModel {
    pub id: String,
    pub order_id: String,
    pub order_item_id: String,
    pub stock_item_id: String,
    pub stock_unit: sea_orm_active_enums::StockUnit,
    pub reduction_quantity: f64,
}

#[cfg(feature = "server")]
impl From<stock_backorder_active_reduce::Model> for ReduceModel {
    fn from(model: stock_backorder_active_reduce::Model) -> Self {
        Self {
            id: model.id,
            order_id: model.order_id,
            order_item_id: model.order_item_id,
            stock_item_id: model.stock_item_id,
            stock_unit: model.stock_unit,
            reduction_quantity: model.reduction_quantity,
        }
    }
}

#[cfg(feature = "server")]
impl From<stock_preorder_active_reduce::Model> for ReduceModel {
    fn from(model: stock_preorder_active_reduce::Model) -> Self {
        Self {
            id: model.id,
            order_id: model.order_id,
            order_item_id: model.order_item_id,
            stock_item_id: model.stock_item_id,
            stock_unit: model.stock_unit,
            reduction_quantity: model.reduction_quantity,
        }
    }
}

#[cfg(feature = "server")]
fn find_product_variant<'a>(
    products_with_variants: &'a Vec<(products::Model, Vec<product_variants::Model>)>,
    variant_id: &str,
) -> Option<(&'a products::Model, &'a product_variants::Model)> {
    for (product, variants) in products_with_variants {
        if let Some(variant) = variants.iter().find(|v| v.id == variant_id) {
            return Some((product, variant));
        }
    }
    None
}

// BITCART LOGIC
#[derive(Serialize)]
#[cfg(feature = "server")]
struct InvoiceRequest {
    store_id: String,
    price: f64,
    currency: String,
    expiration: i32,
    redirect_url: String,
}

#[derive(Deserialize)]
struct InvoiceResponse {
    id: String,
}

// Create a new bitcart payment. Doesn't modify any entities, only returns the id and link of the payment.
#[cfg(feature = "server")]
async fn create_bitcart_payment(usd: f64, payment_id: String) -> Result<(String, String), String> {
    // Create a new reqwest client
    let client = Client::new();
    // Get the environment variables for the URL prefixes
    let base_url = std::env::var("BITCART_BASE_URL")
        .expect("BITCART_BASE_URL environment variable must be set.");
    let base_invoice_url = std::env::var("BITCART_BASE_INVOICE_URL")
        .expect("BITCART_BASE_INVOICE_URL environment variable must be set.");
    let store_id = std::env::var("BITCART_STORE_ID")
        .expect("BITCART_STORE_ID environment variable must be set.");
    let redirect_url = std::env::var("BITCART_REDIRECT_URL")
        .expect("BITCART_REDIRECT_URL environment variable must be set.");
    // Get the sensitive api token from environment variables
    let token = std::env::var("BITCART_AUTH_TOKEN")
        .expect("BITCART_AUTH_TOKEN environment variable must be set."); // Replace with your actual authorization token
    // Create an invoice reqwest which is sent in the request to the bitcart API
    let body = InvoiceRequest {
        store_id: store_id,
        price: usd,
        currency: "USD".to_string(),
        // Expiration for payment
        expiration: 180, // SHOULD BE 120-180 minutes, 10 is for for testing
        redirect_url: format!("{redirect_url}/{}", payment_id),
    };
    // Send the create invoice reqwest and map it to response
    let response = client
        .post(format!("{base_url}/invoices"))
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    // Check for any errors (this should be displayed in the frontend UI)
    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }
    // Map the json response to our rust-friendly struct
    let invoice: InvoiceResponse = response.json().await.map_err(|e| e.to_string())?;
    // Create a url where the invoice can be accessed using the invoice id
    let url = format!("{base_invoice_url}/i/{}", invoice.id);
    Ok((invoice.id, url))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentUStatus {
    Pending,
    Invalid,
    Expired,
    Complete,
}

impl PaymentUStatus {
    fn from_string(s: &str) -> Result<Self, String> {
        match s {
            "pending" => Ok(Self::Pending),
            "invalid" => Ok(Self::Invalid),
            "expired" => Ok(Self::Expired),
            "complete"|"paid" => Ok(Self::Complete),
            _ => Err(format!("Unknown payment status: {}", s)),
        }
    }
}

#[derive(Debug, Deserialize)]
struct InvoiceGetResponse {
    status: String,
}

#[cfg(feature = "server")]
async fn get_bitcart_payment_status(payment_id: &str) -> Result<PaymentUStatus, String> {
    // Create a new reqwest client
    let client = Client::new();
    // Get the environment variables for the URL prefixes
    let base_url = std::env::var("BITCART_BASE_URL")
        .expect("BITCART_BASE_URL environment variable must be set.");
    // Get the sensitive api token from environment variables
    let token = std::env::var("BITCART_AUTH_TOKEN")
        .expect("BITCART_AUTH_TOKEN environment variable must be set.");
    // Construct the URL for the GET request
    let url = format!("{base_url}/invoices/{}", payment_id);
    // Send the GET request to retrieve the invoice
    let response = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    // Check for any errors (this should be displayed in the frontend UI)
    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }
    // Map the json response to our rust-friendly struct
    let invoice: InvoiceGetResponse = response.json().await.map_err(|e| e.to_string())?;

    tracing::info!("{:?}", invoice);

    // Map the status string to the enum
    PaymentUStatus::from_string(&invoice.status)
}

#[cfg(feature = "server")]
async fn delete_bitcart_payment(payment_id: &str) -> Result<(), String> {
    // Create a new reqwest client
    let client = Client::new();
    // Get the environment variables
    let base_url = std::env::var("BITCART_BASE_URL")
        .expect("BITCART_BASE_URL environment variable must be set.");
    let token = std::env::var("BITCART_AUTH_TOKEN")
        .expect("BITCART_AUTH_TOKEN environment variable must be set.");

    // Construct the URL for the DELETE request
    let url = format!("{base_url}/invoices/{}", payment_id);
    // Send the DELETE request to retrieve the invoice
    let response = client
        .delete(url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // Check for any errors
    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    // Map the status string to the enum
    Ok(())
}

#[cfg(feature = "server")]
pub struct OrderCodeGenerator;

#[cfg(feature = "server")]
impl OrderCodeGenerator {
    /// Generates a random 6-character order code with format: ABC123
    /// First 3 characters are uppercase letters (A-Z)
    /// Last 3 characters are numbers (0-9)
    pub fn generate() -> String {
        let mut rng = rand::thread_rng();
        let mut code = String::with_capacity(6);

        // Generate 3 random uppercase letters
        for _ in 0..3 {
            let letter = rng.gen_range(b'A'..=b'Z') as char;
            code.push(letter);
        }

        // Generate 3 random numbers
        for _ in 0..3 {
            let number = rng.gen_range(0..=9);
            code.push_str(&number.to_string());
        }

        code
    }

    /// Validate that a code matches the expected format
    pub fn is_valid_format(code: &str) -> bool {
        if code.len() != 6 {
            return false;
        }

        let chars: Vec<char> = code.chars().collect();

        // Check first 3 are uppercase letters
        for i in 0..3 {
            if !chars[i].is_ascii_uppercase() {
                return false;
            }
        }

        // Check last 3 are digits
        for i in 3..6 {
            if !chars[i].is_ascii_digit() {
                return false;
            }
        }

        true
    }
}
