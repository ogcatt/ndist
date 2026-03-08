// src/backend/server_functions/auth.rs

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::auth::User as AuthUser;
type User = AuthUser;

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use super::super::email::EmailService;

#[cfg(feature = "server")]
use entity::{auth_tokens, user_sessions, users as entity_users, group_members};

#[cfg(feature = "server")]
use sea_orm::{IntoActiveModel, ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

#[cfg(feature = "server")]
use chrono::{Duration, Utc};

#[cfg(feature = "server")]
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

#[cfg(feature = "server")]
use uuid::Uuid;

#[cfg(feature = "server")]
use supabase_auth::models::AuthClient;

// Rate limiting cache for OTP and verification attempts
#[cfg(feature = "server")]
mod otp_rate_limit {
    use chrono::{Duration, Utc};
    use once_cell::sync::Lazy;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Debug, Clone)]
    struct RateLimitEntry {
        email: String,
        minute_count: u32,
        day_count: u32,
        last_send_time: Option<chrono::DateTime<Utc>>,
        verify_attempts: u32,
        last_verify_time: Option<chrono::DateTime<Utc>>,
    }

    static RATE_LIMIT_CACHE: Lazy<Mutex<HashMap<String, RateLimitEntry>>> =
        Lazy::new(|| Mutex::new(HashMap::new()));

    const MINUTE_LIMIT: u32 = 5;
    const DAY_LIMIT: u32 = 20;
    const VERIFY_LIMIT: u32 = 5;
    const SEND_COOLDOWN_SECS: i64 = 30;

    pub fn check_send_rate_limit(email: &str) -> Result<(), String> {
        let mut cache = RATE_LIMIT_CACHE.lock().unwrap();
        let now = Utc::now();

        let entry = cache
            .entry(email.to_string())
            .or_insert_with(|| RateLimitEntry {
                email: email.to_string(),
                minute_count: 0,
                day_count: 0,
                last_send_time: None,
                verify_attempts: 0,
                last_verify_time: None,
            });

        if let Some(last_time) = entry.last_send_time {
            if now.signed_duration_since(last_time).num_seconds() < SEND_COOLDOWN_SECS {
                let remaining =
                    SEND_COOLDOWN_SECS - now.signed_duration_since(last_time).num_seconds();
                return Err(format!(
                    "Please wait {} seconds before requesting a new code.",
                    remaining
                ));
            }
        }

        let one_minute_ago = now - Duration::minutes(1);
        if entry.last_send_time.map_or(false, |t| t > one_minute_ago) {
            entry.minute_count += 1;
        } else {
            entry.minute_count = 1;
        }

        let one_day_ago = now - Duration::days(1);
        if entry.last_send_time.map_or(false, |t| t > one_day_ago) {
            entry.day_count += 1;
        } else {
            entry.day_count = 1;
        }

        entry.last_send_time = Some(now);

        if entry.minute_count > MINUTE_LIMIT {
            return Err("Too many requests this minute. Please try again later.".to_string());
        }
        if entry.day_count > DAY_LIMIT {
            return Err("Daily limit reached. Please try again tomorrow.".to_string());
        }

        Ok(())
    }

    pub fn check_verify_rate_limit(email: &str) -> Result<(), String> {
        let mut cache = RATE_LIMIT_CACHE.lock().unwrap();
        let now = Utc::now();

        let entry = cache
            .entry(email.to_string())
            .or_insert_with(|| RateLimitEntry {
                email: email.to_string(),
                minute_count: 0,
                day_count: 0,
                last_send_time: None,
                verify_attempts: 0,
                last_verify_time: None,
            });

        let one_minute_ago = now - Duration::minutes(1);
        if entry.last_verify_time.map_or(false, |t| t > one_minute_ago) {
            entry.verify_attempts += 1;
        } else {
            entry.verify_attempts = 1;
        }

        entry.last_verify_time = Some(now);

        if entry.verify_attempts > VERIFY_LIMIT {
            return Err("Too many verification attempts. Please request a new code.".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub struct SupabaseTokenClaims {
    pub sub: String,
    pub email: Option<String>,
    pub exp: usize,
    pub iat: usize,
    pub iss: Option<String>,
    pub aud: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionInfo {
    pub authenticated: bool,
    pub user_id: String,
    pub email: String,
    pub name: String,
    pub admin: bool,
    pub group_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VerifyOtpResponse {
    pub session_token: Option<String>,
    pub is_new_user: bool,
    pub message: Option<String>,
}

#[cfg(feature = "server")]
fn generate_otp_code() -> String {
    let code: u32 = rand::random::<u32>() % 900000 + 100000;
    format!("{:06}", code)
}

#[server]
pub async fn send_magic_link(email: String) -> Result<AuthResponse, ServerFnError> {
    let db = get_db().await;

    let user = entity_users::Entity::find()
        .filter(entity_users::Column::Email.eq(&email))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if user.is_none() {
        return Ok(AuthResponse {
            success: false,
            message: "User not found. Please contact an administrator.".to_string(),
        });
    }

    let project_url = std::env::var("SUPABASE_URL")
        .map_err(|_| ServerFnError::new("SUPABASE_URL not found".to_string()))?;
    let api_key = std::env::var("SUPABASE_ANON_KEY")
        .map_err(|_| ServerFnError::new("SUPABASE_ANON_KEY not found".to_string()))?;
    let jwt_secret = std::env::var("SUPABASE_JWT_SECRET")
        .map_err(|_| ServerFnError::new("SUPABASE_JWT_SECRET not found".to_string()))?;

    let supabase_client = AuthClient::new(project_url, api_key, jwt_secret);

    match supabase_client
        .send_login_email_with_magic_link(&email)
        .await
    {
        Ok(_) => Ok(AuthResponse {
            success: true,
            message: "A sign in link has been sent to your email.".to_string(),
        }),
        Err(e) => Ok(AuthResponse {
            success: false,
            message: format!("Error sending magic link: {:?}", e),
        }),
    }
}

#[server]
pub async fn verify_magic_link(access_token: String) -> Result<AuthResponse, ServerFnError> {
    let db = get_db().await;

    let jwt_secret = std::env::var("SUPABASE_JWT_SECRET")
        .map_err(|_| ServerFnError::new("SUPABASE_JWT_SECRET not found".to_string()))?;

    let key = DecodingKey::from_secret(jwt_secret.as_ref());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["authenticated"]);

    let token_data = decode::<SupabaseTokenClaims>(&access_token, &key, &validation)
        .map_err(|e| ServerFnError::new(format!("Token verification failed: {}", e)))?;

    let email = token_data
        .claims
        .email
        .ok_or_else(|| ServerFnError::new("Email not found in token".to_string()))?;

    let user = entity_users::Entity::find()
        .filter(entity_users::Column::Email.eq(&email))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::new("User not found".to_string()))?;

    let session_token = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();
    let expires_at = now + Duration::days(7);

    let session = user_sessions::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4().to_string()),
        user_id: ActiveValue::Set(user.id.clone()),
        token: ActiveValue::Set(session_token.clone()),
        expires_at: ActiveValue::Set(expires_at),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    user_sessions::Entity::insert(session)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create session: {}", e)))?;

    #[cfg(feature = "server")]
    {
        use axum::http::HeaderValue;
        use dioxus::fullstack::FullstackContext;

        let cookie_value = format!(
            "session_token={}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=7776000",
            session_token
        );

        let server_ctx =
            FullstackContext::current().expect("Server context should be available");
        let header_value = HeaderValue::from_str(&cookie_value)
            .map_err(|e| ServerFnError::new(format!("Invalid cookie value: {}", e)))?;

        server_ctx.add_response_header(axum::http::header::SET_COOKIE, header_value);
    }

    Ok(AuthResponse {
        success: true,
        message: "Authentication successful".to_string(),
    })
}

#[server]
pub async fn send_otp(email: String) -> Result<AuthResponse, ServerFnError> {
    use regex::Regex;

    let db = get_db().await;
    let now = Utc::now();

    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .map_err(|_| ServerFnError::new("Invalid regex".to_string()))?;

    if !email_regex.is_match(&email) {
        return Ok(AuthResponse {
            success: false,
            message: "Email does not match email pattern (typed incorrectly).".to_string(),
        });
    }

    match otp_rate_limit::check_send_rate_limit(&email) {
        Ok(()) => {}
        Err(msg) => {
            return Ok(AuthResponse {
                success: false,
                message: msg,
            });
        }
    }

    let otp_code = generate_otp_code();

    let auth_token = auth_tokens::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4().to_string()),
        email: ActiveValue::Set(email.clone()),
        otp_code: ActiveValue::Set(otp_code.clone()),
        used: ActiveValue::Set(false),
        attempts: ActiveValue::Set(0),
        expires_at: ActiveValue::Set((now + Duration::minutes(20)).naive_utc()),
        created_at: ActiveValue::Set(now.naive_utc()),
    };

    auth_tokens::Entity::insert(auth_token)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let email_service = EmailService::new()
        .map_err(|e| ServerFnError::new(format!("Email service error: {}", e)))?;

    match email_service.send_otp(&email, &email, otp_code).await {
        Ok(()) => Ok(AuthResponse {
            success: true,
            message: "OTP sent to your email.".to_string(),
        }),
        Err(e) => {
            tracing::info!("Could not send email on server side: {e:?}");
            Ok(AuthResponse {
                success: false,
                message: format!("Could not send email. Error: {:?}", e),
            })
        }
    }
}

