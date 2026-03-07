// src/backend/server_functions/orders.rs

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use super::super::email::{EmailService, EmailType};

#[cfg(feature = "server")]
use entity::{
    address, order, order_item, payment, pre_order, stock_backorder_active_reduce,
    stock_preorder_active_reduce,
};

#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

#[cfg(feature = "server")]
use chrono::Utc;

use super::super::front_entities::*;
use super::auth::{check_admin_permission, get_current_user};
use super::super::payments;

#[cfg(feature = "server")]
use super::basket::DbErrExt;

#[server]
pub async fn check_payment(payment_id: String) -> Result<PaymentShortInfo, ServerFnError> {
    return payments::check_payment(&payment_id).await;
}

#[server]
pub async fn delete_payment(payment_id: String) -> Result<(), ServerFnError> {
    return payments::cancel_payment(&payment_id, false).await;
}

#[server]
pub async fn get_short_order(order_id: String) -> Result<OrderShortInfo, ServerFnError> {
    let db = get_db().await;

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let payments_fut = payment::Entity::find()
        .filter(payment::Column::OrderId.eq(&order_id))
        .all(db);

    let order_item_fut = order_item::Entity::find()
        .filter(order_item::Column::OrderId.eq(&order_id))
        .all(db);

    let stock_backorder_active_reduces_fut = stock_backorder_active_reduce::Entity::find()
        .filter(stock_backorder_active_reduce::Column::OrderId.eq(&order_id))
        .all(db);

    let pre_order_fut = pre_order::Entity::find()
        .filter(pre_order::Column::ParentOrderId.eq(&order_id))
        .all(db);

    let (
        order_res,
        payments_res,
        order_item_res,
        stock_backorder_active_reduces_res,
        pre_order_res,
    ) = tokio::join!(
        order_fut,
        payments_fut,
        order_item_fut,
        stock_backorder_active_reduces_fut,
        pre_order_fut
    );

    let order_items = order_item_res.map_db_err()?;
    let payments = payments_res.map_db_err()?;
    let stock_backorder_active_reduces = stock_backorder_active_reduces_res.map_db_err()?;
    let pre_orders_entity = pre_order_res.map_db_err()?;

    let order_mod = order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));

    let mut order_short_items: Vec<OrderShortItem> = vec![];

    let paid_at: Option<NaiveDateTime> = payments
        .into_iter()
        .min_by_key(|payment| payment.created_at)
        .map(|payment| payment.created_at);

    for item in order_items {
        order_short_items.push(OrderShortItem {
            id: item.id,
            product_variant_id: item.product_variant_id,
            quantity: item.quantity,
            price_usd: item.price_usd,
            product_title: item.product_title,
            variant_name: item.variant_name,
            pre_order_on_purchase: item.pre_order_on_purchase,
        });
    }

    let mut short_pre_orders: Vec<ShortPreOrder> = vec![];

    for po in &pre_orders_entity {
        if po.parent_order_id == order_id {
            short_pre_orders.push(ShortPreOrder::from(po.clone()))
        }
    }

    let order_short_info = OrderShortInfo {
        ref_code: order_mod.ref_code,
        shipping_option: ShippingOption::from_seaorm(order_mod.shipping_option),
        billing_country: order_mod.billing_country,
        tracking_url: order_mod.tracking_url,
        total_amount_usd: order_mod.total_amount_usd,
        paid: if order_mod.status == entity::sea_orm_active_enums::OrderStatus::Paid {
            true
        } else {
            false
        },
        created_at: Some(order_mod.created_at),
        paid_at: paid_at,
        prepared_at: order_mod.prepared_at,
        fulfilled_at: order_mod.fulfilled_at,
        items: order_short_items,
        pre_orders: short_pre_orders,
        contains_back_order: if stock_backorder_active_reduces.len() > 0 {
            true
        } else {
            false
        },
    };

    Ok(order_short_info)
}

