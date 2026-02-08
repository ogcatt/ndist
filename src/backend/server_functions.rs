// src/backend/server_functions.rs (don't remove this line)

// IMPORTANT: Admin server routes must be prefixed with admin_

use dioxus::prelude::*;

// Allow payments access with server_functions::payments
pub use super::payments;

#[cfg(feature = "server")]
use super::db::get_db;

#[cfg(feature = "server")]
use super::shipping_calculations::{
    calculate_shipping_cost, calculate_shipping_cost_with_preorder_surcharge,
    round_up_to_nearest_quarter,
};

#[cfg(feature = "server")]
use super::email::{EmailService, EmailType};

use super::auth::Manager;

#[cfg(feature = "server")]
use entity::{
    self, address, basket_items, blog_posts, customer_baskets, discounts, manager_sessions,
    managers, order, order_item, payment, pre_order, product_variant_stock_item_relations,
    product_variants, products, sea_orm_active_enums, stock_active_reduce,
    stock_backorder_active_reduce, stock_batches, stock_item_relations, stock_items,
    stock_preorder_active_reduce,
}; // Import the products and auth modules from your entity crate
#[cfg(feature = "server")]
use sea_orm::{
    self, ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QuerySelect,
    TransactionTrait,
}; // Required for database operations

use chrono::NaiveDateTime;

#[cfg(feature = "server")]
use chrono::{DateTime, Duration, Utc};
#[cfg(feature = "server")]
use futures;
#[cfg(feature = "server")]
use http::header::{COOKIE, SET_COOKIE};
#[cfg(feature = "server")]
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
#[cfg(feature = "server")]
use reqwest;
#[cfg(feature = "server")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "server")]
use std::fs;
#[cfg(feature = "server")]
use std::io::Read;
#[cfg(feature = "server")]
use supabase_auth::models::AuthClient;
#[cfg(feature = "server")]
use tokio;
#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
use axum::http::header::SET_COOKIE as AXUM_SET_COOKIE;
#[cfg(feature = "server")]
use axum::response::ResponseParts;

use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::front_entities::*;

#[cfg(feature = "server")]
use super::entity_conversions;

#[cfg(feature = "server")]
use super::media_optimise::*;

// Helper trait to convert sea_orm::DbErr to ServerFnError
// Since we can't implement From<sea_orm::DbErr> for ServerFnError (orphan rule),
// we provide a helper method instead
#[cfg(feature = "server")]
pub trait DbErrExt<T> {
    fn map_db_err(self) -> Result<T, ServerFnError>;
}

#[cfg(feature = "server")]
impl<T> DbErrExt<T> for Result<T, sea_orm::DbErr> {
    fn map_db_err(self) -> Result<T, ServerFnError> {
        self.map_err(|e| {
            let anyhow_err: anyhow::Error = e.into();
            anyhow_err.into()
        })
    }
}

// Implement From trait for validator::ValidationErrors to allow using ? in server functions
// Commented out as validator crate is not currently in dependencies
// #[cfg(feature = "server")]
// impl From<validator::ValidationErrors> for ServerFnError {
//     fn from(err: validator::ValidationErrors) -> Self {
//         ServerFnError::new(format!("Validation error: {}", err))
//     }
// }

/*
#[server]
pub async fn test_db() -> Result<usize, ServerFnError> {
    let db = get_db();

    let db_products: Vec<products::Model> = products::Entity::find().all(db.await).await.map_db_err()?;
    tracing::info!("{:#?}", db_products);


    let public_products: Vec<Product> = entity_conversions::convert_products_batch(db_products);
    // This should be manipulated so that private/unlisted products don't show
    tracing::info!("{:#?}", public_products);

    Ok(public_products.len())
}
*/

#[server]
pub async fn get_policies() -> Result<(String, String), ServerFnError> {
    // Read tos.md
    let tos_content = include_str!("../data/md/tos.md");
    let tos_html = entity_conversions::markdown_to_html(&tos_content);

    // Read privacy.md
    let privacy_content = include_str!("../data/md/privacy.md");
    let privacy_html = entity_conversions::markdown_to_html(&privacy_content);

    Ok((tos_html, privacy_html))
}

#[server]
pub async fn get_products() -> Result<Vec<Product>, ServerFnError> {
    let db = get_db().await; // Await the db once here

    // Make all three requests concurrently
    let (products_with_variants_result, variant_relations_result, stock_qty_results_result) = tokio::join!(
        // Product data request
        products::Entity::find()
            .filter(
                products::Column::Visibility.eq(sea_orm_active_enums::ProductVisibility::Public)
            )
            .find_with_related(product_variants::Entity)
            .all(db), // Use reference to db
        // Variant relations request
        async {
            product_variant_stock_item_relations::Entity::find()
                //.filter(product_variant_stock_item_relations::Column::ProductVariantId.is_in(variant_ids))
                .all(db) // Use reference to db
                .await
        },
        // Stock quantities request
        get_stock_quantities_for_stock_items(None)
    );

    // Handle results
    let products_with_variants = products_with_variants_result.map_db_err()?;
    let variant_relations = variant_relations_result.map_db_err()?;
    let stock_qty_results = stock_qty_results_result?;

    // Separate products and create contexts
    let (product_models, contexts): (Vec<_>, Vec<_>) = products_with_variants
        .into_iter()
        .map(|(product_model, variant_models)| {
            let converted_variants = if !variant_models.is_empty() {
                Some(entity_conversions::convert_product_variants(variant_models))
            } else {
                None
            };

            let context = entity_conversions::ProductConversionContext {
                product_phase: ProductPhase::default(),
                variants: converted_variants,
            };

            (product_model, context)
        })
        .unzip();

    // Batch convert
    let mut products =
        entity_conversions::convert_products_batch_with_context(product_models, contexts)?;

    products = calculate_variant_stock_quantities(products, variant_relations, stock_qty_results);

    return Ok(products);
}

#[server]
pub async fn admin_get_products(convert_markdown: bool) -> Result<Vec<Product>, ServerFnError> {
    let db = get_db();

    // Load products with their related variants
    let products_with_variants: Vec<(products::Model, Vec<product_variants::Model>)> =
        products::Entity::find()
            .find_with_related(product_variants::Entity)
            .all(db.await)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Separate products and create contexts
    let (product_models, contexts): (Vec<_>, Vec<_>) = products_with_variants
        .into_iter()
        .map(|(product_model, variant_models)| {
            let converted_variants = if !variant_models.is_empty() {
                Some(entity_conversions::convert_product_variants(variant_models))
            } else {
                None
            };

            let context = entity_conversions::ProductConversionContext {
                product_phase: ProductPhase::default(),
                variants: converted_variants,
            };

            (product_model, context)
        })
        .unzip();

    // Batch convert
    let products = if convert_markdown {
        entity_conversions::convert_products_batch_with_context(product_models, contexts)?
    } else {
        entity_conversions::convert_products_batch_with_context_no_markdown(
            product_models,
            contexts,
        )?
    };

    return Ok(products);
}

#[server]
pub async fn admin_get_stock_items() -> Result<Vec<StockItem>, ServerFnError> {
    let db = get_db();

    // Load inventory items and related data
    let stock_items_models: Vec<stock_items::Model> =
        stock_items::Entity::find().all(db.await).await.map_db_err()?;

    // Get stock quantities for all stock items
    let stock_quantities = get_stock_quantities_for_stock_items(Some(
        stock_items_models
            .iter()
            .map(|model| model.id.clone())
            .collect(),
    ))
    .await?;

    tracing::info!("{:#?}", &stock_quantities);

    // Convert stock items with quantities
    let stock_items = entity_conversions::convert_stock_items_batch_with_quantities(
        stock_items_models,
        &stock_quantities,
    );

    Ok(stock_items)
}

#[server]
pub async fn admin_get_stock_batches() -> Result<Vec<StockBatch>, ServerFnError> {
    let db = get_db();

    // Load inventory batches and related data
    let stock_batches_models: Vec<stock_batches::Model> =
        stock_batches::Entity::find().all(db.await).await.map_db_err()?;

    let stock_batches = entity_conversions::convert_stock_batches_batch(stock_batches_models);

    Ok(stock_batches)
}

#[server]
pub async fn admin_get_stock_item_relations() -> Result<Vec<StockItemRelation>, ServerFnError> {
    let db = get_db();

    // Load inventory batches and related data
    let stock_item_relations_models: Vec<stock_item_relations::Model> =
        stock_item_relations::Entity::find().all(db.await).await.map_db_err()?;

    let stock_item_relations =
        entity_conversions::convert_stock_item_relations_batch(stock_item_relations_models);

    Ok(stock_item_relations)
}

#[server]
pub async fn admin_get_product_variant_stock_item_relations()
-> Result<Vec<ProductVariantStockItemRelation>, ServerFnError> {
    let db = get_db();

    // Load inventory batches and related data
    let stock_relations_models: Vec<product_variant_stock_item_relations::Model> =
        product_variant_stock_item_relations::Entity::find()
            .all(db.await)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    let stock_relations =
        entity_conversions::convert_variant_stock_item_relations_batch(stock_relations_models);

    Ok(stock_relations)
}

#[server]
pub async fn admin_get_discounts() -> Result<Vec<Discount>, ServerFnError> {
    let db = get_db();

    // Load inventory batches and related data
    let discount_models: Vec<discounts::Model> = discounts::Entity::find().all(db.await).await.map_db_err()?;

    let discounts_final = entity_conversions::convert_discounts_batch(discount_models);

    Ok(discounts_final)
}

#[server]
pub async fn admin_get_pre_or_back_order_reduces(
    stock_item_id: String,
) -> Result<
    (
        Vec<BackOrPreOrderActiveReduce>,
        Vec<BackOrPreOrderActiveReduce>,
    ),
    ServerFnError,
