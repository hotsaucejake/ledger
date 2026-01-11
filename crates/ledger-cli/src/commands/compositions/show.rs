use uuid::Uuid;

use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::CompositionShowArgs;

pub fn handle_show(ctx: &AppContext, args: &CompositionShowArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    // Try to find by name first, then by ID
    let composition = if let Ok(uuid) = Uuid::parse_str(&args.name_or_id) {
        storage.get_composition_by_id(&uuid)?
    } else {
        storage.get_composition(&args.name_or_id)?
    };

    let composition = composition
        .ok_or_else(|| anyhow::anyhow!("Composition '{}' not found", args.name_or_id))?;

    if args.json {
        let entries = storage.get_composition_entries(&composition.id)?;
        let json_output = serde_json::json!({
            "id": composition.id.to_string(),
            "name": composition.name,
            "description": composition.description,
            "created_at": composition.created_at.to_rfc3339(),
            "device_id": composition.device_id.to_string(),
            "metadata": composition.metadata,
            "entry_count": entries.len(),
        });
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else {
        println!("Name:        {}", composition.name);
        println!("ID:          {}", composition.id);
        if let Some(ref desc) = composition.description {
            println!("Description: {}", desc);
        }
        println!(
            "Created:     {}",
            composition.created_at.format("%Y-%m-%d %H:%M:%S")
        );

        let entries = storage.get_composition_entries(&composition.id)?;
        println!("Entries:     {}", entries.len());
    }

    Ok(())
}
