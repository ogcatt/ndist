use chrono::Local;
use dioxus::prelude::*;
use std::time::Duration;

#[cfg(feature = "web")]
use js_sys::Date;
#[cfg(feature = "web")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "web")]
use web_sys::window;

#[cfg(feature = "server")]
use once_cell::sync::Lazy;
#[cfg(feature = "server")]
use std::collections::HashMap;
#[cfg(feature = "server")]
use std::sync::{Arc, RwLock};
#[cfg(feature = "server")]
use std::time::Instant;

#[cfg(feature = "web")]
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedData<T> {
    data: T,
    expires_at: f64, // unix-epoch in ms
}

/// Server-side cache entry (stores serialized data)
#[cfg(feature = "server")]
#[derive(Clone)]
struct ServerCacheEntry {
    data: String, // JSON serialized data
    expires_at: Instant,
}

/// Global server-side cache storage
#[cfg(feature = "server")]
type ServerCache = Arc<RwLock<HashMap<String, ServerCacheEntry>>>;

#[cfg(feature = "server")]
static GLOBAL_SERVER_CACHE: Lazy<ServerCache> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// A hook that caches the result of a server function for the specified duration.
/// – If the cache is still valid, the cached value is returned.
/// – Otherwise we hit the server, store the result and return it.
///
/// Typical usage:
/// ```rust
/// let data = use_cached_server(
///     "my_key",
///     || my_server_fn(),
///     Duration::from_secs(60 * 5),
/// );
/// ```
pub fn use_cached_server<T, F, Fut>(
    key: &str,
    server_fn: F,
    cache_duration: Duration,
) -> Resource<Result<T, ServerFnError>>
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let cache_key = format!("cache_{}", key);
    let duration_ms = cache_duration.as_secs_f64() * 1000.0;

    use_resource(move || {
        let server_fn = server_fn.clone();
        let cache_key = cache_key.clone();

        async move {
            #[cfg(feature = "web")]
            {
                let now = Date::now();

                // 1. Try the cache -----------------------------------------------------------
                if let Some(window) = window() {
                    if let Ok(Some(storage)) = window.local_storage() {
                        if let Ok(Some(json)) = storage.get_item(&cache_key) {
                            if let Ok(cached) = serde_json::from_str::<CachedData<T>>(&json) {
                                if cached.expires_at > now {
                                    return Ok(cached.data);
                                }
                            }
                        }
                    }
                }

                // 2. Cache miss => go to the server ----------------------------------------
                let result = server_fn().await?;

                // 3. Persist in localStorage ------------------------------------------------
                let expires_at = now + duration_ms;
                let cached = CachedData {
                    data: result.clone(),
                    expires_at,
                };

                if let Ok(json) = serde_json::to_string(&cached) {
                    if let Some(window) = window() {
                        if let Ok(Some(storage)) = window.local_storage() {
                            let _ = storage.set_item(&cache_key, &json);
                        }
                    }
                }

                Ok(result)
            }

            #[cfg(not(feature = "web"))]
            {
                // non-web platforms: no caching, just call the fn.
                server_fn().await
            }
        }
    })
}

/// A hook that ONLY peeks into the cache.
///
/// • If a *valid* entry exists → `Some(data)`
/// • Otherwise                   `None`
///
/// It never triggers a network request, making it ideal for e.g. a 5-minute
/// interval that gives the UI instant data while a fresh `use_cached_server`
/// call is still in-flight.
///
/// Example:
/// ```rust
/// let maybe_data: Option<MyType> = use_existing_cached_server("my_key");
/// ```
pub fn use_existing_cached_server<T>(key: &str) -> Option<T>
where
    T: Clone + PartialEq + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
{
    let cache_key = format!("cache_{}", key);

    let cached = use_memo(move || {
        #[cfg(feature = "web")]
        {
            if let Some(window) = window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    if let Ok(Some(json)) = storage.get_item(&cache_key) {
                        if let Ok(cached) = serde_json::from_str::<CachedData<T>>(&json) {
                            if cached.expires_at > Date::now() {
                                return Some(cached.data);
                            }
                        }
                    }
                }
            }
            None
        }

        #[cfg(not(feature = "web"))]
        {
            None
        }
    });

    (*cached)().clone()
}

