// src/backend/server_functions/blog.rs

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use super::super::db::get_db;

#[cfg(feature = "server")]
use super::super::entity_conversions;

#[cfg(feature = "server")]
use entity::blog_posts;

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
pub struct CreateBlogPostRequest {
    pub title: String,
    pub subtitle: Option<String>,
    pub thumbnail_url: Option<String>,
    pub blog_md: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlogPostResponse {
    pub success: bool,
    pub message: String,
    pub blog_post_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditBlogPostRequest {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub thumbnail_url: Option<String>,
    pub blog_md: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditBlogPostResponse {
    pub success: bool,
    pub message: String,
}

#[server]
pub async fn admin_get_blog_post(id: String) -> Result<BlogPost, ServerFnError> {
    let db = get_db().await;

    let blog_post_model: blog_posts::Model = blog_posts::Entity::find()
        .filter(blog_posts::Column::Id.eq(&id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .expect("Could not get blog post model when fetching singleton");

    let blog_post_final = entity_conversions::convert_blog_post(blog_post_model, false)?;

    Ok(blog_post_final)
}

#[server]
pub async fn admin_get_blog_posts() -> Result<Vec<BlogPost>, ServerFnError> {
    let db = get_db().await;

    let blog_post_models: Vec<blog_posts::Model> =
        blog_posts::Entity::find().all(db).await.map_db_err()?;
    let blog_posts_final =
        entity_conversions::convert_blog_posts_batch(blog_post_models, false)?;

    Ok(blog_posts_final)
}

#[server]
pub async fn get_blog_posts() -> Result<Vec<BlogPost>, ServerFnError> {
    let db = get_db().await;

    let blog_post_models: Vec<blog_posts::Model> =
        blog_posts::Entity::find().all(db).await.map_db_err()?;
    let blog_posts_final = entity_conversions::convert_blog_posts_batch(blog_post_models, true)?;

    Ok(blog_posts_final)
}

#[server]
pub async fn admin_create_blog_post(
    request: CreateBlogPostRequest,
) -> Result<CreateBlogPostResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(CreateBlogPostResponse {
            success: false,
            message: "Unauthorized".to_string(),
            blog_post_id: None,
        });
    }

    if request.title.trim().is_empty() {
        return Ok(CreateBlogPostResponse {
            success: false,
            message: "Blog title is required".to_string(),
            blog_post_id: None,
        });
    }

    if request.blog_md.trim().is_empty() {
        return Ok(CreateBlogPostResponse {
            success: false,
            message: "Blog content is required".to_string(),
            blog_post_id: None,
        });
    }

    if let Some(ref subtitle) = request.subtitle {
        if subtitle.trim().is_empty() {
            return Ok(CreateBlogPostResponse {
                success: false,
                message: "Subtitle cannot be empty (leave blank if not needed)".to_string(),
                blog_post_id: None,
            });
        }
        if subtitle.len() > 200 {
            return Ok(CreateBlogPostResponse {
                success: false,
                message: "Subtitle cannot exceed 200 characters".to_string(),
                blog_post_id: None,
            });
        }
    }

    if let Some(ref thumbnail) = request.thumbnail_url {
        if thumbnail.trim().is_empty() {
            return Ok(CreateBlogPostResponse {
                success: false,
                message: "Thumbnail URL cannot be empty (leave blank if not needed)".to_string(),
                blog_post_id: None,
            });
        }
        if !thumbnail.starts_with("http://") && !thumbnail.starts_with("https://") {
            return Ok(CreateBlogPostResponse {
                success: false,
                message: "Thumbnail URL must start with http:// or https://".to_string(),
                blog_post_id: None,
            });
        }
    }

    let db = get_db().await;

    let existing_post = blog_posts::Entity::find()
        .filter(blog_posts::Column::Title.eq(&request.title))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if existing_post.is_some() {
        return Ok(CreateBlogPostResponse {
            success: false,
            message: "A blog post with this title already exists".to_string(),
            blog_post_id: None,
        });
    }

    let blog_post_id = Uuid::new_v4().to_string();
    let now = Utc::now().naive_utc();

    let blog_post = blog_posts::ActiveModel {
        id: ActiveValue::Set(blog_post_id.clone()),
        title: ActiveValue::Set(request.title),
        subtitle: ActiveValue::Set(request.subtitle),
        thumbnail_url: ActiveValue::Set(request.thumbnail_url),
        blog_md: ActiveValue::Set(request.blog_md),
        posted_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
    };

    blog_posts::Entity::insert(blog_post)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create blog post: {}", e)))?;

    Ok(CreateBlogPostResponse {
        success: true,
        message: "Blog post created successfully".to_string(),
        blog_post_id: Some(blog_post_id),
    })
}

#[server]
pub async fn admin_edit_blog_post(
    request: EditBlogPostRequest,
) -> Result<EditBlogPostResponse, ServerFnError> {
    let user = get_current_user().await?;
    if user.is_none() || !check_admin_permission().await? {
        return Ok(EditBlogPostResponse {
            success: false,
            message: "Unauthorized".to_string(),
        });
    }

    if request.title.trim().is_empty() {
        return Ok(EditBlogPostResponse {
            success: false,
            message: "Blog title is required".to_string(),
        });
    }

    if request.blog_md.trim().is_empty() {
        return Ok(EditBlogPostResponse {
            success: false,
            message: "Blog content is required".to_string(),
        });
    }

    if let Some(ref subtitle) = request.subtitle {
        if subtitle.trim().is_empty() {
            return Ok(EditBlogPostResponse {
                success: false,
                message: "Subtitle cannot be empty (leave blank if not needed)".to_string(),
            });
        }
        if subtitle.len() > 200 {
            return Ok(EditBlogPostResponse {
                success: false,
                message: "Subtitle cannot exceed 200 characters".to_string(),
            });
        }
    }

    if let Some(ref thumbnail) = request.thumbnail_url {
        if thumbnail.trim().is_empty() {
            return Ok(EditBlogPostResponse {
                success: false,
                message: "Thumbnail URL cannot be empty (leave blank if not needed)".to_string(),
            });
        }
    }

    let db = get_db().await;

    let existing_post = blog_posts::Entity::find()
        .filter(blog_posts::Column::Id.eq(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let existing_post = match existing_post {
        Some(post) => post,
        None => {
            return Ok(EditBlogPostResponse {
                success: false,
                message: "Blog post not found".to_string(),
            });
        }
    };

    let title_conflict = blog_posts::Entity::find()
        .filter(blog_posts::Column::Title.eq(&request.title))
        .filter(blog_posts::Column::Id.ne(&request.id))
        .one(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    if title_conflict.is_some() {
        return Ok(EditBlogPostResponse {
            success: false,
            message: "Another blog post with this title already exists".to_string(),
        });
    }

    let now = Utc::now().naive_utc();

    let mut blog_post: blog_posts::ActiveModel = existing_post.into();
    blog_post.title = ActiveValue::Set(request.title);
    blog_post.subtitle = ActiveValue::Set(request.subtitle);
    blog_post.thumbnail_url = ActiveValue::Set(request.thumbnail_url);
    blog_post.blog_md = ActiveValue::Set(request.blog_md);
    blog_post.updated_at = ActiveValue::Set(now);

    blog_posts::Entity::update(blog_post)
        .exec(db)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to update blog post: {}", e)))?;

    Ok(EditBlogPostResponse {
        success: true,
        message: "Blog post updated successfully".to_string(),
    })
}
