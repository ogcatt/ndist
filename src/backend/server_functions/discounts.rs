// src/backend/server_functions/discounts.rs

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use super::super::entity_conversions;

#[cfg(feature = "server")]
use entity::{basket_items, discounts, product_variants};

#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

#[cfg(feature = "server")]
use chrono::Utc;

#[cfg(feature = "server")]
use uuid::Uuid;

use super::super::front_entities::*;
use super::auth::{check_admin_permission, get_current_user};

#[cfg(feature = "server")]
use super::basket::DbErrExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscountRequest {
    pub code: String,
    pub discount_type: DiscountType,
    pub discount_percentage: Option<f64>,
    pub discount_amount: Option<f64>,
    pub active: bool,
    pub maximum_uses: Option<i32>,
    pub valid_countries: Option<Vec<String>>,
    pub valid_after_x_products: Option<i32>,
    pub valid_after_x_total: Option<f64>,
    pub auto_apply: bool,
    pub expire_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscountResponse {
    pub success: bool,
    pub message: String,
    pub discount_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDiscountRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDiscountResponse {
    pub success: bool,
    pub message: String,
    pub discount: Option<Discount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiscountRequest {
    pub id: String,
    pub code: String,
    pub discount_percentage: Option<f64>,
    pub discount_amount: Option<f64>,
    pub active: bool,
    pub maximum_uses: Option<i32>,
    pub valid_countries: Option<Vec<String>>,
    pub valid_after_x_products: Option<i32>,
    pub valid_after_x_total: Option<f64>,
    pub auto_apply: bool,
    pub expire_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiscountResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteDiscountRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteDiscountResponse {
    pub success: bool,
    pub message: String,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, PartialEq)]
pub struct DiscountValidationResponse {
    pub discount: discounts::Model,
    pub is_valid: bool,
}

#[server]
pub async fn admin_get_discounts() -> Result<Vec<Discount>, ServerFnError> {
    let db = get_db().await;

    let discount_models: Vec<discounts::Model> =
        discounts::Entity::find().all(db).await.map_db_err()?;

    let discounts_final = entity_conversions::convert_discounts_batch(discount_models);

    Ok(discounts_final)
}

#[server]
pub async fn admin_create_discount(
    request: CreateDiscountRequest,
) -> Result<CreateDiscountResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(CreateDiscountResponse {
            success: false,
            message: "Unauthorized".to_string(),
            discount_id: None,
        });
    }

    if request.code.trim().is_empty() {
        return Ok(CreateDiscountResponse {
            success: false,
            message: "Discount code is required".to_string(),
            discount_id: None,
        });
    }

    if !request
        .code
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Ok(CreateDiscountResponse {
            success: false,
            message: "Discount code can only contain letters, numbers, underscores, and hyphens"
                .to_string(),
            discount_id: None,
        });
    }

    match request.discount_type {
        DiscountType::Percentage | DiscountType::PercentageOnShipping => {
            if request.discount_percentage.is_none() {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount percentage is required for percentage-based discounts"
                        .to_string(),
                    discount_id: None,
                });
            }
            let percentage = request.discount_percentage.unwrap();
            if percentage <= 0.0 {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount percentage must be greater than 0".to_string(),
                    discount_id: None,
                });
            }
            if percentage > 100.0 {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount percentage cannot exceed 100%".to_string(),
                    discount_id: None,
                });
            }
            if request.discount_amount.is_some() {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount amount must not be set for percentage-based discounts"
                        .to_string(),
                    discount_id: None,
                });
            }
        }
        DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => {
            if request.discount_amount.is_none() {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount amount is required for fixed amount discounts".to_string(),
                    discount_id: None,
                });
            }
            let amount = request.discount_amount.unwrap();
            if amount <= 0.0 {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount amount must be greater than 0".to_string(),
                    discount_id: None,
                });
            }
            if request.discount_percentage.is_some() {
                return Ok(CreateDiscountResponse {
                    success: false,
                    message: "Discount percentage must not be set for fixed amount discounts"
                        .to_string(),
                    discount_id: None,
                });
            }
        }
    }

    if let Some(max_uses) = request.maximum_uses {
        if max_uses <= 0 {
            return Ok(CreateDiscountResponse {
                success: false,
                message: "Maximum uses must be greater than 0".to_string(),
                discount_id: None,
            });
        }
    }

    if let Some(min_products) = request.valid_after_x_products {
        if min_products < 0 {
            return Ok(CreateDiscountResponse {
                success: false,
                message: "Minimum products cannot be negative".to_string(),
                discount_id: None,
            });
        }
    }

    if let Some(min_total) = request.valid_after_x_total {
        if min_total < 0.0 {
            return Ok(CreateDiscountResponse {
                success: false,
                message: "Minimum cart total cannot be negative".to_string(),
                discount_id: None,
            });
        }
    }

    if let Some(expire_date) = request.expire_at {
        if expire_date <= Utc::now().naive_utc() {
            return Ok(CreateDiscountResponse {
                success: false,
                message: "Expiration date must be in the future".to_string(),
                discount_id: None,
            });
        }
    }

    let db = get_db().await;

    let existing_discount = discounts::Entity::find()
        .filter(discounts::Column::Code.eq(&request.code.to_uppercase()))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if existing_discount.is_some() {
        return Ok(CreateDiscountResponse {
            success: false,
            message: "A discount with this code already exists".to_string(),
            discount_id: None,
        });
    }

    let discount_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    let discount_type_seaorm = match request.discount_type {
        DiscountType::Percentage => entity::sea_orm_active_enums::DiscountType::Percentage,
        DiscountType::FixedAmount => entity::sea_orm_active_enums::DiscountType::FixedAmount,
        DiscountType::PercentageOnShipping => {
            entity::sea_orm_active_enums::DiscountType::PercentageOnShipping
        }
        DiscountType::FixedAmountOnShipping => {
            entity::sea_orm_active_enums::DiscountType::FixedAmountOnShipping
        }
    };

    let discount = discounts::ActiveModel {
        id: ActiveValue::Set(discount_id.clone()),
        code: ActiveValue::Set(request.code.to_uppercase()),
        affiliate_id: ActiveValue::Set(None),
        active: ActiveValue::Set(request.active),
        discount_type: ActiveValue::Set(discount_type_seaorm),
        discount_percentage: ActiveValue::Set(request.discount_percentage),
        discount_amount: ActiveValue::Set(request.discount_amount),
        amount_used: ActiveValue::Set(None),
        maximum_uses: ActiveValue::Set(request.maximum_uses),
        discount_used: ActiveValue::Set(0),
        active_reduce_quantity: ActiveValue::Set(0),
        valid_countries: ActiveValue::Set(request.valid_countries),
        valid_after_x_products: ActiveValue::Set(request.valid_after_x_products),
        valid_after_x_total: ActiveValue::Set(request.valid_after_x_total),
        auto_apply: ActiveValue::Set(request.auto_apply),
        expire_at: ActiveValue::Set(request.expire_at),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    discounts::Entity::insert(discount)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create discount: {}", e)))?;

    Ok(CreateDiscountResponse {
        success: true,
        message: "Discount created successfully".to_string(),
        discount_id: Some(discount_id),
    })
}

