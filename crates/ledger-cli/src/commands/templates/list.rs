use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::TemplateListArgs;
use crate::helpers::require_entry_type;

pub fn handle_list(ctx: &AppContext, args: &TemplateListArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    let templates = storage.list_templates()?;

    // Filter by entry type if specified
    let filtered_templates: Vec<_> = if let Some(ref entry_type_name) = args.entry_type {
        let entry_type = require_entry_type(&storage, entry_type_name)?;
        templates
            .into_iter()
            .filter(|t| t.entry_type_id == entry_type.id)
            .collect()
    } else {
        templates
    };

    // Build entry type name lookup
    let entry_types = storage.list_entry_types()?;
    let entry_type_names: std::collections::HashMap<_, _> = entry_types
        .iter()
        .map(|et| (et.id, et.name.clone()))
        .collect();

    if args.json {
        let json_output: Vec<_> = filtered_templates
            .iter()
            .map(|t| {
                let entry_type_name = entry_type_names
                    .get(&t.entry_type_id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                serde_json::json!({
                    "id": t.id.to_string(),
                    "name": t.name,
                    "entry_type": entry_type_name,
                    "entry_type_id": t.entry_type_id.to_string(),
                    "version": t.version,
                    "description": t.description,
                    "created_at": t.created_at.to_rfc3339(),
                    "template_json": t.template_json,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else if filtered_templates.is_empty() {
        if !ctx.quiet() {
            println!("No templates found.");
        }
    } else {
        for tmpl in &filtered_templates {
            let entry_type_name = entry_type_names
                .get(&tmpl.entry_type_id)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            let desc = tmpl.description.as_deref().unwrap_or("");
            if desc.is_empty() {
                println!(
                    "{} [{}] v{} ({})",
                    tmpl.name, entry_type_name, tmpl.version, tmpl.id
                );
            } else {
                println!(
                    "{} [{}] v{} - {} ({})",
                    tmpl.name, entry_type_name, tmpl.version, desc, tmpl.id
                );
            }
        }
    }

    Ok(())
}
