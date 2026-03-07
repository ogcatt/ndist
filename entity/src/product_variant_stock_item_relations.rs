//! `SeaORM` Entity for ProductStockItemRelations

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "product_variant_stock_item_relations")]
pub struct Model {
    #[sea_orm(primary_key, column_name = "productVariantId", auto_increment = false, column_type = "Text")]
    pub product_variant_id: String,
    #[sea_orm(primary_key, column_name = "stockItemId", auto_increment = false, column_type = "Text")]
    pub stock_item_id: String,
    pub quantity: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::product_variants::Entity",
        from = "Column::ProductVariantId",
        to = "super::product_variants::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    ProductVariants,
    #[sea_orm(
        belongs_to = "super::stock_items::Entity",
        from = "Column::StockItemId",
        to = "super::stock_items::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    StockItems,
}

impl Related<super::product_variants::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductVariants.def()
    }
}

impl Related<super::stock_items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockItems.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
