use super::sea_orm_active_enums::{ShippingOption};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "PreOrder")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_name = "orderItemId", column_type = "Text")]
    pub order_item_id: String, // RELATE THIS
    #[sea_orm(column_name = "parentOrderId", column_type = "Text")]
    pub parent_order_id: String, // RELATE THIS
    #[sea_orm(column_name = "addToEmailList")]
    pub add_to_email_list: bool,
    #[sea_orm(column_name = "shippingOption")]
    pub shipping_option: ShippingOption,
    #[sea_orm(column_name = "orderWeight", column_type = "Double")]
    pub pre_order_weight: f64,
    #[sea_orm(column_name = "fulfilledAt")]
    pub fulfilled_at: Option<DateTime>,
    #[sea_orm(column_name = "preparedAt")]
    pub prepared_at: Option<DateTime>,
    #[sea_orm(column_name = "trackingUrl", column_type = "Text", nullable)]
    pub tracking_url: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub notes: Option<String>,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::order_item::Entity",
        from = "Column::OrderItemId",
        to = "super::order_item::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    OrderItem,
    #[sea_orm(
        belongs_to = "super::order::Entity",
        from = "Column::ParentOrderId",
        to = "super::order::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Order,
}

impl Related<super::order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderItem.def()
    }
}

impl Related<super::order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Order.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
