use ledger_core::storage::{EntryFilter, StorageEngine};

use crate::app::{exit_not_found_with_hint, open_storage_with_retry};
use crate::cli::Cli;
use crate::helpers::{ensure_journal_type_name, parse_datetime};
use crate::output::{entries_json, entry_type_name_map};

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
