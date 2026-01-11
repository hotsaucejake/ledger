use ledger_core::storage::{NewEntry, StorageEngine};

use crate::app::AppContext;
use crate::cli::AddArgs;
use crate::helpers::{parse_datetime, read_entry_body, require_entry_type};

pub fn handle_add(ctx: &AppContext, args: &AddArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(args.no_input)?;
    let entry_type_record = require_entry_type(&storage, &args.entry_type)?;

    let editor_override = ctx.editor()?;
    let body = read_entry_body(args.no_input, args.body.clone(), editor_override, None)?;
    let data = serde_json::json!({ "body": body });
    let metadata = storage.metadata()?;
    let mut new_entry = NewEntry::new(
        entry_type_record.id,
        entry_type_record.version,
        data,
        metadata.device_id,
    )
    .with_tags(args.tag.clone());
    if let Some(ref value) = args.date {
        let parsed = parse_datetime(value)?;
        new_entry = new_entry.with_created_at(parsed);
    }

    let entry_id = storage.insert_entry(&new_entry)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        println!("Added entry {}", entry_id);
    }
    Ok(())
}
