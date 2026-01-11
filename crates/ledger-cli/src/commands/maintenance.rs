use std::io::IsTerminal;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ledger_core::StorageEngine;

use crate::app::{
    missing_config_message, missing_ledger_message, open_storage_with_retry, resolve_config_path,
    resolve_ledger_path,
};
use crate::cache::{cache_clear, cache_socket_path, run_cache_daemon};
use crate::cli::Cli;
use crate::config::read_config;

pub fn handle_check(cli: &Cli) -> anyhow::Result<()> {
    let (storage, _passphrase) = open_storage_with_retry(cli, false)?;
    match storage.check_integrity() {
        Ok(()) => {
            if !cli.quiet {
                println!("Integrity check: OK");
                println!("- foreign keys: OK");
                println!("- entries FTS: OK");
                println!("- entry type versions: OK");
                println!("- metadata keys: OK");
            }
        }
        Err(err) => {
            eprintln!("Integrity check: FAILED");
            eprintln!("- error: {}", err);
            eprintln!("Hint: Restore from a backup or export data before retrying.");
            return Err(anyhow::anyhow!("Integrity check failed"));
        }
    }
    Ok(())
}

pub fn handle_backup(cli: &Cli, destination: &str) -> anyhow::Result<()> {
    let source = resolve_ledger_path(cli)?;
    let source_path = Path::new(&source);
    if !source_path.exists() {
        return Err(anyhow::anyhow!(missing_ledger_message(source_path)));
    }
    if std::io::stdin().is_terminal() && !cli.quiet {
        let proceed = dialoguer::Confirm::new()
            .with_prompt(format!("Back up ledger to {}?", destination))
            .default(true)
            .interact()?;
        if !proceed {
            return Err(anyhow::anyhow!("Backup cancelled"));
        }
    }
    let count = backup_atomic_copy(source_path, Path::new(destination))?;
    if count == 0 {
        return Err(anyhow::anyhow!("Backup failed: zero bytes written"));
    }
    if !cli.quiet {
        println!("Backed up ledger to {}", destination);
    }
    Ok(())
}

pub fn handle_lock(cli: &Cli) -> anyhow::Result<()> {
    if let Ok(socket_path) = cache_socket_path() {
        let _ = cache_clear(&socket_path);
    }
    if !cli.quiet {
        println!("Passphrase cache cleared.");
    }
    Ok(())
}

pub fn handle_doctor(cli: &Cli, no_input: bool) -> anyhow::Result<()> {
    let config_path = resolve_config_path()?;
    if !config_path.exists() {
        eprintln!("{}", missing_config_message(&config_path));
        return Err(anyhow::anyhow!("Ledger is not initialized"));
    }

    let config = read_config(&config_path).map_err(|e| anyhow::anyhow!("Config error: {}", e))?;
    let ledger_path = std::path::PathBuf::from(config.ledger.path);
    if !ledger_path.exists() {
        eprintln!("{}", missing_ledger_message(&ledger_path));
        return Err(anyhow::anyhow!("Ledger file missing"));
    }

    let (storage, _passphrase) = open_storage_with_retry(cli, no_input).map_err(|e| {
        anyhow::anyhow!(
            "Failed to open ledger for diagnostics: {}\nHint: Set LEDGER_PASSPHRASE or run in a TTY.",
            e
        )
    })?;

    if let Err(err) = storage.check_integrity() {
        eprintln!("Doctor: FAILED");
        eprintln!("- integrity check: FAILED");
        eprintln!("- error: {}", err);
        eprintln!("Hint: Restore from a backup or export data before retrying.");
        return Err(anyhow::anyhow!("Doctor failed"));
    }

    if !cli.quiet {
        println!("Doctor: OK");
        println!("- config: OK ({})", config_path.display());
        println!("- ledger: OK ({})", ledger_path.display());
        println!("- integrity: OK");
    }

    Ok(())
}

pub fn handle_internal_cache_daemon(ttl: u64, socket: &str) -> anyhow::Result<()> {
    let socket_path = std::path::PathBuf::from(socket);
    run_cache_daemon(std::time::Duration::from_secs(ttl), &socket_path)?;
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
