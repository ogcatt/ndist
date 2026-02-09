// src/backend/server_functions/basket.rs

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use super::super::entity_conversions;

#[cfg(feature = "server")]
use super::super::shipping_calculations::{
    calculate_shipping_cost, calculate_shipping_cost_with_preorder_surcharge,
    round_up_to_nearest_quarter,
};

#[cfg(feature = "server")]
use entity::{
    basket_items, customer_baskets, discounts, product_variant_stock_item_relations,
    product_variants, products, sea_orm_active_enums,
};

#[cfg(feature = "server")]
use sea_orm::{
    self, ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    TransactionTrait,
};

#[cfg(feature = "server")]
use chrono::Utc;

#[cfg(feature = "server")]
use uuid::Uuid;

use super::super::front_entities::*;
use super::discounts::check_discount;
use super::stock_calculations::{StockCalculationError, super::front_entities::StockQuantityResult};

#[cfg(feature = "server")]
use super::stock_calculations::get_stock_quantities_for_stock_items;

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

#[cfg(feature = "server")]
pub type CustomerBasketItems = Vec<CustomerBasketItem>;

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddToBasketResponse {
    pub status: String,
    pub basket: CustomerBasket,
}

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

#[server]
pub async fn get_basket() -> Result<CustomerBasket, ServerFnError> {
    get_or_create_basket().await
}