#[server]
pub async fn admin_get_orders(get_expired: bool) -> Result<Vec<OrderInfo>, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        panic!("Unauthorized");
    }

    let db = get_db().await;

    let orders_fut = if get_expired {
        order::Entity::find()
            .filter(order::Column::Status.is_not_in(vec![
                entity::sea_orm_active_enums::OrderStatus::Pending,
                entity::sea_orm_active_enums::OrderStatus::Processing,
            ]))
            .all(db)
    } else {
        order::Entity::find()
            .filter(order::Column::Status.is_not_in(vec![
                entity::sea_orm_active_enums::OrderStatus::Pending,
                entity::sea_orm_active_enums::OrderStatus::Processing,
            ]))
            .all(db)
    };

    let addresses_fut = address::Entity::find().all(db);
    let order_items_fut = order_item::Entity::find().all(db);
    let payments_fut = payment::Entity::find().all(db);
    let backorder_reduces_fut = stock_backorder_active_reduce::Entity::find()
        .filter(stock_backorder_active_reduce::Column::Active.eq(true))
        .all(db);
    let preorder_reduces_fut = stock_preorder_active_reduce::Entity::find()
        .filter(stock_preorder_active_reduce::Column::Active.eq(true))
        .all(db);
    let pre_orders_fut = pre_order::Entity::find().all(db);

    let (
        orders_res,
        addresses_res,
        order_items_res,
        payments_res,
        backorder_reduces_res,
        preorder_reduces_res,
        pre_orders_res,
    ) = tokio::join!(
        orders_fut,
        addresses_fut,
        order_items_fut,
        payments_fut,
        backorder_reduces_fut,
        preorder_reduces_fut,
        pre_orders_fut
    );

    let orders_entity = orders_res.map_db_err()?;
    let addresses_entity = addresses_res.map_db_err()?;
    let order_items_entity = order_items_res.map_db_err()?;
    let payments_entity = payments_res.map_db_err()?;
    let backorder_reduces_entity = backorder_reduces_res.map_db_err()?;
    let preorder_reduces_entity = preorder_reduces_res.map_db_err()?;
    let pre_orders_entity = pre_orders_res.map_db_err()?;

    let mut order_infos: Vec<OrderInfo> = vec![];

    for order in orders_entity {
        let mut payments: Vec<PaymentInfo> = vec![];
        let mut items: Vec<OrderShortItem> = vec![];
        let mut address: Option<CustomerShippingInfo> = None;
        let mut backorder_reduces: Vec<BackOrPreOrderActiveReduce> = vec![];
        let mut preorder_reduces: Vec<BackOrPreOrderActiveReduce> = vec![];
        let mut pre_orders: Vec<PreOrder> = vec![];

        for p in &payments_entity {
            if p.order_id == Some(order.id.clone()) {
                payments.push(PaymentInfo {
                    id: p.id.clone(),
                    method: p.method.clone(),
                    processor_ref: p.processor_ref.clone(),
                    processor_url: p.processor_url.clone(),
                    status: PaymentStatus::from_seaorm(p.status.clone()),
                    amount_usd: p.amount_usd,
                    paid_at: p.paid_at.clone(),
                    created_at: p.created_at.clone(),
                    updated_at: p.updated_at.clone(),
                });
            }
        }

        for o in &order_items_entity {
            if o.order_id == order.id {
                items.push(OrderShortItem {
                    id: o.id.clone(),
                    product_variant_id: o.product_variant_id.clone(),
                    quantity: o.quantity,
                    price_usd: o.price_usd,
                    product_title: o.product_title.clone(),
                    variant_name: o.variant_name.clone(),
                    pre_order_on_purchase: o.pre_order_on_purchase,
                });
            }
        }

        for a in &addresses_entity {
            if a.order_id == order.id {
                address = Some(CustomerShippingInfo {
                    phone: a.phone.clone(),
                    email_list: order.add_to_email_list,
                    first_name: a.first_name.clone(),
                    last_name: a.last_name.clone(),
                    company: a.company.clone(),
                    address_line_1: a.address_line_1.clone(),
                    address_line_2: a.address_line_2.clone(),
                    post_code: a.postal_code.clone(),
                    province: a.province.clone(),
                    city: a.city.clone(),
                    country: Some(a.country.clone()),
                });
            }
        }

        for br in &backorder_reduces_entity {
            if br.order_id == order.id {
                backorder_reduces.push(BackOrPreOrderActiveReduce {
                    id: br.id.clone(),
                    order_id: br.order_id.clone(),
                    order_item_id: br.order_item_id.clone(),
                    stock_item_id: br.stock_item_id.clone(),
                    stock_location_id: br.stock_location_id.clone(),
                    reduction_quantity: br.reduction_quantity,
                    active: br.active,
                    created_at: br.created_at.clone(),
                    updated_at: br.updated_at.clone(),
                });
            }
        }

        for pr in &preorder_reduces_entity {
            if pr.order_id == order.id {
                preorder_reduces.push(BackOrPreOrderActiveReduce {
                    id: pr.id.clone(),
                    order_id: pr.order_id.clone(),
                    order_item_id: pr.order_item_id.clone(),
                    stock_item_id: pr.stock_item_id.clone(),
                    stock_location_id: pr.stock_location_id.clone(),
                    reduction_quantity: pr.reduction_quantity,
                    active: pr.active,
                    created_at: pr.created_at.clone(),
                    updated_at: pr.updated_at.clone(),
                });
            }
        }

        for po in &pre_orders_entity {
            if po.parent_order_id == order.id {
                pre_orders.push(PreOrder::from(po.clone()))
            }
        }

        order_infos.push(OrderInfo {
            id: order.id.clone(),
            ref_code: order.ref_code.clone(),
            customer_id: order.customer_id.clone(),
            customer_email: order.customer_email.clone(),
            add_to_email_list: order.add_to_email_list,
            billing_country: order.billing_country.clone(),
            shipping_option: ShippingOption::from_seaorm(order.shipping_option.clone()),
            subtotal_usd: order.subtotal_usd,
            shipping_usd: order.shipping_usd,
            order_weight: order.order_weight,
            refund_comment: order.refund_comment.clone(),
            status: OrderStatus::from_seaorm(order.status.clone()),
            fulfilled_at: order.fulfilled_at.clone(),
            cancelled_at: order.cancelled_at.clone(),
            refunded_at: order.refunded_at.clone(),
            prepared_at: order.prepared_at.clone(),
            tracking_url: order.tracking_url.clone(),
            total_amount_usd: order.total_amount_usd,
            discount_id: order.discount_id.clone(),
            notes: order.notes.clone(),
            address: address,
            items: items,
            backorder_reduces: backorder_reduces,
            preorder_reduces: preorder_reduces,
            pre_orders: pre_orders,
            payments: payments,
            created_at: order.created_at.clone(),
            updated_at: order.updated_at.clone(),
        });
    }

    Ok(order_infos)
}