#[server]
pub async fn verify_otp(
    email: String,
    otp_code: String,
) -> Result<VerifyOtpResponse, ServerFnError> {
    match otp_rate_limit::check_verify_rate_limit(&email) {
        Ok(()) => {}
        Err(msg) => {
            return Ok(VerifyOtpResponse {
                session_token: None,
                is_new_user: false,
                message: Some(msg),
            });
        }
    }

    let db = get_db().await;
    let now = Utc::now();

    let otp_token = match auth_tokens::Entity::find()
        .filter(auth_tokens::Column::Email.eq(&email))
        .filter(auth_tokens::Column::Used.eq(false))
        .filter(auth_tokens::Column::ExpiresAt.gt(now.naive_utc()))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?
    {
        Some(token) => token,
        None => {
            return Ok(VerifyOtpResponse {
                session_token: None,
                is_new_user: false,
                message: Some("Invalid or expired OTP.".to_string()),
            });
        }
    };

    if otp_token.attempts >= 5 {
        return Ok(VerifyOtpResponse {
            session_token: None,
            is_new_user: false,
            message: Some("Too many verification attempts. Please request a new code.".to_string()),
        });
    }

    if otp_token.otp_code != otp_code {
        let new_attempts = otp_token.attempts + 1;
        let mut updated_otp = otp_token.into_active_model();
        updated_otp.attempts = ActiveValue::Set(new_attempts);
        auth_tokens::Entity::update(updated_otp)
            .exec(db)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        return Ok(VerifyOtpResponse {
            session_token: None,
            is_new_user: false,
            message: Some("Invalid OTP code.".to_string()),
        });
    }

    let mut updated_otp = otp_token.into_active_model();
    updated_otp.used = ActiveValue::Set(true);
    updated_otp.attempts = ActiveValue::Set(0);
    auth_tokens::Entity::update(updated_otp)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let existing_user = entity_users::Entity::find()
        .filter(entity_users::Column::Email.eq(&email))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let is_new_user = existing_user.is_none();
    let session_token = Uuid::new_v4().to_string();
    let expires_at = now + Duration::days(180);

    if let Some(user) = existing_user {
        let session = user_sessions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            user_id: ActiveValue::Set(user.id.clone()),
            token: ActiveValue::Set(session_token.clone()),
            expires_at: ActiveValue::Set(expires_at.naive_utc()),
            created_at: ActiveValue::Set(now.naive_utc()),
            updated_at: ActiveValue::Set(now.naive_utc()),
        };

        user_sessions::Entity::insert(session)
            .exec(db)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    } else {
        let new_user = entity_users::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            email: ActiveValue::Set(email.clone()),
            name: ActiveValue::Set(email.clone()),
            admin: ActiveValue::Set(false),
            created_at: ActiveValue::Set(now.naive_utc()),
            updated_at: ActiveValue::Set(now.naive_utc()),
        };

        let inserted_user = entity_users::Entity::insert(new_user)
            .exec_with_returning(db)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        let session = user_sessions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            user_id: ActiveValue::Set(inserted_user.id.clone()),
            token: ActiveValue::Set(session_token.clone()),
            expires_at: ActiveValue::Set(expires_at.naive_utc()),
            created_at: ActiveValue::Set(now.naive_utc()),
            updated_at: ActiveValue::Set(now.naive_utc()),
        };

        user_sessions::Entity::insert(session)
            .exec(db)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    }

    #[cfg(feature = "server")]
    {
        use axum::http::HeaderValue;
        use dioxus::fullstack::FullstackContext;

        let cookie_value = format!(
            "session_token={}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=15552000",
            session_token
        );

        let server_ctx =
            FullstackContext::current().expect("Server context should be available");
        let header_value = HeaderValue::from_str(&cookie_value)
            .map_err(|e| ServerFnError::new(format!("Invalid cookie value: {}", e)))?;

        server_ctx.add_response_header(axum::http::header::SET_COOKIE, header_value);
    }

    Ok(VerifyOtpResponse {
        session_token: Some(session_token),
        is_new_user,
        message: None,
    })
}

