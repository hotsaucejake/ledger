//! Age-encrypted SQLite storage backend.
//!
//! This module provides an encrypted SQLite storage engine using Age
//! passphrase encryption. The database is held in memory and serialized
//! to disk with encryption on close.

mod row;
mod validation;

use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName, OptionalExtension};
use uuid::Uuid;

use crate::crypto::validate_passphrase;
use crate::error::{LedgerError, Result};
use crate::storage::encryption::{decrypt, encrypt};
use crate::storage::traits::StorageEngine;
use crate::storage::types::{
    Composition, CompositionFilter, Entry, EntryComposition, EntryFilter, EntryType,
    LedgerMetadata, NewComposition, NewEntry, NewEntryType, NewTemplate, Template,
};

use row::EntryRow;
use validation::{fts_content_for_entry, normalize_tags, validate_entry_data, MAX_DATA_BYTES};

/// Age-encrypted SQLite storage engine.
pub struct AgeSqliteStorage {
    path: PathBuf,
    conn: Mutex<Connection>,
    #[allow(dead_code)]
    device_id: Uuid,
}

impl AgeSqliteStorage {
    /// Lock the database connection, returning an error if the mutex is poisoned.
    fn lock_conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))
    }

    fn empty_payload(conn: &Connection) -> Result<Vec<u8>> {
        let data = conn.serialize(DatabaseName::Main)?;
        Ok(data.as_ref().to_vec())
    }

    fn owned_data_from_bytes(bytes: &[u8]) -> Result<OwnedData> {
        if bytes.is_empty() {
            return Err(LedgerError::Storage("SQLite payload is empty".to_string()));
        }

        let size: i32 = bytes
            .len()
            .try_into()
            .map_err(|_| LedgerError::Storage("SQLite payload too large".to_string()))?;

        // SAFETY: sqlite3_malloc is a standard SQLite allocation function that returns
        // a valid pointer or null. We check for null immediately after and return an
        // error if allocation failed. The size has been validated to fit in i32.
        let raw = unsafe { rusqlite::ffi::sqlite3_malloc(size) as *mut u8 };
        if raw.is_null() {
            return Err(LedgerError::Storage("SQLite allocation failed".to_string()));
        }

        // SAFETY:
        // - `raw` is valid: allocated above via sqlite3_malloc, confirmed non-null
        // - `raw` is writable for `bytes.len()` bytes: sqlite3_malloc(size) allocates
        //   exactly `size` bytes, and size == bytes.len() (validated via try_into)
        // - `bytes.as_ptr()` is valid for reads of `bytes.len()` bytes: guaranteed by slice
        // - The regions don't overlap: `raw` is freshly allocated heap memory
        // - `OwnedData::from_raw_nonnull` takes ownership of the sqlite3_malloc'd buffer,
        //   which will be freed by SQLite when the OwnedData is dropped or consumed
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), raw, bytes.len());
            let ptr = NonNull::new(raw).ok_or_else(|| {
                LedgerError::Storage("SQLite allocation returned null".to_string())
            })?;
            Ok(OwnedData::from_raw_nonnull(ptr, bytes.len()))
        }
    }

    fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| LedgerError::Storage("Invalid ledger path".to_string()))?;

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| LedgerError::Storage(format!("System time error: {}", e)))?
            .as_nanos();
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| LedgerError::Storage("Invalid ledger filename".to_string()))?;
        let temp_path = parent.join(format!("{}.{}.tmp", filename, nanos));

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|e| LedgerError::Storage(format!("Temp file create failed: {}", e)))?;
        use std::io::Write;
        file.write_all(data)
            .map_err(|e| LedgerError::Storage(format!("Temp file write failed: {}", e)))?;
        file.sync_all()
            .map_err(|e| LedgerError::Storage(format!("Temp file sync failed: {}", e)))?;

        crate::fs::rename_with_fallback(&temp_path, path)
            .map_err(|e| LedgerError::Storage(format!("Atomic rename failed: {}", e)))?;

        Ok(())
    }
}

impl StorageEngine for AgeSqliteStorage {
    fn create(path: &Path, passphrase: &str) -> Result<Uuid> {
        if path.exists() {
            return Err(LedgerError::Storage(
                "Ledger file already exists".to_string(),
            ));
        }

        validate_passphrase(passphrase)?;

        let device_id = Uuid::new_v4();
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

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

            -- Compositions: semantic grouping of entries
            CREATE TABLE compositions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                created_at TEXT NOT NULL,
                device_id TEXT NOT NULL,
                metadata_json TEXT
            );

            -- Entry-Composition join table (many-to-many)
            CREATE TABLE entry_compositions (
                entry_id TEXT NOT NULL,
                composition_id TEXT NOT NULL,
                added_at TEXT NOT NULL,

                PRIMARY KEY (entry_id, composition_id),
                FOREIGN KEY (entry_id) REFERENCES entries(id),
                FOREIGN KEY (composition_id) REFERENCES compositions(id)
            );

            -- Templates: reusable defaults for entry creation
            CREATE TABLE templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                entry_type_id TEXT NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL,
                device_id TEXT NOT NULL,