#[server]
pub async fn admin_set_order_status(
    order_id: String,
    status: OrderStatus,
) -> Result<(), ServerFnError> {
    match status {
        OrderStatus::Fulfilled => {
            panic!("Can't use set order status to fulfill an order")
        }
        OrderStatus::Cancelled => {
            panic!("Can't use set order status to cancel an order")
        }
        _ => {
            let db = get_db().await;

            let order = order::Entity::find()
                .filter(order::Column::Id.eq(order_id))
                .one(db)
                .await
                .map_db_err()?
                .expect("Could not get order model when trying to update status");

            let mut order_active: order::ActiveModel = order.into();
            order_active.status = ActiveValue::Set(status.to_seaorm());

            order::Entity::update(order_active)
                .exec(db)
                .await
                .map_db_err()?;
        }
    }

    Ok(())
}

#[server]
pub async fn admin_set_order_prepared(order_id: String) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let order = order::Entity::find()
        .filter(order::Column::Id.eq(order_id))
        .one(db)
        .await
        .map_db_err()?
        .expect("Could not get order model when trying to update status");

    let now = Utc::now().naive_utc();

    let mut order_active: order::ActiveModel = order.into();
    order_active.prepared_at = ActiveValue::Set(Some(now));

    order::Entity::update(order_active)
        .exec(db)
        .await
        .map_db_err()?;

    Ok(())
}

