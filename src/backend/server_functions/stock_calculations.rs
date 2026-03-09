// src/backend/server_functions/stock_calculations.rs

use dioxus::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use entity::{
    product_variant_stock_item_relations, stock_backorder_active_reduce,
    stock_location_quantities, stock_preorder_active_reduce,
};

#[cfg(feature = "server")]
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::super::front_entities::*;

#[cfg(feature = "server")]
use super::basket::DbErrExt;

#[derive(Debug, Clone)]
pub struct StockCalculationError {
    pub message: String,
}

impl fmt::Display for StockCalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for StockCalculationError {}

impl StockCalculationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[cfg(feature = "server")]
impl From<StockCalculationError> for ServerFnError {
    fn from(err: StockCalculationError) -> Self {
        ServerFnError::new(err.to_string())
    }
}

/// Returns a map of stock_item_id -> total available quantity across all locations,
/// minus any active backorder/preorder reduces.
#[cfg(feature = "server")]
pub async fn get_available_stock_by_item(
    stock_item_ids: Option<Vec<String>>,
) -> Result<HashMap<String, i32>, ServerFnError> {
    let db = get_db().await;

    // Fetch location quantities
    let location_quantities = match &stock_item_ids {
        Some(ids) => stock_location_quantities::Entity::find()
            .filter(stock_location_quantities::Column::StockItemId.is_in(ids.iter().cloned()))
            .all(db)
            .await
            .map_db_err()?,
        None => stock_location_quantities::Entity::find()
            .all(db)
            .await
            .map_db_err()?,
    };

    // Sum quantities per stock item
    let mut stock_totals: HashMap<String, i32> = HashMap::new();
    for lq in &location_quantities {
        *stock_totals.entry(lq.stock_item_id.clone()).or_insert(0) += lq.quantity;
    }

    // Fetch active backorder reduces and subtract
    let backorder_reduces = match &stock_item_ids {
        Some(ids) => stock_backorder_active_reduce::Entity::find()
            .filter(stock_backorder_active_reduce::Column::StockItemId.is_in(ids.iter().cloned()))
            .filter(stock_backorder_active_reduce::Column::Active.eq(true))
            .all(db)
            .await
            .map_db_err()?,
        None => stock_backorder_active_reduce::Entity::find()
            .filter(stock_backorder_active_reduce::Column::Active.eq(true))
            .all(db)
            .await
            .map_db_err()?,
    };

    for reduce in &backorder_reduces {
        let entry = stock_totals.entry(reduce.stock_item_id.clone()).or_insert(0);
        *entry = (*entry - reduce.reduction_quantity).max(0);
    }

    // Fetch active preorder reduces and subtract
    let preorder_reduces = match &stock_item_ids {
        Some(ids) => stock_preorder_active_reduce::Entity::find()
            .filter(stock_preorder_active_reduce::Column::StockItemId.is_in(ids.iter().cloned()))
            .filter(stock_preorder_active_reduce::Column::Active.eq(true))
            .all(db)
            .await
            .map_db_err()?,
        None => stock_preorder_active_reduce::Entity::find()
            .filter(stock_preorder_active_reduce::Column::Active.eq(true))
            .all(db)
            .await
            .map_db_err()?,
    };

    for reduce in &preorder_reduces {
        let entry = stock_totals.entry(reduce.stock_item_id.clone()).or_insert(0);
        *entry = (*entry - reduce.reduction_quantity).max(0);
    }

    Ok(stock_totals)
}

/// Calculate per-variant stock quantities and update products in place.
/// For each variant, finds linked stock items, gets available stock,
/// divides by the required quantity per unit, returns the minimum.
#[cfg(feature = "server")]
pub fn calculate_variant_stock_quantities(
    mut products: Vec<Product>,
    relations: Vec<product_variant_stock_item_relations::Model>,
    stock_totals: &HashMap<String, i32>,
) -> Vec<Product> {
    let mut variant_relations: HashMap<String, Vec<&product_variant_stock_item_relations::Model>> =
        HashMap::new();
    for relation in &relations {
        variant_relations
            .entry(relation.product_variant_id.clone())
            .or_insert_with(Vec::new)
            .push(relation);
    }

    for product in &mut products {
        if let Some(ref mut variants) = product.variants {
            for variant in variants.iter_mut() {
                let has_relations = variant_relations
                    .get(&variant.id)
                    .map(|r| !r.is_empty())
                    .unwrap_or(false);
                variant.has_stock_relations = has_relations;
                variant.calculated_stock_quantity = Some(calculate_variant_available_stock(
                    &variant.id,
                    &variant_relations,
                    stock_totals,
                ));
            }
        }
    }

    products
}

/// Calculate available stock units for a single variant.
#[cfg(feature = "server")]
pub fn calculate_variant_available_stock(
    variant_id: &str,
    variant_relations: &HashMap<String, Vec<&product_variant_stock_item_relations::Model>>,
    stock_totals: &HashMap<String, i32>,
) -> i32 {
    let relations = match variant_relations.get(variant_id) {
        Some(r) if !r.is_empty() => r,
        _ => return 0,
    };

    let mut min_available = i32::MAX;

    for relation in relations {
        let available_stock = stock_totals
            .get(&relation.stock_item_id)
            .copied()
            .unwrap_or(0);

        if relation.quantity <= 0 {
            return 0;
        }

        let units_possible = available_stock / relation.quantity;

        if units_possible == 0 {
            return 0;
        }

        min_available = min_available.min(units_possible);
    }

    if min_available == i32::MAX {
        0
    } else {
        min_available
    }
}
