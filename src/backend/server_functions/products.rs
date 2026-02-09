// src/backend/server_functions/products.rs

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use super::super::entity_conversions;

#[cfg(feature = "server")]
use entity::{
    product_variant_stock_item_relations, product_variants, products, sea_orm_active_enums,
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
use super::stock_calculations::{
    calculate_variant_stock_quantities, get_stock_quantities_for_stock_items,
};

#[cfg(feature = "server")]
use super::basket::DbErrExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEditProductRequest {
    pub id: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub handle: String,
    pub collections: Vec<Category>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub alternate_names: Vec<String>,
    pub product_form: ProductForm,
    pub visibility: ProductVisibility,
    pub force_no_stock: bool,
    pub purity_standard: Option<f64>,
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
    pub variants: Vec<CreateEditProductVariantRequest>,
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
pub async fn get_policies() -> Result<(String, String), ServerFnError> {
    let tos_content = include_str!("../../data/md/tos.md");
    let tos_html = entity_conversions::markdown_to_html(&tos_content);

    let privacy_content = include_str!("../../data/md/privacy.md");
    let privacy_html = entity_conversions::markdown_to_html(&privacy_content);

    Ok((tos_html, privacy_html))
}

#[server]
pub async fn get_products() -> Result<Vec<Product>, ServerFnError> {
    let db = get_db().await;

    let (products_with_variants_result, variant_relations_result, stock_qty_results_result) =
        tokio::join!(
            products::Entity::find()
                .filter(
                    products::Column::Visibility
                        .eq(sea_orm_active_enums::ProductVisibility::Public)
                )
                .find_with_related(product_variants::Entity)
                .all(db),
            async {
                product_variant_stock_item_relations::Entity::find()
                    .all(db)
                    .await
            },
            get_stock_quantities_for_stock_items(None)
        );

    let products_with_variants = products_with_variants_result.map_db_err()?;
    let variant_relations = variant_relations_result.map_db_err()?;
    let stock_qty_results = stock_qty_results_result?;

    let (product_models, contexts): (Vec<_>, Vec<_>) = products_with_variants
        .into_iter()
        .map(|(product_model, variant_models)| {
            let converted_variants = if !variant_models.is_empty() {
                Some(entity_conversions::convert_product_variants(
                    variant_models,
                ))
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

    let mut products =
        entity_conversions::convert_products_batch_with_context(product_models, contexts)?;

    products = calculate_variant_stock_quantities(products, variant_relations, stock_qty_results);

    return Ok(products);
}

#[server]
pub async fn admin_get_products(convert_markdown: bool) -> Result<Vec<Product>, ServerFnError> {
    let db = get_db().await;

    let products_with_variants: Vec<(products::Model, Vec<product_variants::Model>)> =
        products::Entity::find()
            .find_with_related(product_variants::Entity)
            .all(db)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    let (product_models, contexts): (Vec<_>, Vec<_>) = products_with_variants
        .into_iter()
        .map(|(product_model, variant_models)| {
            let converted_variants = if !variant_models.is_empty() {
                Some(entity_conversions::convert_product_variants(
                    variant_models,
                ))
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
pub async fn admin_get_product_variant_stock_item_relations(
) -> Result<Vec<ProductVariantStockItemRelation>, ServerFnError> {
    let db = get_db().await;

    let stock_relations_models: Vec<product_variant_stock_item_relations::Model> =
        product_variant_stock_item_relations::Entity::find()
            .all(db)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    let stock_relations =
        entity_conversions::convert_variant_stock_item_relations_batch(stock_relations_models);

    Ok(stock_relations)
}

#[server]
pub async fn admin_create_product(
    request: CreateEditProductRequest,
) -> Result<CreateProductResponse, ServerFnError> {
    let manager = get_current_user().await?;
    if manager.is_none() || !check_admin_permission().await? {
        return Ok(CreateProductResponse {
            success: false,
            message: "Unauthorized".to_string(),
            product_id: None,
        });
    }

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

    let product_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

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
        alternate_names: ActiveValue::Set(alternate_names_value),
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

    let _product_result = products::Entity::insert(product)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create product: {}", e)))?;

    let mut variant_models = Vec::new();
    let mut first_variant_id = None;

    for variant_request in request.variants {
        let variant_id = Uuid::new_v4().to_string();

        if first_variant_id.is_none() {
            first_variant_id = Some(variant_id.clone());
        }

        let variant = product_variants::ActiveModel {
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

    product_variants::Entity::insert_many(variant_models)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create variants: {}", e)))?;

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
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(CreateProductResponse {
            success: false,
            message: "Unauthorized".to_string(),
            product_id: None,
        });
    }

    let product_id = request.id.clone().ok_or_else(|| {
        ServerFnError::new("No product ID provided for edit operation".to_string())
    })?;

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

    let txn = db
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to start transaction: {}", e)))?;

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

    let existing_variants = product_variants::Entity::find()
        .filter(product_variants::Column::ProductId.eq(&product_id))
        .all(&txn)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch existing variants: {}", e)))?;

    let existing_variant_ids: HashSet<String> =
        existing_variants.iter().map(|v| v.id.clone()).collect();

    let mut request_variant_ids = HashSet::new();
    let mut new_variants = Vec::new();
    let mut variants_to_update = Vec::new();
    let mut first_variant_id = None;

    for variant_request in request.variants {
        if let Some(variant_id) = &variant_request.id {
            request_variant_ids.insert(variant_id.clone());

            if first_variant_id.is_none() {
                first_variant_id = Some(variant_id.clone());
            }

            let variant_update = product_variants::ActiveModel {
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
            let variant_id = Uuid::new_v4().to_string();
            request_variant_ids.insert(variant_id.clone());

            if first_variant_id.is_none() {
                first_variant_id = Some(variant_id.clone());
            }

            let new_variant = product_variants::ActiveModel {
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

    let variants_to_delete: Vec<String> = existing_variant_ids
        .difference(&request_variant_ids)
        .cloned()
        .collect();

    use entity::product_variant_stock_item_relations as PVSIR;

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

    if !variants_to_delete.is_empty() {
        product_variants::Entity::delete_many()
            .filter(product_variants::Column::Id.is_in(variants_to_delete))
            .exec(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete variants: {}", e)))?;
    }

    for variant_update in variants_to_update {
        product_variants::Entity::update(variant_update)
            .exec(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to update variant: {}", e)))?;
    }

    if !new_variants.is_empty() {
        product_variants::Entity::insert_many(new_variants)
            .exec(&txn)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to create new variants: {}", e)))?;
    }

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

    if let Some(relations) = request.product_variant_stock_item_relations.clone() {
        let current_product_variant_ids: Vec<String> = product_variants::Entity::find()
            .filter(product_variants::Column::ProductId.eq(&product_id))
            .all(&txn)
            .await
            .map_err(|e| {
                ServerFnError::new(format!("Failed to fetch current product variants: {}", e))
            })?
            .into_iter()
            .map(|v| v.id)
            .collect();

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

        let mut desired_keys: HashSet<(String, String)> = HashSet::new();

        for rel in relations {
            if !current_product_variant_ids.contains(&rel.product_variant_id) {
                return Err(ServerFnError::new(format!(
                    "Relation contains variant ID {} that doesn't belong to current variants of product {}",
                    rel.product_variant_id, product_id
                )));
            }

            desired_keys.insert((rel.product_variant_id.clone(), rel.stock_item_id.clone()));

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
                    .update_columns([PVSIR::Column::Quantity])
                    .to_owned(),
                )
                .exec(&txn)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to upsert relation: {}", e)))?;
        }

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
        let current_product_variant_ids: Vec<String> = product_variants::Entity::find()
            .filter(product_variants::Column::ProductId.eq(&product_id))
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
