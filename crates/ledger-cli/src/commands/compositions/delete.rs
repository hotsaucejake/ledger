use dialoguer::Confirm;
use uuid::Uuid;

use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::CompositionDeleteArgs;
use crate::ui::{badge, print, Badge, OutputMode};

pub fn handle_delete(ctx: &AppContext, args: &CompositionDeleteArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(false)?;

    // Try to find by name first, then by ID
    let composition = if let Ok(uuid) = Uuid::parse_str(&args.name_or_id) {
        storage.get_composition_by_id(&uuid)?
    } else {
        storage.get_composition(&args.name_or_id)?
    };

    let composition = composition
        .ok_or_else(|| anyhow::anyhow!("Composition '{}' not found", args.name_or_id))?;

    // Get entry count for confirmation message
    let entries = storage.get_composition_entries(&composition.id)?;
    let entry_count = entries.len();

    if !args.force {
        let confirm_msg = if entry_count > 0 {
            format!(
                "Delete composition '{}' with {} attached entries? (entries will NOT be deleted)",
                composition.name, entry_count
            )
        } else {
            format!("Delete composition '{}'?", composition.name)
        };

        let confirmed = Confirm::new()
            .with_prompt(confirm_msg)
            .default(false)
            .interact()?;

        if !confirmed {
            if !ctx.quiet() {
                let ui_ctx = ctx.ui_context(false, None);
                match ui_ctx.mode {
                    OutputMode::Pretty => {
                        print(&ui_ctx, &badge(&ui_ctx, Badge::Info, "Cancelled"));
                    }
                    OutputMode::Plain | OutputMode::Json => {
                        println!("status=cancelled");
                    }
                }
            }
            return Ok(());
        }
    }

    let name = composition.name.clone();
    storage.delete_composition(&composition.id)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        let ui_ctx = ctx.ui_context(false, None);
        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(
                    &ui_ctx,
                    &badge(
                        &ui_ctx,
                        Badge::Ok,
                        &format!("Deleted composition '{}'", name),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("deleted={}", name);
            }
        }
    }
    Ok(())
}
