use dialoguer::Confirm;
use uuid::Uuid;

use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::TemplateDeleteArgs;
use crate::ui::{badge, print, Badge, OutputMode};

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

    let ui_ctx = ctx.ui_context(false, None);

    if !args.force {
        let confirmed = Confirm::new()
            .with_prompt(format!("Delete template '{}'?", template.name))
            .default(false)
            .interact()?;

        if !confirmed {
            if !ctx.quiet() {
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

    let name = template.name.clone();
    storage.delete_template(&template.id)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(
                    &ui_ctx,
                    &badge(&ui_ctx, Badge::Ok, &format!("Deleted template '{}'", name)),
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
