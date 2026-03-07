// src/backend/server_functions/inventory.rs

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use super::super::entity_conversions;

#[cfg(feature = "server")]
use entity::{
    stock_backorder_active_reduce, stock_items, stock_location_quantities, stock_locations,
    stock_preorder_active_reduce, stock_quantity_adjustments,
};

#[cfg(feature = "server")]
use sea_orm::{
    self, ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    Set, TransactionTrait,
};

#[cfg(feature = "server")]
use chrono::Utc;

#[cfg(feature = "server")]
use uuid::Uuid;

use super::super::front_entities::*;
use super::auth::{check_admin_permission, get_current_user};

#[cfg(feature = "server")]
use super::basket::DbErrExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStockItemRequest {
    pub name: String,
    pub pbi_sku: String,
    pub description: Option<String>,
    pub thumbnail_ref: Option<String>,
    pub assembly_minutes: Option<i32>,
    pub default_shipping_days: Option<i32>,
    pub default_cost: Option<f64>,
    pub warning_quantity: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStockItemResponse {
    pub success: bool,
    pub message: String,
    pub stock_item_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditStockItemRequest {
    pub id: String,
    pub name: String,
    pub pbi_sku: String,
    pub description: Option<String>,
    pub thumbnail_ref: Option<String>,
    pub assembly_minutes: Option<i32>,
    pub default_shipping_days: Option<i32>,
    pub default_cost: Option<f64>,
    pub warning_quantity: Option<i32>,
    pub flatten_pre_or_back_reduces: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditStockItemResponse {
    pub success: bool,
    pub message: String,
    pub stock_item_id: Option<String>,
}

#[server]
pub async fn admin_get_stock_items() -> Result<Vec<StockItem>, ServerFnError> {
    let db = get_db().await;

    let (stock_items_result, location_qty_result, locations_result) = tokio::join!(
        stock_items::Entity::find().all(db),
        stock_location_quantities::Entity::find().all(db),
        stock_locations::Entity::find().all(db)
    );

    let stock_items_models = stock_items_result.map_db_err()?;
    let location_qty_models = location_qty_result.map_db_err()?;
    let locations_models = locations_result.map_db_err()?;

    let location_names: HashMap<String, String> = locations_models
        .into_iter()
        .map(|loc| (loc.id, loc.name))
        .collect();

    let mut qty_by_item: HashMap<String, Vec<StockLocationQuantity>> = HashMap::new();
    for lq in location_qty_models {
        let name = location_names.get(&lq.stock_location_id).cloned();
        let mut slq = StockLocationQuantity::from(lq);
        slq.stock_location_name = name;
        qty_by_item
            .entry(slq.stock_item_id.clone())
            .or_insert_with(Vec::new)
            .push(slq);
    }

    let stock_items = stock_items_models
        .into_iter()
        .map(|model| {
            let loc_qtys = qty_by_item.remove(&model.id).unwrap_or_default();
            entity_conversions::convert_stock_item_with_locations(model, loc_qtys)
        })
        .collect();

    Ok(stock_items)
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

    let (back_order_result, pre_order_result) = tokio::join!(
        stock_backorder_active_reduce::Entity::find()
            .filter(stock_backorder_active_reduce::Column::StockItemId.eq(&stock_item_id))
            .all(db),
        stock_preorder_active_reduce::Entity::find()
            .filter(stock_preorder_active_reduce::Column::StockItemId.eq(&stock_item_id))
            .all(db)
    );

    let back_order_models = back_order_result.map_db_err()?;
    let pre_order_models = pre_order_result.map_db_err()?;

    let back_orders: Vec<BackOrPreOrderActiveReduce> = back_order_models
        .into_iter()
        .map(|model| BackOrPreOrderActiveReduce {
            id: model.id,
            order_id: model.order_id,
            order_item_id: model.order_item_id,
            stock_item_id: model.stock_item_id,
            stock_location_id: model.stock_location_id,
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
            stock_location_id: model.stock_location_id,
            reduction_quantity: model.reduction_quantity,
            active: model.active,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
        .collect();

    Ok((back_orders, pre_orders))
}

#[server]
pub async fn admin_create_stock_item(
    request: CreateStockItemRequest,
) -> Result<CreateStockItemResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(CreateStockItemResponse {
            success: false,
            message: "Unauthorized".to_string(),
            stock_item_id: None,
        });
    }

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
            message: "NDI SKU is required".to_string(),
            stock_item_id: None,
        });
    }

    if !request.pbi_sku.starts_with("NDI") && !request.pbi_sku.starts_with("NDX") {
        return Ok(CreateStockItemResponse {
            success: false,
            message: "SKU must start with 'NDI' or 'NDX'".to_string(),
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

    let stock_item_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    let stock_item = stock_items::ActiveModel {
        id: ActiveValue::Set(stock_item_id.clone()),
        pbi_sku: ActiveValue::Set(request.pbi_sku),
        name: ActiveValue::Set(request.name),
        description: ActiveValue::Set(request.description),
        thumbnail_ref: ActiveValue::Set(request.thumbnail_ref),
        assembly_minutes: ActiveValue::Set(request.assembly_minutes),
        default_shipping_days: ActiveValue::Set(request.default_shipping_days),
        default_cost: ActiveValue::Set(request.default_cost),
        warning_quantity: ActiveValue::Set(request.warning_quantity),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

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

#[server]
pub async fn admin_edit_stock_item(
    request: EditStockItemRequest,
) -> Result<EditStockItemResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(EditStockItemResponse {
            success: false,
            message: "Unauthorized".to_string(),
            stock_item_id: None,
        });
    }

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
            message: "NDI SKU is required".to_string(),
            stock_item_id: None,
        });
    }

    if !request.pbi_sku.starts_with("NDI") && !request.pbi_sku.starts_with("NDX") {
        return Ok(EditStockItemResponse {
            success: false,
            message: "SKU must start with 'NDI' or 'NDX'".to_string(),
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

    let db = get_db().await;

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

    let now = Utc::now().naive_utc();

    let stock_item_update = stock_items::ActiveModel {
        id: ActiveValue::Set(request.id.clone()),
        name: ActiveValue::Set(request.name),
        pbi_sku: ActiveValue::Set(request.pbi_sku),
        description: ActiveValue::Set(request.description),
        thumbnail_ref: ActiveValue::Set(request.thumbnail_ref),
        assembly_minutes: ActiveValue::Set(request.assembly_minutes),
        default_shipping_days: ActiveValue::Set(request.default_shipping_days),
        default_cost: ActiveValue::Set(request.default_cost),
        warning_quantity: ActiveValue::Set(request.warning_quantity),
        updated_at: ActiveValue::Set(now),
        ..Default::default()
    };

    stock_items::Entity::update(stock_item_update)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update stock item: {}", e)))?;

    if request.flatten_pre_or_back_reduces {
        match super::super::payments::flatten_preorder_backorder_reduces(request.id.clone()).await {
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

// ─── Stock Location server functions ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStockLocationRequest {
    pub name: String,
    pub description: Option<String>,
    pub shipping_method: StockLocationShippingMethod,
    pub flat_rate_usd: Option<f64>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditStockLocationRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub shipping_method: StockLocationShippingMethod,
    pub flat_rate_usd: Option<f64>,
    pub country: Option<String>,
}

#[server]
pub async fn admin_get_stock_locations() -> Result<Vec<StockLocation>, ServerFnError> {
    let db = get_db().await;
    let models = stock_locations::Entity::find()
        .all(db)
        .await
        .map_db_err()?;
    Ok(models.into_iter().map(StockLocation::from).collect())
}

#[server]
pub async fn admin_create_stock_location(
    request: CreateStockLocationRequest,
) -> Result<(), ServerFnError> {
    if request.name.trim().is_empty() {
        return Err(ServerFnError::new("Name is required"));
    }
    let db = get_db().await;
    let now = Utc::now().naive_utc();
    let model = stock_locations::ActiveModel {
        id: ActiveValue::Set(uuid::Uuid::new_v4().to_string()),
        name: ActiveValue::Set(request.name),
        description: ActiveValue::Set(request.description),
        shipping_method: ActiveValue::Set(request.shipping_method.to_seaorm()),
        flat_rate_usd: ActiveValue::Set(request.flat_rate_usd),
        country: ActiveValue::Set(request.country),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };
    stock_locations::Entity::insert(model)
        .exec(db)
        .await
        .map_db_err()?;
    Ok(())
}

#[server]
pub async fn admin_edit_stock_location(
    request: EditStockLocationRequest,
) -> Result<(), ServerFnError> {
    if request.name.trim().is_empty() {
        return Err(ServerFnError::new("Name is required"));
    }
    let db = get_db().await;
    let existing = stock_locations::Entity::find_by_id(&request.id)
        .one(db)
        .await
        .map_db_err()?
        .ok_or_else(|| ServerFnError::new("Stock location not found"))?;
    let now = Utc::now().naive_utc();
    let mut active = existing.into_active_model();
    active.name = Set(request.name);
    active.description = Set(request.description);
    active.shipping_method = Set(request.shipping_method.to_seaorm());
    active.flat_rate_usd = Set(request.flat_rate_usd);
    active.country = Set(request.country);
    active.updated_at = Set(now);
    active.update(db).await.map_db_err()?;
    Ok(())
}

#[server]
pub async fn admin_delete_stock_location(id: String) -> Result<(), ServerFnError> {
    let db = get_db().await;
    stock_locations::Entity::delete_by_id(&id)
        .exec(db)
        .await
        .map_db_err()?;
    Ok(())
}

/// Adjust stock quantity for a stock item at a specific location.
/// delta is positive for additions, negative for subtractions.
/// Creates the location-quantity record if it does not exist yet.
/// Returns the new quantity.
#[server]
pub async fn admin_adjust_stock_quantity(
    stock_item_id: String,
    stock_location_id: String,
    delta: i32,
    note: String,
) -> Result<i32, ServerFnError> {
    if note.trim().is_empty() {
        return Err(ServerFnError::new("A note is required for stock adjustments"));
    }
    let db = get_db().await;
    let txn = db.begin().await.map_db_err()?;
    let now = Utc::now().naive_utc();

    let existing = stock_location_quantities::Entity::find()
        .filter(stock_location_quantities::Column::StockItemId.eq(&stock_item_id))
        .filter(stock_location_quantities::Column::StockLocationId.eq(&stock_location_id))
        .one(&txn)
        .await
        .map_db_err()?;

    let (lq_id, new_qty) = if let Some(model) = existing {
        let new_qty = (model.quantity + delta).max(0);
        let mut active = model.into_active_model();
        active.quantity = Set(new_qty);
        active.updated_at = Set(now);
        let updated = active.update(&txn).await.map_db_err()?;
        (updated.id, new_qty)
    } else {
        let new_qty = delta.max(0);
        let id = uuid::Uuid::new_v4().to_string();
        let model = stock_location_quantities::ActiveModel {
            id: ActiveValue::Set(id.clone()),
            stock_item_id: ActiveValue::Set(stock_item_id),
            stock_location_id: ActiveValue::Set(stock_location_id),
            quantity: ActiveValue::Set(new_qty),
            enabled: ActiveValue::Set(true),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        };
        stock_location_quantities::Entity::insert(model)
            .exec(&txn)
            .await
            .map_db_err()?;
        (id, new_qty)
    };

    let adj = stock_quantity_adjustments::ActiveModel {
        id: ActiveValue::Set(uuid::Uuid::new_v4().to_string()),
        stock_location_quantity_id: ActiveValue::Set(lq_id),
        delta: ActiveValue::Set(delta),
        note: ActiveValue::Set(note),
        adjusted_by: ActiveValue::Set(None),
        created_at: ActiveValue::Set(now),
    };
    stock_quantity_adjustments::Entity::insert(adj)
        .exec(&txn)
        .await
        .map_db_err()?;

    txn.commit().await.map_db_err()?;
    Ok(new_qty)
}

/// Enable or disable a stock location for a specific stock item.
/// If no quantity record exists and enabling, creates one with quantity=0.
/// If no record exists and disabling, does nothing.
#[server]
pub async fn admin_toggle_stock_location(
    stock_item_id: String,
    stock_location_id: String,
    enabled: bool,
) -> Result<(), ServerFnError> {
    let db = get_db().await;
    let now = Utc::now().naive_utc();

    let existing = stock_location_quantities::Entity::find()
        .filter(stock_location_quantities::Column::StockItemId.eq(&stock_item_id))
        .filter(stock_location_quantities::Column::StockLocationId.eq(&stock_location_id))
        .one(db)
        .await
        .map_db_err()?;

    if let Some(model) = existing {
        let mut active = model.into_active_model();
        active.enabled = Set(enabled);
        active.updated_at = Set(now);
        active.update(db).await.map_db_err()?;
    } else if enabled {
        // Only create a record when enabling
        let model = stock_location_quantities::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            stock_item_id: ActiveValue::Set(stock_item_id),
            stock_location_id: ActiveValue::Set(stock_location_id),
            quantity: ActiveValue::Set(0),
            enabled: ActiveValue::Set(true),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        };
        stock_location_quantities::Entity::insert(model)
            .exec(db)
            .await
            .map_db_err()?;
    }

    Ok(())
}

/// Fetch the adjustment audit log for a specific stock-location-quantity record.
#[server]
pub async fn admin_get_stock_adjustment_history(
    slq_id: String,
) -> Result<Vec<StockQuantityAdjustment>, ServerFnError> {
    use sea_orm::QueryOrder;
    let db = get_db().await;
    let records = stock_quantity_adjustments::Entity::find()
        .filter(stock_quantity_adjustments::Column::StockLocationQuantityId.eq(&slq_id))
        .order_by_desc(stock_quantity_adjustments::Column::CreatedAt)
        .all(db)
        .await
        .map_db_err()?;
    Ok(records.into_iter().map(StockQuantityAdjustment::from).collect())
}
