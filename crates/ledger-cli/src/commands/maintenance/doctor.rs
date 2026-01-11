use ledger_core::StorageEngine;

use crate::app::{
    missing_config_message, missing_ledger_message, open_storage_with_retry, resolve_config_path,
};
use crate::cli::{Cli, DoctorArgs};
use crate::config::read_config;

pub fn handle_doctor(cli: &Cli, args: &DoctorArgs) -> anyhow::Result<()> {
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

    let (storage, _passphrase) = open_storage_with_retry(cli, args.no_input).map_err(|e| {
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
