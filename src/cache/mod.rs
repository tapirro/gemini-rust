//! Context caching support for Gemini API

use crate::{
    client::GeminiClient,
    error::{Error, Result},
    models::Content,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Time to live for cached content (in seconds)
    pub ttl: Option<u64>,

    /// Display name for the cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Cached content reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedContent {
    /// Resource name of the cached content
    pub name: String,

    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Model used for caching
    pub model: String,

    /// Creation time
    pub create_time: DateTime<Utc>,

    /// Update time
    pub update_time: DateTime<Utc>,

    /// Expiration time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<DateTime<Utc>>,
}

/// Request to create cached content
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateCacheRequest {
    model: String,
    contents: Vec<Content>,

    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<Content>,

    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
}

/// Cache manager for handling context caching
pub struct CacheManager {
    /// In-memory cache tracking
    cache_registry: Arc<RwLock<HashMap<String, CachedContent>>>,

    /// Cache by display name for easy lookup
    name_index: Arc<RwLock<HashMap<String, String>>>,
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Self {
        Self {
            cache_registry: Arc::new(RwLock::new(HashMap::new())),
            name_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new cached content
    pub async fn create_cache(
        &self,
        client: &GeminiClient,
        model: Option<&str>,
        contents: Vec<Content>,
        system_instruction: Option<Content>,
        config: CacheConfig,
    ) -> Result<CachedContent> {
        let model_name = client.config().get_model_name(model);

        // Ensure we use a stable model version for caching
        let cache_model = if model_name.contains("-latest") {
            return Err(Error::Config(
                "Context caching requires a stable model version (e.g., gemini-1.5-pro-001)"
                    .to_string(),
            ));
        } else if !model_name.contains("-00") {
            // Add version suffix if not present
            format!("{}-001", model_name)
        } else {
            model_name.clone()
        };

        let request = CreateCacheRequest {
            model: cache_model,
            contents,
            system_instruction,
            ttl: config.ttl.map(|seconds| format!("{}s", seconds)),
            display_name: config.display_name.clone(),
        };

        let endpoint = format!(
            "{}/{}/cachedContents",
            client.config().base_url,
            client.config().api_version.as_str()
        );

        debug!(
            "Creating cached content with display name: {:?}",
            config.display_name
        );

        let response = client
            .http_client()
            .post(&endpoint)
            .query(&[("key", &client.config().api_key)])
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Cache(format!(
                "Failed to create cache (status {}): {}",
                status, error_body
            )));
        }

        let cached: CachedContent = response.json().await?;

        // Store in registry
        let mut registry = self.cache_registry.write().await;
        registry.insert(cached.name.clone(), cached.clone());

        if let Some(display_name) = &cached.display_name {
            let mut index = self.name_index.write().await;
            index.insert(display_name.clone(), cached.name.clone());
        }

        info!("Created cached content: {}", cached.name);

        Ok(cached)
    }

