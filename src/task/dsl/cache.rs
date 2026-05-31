//! Selector cache with LRU eviction and TTL support.
//!
//! Provides caching for DOM selector queries to improve performance
//! during DSL task execution. Supports TTL-based expiration and
//! LRU eviction when the cache reaches capacity.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Maximum selector cache size (LRU eviction).
const SELECTOR_CACHE_SIZE: usize = 100;

/// Selector cache entry with expiration.
#[derive(Debug, Clone)]
pub struct SelectorCacheEntry {
    /// Whether the selector exists
    pub exists: bool,
    /// Whether the selector is visible
    #[allow(dead_code)]
    pub visible: bool,
    /// Text content (if extracted)
    #[allow(dead_code)]
    pub text: Option<String>,
    /// Element count (for collection selectors)
    #[allow(dead_code)]
    pub count: usize,
    /// Timestamp when cached
    pub cached_at: Instant,
    /// TTL for this cache entry
    pub ttl: Duration,
}

impl SelectorCacheEntry {
    /// Create a new cache entry with default TTL of 5 seconds.
    #[must_use]
    pub fn new(exists: bool, visible: bool, text: Option<String>, count: usize) -> Self {
        Self {
            exists,
            visible,
            text,
            count,
            cached_at: Instant::now(),
            ttl: Duration::from_secs(5),
        }
    }

    /// Create a new cache entry with a custom TTL.
    #[must_use]
    pub fn with_ttl(
        exists: bool,
        visible: bool,
        text: Option<String>,
        count: usize,
        ttl: Duration,
    ) -> Self {
        Self {
            exists,
            visible,
            text,
            count,
            cached_at: Instant::now(),
            ttl,
        }
    }

    /// Check if this cache entry is still valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        Instant::now().duration_since(self.cached_at) < self.ttl
    }
}

/// Selector cache with LRU eviction and TTL.
#[derive(Default)]
pub struct SelectorCache {
    /// Cache storage with access order tracking
    cache: HashMap<String, (SelectorCacheEntry, Instant)>,
    /// Cache hit counter for metrics
    pub hits: u64,
    /// Cache miss counter for metrics
    pub misses: u64,
    /// Total cache evictions
    pub evictions: u64,
}

impl SelectorCache {
    /// Create a new selector cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: HashMap::with_capacity(SELECTOR_CACHE_SIZE),
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Get a cached entry if it exists and is still valid.
    pub fn get(&mut self, selector: &str) -> Option<&SelectorCacheEntry> {
        // Remove expired entries first
        let now = Instant::now();
        let expired: Vec<String> = self
            .cache
            .iter()
            .filter(|(_, (entry, _))| !entry.is_valid())
            .map(|(k, _)| k.clone())
            .collect();
        for key in expired {
            self.cache.remove(&key);
        }

        // Check if selector exists and is valid
        if let Some((entry, _)) = self.cache.get(selector) {
            if entry.is_valid() {
                self.hits += 1;
                // Update access time for LRU tracking
                if let Some((_, last_accessed)) = self.cache.get_mut(selector) {
                    *last_accessed = now;
                }
                // Return entry from cache
                return self.cache.get(selector).map(|(e, _)| e);
            }
            // Entry expired
            self.cache.remove(selector);
        }
        self.misses += 1;
        None
    }

    /// Insert a new entry into the cache.
    pub fn insert(&mut self, selector: String, entry: SelectorCacheEntry) {
        // Evict oldest entry if at capacity
        if self.cache.len() >= SELECTOR_CACHE_SIZE {
            if let Some(oldest) = self
                .cache
                .iter()
                .min_by_key(|(_, (_, accessed))| *accessed)
                .map(|(k, _)| k.clone())
            {
                self.cache.remove(&oldest);
                self.evictions += 1;
            }
        }
        self.cache.insert(selector, (entry, Instant::now()));
    }

    /// Invalidate all cached entries for a selector.
    #[allow(dead_code)]
    pub fn invalidate(&mut self, selector: &str) {
        self.cache.remove(selector);
    }

    /// Clear all cached entries.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }
}

/// Cache statistics for performance monitoring.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    /// Current cache size
    pub size: usize,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of evictions
    pub evictions: u64,
    /// Cache hit rate (0.0 - 1.0)
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_validity() {
        let entry = SelectorCacheEntry::new(true, false, None, 0);
        assert!(entry.is_valid()); // Should be valid for 5 seconds
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = SelectorCache::new();
        let entry = SelectorCacheEntry::new(true, true, Some("test".to_string()), 1);
        cache.insert("test-selector".to_string(), entry);

        let retrieved = cache.get("test-selector");
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().exists);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = SelectorCache::new();
        let result = cache.get("non-existent");
        assert!(result.is_none());
        assert_eq!(cache.misses, 1);
        assert_eq!(cache.hits, 0);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = SelectorCache::new();
        // Fill cache beyond capacity
        for i in 0..SELECTOR_CACHE_SIZE + 1 {
            let entry = SelectorCacheEntry::new(true, false, None, 0);
            cache.insert(format!("selector-{}", i), entry);
        }
        assert!(cache.cache.len() <= SELECTOR_CACHE_SIZE);
        assert!(cache.evictions > 0);
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = SelectorCache::new();
        let entry = SelectorCacheEntry::new(true, false, None, 0);
        cache.insert("key1".to_string(), entry);

        // Hit
        cache.get("key1");
        // Miss
        cache.get("key2");

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.size, 1);
    }
}
