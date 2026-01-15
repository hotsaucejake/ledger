use uuid::Uuid;

use jot_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::TemplateSetDefaultArgs;
use crate::helpers::require_entry_type;
use crate::ui::{badge, print, Badge, OutputMode};

pub fn handle_set_default(ctx: &AppContext, args: &TemplateSetDefaultArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(false)?;
    let entry_type = require_entry_type(&storage, &args.entry_type)?;

    // Try to find template by name first, then by ID
    let template = if let Ok(uuid) = Uuid::parse_str(&args.template) {
        storage.get_template_by_id(&uuid)?
    } else {
        storage.get_template(&args.template)?
    };

    let template =
        template.ok_or_else(|| anyhow::anyhow!("Template '{}' not found", args.template))?;

    // Verify template is for the correct entry type
    if template.entry_type_id != entry_type.id {
        let entry_types = storage.list_entry_types()?;
        let template_entry_type_name = entry_types
            .iter()
            .find(|et| et.id == template.entry_type_id)
            .map(|et| et.name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        return Err(anyhow::anyhow!(
            "Template '{}' is for entry type '{}', not '{}'",
            template.name,
            template_entry_type_name,
            args.entry_type
        ));
    }

    let template_name = template.name.clone();
    storage.set_default_template(&entry_type.id, &template.id)?;
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
                        &format!(
                            "Set '{}' as default template for '{}'",
                            template_name, args.entry_type
                        ),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("template={}", template_name);
                println!("entry_type={}", args.entry_type);
            }
        }
    }
    Ok(())
}
