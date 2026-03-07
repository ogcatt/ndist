use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_quantity_adjustments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_name = "stockLocationQuantityId", column_type = "Text")]
    pub stock_location_quantity_id: String,
    /// Positive for additions, negative for subtractions
    pub delta: i32,
    #[sea_orm(column_type = "Text")]
    pub note: String,
    #[sea_orm(column_name = "adjustedBy", column_type = "Text", nullable)]
    pub adjusted_by: Option<String>,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::stock_location_quantities::Entity",
        from = "Column::StockLocationQuantityId",
        to = "super::stock_location_quantities::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    StockLocationQuantities,
}

impl Related<super::stock_location_quantities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockLocationQuantities.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
