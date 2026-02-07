//! `SeaORM` Entity for StockItemRelations

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_item_relations")]
pub struct Model {
    #[sea_orm(primary_key, column_name = "parentStockItemId", auto_increment = false, column_type = "Text")]
    pub parent_stock_item_id: String,
    #[sea_orm(primary_key, column_name = "childStockItemId", auto_increment = false, column_type = "Text")]
    pub child_stock_item_id: String,
    #[sea_orm(column_type = "Double")]
    pub quantity: f64,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    ParentStockItem,
    ChildStockItem,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::ParentStockItem => Entity::belongs_to(super::stock_items::Entity)
                .from(Column::ParentStockItemId)
                .to(super::stock_items::Column::Id)
                .into(),
            Self::ChildStockItem => Entity::belongs_to(super::stock_items::Entity)
                .from(Column::ChildStockItemId)
                .to(super::stock_items::Column::Id)
                .into(),
        }
    }
}

// For the has_many relationship from stock_items
impl Related<super::stock_items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ParentStockItem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}