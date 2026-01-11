use chrono::Utc;
use ledger_core::StorageEngine;

use crate::app::{exit_not_found_with_hint, open_storage_with_retry};
use crate::cli::{Cli, SearchArgs};
use crate::helpers::{ensure_journal_type_name, parse_duration, parse_output_format, OutputFormat};
use crate::output::{entries_json, entry_summary, entry_table_summary, entry_type_name_map};

const TABLE_SUMMARY_MAX: usize = 80;

pub fn handle_search(cli: &Cli, args: &SearchArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = open_storage_with_retry(cli, false)?;

    let mut entries = storage.search_entries(&args.query)?;
    if let Some(ref t) = args.r#type {
        ensure_journal_type_name(t)?;
        let entry_type_record = storage.get_entry_type(t)?.unwrap_or_else(|| {
            exit_not_found_with_hint(
                &format!("Entry type \"{}\" not found", t),
                "Hint: Only \"journal\" is available in Phase 0.1.",
            )
        });
        entries.retain(|entry| entry.entry_type_id == entry_type_record.id);
    }
    if let Some(ref l) = args.last {
        let window = parse_duration(l)?;
        let since = Utc::now() - window;
        entries.retain(|entry| entry.created_at >= since);
    }
    if !args.history {
        let superseded = storage.superseded_entry_ids()?;
        entries.retain(|entry| !superseded.contains(&entry.id));
    }
    if let Some(lim) = args.limit {
        entries.truncate(lim);
    }

    let format = parse_output_format(args.format.as_deref())?;
    if args.json {
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
