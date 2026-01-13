use std::io::IsTerminal;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::{missing_ledger_message, resolve_ledger_path, AppContext};
use crate::cli::BackupArgs;
use crate::ui::{badge, format_bytes, print, Badge, OutputMode};

pub fn handle_backup(ctx: &AppContext, args: &BackupArgs) -> anyhow::Result<()> {
    let source = resolve_ledger_path(ctx.cli())?;
    let source_path = Path::new(&source);
    if !source_path.exists() {
        return Err(anyhow::anyhow!(missing_ledger_message(source_path)));
    }

    let ui_ctx = ctx.ui_context(false, None);

    if std::io::stdin().is_terminal() && !ctx.quiet() {
        let proceed = dialoguer::Confirm::new()
            .with_prompt(format!("Back up ledger to {}?", args.destination))
            .default(true)
            .interact()?;
        if !proceed {
            match ui_ctx.mode {
                OutputMode::Pretty => {
                    print(&ui_ctx, &badge(&ui_ctx, Badge::Warn, "Backup cancelled"));
                }
                OutputMode::Plain | OutputMode::Json => {
                    println!("status=cancelled");
                }
            }
            return Err(anyhow::anyhow!("Backup cancelled"));
        }
    }

    let bytes = backup_atomic_copy(source_path, Path::new(&args.destination))?;
    if bytes == 0 {
        return Err(anyhow::anyhow!("Backup failed: zero bytes written"));
    }

    if !ctx.quiet() {
        match ui_ctx.mode {
            OutputMode::Pretty => {
                print(
                    &ui_ctx,
                    &badge(
                        &ui_ctx,
                        Badge::Ok,
                        &format!(
                            "Backed up ledger to {} ({})",
                            args.destination,
                            format_bytes(bytes)
                        ),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("destination={}", args.destination);
                println!("bytes={}", bytes);
            }
        }
    }
    Ok(())
}

fn backup_atomic_copy(source: &Path, destination: &Path) -> anyhow::Result<u64> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Backup destination has no parent directory"))?;
    std::fs::create_dir_all(parent).map_err(|e| {
        anyhow::anyhow!(
            "Failed to create backup directory {}: {}",
            parent.display(),
            e
        )
    })?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("System time error: {}", e))?
        .as_nanos();
    let temp_path = parent.join(format!(".ledger-backup-{}.tmp", nanos));

    let bytes = std::fs::copy(source, &temp_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to copy ledger from {} to {}: {}",
            source.display(),
            destination.display(),
            e
        )
    })?;

    ledger_core::fs::rename_with_fallback(&temp_path, destination)
        .map_err(|e| anyhow::anyhow!("Atomic rename failed: {}", e))?;

    Ok(bytes)
}
