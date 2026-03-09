#[cfg(feature = "server")]
pub mod invite {
    use axum::{extract::Json, http::StatusCode, response::IntoResponse};
    use serde::{Deserialize, Serialize};
    use crate::backend::server_functions::invite_codes::api_generate_invite_code;

    #[derive(Deserialize)]
    pub struct GenerateRequest {
        pub api_key: String,
        pub group_id: String,
    }

    #[derive(Serialize)]
    pub struct GenerateResponse {
        pub code: String,
    }

    #[derive(Serialize)]
    pub struct ErrorResponse {
        pub error: String,
    }

    pub async fn generate_invite_code_handler(
        Json(body): Json<GenerateRequest>,
    ) -> impl IntoResponse {
        match api_generate_invite_code(body.api_key, body.group_id).await {
            Ok(code) => (StatusCode::OK, Json(serde_json::json!({ "code": code }))).into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
        }
    }
}
