//! Add entry command handler with template-first prompting.

use uuid::Uuid;

use ledger_core::storage::{NewEntry, StorageEngine};

use crate::app::AppContext;
use crate::cli::AddArgs;
use crate::helpers::{
    parse_cli_fields, parse_datetime, prompt_for_fields, require_entry_type, FieldDef,
    TemplateDefaults,
};

pub fn handle_add(ctx: &AppContext, args: &AddArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(args.no_input)?;
    let entry_type_record = require_entry_type(&storage, &args.entry_type)?;
    let metadata = storage.metadata()?;

    // Get template (explicit or default)
    let template = if let Some(ref template_name) = args.template {
        // Try by name first, then by ID
        let tmpl = if let Ok(uuid) = Uuid::parse_str(template_name) {
            storage.get_template_by_id(&uuid)?
        } else {
            storage.get_template(template_name)?
        };

        let tmpl = tmpl.ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template_name))?;

        // Verify template is for the correct entry type
        if tmpl.entry_type_id != entry_type_record.id {
            return Err(anyhow::anyhow!(
                "Template '{}' is for a different entry type",
                template_name
            ));
        }

        Some(tmpl)
    } else {
        // Get default template for this entry type
        storage.get_default_template(&entry_type_record.id)?
    };

    // Parse template defaults
    let template_defaults = template
        .as_ref()
        .map(|t| TemplateDefaults::from_template_json(&t.template_json))
        .unwrap_or_default();

    // Parse field definitions from entry type schema
    let fields = FieldDef::from_schema(&entry_type_record.schema_json);

    // Parse CLI-provided field values
    let mut cli_values = parse_cli_fields(&args.fields)?;

    // Handle legacy --body flag as a field value
    if let Some(ref body) = args.body {
        cli_values.insert("body".to_string(), body.clone());
    }

    // Get editor override
    let editor_override = ctx.editor()?;

    // Prompt for fields based on schema and template defaults
    let data = prompt_for_fields(
        &fields,
        &template_defaults,
        &cli_values,
        args.no_input,
        editor_override,
    )?;

    // Build entry
    let mut new_entry = NewEntry::new(
        entry_type_record.id,
        entry_type_record.version,
        serde_json::Value::Object(data),
        metadata.device_id,
    );

    // Handle tags: CLI tags override template defaults
    let tags = if !args.tag.is_empty() {
        args.tag.clone()
    } else {
        template_defaults.default_tags.clone()
    };
    new_entry = new_entry.with_tags(tags);

    // Handle custom date
    if let Some(ref value) = args.date {
        let parsed = parse_datetime(value)?;
        new_entry = new_entry.with_created_at(parsed);
    }

    // Insert entry
    let entry_id = storage.insert_entry(&new_entry)?;

    // Handle composition attachments
    if !args.no_compose {
        // Collect compositions to attach to
        let mut composition_ids = Vec::new();

        // Add CLI-specified compositions
        for comp_name in &args.compose {
            let comp = if let Ok(uuid) = Uuid::parse_str(comp_name) {
                storage.get_composition_by_id(&uuid)?
            } else {
                storage.get_composition(comp_name)?
            };

            if let Some(c) = comp {
                composition_ids.push(c.id);
            } else {
                return Err(anyhow::anyhow!("Composition '{}' not found", comp_name));
            }
        }

        // Add template default compositions (unless CLI compositions were specified)
        if args.compose.is_empty() {
            for comp_id_str in &template_defaults.default_compositions {
                if let Ok(uuid) = Uuid::parse_str(comp_id_str) {
                    // Verify composition exists
                    if storage.get_composition_by_id(&uuid)?.is_some() {
                        composition_ids.push(uuid);
                    }
                }
            }
        }

        // Attach entry to compositions
        for comp_id in &composition_ids {
            storage.attach_entry_to_composition(&entry_id, comp_id)?;
        }
    }

    storage.close(&passphrase)?;

    if !ctx.quiet() {
        println!("Added entry {}", entry_id);
    }
    Ok(())
}
