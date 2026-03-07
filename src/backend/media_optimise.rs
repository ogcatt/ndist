#[cfg(feature = "server")]
use image::ImageFormat;
#[cfg(feature = "server")]
use ravif::{Encoder, Img};
#[cfg(feature = "server")]
use rgb::RGBA8;
#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;

#[cfg(feature = "server")]
pub async fn convert_image_to_webp(
    file_data: Vec<u8>,
    content_type: &str,
) -> Result<Vec<u8>, ServerFnError> {
    use std::io::Cursor;

    if content_type == "image/webp" {
        return Ok(file_data);
    }

    let img = image::load_from_memory(&file_data)
        .map_err(|e| ServerFnError::new(format!("Failed to load image: {}", e)))?;

    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::WebP)
        .map_err(|e| ServerFnError::new(format!("Failed to encode WebP: {}", e)))?;

    Ok(buf)
}

#[cfg(feature = "server")]
pub async fn convert_image_to_avif(
    file_data: Vec<u8>,
    content_type: &str,
) -> Result<(Vec<u8>, String), ServerFnError> {
    // Skip conversion if already AVIF
    if content_type == "image/avif" {
        return Ok((file_data, content_type.to_string()));
    }

    // Load the image from bytes
    let img = image::load_from_memory(&file_data)
        .map_err(|e| ServerFnError::new(format!("Failed to load image: {}", e)))?;

    // Convert to RGBA8 format for AVIF encoding
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();

    // Convert to the format expected by ravif
    let pixels: Vec<RGBA8> = rgba_img
        .pixels()
        .map(|p| RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();

    let avif_img = Img::new(&pixels[..], width as usize, height as usize);

    // Configure AVIF encoder with good quality settings
    let encoder = Encoder::new()
        .with_quality(88.0) // High quality
        .with_speed(6); // Reasonable encoding speed

    // Encode to AVIF
    let avif_data = encoder
        .encode_rgba(avif_img)
        .map_err(|e| ServerFnError::new(format!("Failed to encode AVIF: {}", e)))?;

    Ok((avif_data.avif_file, "image/avif".to_string()))
}
