use chrono::Utc;
use uuid::Uuid;

use ledger_core::storage::{NewEntry, StorageEngine};

use crate::app::{exit_not_found_with_hint, AppContext};
use crate::cli::EditArgs;
use crate::helpers::{ensure_journal_type_name, read_entry_body};
use crate::output::entry_type_name_map;
use crate::ui::theme::{styled, styles};
use crate::ui::{badge, blank_line, hint, print, short_id, Badge, OutputMode};

pub fn handle_edit(ctx: &AppContext, args: &EditArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(args.no_input)?;
    let parsed =
        Uuid::parse_str(&args.id).map_err(|e| anyhow::anyhow!("Invalid entry ID: {}", e))?;
    let entry = storage.get_entry(&parsed)?.unwrap_or_else(|| {
        exit_not_found_with_hint(
            "Entry not found",
            "Hint: Run `ledger list --last 7d` to find entry IDs.",
        )
    });

    let entry_type_name = entry_type_name_map(&storage)?
        .get(&entry.entry_type_id)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    ensure_journal_type_name(&entry_type_name)?;

    let existing_body = entry
        .data
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let editor_override = ctx.editor()?;
    let body = read_entry_body(
        args.no_input,
        args.body.clone(),
        editor_override,
        Some(existing_body),
    )?;
    if body.trim().is_empty() {
        return Err(anyhow::anyhow!("Entry body is empty"));
    }

    let data = serde_json::json!({ "body": body });
    let metadata = storage.metadata()?;
    let new_entry = NewEntry::new(
        entry.entry_type_id,
        entry.schema_version,
        data,
        metadata.device_id,
    )
    .with_tags(entry.tags.clone())
    .with_supersedes(entry.id);

    let entry_id = storage.insert_entry(&new_entry)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        let ui_ctx = ctx.ui_context(false, None);
        let edited_at = Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
        let tag_count = entry.tags.len();

        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(&ui_ctx, &badge(&ui_ctx, Badge::Ok, "Edited entry"));
                // Context line with ID, timestamp, and supersedes
                let context = format!(
                    "ID: {}  \u{00B7}  {}  \u{00B7}  supersedes: {}",
                    short_id(&entry_id),
                    edited_at,
                    short_id(&entry.id)
                );
                let context_styled = styled(&context, styles::dim(), ui_ctx.color);
                println!("{}", context_styled);
                // Next step hints
                blank_line(&ui_ctx);
                print(
                    &ui_ctx,
                    &hint(
                        &ui_ctx,
                        &format!("ledger show {}  \u{00B7}  ledger list", short_id(&entry_id)),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("entry_id={}", entry_id);
                println!("supersedes={}", entry.id);
                println!("edited_at={}", edited_at);
                println!("tag_count={}", tag_count);
            }
        }
    }
    Ok(())
}
