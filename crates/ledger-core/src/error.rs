//! Error types for Ledger core operations.
//!
//! This module defines the error hierarchy for all core operations.
//! Errors are descriptive at the core level; the CLI layer will map these
//! to user-friendly messages.

use thiserror::Error;

/// Result type alias for Ledger operations.
pub type Result<T> = std::result::Result<T, LedgerError>;

/// Core error type for Ledger operations.
#[derive(Debug, Error)]
pub enum LedgerError {
    /// Encryption or decryption error
    #[error("Encryption error: {0}")]
    Crypto(String),

    /// Schema validation error
    #[error("Schema error: {0}")]
    Schema(String),

    /// Data validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Storage backend error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid user input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Generic error (fallback)
    #[error("{0}")]
    Other(String),
}

impl From<std::io::Error> for LedgerError {
    fn from(err: std::io::Error) -> Self {
        LedgerError::Storage(err.to_string())
    }
}

impl From<serde_json::Error> for LedgerError {
    fn from(err: serde_json::Error) -> Self {
        LedgerError::Validation(err.to_string())
    }
}
