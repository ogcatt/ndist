use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use {
    chrono::{Duration, Utc},
    entity::{user_sessions, users},
    sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter},
    supabase_auth::models::AuthClient,
    uuid::Uuid,
    axum::{
        extract::{Request, State},
        http::{header::COOKIE, HeaderMap, StatusCode},
        middleware::Next,
        response::{IntoResponse, Redirect, Response},
    },
    std::collections::HashMap,
    jsonwebtoken::{decode, DecodingKey, Validation, Algorithm},
    base64::{Engine as _, engine::general_purpose},
    moka::future::Cache,
    std::time::Duration as StdDuration,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub admin: bool,
    pub authenticated: bool,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: String::new(),
            email: String::new(),
            name: String::new(),
            admin: false,
            authenticated: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseTokenClaims {
    pub sub: String,
    pub email: Option<String>,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub aud: String,
}

#[cfg(feature = "server")]
#[derive(Clone)]
pub struct AppState {
    pub auth_service: std::sync::Arc<AuthService>,
    pub session_cache: Cache<String, User>,
    pub negative_cache: Cache<String, ()>, // Cache for invalid tokens
}

#[cfg(feature = "server")]
pub struct AuthService {
    pub supabase_client: AuthClient,
    pub db: DatabaseConnection,
    pub jwt_secret: String,
}

#[cfg(feature = "server")]
impl AuthService {
    pub fn new(db: DatabaseConnection) -> Result<Self, anyhow::Error> {
        // Read environment variables manually
        let project_url = std::env::var("SUPABASE_URL")
            .map_err(|_| anyhow::anyhow!("SUPABASE_URL not found"))?;
        let api_key = std::env::var("SUPABASE_ANON_KEY")
            .map_err(|_| anyhow::anyhow!("SUPABASE_ANON_KEY not found"))?;
        let jwt_secret = std::env::var("SUPABASE_JWT_SECRET")
            .map_err(|_| anyhow::anyhow!("SUPABASE_JWT_SECRET not found"))?;

        let supabase_client = AuthClient::new(&project_url, &api_key, &jwt_secret);

        Ok(Self {
            supabase_client,
            db,
            jwt_secret,
        })
    }

    pub async fn send_magic_link(&self, email: &str) -> Result<(), anyhow::Error> {
        // Check if user exists
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .one(&self.db)
            .await?;

        if user.is_none() {
            return Err(anyhow::anyhow!("User not found"));
        }

        self.supabase_client.send_login_email_with_magic_link(email).await?;
        Ok(())
    }

    pub async fn verify_and_create_session(&self, access_token: &str) -> Result<String, anyhow::Error> {
        // Verify the JWT token from Supabase
        let email = self.verify_supabase_token(access_token)?;

        // Find the user by email
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(&email))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        // Create a new session
        let session_token = Uuid::new_v4().to_string();
        let now = Utc::now().naive_utc();
        let expires_at = now + Duration::days(120);

