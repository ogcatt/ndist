// src/backend/server_functions/uploads.rs

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use uuid::Uuid;

use super::auth::{check_admin_permission, get_current_user};

#[cfg(feature = "server")]
use super::super::media_optimise::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub success: bool,
    pub url: Option<String>,
    pub message: String,
}

#[server]
pub async fn admin_upload_thumbnails(
    file_data: Vec<u8>,
    file_name: String,
    content_type: String,
) -> Result<UploadResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(UploadResponse {
            success: false,
            url: None,
            message: "Unauthorized".to_string(),
        });
    }

    upload_image_locally(file_data, file_name, content_type).await
}

#[server]
pub async fn admin_upload_private_thumbnails(
    file_data: Vec<u8>,
    file_name: String,
    content_type: String,
) -> Result<UploadResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(UploadResponse {
            success: false,
            url: None,
            message: "Unauthorized".to_string(),
        });
    }

    upload_private_image_locally(file_data, file_name, content_type).await
}

#[cfg(feature = "server")]
async fn upload_image_locally(
    file_data: Vec<u8>,
    _file_name: String,
    content_type: String,
) -> Result<UploadResponse, ServerFnError> {
    use std::fs;
    use std::path::Path;

    let (processed_data, _final_content_type) =
        convert_image_to_avif(file_data, &content_type).await?;

    let random_name = Uuid::new_v4().simple().to_string();
    let unique_filename = format!("{}.avif", random_name);

    let upload_base = if std::env::var("RAILWAY_ENVIRONMENT").is_ok() {
        "/app/assets/uploads"
    } else {
        "assets/uploads"
    };

    let assets_dir = Path::new(upload_base).join("products");

    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir)
            .map_err(|e| ServerFnError::new(format!("Failed to create directory: {}", e)))?;
    }

    let file_path = assets_dir.join(&unique_filename);

    fs::write(&file_path, processed_data)
        .map_err(|e| ServerFnError::new(format!("Failed to write file: {}", e)))?;

    let public_url = format!("/uploads/products/{}", unique_filename);

    Ok(UploadResponse {
        success: true,
        url: Some(public_url),
        message: "Upload successful".to_string(),
    })
}

#[cfg(feature = "server")]
async fn upload_private_image_locally(
    file_data: Vec<u8>,
    _file_name: String,
    content_type: String,
) -> Result<UploadResponse, ServerFnError> {
    use std::fs;
    use std::path::Path;

    let (processed_data, _final_content_type) =
        convert_image_to_avif(file_data, &content_type).await?;

    let random_name = Uuid::new_v4().simple().to_string();
    let unique_filename = format!("{}.avif", random_name);

    let upload_base = if std::env::var("RAILWAY_ENVIRONMENT").is_ok() {
        "/app/assets/private/uploads"
    } else {
        "assets/private/uploads"
    };
    let assets_dir = Path::new(upload_base);

    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir)
            .map_err(|e| ServerFnError::new(format!("Failed to create directory: {}", e)))?;
    }

    let file_path = assets_dir.join(&unique_filename);

    fs::write(&file_path, processed_data)
        .map_err(|e| ServerFnError::new(format!("Failed to write file: {}", e)))?;

    let public_url = format!("/private/uploads/{}", unique_filename);

    Ok(UploadResponse {
        success: true,
        url: Some(public_url),
        message: "Upload successful".to_string(),
    })
}
