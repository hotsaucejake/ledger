use ledger_core::StorageEngine;
use uuid::Uuid;

use crate::app::{exit_not_found_with_hint, AppContext};
use crate::cli::ShowArgs;
use crate::output::{entry_json, entry_type_name_map, print_entry};

pub fn handle_show(ctx: &AppContext, args: &ShowArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    let parsed =
        Uuid::parse_str(&args.id).map_err(|e| anyhow::anyhow!("Invalid entry ID: {}", e))?;
    let entry = storage.get_entry(&parsed)?.unwrap_or_else(|| {
        exit_not_found_with_hint(
            "Entry not found",
            "Hint: Run `ledger list --last 7d` to find entry IDs.",
        )
    });
    if args.json {
        let name_map = entry_type_name_map(&storage)?;
        let output = serde_json::to_string_pretty(&entry_json(&entry, &name_map))?;
        println!("{}", output);
    } else {
        print_entry(&storage, &entry, ctx.quiet())?;
    }
    Ok(())
}
