use ledger_core::StorageEngine;

use crate::app::open_storage_with_retry;
use crate::cli::Cli;

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
