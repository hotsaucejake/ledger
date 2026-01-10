//! Output formatting helpers for the CLI.

use std::collections::HashMap;

use ledger_core::storage::{AgeSqliteStorage, Entry, StorageEngine};
use uuid::Uuid;

/// Build a map of entry type ID -> name for display.
pub fn entry_type_name_map(storage: &AgeSqliteStorage) -> anyhow::Result<HashMap<Uuid, String>> {
    let types = storage.list_entry_types()?;
    let mut map = HashMap::new();
    for entry_type in types {
        map.insert(entry_type.id, entry_type.name);
    }
    Ok(map)
}

/// Extract a summary from an entry's data, preferring the "body" field.
pub fn entry_summary(entry: &Entry) -> String {
    entry
        .data
        .get("body")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| entry.data.to_string())
}

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

/// Print a single entry in human-readable format.
pub fn print_entry(storage: &AgeSqliteStorage, entry: &Entry, quiet: bool) -> anyhow::Result<()> {
    let name_map = entry_type_name_map(storage)?;
    let entry_type_name = name_map
        .get(&entry.entry_type_id)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let body = entry_summary(entry);

    if !quiet {
        println!("ID: {}", entry.id);
        println!("Type: {} (v{})", entry_type_name, entry.schema_version);
        println!("Created: {}", entry.created_at);
        println!("Device: {}", entry.device_id);
        if !entry.tags.is_empty() {
            println!("Tags: {}", entry.tags.join(", "));
        }
        println!();
    }
    println!("{}", body);
    Ok(())
}