                FOREIGN KEY (entry_type_id) REFERENCES entry_types(id)
            );

            -- Template versions (append-only)
            CREATE TABLE template_versions (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                template_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                active INTEGER NOT NULL DEFAULT 1,

                UNIQUE(template_id, version),
                FOREIGN KEY (template_id) REFERENCES templates(id)
            );

            -- Entry type to default template mapping
            CREATE TABLE entry_type_templates (
                entry_type_id TEXT NOT NULL,
                template_id TEXT NOT NULL,
                active INTEGER NOT NULL DEFAULT 1,

                PRIMARY KEY (entry_type_id, template_id),
                FOREIGN KEY (entry_type_id) REFERENCES entry_types(id),
                FOREIGN KEY (template_id) REFERENCES templates(id)
            );

            -- Ensure only one active default template per entry type
            CREATE UNIQUE INDEX entry_type_templates_active
            ON entry_type_templates (entry_type_id)
            WHERE active = 1;
            "#,
        )?;

        // Insert metadata
        let created_at = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            ["format_version", "0.1"],
        )?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            ["device_id", &device_id.to_string()],
        )?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            ["created_at", &created_at],
        )?;
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?, ?)",
            ["last_modified", &created_at],
        )?;

        // Serialize and encrypt
        let plaintext = Self::empty_payload(&conn)?;
        let encrypted = encrypt(&plaintext, passphrase)?;
        Self::write_atomic(path, &encrypted)?;

        Ok(device_id)
    }

    fn open(path: &Path, passphrase: &str) -> Result<Self> {
        if !path.exists() {
            return Err(LedgerError::LedgerNotFound);
        }

        validate_passphrase(passphrase)?;

        let encrypted = fs::read(path)?;
        let plaintext = decrypt(&encrypted, passphrase)?;
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let owned_data = Self::owned_data_from_bytes(&plaintext)?;
        conn.deserialize(DatabaseName::Main, owned_data, false)?;

        // Read device_id from metadata
        let device_id_str: String = conn.query_row(
            "SELECT value FROM meta WHERE key = 'device_id'",
            [],
            |row| row.get(0),
        )?;
        let device_id = Uuid::parse_str(&device_id_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid device_id in metadata: {}", e)))?;

        Ok(Self {
            path: path.to_path_buf(),
            conn: Mutex::new(conn),
            device_id,
        })
    }

    fn close(self, passphrase: &str) -> Result<()> {
        validate_passphrase(passphrase)?;
        let conn = self
            .conn
            .into_inner()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;
        let data = conn.serialize(DatabaseName::Main)?;
        let encrypted = encrypt(data.as_ref(), passphrase)?;
        Self::write_atomic(&self.path, &encrypted)?;
        Ok(())
    }

    fn metadata(&self) -> Result<LedgerMetadata> {
        let conn = self.lock_conn()?;

        let format_version: String = conn.query_row(
            "SELECT value FROM meta WHERE key = 'format_version'",
            [],
            |row| row.get(0),
        )?;

        let created_at_str: String = conn.query_row(
            "SELECT value FROM meta WHERE key = 'created_at'",
            [],
            |row| row.get(0),
        )?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid created_at timestamp: {}", e)))?
            .with_timezone(&Utc);

        let last_modified_str: String = conn.query_row(
            "SELECT value FROM meta WHERE key = 'last_modified'",
            [],
            |row| row.get(0),
        )?;
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

    fn insert_entry(&mut self, entry: &NewEntry) -> Result<Uuid> {
        let mut conn = self.lock_conn()?;

        let tx = conn.transaction()?;

        let exists: Option<String> = tx
            .query_row(
                "SELECT id FROM entry_types WHERE id = ?",
                [entry.entry_type_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(LedgerError::Validation(
                "Entry type does not exist".to_string(),
            ));
        }

        let schema_json: Option<String> = tx
            .query_row(
                "SELECT schema_json FROM entry_type_versions WHERE entry_type_id = ? AND version = ?",
                (entry.entry_type_id.to_string(), entry.schema_version),
                |row| row.get(0),
            )
            .optional()?;
        let schema_json = if let Some(value) = schema_json {
            value
        } else {
            return Err(LedgerError::Validation(
                "Entry schema version does not exist".to_string(),
            ));
        };
        let schema_value: serde_json::Value = serde_json::from_str(&schema_json)
            .map_err(|e| LedgerError::Storage(format!("Invalid schema JSON: {}", e)))?;
        validate_entry_data(&schema_value, &entry.data)?;

        let normalized_tags = normalize_tags(&entry.tags)?;
        let tags_json =
            if normalized_tags.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&normalized_tags).map_err(|e| {
                    LedgerError::Storage(format!("Failed to serialize tags: {}", e))
                })?)
            };

        let data_json = serde_json::to_string(&entry.data)
            .map_err(|e| LedgerError::Storage(format!("Failed to serialize entry data: {}", e)))?;
        if data_json.len() > MAX_DATA_BYTES {
            return Err(LedgerError::Validation(format!(
                "Entry data too large (max {} bytes)",
                MAX_DATA_BYTES
            )));
        }

        let id = Uuid::new_v4();
        let created_at = entry.created_at.unwrap_or_else(Utc::now);
        let created_at_str = created_at.to_rfc3339();
        let last_modified = Utc::now().to_rfc3339();

        tx.execute(
            r#"
            INSERT INTO entries (
                id,
                entry_type_id,
                schema_version,
                data_json,
                tags_json,
                created_at,
                device_id,
                supersedes
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            (
                id.to_string(),
                entry.entry_type_id.to_string(),
                entry.schema_version,
                data_json,
                tags_json,
                created_at_str.clone(),
                entry.device_id.to_string(),
                entry.supersedes.map(|id| id.to_string()),
            ),
        )?;

        let fts_content = fts_content_for_entry(&entry.data);
        tx.execute(
            "INSERT INTO entries_fts (entry_id, content) VALUES (?, ?)",
            (id.to_string(), fts_content),
        )?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [last_modified],
        )?;

        tx.commit()?;

        Ok(id)
    }

    fn get_entry(&self, id: &Uuid) -> Result<Option<Entry>> {
        let conn = self.lock_conn()?;

        let result = conn.query_row(
            r#"
            SELECT id, entry_type_id, schema_version, data_json, tags_json, created_at, device_id, supersedes
            FROM entries
            WHERE id = ?
            "#,
            [id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            },
        );

        match result {
            Ok((
                id,
                entry_type_id,
                schema_version,
                data_json,
                tags_json,
                created_at,
                device_id,
                supersedes,
            )) => {
                let row = EntryRow {
                    id,
                    entry_type_id,
                    schema_version,
                    data_json,
                    tags_json,
                    created_at,
                    device_id,
                    supersedes,
                };
                Ok(Some(row.try_into()?))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn list_entries(&self, filter: &EntryFilter) -> Result<Vec<Entry>> {
        let conn = self.lock_conn()?;

        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(entry_type_id) = filter.entry_type_id {
            conditions.push("e.entry_type_id = ?".to_string());
            params.push(Box::new(entry_type_id.to_string()));
        }

        if let Some(ref tag) = filter.tag {
            let normalized = normalize_tags(std::slice::from_ref(tag))?;
            let normalized_tag = normalized
                .first()
                .ok_or_else(|| LedgerError::Validation("Invalid tag filter".to_string()))?
                .clone();
            conditions.push(
                "e.tags_json IS NOT NULL AND EXISTS (SELECT 1 FROM json_each(e.tags_json) WHERE value = ?)"
                    .to_string(),
            );
            params.push(Box::new(normalized_tag));
        }

        if let Some(since) = filter.since {
            conditions.push("e.created_at >= ?".to_string());
            params.push(Box::new(since.to_rfc3339()));
        }

        if let Some(until) = filter.until {
            conditions.push("e.created_at <= ?".to_string());
            params.push(Box::new(until.to_rfc3339()));
        }

        if let Some(composition_id) = filter.composition_id {
            conditions.push(
                "EXISTS (SELECT 1 FROM entry_compositions ec WHERE ec.entry_id = e.id AND ec.composition_id = ?)"
                    .to_string(),
            );
            params.push(Box::new(composition_id.to_string()));
        }

        let mut query = String::from(
            "SELECT e.id, e.entry_type_id, e.schema_version, e.data_json, e.tags_json, e.created_at, e.device_id, e.supersedes FROM entries e",
        );
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }
        query.push_str(" ORDER BY e.created_at DESC");

        if let Some(limit) = filter.limit {
            query.push_str(" LIMIT ?");
            params.push(Box::new(limit as i64));
        }

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let (
                id,
                entry_type_id,
                schema_version,
                data_json,
                tags_json,
                created_at,
                device_id,
                supersedes,
            ) = row?;
            let entry_row = EntryRow {
                id,
                entry_type_id,
                schema_version,
                data_json,
                tags_json,
                created_at,
                device_id,
                supersedes,
            };
            entries.push(entry_row.try_into()?);
        }

        Ok(entries)
    }

    fn search_entries(&self, query: &str) -> Result<Vec<Entry>> {
        let conn = self.lock_conn()?;

        let mut stmt = conn.prepare(
            r#"
                SELECT e.id, e.entry_type_id, e.schema_version, e.data_json, e.tags_json,
                       e.created_at, e.device_id, e.supersedes
                FROM entries_fts f
                JOIN entries e ON e.id = f.entry_id
                WHERE entries_fts MATCH ?
                ORDER BY bm25(entries_fts), e.created_at DESC
                "#,
        )?;

        let rows = stmt.query_map([query], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let (
                id,
                entry_type_id,
                schema_version,
                data_json,
                tags_json,
                created_at,
                device_id,
                supersedes,
            ) = row?;
            let entry_row = EntryRow {
                id,
                entry_type_id,
                schema_version,
                data_json,
                tags_json,
                created_at,
                device_id,
                supersedes,
            };
            entries.push(entry_row.try_into()?);
        }

        Ok(entries)
    }

    fn superseded_entry_ids(&self) -> Result<HashSet<Uuid>> {
        let conn = self.lock_conn()?;
        let mut stmt =
            conn.prepare("SELECT DISTINCT supersedes FROM entries WHERE supersedes IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut ids = HashSet::new();
        for row in rows {
            let value = row?;
            let parsed = Uuid::parse_str(&value)
                .map_err(|e| LedgerError::Storage(format!("Invalid supersedes UUID: {}", e)))?;
            ids.insert(parsed);
        }
        Ok(ids)
    }

    fn get_entry_type(&self, name: &str) -> Result<Option<EntryType>> {
        let conn = self.lock_conn()?;

        let result = conn.query_row(
            r#"
            SELECT et.id, et.name, etv.version, etv.created_at, et.device_id, etv.schema_json
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

                Ok((
                    id_str,
                    name,
                    version,
                    created_at_str,
                    device_id_str,
                    schema_json_str,
                ))
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
            Err(e) => Err(e.into()),
        }
    }

    fn create_entry_type(&mut self, entry_type: &NewEntryType) -> Result<Uuid> {
        let mut conn = self.lock_conn()?;

        let tx = conn.transaction()?;

        // Check if entry type with this name already exists
        let base_type_id: Option<String> = tx
            .query_row(
                "SELECT id FROM entry_types WHERE name = ?",
                [&entry_type.name],
                |row| row.get(0),
            )
            .optional()?;

        let (base_id, version) = if let Some(ref id_str) = base_type_id {
            // Entry type exists, get the max version and increment
            let base_id = Uuid::parse_str(id_str)
                .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
            let max_version: i32 = tx.query_row(
                "SELECT MAX(version) FROM entry_type_versions WHERE entry_type_id = ?",
                [id_str],
                |row| row.get(0),
            )?;
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
            )?;
            (base_id, 1)
        };

        // Deactivate previous versions for this entry type.
        tx.execute(
            "UPDATE entry_type_versions SET active = 0 WHERE entry_type_id = ? AND active = 1",
            [base_id.to_string()],
        )?;

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
                created_at.clone(),
            ),
        )?;

        // Update last_modified
        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [created_at],
        )?;

        tx.commit()?;

        Ok(base_id)
    }

    fn list_entry_types(&self) -> Result<Vec<EntryType>> {
        let conn = self.lock_conn()?;

        let mut stmt = conn.prepare(
            r#"
                SELECT et.id, et.name, etv.version, etv.created_at, et.device_id, etv.schema_json
                FROM entry_type_versions etv
                JOIN entry_types et ON et.id = etv.entry_type_id
                WHERE etv.active = 1 AND etv.version = (
                    SELECT MAX(version)
                    FROM entry_type_versions
                    WHERE entry_type_id = etv.entry_type_id AND active = 1
                )
                ORDER BY et.name
                "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let name: String = row.get(1)?;
            let version: i32 = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            let device_id_str: String = row.get(4)?;
            let schema_json_str: String = row.get(5)?;

            Ok((
                id_str,
                name,
                version,
                created_at_str,
                device_id_str,
                schema_json_str,
            ))
        })?;

        let mut entry_types = Vec::new();
        for row in rows {
            let (id_str, name, version, created_at_str, device_id_str, schema_json_str) = row?;

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
        let conn = self.lock_conn()?;

        let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
        let mut rows = stmt.query([])?;
        if rows.next()?.is_some() {
            return Err(LedgerError::Storage(
                "Foreign key integrity check failed".to_string(),
            ));
        }

        let missing_fts: i64 = conn.query_row(
            "SELECT COUNT(*) FROM entries e LEFT JOIN entries_fts f ON e.id = f.entry_id WHERE f.entry_id IS NULL",
            [],
            |row| row.get(0),
        )?;
        if missing_fts > 0 {
            return Err(LedgerError::Storage(
                "FTS index missing entries".to_string(),
            ));
        }

        let orphaned_fts: i64 = conn.query_row(
            "SELECT COUNT(*) FROM entries_fts f LEFT JOIN entries e ON f.entry_id = e.id WHERE e.id IS NULL",
            [],
            |row| row.get(0),
        )?;
        if orphaned_fts > 0 {
            return Err(LedgerError::Storage(
                "FTS index has orphaned rows".to_string(),
            ));
        }

        let invalid_active: i64 = conn.query_row(
            "SELECT COUNT(*) FROM (SELECT 1 FROM entry_type_versions GROUP BY entry_type_id HAVING SUM(active) != 1)",
            [],
            |row| row.get(0),
        )?;
        if invalid_active > 0 {
            return Err(LedgerError::Storage(
                "Entry type versions have invalid active state".to_string(),
            ));
        }

        let metadata_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM meta WHERE key IN ('format_version', 'device_id', 'created_at', 'last_modified')",
            [],
            |row| row.get(0),
        )?;
        if metadata_count < 4 {
            return Err(LedgerError::Storage(
                "Metadata table missing required keys".to_string(),
            ));
        }

        Ok(())
    }

    // --- Composition operations ---

    fn create_composition(&mut self, composition: &NewComposition) -> Result<Uuid> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        // Check if composition with this name already exists
        let exists: Option<String> = tx
            .query_row(
                "SELECT id FROM compositions WHERE name = ?",
                [&composition.name],
                |row| row.get(0),
            )
            .optional()?;

        if exists.is_some() {
            return Err(LedgerError::Validation(format!(
                "Composition '{}' already exists",
                composition.name
            )));
        }

        let id = Uuid::new_v4();
        let created_at = Utc::now().to_rfc3339();
        let metadata_json = composition
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| LedgerError::Storage(format!("Failed to serialize metadata: {}", e)))?;

        tx.execute(
            r#"
            INSERT INTO compositions (id, name, description, created_at, device_id, metadata_json)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            (
                id.to_string(),
                &composition.name,
                &composition.description,
                &created_at,
                composition.device_id.to_string(),
                metadata_json,
            ),
        )?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [&created_at],
        )?;

        tx.commit()?;
        Ok(id)
    }

    fn get_composition(&self, name: &str) -> Result<Option<Composition>> {
        let conn = self.lock_conn()?;

        let result = conn.query_row(
            r#"
            SELECT id, name, description, created_at, device_id, metadata_json
            FROM compositions
            WHERE name = ?
            "#,
            [name],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        );

        match result {
            Ok((id, name, description, created_at, device_id, metadata_json)) => {
                let id = Uuid::parse_str(&id)
                    .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
                let device_id = Uuid::parse_str(&device_id)
                    .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
                let created_at = DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc);
                let metadata = metadata_json
                    .map(|s| serde_json::from_str(&s))
                    .transpose()
                    .map_err(|e| LedgerError::Storage(format!("Invalid metadata JSON: {}", e)))?;

                Ok(Some(Composition {
                    id,
                    name,
                    description,
                    created_at,
                    device_id,
                    metadata,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_composition_by_id(&self, id: &Uuid) -> Result<Option<Composition>> {
        let conn = self.lock_conn()?;

        let result = conn.query_row(
            r#"
            SELECT id, name, description, created_at, device_id, metadata_json
            FROM compositions
            WHERE id = ?
            "#,
            [id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        );

        match result {
            Ok((id_str, name, description, created_at, device_id, metadata_json)) => {
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
                let device_id = Uuid::parse_str(&device_id)
                    .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
                let created_at = DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc);
                let metadata = metadata_json
                    .map(|s| serde_json::from_str(&s))
                    .transpose()
                    .map_err(|e| LedgerError::Storage(format!("Invalid metadata JSON: {}", e)))?;

                Ok(Some(Composition {
                    id,
                    name,
                    description,
                    created_at,
                    device_id,
                    metadata,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn list_compositions(&self, filter: &CompositionFilter) -> Result<Vec<Composition>> {
        let conn = self.lock_conn()?;

        let mut query =
            String::from("SELECT id, name, description, created_at, device_id, metadata_json FROM compositions ORDER BY name");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;

        let mut compositions = Vec::new();
        for row in rows {
            let (id_str, name, description, created_at, device_id, metadata_json) = row?;
            let id = Uuid::parse_str(&id_str)
                .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
            let device_id = Uuid::parse_str(&device_id)
                .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);
            let metadata = metadata_json
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| LedgerError::Storage(format!("Invalid metadata JSON: {}", e)))?;

            compositions.push(Composition {
                id,
                name,
                description,
                created_at,
                device_id,
                metadata,
            });
        }

        Ok(compositions)
    }

    fn rename_composition(&mut self, id: &Uuid, new_name: &str) -> Result<()> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        // Check composition exists
        let exists: Option<String> = tx
            .query_row(
                "SELECT id FROM compositions WHERE id = ?",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        if exists.is_none() {
            return Err(LedgerError::NotFound(format!(
                "Composition {} not found",
                id
            )));
        }

        // Check new name doesn't exist (for a different composition)
        let name_exists: Option<String> = tx
            .query_row(
                "SELECT id FROM compositions WHERE name = ? AND id != ?",
                (new_name, id.to_string()),
                |row| row.get(0),
            )
            .optional()?;

        if name_exists.is_some() {
            return Err(LedgerError::Validation(format!(
                "Composition '{}' already exists",
                new_name
            )));
        }

        let last_modified = Utc::now().to_rfc3339();

        tx.execute(
            "UPDATE compositions SET name = ? WHERE id = ?",
            (new_name, id.to_string()),
        )?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [&last_modified],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn delete_composition(&mut self, id: &Uuid) -> Result<()> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        // Check composition exists
        let exists: Option<String> = tx
            .query_row(
                "SELECT id FROM compositions WHERE id = ?",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        if exists.is_none() {
            return Err(LedgerError::NotFound(format!(
                "Composition {} not found",
                id
            )));
        }

        let last_modified = Utc::now().to_rfc3339();

        // Remove all entry associations
        tx.execute(
            "DELETE FROM entry_compositions WHERE composition_id = ?",
            [id.to_string()],
        )?;

        // Delete the composition
        tx.execute("DELETE FROM compositions WHERE id = ?", [id.to_string()])?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [&last_modified],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn attach_entry_to_composition(
        &mut self,
        entry_id: &Uuid,
        composition_id: &Uuid,
    ) -> Result<()> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        // Check entry exists
        let entry_exists: Option<String> = tx
            .query_row(
                "SELECT id FROM entries WHERE id = ?",
                [entry_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        if entry_exists.is_none() {
            return Err(LedgerError::NotFound(format!(
                "Entry {} not found",
                entry_id
            )));
        }

        // Check composition exists
        let comp_exists: Option<String> = tx
            .query_row(
                "SELECT id FROM compositions WHERE id = ?",
                [composition_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        if comp_exists.is_none() {
            return Err(LedgerError::NotFound(format!(
                "Composition {} not found",
                composition_id
            )));
        }

        // Check if already attached
        let already_attached: Option<String> = tx
            .query_row(
                "SELECT entry_id FROM entry_compositions WHERE entry_id = ? AND composition_id = ?",
                (entry_id.to_string(), composition_id.to_string()),
                |row| row.get(0),
            )
            .optional()?;

        if already_attached.is_some() {
            // Already attached, no-op
            return Ok(());
        }

        let added_at = Utc::now().to_rfc3339();

        tx.execute(
            "INSERT INTO entry_compositions (entry_id, composition_id, added_at) VALUES (?, ?, ?)",
            (entry_id.to_string(), composition_id.to_string(), &added_at),
        )?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [&added_at],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn detach_entry_from_composition(
        &mut self,
        entry_id: &Uuid,
        composition_id: &Uuid,
    ) -> Result<()> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        let deleted = tx.execute(
            "DELETE FROM entry_compositions WHERE entry_id = ? AND composition_id = ?",
            (entry_id.to_string(), composition_id.to_string()),
        )?;

        if deleted == 0 {
            return Err(LedgerError::NotFound(format!(
                "Entry {} is not attached to composition {}",
                entry_id, composition_id
            )));
        }

        let last_modified = Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [&last_modified],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn get_entry_compositions(&self, entry_id: &Uuid) -> Result<Vec<Composition>> {
        let conn = self.lock_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT c.id, c.name, c.description, c.created_at, c.device_id, c.metadata_json
            FROM compositions c
            JOIN entry_compositions ec ON c.id = ec.composition_id
            WHERE ec.entry_id = ?
            ORDER BY c.name
            "#,
        )?;

        let rows = stmt.query_map([entry_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;

        let mut compositions = Vec::new();
        for row in rows {
            let (id_str, name, description, created_at, device_id, metadata_json) = row?;
            let id = Uuid::parse_str(&id_str)
                .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
            let device_id = Uuid::parse_str(&device_id)
                .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);
            let metadata = metadata_json
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| LedgerError::Storage(format!("Invalid metadata JSON: {}", e)))?;

            compositions.push(Composition {
                id,
                name,
                description,
                created_at,
                device_id,
                metadata,
            });
        }

        Ok(compositions)
    }

    fn get_composition_entries(&self, composition_id: &Uuid) -> Result<Vec<EntryComposition>> {
        let conn = self.lock_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT entry_id, composition_id, added_at
            FROM entry_compositions
            WHERE composition_id = ?
            ORDER BY added_at DESC
            "#,
        )?;

        let rows = stmt.query_map([composition_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut entry_compositions = Vec::new();
        for row in rows {
            let (entry_id, comp_id, added_at) = row?;
            let entry_id = Uuid::parse_str(&entry_id)
                .map_err(|e| LedgerError::Storage(format!("Invalid entry UUID: {}", e)))?;
            let composition_id = Uuid::parse_str(&comp_id)
                .map_err(|e| LedgerError::Storage(format!("Invalid composition UUID: {}", e)))?;
            let added_at = DateTime::parse_from_rfc3339(&added_at)
                .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);

            entry_compositions.push(EntryComposition {
                entry_id,
                composition_id,
                added_at,
            });
        }

        Ok(entry_compositions)
    }

    // --- Template operations ---

    fn create_template(&mut self, template: &NewTemplate) -> Result<Uuid> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        // Check if template with this name already exists
        let exists: Option<String> = tx
            .query_row(
                "SELECT id FROM templates WHERE name = ?",
                [&template.name],
                |row| row.get(0),
            )
            .optional()?;

        if exists.is_some() {
            return Err(LedgerError::Validation(format!(
                "Template '{}' already exists",
                template.name
            )));
        }

        // Check entry type exists
        let entry_type_exists: Option<String> = tx
            .query_row(
                "SELECT id FROM entry_types WHERE id = ?",
                [template.entry_type_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        if entry_type_exists.is_none() {
            return Err(LedgerError::Validation(format!(
                "Entry type {} does not exist",
                template.entry_type_id
            )));
        }

        let id = Uuid::new_v4();
        let created_at = Utc::now().to_rfc3339();

        // Create base template record
        tx.execute(
            r#"
            INSERT INTO templates (id, name, entry_type_id, description, created_at, device_id)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            (
                id.to_string(),
                &template.name,
                template.entry_type_id.to_string(),
                &template.description,
                &created_at,
                template.device_id.to_string(),
            ),
        )?;

        // Create first version
        let version_id = Uuid::new_v4();
        let template_json_str = serde_json::to_string(&template.template_json)
            .map_err(|e| LedgerError::Storage(format!("Failed to serialize template: {}", e)))?;

        tx.execute(
            r#"
            INSERT INTO template_versions (id, template_id, version, template_json, created_at, active)
            VALUES (?, ?, 1, ?, ?, 1)
            "#,
            (version_id.to_string(), id.to_string(), template_json_str, &created_at),
        )?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [&created_at],
        )?;

        tx.commit()?;
        Ok(id)
    }

    fn get_template(&self, name: &str) -> Result<Option<Template>> {
        let conn = self.lock_conn()?;

        let result = conn.query_row(
            r#"
            SELECT t.id, t.name, t.entry_type_id, tv.version, tv.created_at, t.device_id, t.description, tv.template_json
            FROM template_versions tv
            JOIN templates t ON t.id = tv.template_id
            WHERE t.name = ? AND tv.active = 1
            ORDER BY tv.version DESC
            LIMIT 1
            "#,
            [name],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i32>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        );

        match result {
            Ok((
                id,
                name,
                entry_type_id,
                version,
                created_at,
                device_id,
                description,
                template_json,
            )) => {
                let id = Uuid::parse_str(&id)
                    .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
                let entry_type_id = Uuid::parse_str(&entry_type_id)
                    .map_err(|e| LedgerError::Storage(format!("Invalid entry_type_id: {}", e)))?;
                let device_id = Uuid::parse_str(&device_id)
                    .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
                let created_at = DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc);
                let template_json: serde_json::Value = serde_json::from_str(&template_json)
                    .map_err(|e| LedgerError::Storage(format!("Invalid template JSON: {}", e)))?;

                Ok(Some(Template {
                    id,
                    name,
                    entry_type_id,
                    version,
                    created_at,
                    device_id,
                    description,
                    template_json,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn get_template_by_id(&self, id: &Uuid) -> Result<Option<Template>> {
        let conn = self.lock_conn()?;

        let result = conn.query_row(
            r#"
            SELECT t.id, t.name, t.entry_type_id, tv.version, tv.created_at, t.device_id, t.description, tv.template_json
            FROM template_versions tv
            JOIN templates t ON t.id = tv.template_id
            WHERE t.id = ? AND tv.active = 1
            ORDER BY tv.version DESC
            LIMIT 1
            "#,
            [id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i32>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        );

        match result {
            Ok((
                id_str,
                name,
                entry_type_id,
                version,
                created_at,
                device_id,
                description,
                template_json,
            )) => {
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
                let entry_type_id = Uuid::parse_str(&entry_type_id)
                    .map_err(|e| LedgerError::Storage(format!("Invalid entry_type_id: {}", e)))?;
                let device_id = Uuid::parse_str(&device_id)
                    .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
                let created_at = DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc);
                let template_json: serde_json::Value = serde_json::from_str(&template_json)
                    .map_err(|e| LedgerError::Storage(format!("Invalid template JSON: {}", e)))?;

                Ok(Some(Template {
                    id,
                    name,
                    entry_type_id,
                    version,
                    created_at,
                    device_id,
                    description,
                    template_json,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn list_templates(&self) -> Result<Vec<Template>> {
        let conn = self.lock_conn()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT t.id, t.name, t.entry_type_id, tv.version, tv.created_at, t.device_id, t.description, tv.template_json
            FROM template_versions tv
            JOIN templates t ON t.id = tv.template_id
            WHERE tv.active = 1 AND tv.version = (
                SELECT MAX(version)
                FROM template_versions
                WHERE template_id = tv.template_id AND active = 1
            )
            ORDER BY t.name
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;

        let mut templates = Vec::new();
        for row in rows {
            let (
                id_str,
                name,
                entry_type_id,
                version,
                created_at,
                device_id,
                description,
                template_json,
            ) = row?;

            let id = Uuid::parse_str(&id_str)
                .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
            let entry_type_id = Uuid::parse_str(&entry_type_id)
                .map_err(|e| LedgerError::Storage(format!("Invalid entry_type_id: {}", e)))?;
            let device_id = Uuid::parse_str(&device_id)
                .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);
            let template_json: serde_json::Value = serde_json::from_str(&template_json)
                .map_err(|e| LedgerError::Storage(format!("Invalid template JSON: {}", e)))?;

            templates.push(Template {
                id,
                name,
                entry_type_id,
                version,
                created_at,
                device_id,
                description,
                template_json,
            });
        }

        Ok(templates)
    }

    fn update_template(&mut self, id: &Uuid, template_json: serde_json::Value) -> Result<i32> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        // Check template exists and get max version
        let max_version: Option<i32> = tx
            .query_row(
                "SELECT MAX(version) FROM template_versions WHERE template_id = ?",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

        let max_version = max_version
            .ok_or_else(|| LedgerError::NotFound(format!("Template {} not found", id)))?;

        let new_version = max_version + 1;
        let created_at = Utc::now().to_rfc3339();
        let version_id = Uuid::new_v4();

        // Deactivate old versions
        tx.execute(
            "UPDATE template_versions SET active = 0 WHERE template_id = ? AND active = 1",
            [id.to_string()],
        )?;

        // Create new version
        let template_json_str = serde_json::to_string(&template_json)
            .map_err(|e| LedgerError::Storage(format!("Failed to serialize template: {}", e)))?;

        tx.execute(
            r#"
            INSERT INTO template_versions (id, template_id, version, template_json, created_at, active)
            VALUES (?, ?, ?, ?, ?, 1)
            "#,
            (
                version_id.to_string(),
                id.to_string(),
                new_version,
                template_json_str,
                &created_at,
            ),
        )?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [&created_at],
        )?;

        tx.commit()?;
        Ok(new_version)
    }

    fn delete_template(&mut self, id: &Uuid) -> Result<()> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        // Check template exists
        let exists: Option<String> = tx
            .query_row(
                "SELECT id FROM templates WHERE id = ?",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        if exists.is_none() {
            return Err(LedgerError::NotFound(format!("Template {} not found", id)));
        }

        let last_modified = Utc::now().to_rfc3339();

        // Remove default template mappings
        tx.execute(
            "DELETE FROM entry_type_templates WHERE template_id = ?",
            [id.to_string()],
        )?;

        // Remove all versions
        tx.execute(
            "DELETE FROM template_versions WHERE template_id = ?",
            [id.to_string()],
        )?;

        // Delete the template
        tx.execute("DELETE FROM templates WHERE id = ?", [id.to_string()])?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [&last_modified],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn set_default_template(&mut self, entry_type_id: &Uuid, template_id: &Uuid) -> Result<()> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        // Check entry type exists
        let entry_type_exists: Option<String> = tx
            .query_row(
                "SELECT id FROM entry_types WHERE id = ?",
                [entry_type_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        if entry_type_exists.is_none() {
            return Err(LedgerError::NotFound(format!(
                "Entry type {} not found",
                entry_type_id
            )));
        }

        // Check template exists and is for this entry type
        let template_entry_type_id: Option<String> = tx
            .query_row(
                "SELECT entry_type_id FROM templates WHERE id = ?",
                [template_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        let template_entry_type_id = template_entry_type_id
            .ok_or_else(|| LedgerError::NotFound(format!("Template {} not found", template_id)))?;

        if template_entry_type_id != entry_type_id.to_string() {
            return Err(LedgerError::Validation(format!(
                "Template {} is not for entry type {}",
                template_id, entry_type_id
            )));
        }

        let last_modified = Utc::now().to_rfc3339();

        // Deactivate existing default for this entry type
        tx.execute(
            "UPDATE entry_type_templates SET active = 0 WHERE entry_type_id = ? AND active = 1",
            [entry_type_id.to_string()],
        )?;

        // Insert or update the mapping
        tx.execute(
            r#"
            INSERT INTO entry_type_templates (entry_type_id, template_id, active)
            VALUES (?, ?, 1)
            ON CONFLICT(entry_type_id, template_id) DO UPDATE SET active = 1
            "#,
            (entry_type_id.to_string(), template_id.to_string()),
        )?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [&last_modified],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn clear_default_template(&mut self, entry_type_id: &Uuid) -> Result<()> {
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;

        // Check entry type exists
        let entry_type_exists: Option<String> = tx
            .query_row(
                "SELECT id FROM entry_types WHERE id = ?",
                [entry_type_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        if entry_type_exists.is_none() {
            return Err(LedgerError::NotFound(format!(
                "Entry type {} not found",
                entry_type_id
            )));
        }

        let last_modified = Utc::now().to_rfc3339();

        tx.execute(
            "UPDATE entry_type_templates SET active = 0 WHERE entry_type_id = ? AND active = 1",
            [entry_type_id.to_string()],
        )?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [&last_modified],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn get_default_template(&self, entry_type_id: &Uuid) -> Result<Option<Template>> {
        let conn = self.lock_conn()?;

        // Join through entry_type_templates to get the default template directly
        // This avoids a second lock acquisition from calling get_template_by_id
        let result = conn.query_row(
            r#"
            SELECT t.id, t.name, t.entry_type_id, tv.version, tv.created_at, t.device_id, t.description, tv.template_json
            FROM entry_type_templates ett
            JOIN templates t ON t.id = ett.template_id
            JOIN template_versions tv ON tv.template_id = t.id AND tv.active = 1
            WHERE ett.entry_type_id = ? AND ett.active = 1
            ORDER BY tv.version DESC
            LIMIT 1
            "#,
            [entry_type_id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i32>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        );

        match result {
            Ok((
                id_str,
                name,
                entry_type_id_str,
                version,
                created_at,
                device_id,
                description,
                template_json,
            )) => {
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
                let entry_type_id = Uuid::parse_str(&entry_type_id_str)
                    .map_err(|e| LedgerError::Storage(format!("Invalid entry_type_id: {}", e)))?;
                let device_id = Uuid::parse_str(&device_id)
                    .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
                let created_at = DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc);
                let template_json: serde_json::Value = serde_json::from_str(&template_json)
                    .map_err(|e| LedgerError::Storage(format!("Invalid template JSON: {}", e)))?;

                Ok(Some(Template {
                    id,
                    name,
                    entry_type_id,
                    version,
                    created_at,
                    device_id,
                    description,
                    template_json,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(LedgerError::Storage(format!("Database error: {}", e))),
        }
    }
}
