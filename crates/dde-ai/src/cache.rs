//! Cache Management Module
//!
//! Provides caching for AI generation results to reduce API costs and improve response times.
//! Supports per-task-type TTL configuration and cache statistics.

use super::AiTaskType;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// A single cached entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Cached content
    pub content: String,
    /// Tokens used in the original generation
    pub tokens_used: u32,
    /// Model that generated the content
    pub model: String,
    /// When this entry was cached
    pub cached_at: Instant,
    /// Task type for this entry
    pub task_type: AiTaskType,
    /// Cache hit count
    pub hit_count: u64,
    /// Estimated size in bytes
    pub size_bytes: usize,
}

impl CacheEntry {
    /// Check if this entry has expired based on TTL
    pub fn is_expired(&self, ttl_minutes: u32) -> bool {
        let ttl = Duration::from_secs(ttl_minutes as u64 * 60);
        self.cached_at.elapsed() > ttl
    }

    /// Get age in minutes
    pub fn age_minutes(&self) -> u32 {
        (self.cached_at.elapsed().as_secs() / 60) as u32
    }

    /// Record a cache hit
    pub fn record_hit(&mut self) {
        self.hit_count += 1;
    }
}

/// Cached result for storage/retrieval
#[derive(Debug, Clone)]
pub struct CachedResult {
    pub content: String,
    pub tokens_used: u32,
    pub model: String,
}

/// Statistics for a specific task type
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskTypeCacheStats {
    /// Number of cached items
    pub item_count: usize,
    /// Total size in bytes
    pub total_bytes: usize,
    /// Total hits
    pub hit_count: u64,
    /// Expired entries
    pub expired_count: usize,
}

/// Overall cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total cached items
    pub total_items: usize,
    /// Total memory usage in bytes
    pub total_bytes: usize,
    /// Total memory usage in MB
    pub memory_mb: f32,
    /// Cache hit rate (0.0 - 1.0)
    pub hit_rate: f32,
    /// Total cache hits
    pub total_hits: u64,
    /// Total cache misses
    pub total_misses: u64,
    /// Expired entries
    pub expired_entries: usize,
    /// Statistics per task type
    pub by_task_type: HashMap<AiTaskType, TaskTypeCacheStats>,
    /// TTL configuration per task type (in minutes)
    pub ttl_minutes: HashMap<AiTaskType, u32>,
}

impl Default for CacheStats {
    fn default() -> Self {
        let mut ttl_minutes = HashMap::new();
        for task_type in AiTaskType::all() {
            ttl_minutes.insert(*task_type, task_type.default_ttl_minutes());
        }

        Self {
            total_items: 0,
            total_bytes: 0,
            memory_mb: 0.0,
            hit_rate: 0.0,
            total_hits: 0,
            total_misses: 0,
            expired_entries: 0,
            by_task_type: HashMap::new(),
            ttl_minutes,
        }
    }
}

/// Cache manager for AI generation results
pub struct CacheManager {
    /// Cache storage: request_id -> entry
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// TTL configuration per task type (in minutes)
    ttl_config: Arc<RwLock<HashMap<AiTaskType, u32>>>,
    /// Statistics tracking
    hits: Arc<RwLock<u64>>,
    misses: Arc<RwLock<u64>>,
}

