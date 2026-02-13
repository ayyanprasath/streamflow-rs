//! Configuration management for the processor

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main configuration for the data processor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Maximum number of records to process in a single batch
    pub max_batch_size: usize,
    
    /// Maximum number of concurrent workers
    pub max_workers: usize,
    
    /// Timeout for processing operations
    pub operation_timeout: Duration,
    
    /// Enable metrics collection
    pub enable_metrics: bool,
    
    /// Enable detailed tracing
    pub enable_tracing: bool,
    
    /// Retry configuration
    pub retry_config: RetryConfig,
    
    /// Buffer size for async channels
    pub buffer_size: usize,
    
    /// Enable compression for data storage
    pub enable_compression: bool,
}

/// Retry configuration for failed operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    
    /// Initial backoff duration
    pub initial_backoff: Duration,
    
    /// Maximum backoff duration
    pub max_backoff: Duration,
    
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            max_workers: num_cpus(),
            operation_timeout: Duration::from_secs(30),
            enable_metrics: true,
            enable_tracing: true,
            retry_config: RetryConfig::default(),
            buffer_size: 1000,
            enable_compression: false,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

impl ProcessorConfig {
    /// Create a new builder for ProcessorConfig
    pub fn builder() -> ProcessorConfigBuilder {
        ProcessorConfigBuilder::new()
    }

    /// Validate the configuration
    pub fn validate(&self) -> crate::Result<()> {
        if self.max_batch_size == 0 {
            return Err(crate::Error::config("max_batch_size must be greater than 0"));
        }
        
        if self.max_workers == 0 {
            return Err(crate::Error::config("max_workers must be greater than 0"));
        }
        
        if self.buffer_size == 0 {
            return Err(crate::Error::config("buffer_size must be greater than 0"));
        }
        
        self.retry_config.validate()?;
        
        Ok(())
    }
}

impl RetryConfig {
    /// Validate the retry configuration
    pub fn validate(&self) -> crate::Result<()> {
        if self.max_attempts == 0 {
            return Err(crate::Error::config("max_attempts must be greater than 0"));
        }
        
        if self.backoff_multiplier <= 1.0 {
            return Err(crate::Error::config("backoff_multiplier must be greater than 1.0"));
        }
        
        if self.initial_backoff > self.max_backoff {
            return Err(crate::Error::config(
                "initial_backoff cannot be greater than max_backoff",
            ));
        }
        
        Ok(())
    }

    /// Calculate backoff duration for a given attempt
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        let backoff = self.initial_backoff.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);
        
        let backoff = backoff.min(self.max_backoff.as_millis() as f64);
        Duration::from_millis(backoff as u64)
    }
}

/// Builder for ProcessorConfig
#[derive(Debug)]
pub struct ProcessorConfigBuilder {
    config: ProcessorConfig,
}

impl ProcessorConfigBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            config: ProcessorConfig::default(),
        }
    }

    /// Set maximum batch size
    pub fn max_batch_size(mut self, size: usize) -> Self {
        self.config.max_batch_size = size;
        self
    }

    /// Set maximum number of workers
    pub fn max_workers(mut self, workers: usize) -> Self {
        self.config.max_workers = workers;
        self
    }

    /// Set operation timeout
    pub fn operation_timeout(mut self, timeout: Duration) -> Self {
        self.config.operation_timeout = timeout;
        self
    }

    /// Enable or disable metrics
    pub fn enable_metrics(mut self, enabled: bool) -> Self {
        self.config.enable_metrics = enabled;
        self
    }

    /// Enable or disable tracing
    pub fn enable_tracing(mut self, enabled: bool) -> Self {
        self.config.enable_tracing = enabled;
        self
    }

    /// Set retry configuration
    pub fn retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.config.retry_config = retry_config;
        self
    }

    /// Set buffer size
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    /// Enable or disable compression
    pub fn enable_compression(mut self, enabled: bool) -> Self {
        self.config.enable_compression = enabled;
        self
    }

    /// Build the configuration
    pub fn build(self) -> ProcessorConfig {
        self.config
    }

    /// Build and validate the configuration
    pub fn build_validated(self) -> crate::Result<ProcessorConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for ProcessorConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the number of logical CPU cores
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProcessorConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = ProcessorConfig::builder()
            .max_batch_size(500)
            .max_workers(8)
            .build();
        
        assert_eq!(config.max_batch_size, 500);
        assert_eq!(config.max_workers, 8);
    }

    #[test]
    fn test_invalid_config() {
        let config = ProcessorConfig::builder()
            .max_batch_size(0)
            .build();
        
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_backoff_calculation() {
        let retry_config = RetryConfig::default();
        let backoff = retry_config.calculate_backoff(2);
        assert!(backoff > retry_config.initial_backoff);
        assert!(backoff <= retry_config.max_backoff);
    }
}
