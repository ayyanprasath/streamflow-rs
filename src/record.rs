//! Core data record structure

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a single data record in the processing pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    /// Unique identifier for the record
    pub id: Uuid,
    
    /// Record key
    pub key: String,
    
    /// Record value (JSON-serializable data)
    pub value: serde_json::Value,
    
    /// Metadata associated with the record
    pub metadata: RecordMetadata,
    
    /// Custom tags for categorization and filtering
    pub tags: HashMap<String, String>,
}

/// Metadata for tracking record lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordMetadata {
    /// Timestamp when the record was created
    pub created_at: DateTime<Utc>,
    
    /// Timestamp when the record was last updated
    pub updated_at: DateTime<Utc>,
    
    /// Source system or component that created the record
    pub source: String,
    
    /// Version number for optimistic locking
    pub version: u64,
    
    /// Processing status
    pub status: RecordStatus,
    
    /// Number of times the record has been processed
    pub process_count: u32,
    
    /// Number of times processing has failed
    pub failure_count: u32,
    
    /// Last error message, if any
    pub last_error: Option<String>,
}

/// Processing status of a record
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordStatus {
    /// Record is pending processing
    Pending,
    
    /// Record is currently being processed
    Processing,
    
    /// Record has been successfully processed
    Completed,
    
    /// Record processing failed
    Failed,
    
    /// Record has been archived
    Archived,
}

impl Record {
    /// Create a new record with the given key and value
    pub fn new(key: impl Into<String>, value: impl Serialize) -> Self {
        let now = Utc::now();
        
        Self {
            id: Uuid::new_v4(),
            key: key.into(),
            value: serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
            metadata: RecordMetadata {
                created_at: now,
                updated_at: now,
                source: "default".to_string(),
                version: 1,
                status: RecordStatus::Pending,
                process_count: 0,
                failure_count: 0,
                last_error: None,
            },
            tags: HashMap::new(),
        }
    }

    /// Create a builder for constructing records
    pub fn builder() -> RecordBuilder {
        RecordBuilder::new()
    }

    /// Update the record's value
    pub fn update_value(&mut self, value: impl Serialize) {
        self.value = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
        self.metadata.updated_at = Utc::now();
        self.metadata.version += 1;
    }

    /// Mark the record as processing
    pub fn mark_processing(&mut self) {
        self.metadata.status = RecordStatus::Processing;
        self.metadata.updated_at = Utc::now();
        self.metadata.process_count += 1;
    }

    /// Mark the record as completed
    pub fn mark_completed(&mut self) {
        self.metadata.status = RecordStatus::Completed;
        self.metadata.updated_at = Utc::now();
        self.metadata.last_error = None;
    }

    /// Mark the record as failed
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.metadata.status = RecordStatus::Failed;
        self.metadata.updated_at = Utc::now();
        self.metadata.failure_count += 1;
        self.metadata.last_error = Some(error.into());
    }

    /// Add a tag to the record
    pub fn add_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
        self.metadata.updated_at = Utc::now();
    }

    /// Remove a tag from the record
    pub fn remove_tag(&mut self, key: &str) -> Option<String> {
        let result = self.tags.remove(key);
        if result.is_some() {
            self.metadata.updated_at = Utc::now();
        }
        result
    }

    /// Check if the record has a specific tag
    pub fn has_tag(&self, key: &str) -> bool {
        self.tags.contains_key(key)
    }

    /// Get a tag value
    pub fn get_tag(&self, key: &str) -> Option<&String> {
        self.tags.get(key)
    }

    /// Deserialize the value into a specific type
    pub fn deserialize_value<T: for<'de> Deserialize<'de>>(&self) -> crate::Result<T> {
        serde_json::from_value(self.value.clone()).map_err(Into::into)
    }
}

/// Builder for constructing records
#[derive(Debug)]
pub struct RecordBuilder {
    id: Option<Uuid>,
    key: Option<String>,
    value: Option<serde_json::Value>,
    source: Option<String>,
    tags: HashMap<String, String>,
}

impl RecordBuilder {
    /// Create a new record builder
    pub fn new() -> Self {
        Self {
            id: None,
            key: None,
            value: None,
            source: None,
            tags: HashMap::new(),
        }
    }

    /// Set the record ID
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the record key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Set the record value
    pub fn value(mut self, value: impl Serialize) -> Self {
        self.value = Some(serde_json::to_value(value).unwrap_or(serde_json::Value::Null));
        self
    }

    /// Set the source
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Add a tag
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Build the record
    pub fn build(self) -> crate::Result<Record> {
        let key = self.key.ok_or_else(|| crate::Error::config("Record key is required"))?;
        let value = self.value.ok_or_else(|| crate::Error::config("Record value is required"))?;
        
        let now = Utc::now();
        
        Ok(Record {
            id: self.id.unwrap_or_else(Uuid::new_v4),
            key,
            value,
            metadata: RecordMetadata {
                created_at: now,
                updated_at: now,
                source: self.source.unwrap_or_else(|| "default".to_string()),
                version: 1,
                status: RecordStatus::Pending,
                process_count: 0,
                failure_count: 0,
                last_error: None,
            },
            tags: self.tags,
        })
    }
}

impl Default for RecordBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_creation() {
        let record = Record::new("test_key", "test_value");
        assert_eq!(record.key, "test_key");
        assert_eq!(record.metadata.status, RecordStatus::Pending);
    }

    #[test]
    fn test_record_builder() {
        let record = Record::builder()
            .key("test_key")
            .value("test_value")
            .source("test_source")
            .tag("env", "production")
            .build()
            .unwrap();
        
        assert_eq!(record.key, "test_key");
        assert_eq!(record.metadata.source, "test_source");
        assert_eq!(record.tags.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_record_status_updates() {
        let mut record = Record::new("test", "value");
        
        record.mark_processing();
        assert_eq!(record.metadata.status, RecordStatus::Processing);
        assert_eq!(record.metadata.process_count, 1);
        
        record.mark_completed();
        assert_eq!(record.metadata.status, RecordStatus::Completed);
        
        record.mark_failed("test error");
        assert_eq!(record.metadata.status, RecordStatus::Failed);
        assert_eq!(record.metadata.failure_count, 1);
    }

    #[test]
    fn test_record_tags() {
        let mut record = Record::new("test", "value");
        
        record.add_tag("env", "dev");
        assert!(record.has_tag("env"));
        assert_eq!(record.get_tag("env"), Some(&"dev".to_string()));
        
        record.remove_tag("env");
        assert!(!record.has_tag("env"));
    }
}