    /// Get cached content by resource name
    pub async fn get_cache(&self, client: &GeminiClient, name: &str) -> Result<CachedContent> {
        // Check local registry first
        {
            let registry = self.cache_registry.read().await;
            if let Some(cached) = registry.get(name) {
                // Check if not expired
                if let Some(expire_time) = cached.expire_time {
                    if expire_time > Utc::now() {
                        return Ok(cached.clone());
                    }
                }
            }
        }

        // Fetch from API
        let endpoint = format!(
            "{}/{}/{}",
            client.config().base_url,
            client.config().api_version.as_str(),
            name
        );

        let response = client
            .http_client()
            .get(&endpoint)
            .query(&[("key", &client.config().api_key)])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Cache(format!(
                "Failed to get cache (status {}): {}",
                status, error_body
            )));
        }

        let cached: CachedContent = response.json().await?;

        // Update registry
        let mut registry = self.cache_registry.write().await;
        registry.insert(cached.name.clone(), cached.clone());

        Ok(cached)
    }

    /// Get cached content by display name
    pub async fn get_cache_by_name(
        &self,
        client: &GeminiClient,
        display_name: &str,
    ) -> Result<CachedContent> {
        // Look up resource name from index
        let resource_name = {
            let index = self.name_index.read().await;
            index.get(display_name).cloned()
        };

        match resource_name {
            Some(name) => self.get_cache(client, &name).await,
            None => Err(Error::Cache(format!(
                "No cache found with display name: {}",
                display_name
            ))),
        }
    }

    /// List all cached contents
    pub async fn list_caches(
        &self,
        client: &GeminiClient,
        page_size: Option<i32>,
        page_token: Option<&str>,
    ) -> Result<ListCachesResponse> {
        let endpoint = format!(
            "{}/{}/cachedContents",
            client.config().base_url,
            client.config().api_version.as_str()
        );

        let mut query = vec![("key", client.config().api_key.as_str())];

        let page_size_str;
        if let Some(size) = page_size {
            page_size_str = size.to_string();
            query.push(("pageSize", &page_size_str));
        }

        if let Some(token) = page_token {
            query.push(("pageToken", token));
        }

        let response = client
            .http_client()
            .get(&endpoint)
            .query(&query)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Cache(format!(
                "Failed to list caches (status {}): {}",
                status, error_body
            )));
        }

        let list_response: ListCachesResponse = response.json().await?;

        // Update registry with all caches
        if let Some(caches) = &list_response.cached_contents {
            let mut registry = self.cache_registry.write().await;
            let mut index = self.name_index.write().await;

            for cached in caches {
                registry.insert(cached.name.clone(), cached.clone());

                if let Some(display_name) = &cached.display_name {
                    index.insert(display_name.clone(), cached.name.clone());
                }
            }
        }

        Ok(list_response)
    }

    /// Update cache TTL
    pub async fn update_cache_ttl(
        &self,
        client: &GeminiClient,
        name: &str,
        ttl_seconds: u64,
    ) -> Result<CachedContent> {
        let endpoint = format!(
            "{}/{}/{}",
            client.config().base_url,
            client.config().api_version.as_str(),
            name
        );

        let update_request = serde_json::json!({
            "ttl": format!("{}s", ttl_seconds)
        });

        let response = client
            .http_client()
            .patch(&endpoint)
            .query(&[("key", &client.config().api_key)])
            .query(&[("updateMask", "ttl")])
            .json(&update_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Cache(format!(
                "Failed to update cache (status {}): {}",
                status, error_body
            )));
        }

        let cached: CachedContent = response.json().await?;

        // Update registry
        let mut registry = self.cache_registry.write().await;
        registry.insert(cached.name.clone(), cached.clone());

        Ok(cached)
    }

    /// Delete cached content
    pub async fn delete_cache(&self, client: &GeminiClient, name: &str) -> Result<()> {
        let endpoint = format!(
            "{}/{}/{}",
            client.config().base_url,
            client.config().api_version.as_str(),
            name
        );

        let response = client
            .http_client()
            .delete(&endpoint)
            .query(&[("key", &client.config().api_key)])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(Error::Cache(format!(
                "Failed to delete cache (status {}): {}",
                status, error_body
            )));
        }

        // Remove from registry
        let mut registry = self.cache_registry.write().await;
        if let Some(cached) = registry.remove(name) {
            if let Some(display_name) = cached.display_name {
                let mut index = self.name_index.write().await;
                index.remove(&display_name);
            }
        }

        info!("Deleted cached content: {}", name);

        Ok(())
    }

    /// Clean up expired caches from local registry
    pub async fn cleanup_expired(&self) {
        let now = Utc::now();
        let mut registry = self.cache_registry.write().await;
        let mut index = self.name_index.write().await;

        let expired: Vec<_> = registry
            .iter()
            .filter_map(|(name, cached)| {
                cached
                    .expire_time
                    .filter(|&expire| expire <= now)
                    .map(|_| (name.clone(), cached.display_name.clone()))
            })
            .collect();

        for (name, display_name) in expired {
            registry.remove(&name);
            if let Some(display_name) = display_name {
                index.remove(&display_name);
            }
            debug!("Removed expired cache: {}", name);
        }
    }
}

/// Response from list caches API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCachesResponse {
    /// List of cached contents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_contents: Option<Vec<CachedContent>>,

    /// Token for next page of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Helper to calculate optimal TTL based on content size
pub fn calculate_optimal_ttl(token_count: i32) -> u64 {
    const HOUR: u64 = 3600;
    const DAY: u64 = HOUR * 24;

    match token_count {
        0..=50000 => HOUR,            // 1 hour for small content
        50001..=200000 => HOUR * 4,   // 4 hours for medium content
        200001..=500000 => HOUR * 12, // 12 hours for large content
        _ => DAY,                     // 24 hours for very large content
    }
}