#[cfg(feature = "server")]
async fn extract_session_token_from_request() -> Result<Option<String>, ServerFnError> {
    use dioxus::fullstack::FullstackContext;

    let server_ctx = FullstackContext::current().expect("Server context should be available");
    let request_parts = server_ctx.parts_mut();

    let cookie_header = request_parts
        .headers
        .get("cookie")
        .or_else(|| request_parts.headers.get("Cookie"));

    if let Some(cookie_value) = cookie_header {
        let cookie_str = cookie_value
            .to_str()
            .map_err(|e| ServerFnError::new(format!("Invalid cookie header: {}", e)))?;

        for cookie in cookie_str.split(';') {
            let cookie = cookie.trim();
            if let Some((name, value)) = cookie.split_once('=') {
                if name.trim() == "session_token" {
                    return Ok(Some(value.trim().to_string()));
                }
            }
        }
    }

    Ok(None)
}

#[cfg(feature = "server")]
async fn validate_session_and_get_user(
    token: &str,
    db: &sea_orm::DatabaseConnection,
) -> Result<Option<User>, ServerFnError> {
    let now = Utc::now().naive_utc();

    let session = user_sessions::Entity::find()
        .filter(user_sessions::Column::Token.eq(token))
        .filter(user_sessions::Column::ExpiresAt.gt(now))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if let Some(session) = session {
        let user = entity_users::Entity::find_by_id(&session.user_id)
            .one(db)
            .await
            .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

        if let Some(user) = user {
            return Ok(Some(User {
                id: user.id,
                email: user.email,
                name: user.name,
                admin: user.admin,
                authenticated: true,
            }));
        }
    }

    Ok(None)
}

