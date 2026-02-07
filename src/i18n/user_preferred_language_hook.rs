use super::language::Language;
use crate::i18n::consts::STORAGE_LANGUAGE;
use dioxus::prelude::*;
use dioxus_i18n::prelude::*;
use dioxus_sdk::storage::*;
use std::str::FromStr;
use web_sys::{Storage, window};

// Custom storage wrapper that handles quota exceeded errors
struct SafeStorage;

impl SafeStorage {
    fn get_local_storage() -> Result<Storage, String> {
        window()
            .ok_or_else(|| "Window not available".to_string())?
            .local_storage()
            .map_err(|_| "Cannot access localStorage".to_string())?
            .ok_or_else(|| "localStorage not available".to_string())
    }

    fn get_item(key: &str) -> Option<String> {
        match Self::get_local_storage() {
            Ok(storage) => storage.get_item(key).ok().flatten(),
            Err(e) => {
                tracing::warn!("Failed to access localStorage for reading: {}", e);
                None
            }
        }
    }

    fn set_item(key: &str, value: &str) -> Result<(), String> {
        let storage = Self::get_local_storage()?;

        match storage.set_item(key, value) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Check if it's a quota exceeded error
                let error_string = format!("{:?}", e);
                if error_string.contains("QuotaExceeded") || error_string.contains("quota") {
                    tracing::warn!("localStorage quota exceeded. Clearing storage and retrying...");

                    // Clear localStorage and try again
                    if let Err(clear_err) = storage.clear() {
                        tracing::warn!("Failed to clear localStorage: {:?}", clear_err);
                        return Err("Storage quota exceeded and failed to clear".to_string());
                    }

                    tracing::info!("localStorage cleared successfully. Retrying save...");

                    // Try to save again after clearing
                    storage
                        .set_item(key, value)
                        .map_err(|_| "Failed to save even after clearing storage".to_string())
                } else {
                    Err(format!("Storage error: {:?}", e))
                }
            }
        }
    }

    fn remove_item(key: &str) -> Result<(), String> {
        let storage = Self::get_local_storage()?;
        storage
            .remove_item(key)
            .map_err(|_| "Failed to remove item".to_string())
    }

    // Get storage usage information (for debugging)
    fn get_storage_info() -> String {
        match Self::get_local_storage() {
            Ok(storage) => {
                let mut total_size = 0;
                let mut items = Vec::new();

                if let Ok(length) = storage.length() {
                    for i in 0..length {
                        if let Ok(Some(key)) = storage.key(i) {
                            if let Ok(Some(value)) = storage.get_item(&key) {
                                let size = key.len() + value.len();
                                total_size += size;
                                items.push((key, size));
                            }
                        }
                    }
                }

                items.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by size, largest first
                let top_items: Vec<String> = items
                    .iter()
                    .take(5)
                    .map(|(key, size)| format!("{}: {}B", key, size))
                    .collect();

                format!(
                    "Total: {}B, Top items: [{}]",
                    total_size,
                    top_items.join(", ")
                )
            }
            Err(_) => "Cannot access localStorage".to_string(),
        }
    }
}

pub fn use_user_preferred_language() -> Signal<Option<Language>> {
    const FALLBACK_LANGUAGE: &str = "en-US";

    // Get browser language once
    let document_language = use_resource(|| async {
        let mut eval = document::eval("dioxus.send(navigator.language)");
        let lang = eval
            .recv::<String>()
            .await
            .unwrap_or_else(|_| String::from(FALLBACK_LANGUAGE));
        Language::from_str(lang.as_str())
            .unwrap_or_else(|_| Language::from_str(FALLBACK_LANGUAGE).expect("valid fallback"))
    });

    // Signals
    let mut preferred_language = use_signal(|| None::<Language>);
    let mut show_language_popup = use_signal(|| false);
    let mut has_user_preference = use_signal(|| false);

    // Single effect that handles everything in the right order
    use_effect(move || {
        // Step 1: Check storage for existing preference
        let stored_preference = SafeStorage::get_item(STORAGE_LANGUAGE);

        if let Some(stored_value) = stored_preference {
            // User has a stored preference - use it
            if let Ok(lang) = Language::from_str(&stored_value) {
                preferred_language.set(Some(lang));
                has_user_preference.set(true);
                show_language_popup.set(false);
                tracing::info!("Loaded existing language preference: {}", stored_value);
                return; // Exit early - no popup needed
            }
        }

        // Step 2: No stored preference - this is a first visit
        has_user_preference.set(false);

        // Step 3: Set fallback language for immediate i18n functionality
        if let Ok(fallback) = Language::from_str(FALLBACK_LANGUAGE) {
            preferred_language.set(Some(fallback));
        }

        // Step 4: Show popup for first-time users
        show_language_popup.set(true);
        tracing::info!("First visit detected - showing language popup");
    });

    // Effect for i18n updates
    let mut i18n = use_context::<I18n>();
    use_effect(move || {
        if let Some(lang) = preferred_language.read().as_ref() {
            i18n.set_language(lang.identifier());
        }
    });

    // Provide context
    provide_context(show_language_popup);
    provide_context(preferred_language.clone());
    preferred_language
}

// Updated language setter that uses safe storage
pub fn set_user_language(language: Language) -> Result<(), String> {
    // Use the language's identifier method instead of to_string()
    let language_str = language.identifier();
    match SafeStorage::set_item(STORAGE_LANGUAGE, &language_str.to_string()) {
        Ok(_) => {
            tracing::info!("Successfully saved language preference: {}", language_str);
            Ok(())
        }
        Err(e) => {
            tracing::warn!("Failed to save language preference: {}", e);
            Err(e)
        }
    }
}

// Hook for components to access language popup state
pub fn use_language_popup() -> Signal<bool> {
    use_context::<Signal<bool>>()
}

// Hook for components to set language (with safe storage)
pub fn use_language_setter() -> Signal<Option<Language>> {
    use_context::<Signal<Option<Language>>>()
}

// Utility function to clear all storage (for debugging/emergency)
pub fn clear_all_storage() -> Result<(), String> {
    let storage = SafeStorage::get_local_storage()?;
    storage
        .clear()
        .map_err(|_| "Failed to clear storage".to_string())?;
    tracing::info!("Manually cleared all localStorage");
    Ok(())
}

// Utility function to get storage debug info
pub fn get_storage_debug_info() -> String {
    SafeStorage::get_storage_info()
}
