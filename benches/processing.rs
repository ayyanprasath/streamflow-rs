use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use enterprise_data_processor::{
    processor::Processor,
    record::Record,
    ProcessorConfig,
};
use serde_json::json;
use std::time::Duration;

fn bench_single_record_processing(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let processor = Processor::new(ProcessorConfig::default()).unwrap();

    c.bench_function("process_single_record", |b| {
        b.to_async(&rt).iter(|| async {
            let record = Record::new("test_key", black_box(json!({"value": "test"})));
            processor.process(record).await.unwrap()
        });
    });
}

fn bench_batch_processing(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("batch_processing");
    
    for batch_size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &size| {
                let processor = Processor::new(ProcessorConfig::default()).unwrap();
                
                b.to_async(&rt).iter(|| async {
                    let records: Vec<_> = (0..size)
                        .map(|i| {
                            Record::new(
                                format!("key_{}", i),
                                black_box(json!({"value": i}))
                            )
                        })
                        .collect();
                    
                    processor.process_batch(records).await.unwrap()
                });
            },
        );
    }
    
    group.finish();
}

fn bench_concurrent_processing(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_processing");
    
    for concurrency in [4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &workers| {
                let config = ProcessorConfig::builder()
                    .max_workers(workers)
                    .build();
                let processor = std::sync::Arc::new(Processor::new(config).unwrap());
                
                b.to_async(&rt).iter(|| {
                    let processor = processor.clone();
                    async move {
                        let mut handles = vec![];
                        
                        for i in 0..100 {
                            let processor = processor.clone();
                            let handle = tokio::spawn(async move {
                                let record = Record::new(
                                    format!("key_{}", i),
                                    black_box(json!({"value": i}))
                                );
                                processor.process(record).await
                            });
                            handles.push(handle);
                        }
                        
                        for handle in handles {
                            handle.await.unwrap().unwrap();
                        }
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn bench_record_operations(c: &mut Criterion) {
    c.bench_function("record_creation", |b| {
        b.iter(|| {
            Record::new(
                black_box("test_key"),
                black_box(json!({"value": "test"}))
            )
        });
    });

    c.bench_function("record_update", |b| {
        let mut record = Record::new("test_key", json!({"value": "initial"}));
        b.iter(|| {
            record.update_value(black_box(json!({"value": "updated"})));
        });
    });

    c.bench_function("record_tag_operations", |b| {
        let mut record = Record::new("test_key", json!({"value": "test"}));
        b.iter(|| {
            record.add_tag(black_box("env"), black_box("prod"));
            record.has_tag("env");
            record.remove_tag("env");
        });
    });
}

fn bench_validation(c: &mut Criterion) {
    use enterprise_data_processor::validation::{RequiredFieldRule, Validator};
    use std::sync::Arc;
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut validator = Validator::new();
    validator.add_rule(Arc::new(RequiredFieldRule::new("name")));
    validator.add_rule(Arc::new(RequiredFieldRule::new("email")));
    
    c.bench_function("validation_success", |b| {
        let record = Record::new(
            "test",
            json!({
                "name": "John Doe",
                "email": "john@example.com"
            })
        );
        
        b.to_async(&rt).iter(|| async {
            validator.validate(black_box(&record)).await.unwrap()
        });
    });
}

fn bench_storage_operations(c: &mut Criterion) {
    use enterprise_data_processor::storage::{InMemoryStorage, Storage};
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("storage_write", |b| {
        let storage = InMemoryStorage::new();
        
        b.to_async(&rt).iter(|| {
            let record = Record::new(
                format!("key_{}", rand::random::<u32>()),
                black_box(json!({"value": "test"}))
            );
            
            async {
                storage.store(black_box(&record)).await.unwrap()
            }
        });
    });

    c.bench_function("storage_read", |b| {
        let storage = InMemoryStorage::new();
        let record = Record::new("test_key", json!({"value": "test"}));
        let id = record.id;
        
        rt.block_on(async {
            storage.store(&record).await.unwrap();
        });
        
        b.to_async(&rt).iter(|| async {
            storage.get(black_box(&id)).await.unwrap()
        });
    });
}

criterion_group!(
    benches,
    bench_single_record_processing,
    bench_batch_processing,
    bench_concurrent_processing,
    bench_record_operations,
    bench_validation,
    bench_storage_operations,
);

criterion_main!(benches);