#[server]
pub async fn get_or_create_basket() -> Result<CustomerBasket, ServerFnError> {
    let db = get_db().await;

    if let Some(basket_id) = get_basket_cookie().await? {
        let (
            basket_result,
            basket_items_result,
            products_with_variants_result,
            variant_relations_result,
            stock_qty_results_result,
            discounts_result,
        ) = tokio::join!(
            customer_baskets::Entity::find_by_id(&basket_id).one(db),
            basket_items::Entity::find()
                .filter(basket_items::Column::BasketId.eq(&basket_id))
                .all(db),
            products::Entity::find()
                .filter(
                    products::Column::Visibility
                        .eq(sea_orm_active_enums::ProductVisibility::Public)
                )
                .find_with_related(product_variants::Entity)
                .all(db),
            product_variant_stock_item_relations::Entity::find().all(db),
            get_stock_quantities_for_stock_items(None),
            discounts::Entity::find().all(db)
        );

        let basket = basket_result.map_db_err()?;
        let mut basket_items = basket_items_result.map_db_err()?;
        let products_with_variants = products_with_variants_result.map_db_err()?;
        let variant_relations = variant_relations_result.map_db_err()?;
        let stock_qty_results = stock_qty_results_result?;
        let discounts = discounts_result.map_db_err()?;

        if let Some(basket_model) = basket {
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

            let mut customer_basket = CustomerBasket::from(basket_model.clone());
            customer_basket.items = if basket_items.is_empty() {
                None
            } else {
                Some(entity_conversions::convert_basket_items_batch(
                    basket_items.clone(),
                ))
            };

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
                            discount_type: DiscountType::from_seaorm(
                                discount_match.discount_type.clone(),
                            ),
                            discount_percentage: match DiscountType::from_seaorm(
                                discount_match.discount_type.clone(),
                            ) {
                                DiscountType::Percentage | DiscountType::PercentageOnShipping => {
                                    Some(discount_match.discount_percentage.expect(
                                        "Discount percentage type does not have discount_percentage",
                                    ))
                                }
                                DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => {
                                    None
                                }
                            },
                            discount_amount_left: match DiscountType::from_seaorm(
                                discount_match.discount_type.clone(),
                            ) {
                                DiscountType::Percentage | DiscountType::PercentageOnShipping => {
                                    None
                                }
                                DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => {
                                    Some(
                                        discount_match
                                            .discount_amount
                                            .expect("Discount amount type does not have discount amount")
                                            - discount_match.amount_used.unwrap_or(0.0),
                                    )
                                }
                            },
                            discount_auto_apply: discount_match.auto_apply,
                        })
                    }
                    Err(_validation_error) => {
                        let mut basket_update: customer_baskets::ActiveModel =
                            basket_model.into();
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

#[cfg(feature = "server")]
async fn calculate_shipping_results(
    country_code: &str,
    basket_items: &[basket_items::Model],
    products_with_variants: &[(products::Model, Vec<product_variants::Model>)],
) -> Option<Vec<ShippingResult>> {
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
        (true, true) => Some(Vec::new()),
        (false, true) => {
            if let Some(shipping_quote) =
                calculate_shipping_cost(country_code, regular_weight as u32, regular_cost)
            {
                Some(shipping_quote.available_options)
            } else {
                Some(Vec::new())
            }
        }
        (true, false) => {
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
                    let mut option_types = std::collections::HashSet::new();
                    for result in &regular.available_options {
                        option_types.insert(&result.option);
                    }
                    for result in &preorder.available_options {
                        option_types.insert(&result.option);
                    }

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
                                estimated_days: regular_r.estimated_days.clone(),
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

#[cfg(feature = "server")]
pub fn calculate_total_cart_weight(
    basket_items: &[basket_items::Model],
    products_with_variants: &[(products::Model, Vec<product_variants::Model>)],
    exclude_pre_orders: bool,
) -> f64 {
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
        if let Some(variant) = variant_map.get(&basket_item.variant_id) {
            if let Some(product) = product_map.get(&variant.product_id) {
                if exclude_pre_orders && product.pre_order {
                    continue;
                }
            }

            let item_weight_grams = if let Some(variant_weight) = variant.weight {
                variant_weight
            } else if let Some(product) = product_map.get(&variant.product_id) {
                if let Some(product_weight) = product.weight {
                    product_weight
                } else {
                    80.0
                }
            } else {
                80.0
            };

            total_weight_grams += item_weight_grams * (basket_item.quantity as f64);
        } else {
            total_weight_grams += 80.0 * (basket_item.quantity as f64);
        }
    }

    total_weight_grams = (total_weight_grams * 1.05) + 30.0;
    total_weight_grams
}

#[cfg(feature = "server")]
pub async fn check_cart(
    mut basket_items: Vec<basket_items::Model>,
    products_with_variants: Vec<(products::Model, Vec<product_variants::Model>)>,
    variant_relations: Vec<product_variant_stock_item_relations::Model>,
    stock_quantities: Vec<StockQuantityResult>,
) -> Result<(Vec<basket_items::Model>, Vec<CheckCartResult>), ServerFnError> {
    let db = get_db().await;
    let mut results = Vec::new();
    let mut items_to_remove = Vec::new();

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

        if let (Some(_variant), Some(relations)) = (
            variant_map.get(variant_id),
            variant_relations_map.get(variant_id),
        ) {
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
                results.push(CheckCartResult::Complete);
            } else {
                let mut max_possible_quantity = i32::MAX;
                let mut has_zero_stock = false;

                for relation in relations {
                    if let Some(stock_result) = stock_map.get(&relation.stock_item_id) {
                        let required_per_item = &relation.quantity;
                        let available_stock = &stock_result.total_stock_quantity;

                        if available_stock.is_zero() {
                            has_zero_stock = true;
                            break;
                        }

                        let available_f64 = available_stock.to_f64();
                        let required_f64 = required_per_item;

                        if *required_f64 > 0.0 {
                            let possible_quantity = (available_f64 / required_f64).floor() as i32;
                            max_possible_quantity = max_possible_quantity.min(possible_quantity);
                        }
                    }
                }

                if has_zero_stock || max_possible_quantity == 0 {
                    items_to_remove.push(index);
                    results.push(CheckCartResult::Removed);
                } else if max_possible_quantity < basket_item.quantity {
                    basket_item.quantity = max_possible_quantity;
                    results.push(CheckCartResult::Reduced);
                } else {
                    results.push(CheckCartResult::Complete);
                }
            }
        } else {
            items_to_remove.push(index);
            results.push(CheckCartResult::Error(format!(
                "Variant {} not found or has no stock relations",
                variant_id
            )));
        }
    }

    for &index in items_to_remove.iter().rev() {
        let removed_item = basket_items.remove(index);
        basket_items::Entity::delete_by_id(&removed_item.id)
            .exec(db)
            .await
            .map_db_err()?;
    }

    for (basket_item, result) in basket_items.iter().zip(results.iter()) {
        if matches!(result, CheckCartResult::Reduced) {
            let mut active_model: basket_items::ActiveModel = basket_item.clone().into();
            active_model.quantity = ActiveValue::Set(basket_item.quantity);
            basket_items::Entity::update(active_model)
                .exec(db)
                .await
                .map_db_err()?;
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
        customer_id: ActiveValue::NotSet,
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

    set_basket_cookie(&basket_id)
        .await
        .map_err(|e| BasketError::CookieError(e.to_string()))?;

    let mut customer_basket = CustomerBasket::from(basket_model);
    customer_basket.items = None;

    Ok(customer_basket)
}

#[cfg(feature = "server")]
async fn get_basket_cookie() -> Result<Option<String>, CookieError> {
    match get_basket_id_from_cookie().await {
        Ok(basket_id) => Ok(basket_id),
        Err(e) => Err(CookieError::ExtractionError(e.to_string())),
    }
}

#[cfg(feature = "server")]
async fn set_basket_cookie(basket_id: &str) -> Result<(), CookieError> {
    set_basket_id_cookie(basket_id.to_string())
        .await
        .map_err(|e| CookieError::SettingError(e.to_string()))
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

    let server_ctx = FullstackContext::current().expect("Server context should be available");
    let request_parts = server_ctx.parts_mut();

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
    use axum::http::{header::SET_COOKIE, HeaderValue};
    use dioxus::fullstack::FullstackContext;

    let cookie_value = format!(
        "customer_basket_id={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        basket_id,
        60 * 60 * 24 * 30
    );

    let server_ctx = FullstackContext::current().expect("Server context should be available");
    let header_value = HeaderValue::from_str(&cookie_value)
        .map_err(|e| CookieError::SettingError(format!("Invalid cookie value: {}", e)))?;

    server_ctx.add_response_header(SET_COOKIE, header_value);

    Ok(())
}

#[server]
pub async fn add_or_update_basket_item(
    variant_id: String,
    requested_quantity: i32,
) -> Result<AddToBasketResponse, ServerFnError> {
    use sea_orm::{ActiveModelTrait, ActiveValue, TransactionTrait};

    let db = get_db().await;

    if requested_quantity <= 0 {
        let basket = get_or_create_basket().await?;
        return Ok(AddToBasketResponse {
            status: "Invalid".to_string(),
            basket,
        });
    }

    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();

    if basket.locked {
        panic!("Basket locked")
    }

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

    let stock_map: std::collections::HashMap<String, StockQuantityResult> = stock_quantities
        .into_iter()
        .map(|sq| (sq.stock_item_id.clone(), sq))
        .collect();

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
        let basket = get_or_create_basket().await?;
        return Ok(AddToBasketResponse {
            status: "NotFound".to_string(),
            basket,
        });
    }

    let parent_product = parent_product.expect("Parent product should exist if variant exists");
    let is_back_order = parent_product.back_order;
    let is_pre_order = parent_product.pre_order;

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

    let max_possible = if is_back_order || is_pre_order {
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
                zero = true;
                break;
            }
        }
        if zero {
            0
        } else {
            max_q.max(0)
        }
    } else {
        0
    };

    let max_per_item = 12i32;

    let existing_item_opt = current_basket_items
        .iter()
        .find(|item| item.variant_id == variant_id)
        .cloned();

    let existing_qty = existing_item_opt.as_ref().map(|x| x.quantity).unwrap_or(0);
    let desired_total = (existing_qty + requested_quantity).clamp(0, max_per_item);

    let allowed_total = if is_back_order || is_pre_order {
        desired_total
    } else {
        desired_total.min(max_possible)
    };

    let txn = db.begin().await.map_db_err()?;

    let status;
    if allowed_total <= 0 {
        if let Some(existing_item) = existing_item_opt {
            basket_items::Entity::delete_by_id(existing_item.id.clone())
                .exec(&txn)
                .await
                .map_db_err()?;
        }
        status = "Removed".to_string();
    } else {
        if let Some(existing_item) = existing_item_opt {
            if existing_item.quantity != allowed_total {
                let mut am: basket_items::ActiveModel = existing_item.into();
                am.quantity = ActiveValue::Set(allowed_total);
                basket_items::Entity::update(am)
                    .exec(&txn)
                    .await
                    .map_db_err()?;
                status = if allowed_total < desired_total {
                    "Reduced".to_string()
                } else {
                    "Complete".to_string()
                };
            } else {
                status = "Complete".to_string();
            }
        } else {
            let new_id = Uuid::new_v4().to_string();
            let new_bi = basket_items::ActiveModel {
                id: ActiveValue::Set(new_id),
                basket_id: ActiveValue::Set(basket_id.clone()),
                variant_id: ActiveValue::Set(variant_id.clone()),
                quantity: ActiveValue::Set(allowed_total),
            };
            basket_items::Entity::insert(new_bi)
                .exec(&txn)
                .await
                .map_db_err()?;
            status = if allowed_total < desired_total {
                "Reduced".to_string()
            } else {
                "Complete".to_string()
            };
        }
    }

    let updated_basket_model = customer_baskets::Entity::find_by_id(&basket_id)
        .one(&txn)
        .await
        .map_db_err()?
        .expect("Basket should exist");

    if let Some(country_code) = &updated_basket_model.country_code {
        if !auto_apply_discounts.is_empty() {
            let updated_basket_items = basket_items::Entity::find()
                .filter(basket_items::Column::BasketId.eq(&basket_id))
                .all(&txn)
                .await
                .map_db_err()?;

            let variants: Vec<product_variants::Model> = products_with_variants
                .into_iter()
                .flat_map(|(_, variants)| variants)
                .collect();

            let mut best_discount: Option<discounts::Model> = None;
            let mut best_discount_value: f64 = 0.0;

            for discount in auto_apply_discounts {
                let check_result = check_discount(
                    discount.code.clone(),
                    Some(country_code.clone()),
                    Some(vec![discount.clone()]),
                    updated_basket_items.clone(),
                    variants.clone(),
                )
                .await;

                if check_result.is_ok() {
                    let discount_value =
                        calculate_discount_value(&discount, &updated_basket_items, &variants);

                    if discount_value > best_discount_value {
                        best_discount_value = discount_value;
                        best_discount = Some(discount);
                    }
                }
            }

            if let Some(best_discount) = best_discount {
                let mut basket_am: customer_baskets::ActiveModel = updated_basket_model.into();
                basket_am.discount_code = ActiveValue::Set(Some(best_discount.code));
                customer_baskets::Entity::update(basket_am)
                    .exec(&txn)
                    .await
                    .map_db_err()?;
            }
        }
    }

    txn.commit().await.map_db_err()?;

    let updated_basket = get_or_create_basket().await?;
    Ok(AddToBasketResponse {
        status,
        basket: updated_basket,
    })
}

