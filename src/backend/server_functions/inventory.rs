// src/backend/server_functions/inventory.rs

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::NaiveDateTime;

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use super::super::entity_conversions;

#[cfg(feature = "server")]
use entity::{
    stock_backorder_active_reduce, stock_batches, stock_item_relations, stock_items,
    stock_preorder_active_reduce,
};

#[cfg(feature = "server")]
use sea_orm::{
    self, ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait,
};

#[cfg(feature = "server")]
use chrono::Utc;

#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
use sea_orm::sea_query::OnConflict;

use super::super::front_entities::*;
use super::auth::{check_admin_permission, get_current_user};

#[cfg(feature = "server")]
use super::stock_calculations::get_stock_quantities_for_stock_items;

#[cfg(feature = "server")]
use super::basket::DbErrExt;

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
    pub unit: StockUnit,
    pub assembly_minutes: Option<i32>,
    pub default_shipping_days: Option<i32>,
    pub default_cost: Option<f64>,
    pub warning_quantity: Option<f64>,
    pub is_container: bool,
    pub flatten_pre_or_back_reduces: bool,
    pub batches: Option<Vec<EditStockBatchRequest>>,
    pub stock_item_relations: Option<Vec<StockItemRelation>>,
}

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
pub async fn admin_get_stock_items() -> Result<Vec<StockItem>, ServerFnError> {
    let db = get_db().await;

    let stock_items_models: Vec<stock_items::Model> =
        stock_items::Entity::find().all(db).await.map_db_err()?;

    let stock_quantities = get_stock_quantities_for_stock_items(Some(
        stock_items_models
            .iter()
            .map(|model| model.id.clone())
            .collect(),
    ))
    .await?;

    let stock_items = entity_conversions::convert_stock_items_batch_with_quantities(
        stock_items_models,
        &stock_quantities,
    );

    Ok(stock_items)
}

#[server]
pub async fn admin_get_stock_batches() -> Result<Vec<StockBatch>, ServerFnError> {
    let db = get_db().await;

    let stock_batches_models: Vec<stock_batches::Model> =
        stock_batches::Entity::find().all(db).await.map_db_err()?;

    let stock_batches = entity_conversions::convert_stock_batches_batch(stock_batches_models);

    Ok(stock_batches)
}

#[server]
pub async fn admin_get_stock_item_relations() -> Result<Vec<StockItemRelation>, ServerFnError> {
    let db = get_db().await;

    let stock_item_relations_models: Vec<stock_item_relations::Model> =
        stock_item_relations::Entity::find()
            .all(db)
            .await
            .map_db_err()?;

    let stock_item_relations =
        entity_conversions::convert_stock_item_relations_batch(stock_item_relations_models);

    Ok(stock_item_relations)
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
            message: "PBI SKU is required".to_string(),
            stock_item_id: None,
        });
    }

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
        unit: ActiveValue::Set(request.unit.to_seaorm()),
        assembly_minutes: ActiveValue::Set(request.assembly_minutes),
        default_shipping_days: ActiveValue::Set(request.default_shipping_days),
        default_cost: ActiveValue::Set(request.default_cost),
        warning_quantity: ActiveValue::Set(request.warning_quantity),
        is_container: ActiveValue::Set(request.is_container),
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
            message: "PBI SKU is required".to_string(),
            stock_item_id: None,
        });
    }

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

    let txn = db
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to start transaction: {}", e)))?;

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

    if let Some(batches) = request.batches {
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

        for batch_request in batches {
            if let Some(batch_id) = &batch_request.id {
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

        let batches_to_delete: Vec<String> = existing_batch_ids
            .difference(&request_batch_ids)
            .cloned()
            .collect();

        if !batches_to_delete.is_empty() {
            stock_batches::Entity::delete_many()
                .filter(stock_batches::Column::Id.is_in(batches_to_delete))
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to delete batches: {}", e)))?;
        }

        for batch_update in batches_to_update {
            stock_batches::Entity::update(batch_update)
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to update batch: {}", e)))?;
        }

        if !new_batches.is_empty() {
            stock_batches::Entity::insert_many(new_batches)
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to create new batches: {}", e)))?;
        }
    }

    if let Some(relations) = request.stock_item_relations {
        let existing_relations = stock_item_relations::Entity::find()
            .filter(stock_item_relations::Column::ParentStockItemId.eq(&request.id))
            .all(&txn)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to fetch existing relations: {}", e))
            })?;

        let existing_keys: HashSet<(String, String)> = existing_relations
            .iter()
            .map(|r| {
                (
                    r.parent_stock_item_id.clone(),
                    r.child_stock_item_id.clone(),
                )
            })
            .collect();

        let mut desired_keys: HashSet<(String, String)> = HashSet::new();

        for relation in relations {
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

            let am = stock_item_relations::ActiveModel {
                parent_stock_item_id: ActiveValue::Set(relation.parent_stock_item_id),
                child_stock_item_id: ActiveValue::Set(relation.child_stock_item_id),
                quantity: ActiveValue::Set(relation.quantity),
                created_at: ActiveValue::Set(now),
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
        stock_item_relations::Entity::delete_many()
            .filter(stock_item_relations::Column::ParentStockItemId.eq(&request.id))
            .exec(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete all relations: {}", e)))?;
    }

    txn.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to commit transaction: {}", e)))?;

    if request.flatten_pre_or_back_reduces {
        match super::super::payments::flatten_preorder_backorder_reduces(request.id.clone()).await
        {
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
