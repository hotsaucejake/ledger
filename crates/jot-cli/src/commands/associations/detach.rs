use uuid::Uuid;

use jot_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::DetachArgs;
use crate::ui::theme::{styled, styles};
use crate::ui::{badge, blank_line, hint, print, short_id, Badge, OutputMode};

pub fn handle_detach(ctx: &AppContext, args: &DetachArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(false)?;

    // Parse entry ID
    let entry_id = Uuid::parse_str(&args.entry_id)
        .map_err(|_| anyhow::anyhow!("Invalid entry ID: {}", args.entry_id))?;

    // Verify entry exists
    let entry = storage.get_entry(&entry_id)?;
    if entry.is_none() {
        return Err(anyhow::anyhow!("Entry '{}' not found", args.entry_id));
    }

    // Find composition by name or ID
    let composition = if let Ok(uuid) = Uuid::parse_str(&args.composition) {
        storage.get_composition_by_id(&uuid)?
    } else {
        storage.get_composition(&args.composition)?
    };

    let composition = composition
        .ok_or_else(|| anyhow::anyhow!("Composition '{}' not found", args.composition))?;

    storage.detach_entry_from_composition(&entry_id, &composition.id)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        let ui_ctx = ctx.ui_context(false, None);
        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(
                    &ui_ctx,
                    &badge(&ui_ctx, Badge::Ok, "Detached entry from composition"),
                );
                // Context line with entry ID and composition name
                let context = format!(
                    "Entry: {}  \u{00B7}  Composition: {}",
                    short_id(&entry_id),
                    composition.name
                );
                let context_styled = styled(&context, styles::dim(), ui_ctx.color);
                println!("{}", context_styled);
                // Next step hints
                blank_line(&ui_ctx);
                print(
                    &ui_ctx,
                    &hint(
                        &ui_ctx,
                        &format!("jot show {}  \u{00B7}  jot list", short_id(&entry_id)),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("entry_id={}", entry_id);
                println!("composition={}", composition.name);
                println!("composition_id={}", composition.id);
            }
        }
    }
    Ok(())
}
