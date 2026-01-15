use std::io::IsTerminal;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::{missing_jot_message, resolve_jot_path, AppContext};
use crate::cli::BackupArgs;
use crate::ui::progress::Spinner;
use crate::ui::theme::{styled, styles};
use crate::ui::{badge, blank_line, format_bytes, hint, print, Badge, OutputMode};

pub fn handle_backup(ctx: &AppContext, args: &BackupArgs) -> anyhow::Result<()> {
    let source = resolve_jot_path(ctx.cli())?;
    let source_path = Path::new(&source);
    if !source_path.exists() {
        return Err(anyhow::anyhow!(missing_jot_message(source_path)));
    }

    let ui_ctx = ctx.ui_context(false, None);

    if std::io::stdin().is_terminal() && !ctx.quiet() {
        let proceed = dialoguer::Confirm::new()
            .with_prompt(format!("Back up jot to {}?", args.destination))
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

    // Show spinner during backup for interactive mode
    let spinner = if ui_ctx.mode.is_pretty() && !ctx.quiet() {
        let s = Spinner::new(&ui_ctx, "Backing up");
        s.start();
        Some(s)
    } else {
        None
    };

    let bytes = backup_atomic_copy(source_path, Path::new(&args.destination))?;

    // Finish spinner
    if let Some(s) = spinner {
        s.finish("Backup complete");
    }

    if bytes == 0 {
        return Err(anyhow::anyhow!("Backup failed: zero bytes written"));
    }

    if !ctx.quiet() {
        match ui_ctx.mode {
            OutputMode::Pretty => {
                // Context line with destination and size
                let context = format!(
                    "Path: {}  \u{00B7}  Size: {}",
                    args.destination,
                    format_bytes(bytes)
                );
                let context_styled = styled(&context, styles::dim(), ui_ctx.color);
                println!("{}", context_styled);
                // Next step hints
                blank_line(&ui_ctx);
                print(&ui_ctx, &hint(&ui_ctx, "jot doctor  \u{00B7}  jot check"));
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
    let temp_path = parent.join(format!(".jot-backup-{}.tmp", nanos));

    let bytes = std::fs::copy(source, &temp_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to copy jot from {} to {}: {}",
            source.display(),
            destination.display(),
            e
        )
    })?;

    jot_core::fs::rename_with_fallback(&temp_path, destination)
        .map_err(|e| anyhow::anyhow!("Atomic rename failed: {}", e))?;

    Ok(bytes)
}
