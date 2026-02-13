//! Main data processor implementation

use crate::{config::ProcessorConfig, error::Result, record::Record, Error};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Main data processor
#[derive(Debug)]
pub struct Processor {
    config: ProcessorConfig,
    state: Arc<ProcessorState>,
    transform_registry: Arc<TransformRegistry>,
}

/// Internal processor state
#[derive(Debug)]
struct ProcessorState {
    records: DashMap<Uuid, Record>,
    active_tasks: RwLock<u64>,
    semaphore: Semaphore,
}

/// Registry for transformation functions
#[derive(Debug, Default)]
struct TransformRegistry {
    transforms: DashMap<String, Arc<dyn Transform>>,
}

/// Trait for record transformations
#[async_trait]
pub trait Transform: Send + Sync + std::fmt::Debug {
    /// Transform a record
    async fn transform(&self, record: Record) -> Result<Record>;
    
    /// Name of the transformation
    fn name(&self) -> &str;
}

/// Result of processing a record
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// The processed record
    pub record: Record,
    
    /// Processing duration in milliseconds
    pub duration_ms: u64,
    
    /// Whether the processing was successful
    pub success: bool,
    
    /// Error message if processing failed
    pub error: Option<String>,
}

impl Processor {
    /// Create a new processor with the given configuration
    pub fn new(config: ProcessorConfig) -> Result<Self> {
        config.validate()?;
        
        info!(
            max_workers = config.max_workers,
            max_batch_size = config.max_batch_size,
            "Creating new processor"
        );
        
        Ok(Self {
            config: config.clone(),
            state: Arc::new(ProcessorState {
                records: DashMap::new(),
                active_tasks: RwLock::new(0),
                semaphore: Semaphore::new(config.max_workers),
            }),
            transform_registry: Arc::new(TransformRegistry::default()),
        })
    }

    /// Process a single record
    pub async fn process(&self, mut record: Record) -> Result<ProcessingResult> {
        let start = std::time::Instant::now();
        
        debug!(record_id = %record.id, key = %record.key, "Processing record");
        
        // Acquire semaphore to limit concurrency
        let _permit = self
            .state
            .semaphore
            .acquire()
            .await
            .map_err(|e| Error::concurrency(format!("Failed to acquire permit: {}", e)))?;
        
        // Increment active tasks
        {
            let mut active = self.state.active_tasks.write().await;
            *active += 1;
        }
        
        // Mark record as processing
        record.mark_processing();
        
        // Store record
        self.state.records.insert(record.id, record.clone());
        
        // Perform actual processing
        let result = self.process_internal(record).await;
        
        // Decrement active tasks
        {
            let mut active = self.state.active_tasks.write().await;
            *active -= 1;
        }
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        match result {
            Ok(processed_record) => {
                info!(
                    record_id = %processed_record.id,
                    duration_ms,
                    "Record processed successfully"
                );
                
                Ok(ProcessingResult {
                    record: processed_record,
                    duration_ms,
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                error!(error = %e, duration_ms, "Record processing failed");
                
                Ok(ProcessingResult {
                    record: self
                        .state
                        .records
                        .get(&record.id)
                        .map(|r| r.clone())
                        .unwrap_or(record),
                    duration_ms,
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Process a batch of records
    pub async fn process_batch(&self, records: Vec<Record>) -> Result<Vec<ProcessingResult>> {
        info!(count = records.len(), "Processing batch of records");
        
        if records.len() > self.config.max_batch_size {
            return Err(Error::processing(format!(
                "Batch size {} exceeds maximum of {}",
                records.len(),
                self.config.max_batch_size
            )));
        }
        
        let mut handles = Vec::new();
        
        for record in records {
            let processor = self.clone();
            let handle = tokio::spawn(async move { processor.process(record).await });
            handles.push(handle);
        }
        
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => {
                    warn!(error = %e, "Failed to process record in batch");
                }
                Err(e) => {
                    error!(error = %e, "Task panicked while processing record");
                }
            }
        }
        
        Ok(results)
    }

    /// Internal processing logic
    async fn process_internal(&self, mut record: Record) -> Result<Record> {
        // Apply all registered transforms
        for transform_ref in self.transform_registry.transforms.iter() {
            let transform = transform_ref.value();
            debug!(
                record_id = %record.id,
                transform = transform.name(),
                "Applying transform"
            );
            
            record = transform.transform(record).await?;
        }
        
        // Mark as completed
        record.mark_completed();
        
        // Update stored record
        self.state.records.insert(record.id, record.clone());
        
        Ok(record)
    }

    /// Register a transform
    pub fn register_transform(&self, transform: Arc<dyn Transform>) {
        let name = transform.name().to_string();
        info!(transform = %name, "Registering transform");
        self.transform_registry.transforms.insert(name, transform);
    }

    /// Get a record by ID
    pub fn get_record(&self, id: &Uuid) -> Option<Record> {
        self.state.records.get(id).map(|r| r.clone())
    }

    /// Get current number of active tasks
    pub async fn active_tasks(&self) -> u64 {
        *self.state.active_tasks.read().await
    }

    /// Get total number of stored records
    pub fn total_records(&self) -> usize {
        self.state.records.len()
    }

    /// Clear all stored records
    pub fn clear_records(&self) {
        self.state.records.clear();
    }

    /// Get configuration
    pub fn config(&self) -> &ProcessorConfig {
        &self.config
    }
}

impl Clone for Processor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: Arc::clone(&self.state),
            transform_registry: Arc::clone(&self.transform_registry),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_processor_creation() {
        let config = ProcessorConfig::default();
        let processor = Processor::new(config).unwrap();
        assert_eq!(processor.total_records(), 0);
    }

    #[tokio::test]
    async fn test_process_single_record() {
        let processor = Processor::new(ProcessorConfig::default()).unwrap();
        let record = Record::new("test_key", "test_value");
        
        let result = processor.process(record).await.unwrap();
        assert!(result.success);
        assert_eq!(processor.total_records(), 1);
    }

    #[tokio::test]
    async fn test_process_batch() {
        let processor = Processor::new(ProcessorConfig::default()).unwrap();
        
        let records: Vec<_> = (0..10)
            .map(|i| Record::new(format!("key_{}", i), format!("value_{}", i)))
            .collect();
        
        let results = processor.process_batch(records).await.unwrap();
        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|r| r.success));
    }
}
