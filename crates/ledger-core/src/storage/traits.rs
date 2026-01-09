//! Storage engine trait definition.
//!
//! The `StorageEngine` trait defines the interface that all storage backends
//! must implement. This abstraction allows Ledger to support multiple backends
//! (Age+SQLite, SQLCipher, GPG+files) without changing the core logic.

use std::path::Path;
use uuid::Uuid;

use super::types::{Entry, EntryFilter, EntryType, LedgerMetadata, NewEntry, NewEntryType};
use crate::error::Result;

/// Storage engine interface for encrypted ledger storage.
///
/// All implementations must ensure:
/// - Data is encrypted at rest
/// - Operations are atomic where possible
/// - UUIDs are used for all identifiers
/// - Append-only semantics for entries
///
/// See RFC-001 for the complete storage model specification.
pub trait StorageEngine: Send + Sync {
    /// Create a new ledger at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the ledger will be created
    /// * `passphrase` - Passphrase for encryption
    ///
    /// # Returns
    ///
    /// Returns the device ID for this ledger.
    ///
    /// # Errors
    ///
    /// Returns `LedgerError::Storage` if:
    /// - File already exists
    /// - Cannot write to path
    /// - Encryption fails
    fn create(path: &Path, passphrase: &str) -> Result<Uuid>
    where
        Self: Sized;

    /// Open an existing ledger.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the ledger file
    /// * `passphrase` - Passphrase for decryption
    ///
    /// # Errors
    ///
    /// Returns `LedgerError::Crypto` if:
    /// - Passphrase is incorrect
    /// - File is corrupted
    /// - Decryption fails
    fn open(path: &Path, passphrase: &str) -> Result<Self>
    where
        Self: Sized;

    /// Close the ledger, persisting all changes.
    ///
    /// This method encrypts and writes the ledger to disk atomically.
    /// After calling this method, the ledger instance should not be used.
    ///
    /// # Errors
    ///
    /// Returns `LedgerError::Storage` if:
    /// - Cannot write to disk
    /// - Encryption fails
    fn close(self, passphrase: &str) -> Result<()>;

    /// Get ledger metadata.
    fn metadata(&self) -> Result<LedgerMetadata>;

    // --- Entry operations ---

    /// Insert a new entry.
    ///
    /// # Returns
    ///
    /// Returns the UUID of the created entry.
    ///
    /// # Errors
    ///
    /// Returns `LedgerError::Validation` if:
    /// - Entry type does not exist
    /// - Schema version is invalid
    /// - Data does not match schema
    fn insert_entry(&mut self, entry: &NewEntry) -> Result<Uuid>;

    /// Get an entry by ID.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(entry))` if found, `Ok(None)` if not found.
    fn get_entry(&self, id: &Uuid) -> Result<Option<Entry>>;

    /// List entries matching the filter.
    ///
    /// Entries are returned in reverse chronological order (newest first).
    fn list_entries(&self, filter: &EntryFilter) -> Result<Vec<Entry>>;

    /// Search entries using full-text search.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    ///
    /// # Returns
    ///
    /// Returns entries ordered by relevance, then recency.
    fn search_entries(&self, query: &str) -> Result<Vec<Entry>>;

    // --- Entry type operations ---

    /// Get an entry type by name.
    ///
    /// # Returns
    ///
    /// Returns the latest active version of the entry type, or `None` if not found.
    fn get_entry_type(&self, name: &str) -> Result<Option<EntryType>>;

    /// Create a new entry type.
    ///
    /// # Returns
    ///
    /// Returns the UUID of the created entry type.
    ///
    /// # Errors
    ///
    /// Returns `LedgerError::InvalidInput` if:
    /// - Name already exists
    /// - Schema is invalid
    fn create_entry_type(&mut self, entry_type: &NewEntryType) -> Result<Uuid>;

    /// List all entry types.
    fn list_entry_types(&self) -> Result<Vec<EntryType>>;

    // --- Maintenance operations ---

    /// Check ledger integrity.
    ///
    /// Verifies:
    /// - Schema consistency
    /// - Foreign key relationships
    /// - FTS index synchronization
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if ledger is valid, or an error describing the problem.
    fn check_integrity(&self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests verify the trait contract exists
    // Actual implementations will be tested in their own modules

    #[test]
    fn test_trait_definition_compiles() {
        // This test simply ensures the trait definition is valid
        // and can be used as a trait bound
        fn _accepts_storage_engine<T: StorageEngine>(_engine: T) {}
    }
}