> {
    let db = get_db().await;

    // Query both entities concurrently using tokio::join with stock_item_id filter
    let (back_order_result, pre_order_result) = tokio::join!(
        stock_backorder_active_reduce::Entity::find()
            .filter(stock_backorder_active_reduce::Column::StockItemId.eq(&stock_item_id))
            .all(db),
        stock_preorder_active_reduce::Entity::find()
            .filter(stock_preorder_active_reduce::Column::StockItemId.eq(&stock_item_id))
            .all(db)
    );

    // Handle results and convert to the shared struct
    let back_order_models = back_order_result.map_db_err()?;
    let pre_order_models = pre_order_result.map_db_err()?;

    let back_orders: Vec<BackOrPreOrderActiveReduce> = back_order_models
        .into_iter()
        .map(|model| BackOrPreOrderActiveReduce {
            id: model.id,
            order_id: model.order_id,
            order_item_id: model.order_item_id,
            stock_item_id: model.stock_item_id,
            stock_unit: StockUnit::from_seaorm(model.stock_unit),
            reduction_quantity: model.reduction_quantity,
            active: model.active,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
        .collect();

    let pre_orders: Vec<BackOrPreOrderActiveReduce> = pre_order_models
        .into_iter()
        .map(|model| BackOrPreOrderActiveReduce {
            id: model.id,
            order_id: model.order_id,
            order_item_id: model.order_item_id,
            stock_item_id: model.stock_item_id,
            stock_unit: StockUnit::from_seaorm(model.stock_unit),
            reduction_quantity: model.reduction_quantity,
            active: model.active,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
        .collect();

    Ok((back_orders, pre_orders))
}

#[server]
pub async fn admin_get_blog_post(id: String) -> Result<BlogPost, ServerFnError> {
    let db = get_db();

    // Load inventory batches and related data
    let blog_post_model: blog_posts::Model = blog_posts::Entity::find()
        .filter(blog_posts::Column::Id.eq(&id))
        .one(db.await)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .expect("Could not get blog post model when fetching singleton");

    let blog_post_final = entity_conversions::convert_blog_post(blog_post_model, false)?;

    Ok(blog_post_final)
}

#[server]
pub async fn admin_get_blog_posts() -> Result<Vec<BlogPost>, ServerFnError> {
    let db = get_db();

    // Load inventory batches and related data
    let blog_post_models: Vec<blog_posts::Model> = blog_posts::Entity::find().all(db.await).await.map_db_err()?;
    let blog_posts_final = entity_conversions::convert_blog_posts_batch(blog_post_models, false)?;

    Ok(blog_posts_final)
}

#[server]
pub async fn get_blog_posts() -> Result<Vec<BlogPost>, ServerFnError> {
    let db = get_db();

    // Load inventory batches and related data
    let blog_post_models: Vec<blog_posts::Model> = blog_posts::Entity::find().all(db.await).await.map_db_err()?;
    let blog_posts_final = entity_conversions::convert_blog_posts_batch(blog_post_models, true)?;

    Ok(blog_posts_final)
}

// START OF BASKET FUNCTIONS

// Custom error types
#[derive(Debug, Clone)]
pub enum CartError {
    DatabaseError(String),
    ValidationError(String),
    NotFound(String),
}

impl std::fmt::Display for CartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CartError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            CartError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            CartError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for CartError {}

// Implement From<CartError> for ServerFnError
#[cfg(feature = "server")]
impl From<CartError> for ServerFnError {
    fn from(err: CartError) -> Self {
        ServerFnError::new(err.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum BasketError {
    DatabaseError(String),
    CookieError(String),
    CreationError(String),
}

impl std::fmt::Display for BasketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BasketError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            BasketError::CookieError(msg) => write!(f, "Cookie error: {}", msg),
            BasketError::CreationError(msg) => write!(f, "Creation error: {}", msg),
        }
    }
}

impl std::error::Error for BasketError {}

// Implement From<BasketError> for ServerFnError
#[cfg(feature = "server")]
impl From<BasketError> for ServerFnError {
    fn from(err: BasketError) -> Self {
        ServerFnError::new(err.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum CookieError {
    ExtractionError(String),
    ParsingError(String),
    SettingError(String),
    NoContext,
}

impl std::fmt::Display for CookieError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CookieError::ExtractionError(msg) => write!(f, "Extraction error: {}", msg),
            CookieError::ParsingError(msg) => write!(f, "Parsing error: {}", msg),
            CookieError::SettingError(msg) => write!(f, "Setting error: {}", msg),
            CookieError::NoContext => write!(f, "No server context available"),
        }
    }
}

#[cfg(feature = "server")]
impl std::error::Error for CookieError {}

// Implement From<CookieError> for ServerFnError
#[cfg(feature = "server")]
impl From<CookieError> for ServerFnError {
    fn from(err: CookieError) -> Self {
        ServerFnError::new(err.to_string())
    }
}

#[cfg(feature = "server")]
impl From<StockCalculationError> for ServerFnError {
    fn from(err: StockCalculationError) -> Self {
        ServerFnError::new(err.to_string())
    }
}

// Type alias for easier reading
#[cfg(feature = "server")]
pub type CustomerBasketItems = Vec<CustomerBasketItem>;

// Convert SeaORM errors to custom errors
#[cfg(feature = "server")]
impl From<sea_orm::DbErr> for CartError {
    fn from(err: sea_orm::DbErr) -> Self {
        CartError::DatabaseError(err.to_string())
    }
}

#[cfg(feature = "server")]
impl From<sea_orm::DbErr> for BasketError {
    fn from(err: sea_orm::DbErr) -> Self {
        BasketError::DatabaseError(err.to_string())
    }
}

#[cfg(feature = "server")]
impl From<sea_orm::DbErr> for CookieError {
    fn from(err: sea_orm::DbErr) -> Self {
        CookieError::ExtractionError(err.to_string())
    }
}

#[server]
pub async fn get_or_create_basket() -> Result<CustomerBasket, ServerFnError> {
    let db = get_db().await;

    // Check for existing basket cookie
    if let Some(basket_id) = get_basket_cookie().await? {
        // Make all requests concurrently
        let (
            basket_result,
            basket_items_result,
            products_with_variants_result,
            variant_relations_result,
            stock_qty_results_result,
            discounts_result,
        ) = tokio::join!(
            // Get existing basket
            customer_baskets::Entity::find_by_id(&basket_id).one(db),
            // Get basket items for this basket
            basket_items::Entity::find()
                .filter(basket_items::Column::BasketId.eq(&basket_id))
                .all(db),
            // Product data request
            products::Entity::find()
                .filter(
                    products::Column::Visibility
                        .eq(sea_orm_active_enums::ProductVisibility::Public)
                )
                .find_with_related(product_variants::Entity)
                .all(db),
            // Variant relations request
            product_variant_stock_item_relations::Entity::find().all(db),
            // Stock quantities request
            get_stock_quantities_for_stock_items(None),
            // Get discounts
            discounts::Entity::find().all(db)
        );

        // Handle results using map_db_err to convert sea_orm::DbErr to ServerFnError
        let basket = basket_result.map_db_err()?;
        let mut basket_items = basket_items_result.map_db_err()?;
        let products_with_variants = products_with_variants_result.map_db_err()?;
        let variant_relations = variant_relations_result.map_db_err()?;
        let stock_qty_results = stock_qty_results_result?;
        let discounts = discounts_result.map_db_err()?;

        if let Some(basket_model) = basket {
            // Check and update cart if there are items
            if !basket_items.is_empty() {
                let (updated_items, _check_results) = check_cart(
                    basket_items,
                    products_with_variants.clone(),
                    variant_relations,
                    stock_qty_results,
                )
                .await?;

                basket_items = updated_items;
            }

            // Convert to frontend types
            let mut customer_basket = CustomerBasket::from(basket_model.clone());
            customer_basket.items = if basket_items.is_empty() {
                None
            } else {
                Some(entity_conversions::convert_basket_items_batch(
                    basket_items.clone(),
                ))
            };

            // Calculate shipping results if country_code exists and we have items
            customer_basket.shipping_results = if let Some(country_code) =
                &basket_model.country_code
            {
                if !basket_items.is_empty() {
                    calculate_shipping_results(country_code, &basket_items, &products_with_variants)
                        .await
                } else {
                    Some(Vec::new())
                }
            } else {
                None
            };

            if let Some(discount_code) = &basket_model.discount_code
                && basket_model.country_code.is_some()
            {
                let variants: Vec<product_variants::Model> = products_with_variants
                    .into_iter()
                    .flat_map(|(_, variants)| variants)
                    .collect();

                let check = check_discount(
                    discount_code.clone(),
                    basket_model.country_code.clone(),
                    Some(discounts.clone()),
                    basket_items,
                    variants,
                )
                .await;

                match check {
                    Ok(_) => {
                        let discount_match = discounts
                            .iter()
                            .find(|d| d.code == *discount_code)
                            .expect("Failed discount match (some logic error)");
                        customer_basket.discount = Some(BasketDiscountData {
                            discount_type: DiscountType::from_seaorm(discount_match.discount_type.clone()),
                            discount_percentage: match DiscountType::from_seaorm(discount_match.discount_type.clone()) {
                                DiscountType::Percentage | DiscountType::PercentageOnShipping => { Some(discount_match.discount_percentage.expect("Discount percentage type does not have discount_percentage")) },
                                DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => { None }
                            },
                            discount_amount_left: match DiscountType::from_seaorm(discount_match.discount_type.clone()) {
                                DiscountType::Percentage | DiscountType::PercentageOnShipping => { None },
                                DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => { Some( discount_match.discount_amount.expect("Discount amount type does not have discount amount")-discount_match.amount_used.unwrap_or(0.0) ) }
                            },
                            discount_auto_apply: discount_match.auto_apply
                        })
                    }
                    Err(validation_error) => {
                        // Remove the discount code from the basket
                        let mut basket_update: customer_baskets::ActiveModel = basket_model.into();
                        basket_update.discount_code = ActiveValue::Set(None);

                        basket_update.update(db).await.map_db_err()?;
                    }
                }
            }

            return Ok(customer_basket);
        }
    }

    let basket = create_new_basket().await?;
    Ok(basket)
}

/// Calculate shipping results for the basket
#[cfg(feature = "server")]
async fn calculate_shipping_results(
    country_code: &str,
    basket_items: &[basket_items::Model],
    products_with_variants: &[(products::Model, Vec<product_variants::Model>)],
) -> Option<Vec<ShippingResult>> {
    // Create lookup maps
    let variant_map: std::collections::HashMap<String, &product_variants::Model> =
        products_with_variants
            .iter()
            .flat_map(|(_, variants)| variants.iter())
            .map(|variant| (variant.id.clone(), variant))
            .collect();

    let product_map: std::collections::HashMap<String, &products::Model> = products_with_variants
        .iter()
        .map(|(product, _)| (product.id.clone(), product))
        .collect();

    let variant_to_product_map: std::collections::HashMap<String, String> = products_with_variants
        .iter()
        .flat_map(|(product, variants)| {
            variants
                .iter()
                .map(move |variant| (variant.id.clone(), product.id.clone()))
        })
        .collect();

    // Helper function to calculate basket cost
    let calculate_basket_cost = |items: &[basket_items::Model]| -> f64 {
        items
            .iter()
            .map(|item| {
                if let Some(variant) = variant_map.get(&item.variant_id) {
                    variant.price_standard_usd * item.quantity as f64
                } else {
                    0.0
                }
            })
            .sum()
    };

    // Separate basket items into pre-order and regular items
    let mut preorder_items = Vec::new();
    let mut regular_items = Vec::new();

    for item in basket_items {
        let is_preorder = if let Some(product_id) = variant_to_product_map.get(&item.variant_id) {
            if let Some(product) = product_map.get(product_id) {
                product.pre_order
            } else {
                false
            }
        } else {
            false
        };

        if is_preorder {
            preorder_items.push(item.clone());
        } else {
            regular_items.push(item.clone());
        }
    }

    // Calculate weights and costs
    let regular_weight = if !regular_items.is_empty() {
        calculate_total_cart_weight(&regular_items, products_with_variants, false)
    } else {
        0.0
    };

    let preorder_weight = if !preorder_items.is_empty() {
        calculate_total_cart_weight(&preorder_items, products_with_variants, false)
    } else {
        0.0
    };

    let regular_cost = calculate_basket_cost(&regular_items);
    let preorder_cost = calculate_basket_cost(&preorder_items);

    match (regular_items.is_empty(), preorder_items.is_empty()) {
        (true, true) => {
            // No items
            Some(Vec::new())
        }
        (false, true) => {
            // Only regular items
            if let Some(shipping_quote) =
                calculate_shipping_cost(country_code, regular_weight as u32, regular_cost)
            {
                Some(shipping_quote.available_options)
            } else {
                Some(Vec::new())
            }
        }
        (true, false) => {
            // Only pre-order items
            if let Some(shipping_quote) = calculate_shipping_cost_with_preorder_surcharge(
                country_code,
                preorder_weight as u32,
                preorder_cost,
            ) {
                Some(shipping_quote.available_options)
            } else {
                Some(Vec::new())
            }
        }
        (false, false) => {
            // Both regular and pre-order items - combine shipping costs
            let regular_quote =
                calculate_shipping_cost(country_code, regular_weight as u32, regular_cost);
            let preorder_quote = calculate_shipping_cost_with_preorder_surcharge(
                country_code,
                preorder_weight as u32,
                preorder_cost,
            );

            match (regular_quote, preorder_quote) {
                (Some(regular), Some(preorder)) => {
                    let mut combined_options = Vec::new();

                    // Find all unique shipping options available in both quotes
                    let mut option_types = std::collections::HashSet::new();
                    for result in &regular.available_options {
                        option_types.insert(&result.option);
                    }
                    for result in &preorder.available_options {
                        option_types.insert(&result.option);
                    }

                    // For each option type, combine costs if available in both
                    for option_type in option_types {
                        let regular_result = regular
                            .available_options
                            .iter()
                            .find(|r| &r.option == option_type);
                        let preorder_result = preorder
                            .available_options
                            .iter()
                            .find(|r| &r.option == option_type);

                        if let (Some(regular_r), Some(preorder_r)) =
                            (regular_result, preorder_result)
                        {
                            combined_options.push(ShippingResult {
                                option: option_type.clone(),
                                cost_usd: round_up_to_nearest_quarter(
                                    regular_r.cost_usd + preorder_r.cost_usd,
                                ),
                                estimated_days: regular_r.estimated_days.clone(), // Use regular item timing
                            });
                        }
                    }

                    Some(combined_options)
                }
                _ => Some(Vec::new()),
            }
        }
    }
}

/// Calculate the total weight of all items in the cart
#[cfg(feature = "server")]
pub fn calculate_total_cart_weight(
    basket_items: &[basket_items::Model],
    products_with_variants: &[(products::Model, Vec<product_variants::Model>)],
    exclude_pre_orders: bool,
) -> f64 {
    // Create lookup maps for efficiency
    let variant_map: std::collections::HashMap<String, &product_variants::Model> =
        products_with_variants
            .iter()
            .flat_map(|(_, variants)| variants.iter())
            .map(|variant| (variant.id.clone(), variant))
            .collect();
    let product_map: std::collections::HashMap<String, &products::Model> = products_with_variants
        .iter()
        .map(|(product, _)| (product.id.clone(), product))
        .collect();

    let mut total_weight_grams = 0.0f64;

    for basket_item in basket_items {
        // Get the variant
        if let Some(variant) = variant_map.get(&basket_item.variant_id) {
            // Get the parent product to check pre_order status
            if let Some(product) = product_map.get(&variant.product_id) {
                // Skip this item if we're excluding pre-orders and this product is a pre-order
                if exclude_pre_orders && product.pre_order {
                    continue;
                }
            }

            // Try to get weight from variant first
            let item_weight_grams = if let Some(variant_weight) = variant.weight {
                variant_weight // Weight is already in grams
            } else if let Some(product) = product_map.get(&variant.product_id) {
                // Try to get weight from parent product
                if let Some(product_weight) = product.weight {
                    product_weight // Weight is already in grams
                } else {
                    80.0 // Default to 80 grams
                }
            } else {
                80.0 // Default to 80 grams
            };

            // Multiply by quantity and add to total
            total_weight_grams += item_weight_grams * (basket_item.quantity as f64);
        } else {
            // Variant not found, use default weight
            // Note: We can't check pre_order status here since we don't have the variant/product
            // You might want to handle this case differently based on your requirements
            total_weight_grams += 80.0 * (basket_item.quantity as f64);
        }
    }

    // Apply packaging weight: (weight * 1.05) + 30g
    total_weight_grams = (total_weight_grams * 1.05) + 30.0;
    total_weight_grams
}

#[cfg(feature = "server")]
pub async fn check_cart(
    mut basket_items: Vec<basket_items::Model>,
    products_with_variants: Vec<(products::Model, Vec<product_variants::Model>)>,
    variant_relations: Vec<product_variant_stock_item_relations::Model>,
    stock_quantities: Vec<StockQuantityResult>,
    //discounts: discounts::Model
) -> Result<(Vec<basket_items::Model>, Vec<CheckCartResult>), ServerFnError> {
    let db = get_db().await;
    let mut results = Vec::new();
    let mut items_to_remove = Vec::new();

    // Create lookup maps for efficiency
    let stock_map: std::collections::HashMap<String, &StockQuantityResult> = stock_quantities
        .iter()
        .map(|sq| (sq.stock_item_id.clone(), sq))
        .collect();

    let variant_map: std::collections::HashMap<String, &product_variants::Model> =
        products_with_variants
            .iter()
            .flat_map(|(_, variants)| variants.iter())
            .map(|variant| (variant.id.clone(), variant))
            .collect();

    // Create product lookup map to check back_order and pre_order flags
    let product_map: std::collections::HashMap<String, &products::Model> = products_with_variants
        .iter()
        .map(|(product, _)| (product.id.clone(), product))
        .collect();

    // Create variant-to-product mapping
    let variant_to_product_map: std::collections::HashMap<String, String> = products_with_variants
        .iter()
        .flat_map(|(product, variants)| {
            variants
                .iter()
                .map(move |variant| (variant.id.clone(), product.id.clone()))
        })
        .collect();

    // Group relations by variant ID for easier lookup
    let variant_relations_map: std::collections::HashMap<
        String,
        Vec<&product_variant_stock_item_relations::Model>,
    > = {
        let mut map = std::collections::HashMap::new();
        for relation in &variant_relations {
            map.entry(relation.product_variant_id.clone())
                .or_insert_with(Vec::new)
                .push(relation);
        }
        map
    };

    for (index, basket_item) in basket_items.iter_mut().enumerate() {
        let variant_id = &basket_item.variant_id;

        // Get the variant and its stock item relations
        if let (Some(_variant), Some(relations)) = (
            variant_map.get(variant_id),
            variant_relations_map.get(variant_id),
        ) {
            // Check if this variant belongs to a back_order or pre_order product
            let is_special_order = if let Some(product_id) = variant_to_product_map.get(variant_id)
            {
                if let Some(product) = product_map.get(product_id) {
                    product.back_order || product.pre_order
                } else {
                    false
                }
            } else {
                false
            };

            if is_special_order {
                // For back_order or pre_order products, no stock restrictions apply
                results.push(CheckCartResult::Complete);
            } else {
                // Apply normal stock checking logic for regular products
                let mut max_possible_quantity = i32::MAX;
                let mut has_zero_stock = false;

                // Check each stock item relation for this variant
                for relation in relations {
                    if let Some(stock_result) = stock_map.get(&relation.stock_item_id) {
                        let required_per_item = &relation.quantity;
                        let available_stock = &stock_result.total_stock_quantity;

                        // Check if there's any stock available
                        if available_stock.is_zero() {
                            has_zero_stock = true;
                            break;
                        }

                        // Calculate maximum possible quantity for this stock item
                        let available_f64 = available_stock.to_f64();
                        let required_f64 = required_per_item;

                        if *required_f64 > 0.0 {
                            let possible_quantity = (available_f64 / required_f64).floor() as i32;
                            max_possible_quantity = max_possible_quantity.min(possible_quantity);
                        }
                    }
                }

                // Determine action based on stock availability
                if has_zero_stock || max_possible_quantity == 0 {
                    // Remove item - no stock available
                    items_to_remove.push(index);
                    results.push(CheckCartResult::Removed);
                } else if max_possible_quantity < basket_item.quantity {
                    // Reduce quantity
                    basket_item.quantity = max_possible_quantity;
                    results.push(CheckCartResult::Reduced);
                } else {
                    // No changes needed
                    results.push(CheckCartResult::Complete);
                }
            }
        } else {
            // Variant not found or no relations - remove item
            items_to_remove.push(index);
            results.push(CheckCartResult::Error(format!(
                "Variant {} not found or has no stock relations",
                variant_id
            )));
        }
    }

    // Remove items that need to be removed (in reverse order to maintain indices)
    for &index in items_to_remove.iter().rev() {
        let removed_item = basket_items.remove(index);

        // Delete from database
        basket_items::Entity::delete_by_id(&removed_item.id)
            .exec(db)
            .await
            .map_db_err()?;
    }

    // Update quantities in database for modified items
    for (basket_item, result) in basket_items.iter().zip(results.iter()) {
        if matches!(result, CheckCartResult::Reduced) {
            let mut active_model: basket_items::ActiveModel = basket_item.clone().into();
            active_model.quantity = ActiveValue::Set(basket_item.quantity);

            basket_items::Entity::update(active_model).exec(db).await.map_db_err()?;
        }
    }

    Ok((basket_items, results))
}

#[cfg(feature = "server")]
pub async fn create_new_basket() -> Result<CustomerBasket, BasketError> {
    let db = get_db().await;
    let now = chrono::Utc::now().naive_utc();
    let basket_id = Uuid::new_v4().to_string();

    let new_basket = customer_baskets::ActiveModel {
        id: ActiveValue::Set(basket_id.clone()),
        customer_id: ActiveValue::NotSet, // Set to None as requested
        country_code: ActiveValue::NotSet,
        discount_code: ActiveValue::NotSet,
        shipping_option: ActiveValue::NotSet,
        locked: ActiveValue::Set(false),
        payment_id: ActiveValue::Set(None),
        payment_failed_at: ActiveValue::NotSet,
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    let basket_model = customer_baskets::Entity::insert(new_basket)
        .exec_with_returning(db)
        .await?;

    // Set cookie for future requests
    set_basket_cookie(&basket_id)
        .await
        .map_err(|e| BasketError::CookieError(e.to_string()))?;

    let mut customer_basket = CustomerBasket::from(basket_model);
    customer_basket.items = None; // New basket has no items

    Ok(customer_basket)
}

#[cfg(feature = "server")]
async fn get_basket_cookie() -> Result<Option<String>, CookieError> {
    // Use the server function instead of direct context access
    match get_basket_id_from_cookie().await {
        Ok(basket_id) => Ok(basket_id),
        Err(e) => Err(CookieError::ExtractionError(e.to_string())),
    }
}

#[cfg(feature = "server")]
async fn set_basket_cookie(basket_id: &str) -> Result<(), CookieError> {
    // Use the server function instead of direct context access
    set_basket_id_cookie(basket_id.to_string())
        .await
        .map_err(|e| CookieError::SettingError(e.to_string()))
}

// Helper function to parse cookie strings
#[cfg(feature = "server")]
fn parse_cookies(cookie_str: &str) -> HashMap<String, String> {
    let mut cookies = HashMap::new();

    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        if let Some((key, value)) = cookie.split_once('=') {
            cookies.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    cookies
}

#[cfg(feature = "server")]
fn parse_cookies_from_string(cookie_str: &str) -> HashMap<String, String> {
    let mut cookies = HashMap::new();

    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        if let Some((key, value)) = cookie.split_once('=') {
            cookies.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    cookies
}

#[cfg(feature = "server")]
async fn get_basket_id_from_cookie() -> Result<Option<String>, CookieError> {
    use dioxus::fullstack::FullstackContext;
    
    // Access request headers via Dioxus server context
    let server_ctx = FullstackContext::current()
        .expect("Server context should be available");
    let request_parts = server_ctx.parts_mut();

    // Try both lowercase and capitalized forms for compatibility
    let cookie_header = request_parts
        .headers
        .get("cookie")
        .or_else(|| request_parts.headers.get("Cookie"));

    if let Some(cookie_value) = cookie_header {
        let cookie_str = cookie_value
            .to_str()
            .map_err(|e| CookieError::ExtractionError(format!("Invalid cookie header: {}", e)))?;

        let cookies = parse_cookies_from_string(cookie_str);
        return Ok(cookies.get("customer_basket_id").cloned());
    }

    Ok(None)
}

#[cfg(feature = "server")]
async fn set_basket_id_cookie(basket_id: String) -> Result<(), CookieError> {
    use axum::http::{HeaderValue, header::SET_COOKIE};
    use dioxus::fullstack::FullstackContext;

    let cookie_value = format!(
        "customer_basket_id={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        basket_id,
        60 * 60 * 24 * 30
    );

    // Access response headers via Dioxus server context
    let server_ctx = FullstackContext::current()
        .expect("Server context should be available");
    let header_value = HeaderValue::from_str(&cookie_value)
        .map_err(|e| CookieError::SettingError(format!("Invalid cookie value: {}", e)))?;

    // Multiple Set-Cookie headers are allowed; append instead of overwrite
    server_ctx.add_response_header(SET_COOKIE, header_value);

    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddToBasketResponse {
    pub status: String, // "Complete" | "Reduced" | "Removed" | "NotFound" | "Invalid"
    pub basket: CustomerBasket,
}

#[server]
pub async fn get_basket() -> Result<CustomerBasket, ServerFnError> {
    get_or_create_basket().await
}

#[server]
pub async fn check_payment(payment_id: String) -> Result<PaymentShortInfo, ServerFnError> {
    return payments::check_payment(&payment_id).await;
}

#[server]
pub async fn delete_payment(payment_id: String) -> Result<(), ServerFnError> {
    return payments::cancel_payment(&payment_id, false).await;
}

#[server]
pub async fn get_short_order(order_id: String) -> Result<OrderShortInfo, ServerFnError> {
    let db = get_db().await;

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let payments_fut = payment::Entity::find()
        .filter(payment::Column::OrderId.eq(&order_id))
        .all(db);

    let order_item_fut = order_item::Entity::find()
        .filter(order_item::Column::OrderId.eq(&order_id))
        .all(db);

    let stock_backorder_active_reduces_fut = stock_backorder_active_reduce::Entity::find()
        .filter(stock_backorder_active_reduce::Column::OrderId.eq(&order_id))
        .all(db);

    let pre_order_fut = pre_order::Entity::find()
        .filter(pre_order::Column::ParentOrderId.eq(&order_id))
        .all(db);

    let (
        order_res,
        payments_res,
        order_item_res,
        stock_backorder_active_reduces_res,
        pre_order_res,
    ) = tokio::join!(
        order_fut,
        payments_fut,
        order_item_fut,
        stock_backorder_active_reduces_fut,
        pre_order_fut
    );

    let order_items = order_item_res.map_db_err()?;
    let payments = payments_res.map_db_err()?;
    let stock_backorder_active_reduces = stock_backorder_active_reduces_res.map_db_err()?;
    let pre_orders_entity = pre_order_res.map_db_err()?;

    let order_mod =
        order_res.map_db_err()?.unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));

    let mut order_short_items: Vec<OrderShortItem> = vec![];

    let paid_at: Option<NaiveDateTime> = payments
        .into_iter()
        .min_by_key(|payment| payment.created_at)
        .map(|payment| payment.created_at);

    for item in order_items {
        order_short_items.push({
            OrderShortItem {
                id: item.id,
                product_variant_id: item.product_variant_id,
                quantity: item.quantity,
                price_usd: item.price_usd,
                product_title: item.product_title,
                variant_name: item.variant_name,
                pre_order_on_purchase: item.pre_order_on_purchase,
            }
        });
    }

    let mut short_pre_orders: Vec<ShortPreOrder> = vec![];

    // Link preorder reduces
    for po in &pre_orders_entity {
        if po.parent_order_id == order_id {
            short_pre_orders.push(ShortPreOrder::from(po.clone()))
        }
    }

    let order_short_info = OrderShortInfo {
        ref_code: order_mod.ref_code,
        shipping_option: ShippingOption::from_seaorm(order_mod.shipping_option),
        billing_country: order_mod.billing_country,
        tracking_url: order_mod.tracking_url,
        total_amount_usd: order_mod.total_amount_usd,
        paid: if order_mod.status == sea_orm_active_enums::OrderStatus::Paid {
            true
        } else {
            false
        },
        created_at: Some(order_mod.created_at),
        paid_at: paid_at,
        prepared_at: order_mod.prepared_at,
        fulfilled_at: order_mod.fulfilled_at,
        items: order_short_items,
        pre_orders: short_pre_orders,
        contains_back_order: if stock_backorder_active_reduces.len() > 0 {
            true
        } else {
            false
        },
    };

    Ok(order_short_info)
}

#[server]
pub async fn admin_get_orders(get_expired: bool) -> Result<Vec<OrderInfo>, ServerFnError> {
    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        panic!("Unauthorized");
    }

    let db = get_db().await;

    // GET DATABASE ENTIRES:

    // Retrieve orders:
    let orders_fut = if get_expired {
        order::Entity::find()
            .filter(order::Column::Status.is_not_in(vec![
                sea_orm_active_enums::OrderStatus::Pending,
                sea_orm_active_enums::OrderStatus::Processing,
            ]))
            .all(db)
    } else {
        // Update this when there is an expired value
        order::Entity::find()
            .filter(order::Column::Status.is_not_in(vec![
                sea_orm_active_enums::OrderStatus::Pending,
                sea_orm_active_enums::OrderStatus::Processing,
            ]))
            .all(db)
    };

    // Retrieve order addresses:
    let addresses_fut = address::Entity::find().all(db);

    // Retrieve order items:
    let order_items_fut = order_item::Entity::find().all(db);

    // Retrieve payments:
    let payments_fut = payment::Entity::find().all(db);

    // Retrieve active backorder reduces:
    let backorder_reduces_fut = stock_backorder_active_reduce::Entity::find()
        .filter(stock_backorder_active_reduce::Column::Active.eq(true))
        .all(db);

    // Retrieve active preorder reduces:
    let preorder_reduces_fut = stock_preorder_active_reduce::Entity::find()
        .filter(stock_preorder_active_reduce::Column::Active.eq(true))
        .all(db);

    // Retrieve payments:
    let pre_orders_fut = pre_order::Entity::find().all(db);

    // Collect all database requests
    let (
        orders_res,
        addresses_res,
        order_items_res,
        payments_res,
        backorder_reduces_res,
        preorder_reduces_res,
        pre_orders_res,
    ) = tokio::join!(
        orders_fut,
        addresses_fut,
        order_items_fut,
        payments_fut,
        backorder_reduces_fut,
        preorder_reduces_fut,
        pre_orders_fut
    );

    let orders_entity = orders_res.map_db_err()?;
    let addresses_entity = addresses_res.map_db_err()?;
    let order_items_entity = order_items_res.map_db_err()?;
    let payments_entity = payments_res.map_db_err()?;
    let backorder_reduces_entity = backorder_reduces_res.map_db_err()?;
    let preorder_reduces_entity = preorder_reduces_res.map_db_err()?;
    let pre_orders_entity = pre_orders_res.map_db_err()?;

    // Create OrderInfos
    let mut order_infos: Vec<OrderInfo> = vec![];

    for order in orders_entity {
        let mut payments: Vec<PaymentInfo> = vec![];
        let mut items: Vec<OrderShortItem> = vec![];
        let mut address: Option<CustomerShippingInfo> = None;
        let mut backorder_reduces: Vec<BackOrPreOrderActiveReduce> = vec![];
        let mut preorder_reduces: Vec<BackOrPreOrderActiveReduce> = vec![];
        let mut pre_orders: Vec<PreOrder> = vec![];

        for p in &payments_entity {
            if p.order_id == Some(order.id.clone()) {
                payments.push(PaymentInfo {
                    id: p.id.clone(),
                    method: p.method.clone(),
                    processor_ref: p.processor_ref.clone(),
                    processor_url: p.processor_url.clone(),
                    status: PaymentStatus::from_seaorm(p.status.clone()),
                    amount_usd: p.amount_usd,
                    paid_at: p.paid_at.clone(),
                    created_at: p.created_at.clone(),
                    updated_at: p.updated_at.clone(),
                });
            }
        }

        for o in &order_items_entity {
            if o.order_id == order.id {
                items.push(OrderShortItem {
                    id: o.id.clone(),
                    product_variant_id: o.product_variant_id.clone(),
                    quantity: o.quantity,
                    price_usd: o.price_usd,
                    product_title: o.product_title.clone(),
                    variant_name: o.variant_name.clone(),
                    pre_order_on_purchase: o.pre_order_on_purchase,
                });
            }
        }

        for a in &addresses_entity {
            if a.order_id == order.id {
                address = Some(CustomerShippingInfo {
                    phone: a.phone.clone(),
                    email_list: order.add_to_email_list,
                    first_name: a.first_name.clone(),
                    last_name: a.last_name.clone(),
                    company: a.company.clone(),
                    address_line_1: a.address_line_1.clone(),
                    address_line_2: a.address_line_2.clone(),
                    post_code: a.postal_code.clone(),
                    province: a.province.clone(),
                    city: a.city.clone(),
                    country: Some(a.country.clone()),
                });
            }
        }

        // Link backorder reduces
        for br in &backorder_reduces_entity {
            if br.order_id == order.id {
                backorder_reduces.push(BackOrPreOrderActiveReduce {
                    id: br.id.clone(),
                    order_id: br.order_id.clone(),
                    order_item_id: br.order_item_id.clone(),
                    stock_item_id: br.stock_item_id.clone(),
                    stock_unit: StockUnit::from_seaorm(br.stock_unit.clone()),
                    reduction_quantity: br.reduction_quantity,
                    active: br.active,
                    created_at: br.created_at.clone(),
                    updated_at: br.updated_at.clone(),
                });
            }
        }

        // Link preorder reduces
        for pr in &preorder_reduces_entity {
            if pr.order_id == order.id {
                preorder_reduces.push(BackOrPreOrderActiveReduce {
                    id: pr.id.clone(),
                    order_id: pr.order_id.clone(),
                    order_item_id: pr.order_item_id.clone(),
                    stock_item_id: pr.stock_item_id.clone(),
                    stock_unit: StockUnit::from_seaorm(pr.stock_unit.clone()),
                    reduction_quantity: pr.reduction_quantity,
                    active: pr.active,
                    created_at: pr.created_at.clone(),
                    updated_at: pr.updated_at.clone(),
                });
            }
        }

        // Link preorder reduces
        for po in &pre_orders_entity {
            if po.parent_order_id == order.id {
                pre_orders.push(PreOrder::from(po.clone()))
            }
        }

        order_infos.push(OrderInfo {
            id: order.id.clone(),
            ref_code: order.ref_code.clone(),
            customer_id: order.customer_id.clone(),
            customer_email: order.customer_email.clone(),
            add_to_email_list: order.add_to_email_list,
            billing_country: order.billing_country.clone(),
            shipping_option: ShippingOption::from_seaorm(order.shipping_option.clone()),
            subtotal_usd: order.subtotal_usd,
            shipping_usd: order.shipping_usd,
            order_weight: order.order_weight,
            refund_comment: order.refund_comment.clone(),
            status: OrderStatus::from_seaorm(order.status.clone()),
            fulfilled_at: order.fulfilled_at.clone(),
            cancelled_at: order.cancelled_at.clone(),
            refunded_at: order.refunded_at.clone(),
            prepared_at: order.prepared_at.clone(),
            tracking_url: order.tracking_url.clone(),
            total_amount_usd: order.total_amount_usd,
            discount_id: order.discount_id.clone(),
            notes: order.notes.clone(),
            address: address,
            items: items,
            backorder_reduces: backorder_reduces,
            preorder_reduces: preorder_reduces,
            pre_orders: pre_orders,
            payments: payments,
            created_at: order.created_at.clone(),
            updated_at: order.updated_at.clone(),
        });
    }

    tracing::info!("{:#?}", order_infos);

    Ok(order_infos)
}

#[server]
pub async fn admin_set_order_status(
    order_id: String,
    status: OrderStatus,
) -> Result<(), ServerFnError> {
    match status {
        OrderStatus::Fulfilled => {
            panic!("Can't use set order status to fulfill an order")
        }
        OrderStatus::Cancelled => {
            panic!("Can't use set order status to cancel an order")
        }
        _ => {
            let db = get_db().await;

            let order = order::Entity::find()
                .filter(order::Column::Id.eq(order_id))
                .one(db)
                .await
                .map_db_err()?
                .expect("Could not get order model when trying to update status");

            // Convert to ActiveModel and update the status
            let mut order_active: order::ActiveModel = order.into();
            order_active.status = ActiveValue::Set(status.to_seaorm());

            // Save the updated order
            order::Entity::update(order_active).exec(db).await.map_db_err()?;
        }
    }

    Ok(())
}

#[server]
pub async fn admin_set_order_prepared(order_id: String) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let order = order::Entity::find()
        .filter(order::Column::Id.eq(order_id))
        .one(db)
        .await
        .map_db_err()?
        .expect("Could not get order model when trying to update status");

    let now = Utc::now().naive_utc();

    // Convert to ActiveModel and update the status
    let mut order_active: order::ActiveModel = order.into();
    order_active.prepared_at = ActiveValue::Set(Some(now));

    // Save the updated order
    order::Entity::update(order_active).exec(db).await.map_db_err()?;

    Ok(())
}

#[server]
pub async fn admin_set_preorder_prepared(
    order_item_id: String,
    parent_order_id: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;
    let now = Utc::now().naive_utc();

    // First, get the parent order to extract needed values
    let parent_order = order::Entity::find()
        .filter(order::Column::Id.eq(&parent_order_id))
        .one(db)
        .await
        .map_db_err()?
        .ok_or_else(|| ServerFnError::new("Parent order not found"))?;

    // Get the order item with its associated product variant
    let order_item_with_variant = order_item::Entity::find()
        .filter(order_item::Column::Id.eq(&order_item_id))
        .find_also_related(product_variants::Entity)
        .one(db)
        .await
        .map_db_err()?
        .ok_or_else(|| ServerFnError::new("Order item not found"))?;

    let (order_item, variant) = order_item_with_variant;

    // Calculate item weight from variant, defaulting to 80 * quantity if variant weight is None
    let item_weight = match variant {
        Some(variant) => match variant.weight {
            Some(weight) => weight * order_item.quantity as f64,
            None => (80.0 * order_item.quantity as f64 + 30.0),
        },
        None => (80.0 * order_item.quantity as f64 + 30.0),
    };

    // Create new pre-order entry
    let new_preorder = pre_order::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4().to_string()),
        order_item_id: ActiveValue::Set(order_item_id),
        parent_order_id: ActiveValue::Set(parent_order_id),
        add_to_email_list: ActiveValue::Set(parent_order.add_to_email_list),
        shipping_option: ActiveValue::Set(parent_order.shipping_option),
        pre_order_weight: ActiveValue::Set(item_weight),
        fulfilled_at: ActiveValue::Set(None),
        prepared_at: ActiveValue::Set(Some(now)),
        tracking_url: ActiveValue::Set(None),
        notes: ActiveValue::Set(None),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    // Insert the new pre-order
    pre_order::Entity::insert(new_preorder).exec(db).await.map_db_err()?;

    Ok(())
}

#[server]
pub async fn admin_set_order_fulfilled(
    order_id: String,
    tracking_url: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let (address_res, order_res) = tokio::join!(address_fut, order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod =
        order_res.map_db_err()?.unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));

    let now = Utc::now().naive_utc();

    // Convert to ActiveModel and update the status
    let mut order_active: order::ActiveModel = order_mod.clone().into();
    order_active.status = ActiveValue::Set(OrderStatus::Fulfilled.to_seaorm());
    order_active.fulfilled_at = ActiveValue::Set(Some(now));
    order_active.tracking_url = ActiveValue::Set(Some(tracking_url.clone()));

    // Save the updated order
    order::Entity::update(order_active).exec(db).await.map_db_err()?;

    // Send the fulfillment email

    let email_service = EmailService::new()?;

    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::TrackingConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
        tracking_url: tracking_url.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
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

#[server]
pub async fn admin_set_pre_order_fulfilled(
    order_id: String,
    pre_order_id: String,
    tracking_url: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let pre_order_fut = pre_order::Entity::find()
        .filter(pre_order::Column::Id.eq(&pre_order_id))
        .one(db);

    let (address_res, order_res, pre_order_res) =
        tokio::join!(address_fut, order_fut, pre_order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod =
        order_res.map_db_err()?.unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));
    let pre_order_mod = pre_order_res.map_db_err()?
        .unwrap_or_else(|| panic!("Could not get pre order by pre_order_id (does it exist?)..."));

    let now = Utc::now().naive_utc();

    // Convert to ActiveModel and update the status
    let mut pre_order_active: pre_order::ActiveModel = pre_order_mod.clone().into();
    pre_order_active.fulfilled_at = ActiveValue::Set(Some(now));
    pre_order_active.tracking_url = ActiveValue::Set(Some(tracking_url.clone()));

    // Save the updated order
    pre_order::Entity::update(pre_order_active).exec(db).await.map_db_err()?;

    // Send the fulfillment email

    let email_service = EmailService::new()?;

    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::PreOrderTrackingConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
        tracking_url: tracking_url.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
    };

    match email_service
        .send_email(
            &customer_email,
            &customer_name,
            email_type,
            pre_order_mod.add_to_email_list,
        )
        .await
    {
        Ok(()) => tracing::info!("success sending pre-order confirmation email"),
        Err(e) => tracing::info!("{e:?}"),
    }

    Ok(())
}

#[server]
pub async fn admin_express_fulfilled_notracking(order_id: String) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let (address_res, order_res) = tokio::join!(address_fut, order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod =
        order_res.map_db_err()?.unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));

    let now = Utc::now().naive_utc();

    // Convert to ActiveModel and update the status
    let mut order_active: order::ActiveModel = order_mod.clone().into();
    order_active.status = ActiveValue::Set(OrderStatus::Fulfilled.to_seaorm());
    order_active.fulfilled_at = ActiveValue::Set(Some(now));

    // Save the updated order
    order::Entity::update(order_active).exec(db).await.map_db_err()?;

    // Send the fulfillment email

    let email_service = EmailService::new()?;

    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::ExpressDispatchConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
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

