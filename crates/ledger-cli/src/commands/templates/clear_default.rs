use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::TemplateClearDefaultArgs;
use crate::helpers::require_entry_type;

pub fn handle_clear_default(
    ctx: &AppContext,
    args: &TemplateClearDefaultArgs,
) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(false)?;
    let entry_type = require_entry_type(&storage, &args.entry_type)?;

    storage.clear_default_template(&entry_type.id)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        println!("Cleared default template for '{}'", args.entry_type);
    }
    Ok(())
}
