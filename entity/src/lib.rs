// Re-export all modules
pub mod address;
pub mod affiliate_users;
pub mod affiliate_withdrawls;
pub mod audit_log;
pub mod auth_tokens;
pub mod basket_items;
pub mod customer_baskets;
pub mod customers;
pub mod discounts;
pub mod user_sessions;
pub mod users;
pub mod order;
pub mod order_item;
pub mod payment;
pub mod prelude;
pub mod product_categories;
pub mod product_images;
pub mod product_variant_stock_item_relations;
pub mod product_variants;
pub mod products;
//pub mod regions;
pub mod blog_posts;
pub mod sea_orm_active_enums;
pub mod stock_backorder_active_reduce;
pub mod stock_preorder_active_reduce;
pub mod stock_items;
pub mod stock_locations;
pub mod stock_location_quantities;
pub mod stock_quantity_adjustments;
pub mod variant_images;
pub mod pre_order;
pub mod groups;
pub mod group_members;

// Add this custom type for handling string arrays
pub mod types {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::Value as JsonValue;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct StringVec(pub Vec<String>);

    impl From<StringVec> for JsonValue {
        fn from(vec: StringVec) -> Self {
            serde_json::to_value(vec.0).unwrap_or(JsonValue::Null)
        }
    }

    impl From<StringVec> for Value {
        fn from(vec: StringVec) -> Self {
            // Box the JsonValue properly
            let json_value = serde_json::to_value(vec.0).unwrap_or(JsonValue::Null);
            Value::Json(Some(Box::new(json_value)))
        }
    }

    impl From<Vec<String>> for StringVec {
        fn from(vec: Vec<String>) -> Self {
            StringVec(vec)
        }
    }
}
