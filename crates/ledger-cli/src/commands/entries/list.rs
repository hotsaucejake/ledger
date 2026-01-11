use chrono::Utc;

use ledger_core::storage::{EntryFilter, StorageEngine};

use crate::app::AppContext;
use crate::cli::ListArgs;
use crate::helpers::{parse_duration, parse_output_format, require_entry_type};
use crate::output::print_entry_list;

const DEFAULT_LIST_LIMIT: usize = 20;

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

    let format = parse_output_format(args.format.as_deref())?;
    print_entry_list(&storage, &entries, args.json, format, ctx.quiet())
}
