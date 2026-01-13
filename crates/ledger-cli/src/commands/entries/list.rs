use chrono::Utc;

use ledger_core::storage::{EntryFilter, StorageEngine};

use crate::app::AppContext;
use crate::cli::ListArgs;
use crate::helpers::{parse_duration, require_entry_type};
use crate::output::{entries_json, entry_type_name_map};
use crate::ui::{blank_line, header, hint, print, simple_table, truncate, Column, OutputMode};

const DEFAULT_LIST_LIMIT: usize = 20;
const TABLE_SUMMARY_MAX: usize = 80;

/// Extract a summary from an entry's data, preferring the "body" field.
fn entry_summary(entry: &ledger_core::storage::Entry) -> String {
    entry
        .data
        .get("body")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| entry.data.to_string())
}

pub fn handle_list(ctx: &AppContext, args: &ListArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    let mut filter = EntryFilter::new();
    if let Some(ref t) = args.entry_type {
        let entry_type_record = require_entry_type(&storage, t)?;
        filter = filter.entry_type(entry_type_record.id);
    }
    if let Some(ref t) = args.tag {
        filter = filter.tag(t.clone());
    }
    if let Some(ref l) = args.last {
        let window = parse_duration(l)?;
        let since_time = Utc::now() - window;
        filter = filter.since(since_time);
    }
    if let Some(ref s) = args.since {
        let parsed = chrono::DateTime::parse_from_rfc3339(s)
            .map_err(|e| anyhow::anyhow!("Invalid since timestamp: {}", e))?;
        filter = filter.since(parsed.with_timezone(&chrono::Utc));
    }
    if let Some(ref u) = args.until {
        let parsed = chrono::DateTime::parse_from_rfc3339(u)
            .map_err(|e| anyhow::anyhow!("Invalid until timestamp: {}", e))?;
        filter = filter.until(parsed.with_timezone(&chrono::Utc));
    }
    if let Some(lim) = args.limit {
        filter = filter.limit(lim);
    } else if args.last.is_none() && args.since.is_none() && args.until.is_none() {
        filter = filter.limit(DEFAULT_LIST_LIMIT);
    }

    let mut entries = storage.list_entries(&filter)?;
    if !args.history {
        let superseded = storage.superseded_entry_ids()?;
        entries.retain(|entry| !superseded.contains(&entry.id));
    }

    // Create UI context from flags
    let ui_ctx = ctx.ui_context(args.json, args.format.as_deref());

    // Handle JSON output separately
    if ui_ctx.mode.is_json() {
        if args.format.is_some() {
            return Err(anyhow::anyhow!("--format cannot be used with --json"));
        }
        let name_map = entry_type_name_map(&storage)?;
        let output = serde_json::to_string_pretty(&entries_json(&entries, &name_map))?;
        println!("{}", output);
        return Ok(());
    }

    // Empty result handling
    if entries.is_empty() {
        if !ctx.quiet() {
            match ui_ctx.mode {
                OutputMode::Pretty => {
                    print(&ui_ctx, &header(&ui_ctx, "list", None));
                    blank_line(&ui_ctx);
                    print(&ui_ctx, &hint(&ui_ctx, "No entries found."));
                }
                OutputMode::Plain | OutputMode::Json => {
                    println!("count=0");
                }
            }
        }
        return Ok(());
    }

    // Render entries
    match ui_ctx.mode {
        OutputMode::Pretty => {
            print(&ui_ctx, &header(&ui_ctx, "list", None));
            blank_line(&ui_ctx);

            let columns = [
                Column::new("ID"),
                Column::new("Created"),
                Column::new("Summary"),
            ];

            let rows: Vec<Vec<String>> = entries
                .iter()
                .map(|entry| {
                    vec![
                        entry.id.to_string()[..8].to_string(), // short ID
                        entry.created_at.format("%Y-%m-%d %H:%M").to_string(),
                        truncate(&entry_summary(entry), TABLE_SUMMARY_MAX),
                    ]
                })
                .collect();

            print(&ui_ctx, &simple_table(&ui_ctx, &columns, &rows));
            blank_line(&ui_ctx);
            print(
                &ui_ctx,
                &hint(
                    &ui_ctx,
                    &format!(
                        "Showing {} entries. Use --limit to see more.",
                        entries.len()
                    ),
                ),
            );
        }
        OutputMode::Plain | OutputMode::Json => {
            // Plain mode: space-separated values
            for entry in &entries {
                let summary = entry_summary(entry);
                println!("{} {} {}", entry.id, entry.created_at, summary);
            }
        }
    }

    Ok(())
}
