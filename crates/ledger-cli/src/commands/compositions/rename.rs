use uuid::Uuid;

use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::CompositionRenameArgs;
use crate::ui::{badge, print, Badge, OutputMode};

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
        let ui_ctx = ctx.ui_context(false, None);
        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(
                    &ui_ctx,
                    &badge(
                        &ui_ctx,
                        Badge::Ok,
                        &format!("Renamed composition '{}' to '{}'", old_name, args.new_name),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("old_name={}", old_name);
                println!("new_name={}", args.new_name);
            }
        }
    }
    Ok(())
}
