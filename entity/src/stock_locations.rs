use super::sea_orm_active_enums::StockLocationShippingMethod;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_locations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text")]
    pub name: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    #[sea_orm(column_name = "shippingMethod")]
    pub shipping_method: StockLocationShippingMethod,
    #[sea_orm(column_name = "flatRateUsd", column_type = "Double", nullable)]
    pub flat_rate_usd: Option<f64>,
    #[sea_orm(column_type = "Text", nullable)]
    pub country: Option<String>,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::stock_location_quantities::Entity")]
    StockLocationQuantities,
    #[sea_orm(has_many = "super::customer_baskets::Entity")]
    CustomerBaskets,
}

impl Related<super::stock_location_quantities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockLocationQuantities.def()
    }
}

impl Related<super::customer_baskets::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CustomerBaskets.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
