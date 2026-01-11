//! Text and table output formatting for entries.

use std::collections::HashMap;

use ledger_core::storage::{AgeSqliteStorage, Entry, StorageEngine};
use uuid::Uuid;

use crate::helpers::OutputFormat;

use super::json::entries_json;

const TABLE_SUMMARY_MAX: usize = 80;

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

/// Extract a summary for table output with truncation.
pub fn entry_table_summary(entry: &Entry, max_len: usize) -> String {
    let summary = entry_summary(entry);
    if summary.len() <= max_len {
        return summary;
    }
    if max_len <= 3 {
        return summary.chars().take(max_len).collect();
    }
    let trimmed: String = summary.chars().take(max_len - 3).collect();
    format!("{}...", trimmed)
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
        if let Some(supersedes) = entry.supersedes {
            println!("Supersedes: {}", supersedes);
        }
        println!();
        println!("---");
        println!();
    }
    println!("{}", body);
    Ok(())
}

/// Print a list of entries in the requested format.
///
/// This consolidates the common output logic used by list and search commands.
/// Supports JSON output and table/plain text formats.
pub fn print_entry_list(
    storage: &AgeSqliteStorage,
    entries: &[Entry],
    json: bool,
    format: Option<OutputFormat>,
    quiet: bool,
) -> anyhow::Result<()> {
    if json {
        if format.is_some() {
            return Err(anyhow::anyhow!("--format cannot be used with --json"));
        }
        let name_map = entry_type_name_map(storage)?;
        let output = serde_json::to_string_pretty(&entries_json(entries, &name_map))?;
        println!("{}", output);
        return Ok(());
    }

    if entries.is_empty() {
        if !quiet {
            println!("No entries found.");
        }
        return Ok(());
    }

    match format.unwrap_or(OutputFormat::Table) {
        OutputFormat::Table => {
            if !quiet {
                println!("ID | CREATED_AT | SUMMARY");
            }
            for entry in entries {
                let summary = entry_table_summary(entry, TABLE_SUMMARY_MAX);
                println!("{} | {} | {}", entry.id, entry.created_at, summary);
            }
        }
        OutputFormat::Plain => {
            for entry in entries {
                let summary = entry_summary(entry);
                println!("{} {} {}", entry.id, entry.created_at, summary);
            }
        }
    }
    Ok(())
}
