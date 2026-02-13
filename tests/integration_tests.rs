//! Integration tests for the enterprise data processor

use enterprise_data_processor::{
    pipeline::PipelineBuilder,
    processor::Processor,
    record::Record,
    storage::InMemoryStorage,
    transform::{EnrichTransform, NormalizeTransform},
    validation::{NonEmptyStringRule, RequiredFieldRule, Validator},
    ProcessorConfig,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_end_to_end_processing() {
    // Initialize
    enterprise_data_processor::init();

    // Create configuration
    let config = ProcessorConfig::builder()
        .max_batch_size(50)
        .max_workers(4)
        .operation_timeout(Duration::from_secs(5))
        .build();

    // Create processor
    let processor = Processor::new(config).unwrap();

    // Create and process a record
    let record = Record::new(
        "user_001",
        json!({
            "name": "Alice Johnson",
            "email": "alice@example.com",
            "age": 28
        }),
    );

    let result = processor.process(record).await.unwrap();
    assert!(result.success);
    assert_eq!(processor.total_records(), 1);
}

#[tokio::test]
async fn test_batch_processing_with_mixed_results() {
    let processor = Processor::new(ProcessorConfig::default()).unwrap();

    // Create batch with valid and invalid records
    let records = vec![
        Record::new("valid_1", json!({"data": "value1"})),
        Record::new("valid_2", json!({"data": "value2"})),
        Record::new("valid_3", json!({"data": "value3"})),
    ];

    let results = processor.process_batch(records).await.unwrap();
    
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.success));
}

#[tokio::test]
async fn test_pipeline_with_validation_and_transforms() {
    // Setup validator
    let mut validator = Validator::new();
    validator.add_rule(Arc::new(RequiredFieldRule::new("email")));
    validator.add_rule(Arc::new(NonEmptyStringRule::new("name")));

    // Setup transforms
    let normalize = Arc::new(NormalizeTransform::new(
        "normalize",
        vec!["name".to_string()],
    ));

    let enrich = Arc::new(EnrichTransform::new(
        "enrich",
        "processed_at",
        json!("2024-01-15T10:00:00Z"),
    ));

    // Setup storage
    let storage = Arc::new(InMemoryStorage::new());

    // Build pipeline
    let pipeline = PipelineBuilder::new("user_pipeline")
        .validate(Arc::new(validator))
        .transform(normalize)
        .transform(enrich)
        .store(storage.clone())
        .build();

    // Process record
    let record = Record::new(
        "user_123",
        json!({
            "name": "  JOHN DOE  ",
            "email": "john@example.com"
        }),
    );

    let result = pipeline.execute(record).await.unwrap();

    // Verify transformations
    assert_eq!(result.value["name"], json!("john doe"));
    assert_eq!(result.value["processed_at"], json!("2024-01-15T10:00:00Z"));

    // Verify storage
    assert_eq!(storage.count().await.unwrap(), 1);
}

#[tokio::test]
async fn test_pipeline_validation_failure() {
    let mut validator = Validator::new();
    validator.add_rule(Arc::new(RequiredFieldRule::new("required_field")));

    let pipeline = PipelineBuilder::new("validation_test")
        .validate(Arc::new(validator))
        .build();

    // Record missing required field
    let record = Record::new("test", json!({"other_field": "value"}));

    let result = pipeline.execute(record).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_processing() {
    let processor = Arc::new(Processor::new(ProcessorConfig::default()).unwrap());
    let mut handles = vec![];

    // Spawn multiple concurrent tasks
    for i in 0..20 {
        let processor_clone = Arc::clone(&processor);
        let handle = tokio::spawn(async move {
            let record = Record::new(format!("key_{}", i), format!("value_{}", i));
            processor_clone.process(record).await
        });
        handles.push(handle);
    }

    // Wait for all tasks
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(result)) = handle.await {
            if result.success {
                success_count += 1;
            }
        }
    }

    assert_eq!(success_count, 20);
}

#[tokio::test]
async fn test_record_lifecycle() {
    let mut record = Record::new("test_key", json!({"initial": "value"}));

    // Check initial state
    assert_eq!(record.metadata.process_count, 0);
    assert_eq!(record.metadata.failure_count, 0);

    // Mark as processing
    record.mark_processing();
    assert_eq!(record.metadata.process_count, 1);

    // Mark as completed
    record.mark_completed();
    assert!(record.metadata.last_error.is_none());

    // Mark as processing again
    record.mark_processing();
    assert_eq!(record.metadata.process_count, 2);

    // Mark as failed
    record.mark_failed("Test error");
    assert_eq!(record.metadata.failure_count, 1);
    assert!(record.metadata.last_error.is_some());
}

#[tokio::test]
async fn test_storage_operations() {
    let storage = InMemoryStorage::new();
    
    // Create records
    let record1 = Record::new("key1", "value1");
    let record2 = Record::new("key2", "value2");
    
    let id1 = record1.id;
    let id2 = record2.id;

    // Store records
    storage.store(&record1).await.unwrap();
    storage.store(&record2).await.unwrap();

    // Verify count
    assert_eq!(storage.count().await.unwrap(), 2);

    // Retrieve records
    let retrieved = storage.get(&id1).await.unwrap();
    assert!(retrieved.is_some());

    // List all
    let ids = storage.list().await.unwrap();
    assert_eq!(ids.len(), 2);

    // Delete one
    let deleted = storage.delete(&id1).await.unwrap();
    assert!(deleted);
    assert_eq!(storage.count().await.unwrap(), 1);

    // Clear all
    storage.clear().await.unwrap();
    assert_eq!(storage.count().await.unwrap(), 0);
}

#[tokio::test]
async fn test_error_handling_and_retry_logic() {
    use enterprise_data_processor::Error;

    // Test error types
    let config_error = Error::config("test config error");
    assert_eq!(config_error.code(), "CONFIG_ERROR");
    assert!(!config_error.is_retryable());

    let timeout_error = Error::timeout("test timeout");
    assert_eq!(timeout_error.code(), "TIMEOUT");
    assert!(timeout_error.is_retryable());

    // Test retry configuration
    let config = ProcessorConfig::default();
    let backoff = config.retry_config.calculate_backoff(0);
    assert_eq!(backoff, config.retry_config.initial_backoff);

    let backoff2 = config.retry_config.calculate_backoff(1);
    assert!(backoff2 > config.retry_config.initial_backoff);
}

#[tokio::test]
async fn test_record_tags() {
    let mut record = Record::new("test", "value");

    // Add tags
    record.add_tag("environment", "production");
    record.add_tag("region", "us-west-2");
    record.add_tag("priority", "high");

    // Check tags
    assert!(record.has_tag("environment"));
    assert_eq!(record.get_tag("environment"), Some(&"production".to_string()));

    // Remove tag
    let removed = record.remove_tag("priority");
    assert_eq!(removed, Some("high".to_string()));
    assert!(!record.has_tag("priority"));
}

#[tokio::test]
async fn test_processor_statistics() {
    let processor = Processor::new(ProcessorConfig::default()).unwrap();

    // Initial state
    assert_eq!(processor.total_records(), 0);
    assert_eq!(processor.active_tasks().await, 0);

    // Process some records
    let records: Vec<_> = (0..5)
        .map(|i| Record::new(format!("key_{}", i), format!("value_{}", i)))
        .collect();

    for record in records {
        processor.process(record).await.unwrap();
    }

    assert_eq!(processor.total_records(), 5);

    // Clear records
    processor.clear_records();
    assert_eq!(processor.total_records(), 0);
}