#[server]
pub async fn admin_get_discount(
    request: GetDiscountRequest,
) -> Result<GetDiscountResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(GetDiscountResponse {
            success: false,
            message: "Unauthorized".to_string(),
            discount: None,
        });
    }

    let db = get_db().await;

    let discount_model = discounts::Entity::find()
        .filter(discounts::Column::Id.eq(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    match discount_model {
        Some(model) => {
            let discount_type = match model.discount_type {
                entity::sea_orm_active_enums::DiscountType::Percentage => DiscountType::Percentage,
                entity::sea_orm_active_enums::DiscountType::FixedAmount => {
                    DiscountType::FixedAmount
                }
                entity::sea_orm_active_enums::DiscountType::PercentageOnShipping => {
                    DiscountType::PercentageOnShipping
                }
                entity::sea_orm_active_enums::DiscountType::FixedAmountOnShipping => {
                    DiscountType::FixedAmountOnShipping
                }
            };

            let discount = Discount {
                id: model.id,
                code: model.code,
                affiliate_id: model.affiliate_id,
                active: model.active,
                discount_type,
                discount_percentage: model.discount_percentage,
                discount_amount: model.discount_amount,
                amount_used: model.amount_used,
                maximum_uses: model.maximum_uses,
                discount_used: model.discount_used,
                active_reduce_quantity: model.active_reduce_quantity,
                valid_countries: model.valid_countries,
                valid_after_x_products: model.valid_after_x_products,
                valid_after_x_total: model.valid_after_x_total,
                auto_apply: model.auto_apply,
                expire_at: model.expire_at,
                created_at: model.created_at,
                updated_at: model.updated_at,
            };

            Ok(GetDiscountResponse {
                success: true,
                message: "Discount found".to_string(),
                discount: Some(discount),
            })
        }
        None => Ok(GetDiscountResponse {
            success: false,
            message: "Discount not found".to_string(),
            discount: None,
        }),
    }
}

#[server]
pub async fn admin_update_discount(
    request: UpdateDiscountRequest,
) -> Result<UpdateDiscountResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(UpdateDiscountResponse {
            success: false,
            message: "Unauthorized".to_string(),
        });
    }

    if request.code.trim().is_empty() {
        return Ok(UpdateDiscountResponse {
            success: false,
            message: "Discount code is required".to_string(),
        });
    }

    if !request
        .code
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Ok(UpdateDiscountResponse {
            success: false,
            message: "Discount code can only contain letters, numbers, underscores, and hyphens"
                .to_string(),
        });
    }

    if let Some(max_uses) = request.maximum_uses {
        if max_uses <= 0 {
            return Ok(UpdateDiscountResponse {
                success: false,
                message: "Maximum uses must be greater than 0".to_string(),
            });
        }
    }

    if let Some(min_products) = request.valid_after_x_products {
        if min_products < 0 {
            return Ok(UpdateDiscountResponse {
                success: false,
                message: "Minimum products cannot be negative".to_string(),
            });
        }
    }

    if let Some(min_total) = request.valid_after_x_total {
        if min_total < 0.0 {
            return Ok(UpdateDiscountResponse {
                success: false,
                message: "Minimum cart total cannot be negative".to_string(),
            });
        }
    }

    if let Some(expire_date) = request.expire_at {
        if expire_date <= Utc::now().naive_utc() {
            return Ok(UpdateDiscountResponse {
                success: false,
                message: "Expiration date must be in the future".to_string(),
            });
        }
    }

    let db = get_db().await;

    let existing_discount = discounts::Entity::find()
        .filter(discounts::Column::Id.eq(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let discount_model = match existing_discount {
        Some(model) => model,
        None => {
            return Ok(UpdateDiscountResponse {
                success: false,
                message: "Discount not found".to_string(),
            });
        }
    };

    match discount_model.discount_type {
        entity::sea_orm_active_enums::DiscountType::Percentage
        | entity::sea_orm_active_enums::DiscountType::PercentageOnShipping => {
            if request.discount_percentage.is_none() {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount percentage is required for percentage-based discounts"
                        .to_string(),
                });
            }
            let percentage = request.discount_percentage.unwrap();
            if percentage <= 0.0 {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount percentage must be greater than 0".to_string(),
                });
            }
            if percentage > 100.0 {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount percentage cannot exceed 100%".to_string(),
                });
            }
            if request.discount_amount.is_some() {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount amount must not be set for percentage-based discounts"
                        .to_string(),
                });
            }
        }
        entity::sea_orm_active_enums::DiscountType::FixedAmount
        | entity::sea_orm_active_enums::DiscountType::FixedAmountOnShipping => {
            if request.discount_amount.is_none() {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount amount is required for fixed amount discounts".to_string(),
                });
            }
            let amount = request.discount_amount.unwrap();
            if amount <= 0.0 {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount amount must be greater than 0".to_string(),
                });
            }
            if request.discount_percentage.is_some() {
                return Ok(UpdateDiscountResponse {
                    success: false,
                    message: "Discount percentage must not be set for fixed amount discounts"
                        .to_string(),
                });
            }
        }
    }

    let code_conflict = discounts::Entity::find()
        .filter(discounts::Column::Code.eq(&request.code.to_uppercase()))
        .filter(discounts::Column::Id.ne(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if code_conflict.is_some() {
        return Ok(UpdateDiscountResponse {
            success: false,
            message: "A discount with this code already exists".to_string(),
        });
    }

    let now = Utc::now().naive_utc();

    let updated_discount = discounts::ActiveModel {
        id: ActiveValue::Unchanged(discount_model.id),
        code: ActiveValue::Set(request.code.to_uppercase()),
        affiliate_id: ActiveValue::Unchanged(discount_model.affiliate_id),
        active: ActiveValue::Set(request.active),
        discount_type: ActiveValue::Unchanged(discount_model.discount_type),
        discount_percentage: ActiveValue::Set(request.discount_percentage),
        discount_amount: ActiveValue::Set(request.discount_amount),
        amount_used: ActiveValue::Unchanged(discount_model.amount_used),
        maximum_uses: ActiveValue::Set(request.maximum_uses),
        discount_used: ActiveValue::Unchanged(discount_model.discount_used),
        active_reduce_quantity: ActiveValue::Unchanged(discount_model.active_reduce_quantity),
        valid_countries: ActiveValue::Set(request.valid_countries),
        valid_after_x_products: ActiveValue::Set(request.valid_after_x_products),
        valid_after_x_total: ActiveValue::Set(request.valid_after_x_total),
        auto_apply: ActiveValue::Set(request.auto_apply),
        expire_at: ActiveValue::Set(request.expire_at),
        created_at: ActiveValue::Unchanged(discount_model.created_at),
        updated_at: ActiveValue::Set(now),
    };

    discounts::Entity::update(updated_discount)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update discount: {}", e)))?;

    Ok(UpdateDiscountResponse {
        success: true,
        message: "Discount updated successfully".to_string(),
    })
}

#[server]
pub async fn admin_delete_discount(
    request: DeleteDiscountRequest,
) -> Result<DeleteDiscountResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(DeleteDiscountResponse {
            success: false,
            message: "Unauthorized".to_string(),
        });
    }
    let db = get_db().await;
    let res = discounts::Entity::delete_by_id(request.id)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    if res.rows_affected == 0 {
        Ok(DeleteDiscountResponse {
            success: false,
            message: "Discount not found".to_string(),
        })
    } else {
        Ok(DeleteDiscountResponse {
            success: true,
            message: "Discount deleted successfully".to_string(),
        })
    }
}