#[server]
pub async fn get_current_user() -> Result<Option<User>, ServerFnError> {
    let db = get_db().await;

    #[cfg(feature = "server")]
    {
        let session_token = extract_session_token_from_request().await?;

        if let Some(token) = session_token {
            return validate_session_and_get_user(&token, db).await;
        }
    }

    Ok(None)
}

#[server]
pub async fn check_auth() -> Result<bool, ServerFnError> {
    match get_current_user().await? {
        Some(user) => Ok(user.authenticated),
        None => Ok(false),
    }
}

#[server]
pub async fn check_admin_permission() -> Result<bool, ServerFnError> {
    match get_current_user().await? {
        Some(user) => Ok(user.admin),
        None => Ok(false),
    }
}

#[server]
pub async fn logout_user() -> Result<AuthResponse, ServerFnError> {
    let db = get_db().await;
    let session_token = extract_session_token_from_request().await?;

    if let Some(token) = session_token {
        user_sessions::Entity::delete_many()
            .filter(user_sessions::Column::Token.eq(&token))
            .exec(db)
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to delete session: {}", e)))?;
    }

    #[cfg(feature = "server")]
    {
        use axum::http::HeaderValue;
        use dioxus::fullstack::FullstackContext;

        let cookie_value = "session_token=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0";

        let server_ctx =
            FullstackContext::current().expect("Server context should be available");
        let header_value = HeaderValue::from_str(cookie_value)
            .map_err(|e| ServerFnError::new(format!("Invalid cookie value: {}", e)))?;

        server_ctx.add_response_header(axum::http::header::SET_COOKIE, header_value);
    }

    Ok(AuthResponse {
        success: true,
        message: "Logged out successfully".to_string(),
    })
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    logout_user().await?;
    Ok(())
}

