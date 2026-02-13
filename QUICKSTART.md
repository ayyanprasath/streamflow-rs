# Quick Start Guide

## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
enterprise-data-processor = "1.0"
tokio = { version = "1.35", features = ["full"] }
serde_json = "1.0"
```

## 5-Minute Tutorial

### 1. Basic Processing

```rust
use enterprise_data_processor::{Processor, ProcessorConfig, Record};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize
    enterprise_data_processor::init();
    
    // Create processor
    let processor = Processor::new(ProcessorConfig::default())?;
    
    // Process a record
    let record = Record::new("user_123", json!({
        "name": "Alice",
        "email": "alice@example.com"
    }));
    
    let result = processor.process(record).await?;
    println!("Success: {}", result.success);
    
    Ok(())
}
```

### 2. Batch Processing

```rust
// Create batch
let records: Vec<_> = (0..100)
    .map(|i| Record::new(format!("key_{}", i), json!({"value": i})))
    .collect();

// Process batch
let results = processor.process_batch(records).await?;
println!("Processed {} records", results.len());
```

### 3. With Validation

```rust
use enterprise_data_processor::validation::{Validator, RequiredFieldRule};
use std::sync::Arc;

// Create validator
let mut validator = Validator::new();
validator.add_rule(Arc::new(RequiredFieldRule::new("email")));

// Validate
let record = Record::new("user", json!({"email": "test@example.com"}));
validator.validate(&record).await?;
```

### 4. With Transforms

```rust
use enterprise_data_processor::transform::EnrichTransform;

// Create transform
let transform = Arc::new(EnrichTransform::new(
    "add_timestamp",
    "timestamp",
    json!("2024-01-15T10:00:00Z")
));

// Register with processor
processor.register_transform(transform);

// Now all records will be enriched
let result = processor.process(record).await?;
```

### 5. Complete Pipeline

```rust
use enterprise_data_processor::{
    pipeline::PipelineBuilder,
    storage::InMemoryStorage,
};

// Build pipeline
let pipeline = PipelineBuilder::new("my_pipeline")
    .validate(Arc::new(validator))
    .transform(transform)
    .store(Arc::new(InMemoryStorage::new()))
    .build();

// Execute
let result = pipeline.execute(record).await?;
```

## Common Patterns

### Error Handling

```rust
use enterprise_data_processor::Error;

match processor.process(record).await {
    Ok(result) => println!("Success: {:?}", result),
    Err(Error::Validation(e)) => println!("Invalid: {}", e),
    Err(e) if e.is_retryable() => {
        // Retry logic
    }
    Err(e) => println!("Error: {}", e),
}
```

### Custom Configuration

```rust
use std::time::Duration;

let config = ProcessorConfig::builder()
    .max_batch_size(500)
    .max_workers(8)
    .operation_timeout(Duration::from_secs(30))
    .enable_metrics(true)
    .build();
```

### Record Metadata

```rust
let mut record = Record::new("key", value);

// Add tags
record.add_tag("env", "production");
record.add_tag("priority", "high");

// Update value
record.update_value(new_value);

// Check status
println!("Status: {:?}", record.metadata.status);
```

## Next Steps

- Read the [full documentation](README.md)
- Check out [examples](examples/)
- Review [architecture](docs/ARCHITECTURE.md)
- Run benchmarks: `cargo bench`
- Explore [integration tests](tests/)

## Troubleshooting

### "Validation failed"
- Check that required fields are present
- Verify field types match validation rules

### "Operation timed out"
- Increase `operation_timeout` in config
- Check for blocking operations in transforms

### "Too many active tasks"
- Reduce batch size
- Increase `max_workers` in config

### Performance issues
- Run benchmarks to identify bottlenecks
- Enable metrics to monitor performance
- Consider batch processing for throughput

## Resources

- Documentation: https://docs.rs/enterprise-data-processor
- GitHub: https://github.com/yourorg/enterprise-data-processor
- Examples: https://github.com/yourorg/enterprise-data-processor/tree/main/examples
