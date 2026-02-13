//! Data processing pipeline module

use crate::{processor::Transform, record::Record, storage::Storage, validation::Validator, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info};

/// A data processing pipeline that chains operations
#[derive(Debug)]
pub struct Pipeline {
    name: String,
    stages: Vec<Arc<dyn PipelineStage>>,
}

/// Trait for pipeline stages
#[async_trait]
pub trait PipelineStage: Send + Sync + std::fmt::Debug {
    /// Execute the stage on a record
    async fn execute(&self, record: Record) -> Result<Record>;
    
    /// Name of the stage
    fn name(&self) -> &str;
}

impl Pipeline {
    /// Create a new pipeline
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stages: Vec::new(),
        }
    }

    /// Add a stage to the pipeline
    pub fn add_stage(&mut self, stage: Arc<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    /// Execute the pipeline on a record
    pub async fn execute(&self, mut record: Record) -> Result<Record> {
        info!(
            pipeline = %self.name,
            record_id = %record.id,
            stages = self.stages.len(),
            "Executing pipeline"
        );

        for (idx, stage) in self.stages.iter().enumerate() {
            debug!(
                pipeline = %self.name,
                stage = stage.name(),
                stage_index = idx,
                "Executing stage"
            );

            record = stage.execute(record).await?;
        }

        Ok(record)
    }

    /// Get pipeline name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get number of stages
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }
}

/// Builder for creating pipelines
#[derive(Debug)]
pub struct PipelineBuilder {
    pipeline: Pipeline,
}

impl PipelineBuilder {
    /// Create a new pipeline builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            pipeline: Pipeline::new(name),
        }
    }

    /// Add a validation stage
    pub fn validate(mut self, validator: Arc<Validator>) -> Self {
        self.pipeline.add_stage(Arc::new(ValidationStage::new(validator)));
        self
    }

    /// Add a transformation stage
    pub fn transform(mut self, transform: Arc<dyn Transform>) -> Self {
        self.pipeline.add_stage(Arc::new(TransformStage::new(transform)));
        self
    }

    /// Add a storage stage
    pub fn store(mut self, storage: Arc<dyn Storage>) -> Self {
        self.pipeline.add_stage(Arc::new(StorageStage::new(storage)));
        self
    }

    /// Build the pipeline
    pub fn build(self) -> Pipeline {
        self.pipeline
    }
}

/// Validation pipeline stage
#[derive(Debug)]
struct ValidationStage {
    validator: Arc<Validator>,
}

impl ValidationStage {
    fn new(validator: Arc<Validator>) -> Self {
        Self { validator }
    }
}

#[async_trait]
impl PipelineStage for ValidationStage {
    async fn execute(&self, record: Record) -> Result<Record> {
        self.validator.validate(&record).await?;
        Ok(record)
    }

    fn name(&self) -> &str {
        "validation"
    }
}

/// Transformation pipeline stage
#[derive(Debug)]
struct TransformStage {
    transform: Arc<dyn Transform>,
}

impl TransformStage {
    fn new(transform: Arc<dyn Transform>) -> Self {
        Self { transform }
    }
}

#[async_trait]
impl PipelineStage for TransformStage {
    async fn execute(&self, record: Record) -> Result<Record> {
        self.transform.transform(record).await
    }

    fn name(&self) -> &str {
        self.transform.name()
    }
}

/// Storage pipeline stage
#[derive(Debug)]
struct StorageStage {
    storage: Arc<dyn Storage>,
}

impl StorageStage {
    fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl PipelineStage for StorageStage {
    async fn execute(&self, record: Record) -> Result<Record> {
        self.storage.store(&record).await?;
        Ok(record)
    }

    fn name(&self) -> &str {
        "storage"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        storage::InMemoryStorage,
        transform::EnrichTransform,
        validation::{RequiredFieldRule, Validator},
    };
    use serde_json::json;

    #[tokio::test]
    async fn test_pipeline_execution() {
        let mut validator = Validator::new();
        validator.add_rule(Arc::new(RequiredFieldRule::new("name")));

        let transform = Arc::new(EnrichTransform::new(
            "enrich",
            "processed",
            json!(true),
        ));

        let storage = Arc::new(InMemoryStorage::new());

        let pipeline = PipelineBuilder::new("test_pipeline")
            .validate(Arc::new(validator))
            .transform(transform)
            .store(storage.clone())
            .build();

        let record = Record::new("test", json!({"name": "test"}));
        let result = pipeline.execute(record).await.unwrap();

        assert_eq!(result.value["processed"], json!(true));
        assert_eq!(storage.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_pipeline_validation_failure() {
        let mut validator = Validator::new();
        validator.add_rule(Arc::new(RequiredFieldRule::new("name")));

        let pipeline = PipelineBuilder::new("test_pipeline")
            .validate(Arc::new(validator))
            .build();

        let record = Record::new("test", json!({}));
        let result = pipeline.execute(record).await;

        assert!(result.is_err());
    }
}
