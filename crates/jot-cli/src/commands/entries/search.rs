use chrono::Utc;
use jot_core::storage::StorageEngine;

use crate::app::{resolve_jot_path, AppContext};
use crate::cli::SearchArgs;
use crate::helpers::{parse_duration, require_entry_type};
use crate::output::{entries_json, entry_type_name_map};
use crate::ui::{
    blank_line, entry_summary, header_with_context, highlight_matches, hint, print, short_id,
    simple_table, truncate, Column, OutputMode,
};

const TABLE_SUMMARY_MAX: usize = 80;

pub fn handle_search(ctx: &AppContext, args: &SearchArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    // Get jot path for header
    let jot_path = resolve_jot_path(ctx.cli()).ok();

    // Build entry type name map for display
    let name_map = entry_type_name_map(&storage)?;

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

    // Build filter context for header
    let filter_context = build_filter_context(args);

    // Handle JSON output separately
    if ui_ctx.mode.is_json() {
        if args.format.is_some() {
            return Err(anyhow::anyhow!("--format cannot be used with --json"));
        }
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
                        &header_with_context(
                            &ui_ctx,
                            "search",
                            filter_context.as_deref(),
                            jot_path.as_deref(),
                        ),
                    );
                    blank_line(&ui_ctx);
                    print(
                        &ui_ctx,
                        &hint(
                            &ui_ctx,
                            "No entries found. Try a different query or broader filter.",
                        ),
                    );
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
                &header_with_context(
                    &ui_ctx,
                    "search",
                    filter_context.as_deref(),
                    jot_path.as_deref(),
                ),
            );
            blank_line(&ui_ctx);

            let columns = [
                Column::new("ID"),
                Column::new("Created"),
                Column::new("Type"),
                Column::new("Summary"),
                Column::new("Tags"),
            ];

            let rows: Vec<Vec<String>> = entries
                .iter()
                .map(|entry| {
                    let type_name = name_map
                        .get(&entry.entry_type_id)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    let tags_display = if entry.tags.is_empty() {
                        "-".to_string()
                    } else {
                        entry.tags.join(", ")
                    };
                    // Get summary and highlight matches
                    let summary = truncate(&entry_summary(entry), TABLE_SUMMARY_MAX);
                    let highlighted_summary =
                        highlight_matches(&summary, &args.query, ui_ctx.color);
                    vec![
                        short_id(&entry.id),
                        entry.created_at.format("%Y-%m-%d %H:%M").to_string(),
                        type_name,
                        highlighted_summary,
                        tags_display,
                    ]
                })
                .collect();

            print(&ui_ctx, &simple_table(&ui_ctx, &columns, &rows));
            blank_line(&ui_ctx);

            // Actionable hints with first entry ID
            let first_id = entries.first().map(|e| short_id(&e.id));
            let hint_text = if let Some(id) = first_id {
                format!(
                    "{} entries. jot show {}  \u{00B7}  jot list",
                    entries.len(),
                    id
                )
            } else {
                format!("{} entries. jot list", entries.len())
            };
            print(&ui_ctx, &hint(&ui_ctx, &hint_text));
        }
        OutputMode::Plain | OutputMode::Json => {
            // Plain mode: space-separated values with type
            for entry in &entries {
                let type_name = name_map
                    .get(&entry.entry_type_id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                let summary = entry_summary(entry);
                let tags = if entry.tags.is_empty() {
                    "-".to_string()
                } else {
                    entry.tags.join(",")
                };
                println!(
                    "{} {} {} {} {}",
                    entry.id, entry.created_at, type_name, tags, summary
                );
            }
        }
    }

    Ok(())
}

/// Build a filter context string for the header.
fn build_filter_context(args: &SearchArgs) -> Option<String> {
    let mut parts = vec![format!("\"{}\"", args.query)];

    if let Some(ref l) = args.last {
        parts.push(format!("last {}", l));
    }
    if let Some(ref t) = args.r#type {
        parts.push(format!("type: {}", t));
    }

    Some(parts.join(", "))
}
