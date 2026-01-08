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
//! ## Phase 0.1 (Milestone 0)
//!
//! This milestone provides the project skeleton with no real logic.
//! The CLI will compile and show help, but core functionality is not yet implemented.

pub mod error;

pub use error::{LedgerError, Result};

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