/// Helper function to get cached data regardless of expiry
#[cfg(feature = "web")]
fn get_any_cached_data<T>(cache_key: &str) -> Option<T>
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item(cache_key) {
                if let Ok(cached) = serde_json::from_str::<CachedData<T>>(&json) {
                    return Some(cached.data);
                }
            }
        }
    }
    None
}

/// Helper function to store data in cache
#[cfg(feature = "web")]
fn store_cached_data<T>(cache_key: &str, data: &T, duration_ms: f64)
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    let now = Date::now();
    let expires_at = now + duration_ms;
    let cached = CachedData {
        data: data.clone(),
        expires_at,
    };

    if let Ok(json) = serde_json::to_string(&cached) {
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(cache_key, &json);
            }
        }
    }
}

/// NEW: Stale-while-revalidate hook with callback support
/// Returns a signal that contains the current data and automatically updates
/// when new data is available (cached or fresh)
pub fn use_stale_while_revalidate_with_callback<T, F, Fut, C>(
    cache_key: &str,
    server_fn: F,
    cache_duration: Duration,
    callback: C,
) -> Signal<Option<T>>
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
    C: Fn(&T) + Clone + 'static,
{
    let mut data_signal = use_signal(|| None::<T>);
    let cache_key_formatted = format!("cache_{}", cache_key);
    let duration_ms = cache_duration.as_secs_f64() * 1000.0;

    // Initialize with cached data if available
    use_effect({
        let cache_key_formatted = cache_key_formatted.clone();
        let callback = callback.clone();
        move || {
            #[cfg(feature = "web")]
            {
                tracing::info!(
                    "Checking for cache data for {} at {}",
                    cache_key_formatted,
                    Local::now().format("%M:%S%.3f")
                );
                if let Some(cached_data) = get_any_cached_data::<T>(&cache_key_formatted) {
                    data_signal.set(Some(cached_data.clone()));
                    callback(&cached_data);
                    tracing::info!(
                        "Got cache data for {} at {}",
                        cache_key_formatted,
                        Local::now().format("%M:%S%.3f")
                    );
                }
            }
        }
    });

    // Spawn async task to fetch fresh data
    use_effect({
        let server_fn = server_fn.clone();
        let cache_key_formatted = cache_key_formatted.clone();
        let callback = callback.clone();

        move || {
            spawn({
                let server_fn = server_fn.clone();
                let cache_key_formatted = cache_key_formatted.clone();
                let callback = callback.clone();

                async move {
                    // Small delay to allow cached data to be set first
                    #[cfg(feature = "web")]
                    {
                        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                            &wasm_bindgen::JsValue::from(0),
                        ))
                        .await
                        .ok();
                    }

                    tracing::info!(
                        "Getting fresh data for {} at {}",
                        cache_key_formatted,
                        Local::now().format("%M:%S%.3f")
                    );
                    match server_fn().await {
                        Ok(fresh_data) => {
                            #[cfg(feature = "web")]
                            store_cached_data(&cache_key_formatted, &fresh_data, duration_ms);

                            data_signal.set(Some(fresh_data.clone()));
                            tracing::info!(
                                "Got fresh data for {} at {}",
                                cache_key_formatted,
                                chrono::Local::now().format("%M:%S%.3f")
                            );
                            callback(&fresh_data);
                        }
                        Err(_) => {
                            // If server call fails but we have cached data, keep using it
                            // Otherwise, the signal remains None or keeps its current value
                        }
                    }
                }
            });
        }
    });

    data_signal
}

