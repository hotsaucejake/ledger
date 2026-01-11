use uuid::Uuid;

use ledger_core::storage::StorageEngine;

use crate::app::AppContext;
use crate::cli::TemplateShowArgs;

pub fn handle_show(ctx: &AppContext, args: &TemplateShowArgs) -> anyhow::Result<()> {
    let (storage, _passphrase) = ctx.open_storage(false)?;

    // Try to find by name first, then by ID
    let template = if let Ok(uuid) = Uuid::parse_str(&args.name_or_id) {
        storage.get_template_by_id(&uuid)?
    } else {
        storage.get_template(&args.name_or_id)?
    };

    let template =
        template.ok_or_else(|| anyhow::anyhow!("Template '{}' not found", args.name_or_id))?;

    // Get entry type name by finding in the list
    let entry_types = storage.list_entry_types()?;
    let entry_type_name = entry_types
        .iter()
        .find(|et| et.id == template.entry_type_id)
        .map(|et| et.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    if args.json {
        let json_output = serde_json::json!({
            "id": template.id.to_string(),
            "name": template.name,
            "entry_type": entry_type_name,
            "entry_type_id": template.entry_type_id.to_string(),
            "version": template.version,
            "description": template.description,
            "created_at": template.created_at.to_rfc3339(),
            "device_id": template.device_id.to_string(),
            "template_json": template.template_json,
        });
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else {
        println!("Name:        {}", template.name);
        println!("ID:          {}", template.id);
        println!("Entry Type:  {}", entry_type_name);
        println!("Version:     {}", template.version);
        if let Some(ref desc) = template.description {
            println!("Description: {}", desc);
        }
        println!(
            "Created:     {}",
            template.created_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!("Defaults:");
        println!("{}", serde_json::to_string_pretty(&template.template_json)?);
    }

    Ok(())
}
