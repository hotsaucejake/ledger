use uuid::Uuid;

use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::AttachArgs;

pub fn handle_attach(ctx: &AppContext, args: &AttachArgs) -> anyhow::Result<()> {
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

    storage.attach_entry_to_composition(&entry_id, &composition.id)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        println!(
            "Attached entry {} to composition '{}'",
            args.entry_id, composition.name
        );
    }
    Ok(())
}