        let session = user_sessions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4().to_string()),
            user_id: ActiveValue::Set(user.id.clone()),
            token: ActiveValue::Set(session_token.clone()),
            expires_at: ActiveValue::Set(expires_at),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        };

        user_sessions::Entity::insert(session)
            .exec(&self.db)
            .await?;

        Ok(session_token)
    }

    pub async fn validate_session(&self, token: &str) -> Result<User, anyhow::Error> {
        let now = Utc::now().naive_utc();

        // Find valid session
        let session = user_sessions::Entity::find()
            .filter(user_sessions::Column::Token.eq(token))
            .filter(user_sessions::Column::ExpiresAt.gt(now))
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Invalid or expired session"))?;

        // Get the user
        let user = users::Entity::find_by_id(&session.user_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        Ok(User {
            id: user.id,
            email: user.email,
            name: user.name,
            admin: user.admin,
            authenticated: true,
        })
    }

    pub async fn validate_session_with_negative_cache(
        &self,
        token: &str,
        session_cache: &Cache<String, User>,
        negative_cache: &Cache<String, ()>
    ) -> Result<User, anyhow::Error> {
        // Check if token is in negative cache (known to be invalid)
        if negative_cache.get(token).await.is_some() {
            tracing::info!("Token found in negative cache: {}", &token[..8]);
            return Err(anyhow::anyhow!("Token is cached as invalid"));
        }

        // Check positive cache
        if let Some(cached_user) = session_cache.get(token).await {
            tracing::info!("Session found in positive cache for token: {}", &token[..8]);
            return Ok(cached_user);
        }

        tracing::info!("Session not in cache, validating from database for token: {}", &token[..8]);

        // Validate from database
        match self.validate_session(token).await {
            Ok(user) => {
                // Cache successful validation
                session_cache.insert(token.to_string(), user.clone()).await;
                tracing::info!("Session cached successfully for token: {}", &token[..8]);
                Ok(user)
            }
            Err(e) => {
                // Cache failed validation for a short time to avoid repeated DB hits
                negative_cache.insert(token.to_string(), ()).await;
                tracing::info!("Session validation failed, added to negative cache: {}", &token[..8]);
                Err(e)
            }
        }
    }

    pub async fn logout_and_invalidate_cache(
        &self,
        token: &str,
        session_cache: &Cache<String, User>,
        negative_cache: &Cache<String, ()>
    ) -> Result<(), anyhow::Error> {
        // Remove from database
        user_sessions::Entity::delete_many()
            .filter(user_sessions::Column::Token.eq(token))
            .exec(&self.db)
            .await?;

        // Remove from both caches - use the token string directly
        session_cache.invalidate(token).await;
        negative_cache.invalidate(token).await;

        tracing::info!("Session invalidated for token: {}", &token[..8]);
        Ok(())
    }

    pub async fn logout(&self, token: &str) -> Result<(), anyhow::Error> {
        user_sessions::Entity::delete_many()
            .filter(user_sessions::Column::Token.eq(token))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    // Method to invalidate all sessions for a user
    pub async fn invalidate_user_sessions(
        &self,
        user_id: &str,
        session_cache: &Cache<String, User>,
        negative_cache: &Cache<String, ()>
    ) -> Result<(), anyhow::Error> {
        // Remove all sessions for this user from database
        user_sessions::Entity::delete_many()
            .filter(user_sessions::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;

        // Remove from cache (we need to iterate through cache entries)
        // This is less efficient but necessary for user-based invalidation
        session_cache.run_pending_tasks().await;
        let snapshot: Vec<_> = session_cache.iter().collect();
        for (token, user) in snapshot {
            if user.id == user_id {
                // Use as_ref() to get &str from Arc<String>
                session_cache.invalidate(token.as_ref()).await;
                negative_cache.invalidate(token.as_ref()).await;
            }
        }

        tracing::info!("All sessions invalidated for user: {}", user_id);
        Ok(())
    }

    fn verify_supabase_token(&self, token: &str) -> Result<String, anyhow::Error> {
        let key = DecodingKey::from_secret(self.jwt_secret.as_ref());
        let validation = Validation::new(Algorithm::HS256);

        let token_data = decode::<SupabaseTokenClaims>(token, &key, &validation)?;

        token_data.claims.email
            .ok_or_else(|| anyhow::anyhow!("Email not found in token"))
    }
}

#[cfg(feature = "server")]
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let uri = request.uri();
    let path = uri.path();

    // Skip auth for certain routes - account popup handles signin
    if path == "/" || path.starts_with("/api/auth") {
        return Ok(next.run(request).await);
    }

    // Check for session cookie
    let headers = request.headers();
    let cookies = parse_cookies(headers);

    let mut user_opt: Option<User> = None;

    // Check if this is an admin route - if so, read from DB directly (no cache)
    // This ensures admin status changes are reflected immediately
    // This covers server functions prefixed with admin, which at run time may be like /api/admin_get_orders2331231805942226593
    let is_admin_route = path.starts_with("/admin") || path.starts_with("/api/admin");

    if let Some(session_token) = cookies.get("session_token") {
        if is_admin_route {
            // For admin routes, always validate from database to get current admin status
            match state.auth_service.validate_session(session_token).await {
                Ok(user) => {
                    user_opt = Some(user);
                }
                Err(e) => {
                    tracing::info!("Session validation failed for admin route: {}", e);
                    // Invalidate from both caches
                    state.session_cache.invalidate(session_token).await;
                    state.negative_cache.insert(session_token.to_string(), ()).await;
                }
            }
        } else {
            // For non-admin routes, use cached validation
            match state.auth_service.validate_session_with_negative_cache(
                session_token,
                &state.session_cache,
                &state.negative_cache
            ).await {
                Ok(user) => {
                    user_opt = Some(user);
                }
                Err(e) => {
                    tracing::info!("Session validation failed: {}", e);
                    // Invalid session, ensure it's in negative cache
                    state.negative_cache.insert(session_token.to_string(), ()).await;
                }
            }
        }
    }

    // Check if this is an admin route that requires authentication
    if is_admin_route {
        match user_opt {
            Some(user) => {
                // Check if user has admin privileges for /admin routes
                if !user.admin {
                    tracing::info!("Non-admin user {} attempted to admin route: {}", user.email, path);
                    // For web requests, redirect to home; for API, return unauthorized
                    if path.starts_with("/api/admin") {
                        return Ok(StatusCode::UNAUTHORIZED.into_response());
                    }
                    return Ok(Redirect::to("/").into_response());
                }
                // Session is valid, add user to request extensions
                request.extensions_mut().insert(user);
                return Ok(next.run(request).await);
            }
            None => {
                // No valid session for admin route - redirect to account popup via home
                // For API routes, return unauthorized
                if path.starts_with("/api/admin") {
                    return Ok(StatusCode::UNAUTHORIZED.into_response());
                }
                return Ok(Redirect::to("/").into_response());
            }
        }
    }

    // For non-admin routes, proceed without requiring authentication
    if let Some(user) = user_opt {
        request.extensions_mut().insert(user);
    }
    Ok(next.run(request).await)
}

#[cfg(feature = "server")]
fn parse_cookies(headers: &HeaderMap) -> HashMap<String, String> {
    let mut cookies = HashMap::new();

    if let Some(cookie_header) = headers.get(COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let mut parts = cookie.trim().splitn(2, '=');
                if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                    cookies.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    cookies
}

// Helper function to extract authenticated user from request extensions
#[cfg(feature = "server")]
pub fn get_authenticated_user(request: &Request) -> Option<&User> {
    request.extensions().get::<User>()
}

#[cfg(feature = "server")]
pub async fn setup_app_state() -> Result<AppState, anyhow::Error> {
    use sea_orm::Database;

    let database_url = std::env::var("DATABASE_URL")?;
    let db = Database::connect(&database_url).await?;
    let auth_service = AuthService::new(db)?;

    // Positive cache for valid sessions
    let session_cache = Cache::builder()
        .max_capacity(10_000) // Maximum number of sessions to cache
        .time_to_live(StdDuration::from_secs(15 * 60)) // 15 minutes TTL
        .time_to_idle(StdDuration::from_secs(5 * 60))  // 5 minutes idle time
        .build();

    // Negative cache for invalid tokens (shorter TTL to prevent abuse)
    let negative_cache = Cache::builder()
        .max_capacity(5_000) // Smaller capacity for invalid tokens
        .time_to_live(StdDuration::from_secs(2 * 60)) // 2 minutes only
        .build();

    tracing::info!("Auth caches initialized - Session cache: 15min TTL, Negative cache: 2min TTL");

    Ok(AppState {
        auth_service: std::sync::Arc::new(auth_service),
        session_cache,
        negative_cache,
    })
}

// Additional utility functions for cache management
#[cfg(feature = "server")]
impl AppState {
    pub async fn get_cache_stats(&self) -> (u64, u64) {
        (
            self.session_cache.entry_count(),
            self.negative_cache.entry_count()
        )
    }

    pub async fn clear_all_caches(&self) {
        self.session_cache.invalidate_all();
        self.negative_cache.invalidate_all();
        tracing::info!("All caches cleared");
    }

    pub async fn logout_user(&self, token: &str) -> Result<(), anyhow::Error> {
        self.auth_service.logout_and_invalidate_cache(
            token,
            &self.session_cache,
            &self.negative_cache
        ).await
    }
}
