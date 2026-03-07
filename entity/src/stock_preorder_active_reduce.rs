use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_preorder_active_reduce")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_name = "orderId", column_type = "Text")]
    pub order_id: String,
    #[sea_orm(column_name = "orderItemId", column_type = "Text")]
    pub order_item_id: String,
    #[sea_orm(column_name = "stockItemId", column_type = "Text")]
    pub stock_item_id: String,
    #[sea_orm(column_name = "reductionQuantity")]
    pub reduction_quantity: i32,
    pub active: bool,
    #[sea_orm(column_name = "stockLocationId", column_type = "Text", nullable)]
    pub stock_location_id: Option<String>,
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
        belongs_to = "super::stock_items::Entity",
        from = "Column::StockItemId",
        to = "super::stock_items::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    StockItems,
    #[sea_orm(
        belongs_to = "super::stock_locations::Entity",
        from = "Column::StockLocationId",
        to = "super::stock_locations::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    StockLocations,
}

impl Related<super::order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Order.def()
    }
}

impl Related<super::stock_items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockItems.def()
    }
}

impl Related<super::stock_locations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockLocations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