#[cfg(feature = "server")]
fn calculate_discount_value(
    discount: &discounts::Model,
    basket_items: &[basket_items::Model],
    variants: &[product_variants::Model],
) -> f64 {
    use sea_orm_active_enums::DiscountType;

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
        DiscountType::PercentageOnShipping | DiscountType::FixedAmountOnShipping => 0.1,
    }
}

#[server]
pub async fn update_basket_country(country_code: String) -> Result<CustomerBasket, ServerFnError> {
    use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
    let db = get_db().await;
    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();

    if basket.locked {
        panic!("Basket locked")
    }

    let basket_entity = customer_baskets::Entity::find()
        .filter(customer_baskets::Column::Id.eq(&basket_id))
        .one(db)
        .await
        .map_db_err()?;

    if let Some(basket_model) = basket_entity {
        let mut basket_active: customer_baskets::ActiveModel = basket_model.into();
        basket_active.country_code = if country_code.is_empty() {
            ActiveValue::Set(None)
        } else {
            ActiveValue::Set(Some(country_code))
        };
        basket_active.shipping_option = ActiveValue::Set(None);

        customer_baskets::Entity::update(basket_active)
            .exec(db)
            .await
            .map_db_err()?;
    }
    get_or_create_basket().await
}

#[server]
pub async fn update_basket_shipping_option(
    shipping_option: ShippingOption,
) -> Result<CustomerBasket, ServerFnError> {
    use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

    let db = get_db().await;
    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();

    if basket.locked {
        panic!("Basket locked")
    }

    let basket_entity = customer_baskets::Entity::find()
        .filter(customer_baskets::Column::Id.eq(&basket_id))
        .one(db)
        .await
        .map_db_err()?;

    if let Some(basket_model) = basket_entity {
        let mut basket_active: customer_baskets::ActiveModel = basket_model.into();
        basket_active.shipping_option = ActiveValue::Set(Some(shipping_option.to_seaorm()));

        customer_baskets::Entity::update(basket_active)
            .exec(db)
            .await
            .map_db_err()?;
    }

    get_or_create_basket().await
}

