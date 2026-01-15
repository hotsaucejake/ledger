use jot_core::StorageEngine;
use uuid::Uuid;

use crate::app::{exit_not_found_with_hint, AppContext};
use crate::cli::ShowArgs;
use crate::output::{entry_json, entry_type_name_map};
use crate::ui::{blank_line, divider, header, kv, print, OutputMode};

pub fn handle_show(ctx: &AppContext, args: &ShowArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    let parsed =
        Uuid::parse_str(&args.id).map_err(|e| anyhow::anyhow!("Invalid entry ID: {}", e))?;
    let entry = storage.get_entry(&parsed)?.unwrap_or_else(|| {
        exit_not_found_with_hint(
            "Entry not found",
            "Hint: Run `jot list --last 7d` to find entry IDs.",
        )
    });

    // Create UI context
    let ui_ctx = ctx.ui_context(args.json, None);

    // Handle JSON output
    if ui_ctx.mode.is_json() {
        let name_map = entry_type_name_map(&storage)?;
        let output = serde_json::to_string_pretty(&entry_json(&entry, &name_map))?;
        println!("{}", output);
        return Ok(());
    }

    // Get entry type name for display
    let name_map = entry_type_name_map(&storage)?;
    let entry_type_name = name_map
        .get(&entry.entry_type_id)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    // Extract body from entry data
    let body = entry
        .data
        .get("body")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| entry.data.to_string());

    match ui_ctx.mode {
        OutputMode::Pretty => {
            if !ctx.quiet() {
                print(&ui_ctx, &header(&ui_ctx, "show", None));
                blank_line(&ui_ctx);
                print(&ui_ctx, &kv(&ui_ctx, "ID", &entry.id.to_string()));
                print(
                    &ui_ctx,
                    &kv(
                        &ui_ctx,
                        "Type",
                        &format!("{} (v{})", entry_type_name, entry.schema_version),
                    ),
                );
                print(
                    &ui_ctx,
                    &kv(
                        &ui_ctx,
                        "Created",
                        &entry.created_at.format("%Y-%m-%d %H:%M UTC").to_string(),
                    ),
                );
                print(
                    &ui_ctx,
                    &kv(&ui_ctx, "Device", &entry.device_id.to_string()),
                );
                if !entry.tags.is_empty() {
                    print(&ui_ctx, &kv(&ui_ctx, "Tags", &entry.tags.join(", ")));
                }
                if let Some(supersedes) = entry.supersedes {
                    print(&ui_ctx, &kv(&ui_ctx, "Supersedes", &supersedes.to_string()));
                }
                blank_line(&ui_ctx);
                print(&ui_ctx, &divider(&ui_ctx));
                blank_line(&ui_ctx);
            }
            println!("{}", body);
        }
        OutputMode::Plain | OutputMode::Json => {
            if !ctx.quiet() {
                println!("id={}", entry.id);
                println!("type={}", entry_type_name);
                println!("schema_version={}", entry.schema_version);
                println!("created_at={}", entry.created_at.to_rfc3339());
                println!("device_id={}", entry.device_id);
                if !entry.tags.is_empty() {
                    println!("tags={}", entry.tags.join(","));
                }
                if let Some(supersedes) = entry.supersedes {
                    println!("supersedes={}", supersedes);
                }
            }
            println!("{}", body);
        }
    }

    Ok(())
}