#[cfg(feature = "server")]
pub async fn check_discount(
    discount_code: String,
    country_code: Option<String>,
    discounts_data: Option<Vec<discounts::Model>>,
    basket_items_data: Vec<basket_items::Model>,
    product_variants_data: Vec<product_variants::Model>,
) -> Result<DiscountValidationResponse, DiscountValidationError> {
    use entity::sea_orm_active_enums::DiscountType;

    let discount = if let Some(discounts) = discounts_data {
        discounts
            .into_iter()
            .find(|d| d.code == discount_code)
            .ok_or(DiscountValidationError::DiscountNotFound)?
    } else {
        let db = get_db().await;
        discounts::Entity::find()
            .filter(discounts::Column::Code.eq(&discount_code))
            .one(db)
            .await
            .map_err(|_| DiscountValidationError::DatabaseError)?
            .ok_or(DiscountValidationError::DiscountNotFound)?
    };

    if !discount.active {
        return Err(DiscountValidationError::DiscountInactive);
    }

    let now = Utc::now().naive_utc();
    if let Some(expire_at) = discount.expire_at {
        if expire_at < now {
            return Err(DiscountValidationError::DiscountExpired);
        }
    }

    if let Some(max_uses) = discount.maximum_uses {
        if (discount.discount_used + discount.active_reduce_quantity) >= max_uses {
            return Err(DiscountValidationError::MaximumUsesExceeded);
        }
    }

    match discount.discount_type {
        DiscountType::FixedAmount | DiscountType::FixedAmountOnShipping => {
            if let (Some(discount_amount), Some(amount_used)) =
                (discount.discount_amount, discount.amount_used)
            {
                if amount_used >= discount_amount {
                    return Err(DiscountValidationError::AmountExceeded);
                }
            }
        }
        _ => {}
    }

    if let Some(valid_countries) = &discount.valid_countries {
        if !valid_countries.is_empty() {
            match country_code {
                None => {
                    return Err(DiscountValidationError::CountryRequired);
                }
                Some(country) => {
                    if !valid_countries.contains(&country) {
                        return Err(DiscountValidationError::InvalidCountry);
                    }
                }
            }
        }
    }

    if let Some(min_products) = discount.valid_after_x_products {
        let total_product_count: i32 = basket_items_data.iter().map(|item| item.quantity).sum();
        if total_product_count <= min_products {
            return Err(DiscountValidationError::MinimumProductsRequired);
        }
    }

    if let Some(min_total) = discount.valid_after_x_total {
        let total_cost: f64 = basket_items_data
            .iter()
            .filter_map(|basket_item| {
                product_variants_data
                    .iter()
                    .find(|variant| variant.id == basket_item.variant_id)
                    .map(|variant| basket_item.quantity as f64 * variant.price_standard_usd)
            })
            .sum();

        if total_cost < min_total {
            return Err(DiscountValidationError::MinimumTotalRequired);
        }
    }

    Ok(DiscountValidationResponse {
        discount,
        is_valid: true,
    })
}
