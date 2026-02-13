//! Complete example demonstrating enterprise data processor features

use enterprise_data_processor::{
    pipeline::PipelineBuilder,
    processor::Processor,
    record::Record,
    storage::{CachedStorage, InMemoryStorage},
    transform::{EnrichTransform, NormalizeTransform},
    validation::{NonEmptyStringRule, NumericRangeRule, RequiredFieldRule, Validator},
    ProcessorConfig,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the library with structured logging
    enterprise_data_processor::init();

    info!("Starting enterprise data processor example");

    // Example 1: Basic single record processing
    example_basic_processing().await?;

    // Example 2: Batch processing
    example_batch_processing().await?;

    // Example 3: Complete pipeline with validation, transforms, and storage
    example_complete_pipeline().await?;

    // Example 4: Custom configuration
    example_custom_config().await?;

    info!("All examples completed successfully");
    Ok(())
}

async fn example_basic_processing() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Example 1: Basic Processing ===");

    let processor = Processor::new(ProcessorConfig::default())?;

    let record = Record::new(
        "order_12345",
        json!({
            "customer_id": "cust_789",
            "items": [
                {"sku": "PROD-001", "quantity": 2, "price": 29.99},
                {"sku": "PROD-002", "quantity": 1, "price": 49.99}
            ],
            "total": 109.97,
            "status": "pending"
        }),
    );

    let result = processor.process(record).await?;

    info!(
        "Processed order in {}ms, success: {}",
        result.duration_ms, result.success
    );

    Ok(())
}

async fn example_batch_processing() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Example 2: Batch Processing ===");

    let processor = Processor::new(ProcessorConfig::default())?;

    // Create batch of user records
    let records: Vec<_> = (1..=20)
        .map(|i| {
            Record::new(
                format!("user_{:03}", i),
                json!({
                    "user_id": format!("user_{:03}", i),
                    "name": format!("User {}", i),
                    "email": format!("user{}@example.com", i),
                    "created_at": "2024-01-15T10:00:00Z",
                    "active": true
                }),
            )
        })
        .collect();

    info!("Processing batch of {} records", records.len());

    let results = processor.process_batch(records).await?;

    let success_count = results.iter().filter(|r| r.success).count();
    let avg_duration: u64 = results.iter().map(|r| r.duration_ms).sum::<u64>() / results.len() as u64;

    info!(
        "Batch complete: {}/{} successful, average duration: {}ms",
        success_count,
        results.len(),
        avg_duration
    );

    Ok(())
}

async fn example_complete_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Example 3: Complete Pipeline ===");

    // 1. Setup validation
    let mut validator = Validator::new();
    validator.add_rule(Arc::new(RequiredFieldRule::new("email")));
    validator.add_rule(Arc::new(RequiredFieldRule::new("name")));
    validator.add_rule(Arc::new(NonEmptyStringRule::new("name")));
    validator.add_rule(Arc::new(NumericRangeRule::new("age").min(0.0).max(150.0)));

    // 2. Setup transformations
    let normalize = Arc::new(NormalizeTransform::new(
        "normalize_names",
        vec!["name".to_string(), "email".to_string()],
    ));

    let enrich = Arc::new(EnrichTransform::new(
        "add_metadata",
        "metadata",
        json!({
            "processed_at": "2024-01-15T10:30:00Z",
            "processor_version": "1.0.0",
            "environment": "production"
        }),
    ));

    // 3. Setup storage with caching
    let base_storage = InMemoryStorage::new();
    let cached_storage = Arc::new(CachedStorage::new(base_storage, 100));

    // 4. Build pipeline
    let pipeline = PipelineBuilder::new("user_onboarding")
        .validate(Arc::new(validator))
        .transform(normalize)
        .transform(enrich)
        .store(cached_storage.clone())
        .build();

    info!("Pipeline configured with {} stages", pipeline.stage_count());

    // 5. Process records through pipeline
    let test_records = vec![
        Record::new(
            "user_001",
            json!({
                "name": "  ALICE JOHNSON  ",
                "email": "ALICE@EXAMPLE.COM",
                "age": 28
            }),
        ),
        Record::new(
            "user_002",
            json!({
                "name": "  bob smith  ",
                "email": "BOB@EXAMPLE.COM",
                "age": 35
            }),
        ),
    ];

    for record in test_records {
        match pipeline.execute(record).await {
            Ok(result) => {
                info!("Pipeline success for {}", result.key);
                info!("  Normalized name: {}", result.value["name"]);
                info!("  Normalized email: {}", result.value["email"]);
                info!("  Metadata added: {}", result.value["metadata"]);
            }
            Err(e) => {
                info!("Pipeline failed: {}", e);
            }
        }
    }

    info!("Storage contains {} records", cached_storage.count().await?);

    Ok(())
}

async fn example_custom_config() -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Example 4: Custom Configuration ===");

    let config = ProcessorConfig::builder()
        .max_batch_size(500)
        .max_workers(16)
        .operation_timeout(Duration::from_secs(60))
        .enable_metrics(true)
        .enable_tracing(true)
        .buffer_size(2000)
        .enable_compression(false)
        .build();

    info!("Configuration:");
    info!("  Max batch size: {}", config.max_batch_size);
    info!("  Max workers: {}", config.max_workers);
    info!("  Operation timeout: {:?}", config.operation_timeout);
    info!("  Metrics enabled: {}", config.enable_metrics);

    let processor = Processor::new(config)?;

    // Demonstrate high-concurrency processing
    let mut handles = vec![];

    for i in 0..100 {
        let processor = processor.clone();
        let handle = tokio::spawn(async move {
            let record = Record::new(
                format!("concurrent_{}", i),
                json!({"index": i, "timestamp": "2024-01-15T10:00:00Z"}),
            );
            processor.process(record).await
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(result)) = handle.await {
            if result.success {
                success_count += 1;
            }
        }
    }

    info!("Concurrent processing: {}/100 successful", success_count);
    info!("Total records stored: {}", processor.total_records());
    info!("Active tasks: {}", processor.active_tasks().await);

    Ok(())
}
