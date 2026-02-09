// src/backend/server_functions/stock_calculations.rs

use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use entity::{
    product_variant_stock_item_relations, stock_active_reduce, stock_backorder_active_reduce,
    stock_batches, stock_item_relations, stock_items, stock_preorder_active_reduce,
};

#[cfg(feature = "server")]
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[cfg(feature = "server")]
use futures;

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

#[cfg(feature = "server")]
pub async fn get_stock_quantities_for_stock_items(
    stock_item_ids: Option<Vec<String>>,
) -> Result<Vec<StockQuantityResult>, ServerFnError> {
    let db = get_db().await;

    let (target_stock_item_ids, all_needed_stock_item_ids) = match &stock_item_ids {
        Some(ids) => {
            let all_relations = stock_item_relations::Entity::find()
                .all(db)
                .await
                .map_err(|e| {
                    StockCalculationError::new(format!("Failed to fetch relations: {}", e))
                })?;

            let mut relations_by_parent: HashMap<String, Vec<String>> = HashMap::new();
            for relation in &all_relations {
                relations_by_parent
                    .entry(relation.parent_stock_item_id.clone())
                    .or_insert_with(Vec::new)
                    .push(relation.child_stock_item_id.clone());
            }

            let mut all_needed_stock_item_ids = HashSet::new();
            let mut queue = ids.clone();

            while !queue.is_empty() {
                let mut next_queue = Vec::new();

                for stock_item_id in queue {
                    if !all_needed_stock_item_ids.contains(&stock_item_id) {
                        all_needed_stock_item_ids.insert(stock_item_id.clone());

                        if let Some(children) = relations_by_parent.get(&stock_item_id) {
                            next_queue.extend(children.iter().cloned());
                        }
                    }
                }

                queue = next_queue;
            }

            (ids.clone(), Some(all_needed_stock_item_ids))
        }
        None => (vec![], None),
    };

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

    let reduces_future = stock_active_reduce::Entity::find().all(db);
    let backorder_reduces_future = stock_backorder_active_reduce::Entity::find().all(db);
    let preorder_reduces_future = stock_preorder_active_reduce::Entity::find().all(db);

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

    let final_target_ids = if target_stock_item_ids.is_empty() {
        all_stock_items.iter().map(|item| item.id.clone()).collect()
    } else {
        target_stock_item_ids
    };

    let stock_items = super::super::entity_conversions::convert_stock_items_batch(all_stock_items);
    let stock_batches =
        super::super::entity_conversions::convert_stock_batches_batch(all_batches);
    let stock_relations: Vec<StockItemRelation> = all_relations
        .into_iter()
        .map(StockItemRelation::from)
        .collect();

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

    let reduces_by_batch: HashMap<String, Vec<&stock_active_reduce::Model>> =
        all_reduces.iter().fold(HashMap::new(), |mut acc, reduce| {
            acc.entry(reduce.stock_batch_id.clone())
                .or_insert_with(Vec::new)
                .push(reduce);
            acc
        });

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

    let preorder_reduces_by_stock_item: HashMap<
        String,
        Vec<&stock_preorder_active_reduce::Model>,
    > = preorder_reduces
        .iter()
        .fold(HashMap::new(), |mut acc, reduce| {
            acc.entry(reduce.stock_item_id.clone())
                .or_insert_with(Vec::new)
                .push(reduce);
            acc
        });

    let relations_by_parent: HashMap<String, Vec<&StockItemRelation>> =
        stock_relations
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

    let child_stock_allocations = calculate_child_stock_allocations(
        &stock_items_map,
        &batches_by_stock_item,
        &relations_by_parent,
        &relations_by_child,
        &reduces_by_batch,
        &backorder_reduces_by_stock_item,
        &preorder_reduces_by_stock_item,
    )?;

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

    let ready_stock_quantity = calculate_ready_stock(
        stock_item,
        batches_by_stock_item,
        reduces_by_batch,
        backorder_reduces_by_stock_item,
        preorder_reduces_by_stock_item,
    );

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

    let total_stock_quantity = ready_stock_quantity
        .add(&unready_stock_quantity)
        .map_err(|e| StockCalculationError::new(format!("Cannot add stock quantities: {}", e)))?;

    let stock_too_low = if let Some(warning_qty) = stock_item.warning_quantity {
        total_stock_quantity.to_f64() < warning_qty
    } else {
        false
    };

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
        let complete_batches: Vec<&StockBatch> = batches
            .iter()
            .filter(|batch| {
                batch.status == StockBatchStatus::Complete
                    && batch.stock_unit_on_creation == stock_item.unit
            })
            .copied()
            .collect();

        if !complete_batches.is_empty() {
            let mut batches_with_reductions: Vec<StockBatch> = complete_batches
                .into_iter()
                .map(|batch| {
                    let mut batch_clone = batch.clone();
                    if let Some(reduces) = reduces_by_batch.get(&batch.id) {
                        let total_reduction: f64 = reduces
                            .iter()
                            .filter(|reduce| {
                                StockUnit::from_seaorm(reduce.stock_unit.clone())
                                    == batch.stock_unit_on_creation
                            })
                            .map(|reduce| reduce.reduction_quantity)
                            .sum();

                        let current_live_quantity = batch.live_quantity.to_f64();
                        let new_live_quantity = (current_live_quantity - total_reduction).max(0.0);

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

    let mut total_stock_item_reduction = 0.0;

    if let Some(backorder_reduces) = backorder_reduces_by_stock_item.get(&stock_item.id) {
        let backorder_reduction: f64 = backorder_reduces
            .iter()
            .filter(|reduce| StockUnit::from_seaorm(reduce.stock_unit.clone()) == stock_item.unit)
            .map(|reduce| reduce.reduction_quantity)
            .sum();
        total_stock_item_reduction += backorder_reduction;
    }

    if let Some(preorder_reduces) = preorder_reduces_by_stock_item.get(&stock_item.id) {
        let preorder_reduction: f64 = preorder_reduces
            .iter()
            .filter(|reduce| StockUnit::from_seaorm(reduce.stock_unit.clone()) == stock_item.unit)
            .map(|reduce| reduce.reduction_quantity)
            .sum();
        total_stock_item_reduction += preorder_reduction;
    }

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

    if let Some(child_relations) = relations_by_parent.get(stock_item_id) {
        for relation in child_relations {
            total_child_count += 1;

            if let Some(child_stock_item) = stock_items_map.get(&relation.child_stock_item_id) {
                let allocated_child_stock = child_stock_allocations
                    .get(stock_item_id)
                    .and_then(|allocations| allocations.get(&relation.child_stock_item_id))
                    .copied()
                    .unwrap_or(0.0);

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

                let total_child_stock = allocated_child_stock + child_unready_stock.to_f64();

                let parent_batches_possible = if relation.quantity > 0.0 && total_child_stock > 0.0
                {
                    let available_units = total_child_stock / relation.quantity;

                    match stock_item.unit {
                        StockUnit::Multiples => available_units.floor(),
                        StockUnit::Grams => available_units,
                        StockUnit::Milliliters => available_units,
                    }
                } else {
                    0.0
                };

                match min_unready_quantity {
                    None => {
                        min_unready_quantity = Some(parent_batches_possible);
                    }
                    Some(current_min) => {
                        if parent_batches_possible < current_min {
                            min_unready_quantity = Some(parent_batches_possible);
                        }
                    }
                }
            }
        }
    }

    visited.remove(stock_item_id);

    let unready_value = min_unready_quantity.unwrap_or(0.0);

    let unready_quantity = match stock_item.unit {
        StockUnit::Multiples => StockUnitQuantity::Multiples(unready_value as i32),
        StockUnit::Grams => StockUnitQuantity::Grams(unready_value),
        StockUnit::Milliliters => StockUnitQuantity::Milliliters(unready_value),
    };

    Ok((unready_quantity, total_child_count))
}

#[cfg(feature = "server")]
fn calculate_required_father_replacement(
    stock_item_id: &str,
    stock_item: &StockItem,
    relations_by_child: &HashMap<String, Vec<&StockItemRelation>>,
    stock_items_map: &HashMap<String, &StockItem>,
) -> Result<Option<StockUnitQuantity>, StockCalculationError> {
    if let Some(parent_relations) = relations_by_child.get(stock_item_id) {
        let mut total_replacement_needed = 0.0;

        for relation in parent_relations {
            if let Some(parent_stock_item) = stock_items_map.get(&relation.parent_stock_item_id) {
                if let Some(warning_qty) = parent_stock_item.warning_quantity {
                    let child_requirement = warning_qty * relation.quantity;
                    total_replacement_needed += child_requirement;
                }
            }
        }

        if total_replacement_needed > 0.0 {
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

    for (child_id, parent_relations) in relations_by_child {
        if parent_relations.is_empty() {
            continue;
        }

        let available_stock = child_total_stock.get(child_id).copied().unwrap_or(0.0);
        if available_stock <= 0.0 {
            continue;
        }

        if parent_relations.len() == 1 {
            let parent_id = &parent_relations[0].parent_stock_item_id;
            allocations
                .entry(parent_id.clone())
                .or_insert_with(HashMap::new)
                .insert(child_id.clone(), available_stock);
            continue;
        }

        let mut parent_info: Vec<(String, f64, chrono::NaiveDateTime)> = Vec::new();

        for relation in parent_relations {
            if let Some(parent_item) = stock_items_map.get(&relation.parent_stock_item_id) {
                parent_info.push((
                    relation.parent_stock_item_id.clone(),
                    relation.quantity,
                    parent_item.created_at,
                ));
            }
        }

        if parent_info.is_empty() {
            continue;
        }

        parent_info.sort_by(|a, b| a.2.cmp(&b.2));

        let total_demand_for_one_unit: f64 = parent_info.iter().map(|(_, qty, _)| qty).sum();
        let max_units_all_parents = if total_demand_for_one_unit > 0.0 {
            available_stock / total_demand_for_one_unit
        } else {
            0.0
        };

        if max_units_all_parents >= 1.0 {
            let units_to_make = max_units_all_parents.floor();
            for (parent_id, qty_needed, _) in parent_info {
                let allocation = units_to_make * qty_needed;
                if allocation > 0.0 {
                    allocations
                        .entry(parent_id.clone())
                        .or_insert_with(HashMap::new)
                        .insert(child_id.clone(), allocation);
                }
            }
        } else {
            let mut remaining_stock = available_stock;

            for (parent_id, qty_needed, _created_at) in parent_info {
                if remaining_stock <= 0.0 {
                    break;
                }

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
                }
            }
        }
    }

    Ok(allocations)
}

pub fn calculate_variant_stock_quantities(
    mut products: Vec<Product>,
    relations: Vec<product_variant_stock_item_relations::Model>,
    stock_results: Vec<StockQuantityResult>,
) -> Vec<Product> {
    let stock_lookup: HashMap<String, &StockQuantityResult> = stock_results
        .iter()
        .map(|result| (result.stock_item_id.clone(), result))
        .collect();

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

pub fn calculate_variant_available_stock(
    variant_id: &str,
    variant_relations: &HashMap<String, Vec<&product_variant_stock_item_relations::Model>>,
    stock_lookup: &HashMap<String, &StockQuantityResult>,
) -> i32 {
    let relations = match variant_relations.get(variant_id) {
        Some(relations) => relations,
        None => return 0,
    };

    if relations.is_empty() {
        return 0;
    }

    let mut min_available_stock = i32::MAX;

    for relation in relations {
        let stock_result = match stock_lookup.get(&relation.stock_item_id) {
            Some(result) => result,
            None => {
                return 0;
            }
        };

        let available_units = calculate_available_units_from_stock(
            &stock_result.total_stock_quantity,
            relation.quantity,
            &StockUnit::from_seaorm(relation.stock_unit_on_creation.clone()),
        );

        if available_units == 0 {
            return 0;
        }

        min_available_stock = min_available_stock.min(available_units);
    }

    if min_available_stock == i32::MAX {
        0
    } else {
        min_available_stock
    }
}

fn calculate_available_units_from_stock(
    total_stock: &StockUnitQuantity,
    required_quantity: f64,
    _stock_unit: &StockUnit,
) -> i32 {
    let available_stock = total_stock.to_f64();

    if available_stock <= 0.0 {
        return 0;
    }

    if required_quantity <= 0.0 {
        return 0;
    }

    let units_possible = available_stock / required_quantity;

    units_possible.floor() as i32
}
