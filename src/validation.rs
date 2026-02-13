//! Data validation module

use crate::{error::ValidationError, record::Record, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for implementing validation rules
#[async_trait]
pub trait ValidationRule: Send + Sync {
    /// Validate a record
    async fn validate(&self, record: &Record) -> Result<()>;
    
    /// Name of the validation rule
    fn name(&self) -> &str;
    
    /// Description of what the rule validates
    fn description(&self) -> &str;
}

/// Validator that applies multiple rules
#[derive(Debug, Default)]
pub struct Validator {
    rules: Vec<Arc<dyn ValidationRule>>,
}

impl Validator {
    /// Create a new validator
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a validation rule
    pub fn add_rule(&mut self, rule: Arc<dyn ValidationRule>) {
        self.rules.push(rule);
    }

    /// Validate a record against all rules
    pub async fn validate(&self, record: &Record) -> Result<()> {
        for rule in &self.rules {
            rule.validate(record).await?;
        }
        Ok(())
    }

    /// Get number of registered rules
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// Validation rule for required fields
#[derive(Debug)]
pub struct RequiredFieldRule {
    field: String,
}

impl RequiredFieldRule {
    /// Create a new required field rule
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
        }
    }
}

#[async_trait]
impl ValidationRule for RequiredFieldRule {
    async fn validate(&self, record: &Record) -> Result<()> {
        if record.value.get(&self.field).is_none() {
            return Err(ValidationError {
                field: self.field.clone(),
                rule: "required".to_string(),
                message: format!("Field '{}' is required", self.field),
            }
            .into());
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "required_field"
    }

    fn description(&self) -> &str {
        "Validates that a required field is present"
    }
}

/// Validation rule for non-empty strings
#[derive(Debug)]
pub struct NonEmptyStringRule {
    field: String,
}

impl NonEmptyStringRule {
    /// Create a new non-empty string rule
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
        }
    }
}

#[async_trait]
impl ValidationRule for NonEmptyStringRule {
    async fn validate(&self, record: &Record) -> Result<()> {
        if let Some(value) = record.value.get(&self.field) {
            if let Some(s) = value.as_str() {
                if s.is_empty() {
                    return Err(ValidationError {
                        field: self.field.clone(),
                        rule: "non_empty".to_string(),
                        message: format!("Field '{}' cannot be empty", self.field),
                    }
                    .into());
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "non_empty_string"
    }

    fn description(&self) -> &str {
        "Validates that a string field is not empty"
    }
}

/// Validation rule for numeric ranges
#[derive(Debug)]
pub struct NumericRangeRule {
    field: String,
    min: Option<f64>,
    max: Option<f64>,
}

impl NumericRangeRule {
    /// Create a new numeric range rule
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            min: None,
            max: None,
        }
    }

    /// Set minimum value
    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum value
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }
}

#[async_trait]
impl ValidationRule for NumericRangeRule {
    async fn validate(&self, record: &Record) -> Result<()> {
        if let Some(value) = record.value.get(&self.field) {
            if let Some(num) = value.as_f64() {
                if let Some(min) = self.min {
                    if num < min {
                        return Err(ValidationError {
                            field: self.field.clone(),
                            rule: "min_value".to_string(),
                            message: format!(
                                "Field '{}' must be at least {}",
                                self.field, min
                            ),
                        }
                        .into());
                    }
                }
                if let Some(max) = self.max {
                    if num > max {
                        return Err(ValidationError {
                            field: self.field.clone(),
                            rule: "max_value".to_string(),
                            message: format!(
                                "Field '{}' must be at most {}",
                                self.field, max
                            ),
                        }
                        .into());
                    }
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "numeric_range"
    }

    fn description(&self) -> &str {
        "Validates that a numeric field is within a specified range"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_required_field_rule() {
        let rule = RequiredFieldRule::new("name");
        
        let mut record = Record::new("test", json!({"name": "value"}));
        assert!(rule.validate(&record).await.is_ok());
        
        record.update_value(json!({}));
        assert!(rule.validate(&record).await.is_err());
    }

    #[tokio::test]
    async fn test_non_empty_string_rule() {
        let rule = NonEmptyStringRule::new("name");
        
        let record = Record::new("test", json!({"name": "value"}));
        assert!(rule.validate(&record).await.is_ok());
        
        let empty_record = Record::new("test", json!({"name": ""}));
        assert!(rule.validate(&empty_record).await.is_err());
    }

    #[tokio::test]
    async fn test_numeric_range_rule() {
        let rule = NumericRangeRule::new("age").min(0.0).max(150.0);
        
        let record = Record::new("test", json!({"age": 25}));
        assert!(rule.validate(&record).await.is_ok());
        
        let invalid_record = Record::new("test", json!({"age": 200}));
        assert!(rule.validate(&invalid_record).await.is_err());
    }

    #[tokio::test]
    async fn test_validator() {
        let mut validator = Validator::new();
        validator.add_rule(Arc::new(RequiredFieldRule::new("name")));
        validator.add_rule(Arc::new(NonEmptyStringRule::new("name")));
        
        let record = Record::new("test", json!({"name": "John"}));
        assert!(validator.validate(&record).await.is_ok());
        
        let invalid_record = Record::new("test", json!({}));
        assert!(validator.validate(&invalid_record).await.is_err());
    }
}
