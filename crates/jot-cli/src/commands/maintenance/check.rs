use jot_core::StorageEngine;

use crate::app::AppContext;
use crate::ui::{badge, hint, print, Badge, OutputMode, StepList};

pub fn handle_check(ctx: &AppContext) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    let ui_ctx = ctx.ui_context(false, None);

    match storage.check_integrity() {
        Ok(()) => {
            if !ctx.quiet() {
                match ui_ctx.mode {
                    OutputMode::Pretty => {
                        let mut steps = StepList::new(
                            &ui_ctx,
                            &[
                                "foreign keys",
                                "entries FTS",
                                "entry type versions",
                                "metadata keys",
                            ],
                        );
                        steps.start("Integrity check");
                        steps.ok();
                        steps.ok();
                        steps.ok();
                        steps.ok();
                        println!();
                        print(&ui_ctx, &badge(&ui_ctx, Badge::Ok, "All checks passed"));
                    }
                    OutputMode::Plain | OutputMode::Json => {
                        println!("check=foreign_keys ok");
                        println!("check=entries_fts ok");
                        println!("check=entry_type_versions ok");
                        println!("check=metadata_keys ok");
                        println!("status=ok");
                    }
                }
            }
        }
        Err(err) => {
            match ui_ctx.mode {
                OutputMode::Pretty => {
                    print(
                        &ui_ctx,
                        &badge(&ui_ctx, Badge::Err, "Integrity check failed"),
                    );
                    eprintln!("Error: {}", err);
                    print(
                        &ui_ctx,
                        &hint(
                            &ui_ctx,
                            "Restore from a backup or export data before retrying.",
                        ),
                    );
                }
                OutputMode::Plain | OutputMode::Json => {
                    eprintln!("status=failed");
                    eprintln!("error={}", err);
                }
            }
            return Err(anyhow::anyhow!("Integrity check failed"));
        }
    }
    Ok(())
}
