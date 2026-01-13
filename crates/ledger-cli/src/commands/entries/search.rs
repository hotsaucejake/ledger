use chrono::Utc;
use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::SearchArgs;
use crate::helpers::{parse_duration, require_entry_type};
use crate::output::{entries_json, entry_type_name_map};
use crate::ui::{blank_line, header, hint, print, simple_table, truncate, Column, OutputMode};

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

pub fn handle_search(ctx: &AppContext, args: &SearchArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    let mut entries = storage.search_entries(&args.query)?;
    if let Some(ref t) = args.r#type {
        let entry_type_record = require_entry_type(&storage, t)?;
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
                    print(
                        &ui_ctx,
                        &header(&ui_ctx, "search", Some(&format!("\"{}\"", args.query))),
                    );
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
            print(
                &ui_ctx,
                &header(&ui_ctx, "search", Some(&format!("\"{}\"", args.query))),
            );
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
                        "Found {} entries matching \"{}\"",
                        entries.len(),
                        args.query
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
