//! # Enterprise Data Processor
//!
//! A production-ready, enterprise-grade data processing library built with Rust.
//!
//! ## Features
//!
//! - **Async Processing**: Built on Tokio for high-performance async operations
//! - **Type Safety**: Leverages Rust's type system for compile-time guarantees
//! - **Error Handling**: Comprehensive error types with context
//! - **Observability**: Built-in metrics, tracing, and logging
//! - **Validation**: Strong data validation with custom rules
//! - **Extensibility**: Plugin architecture for custom processors
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use enterprise_data_processor::{Processor, ProcessorConfig, Record};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ProcessorConfig::default();
//!     let processor = Processor::new(config)?;
//!     
//!     let record = Record::new("data", "value");
//!     let result = processor.process(record).await?;
//!     
//!     Ok(())
//! }
//! ```

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod config;
pub mod error;
pub mod metrics;
pub mod pipeline;
pub mod processor;
pub mod record;
pub mod storage;
pub mod transform;
pub mod validation;

// Re-export main types
pub use config::ProcessorConfig;
pub use error::{Error, Result};
pub use processor::Processor;
pub use record::Record;

use tracing::info;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the library with default settings
///
/// This should be called once at application startup.
///
/// # Examples
///
/// ```rust
/// use enterprise_data_processor;
///
/// enterprise_data_processor::init();
/// ```
pub fn init() {
    init_with_config(config::ProcessorConfig::default())
}

/// Initialize the library with custom configuration
///
/// # Arguments
///
/// * `config` - Custom processor configuration
///
/// # Examples
///
/// ```rust
/// use enterprise_data_processor::{ProcessorConfig, init_with_config};
///
/// let config = ProcessorConfig::builder()
///     .max_batch_size(1000)
///     .build();
/// init_with_config(config);
/// ```
pub fn init_with_config(config: config::ProcessorConfig) {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    info!(
        version = VERSION,
        config = ?config,
        "Enterprise Data Processor initialized"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
