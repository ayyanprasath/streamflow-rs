//! Data transformation module

use crate::{processor::Transform, record::Record, Result};
use async_trait::async_trait;
use serde_json::Value;

/// Transform that filters records based on a predicate
#[derive(Debug)]
pub struct FilterTransform<F>
where
    F: Fn(&Record) -> bool + Send + Sync,
{
    name: String,
    predicate: F,
}

impl<F> FilterTransform<F>
where
    F: Fn(&Record) -> bool + Send + Sync,
{
    /// Create a new filter transform
    pub fn new(name: impl Into<String>, predicate: F) -> Self {
        Self {
            name: name.into(),
            predicate,
        }
    }
}

#[async_trait]
impl<F> Transform for FilterTransform<F>
where
    F: Fn(&Record) -> bool + Send + Sync,
{
    async fn transform(&self, record: Record) -> Result<Record> {
        if (self.predicate)(&record) {
            Ok(record)
        } else {
            Err(crate::Error::processing("Record filtered out"))
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Transform that maps record values
#[derive(Debug)]
pub struct MapTransform<F>
where
    F: Fn(Value) -> Value + Send + Sync,
{
    name: String,
    mapper: F,
}

impl<F> MapTransform<F>
where
    F: Fn(Value) -> Value + Send + Sync,
{
    /// Create a new map transform
    pub fn new(name: impl Into<String>, mapper: F) -> Self {
        Self {
            name: name.into(),
            mapper,
        }
    }
}

#[async_trait]
impl<F> Transform for MapTransform<F>
where
    F: Fn(Value) -> Value + Send + Sync,
{
    async fn transform(&self, mut record: Record) -> Result<Record> {
        record.value = (self.mapper)(record.value);
        Ok(record)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Transform that enriches records with additional data
#[derive(Debug)]
pub struct EnrichTransform {
    name: String,
    field: String,
    value: Value,
}

impl EnrichTransform {
    /// Create a new enrich transform
    pub fn new(name: impl Into<String>, field: impl Into<String>, value: Value) -> Self {
        Self {
            name: name.into(),
            field: field.into(),
            value,
        }
    }
}

#[async_trait]
impl Transform for EnrichTransform {
    async fn transform(&self, mut record: Record) -> Result<Record> {
        if let Some(obj) = record.value.as_object_mut() {
            obj.insert(self.field.clone(), self.value.clone());
        }
        Ok(record)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Transform that normalizes string fields
#[derive(Debug)]
pub struct NormalizeTransform {
    name: String,
    fields: Vec<String>,
}

impl NormalizeTransform {
    /// Create a new normalize transform
    pub fn new(name: impl Into<String>, fields: Vec<String>) -> Self {
        Self {
            name: name.into(),
            fields,
        }
    }
}

#[async_trait]
impl Transform for NormalizeTransform {
    async fn transform(&self, mut record: Record) -> Result<Record> {
        if let Some(obj) = record.value.as_object_mut() {
            for field in &self.fields {
                if let Some(Value::String(s)) = obj.get_mut(field) {
                    *s = s.trim().to_lowercase();
                }
            }
        }
        Ok(record)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_filter_transform() {
        let transform = FilterTransform::new("test_filter", |r: &Record| {
            r.key.starts_with("keep_")
        });

        let keep_record = Record::new("keep_this", "value");
        assert!(transform.transform(keep_record).await.is_ok());

        let drop_record = Record::new("drop_this", "value");
        assert!(transform.transform(drop_record).await.is_err());
    }

    #[tokio::test]
    async fn test_map_transform() {
        let transform = MapTransform::new("test_map", |mut v: Value| {
            if let Some(obj) = v.as_object_mut() {
                if let Some(Value::Number(n)) = obj.get("count") {
                    obj.insert(
                        "count".to_string(),
                        json!(n.as_i64().unwrap_or(0) * 2),
                    );
                }
            }
            v
        });

        let record = Record::new("test", json!({"count": 5}));
        let result = transform.transform(record).await.unwrap();
        assert_eq!(result.value["count"], json!(10));
    }

    #[tokio::test]
    async fn test_enrich_transform() {
        let transform = EnrichTransform::new(
            "test_enrich",
            "timestamp",
            json!("2024-01-01T00:00:00Z"),
        );

        let record = Record::new("test", json!({"name": "test"}));
        let result = transform.transform(record).await.unwrap();
        assert_eq!(result.value["timestamp"], json!("2024-01-01T00:00:00Z"));
    }

    #[tokio::test]
    async fn test_normalize_transform() {
        let transform = NormalizeTransform::new("test_normalize", vec!["name".to_string()]);

        let record = Record::new("test", json!({"name": "  JOHN DOE  "}));
        let result = transform.transform(record).await.unwrap();
        assert_eq!(result.value["name"], json!("john doe"));
    }
}
