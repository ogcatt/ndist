use super::sea_orm_active_enums::StockUnit;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_name = "pbiSku", column_type = "Text", unique)]
    pub pbi_sku: String,
    #[sea_orm(column_type = "Text")]
    pub name: String,
    #[sea_orm(column_name = "description", column_type = "Text", nullable)]
    pub description: Option<String>,
    #[sea_orm(column_name = "thumbnailRef", column_type = "Text", nullable)]
    pub thumbnail_ref: Option<String>,
    pub unit: StockUnit,
    #[sea_orm(column_name = "assemblyMinutes", nullable)]
    pub assembly_minutes: Option<i32>,
    #[sea_orm(column_name = "defaultShippingDays", nullable)]
    pub default_shipping_days: Option<i32>,
    #[sea_orm(column_name = "defaultCost", column_type = "Double", nullable)]
    pub default_cost: Option<f64>,
    #[sea_orm(column_name = "warningQuantity", column_type = "Double", nullable)]
    pub warning_quantity: Option<f64>,
    #[sea_orm(column_name = "isContainer")]
    pub is_container: bool,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    ProductVariantStockItemRelations,
    StockBatches,
    StockItemRelations,
    StockBackorderActiveReduce,
    StockPreorderActiveReduce,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::ProductVariantStockItemRelations => {
                Entity::has_many(super::product_variant_stock_item_relations::Entity).into()
            }
            Self::StockBatches => Entity::has_many(super::stock_batches::Entity).into(),
            Self::StockItemRelations => {
                Entity::has_many(super::stock_item_relations::Entity).into()
            }
            Self::StockBackorderActiveReduce => {
                Entity::has_many(super::stock_backorder_active_reduce::Entity).into()
            }
            Self::StockPreorderActiveReduce => {
                Entity::has_many(super::stock_preorder_active_reduce::Entity).into()
            }
        }
    }
}

impl Related<super::product_variant_stock_item_relations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductVariantStockItemRelations.def()
    }
}

impl Related<super::stock_batches::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockBatches.def()
    }
}

impl Related<super::stock_item_relations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockItemRelations.def()
    }
}

impl Related<super::stock_backorder_active_reduce::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockBackorderActiveReduce.def()
    }
}

impl Related<super::stock_preorder_active_reduce::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockPreorderActiveReduce.def()
    }
}

// Many-to-many relationship through the junction table
impl Related<super::product_variants::Entity> for Entity {
    fn to() -> RelationDef {
        super::product_variant_stock_item_relations::Relation::ProductVariants.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::product_variant_stock_item_relations::Relation::StockItems.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
