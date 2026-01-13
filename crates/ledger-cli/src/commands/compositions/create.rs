use ledger_core::storage::{NewComposition, StorageEngine};

use crate::app::AppContext;
use crate::cli::CompositionCreateArgs;
use crate::ui::{badge, print, short_id, Badge, OutputMode};

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
        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(
                    &ui_ctx,
                    &badge(
                        &ui_ctx,
                        Badge::Ok,
                        &format!(
                            "Created composition '{}' ({})",
                            args.name,
                            short_id(&composition_id)
                        ),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("composition_id={}", composition_id);
                println!("name={}", args.name);
            }
        }
    }
    Ok(())
}
