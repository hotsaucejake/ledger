use ledger_core::StorageEngine;

use crate::app::AppContext;

pub fn handle_check(ctx: &AppContext) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;
    match storage.check_integrity() {
        Ok(()) => {
            if !ctx.quiet() {
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
