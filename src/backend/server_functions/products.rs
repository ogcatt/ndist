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
    group_members, product_variant_stock_item_relations, product_variants, products,
    sea_orm_active_enums,
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
use super::stock_calculations::{
    calculate_variant_stock_quantities, get_available_stock_by_item,
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
    pub mechanism: Option<String>,
    pub access_groups: Vec<String>,
    pub access_users: Vec<String>,
    pub show_private_preview: bool,
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

#[cfg(feature = "server")]
async fn get_user_group_ids(user_id: &str) -> Result<HashSet<String>, ServerFnError> {
    let db = get_db().await;
    let group_memberships = group_members::Entity::find()
        .filter(group_members::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_db_err()?;

    Ok(group_memberships.into_iter().map(|gm| gm.group_id).collect())
}

#[cfg(feature = "server")]
fn user_has_product_access(
    product_access_groups: &Option<Vec<String>>,
    product_access_users: &Option<Vec<String>>,
    user_id: Option<&str>,
    user_group_ids: &HashSet<String>,
    is_admin: bool,
) -> bool {
    // Admin has access to everything
    if is_admin {
        return true;
    }

    let groups = product_access_groups.as_deref().unwrap_or(&[]);
    let users = product_access_users.as_deref().unwrap_or(&[]);

    // If neither restriction is set, product is accessible to all
    if groups.is_empty() && users.is_empty() {
        return true;
    }

    // Check user-level access first (direct allowlist)
    if let Some(uid) = user_id {
        if users.iter().any(|u| u == uid) {
            return true;
        }
    }

    // Check group-level access
    if !groups.is_empty() {
        return groups.iter().any(|ag| user_group_ids.contains(ag));
    }

    false
}

#[cfg(feature = "server")]
fn strip_product_to_preview(mut product: Product) -> Product {
    // Keep only core information, strip sensitive/detailed data
    product.small_description_md = None;
    product.main_description_md = None;
    product.alternate_names = Some(Vec::new());
    product.cas = None;
    product.iupac = None;
    product.mol_form = None;
    product.smiles = None;
    product.pubchem_cid = None;
    product.analysis_url_qnmr = None;
    product.analysis_url_hplc = None;
    product.analysis_url_qh1 = None;
    product.dimensions_height = None;
    product.dimensions_length = None;
    product.dimensions_width = None;
    product.purity = None;
    product.mechanism = None;

    product
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

    // Get current user and their group memberships in parallel with products
    let (user_result, products_with_variants_result, variant_relations_result, stock_qty_results_result) =
        tokio::join!(
            get_current_user(),
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
            get_available_stock_by_item(None)
        );

    let user = user_result?;
    let is_admin = check_admin_permission().await.unwrap_or(false);
    let user_id = user.as_ref().map(|u| u.id.as_str());

    // Get user's group memberships
    let user_group_ids = if let Some(ref user) = user {
        get_user_group_ids(&user.id).await?
    } else {
        HashSet::new()
    };

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

    products = calculate_variant_stock_quantities(products, variant_relations, &stock_qty_results);

    // Filter products based on access groups
    let filtered_products: Vec<Product> = products
        .into_iter()
        .filter_map(|product| {
            let has_access = user_has_product_access(&product.access_groups, &product.access_users, user_id, &user_group_ids, is_admin);

            if has_access {
                // User has full access
                Some(product)
            } else if product.show_private_preview {
                Some(strip_product_to_preview(product))
            } else {
                None
            }
        })
        .collect();

    Ok(filtered_products)
}

#[server]
pub async fn get_product_by_handle(handle: String) -> Result<Option<Product>, ServerFnError> {
    let db = get_db().await;

    // Fetch product and variants in parallel with user info
    let (product_with_variants_result, user_result) = tokio::join!(
        products::Entity::find()
            .filter(products::Column::Handle.eq(handle))
            .find_with_related(product_variants::Entity)
            .all(db),
        get_current_user()
    );

    let products_with_variants = product_with_variants_result
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Check if product exists
    if products_with_variants.is_empty() {
        return Ok(None);
    }

    let (product_model, variant_models) = products_with_variants
        .into_iter()
        .next()
        .ok_or_else(|| ServerFnError::new("Product not found".to_string()))?;

    let user = user_result?;
    let is_admin = check_admin_permission().await.unwrap_or(false);

    // Check visibility permissions first
    match product_model.visibility {
        sea_orm_active_enums::ProductVisibility::Private => {
            // Private products only visible to admins
            if !is_admin {
                return Ok(None);
            }
        }
        sea_orm_active_enums::ProductVisibility::Unlisted |
        sea_orm_active_enums::ProductVisibility::Public => {
            // Unlisted and public products - check access groups
        }
    }

    // Get user's group memberships
    let user_group_ids = if let Some(ref user) = user {
        get_user_group_ids(&user.id).await?
    } else {
        HashSet::new()
    };

    // Check if user has access to this product based on access_groups / access_users
    let user_id = user.as_ref().map(|u| u.id.as_str());
    let has_access = user_has_product_access(&product_model.access_groups, &product_model.access_users, user_id, &user_group_ids, is_admin);

    // For get_product_by_handle, if user doesn't have access, return None
    // (show_private_preview only applies to the list view, not direct access)
    if !has_access {
        return Ok(None);
    }

    // Fetch variant relations and stock quantities
    let (variant_relations_result, stock_qty_results_result) = tokio::join!(
        async {
            product_variant_stock_item_relations::Entity::find()
                .all(db)
                .await
        },
        get_available_stock_by_item(None)
    );

    let variant_relations = variant_relations_result.map_db_err()?;
    let stock_qty_results = stock_qty_results_result?;

    // Convert variants
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

    // Convert product
    let mut products = entity_conversions::convert_products_batch_with_context(
        vec![product_model],
        vec![context],
    )?;

    // Calculate stock quantities
    products = calculate_variant_stock_quantities(products, variant_relations, &stock_qty_results);

    Ok(products.into_iter().next())
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
        mechanism: ActiveValue::Set(request.mechanism),
        metadata: ActiveValue::NotSet,
        access_groups: ActiveValue::Set(if request.access_groups.is_empty() { None } else { Some(request.access_groups) }),
        access_users: ActiveValue::Set(if request.access_users.is_empty() { None } else { Some(request.access_users) }),
        show_private_preview: ActiveValue::Set(request.show_private_preview),
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
        mechanism: ActiveValue::Set(request.mechanism),
        access_groups: ActiveValue::Set(if request.access_groups.is_empty() { None } else { Some(request.access_groups) }),
        access_users: ActiveValue::Set(if request.access_users.is_empty() { None } else { Some(request.access_users) }),
        show_private_preview: ActiveValue::Set(request.show_private_preview),
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
