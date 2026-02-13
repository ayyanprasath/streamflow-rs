//! Error types and result aliases for the library

use std::fmt;
use thiserror::Error;

/// Result type alias using our custom Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the library
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Processing error
    #[error("Processing error: {0}")]
    Processing(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid state error
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Concurrent access error
    #[error("Concurrent access error: {0}")]
    Concurrency(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Validation error details
#[derive(Debug)]
pub struct ValidationError {
    /// Field that failed validation
    pub field: String,
    /// Validation rule that was violated
    pub rule: String,
    /// Additional context
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Field '{}' failed validation rule '{}': {}",
            self.field, self.rule, self.message
        )
    }
}

impl std::error::Error for ValidationError {}

impl Error {
    /// Create a new configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Error::Config(msg.into())
    }

    /// Create a new processing error
    pub fn processing(msg: impl Into<String>) -> Self {
        Error::Processing(msg.into())
    }

    /// Create a new storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Error::Storage(msg.into())
    }

    /// Create a new invalid state error
    pub fn invalid_state(msg: impl Into<String>) -> Self {
        Error::InvalidState(msg.into())
    }

    /// Create a new not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        Error::NotFound(msg.into())
    }

    /// Create a new timeout error
    pub fn timeout(msg: impl Into<String>) -> Self {
        Error::Timeout(msg.into())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Timeout(_) | Error::Concurrency(_) | Error::Io(_)
        )
    }

    /// Get error code for monitoring/alerting
    pub fn code(&self) -> &str {
        match self {
            Error::Config(_) => "CONFIG_ERROR",
            Error::Validation(_) => "VALIDATION_ERROR",
            Error::Processing(_) => "PROCESSING_ERROR",
            Error::Storage(_) => "STORAGE_ERROR",
            Error::Io(_) => "IO_ERROR",
            Error::Serialization(_) => "SERIALIZATION_ERROR",
            Error::InvalidState(_) => "INVALID_STATE",
            Error::NotFound(_) => "NOT_FOUND",
            Error::Timeout(_) => "TIMEOUT",
            Error::Concurrency(_) => "CONCURRENCY_ERROR",
            Error::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = Error::config("test config error");
        assert_eq!(err.code(), "CONFIG_ERROR");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_retryable_errors() {
        let timeout = Error::timeout("test timeout");
        assert!(timeout.is_retryable());
    }
}
