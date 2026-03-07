// src/backend/server_functions/settings.rs

use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use entity::store_settings;

#[cfg(feature = "server")]
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, IntoActiveModel};

#[cfg(feature = "server")]
use chrono::Utc;

use super::auth::check_admin_permission;
use super::super::front_entities::StoreSettingsInfo;

#[server]
pub async fn get_store_settings() -> Result<StoreSettingsInfo, ServerFnError> {
    let db = get_db().await;

    let row = store_settings::Entity::find_by_id("singleton")
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    match row {
        Some(r) => Ok(StoreSettingsInfo {
            lock_store: r.lock_store,
            lock_comment: r.lock_comment,
        }),
        None => Ok(StoreSettingsInfo {
            lock_store: false,
            lock_comment: None,
        }),
    }
}

#[server]
pub async fn admin_update_store_settings(
    lock_store: bool,
    lock_comment: String,
) -> Result<(), ServerFnError> {
    check_admin_permission().await?;
    let db = get_db().await;

    let lock_comment_val = if lock_comment.trim().is_empty() {
        None
    } else {
        Some(lock_comment.trim().to_string())
    };

    // Try to find existing row
    let existing = store_settings::Entity::find_by_id("singleton")
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if let Some(row) = existing {
        let mut active = row.into_active_model();
        active.lock_store = ActiveValue::Set(lock_store);
        active.lock_comment = ActiveValue::Set(lock_comment_val);
        active.updated_at = ActiveValue::Set(Utc::now().naive_utc());
        active
            .update(db)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    } else {
        let model = store_settings::ActiveModel {
            id: ActiveValue::Set("singleton".to_string()),
            lock_store: ActiveValue::Set(lock_store),
            lock_comment: ActiveValue::Set(lock_comment_val),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
        };
        model
            .insert(db)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    }

    Ok(())
}
