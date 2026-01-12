use dialoguer::Confirm;
use uuid::Uuid;

use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::TemplateDeleteArgs;

pub fn handle_delete(ctx: &AppContext, args: &TemplateDeleteArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(false)?;

    // Try to find by name first, then by ID
    let template = if let Ok(uuid) = Uuid::parse_str(&args.name_or_id) {
        storage.get_template_by_id(&uuid)?
    } else {
        storage.get_template(&args.name_or_id)?
    };

    let template =
        template.ok_or_else(|| anyhow::anyhow!("Template '{}' not found", args.name_or_id))?;

    if !args.force {
        let confirmed = Confirm::new()
            .with_prompt(format!("Delete template '{}'?", template.name))
            .default(false)
            .interact()?;

        if !confirmed {
            if !ctx.quiet() {
                println!("Cancelled.");
            }
            return Ok(());
        }
    }

    let name = template.name.clone();
    storage.delete_template(&template.id)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        println!("Deleted template '{}'", name);
    }
    Ok(())
}
