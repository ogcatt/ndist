pub use sea_orm_migration::prelude::*;

mod add_active_to_reduces;
mod add_auth_tokens;
mod add_basket_locks;
mod add_blog_posts;
mod add_cancelled_payments;
mod add_collections_to_products;
mod add_company_to_address;
mod add_discount_reduce;
mod add_discount_to_order;
mod add_discounts;
mod add_discounts_new;
mod add_more_reduces;
mod add_order_item_to_reduces;
mod add_payment_expire;
mod add_payment_failed_at;
mod add_pre_order;
mod add_pre_order_to_order_item;
mod add_processor_url;
mod add_product_metadata;
mod add_stock_active_reduce;
mod add_stock_item_relations;
mod add_weight_to_variants;
mod add_working_inventory;
mod add_workinger_inventory;
mod expand_product;
mod expand_variants;
mod fix_discounts_v1;
mod fix_i32_for_stock_items;
mod fix_order_time;
mod fix_reduce_time;
mod fix_stock_items;
mod fix_stock_items_v2;
mod make_batches_items_instead;
mod make_collections_array;
mod make_customer_optional;
mod make_discount_unique;
mod make_payment_solo;
mod make_phase_required;
mod modify_address;
mod modify_address_order_foreign_key_cascade;
mod modify_order_item_foreign_key;
mod modify_orders;
mod more_basket;
mod optional_expire_at;
mod order_customer_optional;
mod order_email_list;
mod rem_assembled_and_add_batches;
mod remove_old_inventory;
mod rename_managers_to_userss;
mod update_order;
mod add_mechanism_to_products;
mod add_groups;
mod add_access_groups;
mod add_stock_locations;
mod add_stock_location_enabled;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            /*
            Box::new(rename_managers_to_users::Migration),
            Box::new(expand_product::Migration),
            Box::new(expand_variants::Migration),
            Box::new(add_collections_to_products::Migration),
            Box::new(make_collections_array::Migration),
            Box::new(remove_old_inventory::Migration),
            //Box::new(add_working_inventory::Migration) ROLLED BACK
            //Box::new(add_workinger_inventory::Migration)
            //Box::new(fix_stock_items::Migration)
            Box::new(fix_stock_items_v2::Migration),
            Box::new(rem_assembled_and_add_batches::Migration),
            Box::new(fix_i32_for_stock_items::Migration),
            Box::new(add_stock_item_relations::Migration),
            Box::new(make_customer_optional::Migration),
            Box::new(more_basket::Migration),
            Box::new(add_weight_to_variants::Migration),
            //Box::new(add_discounts::Migration), ROLLED BACK
            Box::new(add_discounts_new::Migration),
            Box::new(optional_expire_at::Migration),
            Box::new(make_discount_unique::Migration),
            Box::new(fix_discounts_v1::Migration),
            Box::new(add_basket_locks::Migration),
            Box::new(modify_orders::Migration),
            Box::new(add_processor_url::Migration),
            Box::new(order_customer_optional::Migration),
            Box::new(modify_address::Migration),
            Box::new(add_stock_active_reduce::Migration),
            Box::new(fix_reduce_time::Migration),
            Box::new(add_discount_reduce::Migration),
            Box::new(add_discount_to_order::Migration),
            Box::new(add_cancelled_payments::Migration),
            Box::new(add_payment_failed_at::Migration),
            Box::new(update_order::Migration),
            Box::new(make_payment_solo::Migration),
            Box::new(add_payment_expire::Migration),
            Box::new(order_email_list::Migration),
            Box::new(add_company_to_address::Migration),
            Box::new(fix_order_time::Migration),
            Box::new(add_product_metadata::Migration),
            Box::new(make_phase_required::Migration),
            Box::new(add_blog_posts::Migration),
            Box::new(add_more_reduces::Migration),
            Box::new(add_active_to_reduces::Migration),
            Box::new(add_pre_order_to_order_item::Migration),
            Box::new(add_order_item_to_reduces::Migration),
            Box::new(add_pre_order::Migration),
            Box::new(make_batches_items_instead::Migration),
            Box::new(modify_order_item_foreign_key::Migration),
            Box::new(modify_address_order_foreign_key_cascade::Migration),
            */
            Box::new(add_auth_tokens::Migration),
            Box::new(add_mechanism_to_products::Migration),
            Box::new(rename_managers_to_userss::Migration),
            Box::new(add_groups::Migration),
            Box::new(add_access_groups::Migration),
            Box::new(add_stock_locations::Migration),
            Box::new(add_stock_location_enabled::Migration)
        ]
    }
}