#[server]
pub async fn admin_express_pre_order_fulfilled_notracking(
    order_id: String,
    pre_order_id: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let pre_order_fut = pre_order::Entity::find()
        .filter(pre_order::Column::Id.eq(&pre_order_id))
        .one(db);

    let (address_res, order_res, pre_order_res) =
        tokio::join!(address_fut, order_fut, pre_order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod =
        order_res.map_db_err()?.unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));
    let pre_order_mod = pre_order_res.map_db_err()?
        .unwrap_or_else(|| panic!("Could not get pre order by pre_order_id (does it exist?)..."));

    let now = Utc::now().naive_utc();

    // Convert to ActiveModel and update the status
    let mut pre_order_active: pre_order::ActiveModel = pre_order_mod.clone().into();
    pre_order_active.fulfilled_at = ActiveValue::Set(Some(now));

    // Save the updated order
    pre_order::Entity::update(pre_order_active).exec(db).await.map_db_err()?;

    // Send the fulfillment email

    let email_service = EmailService::new()?;

    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::ExpressPreOrderDispatchConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
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
        Ok(()) => tracing::info!("success sending express notracking pre-order confirmation email"),
        Err(e) => tracing::info!("{e:?}"),
    }

    Ok(())
}

#[server]
pub async fn admin_express_order_send_tracking(
    order_id: String,
    tracking_url: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let (address_res, order_res) = tokio::join!(address_fut, order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod =
        order_res.map_db_err()?.unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));

    let now = Utc::now().naive_utc();

    // Convert to ActiveModel and update the status
    let mut order_active: order::ActiveModel = order_mod.clone().into();
    order_active.tracking_url = ActiveValue::Set(Some(tracking_url.clone()));

    // Save the updated order
    order::Entity::update(order_active).exec(db).await.map_db_err()?;

    // Send the fulfillment email

    let email_service = EmailService::new()?;
    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::ExpressTrackingConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
        tracking_url: tracking_url.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
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

#[server]
pub async fn admin_express_pre_order_send_tracking(
    order_id: String,
    pre_order_id: String,
    tracking_url: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let pre_order_fut = pre_order::Entity::find()
        .filter(pre_order::Column::Id.eq(&pre_order_id))
        .one(db);

    let (address_res, order_res, pre_order_res) =
        tokio::join!(address_fut, order_fut, pre_order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod =
        order_res.map_db_err()?.unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));
    let pre_order_mod = pre_order_res.map_db_err()?
        .unwrap_or_else(|| panic!("Could not get pre order by pre_order_id (does it exist?)..."));

    let now = Utc::now().naive_utc();

    // Convert to ActiveModel and update the status
    let mut pre_order_active: pre_order::ActiveModel = pre_order_mod.clone().into();
    pre_order_active.tracking_url = ActiveValue::Set(Some(tracking_url.clone()));

    // Save the updated order
    pre_order::Entity::update(pre_order_active).exec(db).await.map_db_err()?;

    // Send the fulfillment email

    let email_service = EmailService::new()?;
    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::ExpressPreOrderTrackingConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
        tracking_url: tracking_url.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
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

#[server]
pub async fn add_or_update_basket_item(
    variant_id: String,
    requested_quantity: i32,
) -> Result<AddToBasketResponse, ServerFnError> {
    use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
    };
    use uuid::Uuid;

    let db = get_db().await;

    // Refuse invalid input immediately, but still provide a synced basket back
    if requested_quantity <= 0 {
        let basket = get_or_create_basket().await?;
        return Ok(AddToBasketResponse {
            status: "Invalid".to_string(),
            basket,
        });
    }

    // Ensure we have a basket from cookie or create one
    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();

    // THROW IF BASKET IS LOCKED
    if basket.locked {
        panic!("Basket locked")
    }

    // Load live product variant relations and stock to compute max possible
    let (
        products_with_variants_res,
        variant_relations_res,
        stock_quantities_res,
        auto_apply_discounts_res,
        current_basket_items_res,
    ) = tokio::join!(
        products::Entity::find()
            .filter(
                products::Column::Visibility.eq(sea_orm_active_enums::ProductVisibility::Public)
            )
            .find_with_related(product_variants::Entity)
            .all(db),
        product_variant_stock_item_relations::Entity::find().all(db),
        get_stock_quantities_for_stock_items(None),
        discounts::Entity::find()
            .filter(discounts::Column::Active.eq(true))
            .filter(discounts::Column::AutoApply.eq(true))
            .all(db),
        basket_items::Entity::find()
            .filter(basket_items::Column::BasketId.eq(&basket_id))
            .all(db),
    );

    let products_with_variants = products_with_variants_res.map_db_err()?;
    let variant_relations = variant_relations_res.map_db_err()?;
    let stock_quantities = stock_quantities_res?;
    let auto_apply_discounts = auto_apply_discounts_res.map_db_err()?;
    let current_basket_items = current_basket_items_res.map_db_err()?;

    // Build stock map
    let stock_map: std::collections::HashMap<String, StockQuantityResult> = stock_quantities
        .into_iter()
        .map(|sq| (sq.stock_item_id.clone(), sq))
        .collect();

    // Find variant existence and its parent product
    let mut found_variant: Option<product_variants::Model> = None;
    let mut parent_product: Option<products::Model> = None;
    for (product, vlist) in &products_with_variants {
        if let Some(v) = vlist.iter().find(|v| v.id == variant_id) {
            found_variant = Some(v.clone());
            parent_product = Some(product.clone());
            break;
        }
    }

    if found_variant.is_none() {
        // Variant no longer exists
        let basket = get_or_create_basket().await?;
        return Ok(AddToBasketResponse {
            status: "NotFound".to_string(),
            basket,
        });
    }

    let parent_product = parent_product.expect("Parent product should exist if variant exists");
    let is_back_order = parent_product.back_order;
    let is_pre_order = parent_product.pre_order;

    // Group relations by variant id
    let mut relations_by_variant: std::collections::HashMap<
        String,
        Vec<product_variant_stock_item_relations::Model>,
    > = std::collections::HashMap::new();
    for rel in variant_relations {
        relations_by_variant
            .entry(rel.product_variant_id.clone())
            .or_default()
            .push(rel);
    }

    // Compute max possible quantity from stock (skip if back_order is true)
    let max_possible = if is_back_order || is_pre_order {
        // For back orders, stock doesn't limit quantity
        i32::MAX
    } else if let Some(rels) = relations_by_variant.get(&variant_id) {
        let mut max_q = i32::MAX;
        let mut zero = false;
        for rel in rels {
            if let Some(stock) = stock_map.get(&rel.stock_item_id) {
                if stock.total_stock_quantity.is_zero() {
                    zero = true;
                    break;
                }
                let available = stock.total_stock_quantity.to_f64();
                let per_unit_needed = rel.quantity;
                if per_unit_needed > 0.0 {
                    let possible = (available / per_unit_needed).floor() as i32;
                    if possible < max_q {
                        max_q = possible;
                    }
                }
            } else {
                // Missing stock item => treat as zero
                zero = true;
                break;
            }
        }
        if zero { 0 } else { max_q.max(0) }
    } else {
        // If there are no relations and it's not a back order, we cannot compute stock; treat as unavailable
        0
    };

    // Policy limit per item (align with frontend)
    let max_per_item = 12i32;

    // Fetch existing basket item for this variant if any
    let existing_item_opt = current_basket_items
        .iter()
        .find(|item| item.variant_id == variant_id)
        .cloned();

    // Calculate desired final quantity (existing + request)
    let existing_qty = existing_item_opt.as_ref().map(|x| x.quantity).unwrap_or(0);
    let desired_total = (existing_qty + requested_quantity).clamp(0, max_per_item);

    // Determine final allowed quantity (clamped by stock unless back_order is true)
    let allowed_total = if is_back_order || is_pre_order {
        // For back orders, only limit by max_per_item
        desired_total
    } else {
        // For regular items, limit by both stock and max_per_item
        desired_total.min(max_possible)
    };

    // Start transaction and upsert/update
    let txn = db.begin().await.map_db_err()?;

    let status;
    if allowed_total <= 0 {
        // Remove item from basket if exists
        if let Some(existing_item) = existing_item_opt {
            basket_items::Entity::delete_by_id(existing_item.id.clone())
                .exec(&txn)
                .await.map_db_err()?;
        }
        status = "Removed".to_string();
    } else {
        // If exists: update quantity; else: create
        if let Some(existing_item) = existing_item_opt {
            // If allowed_total differs, update; if equal, keep as complete
            if existing_item.quantity != allowed_total {
                let mut am: basket_items::ActiveModel = existing_item.into();
                am.quantity = ActiveValue::Set(allowed_total);
                basket_items::Entity::update(am).exec(&txn).await.map_db_err()?;
                status = if allowed_total < desired_total {
                    "Reduced".to_string()
                } else {
                    "Complete".to_string()
                };
            } else {
                status = "Complete".to_string();
            }
        } else {
            // Create new
            let new_id = Uuid::new_v4().to_string();
            let new_bi = basket_items::ActiveModel {
                id: ActiveValue::Set(new_id),
                basket_id: ActiveValue::Set(basket_id.clone()),
                variant_id: ActiveValue::Set(variant_id.clone()),
                quantity: ActiveValue::Set(allowed_total),
            };
            basket_items::Entity::insert(new_bi).exec(&txn).await.map_db_err()?;
            status = if allowed_total < desired_total {
                "Reduced".to_string()
            } else {
                "Complete".to_string()
            };
        }
    }

    // Check for auto-apply discounts after basket modification
    let updated_basket_model = customer_baskets::Entity::find_by_id(&basket_id)
        .one(&txn)
        .await.map_db_err()?
        .expect("Basket should exist");

    // Only check auto-apply discounts if we have a country code
    if let Some(country_code) = &updated_basket_model.country_code {
        if !auto_apply_discounts.is_empty() {
            // Get updated basket items after modification
            let updated_basket_items = basket_items::Entity::find()
                .filter(basket_items::Column::BasketId.eq(&basket_id))
                .all(&txn)
                .await.map_db_err()?;;

            // Convert products_with_variants to flat variant list
            let variants: Vec<product_variants::Model> = products_with_variants
                .into_iter()
                .flat_map(|(_, variants)| variants)
                .collect();

            // Check each auto-apply discount and find the best one
            let mut best_discount: Option<discounts::Model> = None;
            let mut best_discount_value: f64 = 0.0;

            for discount in auto_apply_discounts {
                tracing::info!("checking {}", discount.code.clone());
                // Check if this discount applies to the current basket
                let check_result = check_discount(
                    discount.code.clone(),
                    Some(country_code.clone()),
                    Some(vec![discount.clone()]),
                    updated_basket_items.clone(),
                    variants.clone(),
                )
                .await;

                if check_result.is_ok() {
                    // Calculate discount value for comparison
                    let discount_value =
                        calculate_discount_value(&discount, &updated_basket_items, &variants);

                    if discount_value > best_discount_value {
                        best_discount_value = discount_value;
                        best_discount = Some(discount);
                    }
                }
            }

            // Apply the best discount if found
            if let Some(best_discount) = best_discount {
                // Update the basket with the new discount code
                let mut basket_am: customer_baskets::ActiveModel = updated_basket_model.into();
                basket_am.discount_code = ActiveValue::Set(Some(best_discount.code));
                customer_baskets::Entity::update(basket_am)
                    .exec(&txn)
                    .await.map_db_err()?;
            }
        }
    }

    txn.commit().await.map_db_err()?;

    // Return updated basket back to client
    let updated_basket = get_or_create_basket().await?;
    Ok(AddToBasketResponse {
        status,
        basket: updated_basket,
    })
}

// Helper function to calculate the monetary value of a discount
#[cfg(feature = "server")]
fn calculate_discount_value(
    discount: &discounts::Model,
    basket_items: &[basket_items::Model],
    variants: &[product_variants::Model],
) -> f64 {
    use sea_orm_active_enums::DiscountType;

    // Calculate total basket value
    let total_basket_value: f64 = basket_items
        .iter()
        .filter_map(|item| {
            variants
                .iter()
                .find(|v| v.id == item.variant_id)
                .map(|v| item.quantity as f64 * v.price_standard_usd)
        })
        .sum();

    match discount.discount_type {
        DiscountType::Percentage => {
            if let Some(percentage) = discount.discount_percentage {
                total_basket_value * (percentage / 100.0)
            } else {
                0.0
            }
        }
        DiscountType::FixedAmount => discount
            .discount_amount
            .unwrap_or(0.0)
            .min(total_basket_value),
        DiscountType::PercentageOnShipping => {
            // For shipping discounts, we'd need shipping cost calculation
            // For now, return a lower priority value
            0.1
        }
        DiscountType::FixedAmountOnShipping => {
            // For shipping discounts, we'd need shipping cost calculation
            // For now, return a lower priority value
            0.1
        }
    }
}

#[server]
pub async fn update_basket_country(country_code: String) -> Result<CustomerBasket, ServerFnError> {
    use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
    let db = get_db().await;
    // Ensure we have a basket from cookie or create one
    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();

    // THROW IF BASKET IS LOCKED
    if basket.locked {
        panic!("Basket locked")
    }

    // Find the basket record in the database
    let basket_entity = customer_baskets::Entity::find()
        .filter(customer_baskets::Column::Id.eq(&basket_id))
        .one(db)
        .await.map_db_err()?;

    if let Some(basket_model) = basket_entity {
        // Update the country_code
        let mut basket_active: customer_baskets::ActiveModel = basket_model.into();
        basket_active.country_code = if country_code.is_empty() {
            ActiveValue::Set(None)
        } else {
            ActiveValue::Set(Some(country_code))
        };
        // Clear the shipping option when country is updated
        basket_active.shipping_option = ActiveValue::Set(None);

        // Save the updated basket
        customer_baskets::Entity::update(basket_active)
            .exec(db)
            .await.map_db_err()?;
    }
    // Return the updated basket
    get_or_create_basket().await
}

#[server]
pub async fn update_basket_shipping_option(
    shipping_option: ShippingOption,
) -> Result<CustomerBasket, ServerFnError> {
    use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

    let db = get_db().await;

    // Ensure we have a basket from cookie or create one
    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();

    // THROW IF BASKET IS LOCKED
    if basket.locked {
        panic!("Basket locked")
    }

    // Find the basket record in the database
    let basket_entity = customer_baskets::Entity::find()
        .filter(customer_baskets::Column::Id.eq(&basket_id))
        .one(db)
        .await.map_db_err()?;

    if let Some(basket_model) = basket_entity {
        // Update the shipping_option
        let mut basket_active: customer_baskets::ActiveModel = basket_model.into();
        basket_active.shipping_option = ActiveValue::Set(Some(shipping_option.to_seaorm()));

        // Save the updated basket
        customer_baskets::Entity::update(basket_active)
            .exec(db)
            .await.map_db_err()?;
    }

    // Return the updated basket
    get_or_create_basket().await
}

#[server]
pub async fn update_basket_discount(
    discount_code: String,
) -> Result<BasketUpdateResult, ServerFnError> {
    use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
    let db = get_db().await;

    // Ensure we have a basket from cookie or create one
    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();
    let discount_code = discount_code.to_uppercase();

    // THROW IF BASKET IS LOCKED
    if basket.locked {
        panic!("Basket locked")
    }

    let mut discount_error = None;

    // Find the basket record in the database
    let (basket_entity, basket_items_result, variants_result) = tokio::join!(
        customer_baskets::Entity::find()
            .filter(customer_baskets::Column::Id.eq(&basket_id))
            .one(db),
        basket_items::Entity::find()
            .filter(basket_items::Column::BasketId.eq(&basket_id))
            .all(db),
        product_variants::Entity::find().all(db)
    );

    let basket_entity = basket_entity.map_err(|e| {
        ServerFnError::new(format!("Database error: {}", e))
    })?;

    let basket_items = basket_items_result.map_err(|e| {
        ServerFnError::new(format!("Database error: {}", e))
    })?;

    let variants = variants_result.map_err(|e| {
        ServerFnError::new(format!("Database error: {}", e))
    })?;

    if let Some(basket_model) = basket_entity {
        // If discount code is not empty, validate it
        if !discount_code.is_empty() {
            // Check the validity of the discount code using the new check_discount function
            match check_discount(
                discount_code.clone(),
                basket_model.country_code.clone(),
                None,
                basket_items,
                variants,
            )
            .await
            {
                Ok(_) => {
                    // Discount is valid, proceed to update
                }
                Err(validation_error) => {
                    // Store the discount validation error to return with the result
                    discount_error = Some(validation_error);
                    // Don't update the basket if there's an error, but don't fail the entire operation
                    let final_basket = get_or_create_basket().await?;
                    return Ok(BasketUpdateResult {
                        basket: final_basket,
                        discount_error,
                    });
                }
            }
        }

        // Update the discount code on the basket entity
        let mut basket_active: customer_baskets::ActiveModel = basket_model.into();
        basket_active.discount_code = if discount_code.is_empty() {
            ActiveValue::Set(None)
        } else {
            ActiveValue::Set(Some(discount_code))
        };

        // Save the updated basket
        customer_baskets::Entity::update(basket_active)
            .exec(db)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Database error: {}", e))
            })?;
    }

    // Return the updated basket
    let final_basket = get_or_create_basket().await?;
    Ok(BasketUpdateResult {
        basket: final_basket,
        discount_error,
    })
}

