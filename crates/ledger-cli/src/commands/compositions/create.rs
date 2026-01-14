use chrono::Utc;

use ledger_core::storage::{NewComposition, StorageEngine};

use crate::app::AppContext;
use crate::cli::CompositionCreateArgs;
use crate::ui::theme::{styled, styles};
use crate::ui::{badge, blank_line, hint, print, short_id, Badge, OutputMode};

pub fn handle_create(ctx: &AppContext, args: &CompositionCreateArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(false)?;
    let metadata = storage.metadata()?;

    let mut new_composition = NewComposition::new(&args.name, metadata.device_id);
    if let Some(ref desc) = args.description {
        new_composition = new_composition.with_description(desc);
    }

    let composition_id = storage.create_composition(&new_composition)?;
    storage.close(&passphrase)?;

    if !ctx.quiet() {
        let ui_ctx = ctx.ui_context(false, None);
        let created_at = Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();

        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(&ui_ctx, &badge(&ui_ctx, Badge::Ok, "Created composition"));
                // Context line with name, ID, and timestamp
                let context = format!(
                    "Name: {}  \u{00B7}  ID: {}  \u{00B7}  {}",
                    args.name,
                    short_id(&composition_id),
                    created_at
                );
                let context_styled = styled(&context, styles::dim(), ui_ctx.color);
                println!("{}", context_styled);
                // Next step hints
                blank_line(&ui_ctx);
                print(
                    &ui_ctx,
                    &hint(
                        &ui_ctx,
                        &format!(
                            "ledger attach <entry-id> {}  \u{00B7}  ledger composition list",
                            args.name
                        ),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("composition_id={}", composition_id);
                println!("name={}", args.name);
                println!("created_at={}", created_at);
            }
        }
    }
    Ok(())
}
