//! Age-encrypted SQLite storage backend.
//!
//! This is a skeleton implementation that wires create/open/close
//! with Age passphrase encryption. SQLite schema and entry operations
//! will be added in subsequent steps.

use std::fs;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName, OptionalExtension};
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

        // Initialize schema
        conn.execute_batch(
            r#"
            CREATE TABLE meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE entry_types (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL,
                device_id TEXT NOT NULL
            );

            CREATE TABLE entry_type_versions (
                id TEXT PRIMARY KEY,
                entry_type_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                schema_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                active INTEGER NOT NULL DEFAULT 1,

                UNIQUE(entry_type_id, version),
                FOREIGN KEY(entry_type_id) REFERENCES entry_types(id)
            );

            CREATE TABLE entries (
                id TEXT PRIMARY KEY,
                entry_type_id TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                data_json TEXT NOT NULL,
                tags_json TEXT,
                created_at TEXT NOT NULL,
                device_id TEXT NOT NULL,
                supersedes TEXT,

                FOREIGN KEY(entry_type_id) REFERENCES entry_types(id)
            );

            CREATE VIRTUAL TABLE entries_fts USING fts5(
                entry_id UNINDEXED,
                content,
                tokenize = 'porter'
            );
            "#,
        )
        .map_err(Self::sqlite_error)?;

        // Insert metadata
        let created_at = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            ["format_version", "0.1"],
        )
        .map_err(Self::sqlite_error)?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            ["device_id", &device_id.to_string()],
        )
        .map_err(Self::sqlite_error)?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            ["created_at", &created_at],
        )
        .map_err(Self::sqlite_error)?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            ["last_modified", &created_at],
        )
        .map_err(Self::sqlite_error)?;

        // Serialize and encrypt
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

        // Read device_id from metadata
        let device_id_str: String = conn
            .query_row("SELECT value FROM meta WHERE key = 'device_id'", [], |row| {
                row.get(0)
            })
            .map_err(Self::sqlite_error)?;
        let device_id = Uuid::parse_str(&device_id_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid device_id in metadata: {}", e)))?;

        Ok(Self {
            path: path.to_path_buf(),
            conn: Mutex::new(conn),
            device_id,
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
        let conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

        let format_version: String = conn
            .query_row("SELECT value FROM meta WHERE key = 'format_version'", [], |row| row.get(0))
            .map_err(Self::sqlite_error)?;

        let created_at_str: String = conn
            .query_row("SELECT value FROM meta WHERE key = 'created_at'", [], |row| row.get(0))
            .map_err(Self::sqlite_error)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid created_at timestamp: {}", e)))?
            .with_timezone(&Utc);

        let last_modified_str: String = conn
            .query_row("SELECT value FROM meta WHERE key = 'last_modified'", [], |row| row.get(0))
            .map_err(Self::sqlite_error)?;
        let last_modified = DateTime::parse_from_rfc3339(&last_modified_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid last_modified timestamp: {}", e)))?
            .with_timezone(&Utc);

        Ok(LedgerMetadata {
            format_version,
            device_id: self.device_id,
            created_at,
            last_modified,
        })
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

    fn get_entry_type(&self, name: &str) -> Result<Option<EntryType>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

        let result = conn.query_row(
            r#"
            SELECT etv.id, et.name, etv.version, etv.created_at, et.device_id, etv.schema_json
            FROM entry_type_versions etv
            JOIN entry_types et ON et.id = etv.entry_type_id
            WHERE et.name = ? AND etv.active = 1
            ORDER BY etv.version DESC
            LIMIT 1
            "#,
            [name],
            |row| {
                let id_str: String = row.get(0)?;
                let name: String = row.get(1)?;
                let version: i32 = row.get(2)?;
                let created_at_str: String = row.get(3)?;
                let device_id_str: String = row.get(4)?;
                let schema_json_str: String = row.get(5)?;

                Ok((id_str, name, version, created_at_str, device_id_str, schema_json_str))
            },
        );

        match result {
            Ok((id_str, name, version, created_at_str, device_id_str, schema_json_str)) => {
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
                let device_id = Uuid::parse_str(&device_id_str)
                    .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc);
                let schema_json: serde_json::Value = serde_json::from_str(&schema_json_str)
                    .map_err(|e| LedgerError::Storage(format!("Invalid JSON: {}", e)))?;

                Ok(Some(EntryType {
                    id,
                    name,
                    version,
                    created_at,
                    device_id,
                    schema_json,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::sqlite_error(e)),
        }
    }

    fn create_entry_type(&mut self, entry_type: &NewEntryType) -> Result<Uuid> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

        let tx = conn
            .transaction()
            .map_err(Self::sqlite_error)?;

        // Check if entry type with this name already exists
        let base_type_id: Option<String> = tx
            .query_row(
                "SELECT id FROM entry_types WHERE name = ?",
                [&entry_type.name],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::sqlite_error)?;

        let (base_id, version) = if let Some(ref id_str) = base_type_id {
            // Entry type exists, get the max version and increment
            let base_id = Uuid::parse_str(id_str)
                .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
            let max_version: i32 = tx
                .query_row(
                    "SELECT MAX(version) FROM entry_type_versions WHERE entry_type_id = ?",
                    [id_str],
                    |row| row.get(0),
                )
                .map_err(Self::sqlite_error)?;
            (base_id, max_version + 1)
        } else {
            // New entry type, create base record
            let base_id = Uuid::new_v4();
            let created_at = Utc::now().to_rfc3339();
            tx.execute(
                "INSERT INTO entry_types (id, name, created_at, device_id) VALUES (?, ?, ?, ?)",
                (
                    base_id.to_string(),
                    &entry_type.name,
                    created_at,
                    entry_type.device_id.to_string(),
                ),
            )
            .map_err(Self::sqlite_error)?;
            (base_id, 1)
        };

        // Create version record
        let version_id = Uuid::new_v4();
        let created_at = Utc::now().to_rfc3339();
        let schema_json_str = serde_json::to_string(&entry_type.schema_json)
            .map_err(|e| LedgerError::Storage(format!("Failed to serialize schema: {}", e)))?;

        tx.execute(
            r#"
            INSERT INTO entry_type_versions (id, entry_type_id, version, schema_json, created_at, active)
            VALUES (?, ?, ?, ?, ?, 1)
            "#,
            (
                version_id.to_string(),
                base_id.to_string(),
                version,
                schema_json_str,
                created_at,
            ),
        )
        .map_err(Self::sqlite_error)?;

        tx.commit().map_err(Self::sqlite_error)?;

        Ok(version_id)
    }

    fn list_entry_types(&self) -> Result<Vec<EntryType>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT etv.id, et.name, etv.version, etv.created_at, et.device_id, etv.schema_json
                FROM entry_type_versions etv
                JOIN entry_types et ON et.id = etv.entry_type_id
                WHERE etv.active = 1 AND etv.version = (
                    SELECT MAX(version)
                    FROM entry_type_versions
                    WHERE entry_type_id = etv.entry_type_id AND active = 1
                )
                ORDER BY et.name
                "#,
            )
            .map_err(Self::sqlite_error)?;

        let rows = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let name: String = row.get(1)?;
                let version: i32 = row.get(2)?;
                let created_at_str: String = row.get(3)?;
                let device_id_str: String = row.get(4)?;
                let schema_json_str: String = row.get(5)?;

                Ok((id_str, name, version, created_at_str, device_id_str, schema_json_str))
            })
            .map_err(Self::sqlite_error)?;

        let mut entry_types = Vec::new();
        for row in rows {
            let (id_str, name, version, created_at_str, device_id_str, schema_json_str) =
                row.map_err(Self::sqlite_error)?;

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
            let device_id = Uuid::parse_str(&device_id_str)
                .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);
            let schema_json: serde_json::Value = serde_json::from_str(&schema_json_str)
                .map_err(|e| LedgerError::Storage(format!("Invalid JSON: {}", e)))?;

            entry_types.push(EntryType {
                id,
                name,
                version,
                created_at,
                device_id,
                schema_json,
            });
        }

        Ok(entry_types)
    }

    fn check_integrity(&self) -> Result<()> {
        Err(LedgerError::Storage(
            "check_integrity not implemented for AgeSqliteStorage".to_string(),
        ))
    }
}
