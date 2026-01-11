use ledger_core::storage::{CompositionFilter, StorageEngine};

use crate::app::AppContext;
use crate::cli::CompositionListArgs;

pub fn handle_list(ctx: &AppContext, args: &CompositionListArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    let mut filter = CompositionFilter::new();
    if let Some(limit) = args.limit {
        filter = filter.limit(limit);
    }

    let compositions = storage.list_compositions(&filter)?;

    if args.json {
        let json_output: Vec<_> = compositions
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id.to_string(),
                    "name": c.name,
                    "description": c.description,
                    "created_at": c.created_at.to_rfc3339(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else if compositions.is_empty() {
        if !ctx.quiet() {
            println!("No compositions found.");
        }
    } else {
        for comp in &compositions {
            if let Some(ref desc) = comp.description {
                println!("{} - {} ({})", comp.name, desc, comp.id);
            } else {
                println!("{} ({})", comp.name, comp.id);
            }
        }
    }

    Ok(())
}
