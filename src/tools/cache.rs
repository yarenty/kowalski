/// Cache module: Because nobody likes waiting.
/// "Caching is like having a good memory - it's great until you forget to invalidate it." - A Memory Expert

use std::time::Duration;
use cached::TimedCache;
use super::{ToolInput, ToolOutput };
use cached::Cached;

/// Storage types for cache, because one size doesn't fit all.
#[derive(Debug, Clone)]
pub enum Storage {
    Memory,
    #[allow(dead_code)]
    Local(String),
    // Redis(String),
    #[allow(dead_code)]
    None,
}

/// Cache configuration, because even caches need rules.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CacheConfig {
    pub ttl: Duration,
    pub storage: Storage,
    pub max_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(3600), // 1 hour
            storage: Storage::Memory,
            max_size: 1000,
        }
    }
}

/// Tool cache, because recalculating is overrated.
pub struct ToolCache {
    config: CacheConfig,
    memory_cache: TimedCache<String, ToolOutput>,
    storage: Storage,
}

impl ToolCache {
    pub fn new() -> Self {
        let config = CacheConfig::default();
        Self {
            memory_cache: TimedCache::with_lifespan_and_capacity(
                config.ttl.as_secs() as u64,
                config.max_size,
            ),
            config,
            storage: Storage::Memory,
        }
    }

    #[allow(dead_code)]
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.config.ttl = ttl;
        self.memory_cache = TimedCache::with_lifespan_and_capacity(
            ttl.as_secs() as u64,
            self.config.max_size,
        );
        self
    }

    #[allow(dead_code)]
    pub fn with_storage(mut self, storage: Storage) -> Self {
        self.storage = storage;
        self
    }

    pub fn get(&mut self, key: &ToolInput) -> Option<ToolOutput> {
        match &mut self.storage {
            Storage::Memory => self.memory_cache.cache_get(&key.to_string()).cloned(),
            Storage::Local(_) => None, // TODO: Implement local storage
            // Storage::Redis(_) => None, // TODO: Implement Redis storage
            Storage::None => None,
        }
    }

    pub fn set(&mut self, key: &ToolInput, output: &ToolOutput) {
        match &mut self.storage {
            Storage::Memory => {
                self.memory_cache.cache_set(key.to_string(), output.clone());
            }
            Storage::Local(_) => {}, // TODO: Implement local storage
            // Storage::Redis(_) => {}, // TODO: Implement Redis storage
            Storage::None => {},
        }
    }

    #[allow(dead_code)]
    fn create_cache_key(&self, input: &ToolInput) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        format!("tool_cache_{}", hasher.finish())
    }
} 