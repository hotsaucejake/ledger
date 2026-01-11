//! Entry row type for database queries.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::{LedgerError, Result};
use crate::storage::types::Entry;

/// Raw row data from the entries table, before parsing into domain types.
#[derive(Debug)]
pub struct EntryRow {
    pub id: String,
    pub entry_type_id: String,
    pub schema_version: i32,
    pub data_json: String,
    pub tags_json: Option<String>,
    pub created_at: String,
    pub device_id: String,
    pub supersedes: Option<String>,
}

impl TryFrom<EntryRow> for Entry {
    type Error = LedgerError;

    fn try_from(row: EntryRow) -> Result<Self> {
        let id = Uuid::parse_str(&row.id)
            .map_err(|e| LedgerError::Storage(format!("Invalid entry UUID: {}", e)))?;
        let entry_type_id = Uuid::parse_str(&row.entry_type_id)
            .map_err(|e| LedgerError::Storage(format!("Invalid entry_type UUID: {}", e)))?;
        let device_id = Uuid::parse_str(&row.device_id)
            .map_err(|e| LedgerError::Storage(format!("Invalid device_id: {}", e)))?;
        let created_at = DateTime::parse_from_rfc3339(&row.created_at)
            .map_err(|e| LedgerError::Storage(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&Utc);
        let data: serde_json::Value = serde_json::from_str(&row.data_json)
            .map_err(|e| LedgerError::Storage(format!("Invalid JSON: {}", e)))?;
        let tags: Vec<String> = match row.tags_json {
            Some(ref value) => serde_json::from_str(value)
                .map_err(|e| LedgerError::Storage(format!("Invalid tags JSON: {}", e)))?,
            None => Vec::new(),
        };
        let supersedes = row
            .supersedes
            .as_ref()
            .map(|s| {
                Uuid::parse_str(s)
                    .map_err(|e| LedgerError::Storage(format!("Invalid supersedes UUID: {}", e)))
            })
            .transpose()?;

        Ok(Entry {
            id,
            entry_type_id,
            schema_version: row.schema_version,
            data,
            tags,
            created_at,
            device_id,
            supersedes,
        })
    }
}
