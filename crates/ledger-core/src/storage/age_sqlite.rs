//! Age-encrypted SQLite storage backend.
//!
//! This is a skeleton implementation that wires create/open/close
//! with Age passphrase encryption. SQLite schema and entry operations
//! will be added in subsequent steps.

use std::fs;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::Mutex;

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName, OptionalExtension};
use uuid::Uuid;

use age::secrecy::{ExposeSecret, SecretString};

use crate::crypto::validate_passphrase;
use crate::error::{LedgerError, Result};
use crate::storage::encryption::{decrypt, encrypt};
use crate::storage::traits::StorageEngine;
use crate::storage::types::{
    Entry, EntryFilter, EntryType, LedgerMetadata, NewEntry, NewEntryType,
};

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
    const MAX_TAG_BYTES: usize = 128;
    const MAX_TAGS_PER_ENTRY: usize = 100;
    const MAX_DATA_BYTES: usize = 1024 * 1024;

    fn sqlite_error(err: rusqlite::Error) -> LedgerError {
        LedgerError::Storage(format!("SQLite error: {}", err))
    }

    fn normalize_tags(tags: &[String]) -> Result<Vec<String>> {
        if tags.len() > Self::MAX_TAGS_PER_ENTRY {
            return Err(LedgerError::Validation(format!(
                "Too many tags (max {})",
                Self::MAX_TAGS_PER_ENTRY
            )));
        }

        let mut normalized = Vec::new();
        for tag in tags {
            let trimmed = tag.trim().to_ascii_lowercase();
            if trimmed.is_empty() {
                return Err(LedgerError::Validation(
                    "Empty tag is not allowed".to_string(),
                ));
            }
            if trimmed.len() > Self::MAX_TAG_BYTES {
                return Err(LedgerError::Validation(format!(
                    "Tag too long (max {} bytes)",
                    Self::MAX_TAG_BYTES
                )));
            }
            if !trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
            {
                return Err(LedgerError::Validation(
                    "Tag contains invalid characters".to_string(),
                ));
            }
            if !normalized.contains(&trimmed) {
                normalized.push(trimmed);
            }
        }

        Ok(normalized)
    }

    #[allow(clippy::too_many_arguments)]
    fn entry_from_row(
        id_str: String,
        entry_type_id_str: String,
        schema_version: i32,
        data_json_str: String,
        tags_json_str: Option<String>,
        created_at_str: String,
        device_id_str: String,
        supersedes_str: Option<String>,
    ) -> Result<Entry> {
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
        let entry_type_id = Uuid::parse_str(&entry_type_id_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?;
        let device_id = Uuid::parse_str(&device_id_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&Utc);
        let data: serde_json::Value = serde_json::from_str(&data_json_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid JSON: {}", e)))?;
        let tags: Vec<String> = match tags_json_str {
            Some(value) => serde_json::from_str(&value)
                .map_err(|e| LedgerError::Storage(format!("Invalid tags JSON: {}", e)))?,
            None => Vec::new(),
        };
        let supersedes = match supersedes_str {
            Some(value) => Some(
                Uuid::parse_str(&value)
                    .map_err(|e| LedgerError::Storage(format!("Invalid UUID: {}", e)))?,
            ),
            None => None,
        };

        Ok(Entry {
            id,
            entry_type_id,
            schema_version,
            data,
            tags,
            created_at,
            device_id,
            supersedes,
        })
    }

    fn fts_content_for_entry(data: &serde_json::Value) -> String {
        data.get("body")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| data.to_string())
    }

    fn validate_entry_data(
        schema_json: &serde_json::Value,
        data: &serde_json::Value,
    ) -> Result<()> {
        let fields = schema_json
            .get("fields")
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                LedgerError::Validation("Schema fields missing or invalid".to_string())
            })?;

        let data_obj = data.as_object().ok_or_else(|| {
            LedgerError::Validation("Entry data must be a JSON object".to_string())
        })?;

        let mut allowed_fields = Vec::new();

        for field in fields {
            let name = field
                .get("name")
                .and_then(|value| value.as_str())
                .ok_or_else(|| LedgerError::Validation("Schema field name missing".to_string()))?;
            allowed_fields.push(name.to_string());
            let field_type = field
                .get("type")
                .and_then(|value| value.as_str())
                .ok_or_else(|| LedgerError::Validation("Schema field type missing".to_string()))?;
            let required = field
                .get("required")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let nullable = field
                .get("nullable")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);

            let value = data_obj.get(name);
            if value.is_none() {
                if required {
                    return Err(LedgerError::Validation(format!(
                        "Missing required field: {}",
                        name
                    )));
                }
                continue;
            }

            let value = value.unwrap();
            if value.is_null() {
                if !nullable {
                    return Err(LedgerError::Validation(format!(
                        "Field {} cannot be null",
                        name
                    )));
                }
                continue;
            }

            match field_type {
                "string" | "text" => {
                    if !value.is_string() {
                        return Err(LedgerError::Validation(format!(
                            "Field {} must be a string",
                            name
                        )));
                    }
                }
                "number" => {
                    if !value.is_number() {
                        return Err(LedgerError::Validation(format!(
                            "Field {} must be a number",
                            name
                        )));
                    }
                }
                "integer" => {
                    if value.as_i64().is_none() {
                        return Err(LedgerError::Validation(format!(
                            "Field {} must be an integer",
                            name
                        )));
                    }
                }
                "boolean" => {
                    if !value.is_boolean() {
                        return Err(LedgerError::Validation(format!(
                            "Field {} must be a boolean",
                            name
                        )));
                    }
                }
                "date" => {
                    let raw = value.as_str().ok_or_else(|| {
                        LedgerError::Validation(format!("Field {} must be a date string", name))
                    })?;
                    if NaiveDate::parse_from_str(raw, "%Y-%m-%d").is_err() {
                        return Err(LedgerError::Validation(format!(
                            "Field {} must be YYYY-MM-DD",
                            name
                        )));
                    }
                }
                "datetime" => {
                    let raw = value.as_str().ok_or_else(|| {
                        LedgerError::Validation(format!(
                            "Field {} must be an ISO-8601 string",
                            name
                        ))
                    })?;
                    if DateTime::parse_from_rfc3339(raw).is_err() {
                        return Err(LedgerError::Validation(format!(
                            "Field {} must be ISO-8601",
                            name
                        )));
                    }
                }
                other => {
                    return Err(LedgerError::Validation(format!(
                        "Unsupported field type: {}",
                        other
                    )))
                }
            }
        }

        for key in data_obj.keys() {
            if !allowed_fields.contains(key) {
                return Err(LedgerError::Validation(format!("Unknown field: {}", key)));
            }
        }

        Ok(())
    }

    fn empty_payload(conn: &Connection) -> Result<Vec<u8>> {
        let data = conn
            .serialize(DatabaseName::Main)
            .map_err(Self::sqlite_error)?;
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
        let raw = unsafe { rusqlite::ffi::sqlite3_malloc(size) as *mut u8 };
        if raw.is_null() {
            return Err(LedgerError::Storage("SQLite allocation failed".to_string()));
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
            return Err(LedgerError::Storage(
                "Ledger file already exists".to_string(),
            ));
        }

        validate_passphrase(passphrase)?;

        let device_id = Uuid::new_v4();
        let conn = Connection::open_in_memory().map_err(Self::sqlite_error)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(Self::sqlite_error)?;

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
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(Self::sqlite_error)?;
        let owned_data = Self::owned_data_from_bytes(&plaintext)?;
        conn.deserialize(DatabaseName::Main, owned_data, false)
            .map_err(Self::sqlite_error)?;

        // Read device_id from metadata
        let device_id_str: String = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'device_id'",
                [],
                |row| row.get(0),
            )
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
            .query_row(
                "SELECT value FROM meta WHERE key = 'format_version'",
                [],
                |row| row.get(0),
            )
            .map_err(Self::sqlite_error)?;

        let created_at_str: String = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'created_at'",
                [],
                |row| row.get(0),
            )
            .map_err(Self::sqlite_error)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| LedgerError::Storage(format!("Invalid created_at timestamp: {}", e)))?
            .with_timezone(&Utc);

        let last_modified_str: String = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'last_modified'",
                [],
                |row| row.get(0),
            )
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

    fn insert_entry(&mut self, entry: &NewEntry) -> Result<Uuid> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

        let tx = conn.transaction().map_err(Self::sqlite_error)?;

        let exists: Option<String> = tx
            .query_row(
                "SELECT id FROM entry_types WHERE id = ?",
                [entry.entry_type_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Self::sqlite_error)?;
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
            .optional()
            .map_err(Self::sqlite_error)?;
        let schema_json = if let Some(value) = schema_json {
            value
        } else {
            return Err(LedgerError::Validation(
                "Entry schema version does not exist".to_string(),
            ));
        };
        let schema_value: serde_json::Value = serde_json::from_str(&schema_json)
            .map_err(|e| LedgerError::Storage(format!("Invalid schema JSON: {}", e)))?;
        Self::validate_entry_data(&schema_value, &entry.data)?;

        let normalized_tags = Self::normalize_tags(&entry.tags)?;
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
        if data_json.len() > Self::MAX_DATA_BYTES {
            return Err(LedgerError::Validation(format!(
                "Entry data too large (max {} bytes)",
                Self::MAX_DATA_BYTES
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
        )
        .map_err(Self::sqlite_error)?;

        let fts_content = Self::fts_content_for_entry(&entry.data);
        tx.execute(
            "INSERT INTO entries_fts (entry_id, content) VALUES (?, ?)",
            (id.to_string(), fts_content),
        )
        .map_err(Self::sqlite_error)?;

        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [last_modified],
        )
        .map_err(Self::sqlite_error)?;

        tx.commit().map_err(Self::sqlite_error)?;

        Ok(id)
    }

    fn get_entry(&self, id: &Uuid) -> Result<Option<Entry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

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
                id_str,
                entry_type_id_str,
                schema_version,
                data_json_str,
                tags_json_str,
                created_at_str,
                device_id_str,
                supersedes_str,
            )) => Ok(Some(Self::entry_from_row(
                id_str,
                entry_type_id_str,
                schema_version,
                data_json_str,
                tags_json_str,
                created_at_str,
                device_id_str,
                supersedes_str,
            )?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Self::sqlite_error(e)),
        }
    }

    fn list_entries(&self, filter: &EntryFilter) -> Result<Vec<Entry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(entry_type_id) = filter.entry_type_id {
            conditions.push("entry_type_id = ?".to_string());
            params.push(Box::new(entry_type_id.to_string()));
        }

        if let Some(ref tag) = filter.tag {
            let normalized = Self::normalize_tags(std::slice::from_ref(tag))?;
            let normalized_tag = normalized
                .first()
                .ok_or_else(|| LedgerError::Validation("Invalid tag filter".to_string()))?
                .clone();
            conditions.push(
                "tags_json IS NOT NULL AND EXISTS (SELECT 1 FROM json_each(tags_json) WHERE value = ?)"
                    .to_string(),
            );
            params.push(Box::new(normalized_tag));
        }

        if let Some(since) = filter.since {
            conditions.push("created_at >= ?".to_string());
            params.push(Box::new(since.to_rfc3339()));
        }

        if let Some(until) = filter.until {
            conditions.push("created_at <= ?".to_string());
            params.push(Box::new(until.to_rfc3339()));
        }

        let mut query = String::from(
            "SELECT id, entry_type_id, schema_version, data_json, tags_json, created_at, device_id, supersedes FROM entries",
        );
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }
        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            query.push_str(" LIMIT ?");
            params.push(Box::new(limit as i64));
        }

        let mut stmt = conn.prepare(&query).map_err(Self::sqlite_error)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
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
            })
            .map_err(Self::sqlite_error)?;

        let mut entries = Vec::new();
        for row in rows {
            let (
                id_str,
                entry_type_id_str,
                schema_version,
                data_json_str,
                tags_json_str,
                created_at_str,
                device_id_str,
                supersedes_str,
            ) = row.map_err(Self::sqlite_error)?;

            entries.push(Self::entry_from_row(
                id_str,
                entry_type_id_str,
                schema_version,
                data_json_str,
                tags_json_str,
                created_at_str,
                device_id_str,
                supersedes_str,
            )?);
        }

        Ok(entries)
    }

    fn search_entries(&self, query: &str) -> Result<Vec<Entry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT e.id, e.entry_type_id, e.schema_version, e.data_json, e.tags_json,
                       e.created_at, e.device_id, e.supersedes
                FROM entries_fts f
                JOIN entries e ON e.id = f.entry_id
                WHERE entries_fts MATCH ?
                ORDER BY bm25(entries_fts), e.created_at DESC
                "#,
            )
            .map_err(Self::sqlite_error)?;

        let rows = stmt
            .query_map([query], |row| {
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
            })
            .map_err(Self::sqlite_error)?;

        let mut entries = Vec::new();
        for row in rows {
            let (
                id_str,
                entry_type_id_str,
                schema_version,
                data_json_str,
                tags_json_str,
                created_at_str,
                device_id_str,
                supersedes_str,
            ) = row.map_err(Self::sqlite_error)?;

            entries.push(Self::entry_from_row(
                id_str,
                entry_type_id_str,
                schema_version,
                data_json_str,
                tags_json_str,
                created_at_str,
                device_id_str,
                supersedes_str,
            )?);
        }

        Ok(entries)
    }

    fn get_entry_type(&self, name: &str) -> Result<Option<EntryType>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

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
            Err(e) => Err(Self::sqlite_error(e)),
        }
    }

    fn create_entry_type(&mut self, entry_type: &NewEntryType) -> Result<Uuid> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

        let tx = conn.transaction().map_err(Self::sqlite_error)?;

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

        // Deactivate previous versions for this entry type.
        tx.execute(
            "UPDATE entry_type_versions SET active = 0 WHERE entry_type_id = ? AND active = 1",
            [base_id.to_string()],
        )
        .map_err(Self::sqlite_error)?;

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
        )
        .map_err(Self::sqlite_error)?;

        // Update last_modified
        tx.execute(
            "UPDATE meta SET value = ? WHERE key = 'last_modified'",
            [created_at],
        )
        .map_err(Self::sqlite_error)?;

        tx.commit().map_err(Self::sqlite_error)?;

        Ok(base_id)
    }

    fn list_entry_types(&self) -> Result<Vec<EntryType>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

        let mut stmt = conn
            .prepare(
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

                Ok((
                    id_str,
                    name,
                    version,
                    created_at_str,
                    device_id_str,
                    schema_json_str,
                ))
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
        let conn = self
            .conn
            .lock()
            .map_err(|_| LedgerError::Storage("SQLite connection poisoned".to_string()))?;

        let mut stmt = conn
            .prepare("PRAGMA foreign_key_check")
            .map_err(Self::sqlite_error)?;
        let mut rows = stmt.query([]).map_err(Self::sqlite_error)?;
        if rows.next().map_err(Self::sqlite_error)?.is_some() {
            return Err(LedgerError::Storage(
                "Foreign key integrity check failed".to_string(),
            ));
        }

        let missing_fts: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entries e LEFT JOIN entries_fts f ON e.id = f.entry_id WHERE f.entry_id IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(Self::sqlite_error)?;
        if missing_fts > 0 {
            return Err(LedgerError::Storage(
                "FTS index missing entries".to_string(),
            ));
        }

        let orphaned_fts: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entries_fts f LEFT JOIN entries e ON f.entry_id = e.id WHERE e.id IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(Self::sqlite_error)?;
        if orphaned_fts > 0 {
            return Err(LedgerError::Storage(
                "FTS index has orphaned rows".to_string(),
            ));
        }

        let invalid_active: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM (SELECT 1 FROM entry_type_versions GROUP BY entry_type_id HAVING SUM(active) != 1)",
                [],
                |row| row.get(0),
            )
            .map_err(Self::sqlite_error)?;
        if invalid_active > 0 {
            return Err(LedgerError::Storage(
                "Entry type versions have invalid active state".to_string(),
            ));
        }

        let metadata_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM meta WHERE key IN ('format_version', 'device_id', 'created_at', 'last_modified')",
                [],
                |row| row.get(0),
            )
            .map_err(Self::sqlite_error)?;
        if metadata_count < 4 {
            return Err(LedgerError::Storage(
                "Metadata table missing required keys".to_string(),
            ));
        }

        Ok(())
    }
}
