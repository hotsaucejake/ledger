use uuid::Uuid;

use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::TemplateUpdateArgs;
use crate::ui::{badge, print, Badge, OutputMode};

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

    // Wrap user-provided defaults in the proper template JSON structure
    let user_defaults: serde_json::Value = serde_json::from_str(&args.defaults)
        .map_err(|e| anyhow::anyhow!("Invalid JSON for defaults: {}", e))?;
    let new_template_json = serde_json::json!({
        "defaults": user_defaults
    });

    let name = template.name.clone();
    let new_version = storage.update_template(&template.id, new_template_json)?;
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
                        &format!("Updated template '{}' to version {}", name, new_version),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("name={}", name);
                println!("version={}", new_version);
            }
        }
    }
    Ok(())
}
