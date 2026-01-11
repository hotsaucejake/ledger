use uuid::Uuid;

use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::TemplateUpdateArgs;

pub fn handle_update(ctx: &AppContext, args: &TemplateUpdateArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(false)?;

    // Try to find by name first, then by ID
    let template = if let Ok(uuid) = Uuid::parse_str(&args.name_or_id) {
        storage.get_template_by_id(&uuid)?
    } else {
        storage.get_template(&args.name_or_id)?
    };

    let template =
        template.ok_or_else(|| anyhow::anyhow!("Template '{}' not found", args.name_or_id))?;

    let new_defaults: serde_json::Value = serde_json::from_str(&args.defaults)
        .map_err(|e| anyhow::anyhow!("Invalid JSON for defaults: {}", e))?;

    let new_version = storage.update_template(&template.id, new_defaults)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        println!(
            "Updated template '{}' to version {}",
            template.name, new_version
        );
    }
    Ok(())
}