impl CacheManager {
    /// Create a new cache manager with default settings
    pub fn new() -> Self {
        let mut ttl_config = HashMap::new();
        for task_type in AiTaskType::all() {
            ttl_config.insert(*task_type, task_type.default_ttl_minutes());
        }

        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl_config: Arc::new(RwLock::new(ttl_config)),
            hits: Arc::new(RwLock::new(0)),
            misses: Arc::new(RwLock::new(0)),
        }
    }

    /// Get a cached entry by request ID
    pub fn get_cached(&self, request_id: &str) -> Option<CacheEntry> {
        let ttl_config = self.ttl_config.read().unwrap();
        let mut entries = self.entries.write().unwrap();

        if let Some(entry) = entries.get_mut(request_id) {
            let ttl = *ttl_config.get(&entry.task_type).unwrap_or(&60);

            if entry.is_expired(ttl) {
                // Entry expired, remove it
                entries.remove(request_id);
                return None;
            }

            // Record hit
            entry.record_hit();
            
            // Clone the entry before dropping the lock
            let result = entry.clone();
            drop(entries);

            let mut hits = self.hits.write().unwrap();
            *hits += 1;

            return Some(result);
        }

        // Cache miss
        drop(entries);
        let mut misses = self.misses.write().unwrap();
        *misses += 1;

        None
    }

    /// Store a result in the cache
    pub fn store(&self, request_id: &str, result: CachedResult, task_type: AiTaskType) {
        let size_bytes = result.content.len();

        let entry = CacheEntry {
            content: result.content,
            tokens_used: result.tokens_used,
            model: result.model,
            cached_at: Instant::now(),
            task_type,
            hit_count: 0,
            size_bytes,
        };

        let mut entries = self.entries.write().unwrap();
        entries.insert(request_id.to_string(), entry);
    }

    /// Clear all cached entries
    pub fn clear_all(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.clear();

        // Reset stats
        let mut hits = self.hits.write().unwrap();
        let mut misses = self.misses.write().unwrap();
        *hits = 0;
        *misses = 0;
    }

    /// Clear cached entries for a specific task type
    pub fn clear_by_task_type(&self, task_type: AiTaskType) {
        let mut entries = self.entries.write().unwrap();
        entries.retain(|_, entry| entry.task_type != task_type);
    }

    /// Set TTL for a specific task type
    pub fn set_ttl(&self, task_type: AiTaskType, minutes: u32) {
        let mut ttl_config = self.ttl_config.write().unwrap();
        ttl_config.insert(task_type, minutes.max(1)); // Minimum 1 minute
    }

    /// Get TTL for a specific task type
    pub fn get_ttl(&self, task_type: AiTaskType) -> u32 {
        let ttl_config = self.ttl_config.read().unwrap();
        *ttl_config.get(&task_type).unwrap_or(&60)
    }

    /// Get all TTL configurations
    pub fn get_all_ttls(&self) -> HashMap<AiTaskType, u32> {
        let ttl_config = self.ttl_config.read().unwrap();
        ttl_config.clone()
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let entries = self.entries.read().unwrap();
        let ttl_config = self.ttl_config.read().unwrap();
        let hits = *self.hits.read().unwrap();
        let misses = *self.misses.read().unwrap();

        let total_requests = hits + misses;
        let hit_rate = if total_requests > 0 {
            hits as f32 / total_requests as f32
        } else {
            0.0
        };

        let mut by_task_type: HashMap<AiTaskType, TaskTypeCacheStats> = HashMap::new();
        let mut total_bytes = 0usize;
        let mut expired_count = 0usize;

        for (_, entry) in entries.iter() {
            let ttl = *ttl_config.get(&entry.task_type).unwrap_or(&60);
            let is_expired = entry.is_expired(ttl);

            if is_expired {
                expired_count += 1;
            }

            total_bytes += entry.size_bytes;

            let stats = by_task_type.entry(entry.task_type).or_default();
            stats.item_count += 1;
            stats.total_bytes += entry.size_bytes;
            stats.hit_count += entry.hit_count;
            if is_expired {
                stats.expired_count += 1;
            }
        }

        // Initialize stats for all task types (even if empty)
        for task_type in AiTaskType::all() {
            by_task_type.entry(*task_type).or_default();
        }

        CacheStats {
            total_items: entries.len(),
            total_bytes,
            memory_mb: total_bytes as f32 / (1024.0 * 1024.0),
            hit_rate,
            total_hits: hits,
            total_misses: misses,
            expired_entries: expired_count,
            by_task_type,
            ttl_minutes: ttl_config.clone(),
        }
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&self) {
        let ttl_config = self.ttl_config.read().unwrap();
        let mut entries = self.entries.write().unwrap();

        entries.retain(|_, entry| {
            let ttl = *ttl_config.get(&entry.task_type).unwrap_or(&60);
            !entry.is_expired(ttl)
        });
    }

    /// Get entries for a specific task type
    pub fn get_entries_by_type(&self, task_type: AiTaskType) -> Vec<CacheEntry> {
        let entries = self.entries.read().unwrap();
        entries
            .values()
            .filter(|e| e.task_type == task_type)
            .cloned()
            .collect()
    }

    /// Get total number of entries
    pub fn entry_count(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.len()
    }

    /// Check if cache contains a specific request
    pub fn contains(&self, request_id: &str) -> bool {
        let entries = self.entries.read().unwrap();
        entries.contains_key(request_id)
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CacheManager {
    fn clone(&self) -> Self {
        // Create a new manager with same TTL config but empty cache
        let new_manager = Self::new();
        let ttl_config = self.get_all_ttls();
        for (task_type, ttl) in ttl_config {
            new_manager.set_ttl(task_type, ttl);
        }
        new_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_store_and_retrieve() {
        let cache = CacheManager::new();

        let result = CachedResult {
            content: "Test content".to_string(),
            tokens_used: 10,
            model: "gpt-4".to_string(),
        };

        cache.store("req1", result.clone(), AiTaskType::Bark);

        let cached = cache.get_cached("req1");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "Test content");
    }

    #[test]
    fn test_cache_miss() {
        let cache = CacheManager::new();

        let cached = cache.get_cached("nonexistent");
        assert!(cached.is_none());

        let stats = cache.get_stats();
        assert_eq!(stats.total_misses, 1);
    }

    #[test]
    fn test_clear_by_task_type() {
        let cache = CacheManager::new();

        cache.store(
            "req1",
            CachedResult {
                content: "Bark".to_string(),
                tokens_used: 5,
                model: "local".to_string(),
            },
            AiTaskType::Bark,
        );

        cache.store(
            "req2",
            CachedResult {
                content: "Dialogue".to_string(),
                tokens_used: 50,
                model: "gpt-4".to_string(),
            },
            AiTaskType::Dialogue,
        );

        cache.clear_by_task_type(AiTaskType::Bark);

        assert!(cache.get_cached("req1").is_none());
        assert!(cache.get_cached("req2").is_some());
    }

    #[test]
    fn test_ttl_configuration() {
        let cache = CacheManager::new();

        cache.set_ttl(AiTaskType::Bark, 30);
        assert_eq!(cache.get_ttl(AiTaskType::Bark), 30);

        let ttls = cache.get_all_ttls();
        assert_eq!(ttls.get(&AiTaskType::Bark), Some(&30));
    }

    #[test]
    fn test_cache_stats() {
        let cache = CacheManager::new();

        cache.store(
            "req1",
            CachedResult {
                content: "Content".to_string(),
                tokens_used: 10,
                model: "gpt-4".to_string(),
            },
            AiTaskType::Narrative,
        );

        // First get is a hit
        let _ = cache.get_cached("req1");

        let stats = cache.get_stats();
        assert_eq!(stats.total_items, 1);
        assert_eq!(stats.total_hits, 1);
        assert!(stats.hit_rate > 0.0);
    }
}
