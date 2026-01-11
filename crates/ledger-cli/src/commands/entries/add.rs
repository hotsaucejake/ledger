use ledger_core::storage::{NewEntry, StorageEngine};

use crate::app::{exit_not_found_with_hint, open_storage_with_retry};
use crate::cli::{AddArgs, Cli};
use crate::helpers::{ensure_journal_type_name, parse_datetime, read_entry_body};

pub fn handle_add(cli: &Cli, args: &AddArgs, editor_override: Option<&str>) -> anyhow::Result<()> {
    ensure_journal_type_name(&args.entry_type)?;

    let (mut storage, passphrase) = open_storage_with_retry(cli, args.no_input)?;
    let entry_type_record = storage
        .get_entry_type(&args.entry_type)?
        .unwrap_or_else(|| {
            exit_not_found_with_hint(
                &format!("Entry type \"{}\" not found", args.entry_type),
                "Hint: Only \"journal\" is available in Phase 0.1.",
            )
        });

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

    if !cli.quiet {
        println!("Added entry {}", entry_id);
    }
    Ok(())
}
