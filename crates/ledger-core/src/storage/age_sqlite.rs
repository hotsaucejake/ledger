//! Age-encrypted SQLite storage backend.
//!
//! This is a skeleton implementation that wires create/open/close
//! with Age passphrase encryption. SQLite schema and entry operations
//! will be added in subsequent steps.

use std::fs;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::Mutex;

use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName};
use uuid::Uuid;

use age::secrecy::{ExposeSecret, SecretString};

use crate::crypto::validate_passphrase;
use crate::error::{LedgerError, Result};
use crate::storage::encryption::{decrypt, encrypt};
use crate::storage::traits::StorageEngine;
use crate::storage::types::{Entry, EntryFilter, EntryType, LedgerMetadata, NewEntry, NewEntryType};

/// Age-encrypted SQLite storage engine (Phase 0.1).
pub struct AgeSqliteStorage {
    path: PathBuf,
    conn: Mutex<Connection>,
    #[allow(dead_code)]
    device_id: Uuid,
    // Retained to re-encrypt on close; will be replaced with a derived key later.
    passphrase: SecretString,
}

impl AgeSqliteStorage {
    fn sqlite_error(err: rusqlite::Error) -> LedgerError {
        LedgerError::Storage(format!("SQLite error: {}", err))
    }

    fn empty_payload(conn: &Connection) -> Result<Vec<u8>> {
        let data = conn
            .serialize(DatabaseName::Main)
            .map_err(Self::sqlite_error)?;
        Ok(data.as_ref().to_vec())
    }

    fn owned_data_from_bytes(bytes: &[u8]) -> Result<OwnedData> {
        if bytes.is_empty() {
            return Err(LedgerError::Storage(
                "SQLite payload is empty".to_string(),
            ));
        }

        let size: i32 = bytes
            .len()
            .try_into()
            .map_err(|_| LedgerError::Storage("SQLite payload too large".to_string()))?;
        let raw = unsafe { rusqlite::ffi::sqlite3_malloc(size) as *mut u8 };
        if raw.is_null() {
            return Err(LedgerError::Storage(
                "SQLite allocation failed".to_string(),
            ));
        }

        // Allocate with sqlite3_malloc so SQLite can own the buffer on deserialize.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), raw, bytes.len());
            let ptr = NonNull::new(raw).ok_or_else(|| {
                LedgerError::Storage("SQLite allocation returned null".to_string())
            })?;
            Ok(OwnedData::from_raw_nonnull(ptr, bytes.len()))
        }
    }
}

impl StorageEngine for AgeSqliteStorage {
    fn create(path: &Path, passphrase: &str) -> Result<Uuid> {
        if path.exists() {
            return Err(LedgerError::Storage("Ledger file already exists".to_string()));
        }

        validate_passphrase(passphrase)?;

        let device_id = Uuid::new_v4();
        let conn = Connection::open_in_memory().map_err(Self::sqlite_error)?;
        let plaintext = Self::empty_payload(&conn)?;
        let encrypted = encrypt(&plaintext, passphrase)?;
        fs::write(path, encrypted)?;

        Ok(device_id)
    }

    fn open(path: &Path, passphrase: &str) -> Result<Self> {
        if !path.exists() {
            return Err(LedgerError::Storage("Ledger file not found".to_string()));
        }

        validate_passphrase(passphrase)?;

        let encrypted = fs::read(path)?;
        let plaintext = decrypt(&encrypted, passphrase)?;
        let mut conn = Connection::open_in_memory().map_err(Self::sqlite_error)?;
        let owned_data = Self::owned_data_from_bytes(&plaintext)?;
        conn.deserialize(DatabaseName::Main, owned_data, false)
            .map_err(Self::sqlite_error)?;

        Ok(Self {
            path: path.to_path_buf(),
            conn: Mutex::new(conn),
            device_id: Uuid::new_v4(),
            passphrase: SecretString::from(passphrase.to_string()),
        })
    }

    fn close(self) -> Result<()> {
        let conn = self
            .conn
            .into_inner()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;
        let data = conn
            .serialize(DatabaseName::Main)
            .map_err(Self::sqlite_error)?;
        let encrypted = encrypt(data.as_ref(), self.passphrase.expose_secret())?;
        fs::write(self.path, encrypted)?;
        Ok(())
    }

    fn metadata(&self) -> Result<LedgerMetadata> {
        Err(LedgerError::Storage(
            "Metadata not implemented for AgeSqliteStorage".to_string(),
        ))
    }

    fn insert_entry(&mut self, _entry: &NewEntry) -> Result<Uuid> {
        Err(LedgerError::Storage(
            "insert_entry not implemented for AgeSqliteStorage".to_string(),
        ))
    }

    fn get_entry(&self, _id: &Uuid) -> Result<Option<Entry>> {
        Err(LedgerError::Storage(
            "get_entry not implemented for AgeSqliteStorage".to_string(),
        ))
    }

    fn list_entries(&self, _filter: &EntryFilter) -> Result<Vec<Entry>> {
        Err(LedgerError::Storage(
            "list_entries not implemented for AgeSqliteStorage".to_string(),
        ))
    }

    fn search_entries(&self, _query: &str) -> Result<Vec<Entry>> {
        Err(LedgerError::Storage(
            "search_entries not implemented for AgeSqliteStorage".to_string(),
        ))
    }

    fn get_entry_type(&self, _name: &str) -> Result<Option<EntryType>> {
        Err(LedgerError::Storage(
            "get_entry_type not implemented for AgeSqliteStorage".to_string(),
        ))
    }

    fn create_entry_type(&mut self, _entry_type: &NewEntryType) -> Result<Uuid> {
        Err(LedgerError::Storage(
            "create_entry_type not implemented for AgeSqliteStorage".to_string(),
        ))
    }

    fn list_entry_types(&self) -> Result<Vec<EntryType>> {
        Err(LedgerError::Storage(
            "list_entry_types not implemented for AgeSqliteStorage".to_string(),
        ))
    }

    fn check_integrity(&self) -> Result<()> {
        Err(LedgerError::Storage(
            "check_integrity not implemented for AgeSqliteStorage".to_string(),
        ))
    }
}
