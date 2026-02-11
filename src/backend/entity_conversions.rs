#[cfg(feature = "server")]
use super::front_entities::*; // Adjust path to your public entity
#[cfg(feature = "server")]
use chrono::{DateTime, NaiveDateTime, Utc};
#[cfg(feature = "server")]
use entity::{
    basket_items, blog_posts, customer_baskets, discounts, product_variant_stock_item_relations,
    products, stock_batches, stock_item_relations, stock_items, pre_order
};
#[cfg(feature = "server")]
use pulldown_cmark::{Options, Parser, html};
#[cfg(feature = "server")]
use regex::Regex;

#[cfg(feature = "server")]
pub fn markdown_to_html(markdown_input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(markdown_input, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Apply the same transformations as the JavaScript version
    html_output = html_output
        .replace(
            "<h1>",
            r#"<h1 class="text-2xl pt-3 pb-[2px] tagge"><span class="samerow">"#,
        )
        .replace("</h1>", "</span></h1>")
        .replace(
            "<h2>",
            r#"<h2 class="text-xl pt-3 pb-[2px] tagge"><span class="samerow">"#,
        )
        .replace("</h2>", "</span></h2>")
        .replace(
            "<h3>",
            r#"<h3 class="text-xl pt-4 pb-0 tagge"><span class="samerow">"#,
        )
        .replace("</h3>", "</span></h3>")
        .replace(
            "<h4>",
            r#"<h4 class="text-lg pt-4 pb-0 tagge"><span class="samerow">"#,
        )
        .replace("</h4>", "</span></h4>")
        .replace("<p>", r#"<p class="para">"#)
        .replace("<caption>", "<figcaption><small>")
        .replace("</caption>", "</small></figcaption>")
        .replace(
            "<ul>",
            r#"<ul class="py-2 paratext" style="list-style-type: disc;list-style-position: inside;">"#,
        );

    // Handle links with regex for more flexibility
    let link_regex = Regex::new(r#"<a href="https://([^"]*)"#).unwrap();
    html_output = link_regex.replace_all(&html_output, r#"<a title="Open link in new tab." class="new-tab-link" target="_blank" rel="noreferrer" href="https://$1""#).to_string();

    // Handle images
    let img_regex = Regex::new(r"<img src=").unwrap();
    html_output = img_regex
        .replace_all(
            &html_output,
            r#"<img class="md-img" onload="this.style.opacity=1" src="#,
        )
        .to_string();

    html_output
}

/// Convert SeaORM Product entity to public-facing Product
#[cfg(feature = "server")]
impl From<products::Model> for Product {
    fn from(model: products::Model) -> Self {
        convert_product_internal(model, true, ProductConversionContext::default())
    }
}

/// Convert SeaORM StockItem entity to public-facing StockItem
#[cfg(feature = "server")]
impl From<stock_items::Model> for StockItem {
    fn from(model: stock_items::Model) -> Self {
        StockItem {
            id: model.id,
            pbi_sku: model.pbi_sku,
            name: model.name,
            description: model.description,
            thumbnail_ref: model.thumbnail_ref,
            unit: StockUnit::from_seaorm(model.unit),
            assembly_minutes: model.assembly_minutes,
            default_shipping_days: model.default_shipping_days,
            default_cost: model.default_cost,
            warning_quantity: model.warning_quantity,
            is_container: model.is_container,
            //assembled: model.assembled, DEPRECATED
            sub_relations: None, // Ideally should be Some(Vec<StockItemRelation>) if exists
            stock_quantities: None,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

/// Convert SeaORM StockItemRelation entity to public-facing StockItemRelation
#[cfg(feature = "server")]
impl From<stock_item_relations::Model> for StockItemRelation {
    fn from(model: stock_item_relations::Model) -> Self {
        StockItemRelation {
            // Create a composite ID from parent and child IDs
            //ref_id: format!("{}_{}", model.parent_stock_item_id, model.child_stock_item_id),
            parent_stock_item_id: model.parent_stock_item_id,
            child_stock_item_id: model.child_stock_item_id,
            quantity: model.quantity,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

/// Batch conversion for multiple products (with markdown conversion)
#[cfg(feature = "server")]
pub fn convert_stock_item_relations_batch(
    models: Vec<stock_item_relations::Model>,
) -> Vec<StockItemRelation> {
    models.into_iter().map(StockItemRelation::from).collect()
}

/// Convert SeaORM StockItemRelation entity to public-facing StockItemRelation
#[cfg(feature = "server")]
impl From<product_variant_stock_item_relations::Model> for ProductVariantStockItemRelation {
    fn from(model: product_variant_stock_item_relations::Model) -> Self {
        ProductVariantStockItemRelation {
            // Create a composite ID from parent and child IDs
            //ref_id: format!("{}_{}", model.product_variant_id, model.stock_item_id),
            product_variant_id: model.product_variant_id,
            stock_item_id: model.stock_item_id,
            quantity: model.quantity,
            stock_unit_on_creation: StockUnit::from_seaorm(model.stock_unit_on_creation),
        }
    }
}

/// Batch conversion for multiple products (with markdown conversion)
#[cfg(feature = "server")]
pub fn convert_variant_stock_item_relations_batch(
    models: Vec<product_variant_stock_item_relations::Model>,
) -> Vec<ProductVariantStockItemRelation> {
    models
        .into_iter()
        .map(ProductVariantStockItemRelation::from)
        .collect()
}

/// Batch conversion for multiple stock items with relations
#[cfg(feature = "server")]
pub fn convert_stock_items_batch_with_relations(
    models_with_relations: Vec<(stock_items::Model, Vec<stock_item_relations::Model>)>,
) -> Vec<StockItem> {
    models_with_relations
        .into_iter()
        .map(|(stock_item_model, relations_models)| {
            convert_stock_item_with_relations(stock_item_model, relations_models)
        })
        .collect()
}

/// Convert a single stock item with its relations
#[cfg(feature = "server")]
pub fn convert_stock_item_with_relations(
    model: stock_items::Model,
    relations: Vec<stock_item_relations::Model>,
) -> StockItem {
    let sub_relations = if relations.is_empty() {
        None
    } else {
        Some(relations.into_iter().map(StockItemRelation::from).collect())
    };

    StockItem {
        id: model.id,
        pbi_sku: model.pbi_sku,
        name: model.name,
        description: model.description,
        thumbnail_ref: model.thumbnail_ref,
        unit: StockUnit::from_seaorm(model.unit),
        assembly_minutes: model.assembly_minutes,
        default_shipping_days: model.default_shipping_days,
        default_cost: model.default_cost,
        warning_quantity: model.warning_quantity,
        is_container: model.is_container,
        sub_relations,
        stock_quantities: None,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

/// Convert SeaORM StockItem entity to public-facing StockItem with optional stock quantities
#[cfg(feature = "server")]
pub fn convert_stock_item_with_quantities(
    model: stock_items::Model,
    stock_quantities: &[StockQuantityResult],
) -> StockItem {
    // Find matching stock quantity result
    let matching_quantity = stock_quantities
        .iter()
        .find(|sq| sq.stock_item_id == model.id)
        .cloned(); // Clone the result if found

    StockItem {
        id: model.id,
        pbi_sku: model.pbi_sku,
        name: model.name,
        description: model.description,
        thumbnail_ref: model.thumbnail_ref,
        unit: StockUnit::from_seaorm(model.unit),
        assembly_minutes: model.assembly_minutes,
        default_shipping_days: model.default_shipping_days,
        default_cost: model.default_cost,
        warning_quantity: model.warning_quantity,
        is_container: model.is_container,
        sub_relations: None, // Ideally should be Some(Vec<StockItemRelation>) if exists
        stock_quantities: matching_quantity, // Will be Some(...) if found, None if not
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

/// Batch conversion for multiple stock items with stock quantities
#[cfg(feature = "server")]
pub fn convert_stock_items_batch_with_quantities(
    models: Vec<stock_items::Model>,
    stock_quantities: &[StockQuantityResult],
) -> Vec<StockItem> {
    models
        .into_iter()
        .map(|model| convert_stock_item_with_quantities(model, stock_quantities))
        .collect()
}

/// Convert with additional data for fields not present in SeaORM model
#[cfg(feature = "server")]
pub struct ProductConversionContext {
    pub product_phase: ProductPhase,
    pub variants: Option<Vec<ProductVariants>>,
}

#[cfg(feature = "server")]
impl Default for ProductConversionContext {
    fn default() -> Self {
        Self {
            product_phase: ProductPhase::default(),
            variants: None,
        }
    }
}

/// Internal function to handle conversion with markdown processing control
#[cfg(feature = "server")]
fn convert_product_internal(
    model: products::Model,
    convert_markdown: bool,
    context: ProductConversionContext,
) -> Product {
    let small_description = if convert_markdown {
        model
            .small_description_md
            .as_ref()
            .map(|md| markdown_to_html(md))
    } else {
        model.small_description_md.clone()
    };

    let main_description = if convert_markdown {
        model
            .main_description_md
            .as_ref()
            .map(|md| markdown_to_html(md))
    } else {
        model.main_description_md.clone()
    };

    Product {
        id: model.id,
        title: model.title,
        subtitle: model.subtitle,
        alternate_names: model.alternate_names,
        handle: model.handle,
        collections: model.collections,
        product_form: ProductForm::from_seaorm(model.product_form),
        physical_description: model.physical_description,
        default_variant_id: model.default_variant_id,
        force_no_stock: model.force_no_stock,
        plabs_node_id: model.plabs_node_id,
        purity: model.purity,
        visibility: ProductVisibility::from_seaorm(model.visibility),
        small_description_md: small_description,
        main_description_md: main_description,
        cas: model.cas,
        iupac: model.iupac,
        mol_form: model.mol_form,
        smiles: model.smiles,
        enable_render_if_smiles: model.enable_render_if_smiles,
        pubchem_cid: model.pubchem_cid,
        calculated_admet: model.calculated_admet,
        analysis_url_qnmr: model.analysis_url_qnmr,
        analysis_url_hplc: model.analysis_url_hplc,
        analysis_url_qh1: model.analysis_url_qh1,
        weight: model.weight,
        dimensions_height: model.dimensions_height,
        dimensions_length: model.dimensions_length,
        dimensions_width: model.dimensions_width,
        created_at: model.created_at,
        updated_at: model.updated_at,
        pre_order: model.pre_order,
        pre_order_goal: model.pre_order_goal,
        phase: ProductPhase::from_seaorm(model.phase),
        brand: model.brand,
        priority: model.priority,
        back_order: model.back_order,
        mechanism: model.mechanism,
        metadata: model.metadata,
        access_groups: model.access_groups,
        show_private_preview: model.show_private_preview,
        variants: context.variants,
    }
}

/// Convert SeaORM Product with additional context data (with markdown conversion)
#[cfg(feature = "server")]
pub fn convert_product_with_context(
    model: products::Model,
    context: ProductConversionContext,
) -> Product {
    convert_product_internal(model, true, context)
}

/// Convert SeaORM Product with additional context data (without markdown conversion)
#[cfg(feature = "server")]
pub fn convert_product_with_context_no_markdown(
    model: products::Model,
    context: ProductConversionContext,
) -> Product {
    convert_product_internal(model, false, context)
}

/// Convert SeaORM Product with markdown conversion control
#[cfg(feature = "server")]
pub fn convert_product_with_markdown_option(
    model: products::Model,
    convert_markdown: bool,
    context: Option<ProductConversionContext>,
) -> Product {
    convert_product_internal(model, convert_markdown, context.unwrap_or_default())
}

/// Helper function to convert product variants from SeaORM to frontend entities
#[cfg(feature = "server")]
pub fn convert_product_variants(
    variants: Vec<entity::product_variants::Model>,
) -> Vec<ProductVariants> {
    variants
        .into_iter()
        .map(|variant| ProductVariants {
            id: variant.id,
            variant_name: variant.variant_name,
            product_id: variant.product_id,
            pbx_sku: variant.pbx_sku,
            thumbnail_url: variant.thumbnail_url,
            weight: variant.weight,
            price_standard_usd: variant.price_standard_usd,
            price_standard_without_sale: variant.price_standard_without_sale,
            additional_thumbnail_urls: variant.additional_thumbnail_urls,
            calculated_stock_quantity: None,
            created_at: variant.created_at,
            updated_at: variant.updated_at,
        })
        .collect()
}

/// Convert product with variants loaded (with markdown conversion)
#[cfg(feature = "server")]
pub fn convert_product_with_variants(
    model: products::Model,
    variants: Vec<entity::product_variants::Model>,
    product_phase: Option<ProductPhase>,
) -> Product {
    let context = ProductConversionContext {
        product_phase: product_phase.unwrap_or_default(),
        variants: Some(convert_product_variants(variants)),
    };
    convert_product_with_context(model, context)
}

/// Convert product with variants loaded (without markdown conversion)
#[cfg(feature = "server")]
pub fn convert_product_with_variants_no_markdown(
    model: products::Model,
    variants: Vec<entity::product_variants::Model>,
    product_phase: Option<ProductPhase>,
) -> Product {
    let context = ProductConversionContext {
        product_phase: product_phase.unwrap_or_default(),
        variants: Some(convert_product_variants(variants)),
    };
    convert_product_with_context_no_markdown(model, context)
}

/// Convert product with variants loaded and markdown conversion control
#[cfg(feature = "server")]
pub fn convert_product_with_variants_and_markdown_option(
    model: products::Model,
    variants: Vec<entity::product_variants::Model>,
    product_phase: Option<ProductPhase>,
    convert_markdown: bool,
) -> Product {
    let context = ProductConversionContext {
        product_phase: product_phase.unwrap_or_default(),
        variants: Some(convert_product_variants(variants)),
    };
    convert_product_with_markdown_option(model, convert_markdown, Some(context))
}

/// Batch conversion for multiple products (with markdown conversion)
#[cfg(feature = "server")]
pub fn convert_products_batch(models: Vec<products::Model>) -> Vec<Product> {
    models.into_iter().map(Product::from).collect()
}

/// Batch conversion for multiple products (without markdown conversion)
#[cfg(feature = "server")]
pub fn convert_products_batch_no_markdown(models: Vec<products::Model>) -> Vec<Product> {
    models
        .into_iter()
        .map(|model| convert_product_internal(model, false, ProductConversionContext::default()))
        .collect()
}

/// Batch conversion for multiple products with markdown conversion control
#[cfg(feature = "server")]
pub fn convert_products_batch_with_markdown_option(
    models: Vec<products::Model>,
    convert_markdown: bool,
) -> Vec<Product> {
    models
        .into_iter()
        .map(|model| {
            convert_product_internal(model, convert_markdown, ProductConversionContext::default())
        })
        .collect()
}

/// Batch conversion with context for multiple products (with markdown conversion)
#[cfg(feature = "server")]
pub fn convert_products_batch_with_context(
    models: Vec<products::Model>,
    contexts: Vec<ProductConversionContext>,
) -> Result<Vec<Product>, ConversionError> {
    if models.len() != contexts.len() {
        return Err(ConversionError::MismatchedLength {
            models: models.len(),
            contexts: contexts.len(),
        });
    }

    Ok(models
        .into_iter()
        .zip(contexts)
        .map(|(model, context)| convert_product_with_context(model, context))
        .collect())
}

/// Batch conversion with context for multiple products (without markdown conversion)
#[cfg(feature = "server")]
pub fn convert_products_batch_with_context_no_markdown(
    models: Vec<products::Model>,
    contexts: Vec<ProductConversionContext>,
) -> Result<Vec<Product>, ConversionError> {
    if models.len() != contexts.len() {
        return Err(ConversionError::MismatchedLength {
            models: models.len(),
            contexts: contexts.len(),
        });
    }

    Ok(models
        .into_iter()
        .zip(contexts)
        .map(|(model, context)| convert_product_with_context_no_markdown(model, context))
        .collect())
}

/// Batch conversion with context and markdown conversion control for multiple products
#[cfg(feature = "server")]
pub fn convert_products_batch_with_context_and_markdown_option(
    models: Vec<products::Model>,
    contexts: Vec<ProductConversionContext>,
    convert_markdown: bool,
) -> Result<Vec<Product>, ConversionError> {
    if models.len() != contexts.len() {
        return Err(ConversionError::MismatchedLength {
            models: models.len(),
            contexts: contexts.len(),
        });
    }

    Ok(models
        .into_iter()
        .zip(contexts)
        .map(|(model, context)| convert_product_internal(model, convert_markdown, context))
        .collect())
}

/// Convenience function for converting with default context (with markdown conversion)
#[cfg(feature = "server")]
pub fn convert_product_with_defaults(model: products::Model) -> Product {
    convert_product_with_context(model, ProductConversionContext::default())
}

/// Convenience function for converting with default context (without markdown conversion)
#[cfg(feature = "server")]
pub fn convert_product_with_defaults_no_markdown(model: products::Model) -> Product {
    convert_product_with_context_no_markdown(model, ProductConversionContext::default())
}

/// Batch conversion for multiple stock items
#[cfg(feature = "server")]
pub fn convert_stock_items_batch(models: Vec<stock_items::Model>) -> Vec<StockItem> {
    models.into_iter().map(StockItem::from).collect()
}

/// Convert a single stock item (convenience function)
#[cfg(feature = "server")]
pub fn convert_stock_item(model: stock_items::Model) -> StockItem {
    StockItem::from(model)
}

#[cfg(feature = "server")]
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Mismatched length: {models} models but {contexts} contexts")]
    MismatchedLength { models: usize, contexts: usize },
}

// Implement From trait to allow using ? operator with ConversionError in server functions
#[cfg(feature = "server")]
impl From<ConversionError> for dioxus::prelude::ServerFnError {
    fn from(err: ConversionError) -> Self {
        dioxus::prelude::ServerFnError::new(format!("Conversion error: {}", err))
    }
}

// Updated conversion implementations
#[cfg(feature = "server")]
impl From<customer_baskets::Model> for CustomerBasket {
    fn from(model: customer_baskets::Model) -> Self {
        CustomerBasket {
            id: model.id,
            customer_id: model.customer_id,
            country_code: model.country_code,
            discount_code: model.discount_code,
            shipping_option: model.shipping_option.map(ShippingOption::from_seaorm),
            shipping_results: None,
            locked: model.locked,
            payment_id: model.payment_id,
            payment_failed_at: model.payment_failed_at,
            created_at: model.created_at,
            updated_at: model.updated_at,
            items: None, // Will be set separately
            discount: None,
        }
    }
}

#[cfg(feature = "server")]
impl ShippingOption {
    pub fn from_seaorm(visibility: entity::sea_orm_active_enums::ShippingOption) -> Self {
        match visibility {
            entity::sea_orm_active_enums::ShippingOption::Tracked => Self::Tracked,
            entity::sea_orm_active_enums::ShippingOption::Express => Self::Express,
            entity::sea_orm_active_enums::ShippingOption::TrackedUS => Self::TrackedUS,
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::ShippingOption {
        match self {
            Self::Tracked => entity::sea_orm_active_enums::ShippingOption::Tracked,
            Self::Express => entity::sea_orm_active_enums::ShippingOption::Express,
            Self::TrackedUS => entity::sea_orm_active_enums::ShippingOption::TrackedUS,
        }
    }
}

#[cfg(feature = "server")]
impl From<basket_items::Model> for CustomerBasketItem {
    fn from(model: basket_items::Model) -> Self {
        CustomerBasketItem {
            id: model.id,
            basket_id: model.basket_id,
            product_variant_id: model.variant_id,
            quantity: model.quantity,
        }
    }
}

/// Batch conversion for multiple basket items
#[cfg(feature = "server")]
pub fn convert_basket_items_batch(models: Vec<basket_items::Model>) -> Vec<CustomerBasketItem> {
    models.into_iter().map(CustomerBasketItem::from).collect()
}

// Conversion functions for the enums - FROM SeaORM TO Frontend
#[cfg(feature = "server")]
impl ProductForm {
    pub fn from_seaorm(form: entity::sea_orm_active_enums::ProductForm) -> Self {
        match form {
            entity::sea_orm_active_enums::ProductForm::Ampoule => Self::Ampoule,
            entity::sea_orm_active_enums::ProductForm::Capsules => Self::Capsules,
            entity::sea_orm_active_enums::ProductForm::Container => Self::Container,
            entity::sea_orm_active_enums::ProductForm::DirectSpray => Self::DirectSpray,
            entity::sea_orm_active_enums::ProductForm::Multi => Self::Multi,
            entity::sea_orm_active_enums::ProductForm::Other => Self::Other,
            entity::sea_orm_active_enums::ProductForm::Solution => Self::Solution,
            entity::sea_orm_active_enums::ProductForm::VerticalSpray => Self::VerticalSpray,
            entity::sea_orm_active_enums::ProductForm::Vial => Self::Vial,
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::ProductForm {
        match self {
            Self::Ampoule => entity::sea_orm_active_enums::ProductForm::Ampoule,
            Self::Capsules => entity::sea_orm_active_enums::ProductForm::Capsules,
            Self::Container => entity::sea_orm_active_enums::ProductForm::Container,
            Self::DirectSpray => entity::sea_orm_active_enums::ProductForm::DirectSpray,
            Self::Multi => entity::sea_orm_active_enums::ProductForm::Multi,
            Self::Other => entity::sea_orm_active_enums::ProductForm::Other,
            Self::Solution => entity::sea_orm_active_enums::ProductForm::Solution,
            Self::VerticalSpray => entity::sea_orm_active_enums::ProductForm::VerticalSpray,
            Self::Vial => entity::sea_orm_active_enums::ProductForm::Vial,
        }
    }
}

#[cfg(feature = "server")]
impl ProductVisibility {
    pub fn from_seaorm(visibility: entity::sea_orm_active_enums::ProductVisibility) -> Self {
        match visibility {
            entity::sea_orm_active_enums::ProductVisibility::Private => Self::Private,
            entity::sea_orm_active_enums::ProductVisibility::Public => Self::Public,
            entity::sea_orm_active_enums::ProductVisibility::Unlisted => Self::Unlisted,
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::ProductVisibility {
        match self {
            Self::Private => entity::sea_orm_active_enums::ProductVisibility::Private,
            Self::Public => entity::sea_orm_active_enums::ProductVisibility::Public,
            Self::Unlisted => entity::sea_orm_active_enums::ProductVisibility::Unlisted,
        }
    }
}

#[cfg(feature = "server")]
impl Default for ProductPhase {
    fn default() -> Self {
        Self::Blue
    }
}

#[cfg(feature = "server")]
impl StockUnit {
    pub fn from_seaorm(unit: entity::sea_orm_active_enums::StockUnit) -> Self {
        match unit {
            entity::sea_orm_active_enums::StockUnit::Multiples => Self::Multiples,
            entity::sea_orm_active_enums::StockUnit::Grams => Self::Grams,
            entity::sea_orm_active_enums::StockUnit::Milliliters => Self::Milliliters,
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::StockUnit {
        match self {
            Self::Multiples => entity::sea_orm_active_enums::StockUnit::Multiples,
            Self::Grams => entity::sea_orm_active_enums::StockUnit::Grams,
            Self::Milliliters => entity::sea_orm_active_enums::StockUnit::Milliliters,
        }
    }
}

// For stock batches

/// Convert SeaORM StockBatch entity to public-facing StockBatch
#[cfg(feature = "server")]
impl From<stock_batches::Model> for StockBatch {
    fn from(model: stock_batches::Model) -> Self {
        let stock_unit = StockUnit::from_seaorm(model.stock_unit_on_creation.clone());

        StockBatch {
            id: model.id,
            stock_batch_code: model.stock_batch_code,
            stock_item_id: model.stock_item_id,
            comment: model.comment,
            supplier: model.supplier,
            original_quantity: convert_quantity_to_stock_unit_quantity(
                Some(model.original_quantity),
                &stock_unit,
            ),
            live_quantity: convert_quantity_to_stock_unit_quantity(
                Some(model.live_quantity),
                &stock_unit,
            ),
            stock_unit_on_creation: stock_unit,
            cost_usd: model.cost_usd,
            arrival_date: model.arrival_date,
            warehouse_location: StockBatchLocation::from_seaorm(model.warehouse_location),
            tracking_url: model.tracking_url,
            assembled: model.assembled,
            status: StockBatchStatus::from_seaorm(model.status),
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

/// Multi conversion for multiple stock batches (with markdown conversion)
#[cfg(feature = "server")]
pub fn convert_stock_batches_batch(models: Vec<stock_batches::Model>) -> Vec<StockBatch> {
    models.into_iter().map(StockBatch::from).collect()
}

/// StockBatchStatus enum conversions
#[cfg(feature = "server")]
impl StockBatchStatus {
    pub fn from_seaorm(status: entity::sea_orm_active_enums::StockBatchStatus) -> Self {
        match status {
            entity::sea_orm_active_enums::StockBatchStatus::Draft => Self::Draft,
            entity::sea_orm_active_enums::StockBatchStatus::Paid => Self::Paid,
            entity::sea_orm_active_enums::StockBatchStatus::Complete => Self::Complete,
            entity::sea_orm_active_enums::StockBatchStatus::Issue => Self::Issue,
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::StockBatchStatus {
        match self {
            Self::Draft => entity::sea_orm_active_enums::StockBatchStatus::Draft,
            Self::Paid => entity::sea_orm_active_enums::StockBatchStatus::Paid,
            Self::Complete => entity::sea_orm_active_enums::StockBatchStatus::Complete,
            Self::Issue => entity::sea_orm_active_enums::StockBatchStatus::Issue,
        }
    }
}

/// StockBatchLocation enum conversions
#[cfg(feature = "server")]
impl StockBatchLocation {
    pub fn from_seaorm(location: entity::sea_orm_active_enums::StockBatchLocation) -> Self {
        match location {
            entity::sea_orm_active_enums::StockBatchLocation::EU => Self::EU,
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::StockBatchLocation {
        match self {
            Self::EU => entity::sea_orm_active_enums::StockBatchLocation::EU,
        }
    }
}

/// StockMode enum conversions (if you need them)
#[cfg(feature = "server")]
impl StockMode {
    pub fn from_seaorm(mode: entity::sea_orm_active_enums::StockMode) -> Self {
        match mode {
            entity::sea_orm_active_enums::StockMode::Calculated => Self::Calculated,
            entity::sea_orm_active_enums::StockMode::ForceStocked => Self::ForceStocked,
            entity::sea_orm_active_enums::StockMode::ForceUnstocked => Self::ForceUnstocked,
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::StockMode {
        match self {
            Self::Calculated => entity::sea_orm_active_enums::StockMode::Calculated,
            Self::ForceStocked => entity::sea_orm_active_enums::StockMode::ForceStocked,
            Self::ForceUnstocked => entity::sea_orm_active_enums::StockMode::ForceUnstocked,
        }
    }
}

// Discount conversion implementations

// Convert FROM SeaORM TO Frontend for the struct
#[cfg(feature = "server")]
impl From<discounts::Model> for Discount {
    fn from(model: discounts::Model) -> Self {
        Discount {
            id: model.id,
            code: model.code,
            affiliate_id: model.affiliate_id,
            active: model.active,
            discount_type: DiscountType::from_seaorm(model.discount_type),
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
        }
    }
}

/// Batch conversion for multiple products (with markdown conversion)
#[cfg(feature = "server")]
pub fn convert_discounts_batch(models: Vec<discounts::Model>) -> Vec<Discount> {
    models.into_iter().map(Discount::from).collect()
}

// Enum conversion implementations
#[cfg(feature = "server")]
impl DiscountType {
    // Convert FROM SeaORM TO Frontend
    pub fn from_seaorm(discount_type: entity::sea_orm_active_enums::DiscountType) -> Self {
        match discount_type {
            entity::sea_orm_active_enums::DiscountType::Percentage => Self::Percentage,
            entity::sea_orm_active_enums::DiscountType::FixedAmount => Self::FixedAmount,
            entity::sea_orm_active_enums::DiscountType::PercentageOnShipping => {
                Self::PercentageOnShipping
            }
            entity::sea_orm_active_enums::DiscountType::FixedAmountOnShipping => {
                Self::FixedAmountOnShipping
            }
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::DiscountType {
        match self {
            Self::Percentage => entity::sea_orm_active_enums::DiscountType::Percentage,
            Self::FixedAmount => entity::sea_orm_active_enums::DiscountType::FixedAmount,
            Self::PercentageOnShipping => {
                entity::sea_orm_active_enums::DiscountType::PercentageOnShipping
            }
            Self::FixedAmountOnShipping => {
                entity::sea_orm_active_enums::DiscountType::FixedAmountOnShipping
            }
        }
    }
}

/*
* Cancelled,
Failed,
Paid,
Pending,
Refunded,
*/

#[cfg(feature = "server")]
impl PaymentStatus {
    pub fn from_seaorm(status: entity::sea_orm_active_enums::PaymentStatus) -> Self {
        match status {
            entity::sea_orm_active_enums::PaymentStatus::Cancelled => Self::Cancelled,
            entity::sea_orm_active_enums::PaymentStatus::Failed => Self::Failed,
            entity::sea_orm_active_enums::PaymentStatus::Paid => Self::Paid,
            entity::sea_orm_active_enums::PaymentStatus::Pending => Self::Pending,
            entity::sea_orm_active_enums::PaymentStatus::Refunded => Self::Refunded,
            entity::sea_orm_active_enums::PaymentStatus::Expired => Self::Expired,
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::PaymentStatus {
        match self {
            Self::Cancelled => entity::sea_orm_active_enums::PaymentStatus::Cancelled,
            Self::Failed => entity::sea_orm_active_enums::PaymentStatus::Failed,
            Self::Paid => entity::sea_orm_active_enums::PaymentStatus::Paid,
            Self::Pending => entity::sea_orm_active_enums::PaymentStatus::Pending,
            Self::Refunded => entity::sea_orm_active_enums::PaymentStatus::Refunded,
            Self::Expired => entity::sea_orm_active_enums::PaymentStatus::Expired,
        }
    }
}

#[cfg(feature = "server")]
impl OrderStatus {
    pub fn from_seaorm(status: entity::sea_orm_active_enums::OrderStatus) -> Self {
        match status {
            entity::sea_orm_active_enums::OrderStatus::Cancelled => Self::Cancelled,
            entity::sea_orm_active_enums::OrderStatus::Fulfilled => Self::Fulfilled,
            entity::sea_orm_active_enums::OrderStatus::Paid => Self::Paid,
            entity::sea_orm_active_enums::OrderStatus::Pending => Self::Pending,
            entity::sea_orm_active_enums::OrderStatus::Processing => Self::Processing,
            entity::sea_orm_active_enums::OrderStatus::Refunded => Self::Refunded,
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::OrderStatus {
        match self {
            Self::Cancelled => entity::sea_orm_active_enums::OrderStatus::Cancelled,
            Self::Fulfilled => entity::sea_orm_active_enums::OrderStatus::Fulfilled,
            Self::Paid => entity::sea_orm_active_enums::OrderStatus::Paid,
            Self::Pending => entity::sea_orm_active_enums::OrderStatus::Pending,
            Self::Processing => entity::sea_orm_active_enums::OrderStatus::Processing,
            Self::Refunded => entity::sea_orm_active_enums::OrderStatus::Refunded,
        }
    }
}

#[cfg(feature = "server")]
impl ProductPhase {
    pub fn from_seaorm(status: entity::sea_orm_active_enums::ProductPhase) -> Self {
        match status {
            entity::sea_orm_active_enums::ProductPhase::Blue => Self::Blue,
            entity::sea_orm_active_enums::ProductPhase::Purple => Self::Purple,
            entity::sea_orm_active_enums::ProductPhase::Orange => Self::Orange,
        }
    }

    // Convert FROM Frontend TO SeaORM
    pub fn to_seaorm(&self) -> entity::sea_orm_active_enums::ProductPhase {
        match self {
            Self::Blue => entity::sea_orm_active_enums::ProductPhase::Blue,
            Self::Purple => entity::sea_orm_active_enums::ProductPhase::Purple,
            Self::Orange => entity::sea_orm_active_enums::ProductPhase::Orange,
        }
    }
}

/// Convert SeaORM BlogPost entity to public-facing BlogPost
#[cfg(feature = "server")]
fn convert_blog_post_internal(model: blog_posts::Model, convert_markdown: bool) -> BlogPost {
    let blog_md = if convert_markdown {
        markdown_to_html(&model.blog_md)
    } else {
        model.blog_md.clone()
    };

    BlogPost {
        id: model.id,
        title: model.title,
        subtitle: model.subtitle,
        thumbnail_url: model.thumbnail_url,
        blog_md: blog_md,
        posted_at: model.posted_at,
        updated_at: model.updated_at,
    }
}

/// Batch conversion for multiple blog posts (with markdown conversion)
#[cfg(feature = "server")]
pub fn convert_blog_posts_batch(
    models: Vec<blog_posts::Model>,
    convert_markdown: bool,
) -> Result<Vec<BlogPost>, ConversionError> {
    Ok(models
        .into_iter()
        .map(|model| convert_blog_post_internal(model, convert_markdown))
        .collect())
}

#[cfg(feature = "server")]
pub fn convert_blog_post(
    model: blog_posts::Model,
    convert_markdown: bool,
) -> Result<BlogPost, ConversionError> {
    Ok(convert_blog_post_internal(model, convert_markdown))
}


#[cfg(feature = "server")]
impl From<pre_order::Model> for PreOrder {
    fn from(model: pre_order::Model) -> Self {
        PreOrder {
            id: model.id,
            order_item_id: model.order_item_id,
            parent_order_id: model.parent_order_id,
            add_to_email_list: model.add_to_email_list,
            shipping_option: ShippingOption::from_seaorm(model.shipping_option),
            pre_order_weight: model.pre_order_weight,
            fulfilled_at: model.fulfilled_at,
            prepared_at: model.prepared_at,
            tracking_url: model.tracking_url,
            notes: model.notes,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

// Convert multiple pre orders
#[cfg(feature = "server")]
pub fn convert_pre_orders_batch(models: Vec<pre_order::Model>) -> Vec<PreOrder> {
    models.into_iter().map(PreOrder::from).collect()
}



#[cfg(feature = "server")]
impl From<pre_order::Model> for ShortPreOrder {
    fn from(model: pre_order::Model) -> Self {
        ShortPreOrder {
            id: model.id,
            order_item_id: model.order_item_id,
            parent_order_id: model.parent_order_id,
            shipping_option: ShippingOption::from_seaorm(model.shipping_option),
            fulfilled_at: model.fulfilled_at,
            prepared_at: model.prepared_at,
            tracking_url: model.tracking_url,
        }
    }
}

// Convert multiple pre orders
#[cfg(feature = "server")]
pub fn convert_short_pre_orders_batch(models: Vec<pre_order::Model>) -> Vec<ShortPreOrder> {
    models.into_iter().map(ShortPreOrder::from).collect()
}
