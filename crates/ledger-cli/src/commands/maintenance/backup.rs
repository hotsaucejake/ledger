use std::io::IsTerminal;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::{missing_ledger_message, resolve_ledger_path, AppContext};
use crate::cli::BackupArgs;

pub fn handle_backup(ctx: &AppContext, args: &BackupArgs) -> anyhow::Result<()> {
    let source = resolve_ledger_path(ctx.cli())?;
    let source_path = Path::new(&source);
    if !source_path.exists() {
        return Err(anyhow::anyhow!(missing_ledger_message(source_path)));
    }
    if std::io::stdin().is_terminal() && !ctx.quiet() {
        let proceed = dialoguer::Confirm::new()
            .with_prompt(format!("Back up ledger to {}?", args.destination))
            .default(true)
            .interact()?;
        if !proceed {
            return Err(anyhow::anyhow!("Backup cancelled"));
        }
    }
    let count = backup_atomic_copy(source_path, Path::new(&args.destination))?;
    if count == 0 {
        return Err(anyhow::anyhow!("Backup failed: zero bytes written"));
    }
    if !ctx.quiet() {
        println!("Backed up ledger to {}", args.destination);
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

    if let Err(err) = std::fs::rename(&temp_path, destination) {
        let _ = std::fs::remove_file(destination);
        std::fs::rename(&temp_path, destination).map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            anyhow::anyhow!("Atomic rename failed ({}): {}", err, e)
        })?;
    }

    Ok(bytes)
}