#[server]
pub async fn admin_set_preorder_prepared(
    order_item_id: String,
    parent_order_id: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;
    let now = Utc::now().naive_utc();

    let parent_order = order::Entity::find()
        .filter(order::Column::Id.eq(&parent_order_id))
        .one(db)
        .await
        .map_db_err()?
        .ok_or_else(|| ServerFnError::new("Parent order not found"))?;

    let order_item_with_variant = order_item::Entity::find()
        .filter(order_item::Column::Id.eq(&order_item_id))
        .find_also_related(entity::product_variants::Entity)
        .one(db)
        .await
        .map_db_err()?
        .ok_or_else(|| ServerFnError::new("Order item not found"))?;

    let (order_item, variant) = order_item_with_variant;

    let item_weight = match variant {
        Some(variant) => match variant.weight {
            Some(weight) => weight * order_item.quantity as f64,
            None => (80.0 * order_item.quantity as f64 + 30.0),
        },
        None => (80.0 * order_item.quantity as f64 + 30.0),
    };

    let new_preorder = pre_order::ActiveModel {
        id: ActiveValue::Set(uuid::Uuid::new_v4().to_string()),
        order_item_id: ActiveValue::Set(order_item_id),
        parent_order_id: ActiveValue::Set(parent_order_id),
        add_to_email_list: ActiveValue::Set(parent_order.add_to_email_list),
        shipping_option: ActiveValue::Set(parent_order.shipping_option),
        pre_order_weight: ActiveValue::Set(item_weight),
        fulfilled_at: ActiveValue::Set(None),
        prepared_at: ActiveValue::Set(Some(now)),
        tracking_url: ActiveValue::Set(None),
        notes: ActiveValue::Set(None),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    pre_order::Entity::insert(new_preorder)
        .exec(db)
        .await
        .map_db_err()?;

    Ok(())
}

#[server]
pub async fn admin_set_order_fulfilled(
    order_id: String,
    tracking_url: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let (address_res, order_res) = tokio::join!(address_fut, order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod = order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));

    let now = Utc::now().naive_utc();

    let mut order_active: order::ActiveModel = order_mod.clone().into();
    order_active.status = ActiveValue::Set(OrderStatus::Fulfilled.to_seaorm());
    order_active.fulfilled_at = ActiveValue::Set(Some(now));
    order_active.tracking_url = ActiveValue::Set(Some(tracking_url.clone()));

    order::Entity::update(order_active)
        .exec(db)
        .await
        .map_db_err()?;

    let email_service = EmailService::new()?;
    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::TrackingConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
        tracking_url: tracking_url.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
    };

    match email_service
        .send_email(
            &customer_email,
            &customer_name,
            email_type,
            order_mod.add_to_email_list,
        )
        .await
    {
        Ok(()) => tracing::info!("success sending order confirmation email"),
        Err(e) => tracing::info!("{e:?}"),
    }

    Ok(())
}

#[server]
pub async fn admin_set_pre_order_fulfilled(
    order_id: String,
    pre_order_id: String,
    tracking_url: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let pre_order_fut = pre_order::Entity::find()
        .filter(pre_order::Column::Id.eq(&pre_order_id))
        .one(db);

    let (address_res, order_res, pre_order_res) =
        tokio::join!(address_fut, order_fut, pre_order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod = order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));
    let pre_order_mod = pre_order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get pre order by pre_order_id (does it exist?)..."));

    let now = Utc::now().naive_utc();

    let mut pre_order_active: pre_order::ActiveModel = pre_order_mod.clone().into();
    pre_order_active.fulfilled_at = ActiveValue::Set(Some(now));
    pre_order_active.tracking_url = ActiveValue::Set(Some(tracking_url.clone()));

    pre_order::Entity::update(pre_order_active)
        .exec(db)
        .await
        .map_db_err()?;

    let email_service = EmailService::new()?;
    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::PreOrderTrackingConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
        tracking_url: tracking_url.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
    };

    match email_service
        .send_email(
            &customer_email,
            &customer_name,
            email_type,
            pre_order_mod.add_to_email_list,
        )
        .await
    {
        Ok(()) => tracing::info!("success sending pre-order confirmation email"),
        Err(e) => tracing::info!("{e:?}"),
    }

    Ok(())
}

#[server]
pub async fn admin_express_fulfilled_notracking(order_id: String) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let (address_res, order_res) = tokio::join!(address_fut, order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod = order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));

    let now = Utc::now().naive_utc();

    let mut order_active: order::ActiveModel = order_mod.clone().into();
    order_active.status = ActiveValue::Set(OrderStatus::Fulfilled.to_seaorm());
    order_active.fulfilled_at = ActiveValue::Set(Some(now));

    order::Entity::update(order_active)
        .exec(db)
        .await
        .map_db_err()?;

    let email_service = EmailService::new()?;
    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::ExpressDispatchConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
    };

    match email_service
        .send_email(
            &customer_email,
            &customer_name,
            email_type,
            order_mod.add_to_email_list,
        )
        .await
    {
        Ok(()) => tracing::info!("success sending order confirmation email"),
        Err(e) => tracing::info!("{e:?}"),
    }

    Ok(())
}

