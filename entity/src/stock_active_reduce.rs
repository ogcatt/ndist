use super::sea_orm_active_enums::StockUnit;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_active_reduce")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_name = "orderId", column_type = "Text")]
    pub order_id: String,
    #[sea_orm(column_name = "stockBatchId", column_type = "Text")]
    pub stock_batch_id: String,
    #[sea_orm(column_name = "stockUnit")]
    pub stock_unit: StockUnit,
    #[sea_orm(column_name = "reductionQuantity", column_type = "Double")] // Fixed typo: was #[seaorm]
    pub reduction_quantity: f64,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::order::Entity",
        from = "Column::OrderId",
        to = "super::order::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Order,
    #[sea_orm(
        belongs_to = "super::stock_batches::Entity",
        from = "Column::StockBatchId",
        to = "super::stock_batches::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    StockBatches,
}

impl Related<super::order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Order.def()
    }
}

impl Related<super::stock_batches::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockBatches.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
