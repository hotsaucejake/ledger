use chrono::Utc;
use uuid::Uuid;

use ledger_core::storage::{EntryFilter, NewEntry, StorageEngine};

use crate::app::{exit_not_found_with_hint, open_storage_with_retry};
use crate::cli::Cli;
use crate::helpers::{
    ensure_journal_type_name, parse_datetime, parse_duration, parse_output_format, read_entry_body,
    OutputFormat,
};
use crate::output::{
    entries_json, entry_json, entry_summary, entry_table_summary, entry_type_name_map, print_entry,
};

const DEFAULT_LIST_LIMIT: usize = 20;
const TABLE_SUMMARY_MAX: usize = 80;

pub fn handle_add(
    cli: &Cli,
    entry_type: &str,
    tag: &[String],
    date: &Option<String>,
    no_input: bool,
    body: &Option<String>,
    editor_override: Option<&str>,
) -> anyhow::Result<()> {
    ensure_journal_type_name(entry_type)?;

    let (mut storage, passphrase) = open_storage_with_retry(cli, no_input)?;
    let entry_type_record = storage.get_entry_type(entry_type)?.unwrap_or_else(|| {
        exit_not_found_with_hint(
            &format!("Entry type \"{}\" not found", entry_type),
            "Hint: Only \"journal\" is available in Phase 0.1.",
        )
    });

    let body = read_entry_body(no_input, body.clone(), editor_override, None)?;
    let data = serde_json::json!({ "body": body });
    let metadata = storage.metadata()?;
    let mut new_entry = NewEntry::new(
        entry_type_record.id,
        entry_type_record.version,
        data,
        metadata.device_id,
    )
    .with_tags(tag.to_vec());
    if let Some(value) = date {
        let parsed = parse_datetime(value)?;
        new_entry = new_entry.with_created_at(parsed);
    }

    let entry_id = storage.insert_entry(&new_entry)?;
    storage.close(&passphrase)?;

    if !cli.quiet {
        println!("Added entry {}", entry_id);
    }
    Ok(())
}

