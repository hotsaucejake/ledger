use chrono::Utc;
use ledger_core::StorageEngine;

use crate::app::AppContext;
use crate::cli::SearchArgs;
use crate::helpers::{parse_duration, parse_output_format, require_entry_type};
use crate::output::print_entry_list;

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

    let format = parse_output_format(args.format.as_deref())?;
    print_entry_list(&storage, &entries, args.json, format, ctx.quiet())
}