/// NEW: Stale-while-revalidate hook without callback
/// Returns a signal that contains the current data and automatically updates
/// when new data is available (cached or fresh)
pub fn use_stale_while_revalidate<T, F, Fut>(
    cache_key: &str,
    server_fn: F,
    cache_duration: Duration,
) -> Signal<Option<T>>
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    use_stale_while_revalidate_with_callback(
        cache_key,
        server_fn,
        cache_duration,
        |_| {}, // No-op callback
    )
}

/// NEW: Stale-while-revalidate hook with loading state
/// Returns a tuple of (data_signal, loading_signal)
pub fn use_stale_while_revalidate_with_loading<T, F, Fut>(
    cache_key: &str,
    server_fn: F,
    cache_duration: Duration,
) -> (Signal<Option<T>>, Signal<bool>)
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let mut data_signal = use_signal(|| None::<T>);
    let mut loading_signal = use_signal(|| true);
    let cache_key_formatted = format!("cache_{}", cache_key);
    let duration_ms = cache_duration.as_secs_f64() * 1000.0;

    // Initialize with cached data if available
    use_effect({
        let cache_key_formatted = cache_key_formatted.clone();
        move || {
            #[cfg(feature = "web")]
            {
                if let Some(cached_data) = get_any_cached_data::<T>(&cache_key_formatted) {
                    data_signal.set(Some(cached_data));
                    loading_signal.set(false);
                } else {
                    loading_signal.set(true);
                }
            }

            #[cfg(not(feature = "web"))]
            {
                loading_signal.set(true);
            }
        }
    });

    // Spawn async task to fetch fresh data
    use_effect({
        let server_fn = server_fn.clone();
        let cache_key_formatted = cache_key_formatted.clone();

        move || {
            spawn({
                let server_fn = server_fn.clone();
                let cache_key_formatted = cache_key_formatted.clone();

                async move {
                    // Small delay to allow cached data to be set first
                    #[cfg(feature = "web")]
                    {
                        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                            &wasm_bindgen::JsValue::from(0),
                        ))
                        .await
                        .ok();
                    }

                    match server_fn().await {
                        Ok(fresh_data) => {
                            #[cfg(feature = "web")]
                            store_cached_data(&cache_key_formatted, &fresh_data, duration_ms);

                            data_signal.set(Some(fresh_data));
                        }
                        Err(_) => {
                            // If server call fails, we keep whatever data we have
                        }
                    }

                    loading_signal.set(false);
                }
            });
        }
    });

    (data_signal, loading_signal)
}

/// NEW: Stale-while-revalidate hook with callback and loading state
/// Returns a tuple of (data_signal, loading_signal)
pub fn use_stale_while_revalidate_with_callback_and_loading<T, F, Fut, C>(
    cache_key: &str,
    server_fn: F,
    cache_duration: Duration,
    callback: C,
) -> (Signal<Option<T>>, Signal<bool>)
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
    C: Fn(&T) + Clone + 'static,
{
    let mut data_signal = use_signal(|| None::<T>);
    let mut loading_signal = use_signal(|| true);
    let cache_key_formatted = format!("cache_{}", cache_key);
    let duration_ms = cache_duration.as_secs_f64() * 1000.0;

    // Initialize with cached data if available
    use_effect({
        let cache_key_formatted = cache_key_formatted.clone();
        let callback = callback.clone();
        move || {
            #[cfg(feature = "web")]
            {
                if let Some(cached_data) = get_any_cached_data::<T>(&cache_key_formatted) {
                    data_signal.set(Some(cached_data.clone()));
                    loading_signal.set(false);
                    callback(&cached_data);
                } else {
                    loading_signal.set(true);
                }
            }

            #[cfg(not(feature = "web"))]
            {
                loading_signal.set(true);
            }
        }
    });

    // Spawn async task to fetch fresh data
    use_effect({
        let server_fn = server_fn.clone();
        let cache_key_formatted = cache_key_formatted.clone();
        let callback = callback.clone();

        move || {
            spawn({
                let server_fn = server_fn.clone();
                let cache_key_formatted = cache_key_formatted.clone();
                let callback = callback.clone();

                async move {
                    // Small delay to allow cached data to be set first
                    #[cfg(feature = "web")]
                    {
                        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                            &wasm_bindgen::JsValue::from(0),
                        ))
                        .await
                        .ok();
                    }

                    match server_fn().await {
                        Ok(fresh_data) => {
                            #[cfg(feature = "web")]
                            store_cached_data(&cache_key_formatted, &fresh_data, duration_ms);

                            data_signal.set(Some(fresh_data.clone()));
                            callback(&fresh_data);
                        }
                        Err(_) => {
                            // If server call fails, we keep whatever data we have
                        }
                    }

                    loading_signal.set(false);
                }
            });
        }
    });

    (data_signal, loading_signal)
}

