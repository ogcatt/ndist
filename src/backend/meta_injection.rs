use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;

use crate::backend::meta_cache;

/// Middleware to inject server-side meta tags into HTML responses
pub async fn inject_meta_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string();

    // Get the response from the next middleware/handler
    let response = next.run(request).await;

    // Only process HTML responses
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.contains("text/html") {
        return Ok(response);
    }

    // Get meta tags for this path
    let meta_tags = meta_cache::get_meta_tags_or_default(&path).await;

    // Convert response to bytes
    let (parts, body) = response.into_parts();
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Convert to string
    let mut html = match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Inject meta tags after the opening <head> tag
    // This replaces any client-side meta tags with server-side ones
    let meta_html = meta_tags.to_html();

    if let Some(head_pos) = html.find("<head>") {
        let insert_pos = head_pos + "<head>".len();
        html.insert_str(insert_pos, &meta_html);
    }

    // Create new response
    let new_body = Body::from(html);
    Ok(Response::from_parts(parts, new_body))
}
