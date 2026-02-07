use super::sea_orm_active_enums::{StockBatchStatus, StockBatchLocation, StockUnit};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_batches")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_name = "stockBatchCode", column_type = "Text", unique)]
    pub stock_batch_code: String,
    #[sea_orm(column_name = "stockItemId", column_type = "Text")]
    pub stock_item_id: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub comment: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub supplier: Option<String>,
    #[sea_orm(column_name = "originalQuantity")]
    pub original_quantity: f64,
    #[sea_orm(column_name = "liveQuantity")]
    pub live_quantity: f64,
    #[sea_orm(column_name = "stockUnitOnCreation")]
    pub stock_unit_on_creation: StockUnit,
    #[sea_orm(column_name = "costUsd", column_type = "Double", nullable)]
    pub cost_usd: Option<f64>,
    #[sea_orm(column_name = "arrivalDate", nullable)]
    pub arrival_date: Option<DateTime>,
    #[sea_orm(column_name = "warehouseLocation")]
    pub warehouse_location: StockBatchLocation,
    #[sea_orm(column_name = "trackingUrl", column_type = "Text", nullable)]
    pub tracking_url: Option<String>,
    pub status: StockBatchStatus,
    pub assembled: bool,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::stock_items::Entity",
        from = "Column::StockItemId",
        to = "super::stock_items::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    StockItems,
    #[sea_orm(has_many = "super::stock_active_reduce::Entity")] // New relation
    StockActiveReduce,
}

impl Related<super::stock_items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockItems.def()
    }
}

// New relation implementation
impl Related<super::stock_active_reduce::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockActiveReduce.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