/// Force refresh a cache entry - clears cache and fetches fresh data
pub fn use_force_refresh<T, F, Fut>(
    cache_key: &str,
    server_fn: F,
    cache_duration: Duration,
) -> Signal<Option<Result<T, ServerFnError>>>
where
    T: Clone + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let mut result_signal = use_signal(|| None::<Result<T, ServerFnError>>);
    let cache_key_formatted = format!("cache_{}", cache_key);
    let duration_ms = cache_duration.as_secs_f64() * 1000.0;

    use_effect(move || {
        spawn({
            let server_fn = server_fn.clone();
            let cache_key_formatted = cache_key_formatted.clone();

            async move {
                // Clear existing cache
                #[cfg(feature = "web")]
                {
                    if let Some(window) = window() {
                        if let Ok(Some(storage)) = window.local_storage() {
                            let _ = storage.remove_item(&cache_key_formatted);
                        }
                    }
                }

                // Fetch fresh data
                match server_fn().await {
                    Ok(fresh_data) => {
                        #[cfg(feature = "web")]
                        store_cached_data(&cache_key_formatted, &fresh_data, duration_ms);

                        result_signal.set(Some(Ok(fresh_data)));
                    }
                    Err(e) => {
                        result_signal.set(Some(Err(e)));
                    }
                }
            }
        });
    });

    result_signal
}

// ==================== SERVER-SIDE CACHE FUNCTIONS ====================

/// Server function that manages server-side caching
#[server(GetCachedData)]
pub async fn get_cached_data(
    cache_key: String,
    should_refresh: bool,
) -> Result<Option<String>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let cache = GLOBAL_SERVER_CACHE.clone();

        // If refresh is requested, clear the cache entry
        if should_refresh {
            if let Ok(mut cache_map) = cache.write() {
                cache_map.remove(&cache_key);
                return Ok(None);
            }
        }

        // Try to get from cache
        if let Ok(cache_map) = cache.read() {
            if let Some(entry) = cache_map.get(&cache_key) {
                if entry.expires_at > Instant::now() {
                    return Ok(Some(entry.data.clone()));
                }
            }
        }
    }

    Ok(None)
}

/// Server function that stores data in server-side cache
#[server(SetCachedData)]
pub async fn set_cached_data(
    cache_key: String,
    data: String,
    cache_duration_secs: u64,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let cache = GLOBAL_SERVER_CACHE.clone();
        let expires_at = Instant::now() + Duration::from_secs(cache_duration_secs);

        let entry = ServerCacheEntry { data, expires_at };

        if let Ok(mut cache_map) = cache.write() {
            cache_map.insert(cache_key, entry);
        }
    }

    Ok(())
}

/// Helper function to get typed data from server cache
async fn get_server_cached_data<T>(cache_key: &str) -> Result<Option<T>, ServerFnError>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    match get_cached_data(cache_key.to_string(), false).await? {
        Some(json_data) => match serde_json::from_str::<T>(&json_data) {
            Ok(data) => Ok(Some(data)),
            Err(_) => Ok(None),
        },
        None => Ok(None),
    }
}

