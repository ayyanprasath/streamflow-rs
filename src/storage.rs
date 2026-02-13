//! Storage abstraction module

use crate::{record::Record, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Trait for storage backends
#[async_trait]
pub trait Storage: Send + Sync {
    /// Store a record
    async fn store(&self, record: &Record) -> Result<()>;
    
    /// Retrieve a record by ID
    async fn get(&self, id: &Uuid) -> Result<Option<Record>>;
    
    /// Update a record
    async fn update(&self, record: &Record) -> Result<()>;
    
    /// Delete a record
    async fn delete(&self, id: &Uuid) -> Result<bool>;
    
    /// List all record IDs
    async fn list(&self) -> Result<Vec<Uuid>>;
    
    /// Count total records
    async fn count(&self) -> Result<usize>;
    
    /// Clear all records
    async fn clear(&self) -> Result<()>;
}

/// In-memory storage implementation
#[derive(Debug, Clone)]
pub struct InMemoryStorage {
    records: Arc<DashMap<Uuid, Record>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            records: Arc::new(DashMap::new()),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn store(&self, record: &Record) -> Result<()> {
        self.records.insert(record.id, record.clone());
        Ok(())
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Record>> {
        Ok(self.records.get(id).map(|r| r.clone()))
    }

    async fn update(&self, record: &Record) -> Result<()> {
        if self.records.contains_key(&record.id) {
            self.records.insert(record.id, record.clone());
            Ok(())
        } else {
            Err(crate::Error::not_found(format!(
                "Record with ID {} not found",
                record.id
            )))
        }
    }

    async fn delete(&self, id: &Uuid) -> Result<bool> {
        Ok(self.records.remove(id).is_some())
    }

    async fn list(&self) -> Result<Vec<Uuid>> {
        Ok(self.records.iter().map(|r| *r.key()).collect())
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.records.len())
    }

    async fn clear(&self) -> Result<()> {
        self.records.clear();
        Ok(())
    }
}

/// Storage with caching layer
#[derive(Debug)]
pub struct CachedStorage<S: Storage> {
    inner: S,
    cache: Arc<DashMap<Uuid, Record>>,
    cache_size: usize,
}

impl<S: Storage> CachedStorage<S> {
    /// Create a new cached storage
    pub fn new(inner: S, cache_size: usize) -> Self {
        Self {
            inner,
            cache: Arc::new(DashMap::new()),
            cache_size,
        }
    }

    /// Evict oldest entries if cache is full
    fn evict_if_needed(&self) {
        if self.cache.len() >= self.cache_size {
            // Simple eviction: remove first entry
            // In production, use LRU or similar
            if let Some(entry) = self.cache.iter().next() {
                let key = *entry.key();
                drop(entry);
                self.cache.remove(&key);
            }
        }
    }
}

#[async_trait]
impl<S: Storage> Storage for CachedStorage<S> {
    async fn store(&self, record: &Record) -> Result<()> {
        self.evict_if_needed();
        self.cache.insert(record.id, record.clone());
        self.inner.store(record).await
    }

    async fn get(&self, id: &Uuid) -> Result<Option<Record>> {
        // Check cache first
        if let Some(record) = self.cache.get(id) {
            return Ok(Some(record.clone()));
        }

        // Fetch from storage
        let record = self.inner.get(id).await?;
        
        // Update cache
        if let Some(ref r) = record {
            self.evict_if_needed();
            self.cache.insert(*id, r.clone());
        }

        Ok(record)
    }

    async fn update(&self, record: &Record) -> Result<()> {
        self.cache.insert(record.id, record.clone());
        self.inner.update(record).await
    }

    async fn delete(&self, id: &Uuid) -> Result<bool> {
        self.cache.remove(id);
        self.inner.delete(id).await
    }

    async fn list(&self) -> Result<Vec<Uuid>> {
        self.inner.list().await
    }

    async fn count(&self) -> Result<usize> {
        self.inner.count().await
    }

    async fn clear(&self) -> Result<()> {
        self.cache.clear();
        self.inner.clear().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();
        let record = Record::new("test_key", "test_value");
        let id = record.id;

        // Store
        storage.store(&record).await.unwrap();
        assert_eq!(storage.count().await.unwrap(), 1);

        // Get
        let retrieved = storage.get(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().key, "test_key");

        // Update
        let mut updated = record.clone();
        updated.update_value("new_value");
        storage.update(&updated).await.unwrap();

        // Delete
        let deleted = storage.delete(&id).await.unwrap();
        assert!(deleted);
        assert_eq!(storage.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_cached_storage() {
        let inner = InMemoryStorage::new();
        let storage = CachedStorage::new(inner, 10);
        
        let record = Record::new("test_key", "test_value");
        let id = record.id;

        storage.store(&record).await.unwrap();
        
        // First get (from storage)
        let retrieved1 = storage.get(&id).await.unwrap();
        assert!(retrieved1.is_some());

        // Second get (from cache)
        let retrieved2 = storage.get(&id).await.unwrap();
        assert!(retrieved2.is_some());
    }
}
