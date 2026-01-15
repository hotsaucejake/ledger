//! Error types for Jot core operations.
//!
//! This module defines the error hierarchy for all core operations.
//! Errors are descriptive at the core level; the CLI layer will map these
//! to user-friendly messages.

use thiserror::Error;
use uuid::Uuid;

/// Result type alias for Jot operations.
pub type Result<T> = std::result::Result<T, JotError>;

/// Core error type for Jot operations.
#[derive(Debug, Error)]
pub enum JotError {
    /// Incorrect passphrase during decryption
    #[error("Incorrect passphrase")]
    IncorrectPassphrase,

    /// Jot file not found
    #[error("Jot file not found")]
    JotNotFound,

    /// Encryption or decryption error
    #[error("Encryption error: {0}")]
    Crypto(String),

    /// Schema validation error
    #[error("Schema error: {0}")]
    Schema(String),

    /// Data validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Storage backend error (generic)
    #[error("Storage error: {0}")]
    Storage(String),

    /// SQLite-specific storage error
    #[error("SQLite error: {source}")]
    Sqlite {
        #[from]
        source: rusqlite::Error,
    },

    /// Entry not found by ID
    #[error("Entry not found: {0}")]
    EntryNotFound(Uuid),

    /// Entry type not found by name
    #[error("Entry type not found: {0}")]
    EntryTypeNotFound(String),

    /// Generic resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid user input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// I/O error
    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    /// JSON serialization/deserialization error
    #[error("JSON error: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },

    /// Generic error (fallback)
    #[error("{0}")]
    Other(String),
}
