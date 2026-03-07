use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_location_quantities")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_name = "stockItemId", column_type = "Text")]
    pub stock_item_id: String,
    #[sea_orm(column_name = "stockLocationId", column_type = "Text")]
    pub stock_location_id: String,
    pub quantity: i32,
    pub enabled: bool,
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
    #[sea_orm(
        belongs_to = "super::stock_locations::Entity",
        from = "Column::StockLocationId",
        to = "super::stock_locations::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    StockLocations,
    #[sea_orm(has_many = "super::stock_quantity_adjustments::Entity")]
    StockQuantityAdjustments,
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

impl Related<super::stock_quantity_adjustments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockQuantityAdjustments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