/// Helper function to set typed data in server cache
async fn set_server_cached_data<T>(
    cache_key: &str,
    data: &T,
    cache_duration_secs: u64,
) -> Result<(), ServerFnError>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    match serde_json::to_string(data) {
        Ok(json_data) => {
            set_cached_data(cache_key.to_string(), json_data, cache_duration_secs).await
        }
        Err(_) => Ok(()), // Silently fail if serialization fails
    }
}

/// Hook that provides server-side caching with immediate display and updates
///
/// This hook:
/// 1. Immediately displays any cached value from the server
/// 2. Fetches fresh data and updates the display
/// 3. Stores the fresh data on the server for future requests
///
/// Example usage:
/// ```rust
/// let products = use_server_cache(
///     "products_list",
///     || async { fetch_products_from_db().await },
///     Duration::from_secs(300), // Cache for 5 minutes
/// );
/// ```
pub fn use_server_cache<T, F, Fut>(
    cache_key: &str,
    data_fetcher: F,
    cache_duration: Duration,
) -> Signal<Option<T>>
where
    T: Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let mut data_signal = use_signal(|| None::<T>);
    let cache_key = cache_key.to_string();
    let cache_duration_secs = cache_duration.as_secs();

    // Effect to load initial cached data and then fetch fresh data
    use_effect({
        let cache_key = cache_key.clone();
        let data_fetcher = data_fetcher.clone();

        move || {
            spawn({
                let cache_key = cache_key.clone();
                let data_fetcher = data_fetcher.clone();

                async move {
                    // Step 1: Try to get cached data from server immediately
                    match get_server_cached_data::<T>(&cache_key).await {
                        Ok(Some(cached_data)) => {
                            tracing::info!("Loaded cached server data for key: {}", cache_key);
                            data_signal.set(Some(cached_data));
                        }
                        Ok(None) => {
                            tracing::info!("No cached server data found for key: {}", cache_key);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load cached server data: {:?}", e);
                        }
                    }

                    // Step 2: Fetch fresh data
                    match data_fetcher().await {
                        Ok(fresh_data) => {
                            tracing::info!("Fetched fresh data for key: {}", cache_key);

                            // Update the signal
                            data_signal.set(Some(fresh_data.clone()));

                            // Store in server cache for next time
                            if let Err(e) =
                                set_server_cached_data(&cache_key, &fresh_data, cache_duration_secs)
                                    .await
                            {
                                tracing::warn!("Failed to cache data on server: {:?}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to fetch fresh data for key {}: {:?}",
                                cache_key,
                                e
                            );
                            // Keep any existing cached data in the signal
                        }
                    }
                }
            });
        }
    });

    data_signal
}

/// Hook for server cache with loading state
/// Returns (data_signal, loading_signal)
pub fn use_server_cache_with_loading<T, F, Fut>(
    cache_key: &str,
    data_fetcher: F,
    cache_duration: Duration,
) -> (Signal<Option<T>>, Signal<bool>)
where
    T: Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let mut data_signal = use_signal(|| None::<T>);
    let mut loading_signal = use_signal(|| true);
    let cache_key = cache_key.to_string();
    let cache_duration_secs = cache_duration.as_secs();

    use_effect({
        let cache_key = cache_key.clone();
        let data_fetcher = data_fetcher.clone();

        move || {
            spawn({
                let cache_key = cache_key.clone();
                let data_fetcher = data_fetcher.clone();

                async move {
                    // Try to get cached data first
                    match get_server_cached_data::<T>(&cache_key).await {
                        Ok(Some(cached_data)) => {
                            data_signal.set(Some(cached_data));
                            loading_signal.set(false); // We have data, so not loading
                        }
                        Ok(None) => {
                            // No cached data, keep loading state true
                        }
                        Err(_) => {
                            // Error getting cached data, keep loading state true
                        }
                    }

                    // Fetch fresh data
                    match data_fetcher().await {
                        Ok(fresh_data) => {
                            data_signal.set(Some(fresh_data.clone()));
                            let _ = set_server_cached_data(
                                &cache_key,
                                &fresh_data,
                                cache_duration_secs,
                            )
                            .await;
                        }
                        Err(_) => {
                            // If we don't have cached data and fresh fetch fails,
                            // we'll have no data but loading should be false
                        }
                    }

                    loading_signal.set(false);
                }
            });
        }
    });

    (data_signal, loading_signal)
}

