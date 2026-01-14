use chrono::Utc;

use ledger_core::storage::{NewTemplate, StorageEngine};

use crate::app::AppContext;
use crate::cli::TemplateCreateArgs;
use crate::helpers::require_entry_type;
use crate::ui::theme::{styled, styles};
use crate::ui::{badge, blank_line, hint, print, short_id, Badge, OutputMode};

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
        let created_at = Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();

        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(&ui_ctx, &badge(&ui_ctx, Badge::Ok, "Created template"));
                // Context line with name, ID, and entry type
                let context = format!(
                    "Name: {}  \u{00B7}  ID: {}  \u{00B7}  type: {}",
                    args.name,
                    short_id(&template_id),
                    args.entry_type
                );
                let context_styled = styled(&context, styles::dim(), ui_ctx.color);
                println!("{}", context_styled);
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
                // Next step hints
                blank_line(&ui_ctx);
                print(
                    &ui_ctx,
                    &hint(
                        &ui_ctx,
                        &format!(
                            "ledger add {} --template {}  \u{00B7}  ledger template list",
                            args.entry_type, args.name
                        ),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("template_id={}", template_id);
                println!("name={}", args.name);
                println!("entry_type={}", args.entry_type);
                println!("created_at={}", created_at);
                if args.set_default {
                    println!("set_default=true");
                }
            }
        }
    }
    Ok(())
}
