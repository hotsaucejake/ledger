//! # Ledger Core
//!
//! Core library for Ledger - a secure, encrypted, CLI-first personal journal and logbook.
//!
//! This crate provides the core domain logic, storage abstractions, and data models
//! independent of the CLI interface.
//!
//! ## Architecture
//!
//! - **storage**: Storage engine trait and implementations
//! - **entry**: Entry creation and validation
//! - **schema**: Entry type schemas and field definitions
//! - **search**: Full-text search and querying
//! - **tags**: Tag normalization and filtering
//! - **export**: Export formats (JSON, JSONL)
//!
//! ## Milestones
//!
//! - **M0**: Project skeleton ✓
//! - **M1**: Encrypted storage (in progress)
//! - **M2**: Journal entries
//! - **M3**: Full-text search
//! - **M4**: Export & backup

pub mod error;
pub mod storage;

pub use error::{LedgerError, Result};
pub use storage::StorageEngine;

/// Core version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
