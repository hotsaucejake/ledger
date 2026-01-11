use ledger_core::storage::{EntryFilter, StorageEngine};

use crate::app::AppContext;
use crate::cli::ExportArgs;
use crate::helpers::{parse_datetime, require_entry_type};
use crate::output::{entries_json, entry_type_name_map};

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
    match args.format.as_str() {
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
