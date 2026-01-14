use std::time::Instant;

use ledger_core::storage::{EntryFilter, StorageEngine};

use crate::app::AppContext;
use crate::cli::ExportArgs;
use crate::helpers::{parse_datetime, require_entry_type};
use crate::output::{entries_json, entry_json, entry_type_name_map};
use crate::ui::format::format_duration_secs;
use crate::ui::progress::ProgressBar;
use crate::ui::theme::{styled, styles};
use crate::ui::{badge, Badge, OutputMode};

pub fn handle_export(ctx: &AppContext, args: &ExportArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    let mut filter = EntryFilter::new();
    if let Some(ref t) = args.entry_type {
        let entry_type_record = require_entry_type(&storage, t)?;
        filter = filter.entry_type(entry_type_record.id);
    }
    if let Some(ref s) = args.since {
        let parsed = parse_datetime(s)?;
        filter = filter.since(parsed);
    }

    let entries = storage.list_entries(&filter)?;
    let name_map = entry_type_name_map(&storage)?;
    let entry_count = entries.len();
    let start_time = Instant::now();

    // Get UI context for progress display
    let ui_ctx = ctx.ui_context(false, None);
    let show_progress = ui_ctx.mode.is_pretty() && !ctx.quiet() && entry_count > 10;

    match args.format.as_str() {
        "json" => {
            let output = serde_json::to_string_pretty(&entries_json(&entries, &name_map))?;
            println!("{}", output);
        }
        "jsonl" => {
            if show_progress {
                let mut progress = ProgressBar::new(&ui_ctx, entry_count as u64, "Exporting");
                for entry in &entries {
                    let value = entry_json(entry, &name_map);
                    println!("{}", serde_json::to_string(&value)?);
                    progress.inc(1);
                }
                progress.finish();
            } else {
                for value in entries_json(&entries, &name_map) {
                    println!("{}", serde_json::to_string(&value)?);
                }
            }
        }
        other => {
            return Err(anyhow::anyhow!(
                "Unsupported export format: {} (use json or jsonl for portable exports)",
                other
            ));
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();

    // Show summary to stderr (so it doesn't interfere with piped output)
    if !ctx.quiet() {
        match ui_ctx.mode {
            OutputMode::Pretty => {
                eprintln!(
                    "{}",
                    badge(
                        &ui_ctx,
                        Badge::Ok,
                        &format!("Exported {} entries", entry_count)
                    )
                );
                let context = format!(
                    "Format: {}  \u{00B7}  Time: {}",
                    args.format,
                    format_duration_secs(elapsed)
                );
                let context_styled = styled(&context, styles::dim(), ui_ctx.color);
                eprintln!("{}", context_styled);
            }
            OutputMode::Plain | OutputMode::Json => {
                // Plain mode: output stats to stderr so they don't mix with data
                eprintln!("export_count={}", entry_count);
                eprintln!("format={}", args.format);
                eprintln!("elapsed_ms={:.0}", elapsed * 1000.0);
            }
        }
    }

    Ok(())
}
