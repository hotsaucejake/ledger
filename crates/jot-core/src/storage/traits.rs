//! Storage engine trait definition.
//!
//! The `StorageEngine` trait defines the interface that all storage backends
//! must implement. This abstraction allows Jot to support multiple backends
//! (Age+SQLite, SQLCipher, GPG+files) without changing the core logic.

use std::path::Path;
use uuid::Uuid;

use super::types::{
    Composition, CompositionFilter, Entry, EntryComposition, EntryFilter, EntryType, JotMetadata,
    NewComposition, NewEntry, NewEntryType, NewTemplate, Template,
};
use crate::error::Result;

/// Storage engine interface for encrypted jot storage.
///
/// All implementations must ensure:
/// - Data is encrypted at rest
/// - Operations are atomic where possible
/// - UUIDs are used for all identifiers
/// - Append-only semantics for entries
///
/// See RFC-001 for the complete storage model specification.
pub trait StorageEngine: Send + Sync {
    /// Create a new jot at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the jot will be created
    /// * `passphrase` - Passphrase for encryption
    ///
    /// # Returns
    ///
    /// Returns the device ID for this jot.
    ///
    /// # Errors
    ///
    /// Returns `JotError::Storage` if:
    /// - File already exists
    /// - Cannot write to path
    /// - Encryption fails
    fn create(path: &Path, passphrase: &str) -> Result<Uuid>
    where
        Self: Sized;

    /// Open an existing jot.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the jot file
    /// * `passphrase` - Passphrase for decryption
    ///
    /// # Errors
    ///
    /// Returns `JotError::Crypto` if:
    /// - Passphrase is incorrect
    /// - File is corrupted
    /// - Decryption fails
    fn open(path: &Path, passphrase: &str) -> Result<Self>
    where
        Self: Sized;

    /// Close the jot, persisting all changes.
    ///
    /// This method encrypts and writes the jot to disk atomically.
    /// After calling this method, the jot instance should not be used.
    ///
    /// # Errors
    ///
    /// Returns `JotError::Storage` if:
    /// - Cannot write to disk
    /// - Encryption fails
    fn close(self, passphrase: &str) -> Result<()>;

    /// Get jot metadata.
    fn metadata(&self) -> Result<JotMetadata>;

    // --- Entry operations ---

    /// Insert a new entry.
    ///
    /// # Returns
    ///
    /// Returns the UUID of the created entry.
    ///
    /// # Errors
    ///
    /// Returns `JotError::Validation` if:
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

    /// List entry IDs that have been superseded by newer revisions.
    fn superseded_entry_ids(&self) -> Result<std::collections::HashSet<Uuid>>;

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
    /// Returns `JotError::InvalidInput` if:
    /// - Name already exists
    /// - Schema is invalid
    fn create_entry_type(&mut self, entry_type: &NewEntryType) -> Result<Uuid>;

    /// List all entry types.
    fn list_entry_types(&self) -> Result<Vec<EntryType>>;

    // --- Composition operations ---

    /// Create a new composition.
    ///
    /// # Returns
    ///
    /// Returns the UUID of the created composition.
    ///
    /// # Errors
    ///
    /// Returns `JotError::Validation` if:
    /// - Name already exists
    fn create_composition(&mut self, composition: &NewComposition) -> Result<Uuid>;

    /// Get a composition by name.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(composition))` if found, `Ok(None)` if not found.
    fn get_composition(&self, name: &str) -> Result<Option<Composition>>;

    /// Get a composition by ID.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(composition))` if found, `Ok(None)` if not found.
    fn get_composition_by_id(&self, id: &Uuid) -> Result<Option<Composition>>;

    /// List all compositions.
    fn list_compositions(&self, filter: &CompositionFilter) -> Result<Vec<Composition>>;

    /// Rename a composition.
    ///
    /// # Errors
    ///
    /// Returns `JotError::NotFound` if composition doesn't exist.
    /// Returns `JotError::Validation` if new name already exists.
    fn rename_composition(&mut self, id: &Uuid, new_name: &str) -> Result<()>;