/// Force refresh server cache - clears server cache and fetches fresh data
pub fn use_server_cache_refresh<T, F, Fut>(
    cache_key: &str,
    data_fetcher: F,
    cache_duration: Duration,
) -> Signal<Option<Result<T, ServerFnError>>>
where
    T: Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let mut result_signal = use_signal(|| None::<Result<T, ServerFnError>>);
    let cache_key = cache_key.to_string();
    let cache_duration_secs = cache_duration.as_secs();

    use_effect(move || {
        spawn({
            let cache_key = cache_key.clone();
            let data_fetcher = data_fetcher.clone();

            async move {
                // Clear the server cache
                let _ = get_cached_data(cache_key.clone(), true).await;

                // Fetch fresh data
                match data_fetcher().await {
                    Ok(fresh_data) => {
                        let _ =
                            set_server_cached_data(&cache_key, &fresh_data, cache_duration_secs)
                                .await;
                        result_signal.set(Some(Ok(fresh_data)));
                    }
                    Err(e) => {
                        result_signal.set(Some(Err(e)));
                    }
                }
            }
        });
    });

    result_signal
}

/// Hybrid cache hook that uses both client-side and server-side caching
/// This provides the best of both worlds:
/// - Instant display from client cache
/// - Fallback to server cache if client cache is empty
/// - Fresh data fetching and storage in both caches
pub fn use_hybrid_cache<T, F, Fut>(
    cache_key: &str,
    data_fetcher: F,
    cache_duration: Duration,
) -> Signal<Option<T>>
where
    T: Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    F: Fn() -> Fut + Clone + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let mut data_signal = use_signal(|| None::<T>);
    let cache_key = cache_key.to_string();
    let cache_duration_secs = cache_duration.as_secs();
    let duration_ms = cache_duration.as_secs_f64() * 1000.0;

    use_effect({
        let cache_key = cache_key.clone();
        let data_fetcher = data_fetcher.clone();

        move || {
            spawn({
                let cache_key = cache_key.clone();
                let data_fetcher = data_fetcher.clone();

                async move {
                    let client_cache_key = format!("cache_{}", cache_key);
                    let mut has_data = false;

                    // Step 1: Try client-side cache first (fastest)
                    #[cfg(feature = "web")]
                    {
                        if let Some(cached_data) = get_any_cached_data::<T>(&client_cache_key) {
                            tracing::info!("Loaded from client cache: {}", cache_key);
                            data_signal.set(Some(cached_data));
                            has_data = true;
                        }
                    }

                    // Step 2: If no client cache, try server cache
                    if !has_data {
                        match get_server_cached_data::<T>(&cache_key).await {
                            Ok(Some(server_cached_data)) => {
                                tracing::info!("Loaded from server cache: {}", cache_key);
                                data_signal.set(Some(server_cached_data.clone()));

                                // Store in client cache for next time
                                #[cfg(feature = "web")]
                                store_cached_data(
                                    &client_cache_key,
                                    &server_cached_data,
                                    duration_ms,
                                );

                                has_data = true;
                            }
                            Ok(None) => {
                                tracing::info!("No server cache found: {}", cache_key);
                            }
                            Err(e) => {
                                tracing::warn!("Server cache error: {:?}", e);
                            }
                        }
                    }

                    // Step 3: Fetch fresh data (always, for revalidation)
                    match data_fetcher().await {
                        Ok(fresh_data) => {
                            tracing::info!("Fetched fresh data: {}", cache_key);
                            data_signal.set(Some(fresh_data.clone()));

                            // Store in both caches
                            #[cfg(feature = "web")]
                            store_cached_data(&client_cache_key, &fresh_data, duration_ms);

                            let _ = set_server_cached_data(
                                &cache_key,
                                &fresh_data,
                                cache_duration_secs,
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch fresh data: {:?}", e);
                            // Keep any existing data
                        }
                    }
                }
            });
        }
    });

    data_signal
}