#[server]
pub async fn admin_express_pre_order_fulfilled_notracking(
    order_id: String,
    pre_order_id: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let pre_order_fut = pre_order::Entity::find()
        .filter(pre_order::Column::Id.eq(&pre_order_id))
        .one(db);

    let (address_res, order_res, pre_order_res) =
        tokio::join!(address_fut, order_fut, pre_order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod = order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));
    let pre_order_mod = pre_order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get pre order by pre_order_id (does it exist?)..."));

    let now = Utc::now().naive_utc();

    let mut pre_order_active: pre_order::ActiveModel = pre_order_mod.clone().into();
    pre_order_active.fulfilled_at = ActiveValue::Set(Some(now));

    pre_order::Entity::update(pre_order_active)
        .exec(db)
        .await
        .map_db_err()?;

    let email_service = EmailService::new()?;
    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::ExpressPreOrderDispatchConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
    };

    match email_service
        .send_email(
            &customer_email,
            &customer_name,
            email_type,
            order_mod.add_to_email_list,
        )
        .await
    {
        Ok(()) => {
            tracing::info!("success sending express notracking pre-order confirmation email")
        }
        Err(e) => tracing::info!("{e:?}"),
    }

    Ok(())
}

#[server]
pub async fn admin_express_order_send_tracking(
    order_id: String,
    tracking_url: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let (address_res, order_res) = tokio::join!(address_fut, order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod = order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));

    let mut order_active: order::ActiveModel = order_mod.clone().into();
    order_active.tracking_url = ActiveValue::Set(Some(tracking_url.clone()));

    order::Entity::update(order_active)
        .exec(db)
        .await
        .map_db_err()?;

    let email_service = EmailService::new()?;
    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::ExpressTrackingConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
        tracking_url: tracking_url.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
    };

    match email_service
        .send_email(
            &customer_email,
            &customer_name,
            email_type,
            order_mod.add_to_email_list,
        )
        .await
    {
        Ok(()) => tracing::info!("success sending order confirmation email"),
        Err(e) => tracing::info!("{e:?}"),
    }

    Ok(())
}

#[server]
pub async fn admin_express_pre_order_send_tracking(
    order_id: String,
    pre_order_id: String,
    tracking_url: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let address_fut = address::Entity::find()
        .filter(address::Column::OrderId.eq(&order_id))
        .one(db);

    let order_fut = order::Entity::find()
        .filter(order::Column::Id.eq(&order_id))
        .one(db);

    let pre_order_fut = pre_order::Entity::find()
        .filter(pre_order::Column::Id.eq(&pre_order_id))
        .one(db);

    let (address_res, order_res, pre_order_res) =
        tokio::join!(address_fut, order_fut, pre_order_fut);

    let address_entity = address_res.map_db_err()?;
    let order_mod = order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get order by order_id (does it exist?)..."));
    let pre_order_mod = pre_order_res
        .map_db_err()?
        .unwrap_or_else(|| panic!("Could not get pre order by pre_order_id (does it exist?)..."));

    let mut pre_order_active: pre_order::ActiveModel = pre_order_mod.clone().into();
    pre_order_active.tracking_url = ActiveValue::Set(Some(tracking_url.clone()));

    pre_order::Entity::update(pre_order_active)
        .exec(db)
        .await
        .map_db_err()?;

    let email_service = EmailService::new()?;
    let customer_email = order_mod.customer_email.clone();

    let email_type = EmailType::ExpressPreOrderTrackingConfirmation {
        order_id: order_mod.id.clone(),
        order_ref: order_mod.ref_code.clone(),
        tracking_url: tracking_url.clone(),
    };

    let customer_name = if let Some(address) = address_entity {
        address.first_name.clone()
    } else {
        customer_email.clone()
    };

    match email_service
        .send_email(
            &customer_email,
            &customer_name,
            email_type,
            order_mod.add_to_email_list,
        )
        .await
    {
        Ok(()) => tracing::info!("success sending order confirmation email"),
        Err(e) => tracing::info!("{e:?}"),
    }

    Ok(())
}

#[server]
pub async fn admin_update_order_notes(
    order_id: String,
    notes: String,
) -> Result<(), ServerFnError> {
    let db = get_db().await;

    let order = order::Entity::find()
        .filter(order::Column::Id.eq(order_id))
        .one(db)
        .await
        .map_db_err()?
        .expect("Could not get order model when trying to update notes");

    let mut order_active: order::ActiveModel = order.into();
    order_active.notes = ActiveValue::Set(if notes.trim().is_empty() {
        None
    } else {
        Some(notes)
    });

    order::Entity::update(order_active)
        .exec(db)
        .await
        .map_db_err()?;

    Ok(())
}
