use ledger_core::storage::{NewTemplate, StorageEngine};

use crate::app::AppContext;
use crate::cli::TemplateCreateArgs;
use crate::helpers::require_entry_type;
use crate::ui::{badge, print, short_id, Badge, OutputMode};

pub fn handle_create(ctx: &AppContext, args: &TemplateCreateArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(false)?;
    let entry_type = require_entry_type(&storage, &args.entry_type)?;
    let metadata = storage.metadata()?;

    // Wrap user-provided defaults in the proper template JSON structure
    let template_json: serde_json::Value = if let Some(ref defaults) = args.defaults {
        let user_defaults: serde_json::Value = serde_json::from_str(defaults)
            .map_err(|e| anyhow::anyhow!("Invalid JSON for defaults: {}", e))?;
        serde_json::json!({
            "defaults": user_defaults
        })
    } else {
        serde_json::json!({})
    };

    let mut new_template =
        NewTemplate::new(&args.name, entry_type.id, template_json, metadata.device_id);
    if let Some(ref desc) = args.description {
        new_template = new_template.with_description(desc);
    }

    let template_id = storage.create_template(&new_template)?;

    if args.set_default {
        storage.set_default_template(&entry_type.id, &template_id)?;
    }

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
                            "Created template '{}' ({})",
                            args.name,
                            short_id(&template_id)
                        ),
                    ),
                );
                if args.set_default {
                    print(
                        &ui_ctx,
                        &badge(
                            &ui_ctx,
                            Badge::Info,
                            &format!("Set as default for '{}'", args.entry_type),
                        ),
                    );
                }
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("template_id={}", template_id);
                println!("name={}", args.name);
                if args.set_default {
                    println!("set_default=true");
                    println!("entry_type={}", args.entry_type);
                }
            }
        }
    }
    Ok(())
}