#[server]
pub async fn update_basket_discount(
    discount_code: String,
) -> Result<BasketUpdateResult, ServerFnError> {
    use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
    let db = get_db().await;

    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();
    let discount_code = discount_code.to_uppercase();

    if basket.locked {
        panic!("Basket locked")
    }

    let mut discount_error = None;

    let (basket_entity, basket_items_result, variants_result) = tokio::join!(
        customer_baskets::Entity::find()
            .filter(customer_baskets::Column::Id.eq(&basket_id))
            .one(db),
        basket_items::Entity::find()
            .filter(basket_items::Column::BasketId.eq(&basket_id))
            .all(db),
        product_variants::Entity::find().all(db)
    );

    let basket_entity = basket_entity.map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    let basket_items =
        basket_items_result.map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    let variants = variants_result.map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if let Some(basket_model) = basket_entity {
        if !discount_code.is_empty() {
            match check_discount(
                discount_code.clone(),
                basket_model.country_code.clone(),
                None,
                basket_items,
                variants,
            )
            .await
            {
                Ok(_) => {}
                Err(validation_error) => {
                    discount_error = Some(validation_error);
                    let final_basket = get_or_create_basket().await?;
                    return Ok(BasketUpdateResult {
                        basket: final_basket,
                        discount_error,
                    });
                }
            }
        }

        let mut basket_active: customer_baskets::ActiveModel = basket_model.into();
        basket_active.discount_code = if discount_code.is_empty() {
            ActiveValue::Set(None)
        } else {
            ActiveValue::Set(Some(discount_code))
        };

        customer_baskets::Entity::update(basket_active)
            .exec(db)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    }

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
    use sea_orm::{ActiveModelTrait, ActiveValue, TransactionTrait};

    let db = get_db().await;
    let basket = get_or_create_basket().await?;
    let basket_id = basket.id.clone();

    if basket.locked {
        panic!("Basket locked")
    }

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

    let stock_map: std::collections::HashMap<String, StockQuantityResult> = stock_quantities
        .into_iter()
        .map(|sq| (sq.stock_item_id.clone(), sq))
        .collect();

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
        let basket = get_or_create_basket().await?;
        return Ok(AddToBasketResponse {
            status: "NotFound".to_string(),
            basket,
        });
    }

    let parent_product = parent_product.expect("Parent product should exist if variant exists");
    let is_back_order = parent_product.back_order;
    let is_pre_order = parent_product.pre_order;

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

    let max_possible = if is_back_order || is_pre_order {
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
                zero = true;
                break;
            }
        }
        if zero {
            0
        } else {
            max_q.max(0)
        }
    } else {
        0
    };

    let max_per_item = 12i32;
    let existing_item_opt = current_basket_items
        .iter()
        .find(|item| item.variant_id == variant_id)
        .cloned();

    let desired_total = target_quantity.clamp(0, max_per_item);
    let allowed_total = desired_total.min(max_possible);

    let txn = db.begin().await.map_db_err()?;

    let status;
    if allowed_total <= 0 {
        if let Some(existing_item) = existing_item_opt {
            basket_items::Entity::delete_by_id(existing_item.id.clone())
                .exec(&txn)
                .await
                .map_db_err()?;
        }
        status = "Removed".to_string();
    } else {
        if let Some(existing_item) = existing_item_opt {
            if existing_item.quantity != allowed_total {
                let mut am: basket_items::ActiveModel = existing_item.into();
                am.quantity = ActiveValue::Set(allowed_total);
                basket_items::Entity::update(am)
                    .exec(&txn)
                    .await
                    .map_db_err()?;
                status = if allowed_total < desired_total {
                    "Reduced".to_string()
                } else {
                    "Complete".to_string()
                };
            } else {
                status = "Complete".to_string();
            }
        } else {
            let new_id = Uuid::new_v4().to_string();
            let new_bi = basket_items::ActiveModel {
                id: ActiveValue::Set(new_id),
                basket_id: ActiveValue::Set(basket_id.clone()),
                variant_id: ActiveValue::Set(variant_id.clone()),
                quantity: ActiveValue::Set(allowed_total),
            };
            basket_items::Entity::insert(new_bi)
                .exec(&txn)
                .await
                .map_db_err()?;
            status = if allowed_total < desired_total {
                "Reduced".to_string()
            } else {
                "Complete".to_string()
            };
        }
    }

    let updated_basket_model = customer_baskets::Entity::find_by_id(&basket_id)
        .one(&txn)
        .await
        .map_db_err()?
        .expect("Basket should exist");

    if let Some(country_code) = &updated_basket_model.country_code {
        if !auto_apply_discounts.is_empty() {
            let updated_basket_items = basket_items::Entity::find()
                .filter(basket_items::Column::BasketId.eq(&basket_id))
                .all(&txn)
                .await
                .map_db_err()?;

            let variants: Vec<product_variants::Model> = products_with_variants
                .into_iter()
                .flat_map(|(_, variants)| variants)
                .collect();

            let mut best_discount: Option<discounts::Model> = None;
            let mut best_discount_value: f64 = 0.0;

            for discount in auto_apply_discounts {
                let check_result = check_discount(
                    discount.code.clone(),
                    Some(country_code.clone()),
                    Some(vec![discount.clone()]),
                    updated_basket_items.clone(),
                    variants.clone(),
                )
                .await;

                if check_result.is_ok() {
                    let discount_value =
                        calculate_discount_value(&discount, &updated_basket_items, &variants);

                    if discount_value > best_discount_value {
                        best_discount_value = discount_value;
                        best_discount = Some(discount);
                    }
                }
            }

            if let Some(best_discount) = best_discount {
                let mut basket_am: customer_baskets::ActiveModel = updated_basket_model.into();
                basket_am.discount_code = ActiveValue::Set(Some(best_discount.code));
                customer_baskets::Entity::update(basket_am)
                    .exec(&txn)
                    .await
                    .map_db_err()?;
            }
        }
    }

    txn.commit().await.map_db_err()?;

    let updated_basket = get_or_create_basket().await?;
    Ok(AddToBasketResponse {
        status,
        basket: updated_basket,
    })
}
