use super::sea_orm_active_enums::{OrderStatus, ShippingOption};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "Order")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_name = "refCode", column_type = "Text", unique)]
    pub ref_code: String,
    #[sea_orm(column_name = "customerId", column_type = "Text", nullable)]
    pub customer_id: Option<String>,
    #[sea_orm(column_name = "customerEmail", column_type = "Text")]
    pub customer_email: String,
    #[sea_orm(column_name = "addToEmailList")]
    pub add_to_email_list: bool,
    #[sea_orm(column_name = "billingCountry", column_type = "Text")]
    pub billing_country: String,
    #[sea_orm(column_name = "shippingOption")]
    pub shipping_option: ShippingOption,
    #[sea_orm(column_name = "subtotalUsd", column_type = "Double")]
    pub subtotal_usd: f64,
    #[sea_orm(column_name = "shippingUsd", column_type = "Double")]
    pub shipping_usd: f64,
    #[sea_orm(column_name = "orderWeight", column_type = "Double")]
    pub order_weight: f64,
    #[sea_orm(column_name = "refundComment", column_type = "Text", nullable)]
    pub refund_comment: Option<String>,
    pub status: OrderStatus,
    #[sea_orm(column_name = "fulfilledAt")]
    pub fulfilled_at: Option<DateTime>,
    #[sea_orm(column_name = "cancelledAt")]
    pub cancelled_at: Option<DateTime>,
    #[sea_orm(column_name = "refundedAt")]
    pub refunded_at: Option<DateTime>,
    #[sea_orm(column_name = "preparedAt")]
    pub prepared_at: Option<DateTime>,
    #[sea_orm(column_name = "trackingUrl", column_type = "Text", nullable)]
    pub tracking_url: Option<String>,
    #[sea_orm(column_name = "totalAmountUsd", column_type = "Double")]
    pub total_amount_usd: f64,
    #[sea_orm(column_name = "discountId", column_type = "Text", nullable)]
    pub discount_id: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub notes: Option<String>,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
    #[sea_orm(column_name = "updatedAt")]
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::order_item::Entity")]
    OrderItem,
    #[sea_orm(has_many = "super::payment::Entity")]
    Payment,
    #[sea_orm(has_many = "super::address::Entity")]
    Address,
    #[sea_orm(has_many = "super::stock_active_reduce::Entity")]
    StockActiveReduce,
    #[sea_orm(
        belongs_to = "super::customers::Entity",
        from = "Column::CustomerId",
        to = "super::customers::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Customers,
    #[sea_orm(has_many = "super::pre_order::Entity")]
    PreOrder,
}

impl Related<super::order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderItem.def()
    }
}

impl Related<super::payment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Payment.def()
    }
}

impl Related<super::address::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Address.def()
    }
}

impl Related<super::customers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customers.def()
    }
}

impl Related<super::stock_active_reduce::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockActiveReduce.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
