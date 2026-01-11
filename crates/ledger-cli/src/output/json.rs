//! JSON output formatting for entries.

use std::collections::HashMap;

use ledger_core::storage::Entry;
use uuid::Uuid;

/// Convert an entry to JSON for output.
pub fn entry_json(entry: &Entry, name_map: &HashMap<Uuid, String>) -> serde_json::Value {
    let entry_type_name = name_map
        .get(&entry.entry_type_id)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    serde_json::json!({
        "id": entry.id,
        "entry_type_id": entry.entry_type_id,
        "entry_type_name": entry_type_name,
        "schema_version": entry.schema_version,
        "created_at": entry.created_at,
        "device_id": entry.device_id,
        "tags": entry.tags,
        "data": entry.data,
        "supersedes": entry.supersedes,
    })
}

/// Convert multiple entries to JSON array for output.
pub fn entries_json(entries: &[Entry], name_map: &HashMap<Uuid, String>) -> Vec<serde_json::Value> {
    entries
        .iter()
        .map(|entry| entry_json(entry, name_map))
        .collect()
}
