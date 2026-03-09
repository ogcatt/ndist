use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use super::super::db::get_db;
#[cfg(feature = "server")]
use super::auth::{check_admin_permission, get_current_user};
#[cfg(feature = "server")]
use entity::{api_keys, group_invite_codes, group_members, groups};
#[cfg(feature = "server")]
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait, Order,
};
#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use uuid::Uuid;

// ── Response types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub group_id: String,
    /// First 12 chars of the key followed by "..."
    pub key_preview: String,
    pub is_active: bool,
    pub created_at: chrono::NaiveDateTime,
    pub last_used_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub success: bool,
    pub message: String,
    /// Full plaintext key — shown only once
    pub plaintext_key: Option<String>,
    pub key_info: Option<ApiKeyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteCodeInfo {
    pub id: String,
    pub code: String,
    pub group_id: String,
    pub is_api_generated: bool,
    pub is_used: bool,
    pub used_by_email: Option<String>,
    pub used_at: Option<chrono::NaiveDateTime>,
    pub is_revoked: bool,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemCodeResponse {
    pub success: bool,
    pub message: String,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
}

// ── Code generation helper ─────────────────────────────────────────────────────

#[cfg(feature = "server")]
fn generate_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    (0..8)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

#[cfg(feature = "server")]
fn generate_api_key() -> String {
    // Format: sk_<uuid_no_hyphens><uuid_no_hyphens>
    let a = Uuid::new_v4().to_string().replace('-', "");
    let b = Uuid::new_v4().to_string().replace('-', "");
    format!("sk_{}{}", a, b)
}

// ── Admin: API Key management ──────────────────────────────────────────────────

#[server]
pub async fn admin_create_api_key(group_id: String, name: String) -> Result<CreateApiKeyResponse, ServerFnError> {
    let is_admin = check_admin_permission().await?;
    if !is_admin {
        return Err(ServerFnError::new("Unauthorized"));
    }
    let db = get_db().await;

    // Verify group exists
    let group = groups::Entity::find_by_id(&group_id)
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if group.is_none() {
        return Ok(CreateApiKeyResponse { success: false, message: "Group not found".to_string(), plaintext_key: None, key_info: None });
    }

    let plaintext = generate_api_key();
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();
    let key_preview = format!("{}...", &plaintext[..12]);

    let model = api_keys::ActiveModel {
        id: ActiveValue::Set(id.clone()),
        group_id: ActiveValue::Set(group_id.clone()),
        name: ActiveValue::Set(name.clone()),
        key_value: ActiveValue::Set(plaintext.clone()),
        is_active: ActiveValue::Set(true),
        created_at: ActiveValue::Set(now),
        last_used_at: ActiveValue::Set(None),
    };
    model.insert(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(CreateApiKeyResponse {
        success: true,
        message: "API key created. Save the key now — it will not be shown again.".to_string(),
        plaintext_key: Some(plaintext),
        key_info: Some(ApiKeyInfo { id, name, group_id, key_preview, is_active: true, created_at: now, last_used_at: None }),
    })
}

#[server]
pub async fn admin_get_api_keys(group_id: String) -> Result<Vec<ApiKeyInfo>, ServerFnError> {
    let is_admin = check_admin_permission().await?;
    if !is_admin { return Err(ServerFnError::new("Unauthorized")); }
    let db = get_db().await;

    let keys = api_keys::Entity::find()
        .filter(api_keys::Column::GroupId.eq(&group_id))
        .order_by(api_keys::Column::CreatedAt, Order::Desc)
        .all(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(keys.into_iter().map(|k| ApiKeyInfo {
        key_preview: format!("{}...", &k.key_value[..12.min(k.key_value.len())]),
        id: k.id,
        name: k.name,
        group_id: k.group_id,
        is_active: k.is_active,
        created_at: k.created_at,
        last_used_at: k.last_used_at,
    }).collect())
}

#[server]
pub async fn admin_revoke_api_key(key_id: String) -> Result<bool, ServerFnError> {
    let is_admin = check_admin_permission().await?;
    if !is_admin { return Err(ServerFnError::new("Unauthorized")); }
    let db = get_db().await;

    let key = api_keys::Entity::find_by_id(&key_id)
        .one(db).await.map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Key not found"))?;

    let mut active: api_keys::ActiveModel = key.into();
    active.is_active = ActiveValue::Set(false);
    active.update(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(true)
}

#[server]
pub async fn admin_delete_api_key(key_id: String) -> Result<bool, ServerFnError> {
    let is_admin = check_admin_permission().await?;
    if !is_admin { return Err(ServerFnError::new("Unauthorized")); }
    let db = get_db().await;

    api_keys::Entity::delete_by_id(&key_id)
        .exec(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(true)
}

// ── Admin: Invite code management ─────────────────────────────────────────────

#[server]
pub async fn admin_create_invite_code(group_id: String) -> Result<InviteCodeInfo, ServerFnError> {
    let is_admin = check_admin_permission().await?;
    if !is_admin { return Err(ServerFnError::new("Unauthorized")); }
    let db = get_db().await;

    // Generate unique code
    let code = loop {
        let candidate = generate_code();
        let exists = group_invite_codes::Entity::find()
            .filter(group_invite_codes::Column::Code.eq(&candidate))
            .one(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        if exists.is_none() { break candidate; }
    };

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    let model = group_invite_codes::ActiveModel {
        id: ActiveValue::Set(id.clone()),
        code: ActiveValue::Set(code.clone()),
        group_id: ActiveValue::Set(group_id.clone()),
        api_key_id: ActiveValue::Set(None),
        used_by_user_id: ActiveValue::Set(None),
        used_at: ActiveValue::Set(None),
        is_revoked: ActiveValue::Set(false),
        created_at: ActiveValue::Set(now),
    };
    model.insert(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(InviteCodeInfo {
        id,
        code,
        group_id,
        is_api_generated: false,
        is_used: false,
        used_by_email: None,
        used_at: None,
        is_revoked: false,
        created_at: now,
    })
}

#[server]
pub async fn admin_get_group_codes(group_id: String) -> Result<Vec<InviteCodeInfo>, ServerFnError> {
    let is_admin = check_admin_permission().await?;
    if !is_admin { return Err(ServerFnError::new("Unauthorized")); }
    let db = get_db().await;

    let codes = group_invite_codes::Entity::find()
        .filter(group_invite_codes::Column::GroupId.eq(&group_id))
        .order_by(group_invite_codes::Column::CreatedAt, Order::Desc)
        .all(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    // Fetch used-by emails for used codes
    use entity::users;
    let mut infos = Vec::new();
    for c in codes {
        let used_by_email = if let Some(ref uid) = c.used_by_user_id {
            users::Entity::find_by_id(uid)
                .one(db).await.ok().flatten()
                .map(|u| u.email)
        } else {
            None
        };
        infos.push(InviteCodeInfo {
            is_api_generated: c.api_key_id.is_some(),
            is_used: c.used_by_user_id.is_some(),
            used_by_email,
            id: c.id,
            code: c.code,
            group_id: c.group_id,
            used_at: c.used_at,
            is_revoked: c.is_revoked,
            created_at: c.created_at,
        });
    }
    Ok(infos)
}

#[server]
pub async fn admin_revoke_invite_code(code_id: String) -> Result<bool, ServerFnError> {
    let is_admin = check_admin_permission().await?;
    if !is_admin { return Err(ServerFnError::new("Unauthorized")); }
    let db = get_db().await;

    let code = group_invite_codes::Entity::find_by_id(&code_id)
        .one(db).await.map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Code not found"))?;

    let mut active: group_invite_codes::ActiveModel = code.into();
    active.is_revoked = ActiveValue::Set(true);
    active.update(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(true)
}

#[server]
pub async fn admin_delete_invite_code(code_id: String) -> Result<bool, ServerFnError> {
    let is_admin = check_admin_permission().await?;
    if !is_admin { return Err(ServerFnError::new("Unauthorized")); }
    let db = get_db().await;

    group_invite_codes::Entity::delete_by_id(&code_id)
        .exec(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(true)
}

// ── User: Redeem invite code ───────────────────────────────────────────────────

#[server]
pub async fn redeem_invite_code(code: String) -> Result<RedeemCodeResponse, ServerFnError> {
    let user = get_current_user().await?
        .ok_or_else(|| ServerFnError::new("You must be signed in to redeem a code."))?;
    let db = get_db().await;

    let code_upper = code.trim().to_uppercase();

    let invite = group_invite_codes::Entity::find()
        .filter(group_invite_codes::Column::Code.eq(&code_upper))
        .one(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    let invite = match invite {
        Some(c) => c,
        None => return Ok(RedeemCodeResponse { success: false, message: "Invalid code.".to_string(), group_id: None, group_name: None }),
    };

    if invite.is_revoked {
        return Ok(RedeemCodeResponse { success: false, message: "This code has been revoked.".to_string(), group_id: None, group_name: None });
    }
    if invite.used_by_user_id.is_some() {
        return Ok(RedeemCodeResponse { success: false, message: "This code has already been used.".to_string(), group_id: None, group_name: None });
    }

    // Check if user is already in the group
    let already_member = group_members::Entity::find()
        .filter(group_members::Column::GroupId.eq(&invite.group_id))
        .filter(group_members::Column::UserId.eq(&user.id))
        .one(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    // Get group name
    let group = groups::Entity::find_by_id(&invite.group_id)
        .one(db).await.map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Group no longer exists."))?;

    if already_member.is_some() {
        // Mark code used anyway, return success
        let mut active: group_invite_codes::ActiveModel = invite.into();
        active.used_by_user_id = ActiveValue::Set(Some(user.id));
        active.used_at = ActiveValue::Set(Some(Utc::now().naive_utc()));
        active.update(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        return Ok(RedeemCodeResponse {
            success: true,
            message: format!("You joined {}. You now have access to products in this group.", group.name),
            group_id: Some(group.id),
            group_name: Some(group.name),
        });
    }

    // Add user to group
    let member_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();
    let member = group_members::ActiveModel {
        id: ActiveValue::Set(member_id),
        group_id: ActiveValue::Set(invite.group_id.clone()),
        user_id: ActiveValue::Set(user.id.clone()),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };
    member.insert(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    // Mark code as used
    let group_id = invite.group_id.clone();
    let group_name = group.name.clone();
    let mut active: group_invite_codes::ActiveModel = invite.into();
    active.used_by_user_id = ActiveValue::Set(Some(user.id));
    active.used_at = ActiveValue::Set(Some(now));
    active.update(db).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(RedeemCodeResponse {
        success: true,
        message: format!("You joined {}. You now have access to products in this group.", group_name),
        group_id: Some(group_id),
        group_name: Some(group_name),
    })
}

// ── External REST API helper (server-only, not a server fn) ───────────────────

/// Called by the Axum REST handler. Validates API key, enforces rate limit, generates code.
#[cfg(feature = "server")]
pub async fn api_generate_invite_code(raw_key: String, group_id: String) -> Result<String, String> {
    let db = get_db().await;

    // Look up the API key
    let key_record = api_keys::Entity::find()
        .filter(api_keys::Column::KeyValue.eq(&raw_key))
        .filter(api_keys::Column::IsActive.eq(true))
        .one(db).await.map_err(|e| e.to_string())?;

    let key_record = match key_record {
        Some(k) => k,
        None => return Err("Invalid or inactive API key.".to_string()),
    };

    // Verify the key is scoped to the requested group
    if key_record.group_id != group_id {
        return Err("This API key is not authorized for the specified group.".to_string());
    }

    // Rate limit: count codes generated by this key today
    let today_start = {
        let now = Utc::now().naive_utc();
        now.date().and_hms_opt(0, 0, 0).unwrap()
    };
    let today_count = group_invite_codes::Entity::find()
        .filter(group_invite_codes::Column::ApiKeyId.eq(&key_record.id))
        .filter(group_invite_codes::Column::CreatedAt.gt(today_start))
        .all(db).await.map_err(|e| e.to_string())?.len();

    if today_count >= 50 {
        return Err("Daily rate limit of 50 codes reached for this API key.".to_string());
    }

    // Generate unique code
    let code = loop {
        let candidate = generate_code();
        let exists = group_invite_codes::Entity::find()
            .filter(group_invite_codes::Column::Code.eq(&candidate))
            .one(db).await.map_err(|e| e.to_string())?;
        if exists.is_none() { break candidate; }
    };

    let now = Utc::now().naive_utc();
    let model = group_invite_codes::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4().to_string()),
        code: ActiveValue::Set(code.clone()),
        group_id: ActiveValue::Set(group_id),
        api_key_id: ActiveValue::Set(Some(key_record.id.clone())),
        used_by_user_id: ActiveValue::Set(None),
        used_at: ActiveValue::Set(None),
        is_revoked: ActiveValue::Set(false),
        created_at: ActiveValue::Set(now),
    };
    model.insert(db).await.map_err(|e| e.to_string())?;

    // Update last_used_at on the key
    let mut active_key: api_keys::ActiveModel = key_record.into();
    active_key.last_used_at = ActiveValue::Set(Some(now));
    active_key.update(db).await.map_err(|e| e.to_string())?;

    Ok(code)
}