#[server]
pub async fn set_basket_item_quantity(
    variant_id: String,
    target_quantity: i32,
) -> Result<AddToBasketResponse, ServerFnError> {
    use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
    };

    let db = get_db().await;

    // Ensure we have a basket from cookie or create one
    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();

    // THROW IF BASKET IS LOCKED
    if basket.locked {
        panic!("Basket locked")
    }

    // Load live product variant relations and stock to compute max possible
    let (
        products_with_variants_res,
        variant_relations_res,
        stock_quantities_res,
        auto_apply_discounts_res,
        current_basket_items_res,
    ) = tokio::join!(
        products::Entity::find()
            .filter(
                products::Column::Visibility.eq(sea_orm_active_enums::ProductVisibility::Public)
            )
            .find_with_related(product_variants::Entity)
            .all(db),
        product_variant_stock_item_relations::Entity::find().all(db),
        get_stock_quantities_for_stock_items(None),
        discounts::Entity::find()
            .filter(discounts::Column::Active.eq(true))
            .filter(discounts::Column::AutoApply.eq(true))
            .all(db),
        basket_items::Entity::find()
            .filter(basket_items::Column::BasketId.eq(&basket_id))
            .all(db),
    );

    let products_with_variants = products_with_variants_res.map_db_err()?;
    let variant_relations = variant_relations_res.map_db_err()?;
    let stock_quantities = stock_quantities_res?;
    let auto_apply_discounts = auto_apply_discounts_res.map_db_err()?;
    let current_basket_items = current_basket_items_res.map_db_err()?;

    // Build stock map
    let stock_map: std::collections::HashMap<String, StockQuantityResult> = stock_quantities
        .into_iter()
        .map(|sq| (sq.stock_item_id.clone(), sq))
        .collect();

    // Find variant existence and its parent product
    let mut found_variant: Option<product_variants::Model> = None;
    let mut parent_product: Option<products::Model> = None;
    for (product, vlist) in &products_with_variants {
        if let Some(v) = vlist.iter().find(|v| v.id == variant_id) {
            found_variant = Some(v.clone());
            parent_product = Some(product.clone());
            break;
        }
    }

    if found_variant.is_none() {
        // Variant no longer exists
        let basket = get_or_create_basket().await?;
        return Ok(AddToBasketResponse {
            status: "NotFound".to_string(),
            basket,
        });
    }

    let parent_product = parent_product.expect("Parent product should exist if variant exists");
    let is_back_order = parent_product.back_order;
    let is_pre_order = parent_product.pre_order;

    // Group relations by variant id
    let mut relations_by_variant: std::collections::HashMap<
        String,
        Vec<product_variant_stock_item_relations::Model>,
    > = std::collections::HashMap::new();
    for rel in variant_relations {
        relations_by_variant
            .entry(rel.product_variant_id.clone())
            .or_default()
            .push(rel);
    }

    // Compute max possible quantity from stock (skip if back_order or pre_order is true)
    let max_possible = if is_back_order || is_pre_order {
        // For back orders and pre orders, stock doesn't limit quantity
        i32::MAX
    } else if let Some(rels) = relations_by_variant.get(&variant_id) {
        let mut max_q = i32::MAX;
        let mut zero = false;
        for rel in rels {
            if let Some(stock) = stock_map.get(&rel.stock_item_id) {
                if stock.total_stock_quantity.is_zero() {
                    zero = true;
                    break;
                }
                let available = stock.total_stock_quantity.to_f64();
                let per_unit_needed = rel.quantity;
                if per_unit_needed > 0.0 {
                    let possible = (available / per_unit_needed).floor() as i32;
                    if possible < max_q {
                        max_q = possible;
                    }
                }
            } else {
                // Missing stock item => treat as zero
                zero = true;
                break;
            }
        }
        if zero { 0 } else { max_q.max(0) }
    } else {
        // If there are no relations and it's not a back order or pre order, we cannot compute stock; treat as unavailable
        0
    };

    // Policy limit per item (align with frontend)
    let max_per_item = 12i32;

    // Fetch existing basket item for this variant if any
    let existing_item_opt = current_basket_items
        .iter()
        .find(|item| item.variant_id == variant_id)
        .cloned();

    // Desired absolute quantity
    let desired_total = target_quantity.clamp(0, max_per_item);

    // Final allowed by stock
    let allowed_total = desired_total.min(max_possible);

    let txn = db.begin().await.map_db_err()?;

    let status;
    if allowed_total <= 0 {
        // Remove item from basket if exists
        if let Some(existing_item) = existing_item_opt {
            basket_items::Entity::delete_by_id(existing_item.id.clone())
                .exec(&txn)
                .await.map_db_err()?;
        }
        status = "Removed".to_string();
    } else {
        if let Some(existing_item) = existing_item_opt {
            if existing_item.quantity != allowed_total {
                let mut am: basket_items::ActiveModel = existing_item.into();
                am.quantity = ActiveValue::Set(allowed_total);
                basket_items::Entity::update(am).exec(&txn).await.map_db_err()?;
                status = if allowed_total < desired_total {
                    "Reduced".to_string()
                } else {
                    "Complete".to_string()
                };
            } else {
                status = "Complete".to_string();
            }
        } else {
            use uuid::Uuid;
            let new_id = Uuid::new_v4().to_string();
            let new_bi = basket_items::ActiveModel {
                id: ActiveValue::Set(new_id),
                basket_id: ActiveValue::Set(basket_id.clone()),
                variant_id: ActiveValue::Set(variant_id.clone()),
                quantity: ActiveValue::Set(allowed_total),
            };
            basket_items::Entity::insert(new_bi).exec(&txn).await.map_db_err()?;
            status = if allowed_total < desired_total {
                "Reduced".to_string()
            } else {
                "Complete".to_string()
            };
        }
    }

    // Check for auto-apply discounts after basket modification
    let updated_basket_model = customer_baskets::Entity::find_by_id(&basket_id)
        .one(&txn)
        .await.map_db_err()?
        .expect("Basket should exist");

    // Only check auto-apply discounts if we have a country code
    if let Some(country_code) = &updated_basket_model.country_code {
        if !auto_apply_discounts.is_empty() {
            // Get updated basket items after modification
            let updated_basket_items = basket_items::Entity::find()
                .filter(basket_items::Column::BasketId.eq(&basket_id))
                .all(&txn)
                .await.map_db_err()?;;

            // Convert products_with_variants to flat variant list
            let variants: Vec<product_variants::Model> = products_with_variants
                .into_iter()
                .flat_map(|(_, variants)| variants)
                .collect();

            // Check each auto-apply discount and find the best one
            let mut best_discount: Option<discounts::Model> = None;
            let mut best_discount_value: f64 = 0.0;

            for discount in auto_apply_discounts {
                // Check if this discount applies to the current basket
                let check_result = check_discount(
                    discount.code.clone(),
                    Some(country_code.clone()),
                    Some(vec![discount.clone()]),
                    updated_basket_items.clone(),
                    variants.clone(),
                )
                .await;

                if check_result.is_ok() {
                    // Calculate discount value for comparison
                    let discount_value =
                        calculate_discount_value(&discount, &updated_basket_items, &variants);

                    if discount_value > best_discount_value {
                        best_discount_value = discount_value;
                        best_discount = Some(discount);
                    }
                }
            }

            // Apply the best discount if found
            if let Some(best_discount) = best_discount {
                // Update the basket with the new discount code
                let mut basket_am: customer_baskets::ActiveModel = updated_basket_model.into();
                basket_am.discount_code = ActiveValue::Set(Some(best_discount.code));
                customer_baskets::Entity::update(basket_am)
                    .exec(&txn)
                    .await.map_db_err()?;
            }
        }
    }

    txn.commit().await.map_db_err()?;

    // Return updated basket
    let updated_basket = get_or_create_basket().await?;
    Ok(AddToBasketResponse {
        status,
        basket: updated_basket,
    })
}

// Response type for successful discount validation
#[cfg(feature = "server")]
#[derive(Debug, Clone, PartialEq)]
pub struct DiscountValidationResponse {
    pub discount: discounts::Model,
    pub is_valid: bool,
}

#[cfg(feature = "server")]
pub async fn check_discount(
    discount_code: String,
    country_code: Option<String>,
    discounts_data: Option<Vec<discounts::Model>>,
    basket_items_data: Vec<basket_items::Model>,
    product_variants_data: Vec<product_variants::Model>,
) -> Result<DiscountValidationResponse, DiscountValidationError> {
    use sea_orm_active_enums::DiscountType;
    // Get the discount either from provided data or database
    let discount = if let Some(discounts) = discounts_data {
        // Search through provided discounts data
        discounts
            .into_iter()
            .find(|d| d.code == discount_code)
            .ok_or(DiscountValidationError::DiscountNotFound)?
    } else {
        // Query database for the discount
        let db = get_db().await;
        discounts::Entity::find()
            .filter(discounts::Column::Code.eq(&discount_code))
            .one(db)
            .await
            .map_err(|_| DiscountValidationError::DatabaseError)?
            .ok_or(DiscountValidationError::DiscountNotFound)?
    };

    // Check if discount is active
    if !discount.active {
        return Err(DiscountValidationError::DiscountInactive);
    }

    // Check expiration date
    let now = Utc::now().naive_utc();
    if let Some(expire_at) = discount.expire_at {
        if expire_at < now {
            return Err(DiscountValidationError::DiscountExpired);
        }
    }

    // Check maximum uses (factors in active reduce quantity for open payments)
    if let Some(max_uses) = discount.maximum_uses {
        if (discount.discount_used + discount.active_reduce_quantity) >= max_uses {
            return Err(DiscountValidationError::MaximumUsesExceeded);
        }
    }

    // Check amount limits for FixedAmount and FixedAmountOnShipping types
    match discount.discount_type {
        DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => {
            if let (Some(discount_amount), Some(amount_used)) =
                (discount.discount_amount, discount.amount_used)
            {
                if amount_used >= discount_amount {
                    return Err(DiscountValidationError::AmountExceeded);
                }
            }
        }
        _ => {} // No amount check needed for other discount types
    }

    // Check country restrictions
    if let Some(valid_countries) = &discount.valid_countries {
        if !valid_countries.is_empty() {
            match country_code {
                None => {
                    return Err(DiscountValidationError::CountryRequired);
                }
                Some(country) => {
                    if !valid_countries.contains(&country) {
                        return Err(DiscountValidationError::InvalidCountry);
                    }
                }
            }
        }
    }

    // Check valid after X products requirement
    if let Some(min_products) = discount.valid_after_x_products {
        let total_product_count: i32 = basket_items_data.iter().map(|item| item.quantity).sum();
        if total_product_count <= min_products {
            return Err(DiscountValidationError::MinimumProductsRequired);
        }
    }

    // Check valid after X total cost requirement
    if let Some(min_total) = discount.valid_after_x_total {
        let total_cost: f64 = basket_items_data
            .iter()
            .filter_map(|basket_item| {
                // Find the corresponding product variant
                product_variants_data
                    .iter()
                    .find(|variant| variant.id == basket_item.variant_id)
                    .map(|variant| {
                        // Calculate cost: quantity * price
                        basket_item.quantity as f64 * variant.price_standard_usd
                    })
            })
            .sum();

        if total_cost < min_total {
            return Err(DiscountValidationError::MinimumTotalRequired);
        }
    }

    // If all checks pass, return success
    Ok(DiscountValidationResponse {
        discount,
        is_valid: true,
    })
}

// END OF BASKET FUNCTIONS

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub success: bool,
    pub url: Option<String>,
    pub message: String,
}

#[server]
pub async fn admin_upload_thumbnails(
    file_data: Vec<u8>,
    file_name: String,
    content_type: String,
) -> Result<UploadResponse, ServerFnError> {
    // Check if user is authenticated and has admin permissions
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(UploadResponse {
            success: false,
            url: None,
            message: "Unauthorized".to_string(),
        });
    }

    upload_image_locally(file_data, file_name, content_type).await
    //upload_image_to_supabase(file_data, file_name, content_type).await
}

#[server]
pub async fn admin_upload_private_thumbnails(
    file_data: Vec<u8>,
    file_name: String,
    content_type: String,
) -> Result<UploadResponse, ServerFnError> {
    // Check if user is authenticated and has admin permissions
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(UploadResponse {
            success: false,
            url: None,
            message: "Unauthorized".to_string(),
        });
    }

    upload_private_image_locally(file_data, file_name, content_type).await
}

#[cfg(feature = "server")]
async fn upload_image_locally(
    file_data: Vec<u8>,
    file_name: String,
    content_type: String,
) -> Result<UploadResponse, ServerFnError> {
    use std::fs;
    use std::path::Path;

    // Convert image to AVIF format (keeping your existing conversion logic)
    let (processed_data, final_content_type) =
        convert_image_to_avif(file_data, &content_type).await?;

    // Generate random filename using UUID without hyphens
    let random_name = Uuid::new_v4().simple().to_string();
    let unique_filename = format!("{}.avif", random_name);

    // Use environment-aware path for uploads
    let upload_base = if std::env::var("RAILWAY_ENVIRONMENT").is_ok() {
        "/app/assets/uploads"
    } else {
        "assets/uploads"
    };

    let assets_dir = Path::new(upload_base).join("products");

    // Create directory if it doesn't exist
    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir)
            .map_err(|e| ServerFnError::new(format!("Failed to create directory: {}", e)))?;
    }

    // Full file path
    let file_path = assets_dir.join(&unique_filename);

    // Write file to disk
    fs::write(&file_path, processed_data)
        .map_err(|e| ServerFnError::new(format!("Failed to write file: {}", e)))?;

    // Return the public URL path
    let public_url = format!("/uploads/products/{}", unique_filename);

    Ok(UploadResponse {
        success: true,
        url: Some(public_url),
        message: "Upload successful".to_string(),
    })
}

#[cfg(feature = "server")]
async fn upload_private_image_locally(
    file_data: Vec<u8>,
    file_name: String,
    content_type: String,
) -> Result<UploadResponse, ServerFnError> {
    use std::fs;
    use std::path::Path;

    // Convert image to AVIF format (keeping your existing conversion logic)
    let (processed_data, final_content_type) =
        convert_image_to_avif(file_data, &content_type).await?;

    // Generate random filename using UUID without hyphens
    let random_name = Uuid::new_v4().simple().to_string();
    let unique_filename = format!("{}.avif", random_name);

    // Use environment-aware path for private uploads
    let upload_base = if std::env::var("RAILWAY_ENVIRONMENT").is_ok() {
        "/app/assets/private/uploads"
    } else {
        "assets/private/uploads"
    };
    let assets_dir = Path::new(upload_base);

    // Create directory if it doesn't exist
    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir)
            .map_err(|e| ServerFnError::new(format!("Failed to create directory: {}", e)))?;
    }

    // Full file path
    let file_path = assets_dir.join(&unique_filename);

    // Write file to disk
    fs::write(&file_path, processed_data)
        .map_err(|e| ServerFnError::new(format!("Failed to write file: {}", e)))?;

    // Return the public URL path
    let public_url = format!("/private/uploads/{}", unique_filename);

    Ok(UploadResponse {
        success: true,
        url: Some(public_url),
        message: "Upload successful".to_string(),
    })
}