    /// Delete a composition.
    ///
    /// This removes the composition and all entry associations.
    /// Entries themselves are not deleted.
    ///
    /// # Errors
    ///
    /// Returns `JotError::NotFound` if composition doesn't exist.
    fn delete_composition(&mut self, id: &Uuid) -> Result<()>;

    /// Attach an entry to a composition.
    ///
    /// # Errors
    ///
    /// Returns `JotError::NotFound` if entry or composition doesn't exist.
    fn attach_entry_to_composition(&mut self, entry_id: &Uuid, composition_id: &Uuid)
        -> Result<()>;

    /// Detach an entry from a composition.
    ///
    /// # Errors
    ///
    /// Returns `JotError::NotFound` if the association doesn't exist.
    fn detach_entry_from_composition(
        &mut self,
        entry_id: &Uuid,
        composition_id: &Uuid,
    ) -> Result<()>;

    /// Get all compositions for an entry.
    fn get_entry_compositions(&self, entry_id: &Uuid) -> Result<Vec<Composition>>;

    /// Get all entries in a composition.
    fn get_composition_entries(&self, composition_id: &Uuid) -> Result<Vec<EntryComposition>>;

    // --- Template operations ---

    /// Create a new template.
    ///
    /// # Returns
    ///
    /// Returns the UUID of the created template.
    ///
    /// # Errors
    ///
    /// Returns `JotError::Validation` if:
    /// - Name already exists
    /// - Entry type doesn't exist
    fn create_template(&mut self, template: &NewTemplate) -> Result<Uuid>;

    /// Get a template by name.
    ///
    /// # Returns
    ///
    /// Returns the latest active version of the template, or `None` if not found.
    fn get_template(&self, name: &str) -> Result<Option<Template>>;

    /// Get a template by ID.
    ///
    /// # Returns
    ///
    /// Returns the latest active version of the template, or `None` if not found.
    fn get_template_by_id(&self, id: &Uuid) -> Result<Option<Template>>;

    /// List all templates.
    fn list_templates(&self) -> Result<Vec<Template>>;

    /// Update a template (creates a new version).
    ///
    /// # Arguments
    ///
    /// * `id` - The template ID
    /// * `template_json` - The new template data
    ///
    /// # Returns
    ///
    /// Returns the new version number.
    ///
    /// # Errors
    ///
    /// Returns `JotError::NotFound` if template doesn't exist.
    fn update_template(&mut self, id: &Uuid, template_json: serde_json::Value) -> Result<i32>;

    /// Delete a template.
    ///
    /// This removes the template and all its versions.
    /// Also removes any default template mappings.
    ///
    /// # Errors
    ///
    /// Returns `JotError::NotFound` if template doesn't exist.
    fn delete_template(&mut self, id: &Uuid) -> Result<()>;

    /// Set the default template for an entry type.
    ///
    /// # Errors
    ///
    /// Returns `JotError::NotFound` if entry type or template doesn't exist.
    /// Returns `JotError::Validation` if template is not for this entry type.
    fn set_default_template(&mut self, entry_type_id: &Uuid, template_id: &Uuid) -> Result<()>;

    /// Clear the default template for an entry type.
    ///
    /// # Errors
    ///
    /// Returns `JotError::NotFound` if entry type doesn't exist.
    fn clear_default_template(&mut self, entry_type_id: &Uuid) -> Result<()>;

    /// Get the default template for an entry type.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(template))` if a default is set, `Ok(None)` otherwise.
    fn get_default_template(&self, entry_type_id: &Uuid) -> Result<Option<Template>>;

    // --- Maintenance operations ---

    /// Check jot integrity.
    ///
    /// Verifies:
    /// - Schema consistency
    /// - Foreign key relationships
    /// - FTS index synchronization
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if jot is valid, or an error describing the problem.
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