pub fn handle_edit(
    cli: &Cli,
    id: &str,
    body: &Option<String>,
    no_input: bool,
    editor_override: Option<&str>,
) -> anyhow::Result<()> {
    let (mut storage, passphrase) = open_storage_with_retry(cli, no_input)?;
    let parsed = Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid entry ID: {}", e))?;
    let entry = storage.get_entry(&parsed)?.unwrap_or_else(|| {
        exit_not_found_with_hint(
            "Entry not found",
            "Hint: Run `ledger list --last 7d` to find entry IDs.",
        )
    });

    let entry_type_name = entry_type_name_map(&storage)?
        .get(&entry.entry_type_id)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    ensure_journal_type_name(&entry_type_name)?;

    let existing_body = entry
        .data
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let body = read_entry_body(no_input, body.clone(), editor_override, Some(existing_body))?;
    if body.trim().is_empty() {
        return Err(anyhow::anyhow!("Entry body is empty"));
    }

    let data = serde_json::json!({ "body": body });
    let metadata = storage.metadata()?;
    let new_entry = NewEntry::new(
        entry.entry_type_id,
        entry.schema_version,
        data,
        metadata.device_id,
    )
    .with_tags(entry.tags.clone())
    .with_supersedes(entry.id);

    let entry_id = storage.insert_entry(&new_entry)?;
    storage.close(&passphrase)?;

    if !cli.quiet {
        println!("Edited entry {}", entry_id);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_list(
    cli: &Cli,
    entry_type: &Option<String>,
    tag: &Option<String>,
    last: &Option<String>,
    since: &Option<String>,
    until: &Option<String>,
    limit: &Option<usize>,
    json: bool,
    format: &Option<String>,
) -> anyhow::Result<()> {
    let (storage, _passphrase) = open_storage_with_retry(cli, false)?;

    let mut filter = EntryFilter::new();
    if let Some(t) = entry_type {
        ensure_journal_type_name(t)?;
        let entry_type_record = storage.get_entry_type(t)?.unwrap_or_else(|| {
            exit_not_found_with_hint(
                &format!("Entry type \"{}\" not found", t),
                "Hint: Only \"journal\" is available in Phase 0.1.",
            )
        });
        filter = filter.entry_type(entry_type_record.id);
    }
    if let Some(t) = tag {
        filter = filter.tag(t.clone());
    }
    if let Some(l) = last {
        let window = parse_duration(l)?;
        let since_time = Utc::now() - window;
        filter = filter.since(since_time);
    }
    if let Some(s) = since {
        let parsed = chrono::DateTime::parse_from_rfc3339(s)
            .map_err(|e| anyhow::anyhow!("Invalid since timestamp: {}", e))?;
        filter = filter.since(parsed.with_timezone(&chrono::Utc));
    }
    if let Some(u) = until {
        let parsed = chrono::DateTime::parse_from_rfc3339(u)
            .map_err(|e| anyhow::anyhow!("Invalid until timestamp: {}", e))?;
        filter = filter.until(parsed.with_timezone(&chrono::Utc));
    }
    if let Some(lim) = limit {
        filter = filter.limit(*lim);
    } else if last.is_none() && since.is_none() && until.is_none() {
        filter = filter.limit(DEFAULT_LIST_LIMIT);
    }

    let entries = storage.list_entries(&filter)?;
    let format = parse_output_format(format.as_deref())?;
    if json {
        if format.is_some() {
            return Err(anyhow::anyhow!("--format cannot be used with --json"));
        }
        let name_map = entry_type_name_map(&storage)?;
        let output = serde_json::to_string_pretty(&entries_json(&entries, &name_map))?;
        println!("{}", output);
    } else {
        if entries.is_empty() {
            if !cli.quiet {
                println!("No entries found.");
            }
            return Ok(());
        }
        match format.unwrap_or(OutputFormat::Table) {
            OutputFormat::Table => {
                if !cli.quiet {
                    println!("ID | CREATED_AT | SUMMARY");
                }
                for entry in entries {
                    let summary = entry_table_summary(&entry, TABLE_SUMMARY_MAX);
                    println!("{} | {} | {}", entry.id, entry.created_at, summary);
                }
            }
            OutputFormat::Plain => {
                for entry in entries {
                    let summary = entry_summary(&entry);
                    println!("{} {} {}", entry.id, entry.created_at, summary);
                }
            }
        }
    }
    Ok(())
}

pub fn handle_search(
    cli: &Cli,
    query: &str,
    entry_type: &Option<String>,
    last: &Option<String>,
    json: bool,
    limit: &Option<usize>,
    format: &Option<String>,
) -> anyhow::Result<()> {
    let (storage, _passphrase) = open_storage_with_retry(cli, false)?;

    let mut entries = storage.search_entries(query)?;
    if let Some(t) = entry_type {
        ensure_journal_type_name(t)?;
        let entry_type_record = storage.get_entry_type(t)?.unwrap_or_else(|| {
            exit_not_found_with_hint(
                &format!("Entry type \"{}\" not found", t),
                "Hint: Only \"journal\" is available in Phase 0.1.",
            )
        });
        entries.retain(|entry| entry.entry_type_id == entry_type_record.id);
    }
    if let Some(l) = last {
        let window = parse_duration(l)?;
        let since = Utc::now() - window;
        entries.retain(|entry| entry.created_at >= since);
    }
    if let Some(lim) = limit {
        entries.truncate(*lim);
    }

    let format = parse_output_format(format.as_deref())?;
    if json {
        if format.is_some() {
            return Err(anyhow::anyhow!("--format cannot be used with --json"));
        }
        let name_map = entry_type_name_map(&storage)?;
        let output = serde_json::to_string_pretty(&entries_json(&entries, &name_map))?;
        println!("{}", output);
    } else {
        if entries.is_empty() {
            if !cli.quiet {
                println!("No entries found.");
            }
            return Ok(());
        }
        match format.unwrap_or(OutputFormat::Table) {
            OutputFormat::Table => {
                if !cli.quiet {
                    println!("ID | CREATED_AT | SUMMARY");
                }
                for entry in entries {
                    let summary = entry_table_summary(&entry, TABLE_SUMMARY_MAX);
                    println!("{} | {} | {}", entry.id, entry.created_at, summary);
                }
            }
            OutputFormat::Plain => {
                for entry in entries {
                    let summary = entry_summary(&entry);
                    println!("{} {} {}", entry.id, entry.created_at, summary);
                }
            }
        }
    }
    Ok(())
}

pub fn handle_show(cli: &Cli, id: &str, json: bool) -> anyhow::Result<()> {
    let (storage, _passphrase) = open_storage_with_retry(cli, false)?;

    let parsed = Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid entry ID: {}", e))?;
    let entry = storage.get_entry(&parsed)?.unwrap_or_else(|| {
        exit_not_found_with_hint(
            "Entry not found",
            "Hint: Run `ledger list --last 7d` to find entry IDs.",
        )
    });
    if json {
        let name_map = entry_type_name_map(&storage)?;
        let output = serde_json::to_string_pretty(&entry_json(&entry, &name_map))?;
        println!("{}", output);
    } else {
        print_entry(&storage, &entry, cli.quiet)?;
    }
    Ok(())
}

pub fn handle_export(
    cli: &Cli,
    entry_type: &Option<String>,
    format: &str,
    since: &Option<String>,
) -> anyhow::Result<()> {
    let (storage, _passphrase) = open_storage_with_retry(cli, false)?;

    let mut filter = EntryFilter::new();
    if let Some(t) = entry_type {
        ensure_journal_type_name(t)?;
        let entry_type_record = storage.get_entry_type(t)?.unwrap_or_else(|| {
            exit_not_found_with_hint(
                &format!("Entry type \"{}\" not found", t),
                "Hint: Only \"journal\" is available in Phase 0.1.",
            )
        });
        filter = filter.entry_type(entry_type_record.id);
    }
    if let Some(s) = since {
        let parsed = parse_datetime(s)?;
        filter = filter.since(parsed);
    }

    let entries = storage.list_entries(&filter)?;
    let name_map = entry_type_name_map(&storage)?;
    match format {
        "json" => {
            let output = serde_json::to_string_pretty(&entries_json(&entries, &name_map))?;
            println!("{}", output);
        }
        "jsonl" => {
            for value in entries_json(&entries, &name_map) {
                println!("{}", serde_json::to_string(&value)?);
            }
        }
        other => {
            return Err(anyhow::anyhow!(
                "Unsupported export format: {} (use json or jsonl for portable exports)",
                other
            ));
        }
    }
    Ok(())
}
