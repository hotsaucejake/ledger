use uuid::Uuid;

use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::CompositionRenameArgs;

pub fn handle_rename(ctx: &AppContext, args: &CompositionRenameArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(false)?;

    // Try to find by name first, then by ID
    let composition = if let Ok(uuid) = Uuid::parse_str(&args.name_or_id) {
        storage.get_composition_by_id(&uuid)?
    } else {
        storage.get_composition(&args.name_or_id)?
    };

    let composition = composition
        .ok_or_else(|| anyhow::anyhow!("Composition '{}' not found", args.name_or_id))?;

    let old_name = composition.name.clone();
    storage.rename_composition(&composition.id, &args.new_name)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        println!("Renamed composition '{}' to '{}'", old_name, args.new_name);
    }
    Ok(())
}
