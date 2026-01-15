//! Storage abstraction for Jot.
//!
//! This module defines the `StorageEngine` trait and core types for
//! interacting with encrypted jot storage.
//!
//! ## Architecture
//!
//! The storage layer is designed to be backend-agnostic:
//! - Phase 0.1: Age-encrypted SQLite (in-memory)
//! - Future: SQLCipher, GPG + files, etc.
//!
//! All storage engines must implement the `StorageEngine` trait, which
//! provides a consistent interface for entry and schema management.
//!
//! ## Security
//!
//! Storage engines are responsible for:
//! - Encryption at rest (no plaintext modes)
//! - Key derivation and management
//! - Atomic writes to prevent corruption
//!
//! See RFC-001 for the complete storage model.

pub mod age_sqlite;
pub mod encryption;
pub mod traits;
pub mod types;

// Re-export public types
pub use age_sqlite::AgeSqliteStorage;
pub use traits::StorageEngine;
pub use types::{
    Composition, CompositionFilter, Entry, EntryComposition, EntryFilter, EntryType, JotMetadata,
    NewComposition, NewEntry, NewEntryType, NewTemplate, Template,
};
