use uuid::Uuid;

use jot_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::CompositionShowArgs;
use crate::ui::{blank_line, header, kv, print, OutputMode};

pub fn handle_show(ctx: &AppContext, args: &CompositionShowArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    // Try to find by name first, then by ID
    let composition = if let Ok(uuid) = Uuid::parse_str(&args.name_or_id) {
        storage.get_composition_by_id(&uuid)?
    } else {
        storage.get_composition(&args.name_or_id)?
    };

    let composition = composition
        .ok_or_else(|| anyhow::anyhow!("Composition '{}' not found", args.name_or_id))?;

    // Create UI context
    let ui_ctx = ctx.ui_context(args.json, None);

    // Handle JSON output
    if ui_ctx.mode.is_json() {
        let entries = storage.get_composition_entries(&composition.id)?;
        let json_output = serde_json::json!({
            "id": composition.id.to_string(),
            "name": composition.name,
            "description": composition.description,
            "created_at": composition.created_at.to_rfc3339(),
            "device_id": composition.device_id.to_string(),
            "metadata": composition.metadata,
            "entry_count": entries.len(),
        });
        println!("{}", serde_json::to_string_pretty(&json_output)?);
        return Ok(());
    }

    let entries = storage.get_composition_entries(&composition.id)?;

    match ui_ctx.mode {
        OutputMode::Pretty => {
            print(&ui_ctx, &header(&ui_ctx, "composition", None));
            blank_line(&ui_ctx);
            print(&ui_ctx, &kv(&ui_ctx, "Name", &composition.name));
            print(&ui_ctx, &kv(&ui_ctx, "ID", &composition.id.to_string()));
            if let Some(ref desc) = composition.description {
                print(&ui_ctx, &kv(&ui_ctx, "Description", desc));
            }
            print(
                &ui_ctx,
                &kv(
                    &ui_ctx,
                    "Created",
                    &composition
                        .created_at
                        .format("%Y-%m-%d %H:%M UTC")
                        .to_string(),
                ),
            );
            print(&ui_ctx, &kv(&ui_ctx, "Entries", &entries.len().to_string()));
        }
        OutputMode::Plain | OutputMode::Json => {
            println!("name={}", composition.name);
            println!("id={}", composition.id);
            if let Some(ref desc) = composition.description {
                println!("description={}", desc);
            }
            println!("created_at={}", composition.created_at.to_rfc3339());
            println!("device_id={}", composition.device_id);
            println!("entry_count={}", entries.len());
        }
    }

    Ok(())
}