#[cfg(feature = "server")]
async fn upload_image_to_supabase(
    file_data: Vec<u8>,
    file_name: String,
    content_type: String,
) -> Result<UploadResponse, ServerFnError> {
    // Convert image to AVIF format
    let (processed_data, final_content_type) =
        convert_image_to_avif(file_data, &content_type).await?;

    // Update filename extension to .avif if conversion occurred
    let final_filename = if content_type != "image/avif" {
        // Remove existing extension and add .avif
        let name_without_ext = file_name
            .rfind('.')
            .map(|i| &file_name[..i])
            .unwrap_or(&file_name);
        format!("{}.avif", name_without_ext)
    } else {
        file_name
    };

    // Get environment variables
    let supabase_url = std::env::var("SUPABASE_URL")
        .map_err(|_| ServerFnError::new("SUPABASE_URL not found".to_string()))?;
    let service_key = std::env::var("SUPABASE_SERVICE_KEY")
        .map_err(|_| ServerFnError::new("SUPABASE_SERVICE_KEY not found".to_string()))?;

    // Generate unique filename
    let unique_filename = format!("{}_{}", Uuid::new_v4(), final_filename);
    let bucket_name = "public-media";

    // Upload URL
    let upload_url = format!(
        "{}/storage/v1/object/{}/{}",
        supabase_url, bucket_name, unique_filename
    );

    // Create HTTP client
    let client = reqwest::Client::new();

    // Upload file
    let response = client
        .post(&upload_url)
        .header("Authorization", format!("Bearer {}", service_key))
        .header("Content-Type", final_content_type)
        .body(processed_data)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Upload request failed: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Ok(UploadResponse {
            success: false,
            url: None,
            message: format!("Upload failed: {}", error_text),
        });
    }

    // Generate public URL
    let public_url = format!(
        "{}/storage/v1/object/public/{}/{}",
        supabase_url, bucket_name, unique_filename
    );

    Ok(UploadResponse {
        success: true,
        url: Some(public_url),
        message: "Upload successful".to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct SupabaseTokenClaims {
    pub sub: String,
    pub email: Option<String>,
    pub exp: usize,
    pub iat: usize,
    pub iss: Option<String>,
    pub aud: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEditProductRequest {
    // Basic product info
    pub id: Option<String>, // ONLY IS DEFINED IF EDITING A PRODUCT
    pub title: String,
    pub subtitle: Option<String>,
    pub handle: String,
    pub collections: Vec<Category>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub alternate_names: Vec<String>,

    // Product settings
    pub product_form: ProductForm,
    pub visibility: ProductVisibility,
    pub force_no_stock: bool,
    pub purity_standard: Option<f64>,

    // Meta info
    pub physical_description: Option<String>,
    pub plabs_node_id: Option<String>,
    pub cas: Option<String>,
    pub iupac: Option<String>,
    pub mol_form: Option<String>,
    pub smiles: Option<String>,
    pub enable_render_if_smiles: bool,
    pub pubchem_cid: Option<String>,
    pub analysis_url_qnmr: Option<String>,
    pub analysis_url_hplc: Option<String>,
    pub analysis_url_qh1: Option<String>,
    pub weight: Option<f64>,
    pub dimensions_height: Option<f64>,
    pub dimensions_length: Option<f64>,
    pub dimensions_width: Option<f64>,
    pub pre_order: bool,
    pub pre_order_goal: Option<f64>,
    pub phase: ProductPhase,
    pub brand: Option<String>,
    pub priority: Option<i32>,
    pub back_order: bool,

    // Variants
    pub variants: Vec<CreateEditProductVariantRequest>,

    // Stock Item Relations
    pub product_variant_stock_item_relations: Option<Vec<ProductVariantStockItemRelation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEditProductVariantRequest {
    pub id: Option<String>,
    pub name: String,
    pub primary_thumbnail_url: Option<String>,
    pub additional_thumbnail_urls: Option<Vec<String>>,
    pub price_base_standard_usd: f64,
    pub pbx_sku: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductResponse {
    pub success: bool,
    pub message: String,
    pub product_id: Option<String>,
}

#[server]
pub async fn admin_create_product(
    request: CreateEditProductRequest,
) -> Result<CreateProductResponse, ServerFnError> {
    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(CreateProductResponse {
            success: false,
            message: "Unauthorized".to_string(),
            product_id: None,
        });
    }

    // Validation logic (same as before)
    if request.title.trim().is_empty() {
        return Ok(CreateProductResponse {
            success: false,
            message: "Title is required".to_string(),
            product_id: None,
        });
    }

    if request.handle.trim().is_empty() {
        return Ok(CreateProductResponse {
            success: false,
            message: "Handle is required".to_string(),
            product_id: None,
        });
    }

    if request.variants.is_empty() {
        return Ok(CreateProductResponse {
            success: false,
            message: "At least one variant is required".to_string(),
            product_id: None,
        });
    }

    for (i, variant) in request.variants.iter().enumerate() {
        if variant.name.trim().is_empty() {
            return Ok(CreateProductResponse {
                success: false,
                message: format!("Variant {} name is required", i + 1),
                product_id: None,
            });
        }
    }

    let db = get_db().await;

    // Check if handle already exists
    let existing_product = products::Entity::find()
        .filter(products::Column::Handle.eq(&request.handle))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if existing_product.is_some() {
        return Ok(CreateProductResponse {
            success: false,
            message: "A product with this handle already exists".to_string(),
            product_id: None,
        });
    }

    // Create product
    let product_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    // Handle alternate names - now it's directly Vec<String>
    let alternate_names_value = if request.alternate_names.is_empty() {
        None
    } else {
        Some(request.alternate_names)
    };

    let product = products::ActiveModel {
        id: ActiveValue::Set(product_id.clone()),
        title: ActiveValue::Set(request.title),
        subtitle: ActiveValue::Set(request.subtitle),
        handle: ActiveValue::Set(request.handle),
        collections: ActiveValue::Set(Some(
            request
                .collections
                .into_iter()
                .map(|e| e.to_key().to_string())
                .collect::<Vec<String>>(),
        )),
        product_form: ActiveValue::Set(request.product_form.to_seaorm()),
        physical_description: ActiveValue::Set(request.physical_description),
        default_variant_id: ActiveValue::NotSet,
        force_no_stock: ActiveValue::Set(request.force_no_stock),
        plabs_node_id: ActiveValue::Set(request.plabs_node_id),
        purity: ActiveValue::Set(request.purity_standard),
        visibility: ActiveValue::Set(request.visibility.to_seaorm()),
        small_description_md: ActiveValue::Set(request.short_description),
        main_description_md: ActiveValue::Set(request.long_description),
        alternate_names: ActiveValue::Set(alternate_names_value), // Direct assignment
        cas: ActiveValue::Set(request.cas),
        iupac: ActiveValue::Set(request.iupac),
        mol_form: ActiveValue::Set(request.mol_form),
        smiles: ActiveValue::Set(request.smiles),
        enable_render_if_smiles: ActiveValue::Set(request.enable_render_if_smiles),
        pubchem_cid: ActiveValue::Set(request.pubchem_cid),
        calculated_admet: ActiveValue::NotSet,
        analysis_url_qnmr: ActiveValue::Set(request.analysis_url_qnmr),
        analysis_url_hplc: ActiveValue::Set(request.analysis_url_hplc),
        analysis_url_qh1: ActiveValue::Set(request.analysis_url_qh1),
        weight: ActiveValue::Set(request.weight),
        dimensions_height: ActiveValue::Set(request.dimensions_height),
        dimensions_length: ActiveValue::Set(request.dimensions_length),
        dimensions_width: ActiveValue::Set(request.dimensions_width),
        pre_order: ActiveValue::Set(request.pre_order),
        pre_order_goal: ActiveValue::Set(request.pre_order_goal),
        phase: ActiveValue::Set(request.phase.to_seaorm()),
        brand: ActiveValue::Set(request.brand),
        priority: ActiveValue::Set(request.priority),
        back_order: ActiveValue::Set(request.back_order),
        mechanism: ActiveValue::NotSet,
        metadata: ActiveValue::NotSet,
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    // Insert product (rest of the function remains the same)
    let product_result = products::Entity::insert(product)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create product: {}", e)))?;

    // Create variants (same as before)
    let mut variant_models = Vec::new();
    let mut first_variant_id = None;

    for variant_request in request.variants {
        let variant_id = Uuid::new_v4().to_string();

        if first_variant_id.is_none() {
            first_variant_id = Some(variant_id.clone());
        }

        let variant = entity::product_variants::ActiveModel {
            id: ActiveValue::Set(variant_id),
            variant_name: ActiveValue::Set(variant_request.name),
            product_id: ActiveValue::Set(product_id.clone()),
            pbx_sku: ActiveValue::Set(Some(variant_request.pbx_sku)),
            thumbnail_url: ActiveValue::Set(variant_request.primary_thumbnail_url),
            weight: ActiveValue::NotSet,
            price_standard_usd: ActiveValue::Set(variant_request.price_base_standard_usd),
            price_standard_without_sale: ActiveValue::NotSet,
            additional_thumbnail_urls: ActiveValue::Set(variant_request.additional_thumbnail_urls),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        };

        variant_models.push(variant);
    }

    // Insert variants
    entity::product_variants::Entity::insert_many(variant_models)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create variants: {}", e)))?;

    // Update product with default variant ID
    if let Some(default_variant_id) = first_variant_id {
        let mut product_update = products::ActiveModel {
            id: ActiveValue::Set(product_id.clone()),
            ..Default::default()
        };
        product_update.default_variant_id = ActiveValue::Set(Some(default_variant_id));

        products::Entity::update(product_update)
            .exec(db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update default variant: {}", e)))?;
    }

    Ok(CreateProductResponse {
        success: true,
        message: "Product created successfully".to_string(),
        product_id: Some(product_id),
    })
}

#[server]
pub async fn admin_edit_product(
    request: CreateEditProductRequest,
) -> Result<CreateProductResponse, ServerFnError> {
    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(CreateProductResponse {
            success: false,
            message: "Unauthorized".to_string(),
            product_id: None,
        });
    }

    let product_id = request.id.clone().ok_or_else(|| {
        ServerFnError::new("No product ID provided for edit operation".to_string())
    })?;

    // Validation logic (same as create)
    if request.title.trim().is_empty() {
        return Ok(CreateProductResponse {
            success: false,
            message: "Title is required".to_string(),
            product_id: None,
        });
    }

    if request.handle.trim().is_empty() {
        return Ok(CreateProductResponse {
            success: false,
            message: "Handle is required".to_string(),
            product_id: None,
        });
    }

    if request.variants.is_empty() {
        return Ok(CreateProductResponse {
            success: false,
            message: "At least one variant is required".to_string(),
            product_id: None,
        });
    }

    for (i, variant) in request.variants.iter().enumerate() {
        if variant.name.trim().is_empty() {
            return Ok(CreateProductResponse {
                success: false,
                message: format!("Variant {} name is required", i + 1),
                product_id: None,
            });
        }
    }

    let db = get_db().await;

    // Check if the product exists
    let existing_product = products::Entity::find_by_id(&product_id)
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if existing_product.is_none() {
        return Ok(CreateProductResponse {
            success: false,
            message: "Product not found".to_string(),
            product_id: None,
        });
    }

    // Check if handle is taken by another product
    let handle_conflict = products::Entity::find()
        .filter(products::Column::Handle.eq(&request.handle))
        .filter(products::Column::Id.ne(&product_id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if handle_conflict.is_some() {
        return Ok(CreateProductResponse {
            success: false,
            message: "A product with this handle already exists".to_string(),
            product_id: None,
        });
    }

    let now = Utc::now().naive_utc();

    // Start a transaction to ensure consistency
    let txn = db
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to start transaction: {}", e)))?;

    // Update the product
    let alternate_names_value = if request.alternate_names.is_empty() {
        None
    } else {
        Some(request.alternate_names)
    };

    let product_update = products::ActiveModel {
        id: ActiveValue::Set(product_id.clone()),
        title: ActiveValue::Set(request.title),
        subtitle: ActiveValue::Set(request.subtitle),
        handle: ActiveValue::Set(request.handle),
        collections: ActiveValue::Set(Some(
            request
                .collections
                .into_iter()
                .map(|e| e.to_key().to_string())
                .collect::<Vec<String>>(),
        )),
        product_form: ActiveValue::Set(request.product_form.to_seaorm()),
        physical_description: ActiveValue::Set(request.physical_description),
        force_no_stock: ActiveValue::Set(request.force_no_stock),
        plabs_node_id: ActiveValue::Set(request.plabs_node_id),
        purity: ActiveValue::Set(request.purity_standard),
        visibility: ActiveValue::Set(request.visibility.to_seaorm()),
        small_description_md: ActiveValue::Set(request.short_description),
        main_description_md: ActiveValue::Set(request.long_description),
        alternate_names: ActiveValue::Set(alternate_names_value),
        cas: ActiveValue::Set(request.cas),
        iupac: ActiveValue::Set(request.iupac),
        mol_form: ActiveValue::Set(request.mol_form),
        smiles: ActiveValue::Set(request.smiles),
        enable_render_if_smiles: ActiveValue::Set(request.enable_render_if_smiles),
        pubchem_cid: ActiveValue::Set(request.pubchem_cid),
        analysis_url_qnmr: ActiveValue::Set(request.analysis_url_qnmr),
        analysis_url_hplc: ActiveValue::Set(request.analysis_url_hplc),
        analysis_url_qh1: ActiveValue::Set(request.analysis_url_qh1),
        weight: ActiveValue::Set(request.weight),
        dimensions_height: ActiveValue::Set(request.dimensions_height),
        dimensions_length: ActiveValue::Set(request.dimensions_length),
        dimensions_width: ActiveValue::Set(request.dimensions_width),
        pre_order: ActiveValue::Set(request.pre_order),
        pre_order_goal: ActiveValue::Set(request.pre_order_goal),
        phase: ActiveValue::Set(request.phase.to_seaorm()),
        brand: ActiveValue::Set(request.brand),
        priority: ActiveValue::Set(request.priority),
        back_order: ActiveValue::Set(request.back_order),
        updated_at: ActiveValue::Set(now),
        ..Default::default()
    };

    products::Entity::update(product_update)
        .exec(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update product: {}", e)))?;

    // Handle variants
    // Get existing variants for this product
    let existing_variants = entity::product_variants::Entity::find()
        .filter(entity::product_variants::Column::ProductId.eq(&product_id))
        .all(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch existing variants: {}", e)))?;

    let existing_variant_ids: HashSet<String> =
        existing_variants.iter().map(|v| v.id.clone()).collect();

    let mut request_variant_ids = HashSet::new();
    let mut new_variants = Vec::new();
    let mut variants_to_update = Vec::new();
    let mut first_variant_id = None;

    // Process variants from the request
    for variant_request in request.variants {
        if let Some(variant_id) = &variant_request.id {
            // Existing variant - prepare for update
            request_variant_ids.insert(variant_id.clone());

            if first_variant_id.is_none() {
                first_variant_id = Some(variant_id.clone());
            }

            let variant_update = entity::product_variants::ActiveModel {
                id: ActiveValue::Set(variant_id.clone()),
                variant_name: ActiveValue::Set(variant_request.name),
                thumbnail_url: ActiveValue::Set(variant_request.primary_thumbnail_url),
                weight: ActiveValue::NotSet,
                price_standard_usd: ActiveValue::Set(variant_request.price_base_standard_usd),
                additional_thumbnail_urls: ActiveValue::Set(
                    variant_request.additional_thumbnail_urls,
                ),
                pbx_sku: ActiveValue::Set(Some(variant_request.pbx_sku)),
                updated_at: ActiveValue::Set(now),
                ..Default::default()
            };

            variants_to_update.push(variant_update);
        } else {
            // New variant - prepare for creation
            let variant_id = Uuid::new_v4().to_string();
            request_variant_ids.insert(variant_id.clone());

            if first_variant_id.is_none() {
                first_variant_id = Some(variant_id.clone());
            }

            let new_variant = entity::product_variants::ActiveModel {
                id: ActiveValue::Set(variant_id),
                variant_name: ActiveValue::Set(variant_request.name),
                product_id: ActiveValue::Set(product_id.clone()),
                pbx_sku: ActiveValue::Set(Some(variant_request.pbx_sku)),
                thumbnail_url: ActiveValue::Set(variant_request.primary_thumbnail_url),
                weight: ActiveValue::NotSet,
                price_standard_usd: ActiveValue::Set(variant_request.price_base_standard_usd),
                price_standard_without_sale: ActiveValue::NotSet,
                additional_thumbnail_urls: ActiveValue::Set(
                    variant_request.additional_thumbnail_urls,
                ),
                created_at: ActiveValue::Set(now),
                updated_at: ActiveValue::Set(now),
            };

            new_variants.push(new_variant);
        }
    }

    // Find variants to delete (existed before but not in current request)
    let variants_to_delete: Vec<String> = existing_variant_ids
        .difference(&request_variant_ids)
        .cloned()
        .collect();

    // IMPORTANT: Handle stock item relations BEFORE deleting variants
    use entity::product_variant_stock_item_relations as PVSIR;
    use sea_orm::sea_query::OnConflict;

    // First, delete all existing relations for variants that will be deleted
    if !variants_to_delete.is_empty() {
        PVSIR::Entity::delete_many()
            .filter(PVSIR::Column::ProductVariantId.is_in(variants_to_delete.clone()))
            .exec(&txn)
            .await
            .map_err(|e| {
                ServerFnError::new(format!(
                    "Failed to delete relations for removed variants: {}",
                    e
                ))
            })?;
    }

    // Delete removed variants (now safe since relations are cleaned up)
    if !variants_to_delete.is_empty() {
        entity::product_variants::Entity::delete_many()
            .filter(entity::product_variants::Column::Id.is_in(variants_to_delete))
            .exec(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete variants: {}", e)))?;
    }

    // Update existing variants
    for variant_update in variants_to_update {
        entity::product_variants::Entity::update(variant_update)
            .exec(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update variant: {}", e)))?;
    }

    // Insert new variants
    if !new_variants.is_empty() {
        entity::product_variants::Entity::insert_many(new_variants)
            .exec(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to create new variants: {}", e)))?;
    }

    // Update default variant ID if we have variants
    if let Some(default_variant_id) = first_variant_id {
        let mut product_default_update = products::ActiveModel {
            id: ActiveValue::Set(product_id.clone()),
            ..Default::default()
        };
        product_default_update.default_variant_id = ActiveValue::Set(Some(default_variant_id));

        products::Entity::update(product_default_update)
            .exec(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update default variant: {}", e)))?;
    }

    // Handle Product Variant Stock Item Relations
    // Now we work with the CURRENT set of variants (after deletions and additions)
    if let Some(relations) = request.product_variant_stock_item_relations.clone() {
        // Get the current variant IDs for this product (after all variant operations)
        let current_product_variant_ids: Vec<String> = entity::product_variants::Entity::find()
            .filter(entity::product_variants::Column::ProductId.eq(&product_id))
            .all(&txn)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to fetch current product variants: {}", e))
            })?
            .into_iter()
            .map(|v| v.id)
            .collect();

        // Get existing relations for current variants only
        let existing_relations = PVSIR::Entity::find()
            .filter(PVSIR::Column::ProductVariantId.is_in(current_product_variant_ids.clone()))
            .all(&txn)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to fetch existing relations: {}", e))
            })?;

        let existing_keys: HashSet<(String, String)> = existing_relations
            .iter()
            .map(|r| (r.product_variant_id.clone(), r.stock_item_id.clone()))
            .collect();

        // Build the set of desired keys from request (validate variant ownership)
        let mut desired_keys: HashSet<(String, String)> = HashSet::new();

        // Process all relations from request
        for rel in relations {
            // Validate variant belongs to current product variants
            if !current_product_variant_ids.contains(&rel.product_variant_id) {
                return Err(ServerFnError::new(format!(
                    "Relation contains variant ID {} that doesn't belong to current variants of product {}",
                    rel.product_variant_id, product_id
                )));
            }

            desired_keys.insert((rel.product_variant_id.clone(), rel.stock_item_id.clone()));

            // Upsert by composite PK: (product_variant_id, stock_item_id)
            let am = PVSIR::ActiveModel {
                product_variant_id: ActiveValue::Set(rel.product_variant_id),
                stock_item_id: ActiveValue::Set(rel.stock_item_id),
                quantity: ActiveValue::Set(rel.quantity),
                stock_unit_on_creation: ActiveValue::Set(rel.stock_unit_on_creation.to_seaorm()),
            };

            PVSIR::Entity::insert(am)
                .on_conflict(
                    OnConflict::columns([
                        PVSIR::Column::ProductVariantId,
                        PVSIR::Column::StockItemId,
                    ])
                    .update_columns([PVSIR::Column::Quantity]) // keep unit immutable after creation, update quantity only
                    .to_owned(),
                )
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to upsert relation: {}", e)))?;
        }

        // Delete any existing relations that are not in the desired set
        // This handles the case where relations are removed from existing variants
        let to_delete: Vec<(String, String)> =
            existing_keys.difference(&desired_keys).cloned().collect();

        for (pv, si) in to_delete {
            PVSIR::Entity::delete_many()
                .filter(PVSIR::Column::ProductVariantId.eq(pv))
                .filter(PVSIR::Column::StockItemId.eq(si))
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to delete relation: {}", e)))?;
        }
    } else {
        // If no relations provided, delete all existing relations for current variants
        let current_product_variant_ids: Vec<String> = entity::product_variants::Entity::find()
            .filter(entity::product_variants::Column::ProductId.eq(&product_id))
            .all(&txn)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to fetch current product variants: {}", e))
            })?
            .into_iter()
            .map(|v| v.id)
            .collect();

        if !current_product_variant_ids.is_empty() {
            PVSIR::Entity::delete_many()
                .filter(PVSIR::Column::ProductVariantId.is_in(current_product_variant_ids))
                .exec(&txn)
                .await
                .map_err(|e| {
                    ServerFnError::new(format!("Failed to delete all relations: {}", e))
                })?;
        }
    }

    // Commit the transaction
    txn.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to commit transaction: {}", e)))?;

    tracing::info!("Successfully updated product with ID: {}", product_id);

    Ok(CreateProductResponse {
        success: true,
        message: "Product updated successfully".to_string(),
        product_id: Some(product_id),
    })
}

// Stock item logic

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStockItemRequest {
    pub name: String,
    pub pbi_sku: String,
    pub description: Option<String>,
    pub thumbnail_ref: Option<String>,
    pub unit: StockUnit,
    pub assembly_minutes: Option<i32>,
    pub default_shipping_days: Option<i32>,
    pub default_cost: Option<f64>,
    pub warning_quantity: Option<f64>,
    pub is_container: bool,
    //pub assembled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStockItemResponse {
    pub success: bool,
    pub message: String,
    pub stock_item_id: Option<String>,
}

#[server]
pub async fn admin_create_stock_item(
    request: CreateStockItemRequest,
) -> Result<CreateStockItemResponse, ServerFnError> {
    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(CreateStockItemResponse {
            success: false,
            message: "Unauthorized".to_string(),
            stock_item_id: None,
        });
    }

    // Validation logic
    if request.name.trim().is_empty() {
        return Ok(CreateStockItemResponse {
            success: false,
            message: "Name is required".to_string(),
            stock_item_id: None,
        });
    }

    if request.pbi_sku.trim().is_empty() {
        return Ok(CreateStockItemResponse {
            success: false,
            message: "PBI SKU is required".to_string(),
            stock_item_id: None,
        });
    }

    // Validate SKU format (should start with PBI)
    if !request.pbi_sku.starts_with("PBI") && !request.pbi_sku.starts_with("PBX") {
        return Ok(CreateStockItemResponse {
            success: false,
            message: "SKU must start with 'PBI' or 'PBX'".to_string(),
            stock_item_id: None,
        });
    }

    if request.pbi_sku.len() < 7 {
        return Ok(CreateStockItemResponse {
            success: false,
            message: "SKU not the correct length".to_string(),
            stock_item_id: None,
        });
    }

    let db = get_db().await;

    // Check if SKU already exists
    let existing_stock_item = stock_items::Entity::find()
        .filter(stock_items::Column::PbiSku.eq(&request.pbi_sku))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if existing_stock_item.is_some() {
        return Ok(CreateStockItemResponse {
            success: false,
            message: "A stock item with this SKU already exists".to_string(),
            stock_item_id: None,
        });
    }

    // Create stock item
    let stock_item_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    let stock_item = stock_items::ActiveModel {
        id: ActiveValue::Set(stock_item_id.clone()),
        pbi_sku: ActiveValue::Set(request.pbi_sku),
        name: ActiveValue::Set(request.name),
        description: ActiveValue::Set(request.description),
        thumbnail_ref: ActiveValue::Set(request.thumbnail_ref),
        unit: ActiveValue::Set(request.unit.to_seaorm()),
        assembly_minutes: ActiveValue::Set(request.assembly_minutes),
        default_shipping_days: ActiveValue::Set(request.default_shipping_days),
        default_cost: ActiveValue::Set(request.default_cost),
        warning_quantity: ActiveValue::Set(request.warning_quantity),
        is_container: ActiveValue::Set(request.is_container),
        //assembled: ActiveValue::Set(request.assembled), DEPRECATED
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    // Insert stock item
    stock_items::Entity::insert(stock_item)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create stock item: {}", e)))?;

    Ok(CreateStockItemResponse {
        success: true,
        message: "Stock item created successfully".to_string(),
        stock_item_id: Some(stock_item_id),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditStockItemRequest {
    pub id: String,
    pub name: String,
    pub pbi_sku: String,
    pub description: Option<String>,
    pub thumbnail_ref: Option<String>,
    pub unit: StockUnit,
    pub assembly_minutes: Option<i32>,
    pub default_shipping_days: Option<i32>,
    pub default_cost: Option<f64>,
    pub warning_quantity: Option<f64>,
    pub is_container: bool,
    pub flatten_pre_or_back_reduces: bool,
    pub batches: Option<Vec<EditStockBatchRequest>>,
    // Stock Item Relations
    pub stock_item_relations: Option<Vec<StockItemRelation>>,
}

// Unique entry for each batch of stock
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct EditStockBatchRequest {
    pub id: Option<String>,
    pub stock_batch_code: String,
    pub comment: Option<String>,
    pub supplier: Option<String>,
    pub original_quantity: StockUnitQuantity,
    pub live_quantity: StockUnitQuantity,
    pub stock_unit_on_creation: StockUnit,
    pub cost_usd: Option<f64>,
    pub arrival_date: Option<NaiveDateTime>,
    pub warehouse_location: StockBatchLocation,
    pub tracking_url: Option<String>,
    pub assembled: bool,
    pub status: StockBatchStatus,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditStockItemResponse {
    pub success: bool,
    pub message: String,
    pub stock_item_id: Option<String>,
}

#[server]
pub async fn admin_edit_stock_item(
    request: EditStockItemRequest,
) -> Result<EditStockItemResponse, ServerFnError> {
    use sea_orm::sea_query::OnConflict;

    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(EditStockItemResponse {
            success: false,
            message: "Unauthorized".to_string(),
            stock_item_id: None,
        });
    }

    // Validation logic (same as create)
    if request.name.trim().is_empty() {
        return Ok(EditStockItemResponse {
            success: false,
            message: "Name is required".to_string(),
            stock_item_id: None,
        });
    }

    if request.pbi_sku.trim().is_empty() {
        return Ok(EditStockItemResponse {
            success: false,
            message: "PBI SKU is required".to_string(),
            stock_item_id: None,
        });
    }

    // Validate SKU format (should start with PBI or PBX)
    if !request.pbi_sku.starts_with("PBI") && !request.pbi_sku.starts_with("PBX") {
        return Ok(EditStockItemResponse {
            success: false,
            message: "SKU must start with 'PBI' or 'PBX'".to_string(),
            stock_item_id: None,
        });
    }

    if request.pbi_sku.len() < 7 {
        return Ok(EditStockItemResponse {
            success: false,
            message: "SKU not the correct length".to_string(),
            stock_item_id: None,
        });
    }

    // Validate batches if provided
    if let Some(ref batches) = request.batches {
        for (i, batch) in batches.iter().enumerate() {
            if batch.stock_batch_code.trim().is_empty() {
                return Ok(EditStockItemResponse {
                    success: false,
                    message: format!("Batch {} code is required", i + 1),
                    stock_item_id: None,
                });
            }
        }
    }

    let db = get_db().await;

    // Check if the stock item exists
    let existing_stock_item = stock_items::Entity::find_by_id(&request.id)
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if existing_stock_item.is_none() {
        return Ok(EditStockItemResponse {
            success: false,
            message: "Stock item not found".to_string(),
            stock_item_id: None,
        });
    }

    // Check if SKU is taken by another stock item
    let sku_conflict = stock_items::Entity::find()
        .filter(stock_items::Column::PbiSku.eq(&request.pbi_sku))
        .filter(stock_items::Column::Id.ne(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if sku_conflict.is_some() {
        return Ok(EditStockItemResponse {
            success: false,
            message: "A stock item with this SKU already exists".to_string(),
            stock_item_id: None,
        });
    }

    // Check for batch code conflicts with other stock items (only if batches provided)
    if let Some(ref batches) = request.batches {
        for batch in batches {
            let batch_code_conflict = stock_batches::Entity::find()
                .filter(stock_batches::Column::StockBatchCode.eq(&batch.stock_batch_code))
                .filter(stock_batches::Column::StockItemId.ne(&request.id))
                .one(db)
                .await
                .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

            if batch_code_conflict.is_some() {
                return Ok(EditStockItemResponse {
                    success: false,
                    message: format!(
                        "Batch code '{}' is already used by another stock item",
                        batch.stock_batch_code
                    ),
                    stock_item_id: None,
                });
            }

            // Also check for conflicts within the same request
            if let Some(existing_batch_id) = &batch.id {
                let same_code_conflict = stock_batches::Entity::find()
                    .filter(stock_batches::Column::StockBatchCode.eq(&batch.stock_batch_code))
                    .filter(stock_batches::Column::Id.ne(existing_batch_id))
                    .filter(stock_batches::Column::StockItemId.eq(&request.id))
                    .one(db)
                    .await
                    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

                if same_code_conflict.is_some() {
                    return Ok(EditStockItemResponse {
                        success: false,
                        message: format!(
                            "Batch code '{}' is already used by another batch in this stock item",
                            batch.stock_batch_code
                        ),
                        stock_item_id: None,
                    });
                }
            }
        }
    }

    let now = Utc::now().naive_utc();

    // Start a transaction to ensure consistency
    let txn = db
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to start transaction: {}", e)))?;

    // Update the stock item
    let stock_item_update = stock_items::ActiveModel {
        id: ActiveValue::Set(request.id.clone()),
        name: ActiveValue::Set(request.name),
        pbi_sku: ActiveValue::Set(request.pbi_sku),
        description: ActiveValue::Set(request.description),
        thumbnail_ref: ActiveValue::Set(request.thumbnail_ref),
        unit: ActiveValue::Set(request.unit.to_seaorm()),
        assembly_minutes: ActiveValue::Set(request.assembly_minutes),
        default_shipping_days: ActiveValue::Set(request.default_shipping_days),
        default_cost: ActiveValue::Set(request.default_cost),
        warning_quantity: ActiveValue::Set(request.warning_quantity),
        is_container: ActiveValue::Set(request.is_container),
        updated_at: ActiveValue::Set(now),
        ..Default::default()
    };

    stock_items::Entity::update(stock_item_update)
        .exec(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update stock item: {}", e)))?;

    // Handle stock batches only if provided
    if let Some(batches) = request.batches {
        // Get existing batches for this stock item
        let existing_batches = stock_batches::Entity::find()
            .filter(stock_batches::Column::StockItemId.eq(&request.id))
            .all(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch existing batches: {}", e)))?;

        let existing_batch_ids: HashSet<String> =
            existing_batches.iter().map(|b| b.id.clone()).collect();

        let mut request_batch_ids = HashSet::new();
        let mut new_batches = Vec::new();
        let mut batches_to_update = Vec::new();

        // Process batches from the request
        for batch_request in batches {
            if let Some(batch_id) = &batch_request.id {
                // Existing batch - prepare for update
                request_batch_ids.insert(batch_id.clone());

                let batch_update = stock_batches::ActiveModel {
                    id: ActiveValue::Set(batch_id.clone()),
                    stock_batch_code: ActiveValue::Set(batch_request.stock_batch_code),
                    comment: ActiveValue::Set(batch_request.comment),
                    supplier: ActiveValue::Set(batch_request.supplier),
                    original_quantity: ActiveValue::Set(batch_request.original_quantity.to_f64()),
                    live_quantity: ActiveValue::Set(batch_request.live_quantity.to_f64()),
                    stock_unit_on_creation: ActiveValue::Set(
                        batch_request.stock_unit_on_creation.to_seaorm(),
                    ),
                    cost_usd: ActiveValue::Set(batch_request.cost_usd),
                    arrival_date: ActiveValue::Set(batch_request.arrival_date),
                    warehouse_location: ActiveValue::Set(
                        batch_request.warehouse_location.to_seaorm(),
                    ),
                    tracking_url: ActiveValue::Set(batch_request.tracking_url),
                    status: ActiveValue::Set(batch_request.status.to_seaorm()),
                    assembled: ActiveValue::Set(batch_request.assembled),
                    updated_at: ActiveValue::Set(now),
                    ..Default::default()
                };

                batches_to_update.push(batch_update);
            } else {
                // New batch - prepare for creation
                let batch_id = Uuid::new_v4().to_string();
                request_batch_ids.insert(batch_id.clone());

                let new_batch = stock_batches::ActiveModel {
                    id: ActiveValue::Set(batch_id),
                    stock_batch_code: ActiveValue::Set(batch_request.stock_batch_code),
                    stock_item_id: ActiveValue::Set(request.id.clone()),
                    comment: ActiveValue::Set(batch_request.comment),
                    supplier: ActiveValue::Set(batch_request.supplier),
                    original_quantity: ActiveValue::Set(batch_request.original_quantity.to_f64()),
                    live_quantity: ActiveValue::Set(batch_request.live_quantity.to_f64()),
                    stock_unit_on_creation: ActiveValue::Set(
                        batch_request.stock_unit_on_creation.to_seaorm(),
                    ),
                    cost_usd: ActiveValue::Set(batch_request.cost_usd),
                    arrival_date: ActiveValue::Set(batch_request.arrival_date),
                    warehouse_location: ActiveValue::Set(
                        batch_request.warehouse_location.to_seaorm(),
                    ),
                    tracking_url: ActiveValue::Set(batch_request.tracking_url),
                    status: ActiveValue::Set(batch_request.status.to_seaorm()),
                    assembled: ActiveValue::Set(batch_request.assembled),
                    created_at: ActiveValue::Set(now),
                    updated_at: ActiveValue::Set(now),
                };

                new_batches.push(new_batch);
            }
        }

        // Find batches to delete (existed before but not in current request)
        let batches_to_delete: Vec<String> = existing_batch_ids
            .difference(&request_batch_ids)
            .cloned()
            .collect();

        // Delete removed batches
        if !batches_to_delete.is_empty() {
            stock_batches::Entity::delete_many()
                .filter(stock_batches::Column::Id.is_in(batches_to_delete))
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to delete batches: {}", e)))?;
        }

        // Update existing batches
        for batch_update in batches_to_update {
            stock_batches::Entity::update(batch_update)
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to update batch: {}", e)))?;
        }

        // Insert new batches
        if !new_batches.is_empty() {
            stock_batches::Entity::insert_many(new_batches)
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to create new batches: {}", e)))?;
        }
    }

    // Handle stock item relations - FIXED SECTION
    if let Some(relations) = request.stock_item_relations {
        // Get existing relations where this stock item is the parent
        let existing_relations = stock_item_relations::Entity::find()
            .filter(stock_item_relations::Column::ParentStockItemId.eq(&request.id))
            .all(&txn)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to fetch existing relations: {}", e))
            })?;

        // Create a set of existing relation keys (composite primary keys)
        let existing_keys: HashSet<(String, String)> = existing_relations
            .iter()
            .map(|r| {
                (
                    r.parent_stock_item_id.clone(),
                    r.child_stock_item_id.clone(),
                )
            })
            .collect();

        // Build the set of desired keys from request and validate
        let mut desired_keys: HashSet<(String, String)> = HashSet::new();

        // Process relations from the request
        for relation in relations {
            // Validate that this relation has the correct parent stock item id
            if relation.parent_stock_item_id != request.id {
                return Err(ServerFnError::new(format!(
                    "Relation contains parent stock item ID {} that doesn't match the current stock item {}",
                    relation.parent_stock_item_id, request.id
                )));
            }

            let relation_key = (
                relation.parent_stock_item_id.clone(),
                relation.child_stock_item_id.clone(),
            );
            desired_keys.insert(relation_key);

            // Use upsert approach to handle both new and existing relations
            let am = stock_item_relations::ActiveModel {
                parent_stock_item_id: ActiveValue::Set(relation.parent_stock_item_id),
                child_stock_item_id: ActiveValue::Set(relation.child_stock_item_id),
                quantity: ActiveValue::Set(relation.quantity),
                created_at: ActiveValue::Set(now), // This will be ignored on updates due to OnConflict
                updated_at: ActiveValue::Set(now),
            };

            stock_item_relations::Entity::insert(am)
                .on_conflict(
                    OnConflict::columns([
                        stock_item_relations::Column::ParentStockItemId,
                        stock_item_relations::Column::ChildStockItemId,
                    ])
                    .update_columns([
                        stock_item_relations::Column::Quantity,
                        stock_item_relations::Column::UpdatedAt,
                    ])
                    .to_owned(),
                )
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to upsert relation: {}", e)))?;
        }

        // Delete any existing relations that are not in the desired set
        let to_delete: Vec<(String, String)> =
            existing_keys.difference(&desired_keys).cloned().collect();

        for (parent_id, child_id) in to_delete {
            stock_item_relations::Entity::delete_many()
                .filter(
                    stock_item_relations::Column::ParentStockItemId
                        .eq(parent_id)
                        .and(stock_item_relations::Column::ChildStockItemId.eq(child_id)),
                )
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to delete relation: {}", e)))?;
        }
    } else {
        // If no relations provided, delete all existing relations for this stock item
        stock_item_relations::Entity::delete_many()
            .filter(stock_item_relations::Column::ParentStockItemId.eq(&request.id))
            .exec(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete all relations: {}", e)))?;
    }

    // Commit the transaction
    txn.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to commit transaction: {}", e)))?;

    if request.flatten_pre_or_back_reduces {
        match payments::flatten_preorder_backorder_reduces(request.id.clone()).await {
            Ok(()) => {}
            Err(e) => {
                return Err(ServerFnError::new(format!(
                    "Could not flatten pre/back-order reduces: {:?}",
                    e
                )));
            }
        }
    }

    tracing::info!("Successfully updated stock item with ID: {}", request.id);

    Ok(EditStockItemResponse {
        success: true,
        message: "Stock item updated successfully".to_string(),
        stock_item_id: Some(request.id),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscountRequest {
    pub code: String,
    pub discount_type: DiscountType,
    pub discount_percentage: Option<f64>,
    pub discount_amount: Option<f64>,
    pub active: bool,
    pub maximum_uses: Option<i32>,
    pub valid_countries: Option<Vec<String>>,
    pub valid_after_x_products: Option<i32>,
    pub valid_after_x_total: Option<f64>,
    pub auto_apply: bool,
    pub expire_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscountResponse {
    pub success: bool,
    pub message: String,
    pub discount_id: Option<String>,
}

#[server]
pub async fn admin_create_discount(
    request: CreateDiscountRequest,
) -> Result<CreateDiscountResponse, ServerFnError> {
    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(CreateDiscountResponse {
            success: false,
            message: "Unauthorized".to_string(),
            discount_id: None,
        });
    }

    // Validation logic
    if request.code.trim().is_empty() {
        return Ok(CreateDiscountResponse {
            success: false,
            message: "Discount code is required".to_string(),
            discount_id: None,
        });
    }

    // Validate discount code format (alphanumeric, underscores, hyphens)
    if !request
        .code
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Ok(CreateDiscountResponse {
            success: false,
            message: "Discount code can only contain letters, numbers, underscores, and hyphens"
                .to_string(),
            discount_id: None,
        });
    }

    // Validate discount value based on type
    match request.discount_type {
        DiscountType::Percentage | DiscountType::PercentageOnShipping => {
            if request.discount_percentage.is_none() {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount percentage is required for percentage-based discounts"
                        .to_string(),
                    discount_id: None,
                });
            }
            let percentage = request.discount_percentage.unwrap();
            if percentage <= 0.0 {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount percentage must be greater than 0".to_string(),
                    discount_id: None,
                });
            }
            if percentage > 100.0 {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount percentage cannot exceed 100%".to_string(),
                    discount_id: None,
                });
            }
            // Ensure discount_amount is None for percentage types
            if request.discount_amount.is_some() {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount amount must not be set for percentage-based discounts"
                        .to_string(),
                    discount_id: None,
                });
            }
        }
        DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => {
            if request.discount_amount.is_none() {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount amount is required for fixed amount discounts".to_string(),
                    discount_id: None,
                });
            }
            let amount = request.discount_amount.unwrap();
            if amount <= 0.0 {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount amount must be greater than 0".to_string(),
                    discount_id: None,
                });
            }
            // Ensure discount_percentage is None for fixed amount types
            if request.discount_percentage.is_some() {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount percentage must not be set for fixed amount discounts"
                        .to_string(),
                    discount_id: None,
                });
            }
        }
    }

    // Validate maximum uses
    if let Some(max_uses) = request.maximum_uses {
        if max_uses <= 0 {
            return Ok(CreateDiscountResponse {
                success: false,
                message: "Maximum uses must be greater than 0".to_string(),
                discount_id: None,
            });
        }
    }

    // Validate minimum requirements
    if let Some(min_products) = request.valid_after_x_products {
        if min_products < 0 {
            return Ok(CreateDiscountResponse {
                success: false,
                message: "Minimum products cannot be negative".to_string(),
                discount_id: None,
            });
        }
    }

    if let Some(min_total) = request.valid_after_x_total {
        if min_total < 0.0 {
            return Ok(CreateDiscountResponse {
                success: false,
                message: "Minimum cart total cannot be negative".to_string(),
                discount_id: None,
            });
        }
    }

    // Validate expiration date
    if let Some(expire_date) = request.expire_at {
        if expire_date <= Utc::now().naive_utc() {
            return Ok(CreateDiscountResponse {
                success: false,
                message: "Expiration date must be in the future".to_string(),
                discount_id: None,
            });
        }
    }

    let db = get_db().await;

    // Check if discount code already exists (case-insensitive)
    let existing_discount = discounts::Entity::find()
        .filter(discounts::Column::Code.eq(&request.code.to_uppercase()))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if existing_discount.is_some() {
        return Ok(CreateDiscountResponse {
            success: false,
            message: "A discount with this code already exists".to_string(),
            discount_id: None,
        });
    }

    // Create discount
    let discount_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    // Convert DiscountType to the SeaORM enum type
    let discount_type_seaorm = match request.discount_type {
        DiscountType::Percentage => sea_orm_active_enums::DiscountType::Percentage,
        DiscountType::FixedAmount => sea_orm_active_enums::DiscountType::FixedAmount,
        DiscountType::PercentageOnShipping => {
            sea_orm_active_enums::DiscountType::PercentageOnShipping
        }
        DiscountType::FixedAmountOnShipping => {
            sea_orm_active_enums::DiscountType::FixedAmountOnShipping
        }
    };

    let discount = discounts::ActiveModel {
        id: ActiveValue::Set(discount_id.clone()),
        code: ActiveValue::Set(request.code.to_uppercase()),
        affiliate_id: ActiveValue::Set(None), // Ignore as requested
        active: ActiveValue::Set(request.active),
        discount_type: ActiveValue::Set(discount_type_seaorm),
        discount_percentage: ActiveValue::Set(request.discount_percentage),
        discount_amount: ActiveValue::Set(request.discount_amount),
        amount_used: ActiveValue::Set(None), // Will be managed when discount is used
        maximum_uses: ActiveValue::Set(request.maximum_uses),
        discount_used: ActiveValue::Set(0), // Default to 0 as requested
        active_reduce_quantity: ActiveValue::Set(0),
        valid_countries: ActiveValue::Set(request.valid_countries),
        valid_after_x_products: ActiveValue::Set(request.valid_after_x_products),
        valid_after_x_total: ActiveValue::Set(request.valid_after_x_total),
        auto_apply: ActiveValue::Set(request.auto_apply),
        expire_at: ActiveValue::Set(request.expire_at),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    // Insert discount
    discounts::Entity::insert(discount)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create discount: {}", e)))?;

    Ok(CreateDiscountResponse {
        success: true,
        message: "Discount created successfully".to_string(),
        discount_id: Some(discount_id),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDiscountRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDiscountResponse {
    pub success: bool,
    pub message: String,
    pub discount: Option<Discount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiscountRequest {
    pub id: String,
    pub code: String,
    pub discount_percentage: Option<f64>,
    pub discount_amount: Option<f64>,
    pub active: bool,
    pub maximum_uses: Option<i32>,
    pub valid_countries: Option<Vec<String>>,
    pub valid_after_x_products: Option<i32>,
    pub valid_after_x_total: Option<f64>,
    pub auto_apply: bool,
    pub expire_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiscountResponse {
    pub success: bool,
    pub message: String,
}

#[server]
pub async fn admin_get_discount(
    request: GetDiscountRequest,
) -> Result<GetDiscountResponse, ServerFnError> {
    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(GetDiscountResponse {
            success: false,
            message: "Unauthorized".to_string(),
            discount: None,
        });
    }

    let db = get_db().await;

    // Find the discount by ID
    let discount_model = discounts::Entity::find()
        .filter(discounts::Column::Id.eq(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    match discount_model {
        Some(model) => {
            // Convert SeaORM model to frontend entity
            let discount_type = match model.discount_type {
                sea_orm_active_enums::DiscountType::Percentage => DiscountType::Percentage,
                sea_orm_active_enums::DiscountType::FixedAmount => DiscountType::FixedAmount,
                sea_orm_active_enums::DiscountType::PercentageOnShipping => {
                    DiscountType::PercentageOnShipping
                }
                sea_orm_active_enums::DiscountType::FixedAmountOnShipping => {
                    DiscountType::FixedAmountOnShipping
                }
            };

            let discount = Discount {
                id: model.id,
                code: model.code,
                affiliate_id: model.affiliate_id,
                active: model.active,
                discount_type,
                discount_percentage: model.discount_percentage,
                discount_amount: model.discount_amount,
                amount_used: model.amount_used,
                maximum_uses: model.maximum_uses,
                discount_used: model.discount_used,
                active_reduce_quantity: model.active_reduce_quantity,
                valid_countries: model.valid_countries,
                valid_after_x_products: model.valid_after_x_products,
                valid_after_x_total: model.valid_after_x_total,
                auto_apply: model.auto_apply,
                expire_at: model.expire_at,
                created_at: model.created_at,
                updated_at: model.updated_at,
            };

            Ok(GetDiscountResponse {
                success: true,
                message: "Discount found".to_string(),
                discount: Some(discount),
            })
        }
        None => Ok(GetDiscountResponse {
            success: false,
            message: "Discount not found".to_string(),
            discount: None,
        }),
    }
}

#[server]
pub async fn admin_update_discount(
    request: UpdateDiscountRequest,
) -> Result<UpdateDiscountResponse, ServerFnError> {
    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(UpdateDiscountResponse {
            success: false,
            message: "Unauthorized".to_string(),
        });
    }

    // Validation logic
    if request.code.trim().is_empty() {
        return Ok(UpdateDiscountResponse {
            success: false,
            message: "Discount code is required".to_string(),
        });
    }

    // Validate discount code format (alphanumeric, underscores, hyphens)
    if !request
        .code
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Ok(UpdateDiscountResponse {
            success: false,
            message: "Discount code can only contain letters, numbers, underscores, and hyphens"
                .to_string(),
        });
    }

    // Validate maximum uses
    if let Some(max_uses) = request.maximum_uses {
        if max_uses <= 0 {
            return Ok(UpdateDiscountResponse {
                success: false,
                message: "Maximum uses must be greater than 0".to_string(),
            });
        }
    }

    // Validate minimum requirements
    if let Some(min_products) = request.valid_after_x_products {
        if min_products < 0 {
            return Ok(UpdateDiscountResponse {
                success: false,
                message: "Minimum products cannot be negative".to_string(),
            });
        }
    }

    if let Some(min_total) = request.valid_after_x_total {
        if min_total < 0.0 {
            return Ok(UpdateDiscountResponse {
                success: false,
                message: "Minimum cart total cannot be negative".to_string(),
            });
        }
    }

    // Validate expiration date
    if let Some(expire_date) = request.expire_at {
        if expire_date <= Utc::now().naive_utc() {
            return Ok(UpdateDiscountResponse {
                success: false,
                message: "Expiration date must be in the future".to_string(),
            });
        }
    }

    let db = get_db().await;

    // Check if the discount exists and get its current data
    let existing_discount = discounts::Entity::find()
        .filter(discounts::Column::Id.eq(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let discount_model = match existing_discount {
        Some(model) => model,
        None => {
            return Ok(UpdateDiscountResponse {
                success: false,
                message: "Discount not found".to_string(),
            });
        }
    };

    // Validate discount value based on existing type (type cannot be changed)
    match discount_model.discount_type {
        sea_orm_active_enums::DiscountType::Percentage
        | sea_orm_active_enums::DiscountType::PercentageOnShipping => {
            if request.discount_percentage.is_none() {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount percentage is required for percentage-based discounts"
                        .to_string(),
                });
            }
            let percentage = request.discount_percentage.unwrap();
            if percentage <= 0.0 {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount percentage must be greater than 0".to_string(),
                });
            }
            if percentage > 100.0 {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount percentage cannot exceed 100%".to_string(),
                });
            }
            // Ensure discount_amount is None for percentage types
            if request.discount_amount.is_some() {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount amount must not be set for percentage-based discounts"
                        .to_string(),
                });
            }
        }
        sea_orm_active_enums::DiscountType::FixedAmount
        | sea_orm_active_enums::DiscountType::FixedAmountOnShipping => {
            if request.discount_amount.is_none() {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount amount is required for fixed amount discounts".to_string(),
                });
            }
            let amount = request.discount_amount.unwrap();
            if amount <= 0.0 {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount amount must be greater than 0".to_string(),
                });
            }
            // Ensure discount_percentage is None for fixed amount types
            if request.discount_percentage.is_some() {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount percentage must not be set for fixed amount discounts"
                        .to_string(),
                });
            }
        }
    }

    // Check if another discount with this code exists (case-insensitive, excluding current discount)
    let code_conflict = discounts::Entity::find()
        .filter(discounts::Column::Code.eq(&request.code.to_uppercase()))
        .filter(discounts::Column::Id.ne(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if code_conflict.is_some() {
        return Ok(UpdateDiscountResponse {
            success: false,
            message: "A discount with this code already exists".to_string(),
        });
    }

    // Update the discount
    let now = Utc::now().naive_utc();

    let updated_discount = discounts::ActiveModel {
        id: ActiveValue::Unchanged(discount_model.id),
        code: ActiveValue::Set(request.code.to_uppercase()),
        affiliate_id: ActiveValue::Unchanged(discount_model.affiliate_id), // Don't change affiliate_id
        active: ActiveValue::Set(request.active),
        discount_type: ActiveValue::Unchanged(discount_model.discount_type), // Don't change discount_type
        discount_percentage: ActiveValue::Set(request.discount_percentage),
        discount_amount: ActiveValue::Set(request.discount_amount),
        amount_used: ActiveValue::Unchanged(discount_model.amount_used), // Don't reset usage data
        maximum_uses: ActiveValue::Set(request.maximum_uses),
        discount_used: ActiveValue::Unchanged(discount_model.discount_used), // Don't reset usage count
        active_reduce_quantity: ActiveValue::Unchanged(discount_model.active_reduce_quantity),
        valid_countries: ActiveValue::Set(request.valid_countries),
        valid_after_x_products: ActiveValue::Set(request.valid_after_x_products),
        valid_after_x_total: ActiveValue::Set(request.valid_after_x_total),
        auto_apply: ActiveValue::Set(request.auto_apply),
        expire_at: ActiveValue::Set(request.expire_at),
        created_at: ActiveValue::Unchanged(discount_model.created_at), // Don't change creation date
        updated_at: ActiveValue::Set(now),
    };

    // Perform the update
    discounts::Entity::update(updated_discount)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update discount: {}", e)))?;

    // Add this return statement
    Ok(UpdateDiscountResponse {
        success: true,
        message: "Discount updated successfully".to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteDiscountRequest {
    pub id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteDiscountResponse {
    pub success: bool,
    pub message: String,
}
#[server]
pub async fn admin_delete_discount(
    request: DeleteDiscountRequest,
) -> Result<DeleteDiscountResponse, ServerFnError> {
    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(DeleteDiscountResponse {
            success: false,
            message: "Unauthorized".to_string(),
        });
    }
    let db = get_db().await;
    // Delete the discount by ID
    let res = discounts::Entity::delete_by_id(request.id)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    if res.rows_affected == 0 {
        Ok(DeleteDiscountResponse {
            success: false,
            message: "Discount not found".to_string(),
        })
    } else {
        Ok(DeleteDiscountResponse {
            success: true,
            message: "Discount deleted successfully".to_string(),
        })
    }
}

// Custom error type
#[derive(Debug, Clone)]
pub struct StockCalculationError {
    pub message: String,
}

impl fmt::Display for StockCalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlogPostRequest {
    pub title: String,
    pub subtitle: Option<String>,
    pub thumbnail_url: Option<String>,
    pub blog_md: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlogPostResponse {
    pub success: bool,
    pub message: String,
    pub blog_post_id: Option<String>,
}

#[server]
pub async fn admin_create_blog_post(
    request: CreateBlogPostRequest,
) -> Result<CreateBlogPostResponse, ServerFnError> {
    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(CreateBlogPostResponse {
            success: false,
            message: "Unauthorized".to_string(),
            blog_post_id: None,
        });
    }

    // Validation logic
    if request.title.trim().is_empty() {
        return Ok(CreateBlogPostResponse {
            success: false,
            message: "Blog title is required".to_string(),
            blog_post_id: None,
        });
    }

    if request.blog_md.trim().is_empty() {
        return Ok(CreateBlogPostResponse {
            success: false,
            message: "Blog content is required".to_string(),
            blog_post_id: None,
        });
    }

    // Validate subtitle length if provided
    if let Some(ref subtitle) = request.subtitle {
        if subtitle.trim().is_empty() {
            return Ok(CreateBlogPostResponse {
                success: false,
                message: "Subtitle cannot be empty (leave blank if not needed)".to_string(),
                blog_post_id: None,
            });
        }
        if subtitle.len() > 200 {
            return Ok(CreateBlogPostResponse {
                success: false,
                message: "Subtitle cannot exceed 200 characters".to_string(),
                blog_post_id: None,
            });
        }
    }

    // Validate thumbnail URL if provided
    if let Some(ref thumbnail) = request.thumbnail_url {
        if thumbnail.trim().is_empty() {
            return Ok(CreateBlogPostResponse {
                success: false,
                message: "Thumbnail URL cannot be empty (leave blank if not needed)".to_string(),
                blog_post_id: None,
            });
        }
        // Basic URL validation
        if !thumbnail.starts_with("http://") && !thumbnail.starts_with("https://") {
            return Ok(CreateBlogPostResponse {
                success: false,
                message: "Thumbnail URL must start with http:// or https://".to_string(),
                blog_post_id: None,
            });
        }
    }

    let db = get_db().await;

    // Check if a blog post with the same title already exists
    let existing_post = blog_posts::Entity::find()
        .filter(blog_posts::Column::Title.eq(&request.title))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if existing_post.is_some() {
        return Ok(CreateBlogPostResponse {
            success: false,
            message: "A blog post with this title already exists".to_string(),
            blog_post_id: None,
        });
    }

    // Create blog post
    let blog_post_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    let blog_post = blog_posts::ActiveModel {
        id: ActiveValue::Set(blog_post_id.clone()),
        title: ActiveValue::Set(request.title),
        subtitle: ActiveValue::Set(request.subtitle),
        thumbnail_url: ActiveValue::Set(request.thumbnail_url),
        blog_md: ActiveValue::Set(request.blog_md),
        posted_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    // Insert blog post
    blog_posts::Entity::insert(blog_post)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create blog post: {}", e)))?;

    Ok(CreateBlogPostResponse {
        success: true,
        message: "Blog post created successfully".to_string(),
        blog_post_id: Some(blog_post_id),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditBlogPostRequest {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub thumbnail_url: Option<String>,
    pub blog_md: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditBlogPostResponse {
    pub success: bool,
    pub message: String,
}

#[server]
pub async fn admin_edit_blog_post(
    request: EditBlogPostRequest,
) -> Result<EditBlogPostResponse, ServerFnError> {
    // Check authentication
    let manager = get_current_manager().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(EditBlogPostResponse {
            success: false,
            message: "Unauthorized".to_string(),
        });
    }

    // Validation logic
    if request.title.trim().is_empty() {
        return Ok(EditBlogPostResponse {
            success: false,
            message: "Blog title is required".to_string(),
        });
    }

    if request.blog_md.trim().is_empty() {
        return Ok(EditBlogPostResponse {
            success: false,
            message: "Blog content is required".to_string(),
        });
    }

    // Validate subtitle length if provided
    if let Some(ref subtitle) = request.subtitle {
        if subtitle.trim().is_empty() {
            return Ok(EditBlogPostResponse {
                success: false,
                message: "Subtitle cannot be empty (leave blank if not needed)".to_string(),
            });
        }
        if subtitle.len() > 200 {
            return Ok(EditBlogPostResponse {
                success: false,
                message: "Subtitle cannot exceed 200 characters".to_string(),
            });
        }
    }

    // Validate thumbnail URL if provided
    if let Some(ref thumbnail) = request.thumbnail_url {
        if thumbnail.trim().is_empty() {
            return Ok(EditBlogPostResponse {
                success: false,
                message: "Thumbnail URL cannot be empty (leave blank if not needed)".to_string(),
            });
        }
    }

    let db = get_db().await;

    // Check if the blog post exists
    let existing_post = blog_posts::Entity::find()
        .filter(blog_posts::Column::Id.eq(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let mut existing_post = match existing_post {
        Some(post) => post,
        None => {
            return Ok(EditBlogPostResponse {
                success: false,
                message: "Blog post not found".to_string(),
            });
        }
    };

    // Check if another blog post with the same title exists (excluding current post)
    let title_conflict = blog_posts::Entity::find()
        .filter(blog_posts::Column::Title.eq(&request.title))
        .filter(blog_posts::Column::Id.ne(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if title_conflict.is_some() {
        return Ok(EditBlogPostResponse {
            success: false,
            message: "Another blog post with this title already exists".to_string(),
        });
    }

    let now = Utc::now().naive_utc();

    // Update blog post
    let mut blog_post: blog_posts::ActiveModel = existing_post.into();
    blog_post.title = ActiveValue::Set(request.title);
    blog_post.subtitle = ActiveValue::Set(request.subtitle);
    blog_post.thumbnail_url = ActiveValue::Set(request.thumbnail_url);
    blog_post.blog_md = ActiveValue::Set(request.blog_md);
    blog_post.updated_at = ActiveValue::Set(now);

    // Save changes
    blog_posts::Entity::update(blog_post)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update blog post: {}", e)))?;

    Ok(EditBlogPostResponse {
        success: true,
        message: "Blog post updated successfully".to_string(),
    })
}

impl Error for StockCalculationError {}

impl StockCalculationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
struct ChildStockAllocation {
    parent_stock_item_id: String,
    child_stock_item_id: String,
    required_quantity: f64,
    allocated_quantity: f64,
    parent_created_at: NaiveDateTime,
}

#[cfg(feature = "server")]
pub async fn get_stock_quantities_for_stock_items(
    stock_item_ids: Option<Vec<String>>,
) -> Result<Vec<StockQuantityResult>, ServerFnError> {
    let db = get_db().await;

    let (target_stock_item_ids, all_needed_stock_item_ids) = match &stock_item_ids {
        Some(ids) => {
            // Load ALL relations once to avoid N+1 queries
            let all_relations =
                stock_item_relations::Entity::find()
                    .all(db)
                    .await
                    .map_err(|e| {
                        StockCalculationError::new(format!("Failed to fetch relations: {}", e))
                    })?;

            // Build a parent->children lookup map
            let mut relations_by_parent: HashMap<String, Vec<String>> = HashMap::new();
            for relation in &all_relations {
                relations_by_parent
                    .entry(relation.parent_stock_item_id.clone())
                    .or_insert_with(Vec::new)
                    .push(relation.child_stock_item_id.clone());
            }

            // BFS to find all descendant stock items using in-memory lookups
            let mut all_needed_stock_item_ids = HashSet::new();
            let mut queue = ids.clone();

            while !queue.is_empty() {
                let mut next_queue = Vec::new();

                for stock_item_id in queue {
                    if !all_needed_stock_item_ids.contains(&stock_item_id) {
                        all_needed_stock_item_ids.insert(stock_item_id.clone());

                        // Get children from our in-memory map
                        if let Some(children) = relations_by_parent.get(&stock_item_id) {
                            next_queue.extend(children.iter().cloned());
                        }
                    }
                }

                queue = next_queue;
            }

            (ids.clone(), Some(all_needed_stock_item_ids))
        }
        None => {
            // For None case, we'll process all items without filtering
            (vec![], None)
        }
    };

    // Load all required data concurrently
    let stock_items_future = match &all_needed_stock_item_ids {
        Some(ids) => stock_items::Entity::find()
            .filter(stock_items::Column::Id.is_in(ids.iter().cloned()))
            .all(db),
        None => stock_items::Entity::find().all(db),
    };

    let relations_future = match &all_needed_stock_item_ids {
        Some(ids) => stock_item_relations::Entity::find()
            .filter(
                stock_item_relations::Column::ParentStockItemId
                    .is_in(ids.iter().cloned())
                    .or(stock_item_relations::Column::ChildStockItemId.is_in(ids.iter().cloned())),
            )
            .all(db),
        None => stock_item_relations::Entity::find().all(db),
    };

    let batches_future = match &all_needed_stock_item_ids {
        Some(ids) => stock_batches::Entity::find()
            .filter(stock_batches::Column::StockItemId.is_in(ids.iter().cloned()))
            .all(db),
        None => stock_batches::Entity::find().all(db),
    };

    // Load all stock active reduces (batch-level)
    let reduces_future = stock_active_reduce::Entity::find().all(db);

    // Load backorder and preorder reduces (stock item level)
    let backorder_reduces_future = stock_backorder_active_reduce::Entity::find().all(db);

    let preorder_reduces_future = stock_preorder_active_reduce::Entity::find().all(db);

    // Execute all database queries concurrently
    let (
        all_stock_items,
        all_relations,
        all_batches,
        all_reduces,
        backorder_reduces,
        preorder_reduces,
    ) = tokio::try_join!(
        stock_items_future,
        relations_future,
        batches_future,
        reduces_future,
        backorder_reduces_future,
        preorder_reduces_future,
    )
    .map_err(|e| StockCalculationError::new(format!("Failed to fetch data: {}", e)))?;

    // For None case, extract all stock item IDs from the fetched data
    let final_target_ids = if target_stock_item_ids.is_empty() {
        all_stock_items.iter().map(|item| item.id.clone()).collect()
    } else {
        target_stock_item_ids
    };

    // Convert to frontend entities
    let stock_items = entity_conversions::convert_stock_items_batch(all_stock_items);
    let stock_batches = entity_conversions::convert_stock_batches_batch(all_batches);
    let stock_relations: Vec<StockItemRelation> = all_relations
        .into_iter()
        .map(StockItemRelation::from)
        .collect();

    // Create lookup maps
    let stock_items_map: HashMap<String, &StockItem> = stock_items
        .iter()
        .map(|item| (item.id.clone(), item))
        .collect();

    let batches_by_stock_item: HashMap<String, Vec<&StockBatch>> =
        stock_batches.iter().fold(HashMap::new(), |mut acc, batch| {
            acc.entry(batch.stock_item_id.clone())
                .or_insert_with(Vec::new)
                .push(batch);
            acc
        });

    // Create reduces lookup map by batch ID
    let reduces_by_batch: HashMap<String, Vec<&stock_active_reduce::Model>> =
        all_reduces.iter().fold(HashMap::new(), |mut acc, reduce| {
            acc.entry(reduce.stock_batch_id.clone())
                .or_insert_with(Vec::new)
                .push(reduce);
            acc
        });

    // Create stock item level reduces lookup maps
    let backorder_reduces_by_stock_item: HashMap<
        String,
        Vec<&stock_backorder_active_reduce::Model>,
    > = backorder_reduces
        .iter()
        .fold(HashMap::new(), |mut acc, reduce| {
            acc.entry(reduce.stock_item_id.clone())
                .or_insert_with(Vec::new)
                .push(reduce);
            acc
        });

    let preorder_reduces_by_stock_item: HashMap<String, Vec<&stock_preorder_active_reduce::Model>> =
        preorder_reduces
            .iter()
            .fold(HashMap::new(), |mut acc, reduce| {
                acc.entry(reduce.stock_item_id.clone())
                    .or_insert_with(Vec::new)
                    .push(reduce);
                acc
            });

    let relations_by_parent: HashMap<String, Vec<&StockItemRelation>> = stock_relations
        .iter()
        .fold(HashMap::new(), |mut acc, relation| {
            acc.entry(relation.parent_stock_item_id.clone())
                .or_insert_with(Vec::new)
                .push(relation);
            acc
        });

    let relations_by_child: HashMap<String, Vec<&StockItemRelation>> =
        stock_relations
            .iter()
            .fold(HashMap::new(), |mut acc, relation| {
                acc.entry(relation.child_stock_item_id.clone())
                    .or_insert_with(Vec::new)
                    .push(relation);
                acc
            });

    // Calculate stock allocations for all child items first
    let child_stock_allocations = calculate_child_stock_allocations(
        &stock_items_map,
        &batches_by_stock_item,
        &relations_by_parent,
        &relations_by_child,
        &reduces_by_batch,
        &backorder_reduces_by_stock_item,
        &preorder_reduces_by_stock_item,
    )?;

    // Calculate results for each target stock item
    let calculation_futures: Vec<_> = final_target_ids
        .iter()
        .map(|stock_item_id| {
            calculate_stock_quantities_with_allocations(
                stock_item_id,
                &stock_items_map,
                &batches_by_stock_item,
                &relations_by_parent,
                &relations_by_child,
                &child_stock_allocations,
                &reduces_by_batch,
                &backorder_reduces_by_stock_item,
                &preorder_reduces_by_stock_item,
            )
        })
        .collect();

    let results = futures::future::try_join_all(calculation_futures)
        .await
        .map_err(|e| StockCalculationError::from(e))?;

    Ok(results)
}

#[cfg(feature = "server")]
pub async fn calculate_stock_quantities(
    stock_item_id: &str,
    stock_items_map: &HashMap<String, &StockItem>,
    batches_by_stock_item: &HashMap<String, Vec<&StockBatch>>,
    relations_by_parent: &HashMap<String, Vec<&StockItemRelation>>,
    relations_by_child: &HashMap<String, Vec<&StockItemRelation>>,
    reduces_by_batch: &HashMap<String, Vec<&stock_active_reduce::Model>>,
    backorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_backorder_active_reduce::Model>>,
    preorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_preorder_active_reduce::Model>>,
) -> Result<StockQuantityResult, StockCalculationError> {
    let stock_item = stock_items_map
        .get(stock_item_id)
        .ok_or_else(|| StockCalculationError::new("Stock item not found"))?;

    // Calculate ready stock (from direct batches)
    let ready_stock_quantity = calculate_ready_stock(
        stock_item,
        batches_by_stock_item,
        reduces_by_batch,
        backorder_reduces_by_stock_item,
        preorder_reduces_by_stock_item,
    );

    // Calculate unready stock (from child items)
    let (unready_stock_quantity, total_child_stock_items) = calculate_unready_stock(
        stock_item_id,
        stock_item,
        stock_items_map,
        batches_by_stock_item,
        relations_by_parent,
        reduces_by_batch,
        backorder_reduces_by_stock_item,
        preorder_reduces_by_stock_item,
        &mut HashSet::new(), // To prevent infinite recursion
    )?;

    // Calculate total stock
    let total_stock_quantity = ready_stock_quantity
        .add(&unready_stock_quantity)
        .map_err(|e| StockCalculationError::new(format!("Cannot add stock quantities: {}", e)))?;

    // Check if stock is too low
    let stock_too_low = if let Some(warning_qty) = stock_item.warning_quantity {
        total_stock_quantity.to_f64() < warning_qty
    } else {
        false
    };

    // Calculate required father replacement level
    let required_father_replacement_level = calculate_required_father_replacement(
        stock_item_id,
        stock_item,
        relations_by_child,
        stock_items_map,
    )?;

    Ok(StockQuantityResult {
        stock_item_id: stock_item_id.to_string(),
        ready_stock_quantity,
        unready_stock_quantity,
        total_stock_quantity,
        total_child_stock_items,
        stock_too_low,
        required_father_replacement_level,
    })
}

#[cfg(feature = "server")]
fn calculate_ready_stock(
    stock_item: &StockItem,
    batches_by_stock_item: &HashMap<String, Vec<&StockBatch>>,
    reduces_by_batch: &HashMap<String, Vec<&stock_active_reduce::Model>>,
    backorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_backorder_active_reduce::Model>>,
    preorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_preorder_active_reduce::Model>>,
) -> StockUnitQuantity {
    let mut ready_stock = if let Some(batches) = batches_by_stock_item.get(&stock_item.id) {
        // Filter for complete batches with matching units
        let complete_batches: Vec<&StockBatch> = batches
            .iter()
            .filter(|batch| {
                batch.status == StockBatchStatus::Complete
                    && batch.stock_unit_on_creation == stock_item.unit
            })
            .copied()
            .collect();

        if !complete_batches.is_empty() {
            // Convert batches to owned and apply reductions
            let mut batches_with_reductions: Vec<StockBatch> = complete_batches
                .into_iter()
                .map(|batch| {
                    let mut batch_clone = batch.clone();
                    // Apply reductions to live_quantity
                    if let Some(reduces) = reduces_by_batch.get(&batch.id) {
                        let total_reduction: f64 = reduces
                            .iter()
                            .filter(|reduce| {
                                StockUnit::from_seaorm(reduce.stock_unit.clone())
                                    == batch.stock_unit_on_creation
                            })
                            .map(|reduce| reduce.reduction_quantity)
                            .sum();

                        // Convert StockUnitQuantity to f64, subtract reduction, then convert back
                        let current_live_quantity = batch.live_quantity.to_f64();
                        let new_live_quantity = (current_live_quantity - total_reduction).max(0.0);

                        // Convert back to StockUnitQuantity based on the unit type
                        batch_clone.live_quantity = match batch.stock_unit_on_creation {
                            StockUnit::Multiples => {
                                StockUnitQuantity::Multiples(new_live_quantity as i32)
                            }
                            StockUnit::Grams => StockUnitQuantity::Grams(new_live_quantity),
                            StockUnit::Milliliters => {
                                StockUnitQuantity::Milliliters(new_live_quantity)
                            }
                        };
                    }
                    batch_clone
                })
                .collect();

            let (_, live_quantity, _) =
                StockBatch::sum_quantities_by_unit(&batches_with_reductions, &stock_item.unit);
            live_quantity
        } else {
            StockUnitQuantity::from(stock_item.unit.clone())
        }
    } else {
        StockUnitQuantity::from(stock_item.unit.clone())
    };

    // Apply stock item level reduces (backorder and preorder)
    let mut total_stock_item_reduction = 0.0;

    // Apply backorder reduces
    if let Some(backorder_reduces) = backorder_reduces_by_stock_item.get(&stock_item.id) {
        let backorder_reduction: f64 = backorder_reduces
            .iter()
            .filter(|reduce| StockUnit::from_seaorm(reduce.stock_unit.clone()) == stock_item.unit)
            .map(|reduce| reduce.reduction_quantity)
            .sum();
        total_stock_item_reduction += backorder_reduction;
    }

    // Apply preorder reduces
    if let Some(preorder_reduces) = preorder_reduces_by_stock_item.get(&stock_item.id) {
        let preorder_reduction: f64 = preorder_reduces
            .iter()
            .filter(|reduce| StockUnit::from_seaorm(reduce.stock_unit.clone()) == stock_item.unit)
            .map(|reduce| reduce.reduction_quantity)
            .sum();
        total_stock_item_reduction += preorder_reduction;
    }

    // Apply the total stock item level reduction
    if total_stock_item_reduction > 0.0 {
        let current_stock = ready_stock.to_f64();
        let new_stock = (current_stock - total_stock_item_reduction).max(0.0);

        ready_stock = match stock_item.unit {
            StockUnit::Multiples => StockUnitQuantity::Multiples(new_stock as i32),
            StockUnit::Grams => StockUnitQuantity::Grams(new_stock),
            StockUnit::Milliliters => StockUnitQuantity::Milliliters(new_stock),
        };
    }

    ready_stock
}

#[cfg(feature = "server")]
fn calculate_unready_stock(
    stock_item_id: &str,
    stock_item: &StockItem,
    stock_items_map: &HashMap<String, &StockItem>,
    batches_by_stock_item: &HashMap<String, Vec<&StockBatch>>,
    relations_by_parent: &HashMap<String, Vec<&StockItemRelation>>,
    reduces_by_batch: &HashMap<String, Vec<&stock_active_reduce::Model>>,
    backorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_backorder_active_reduce::Model>>,
    preorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_preorder_active_reduce::Model>>,
    visited: &mut HashSet<String>,
) -> Result<(StockUnitQuantity, i32), StockCalculationError> {
    if visited.contains(stock_item_id) {
        // Prevent infinite recursion
        return Ok((StockUnitQuantity::from(stock_item.unit.clone()), 0));
    }
    visited.insert(stock_item_id.to_string());

    let mut total_child_count = 0;
    let mut min_unready_quantity: Option<StockUnitQuantity> = None;

    if let Some(child_relations) = relations_by_parent.get(stock_item_id) {
        for relation in child_relations {
            total_child_count += 1;

            if let Some(child_stock_item) = stock_items_map.get(&relation.child_stock_item_id) {
                // Calculate how much of this child is available
                let child_ready_stock = calculate_ready_stock(
                    child_stock_item,
                    batches_by_stock_item,
                    reduces_by_batch,
                    backorder_reduces_by_stock_item,
                    preorder_reduces_by_stock_item,
                );
                let (child_unready_stock, child_descendants) = calculate_unready_stock(
                    &relation.child_stock_item_id,
                    child_stock_item,
                    stock_items_map,
                    batches_by_stock_item,
                    relations_by_parent,
                    reduces_by_batch,
                    backorder_reduces_by_stock_item,
                    preorder_reduces_by_stock_item,
                    visited,
                )?;

                total_child_count += child_descendants;

                // Total child stock available
                let total_child_stock =
                    child_ready_stock.add(&child_unready_stock).map_err(|e| {
                        StockCalculationError::new(format!("Cannot add child stock: {}", e))
                    })?;

                // Calculate how many complete batches of the parent item this child stock can make
                let parent_batches_possible = if relation.quantity > 0.0 {
                    // Floor division to get complete batches only
                    let available_units = (total_child_stock.to_f64() / relation.quantity).floor();

                    // Convert back to parent's unit type
                    match stock_item.unit {
                        StockUnit::Multiples => {
                            StockUnitQuantity::Multiples(available_units as i32)
                        }
                        StockUnit::Grams => StockUnitQuantity::Grams(available_units),
                        StockUnit::Milliliters => StockUnitQuantity::Milliliters(available_units),
                    }
                } else {
                    StockUnitQuantity::from(stock_item.unit.clone())
                };

                // Find the minimum (bottleneck) across all child items
                match min_unready_quantity {
                    None => min_unready_quantity = Some(parent_batches_possible),
                    Some(ref current_min) => {
                        // Take the minimum between current min and this child's contribution
                        if parent_batches_possible.to_f64() < current_min.to_f64() {
                            min_unready_quantity = Some(parent_batches_possible);
                        }
                    }
                }
            }
        }
    }

    visited.remove(stock_item_id);

    // Return the minimum unready quantity (bottleneck) or zero if no children
    let unready_quantity =
        min_unready_quantity.unwrap_or_else(|| StockUnitQuantity::from(stock_item.unit.clone()));
    Ok((unready_quantity, total_child_count))
}

#[cfg(feature = "server")]
fn calculate_required_father_replacement(
    stock_item_id: &str,
    stock_item: &StockItem, // This is the CHILD stock item we're calculating for
    relations_by_child: &HashMap<String, Vec<&StockItemRelation>>,
    stock_items_map: &HashMap<String, &StockItem>,
) -> Result<Option<StockUnitQuantity>, StockCalculationError> {
    if let Some(parent_relations) = relations_by_child.get(stock_item_id) {
        let mut total_replacement_needed = 0.0;

        for relation in parent_relations {
            if let Some(parent_stock_item) = stock_items_map.get(&relation.parent_stock_item_id) {
                if let Some(warning_qty) = parent_stock_item.warning_quantity {
                    // Calculate how much of this child stock item is needed to reach the parent's warning level
                    let child_requirement = warning_qty * relation.quantity;
                    total_replacement_needed += child_requirement;
                }
            }
        }

        if total_replacement_needed > 0.0 {
            // Return the result in the CHILD stock item's unit (not the parent's)
            let result = match stock_item.unit {
                StockUnit::Multiples => {
                    StockUnitQuantity::Multiples(total_replacement_needed as i32)
                }
                StockUnit::Grams => StockUnitQuantity::Grams(total_replacement_needed),
                StockUnit::Milliliters => StockUnitQuantity::Milliliters(total_replacement_needed),
            };
            return Ok(Some(result));
        }
    }

    Ok(None)
}

/// Processes variant stock item relations and calculates available stock for each variant
///
/// # Arguments
/// * `products` - Mutable vector of products whose variants will be updated with calculated stock
/// * `relations` - Vector of product variant to stock item relations from SeaORM
/// * `stock_results` - Vector of stock quantity results for stock items
///
/// # Returns
/// * Updated vector of products with calculated_stock_quantity set for each variant
#[cfg(feature = "server")]
pub fn calculate_variant_stock_quantities(
    mut products: Vec<Product>,
    relations: Vec<product_variant_stock_item_relations::Model>,
    stock_results: Vec<StockQuantityResult>,
) -> Vec<Product> {
    // Create a lookup map for stock results by stock_item_id for O(1) access
    let stock_lookup: HashMap<String, &StockQuantityResult> = stock_results
        .iter()
        .map(|result| (result.stock_item_id.clone(), result))
        .collect();

    // Group relations by product_variant_id for easier processing
    let mut variant_relations: HashMap<String, Vec<&product_variant_stock_item_relations::Model>> =
        HashMap::new();
    for relation in &relations {
        variant_relations
            .entry(relation.product_variant_id.clone())
            .or_insert_with(Vec::new)
            .push(relation);
    }

    // Process each product and its variants
    for product in &mut products {
        if let Some(ref mut variants) = product.variants {
            for variant in variants.iter_mut() {
                variant.calculated_stock_quantity = Some(calculate_variant_available_stock(
                    &variant.id,
                    &variant_relations,
                    &stock_lookup,
                ));
            }
        }
    }

    products
}

/// Calculates the available stock for a single variant based on its stock item relations
///
/// # Arguments
/// * `variant_id` - ID of the variant to calculate stock for
/// * `variant_relations` - Map of variant IDs to their stock item relations
/// * `stock_lookup` - Map of stock item IDs to their stock quantity results
///
/// # Returns
/// * Available stock quantity as i32 (0 if any required stock item is unavailable)
#[cfg(feature = "server")]
pub fn calculate_variant_available_stock(
    variant_id: &str,
    variant_relations: &HashMap<String, Vec<&product_variant_stock_item_relations::Model>>,
    stock_lookup: &HashMap<String, &StockQuantityResult>,
) -> i32 {
    // Get relations for this variant, return 0 if none found
    let relations = match variant_relations.get(variant_id) {
        Some(relations) => relations,
        None => return 0,
    };

    // If variant has no stock item relations, default to 0
    if relations.is_empty() {
        return 0;
    }

    let mut min_available_stock = i32::MAX;

    // Calculate available quantity for each required stock item
    for relation in relations {
        let stock_result = match stock_lookup.get(&relation.stock_item_id) {
            Some(result) => result,
            None => {
                // If any required stock item has no stock data, variant stock is 0
                return 0;
            }
        };

        // Calculate how many units of this variant can be made with this stock item
        let available_units = calculate_available_units_from_stock(
            &stock_result.total_stock_quantity,
            relation.quantity,
            &StockUnit::from_seaorm(relation.stock_unit_on_creation.clone()),
        );

        // If any stock item has 0 availability, the variant is out of stock
        if available_units == 0 {
            return 0;
        }

        // Track the minimum - this will be our bottleneck
        min_available_stock = min_available_stock.min(available_units);
    }

    // If we never found any valid stock item, return 0
    if min_available_stock == i32::MAX {
        0
    } else {
        min_available_stock
    }
}

/// Calculates how many variant units can be produced given available stock
///
/// # Arguments
/// * `total_stock` - Total available stock quantity from StockQuantityResult
/// * `required_quantity` - Quantity of stock item required per variant unit
/// * `stock_unit` - Unit type of the stock item
///
/// # Returns
/// * Number of variant units that can be produced with available stock
#[cfg(feature = "server")]
fn calculate_available_units_from_stock(
    total_stock: &StockUnitQuantity,
    required_quantity: f64,
    stock_unit: &StockUnit,
) -> i32 {
    // Convert stock quantity to f64 for calculation
    let available_stock = total_stock.to_f64();

    // If no stock available, return 0
    if available_stock <= 0.0 {
        return 0;
    }

    // If no quantity required (shouldn't happen, but guard against division by 0)
    if required_quantity <= 0.0 {
        return 0;
    }

    // Calculate how many complete units can be made
    let units_possible = available_stock / required_quantity;

    // Floor the result to get complete units only
    units_possible.floor() as i32
}

// Stock Allocations

#[cfg(feature = "server")]
fn calculate_child_stock_allocations(
    stock_items_map: &HashMap<String, &StockItem>,
    batches_by_stock_item: &HashMap<String, Vec<&StockBatch>>,
    relations_by_parent: &HashMap<String, Vec<&StockItemRelation>>,
    relations_by_child: &HashMap<String, Vec<&StockItemRelation>>,
    reduces_by_batch: &HashMap<String, Vec<&stock_active_reduce::Model>>,
    backorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_backorder_active_reduce::Model>>,
    preorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_preorder_active_reduce::Model>>,
) -> Result<HashMap<String, HashMap<String, f64>>, StockCalculationError> {
    let mut allocations: HashMap<String, HashMap<String, f64>> = HashMap::new();

    // First, calculate total available stock for each child item
    let mut child_total_stock: HashMap<String, f64> = HashMap::new();

    for stock_item in stock_items_map.values() {
        let ready_stock = calculate_ready_stock(
            stock_item,
            batches_by_stock_item,
            reduces_by_batch,
            backorder_reduces_by_stock_item,
            preorder_reduces_by_stock_item,
        );
        let total_stock = ready_stock.to_f64();

        if total_stock > 0.0 {
            child_total_stock.insert(stock_item.id.clone(), total_stock);
        }
    }

    // For each child stock item that has parents, calculate demand and allocate
    for (child_id, parent_relations) in relations_by_child {
        if parent_relations.is_empty() {
            continue;
        }

        let available_stock = child_total_stock.get(child_id).copied().unwrap_or(0.0);
        if available_stock <= 0.0 {
            continue;
        }

        // Debug logging
        tracing::info!(
            "Child {} has {} available stock, {} parents",
            child_id,
            available_stock,
            parent_relations.len()
        );

        // If only one parent, give all stock to that parent
        if parent_relations.len() == 1 {
            let parent_id = &parent_relations[0].parent_stock_item_id;
            allocations
                .entry(parent_id.clone())
                .or_insert_with(HashMap::new)
                .insert(child_id.clone(), available_stock);

            tracing::info!(
                "Single parent {}, allocated all {} stock",
                parent_id,
                available_stock
            );
            continue;
        }

        // Multiple parents - need to distribute the stock fairly
        let mut parent_info: Vec<(String, f64, NaiveDateTime)> = Vec::new();

        for relation in parent_relations {
            if let Some(parent_item) = stock_items_map.get(&relation.parent_stock_item_id) {
                parent_info.push((
                    relation.parent_stock_item_id.clone(),
                    relation.quantity,
                    parent_item.created_at,
                ));
                tracing::info!(
                    "Parent {} needs {} per unit, created: {:?}",
                    relation.parent_stock_item_id,
                    relation.quantity,
                    parent_item.created_at
                );
            }
        }

        if parent_info.is_empty() {
            continue;
        }

        // Sort by created_at (oldest first) for priority when stock is insufficient
        parent_info.sort_by(|a, b| a.2.cmp(&b.2));

        // Calculate the total demand for making 1 unit of each parent
        let total_demand_for_one_unit: f64 = parent_info.iter().map(|(_, qty, _)| qty).sum();
        let max_units_all_parents = if total_demand_for_one_unit > 0.0 {
            available_stock / total_demand_for_one_unit
        } else {
            0.0
        };

        tracing::info!(
            "Total demand for 1 unit of each: {}, Max units if all parents: {}",
            total_demand_for_one_unit,
            max_units_all_parents
        );

        if max_units_all_parents >= 1.0 {
            // We can make at least 1 unit for all parents - allocate proportionally
            let units_to_make = max_units_all_parents.floor();
            for (parent_id, qty_needed, _) in parent_info {
                let allocation = units_to_make * qty_needed;
                if allocation > 0.0 {
                    allocations
                        .entry(parent_id.clone())
                        .or_insert_with(HashMap::new)
                        .insert(child_id.clone(), allocation);

                    tracing::info!(
                        "Allocated {} to parent {} (for {} units - all parents mode)",
                        allocation,
                        parent_id,
                        units_to_make
                    );
                }
            }
        } else {
            // Not enough stock for all parents - prioritize by created_at date
            tracing::info!("Insufficient stock for all parents, using priority allocation");
            let mut remaining_stock = available_stock;

            for (parent_id, qty_needed, created_at) in parent_info {
                if remaining_stock <= 0.0 {
                    tracing::info!("No remaining stock for parent {}", parent_id);
                    break;
                }

                // Calculate how many complete units this parent can make with remaining stock
                let units_possible = if qty_needed > 0.0 {
                    (remaining_stock / qty_needed).floor()
                } else {
                    0.0
                };

                if units_possible >= 1.0 {
                    let allocation = units_possible * qty_needed;
                    remaining_stock -= allocation;

                    allocations
                        .entry(parent_id.clone())
                        .or_insert_with(HashMap::new)
                        .insert(child_id.clone(), allocation);

                    tracing::info!(
                        "Priority allocated {} to parent {} (for {} units, remaining: {})",
                        allocation,
                        parent_id,
                        units_possible,
                        remaining_stock
                    );
                } else {
                    tracing::info!(
                        "Parent {} cannot make even 1 unit (needs {}, has {})",
                        parent_id,
                        qty_needed,
                        remaining_stock
                    );
                }
            }
        }
    }

    Ok(allocations)
}

#[cfg(feature = "server")]
pub async fn calculate_stock_quantities_with_allocations(
    stock_item_id: &str,
    stock_items_map: &HashMap<String, &StockItem>,
    batches_by_stock_item: &HashMap<String, Vec<&StockBatch>>,
    relations_by_parent: &HashMap<String, Vec<&StockItemRelation>>,
    relations_by_child: &HashMap<String, Vec<&StockItemRelation>>,
    child_stock_allocations: &HashMap<String, HashMap<String, f64>>,
    reduces_by_batch: &HashMap<String, Vec<&stock_active_reduce::Model>>,
    backorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_backorder_active_reduce::Model>>,
    preorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_preorder_active_reduce::Model>>,
) -> Result<StockQuantityResult, StockCalculationError> {
    let stock_item = stock_items_map
        .get(stock_item_id)
        .ok_or_else(|| StockCalculationError::new("Stock item not found"))?;

    // Calculate ready stock (from direct batches)
    let ready_stock_quantity = calculate_ready_stock(
        stock_item,
        batches_by_stock_item,
        reduces_by_batch,
        backorder_reduces_by_stock_item,
        preorder_reduces_by_stock_item,
    );

    // Calculate unready stock using allocations
    let (unready_stock_quantity, total_child_stock_items) =
        calculate_unready_stock_with_allocations(
            stock_item_id,
            stock_item,
            stock_items_map,
            relations_by_parent,
            child_stock_allocations,
            reduces_by_batch,
            backorder_reduces_by_stock_item,
            preorder_reduces_by_stock_item,
            &mut HashSet::new(),
        )?;

    // Calculate total stock
    let total_stock_quantity = ready_stock_quantity
        .add(&unready_stock_quantity)
        .map_err(|e| StockCalculationError::new(format!("Cannot add stock quantities: {}", e)))?;

    // Check if stock is too low
    let stock_too_low = if let Some(warning_qty) = stock_item.warning_quantity {
        total_stock_quantity.to_f64() < warning_qty
    } else {
        false
    };

    // Calculate required father replacement level
    let required_father_replacement_level = calculate_required_father_replacement(
        stock_item_id,
        stock_item,
        relations_by_child,
        stock_items_map,
    )?;

    Ok(StockQuantityResult {
        stock_item_id: stock_item_id.to_string(),
        ready_stock_quantity,
        unready_stock_quantity,
        total_stock_quantity,
        total_child_stock_items,
        stock_too_low,
        required_father_replacement_level,
    })
}

#[cfg(feature = "server")]
fn calculate_unready_stock_with_allocations(
    stock_item_id: &str,
    stock_item: &StockItem,
    stock_items_map: &HashMap<String, &StockItem>,
    relations_by_parent: &HashMap<String, Vec<&StockItemRelation>>,
    child_stock_allocations: &HashMap<String, HashMap<String, f64>>,
    reduces_by_batch: &HashMap<String, Vec<&stock_active_reduce::Model>>,
    backorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_backorder_active_reduce::Model>>,
    preorder_reduces_by_stock_item: &HashMap<String, Vec<&stock_preorder_active_reduce::Model>>,
    visited: &mut HashSet<String>,
) -> Result<(StockUnitQuantity, i32), StockCalculationError> {
    if visited.contains(stock_item_id) {
        return Ok((StockUnitQuantity::from(stock_item.unit.clone()), 0));
    }
    visited.insert(stock_item_id.to_string());

    let mut total_child_count = 0;
    let mut min_unready_quantity: Option<f64> = None;

    tracing::info!("Calculating unready for parent: {}", stock_item_id);

    if let Some(child_relations) = relations_by_parent.get(stock_item_id) {
        for relation in child_relations {
            total_child_count += 1;

            if let Some(child_stock_item) = stock_items_map.get(&relation.child_stock_item_id) {
                // Get allocated stock for this parent-child relationship
                let allocated_child_stock = child_stock_allocations
                    .get(stock_item_id)
                    .and_then(|allocations| allocations.get(&relation.child_stock_item_id))
                    .copied()
                    .unwrap_or(0.0);

                tracing::info!(
                    "  Child: {}, Required: {}, Allocated: {}",
                    relation.child_stock_item_id,
                    relation.quantity,
                    allocated_child_stock
                );

                // Also calculate child's unready stock recursively
                let (child_unready_stock, child_descendants) =
                    calculate_unready_stock_with_allocations(
                        &relation.child_stock_item_id,
                        child_stock_item,
                        stock_items_map,
                        relations_by_parent,
                        child_stock_allocations,
                        reduces_by_batch,
                        backorder_reduces_by_stock_item,
                        preorder_reduces_by_stock_item,
                        visited,
                    )?;

                total_child_count += child_descendants;

                // Total child stock available (allocated ready stock + unready stock)
                let total_child_stock = allocated_child_stock + child_unready_stock.to_f64();

                tracing::info!(
                    "  Total child stock available: {} (allocated: {} + unready: {})",
                    total_child_stock,
                    allocated_child_stock,
                    child_unready_stock.to_f64()
                );

                // Calculate how many complete batches of the parent item this child stock can make
                let parent_batches_possible = if relation.quantity > 0.0 && total_child_stock > 0.0
                {
                    // This gives us how many parent units we can make with this child stock
                    let available_units = total_child_stock / relation.quantity;

                    tracing::info!("  Raw available units: {}", available_units);

                    // For multiples, we need to floor to get complete units
                    match stock_item.unit {
                        StockUnit::Multiples => available_units.floor(),
                        StockUnit::Grams => available_units,
                        StockUnit::Milliliters => available_units,
                    }
                } else {
                    0.0
                };

                tracing::info!(
                    "  Parent batches possible from this child: {}",
                    parent_batches_possible
                );

                // Find the minimum (bottleneck) across all child items
                match min_unready_quantity {
                    None => {
                        min_unready_quantity = Some(parent_batches_possible);
                        tracing::info!(
                            "  First child, setting min to: {}",
                            parent_batches_possible
                        );
                    }
                    Some(current_min) => {
                        if parent_batches_possible < current_min {
                            min_unready_quantity = Some(parent_batches_possible);
                            tracing::info!(
                                "  New bottleneck: {} < {}",
                                parent_batches_possible,
                                current_min
                            );
                        } else {
                            tracing::info!(
                                "  Not a bottleneck: {} >= {}",
                                parent_batches_possible,
                                current_min
                            );
                        }
                    }
                }
            }
        }
    }

    visited.remove(stock_item_id);

    let unready_value = min_unready_quantity.unwrap_or(0.0);
    tracing::info!(
        "Final unready value for {}: {}",
        stock_item_id,
        unready_value
    );

    let unready_quantity = match stock_item.unit {
        StockUnit::Multiples => StockUnitQuantity::Multiples(unready_value as i32),
        StockUnit::Grams => StockUnitQuantity::Grams(unready_value),
        StockUnit::Milliliters => StockUnitQuantity::Milliliters(unready_value),
    };

    Ok((unready_quantity, total_child_count))
}

// AUTH LOGIC

#[server]
pub async fn send_magic_link(email: String) -> Result<AuthResponse, ServerFnError> {
    let db = get_db().await;

    // Check if manager exists
    let manager = managers::Entity::find()
        .filter(managers::Column::Email.eq(&email))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if manager.is_none() {
        return Ok(AuthResponse {
            success: false,
            message: "Manager not found. Please contact an administrator.".to_string(),
        });
    }

    // Read environment variables manually
    let project_url = std::env::var("SUPABASE_URL")
        .map_err(|_| ServerFnError::new("SUPABASE_URL not found".to_string()))?;
    let api_key = std::env::var("SUPABASE_ANON_KEY")
        .map_err(|_| ServerFnError::new("SUPABASE_ANON_KEY not found".to_string()))?;
    let jwt_secret = std::env::var("SUPABASE_JWT_SECRET")
        .map_err(|_| ServerFnError::new("SUPABASE_JWT_SECRET not found".to_string()))?;

    // Create Supabase client with manual configuration
    let supabase_client = AuthClient::new(project_url, api_key, jwt_secret);

    match supabase_client
        .send_login_email_with_magic_link(&email)
        .await
    {
        Ok(_) => Ok(AuthResponse {
            success: true,
            message: "A sign in link has been sent to your email.".to_string(),
        }),
        Err(e) => Ok(AuthResponse {
            success: false,
            message: format!("Error sending magic link: {:?}", e),
        }),
    }
}

#[server]
pub async fn verify_magic_link(access_token: String) -> Result<AuthResponse, ServerFnError> {
    eprintln!("=== DEBUG: access_token = {} ===", access_token);

    let db = get_db().await;

    // Verify the JWT token from Supabase
    let jwt_secret = std::env::var("SUPABASE_JWT_SECRET")
        .map_err(|_| ServerFnError::new("SUPABASE_JWT_SECRET not found".to_string()))?;

    let key = DecodingKey::from_secret(jwt_secret.as_ref());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["authenticated"]);

    let raw_token_data = decode::<serde_json::Value>(&access_token, &key, &validation)
        .map_err(|e| ServerFnError::new(format!("Token verification failed: {}", e)))?;

    println!(
        "DEBUG: Raw token JSON = {}",
        serde_json::to_string_pretty(&raw_token_data.claims).unwrap_or_default()
    );

    // Now decode into your struct
    let token_data = decode::<SupabaseTokenClaims>(&access_token, &key, &validation)
        .map_err(|e| ServerFnError::new(format!("Token verification failed: {}", e)))?;

    let email = token_data
        .claims
        .email
        .ok_or_else(|| ServerFnError::new("Email not found in token".to_string()))?;

    // Find the manager by email
    let manager = managers::Entity::find()
        .filter(managers::Column::Email.eq(&email))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("Manager not found".to_string()))?;

    // Create a new session
    let session_token = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();
    let expires_at = now + Duration::days(7);

    let session = manager_sessions::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4().to_string()),
        manager_id: ActiveValue::Set(manager.id.clone()),
        token: ActiveValue::Set(session_token.clone()),
        expires_at: ActiveValue::Set(expires_at),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    manager_sessions::Entity::insert(session)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create session: {}", e)))?;

    // Set the session cookie
    #[cfg(feature = "server")]
    {
        use axum::http::HeaderValue;
        use dioxus::fullstack::FullstackContext;

        let cookie_value = format!(
            "session_token={}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=7776000",
            session_token
        );

        // Get the server context and set the cookie
        let server_ctx = FullstackContext::current()
            .expect("Server context should be available");
        let header_value = HeaderValue::from_str(&cookie_value)
            .map_err(|e| ServerFnError::new(format!("Invalid cookie value: {}", e)))?;

        server_ctx.add_response_header(axum::http::header::SET_COOKIE, header_value);
    }

    Ok(AuthResponse {
        success: true,
        message: "Authentication successful".to_string(),
    })
}

#[server]
pub async fn get_current_manager() -> Result<Option<Manager>, ServerFnError> {
    let db = get_db().await;

    // Get session token from cookies
    #[cfg(feature = "server")]
    {
        let session_token = extract_session_token_from_request().await?;

        if let Some(token) = session_token {
            return validate_session_and_get_manager(&token, db).await;
        }
    }

    Ok(None)
}

#[cfg(feature = "server")]
async fn extract_session_token_from_request() -> Result<Option<String>, ServerFnError> {
    use dioxus::fullstack::FullstackContext;
    
    let server_ctx = FullstackContext::current()
        .expect("Server context should be available");

    // Store the request parts to avoid borrowing from temporary values
    let request_parts = server_ctx.parts_mut();

    // Get the Cookie header from the request
    let cookie_header = request_parts
        .headers
        .get("cookie")
        .or_else(|| request_parts.headers.get("Cookie"));

    if let Some(cookie_value) = cookie_header {
        let cookie_str = cookie_value
            .to_str()
            .map_err(|e| ServerFnError::new(format!("Invalid cookie header: {}", e)))?;

        // Parse cookies to find session_token
        for cookie in cookie_str.split(';') {
            let cookie = cookie.trim();
            if let Some((name, value)) = cookie.split_once('=') {
                if name.trim() == "session_token" {
                    return Ok(Some(value.trim().to_string()));
                }
            }
        }
    }

    Ok(None)
}

#[cfg(feature = "server")]
async fn validate_session_and_get_manager(
    token: &str,
    db: &sea_orm::DatabaseConnection,
) -> Result<Option<Manager>, ServerFnError> {
    let now = Utc::now().naive_utc();

    // Find valid session
    let session = manager_sessions::Entity::find()
        .filter(manager_sessions::Column::Token.eq(token))
        .filter(manager_sessions::Column::ExpiresAt.gt(now))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if let Some(session) = session {
        // Get the manager
        let manager = managers::Entity::find_by_id(&session.manager_id)
            .one(db)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        if let Some(manager) = manager {
            return Ok(Some(Manager {
                id: manager.id,
                email: manager.email,
                name: manager.name,
                permissions: format!("{:?}", manager.permissions), // Use Debug formatting
                authenticated: true,
            }));
        }
    }

    Ok(None)
}

#[server]
pub async fn check_auth() -> Result<bool, ServerFnError> {
    match get_current_manager().await? {
        Some(manager) => Ok(manager.authenticated),
        None => Ok(false),
    }
}

#[server]
pub async fn check_admin_permission() -> Result<bool, ServerFnError> {
    match get_current_manager().await? {
        Some(manager) => {
            // Check if manager has admin permissions
            // You can customize this based on your permission system
            Ok(manager.authenticated && !manager.permissions.is_empty())
        }
        None => Ok(false),
    }
}

#[server]
pub async fn logout_manager() -> Result<AuthResponse, ServerFnError> {
    let db = get_db().await;

    // Get session token from cookies
    let session_token = extract_session_token_from_request().await?;

    if let Some(token) = session_token {
        // Delete the session from database
        manager_sessions::Entity::delete_many()
            .filter(manager_sessions::Column::Token.eq(&token))
            .exec(db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete session: {}", e)))?;
    }

    // Clear the cookie by setting it with Max-Age=0
    #[cfg(feature = "server")]
    {
        use axum::http::HeaderValue;
        use dioxus::fullstack::FullstackContext;

        let cookie_value = "session_token=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0";

        // Get the server context and set the expired cookie
        let server_ctx = FullstackContext::current()
            .expect("Server context should be available");
        let header_value = HeaderValue::from_str(cookie_value)
            .map_err(|e| ServerFnError::new(format!("Invalid cookie value: {}", e)))?;

        server_ctx.add_response_header(axum::http::header::SET_COOKIE, header_value);
    }

    Ok(AuthResponse {
        success: true,
        message: "Logged out successfully".to_string(),
    })
}

// Legacy function for backward compatibility
#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    logout_manager().await?;
    Ok(())
}

// Helper function to get manager by ID (for internal use)
#[server]
pub async fn admin_get_manager_by_id(manager_id: String) -> Result<Option<Manager>, ServerFnError> {
    let db = get_db().await;

    let manager = managers::Entity::find_by_id(&manager_id)
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if let Some(manager) = manager {
        Ok(Some(Manager {
            id: manager.id,
            email: manager.email,
            name: manager.name,
            permissions: format!("{:?}", manager.permissions), // Use Debug formatting
            authenticated: true,
        }))
    } else {
        Ok(None)
    }
}

// Helper function to check if email exists in managers table
#[server]
pub async fn admin_check_manager_email_exists(email: String) -> Result<bool, ServerFnError> {
    let db = get_db().await;

    let manager = managers::Entity::find()
        .filter(managers::Column::Email.eq(&email))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    Ok(manager.is_some())
}

// Function to clean up expired sessions (can be called periodically)
#[server]
pub async fn cleanup_expired_sessions() -> Result<u64, ServerFnError> {
    let db = get_db().await;
    let now = Utc::now().naive_utc();

    let result = manager_sessions::Entity::delete_many()
        .filter(manager_sessions::Column::ExpiresAt.lt(now))
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete expired sessions: {}", e)))?;

    Ok(result.rows_affected)
}