#[server]
pub async fn get_session_info() -> Result<SessionInfo, ServerFnError> {
    match get_current_user().await {
        Ok(Some(user)) => {
            // Fetch user's group memberships
            let db = get_db().await;
            let group_memberships = group_members::Entity::find()
                .filter(group_members::Column::UserId.eq(&user.id))
                .all(db)
                .await
                .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

            let group_ids = group_memberships.into_iter().map(|gm| gm.group_id).collect();

            Ok(SessionInfo {
                authenticated: true,
                user_id: user.id,
                email: user.email,
                name: user.name,
                admin: user.admin,
                group_ids,
            })
        },
        _ => Ok(SessionInfo {
            authenticated: false,
            user_id: String::new(),
            email: String::new(),
            name: String::new(),
            admin: false,
            group_ids: Vec::new(),
        }),
    }
}

#[server]
pub async fn admin_get_user_by_id(user_id: String) -> Result<Option<User>, ServerFnError> {
    let db = get_db().await;

    let user = entity_users::Entity::find_by_id(&user_id)
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if let Some(user) = user {
        Ok(Some(User {
            id: user.id,
            email: user.email,
            name: user.name,
            admin: user.admin,
            authenticated: true,
        }))
    } else {
        Ok(None)
    }
}

#[server]
pub async fn admin_check_user_email_exists(email: String) -> Result<bool, ServerFnError> {
    let db = get_db().await;

    let user = entity_users::Entity::find()
        .filter(entity_users::Column::Email.eq(&email))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    Ok(user.is_some())
}

#[server]
pub async fn cleanup_expired_sessions() -> Result<u64, ServerFnError> {
    let db = get_db().await;
    let now = Utc::now().naive_utc();

    let result = user_sessions::Entity::delete_many()
        .filter(user_sessions::Column::ExpiresAt.lt(now))
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to delete expired sessions: {}", e)))?;

    Ok(result.rows_affected)
}

// Legacy functions for backward compatibility
#[server]
pub async fn get_current_manager() -> Result<Option<User>, ServerFnError> {
    get_current_user().await
}

#[server]
pub async fn logout_manager() -> Result<AuthResponse, ServerFnError> {
    logout_user().await
}

#[server]
pub async fn admin_get_manager_by_id(manager_id: String) -> Result<Option<User>, ServerFnError> {
    admin_get_user_by_id(manager_id).await
}

#[server]
pub async fn admin_check_manager_email_exists(email: String) -> Result<bool, ServerFnError> {
    admin_check_user_email_exists(email).await
}