/// Server cache cleanup utility - removes expired entries
#[server(CleanupServerCache)]
pub async fn cleanup_server_cache() -> Result<usize, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let cache = GLOBAL_SERVER_CACHE.clone();
        let now = Instant::now();
        let mut removed_count = 0;

        if let Ok(mut cache_map) = cache.write() {
            let keys_to_remove: Vec<String> = cache_map
                .iter()
                .filter_map(|(key, entry)| {
                    if entry.expires_at <= now {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .collect();

            for key in keys_to_remove {
                cache_map.remove(&key);
                removed_count += 1;
            }
        }

        return Ok(removed_count);
    }

    #[cfg(not(feature = "server"))]
    Ok(0)
}

// ==================== CLIENT-SIDE UTILITIES ====================

#[cfg(feature = "web")]
pub fn diagnose_localstorage_performance() {
    use js_sys::Date;
    use web_sys::console;

    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let start_time = Date::now();

            // Count total items
            let length = storage.length().unwrap_or(0);

            // Calculate total storage size
            let mut total_size = 0;
            let mut cache_items = 0;

            for i in 0..length {
                if let Ok(Some(key)) = storage.key(i) {
                    if let Ok(Some(value)) = storage.get_item(&key) {
                        total_size += key.len() + value.len();
                        if key.starts_with("cache_") {
                            cache_items += 1;
                        }
                    }
                }
            }

            let end_time = Date::now();
            let scan_time = end_time - start_time;

            console::log_1(
                &format!(
                    "localStorage Performance Diagnostic:
                - Total items: {}
                - Cache items: {}
                - Total size: ~{} bytes
                - Scan time: {}ms",
                    length, cache_items, total_size, scan_time
                )
                .into(),
            );

            // Test read performance of a specific cache key
            let test_start = Date::now();
            let _ = storage.get_item("cache_get_products");
            let test_end = Date::now();

            console::log_1(&format!("Cache read test time: {}ms", test_end - test_start).into());
        }
    }
}

#[cfg(feature = "web")]
pub fn cleanup_old_cache_entries() {
    use js_sys::Date;

    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let now = Date::now();
            let mut keys_to_remove = Vec::new();

            let length = storage.length().unwrap_or(0);
            for i in 0..length {
                if let Ok(Some(key)) = storage.key(i) {
                    if key.starts_with("cache_") {
                        if let Ok(Some(json)) = storage.get_item(&key) {
                            if let Ok(cached) = serde_json::from_str::<serde_json::Value>(&json) {
                                if let Some(expires_at) =
                                    cached.get("expires_at").and_then(|v| v.as_f64())
                                {
                                    if expires_at < now {
                                        keys_to_remove.push(key);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            for key in keys_to_remove {
                let _ = storage.remove_item(&key);
            }
        }
    }
}

#[cfg(feature = "web")]
pub fn get_cache_stats() -> Option<(usize, usize)> {
    if let Some(window) = window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let length = storage.length().unwrap_or(0) as usize;
            let mut total_size = 0;

            for i in 0..length {
                if let Ok(Some(key)) = storage.key(i as u32) {
                    if let Ok(Some(value)) = storage.get_item(&key) {
                        total_size += key.len() + value.len();
                    }
                }
            }

            return Some((length, total_size));
        }
    }
    None
}
