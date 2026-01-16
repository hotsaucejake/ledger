//! Add entry command handler with template-first prompting.

use std::io::IsTerminal;

use uuid::Uuid;

use ledger_core::storage::{EntryFilter, NewEntry, NewEntryType, NewTemplate, StorageEngine};

use crate::app::AppContext;
use crate::cli::AddArgs;
use crate::helpers::{
    parse_cli_fields, parse_datetime, prompt_for_fields, FieldDef, PromptResult, TemplateDefaults,
};
use crate::ui::prompt::{prompt_confirm, Wizard, WizardStep};
use crate::ui::theme::{styled, styles};
use crate::ui::{badge, blank_line, hint, print, short_id, Badge, OutputMode, UiContext};

/// Print a step indicator for the add wizard flow.
fn print_step(ctx: &UiContext, step: usize, total: usize, title: &str) {
    if !ctx.mode.is_pretty() {
        return;
    }
    let progress = format!("{}/{}", step, total);
    let progress_styled = styled(&progress, styles::dim(), ctx.color);
    let title_styled = styled(title, styles::bold(), ctx.color);
    println!("{}  {}", progress_styled, title_styled);
}

pub fn handle_add(ctx: &AppContext, args: &AddArgs) -> anyhow::Result<()> {
    let (mut storage, passphrase) = ctx.open_storage(args.no_input)?;
    let metadata = storage.metadata()?;

    // Create UI context for step indicators
    let ui_ctx = ctx.ui_context(false, None);
    let interactive = std::io::stdin().is_terminal() && !args.no_input;
    let needs_prompting = args.body.is_none() && args.fields.is_empty();

    let entry_type_record =
        resolve_entry_type(&mut storage, &ui_ctx, args, interactive, metadata.device_id)?;

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

    // Print wizard header if interactive
    if interactive && needs_prompting && ui_ctx.mode.is_pretty() {
        let header = styled("Ledger", styles::bold(), ui_ctx.color);
        println!("{} \u{00B7} add ({})\n", header, args.entry_type);
        print_step(&ui_ctx, 1, 2, "Enter fields");
    }

    // Prompt for fields based on schema and template defaults
    let PromptResult {
        data,
        enum_additions,
    } = prompt_for_fields(
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

    if let Some(ref tmpl) = template {
        if !enum_additions.is_empty() {
            let updated = apply_enum_additions(&tmpl.template_json, &enum_additions);
            storage.update_template(&tmpl.id, updated).map_err(|e| {
                anyhow::anyhow!(
                    "Entry created, but failed to update template enum values: {}",
                    e
                )
            })?;
        }
    }

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
        // Get created timestamp for receipt
        let created_at = new_entry
            .created_at
            .unwrap_or_else(chrono::Utc::now)
            .format("%Y-%m-%d %H:%M UTC")
            .to_string();
        let tag_count = new_entry.tags.len();

        match ui_ctx.mode {
            OutputMode::Pretty => {
                if interactive && needs_prompting {
                    println!();
                    print_step(&ui_ctx, 2, 2, "Creating entry");
                }
                blank_line(&ui_ctx);
                print(
                    &ui_ctx,
                    &badge(
                        &ui_ctx,
                        Badge::Ok,
                        &format!("Added {} entry", args.entry_type),
                    ),
                );
                // Context line with ID, timestamp, and tag count
                let context = format!(
                    "ID: {}  \u{00B7}  {}  \u{00B7}  tags: {}",
                    short_id(&entry_id),
                    created_at,
                    tag_count
                );
                let context_styled = styled(&context, styles::dim(), ui_ctx.color);
                println!("{}", context_styled);
                // Next step hints
                blank_line(&ui_ctx);
                print(
                    &ui_ctx,
                    &hint(
                        &ui_ctx,
                        &format!(
                            "ledger show {}  \u{00B7}  ledger list  \u{00B7}  ledger edit {}",
                            short_id(&entry_id),
                            short_id(&entry_id)
                        ),
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("entry_id={}", entry_id);
                println!("entry_type={}", args.entry_type);
                println!("created_at={}", created_at);
                println!("tag_count={}", tag_count);
            }
        }
    }
    Ok(())
}

fn resolve_entry_type(
    storage: &mut ledger_core::storage::AgeSqliteStorage,
    ui_ctx: &UiContext,
    args: &AddArgs,
    interactive: bool,
    device_id: Uuid,
) -> anyhow::Result<ledger_core::storage::EntryType> {
    if let Some(entry_type) = storage.get_entry_type(&args.entry_type)? {
        return ensure_default_template(storage, ui_ctx, &entry_type, interactive, device_id);
    }

    if !interactive && args.body.is_none() {
        return Err(anyhow::anyhow!(
            "Entry type \"{}\" not found.\nHint: Run `ledger add {}` on a TTY to create a form.",
            args.entry_type,
            args.entry_type
        ));
    }

    if interactive {
        let choice = prompt_select_create_form(ui_ctx, &args.entry_type)?;
        if choice == CreateChoice::BodyOnly {
            let entry_type = create_body_only_entry_type(storage, &args.entry_type, device_id)?;
            let template =
                create_body_only_template(storage, &entry_type.name, entry_type.id, device_id)?;
            storage.set_default_template(&entry_type.id, &template.id)?;
            Ok(entry_type)
        } else {
            let (schema, template_json) = run_form_builder(ui_ctx, &args.entry_type)?;
            let entry_type = create_entry_type(storage, &args.entry_type, schema, device_id)?;
            let template = create_default_template(
                storage,
                &args.entry_type,
                entry_type.id,
                template_json,
                device_id,
            )?;
            storage.set_default_template(&entry_type.id, &template.id)?;
            Ok(entry_type)
        }
    } else {
        let entry_type = create_body_only_entry_type(storage, &args.entry_type, device_id)?;
        let template =
            create_body_only_template(storage, &entry_type.name, entry_type.id, device_id)?;
        storage.set_default_template(&entry_type.id, &template.id)?;
        Ok(entry_type)
    }
}

fn ensure_default_template(
    storage: &mut ledger_core::storage::AgeSqliteStorage,
    ui_ctx: &UiContext,
    entry_type: &ledger_core::storage::EntryType,
    interactive: bool,
    device_id: Uuid,
) -> anyhow::Result<ledger_core::storage::EntryType> {
    let template = storage.get_default_template(&entry_type.id)?;
    if template.is_some() {
        return Ok(entry_type.clone());
    }

    let filter = EntryFilter {
        entry_type_id: Some(entry_type.id),
        limit: Some(1),
        ..Default::default()
    };
    let entries_exist = !storage.list_entries(&filter)?.is_empty();

    if entries_exist {
        let body_template =
            create_body_only_template(storage, &entry_type.name, entry_type.id, device_id)?;
        storage.set_default_template(&entry_type.id, &body_template.id)?;
    }

    if interactive {
        let prompt = if entries_exist {
            "No template exists for this entry type. Create a form now?"
        } else {
            "Create a form for this entry type?"
        };
        let create_form = prompt_confirm(ui_ctx, prompt, true)?;
        if create_form {
            let (schema, template_json) = run_form_builder(ui_ctx, &entry_type.name)?;
            let _ =
                storage.create_entry_type(&NewEntryType::new(&entry_type.name, schema, device_id));
            let updated_entry_type =
                storage.get_entry_type(&entry_type.name)?.ok_or_else(|| {
                    anyhow::anyhow!("Failed to update entry type {}", entry_type.name)
                })?;
            let template = create_default_template(
                storage,
                &entry_type.name,
                updated_entry_type.id,
                template_json,
                device_id,
            )?;
            storage.set_default_template(&updated_entry_type.id, &template.id)?;
            return Ok(updated_entry_type);
        } else if !entries_exist {
            let body_template =
                create_body_only_template(storage, &entry_type.name, entry_type.id, device_id)?;
            storage.set_default_template(&entry_type.id, &body_template.id)?;
        }
    } else if !entries_exist {
        let body_template =
            create_body_only_template(storage, &entry_type.name, entry_type.id, device_id)?;
        storage.set_default_template(&entry_type.id, &body_template.id)?;
    }

    Ok(entry_type.clone())
}

#[derive(Debug, PartialEq, Eq)]
enum CreateChoice {
    Form,
    BodyOnly,
}

fn prompt_select_create_form(ctx: &UiContext, entry_type: &str) -> anyhow::Result<CreateChoice> {
    if !ctx.mode.is_pretty() {
        return Ok(CreateChoice::BodyOnly);
    }

    let steps = vec![WizardStep::new("Create entry type")
        .with_description("Choose how to define the fields for this entry type.")];
    let wizard = Wizard::new(ctx, &format!("add ({})", entry_type), steps);
    wizard.print_header();
    wizard.print_step();

    let options = ["Create a form (recommended)", "Use simple body only"];
    let theme = dialoguer::theme::ColorfulTheme::default();
    let choice = dialoguer::Select::with_theme(&theme)
        .with_prompt("Form setup")
        .items(&options)
        .default(0)
        .interact()?;
    if choice == 0 {
        Ok(CreateChoice::Form)
    } else {
        Ok(CreateChoice::BodyOnly)
    }
}

fn create_entry_type(
    storage: &mut ledger_core::storage::AgeSqliteStorage,
    name: &str,
    schema: serde_json::Value,
    device_id: Uuid,
) -> anyhow::Result<ledger_core::storage::EntryType> {
    let entry_type = NewEntryType::new(name, schema, device_id);
    storage.create_entry_type(&entry_type)?;
    storage
        .get_entry_type(name)?
        .ok_or_else(|| anyhow::anyhow!("Failed to create entry type {}", name))
}

fn create_body_only_entry_type(
    storage: &mut ledger_core::storage::AgeSqliteStorage,
    name: &str,
    device_id: Uuid,
) -> anyhow::Result<ledger_core::storage::EntryType> {
    let schema = serde_json::json!({
        "fields": [
            {"name": "body", "type": "text", "required": true}
        ]
    });
    create_entry_type(storage, name, schema, device_id)
}

fn create_default_template(
    storage: &mut ledger_core::storage::AgeSqliteStorage,
    entry_type_name: &str,
    entry_type_id: Uuid,
    template_json: serde_json::Value,
    device_id: Uuid,
) -> anyhow::Result<ledger_core::storage::Template> {
    let name = unique_template_name(storage, &format!("{}-default", entry_type_name))?;
    let new_template = NewTemplate::new(name, entry_type_id, template_json, device_id);
    let template_id = storage.create_template(&new_template)?;
    storage
        .get_template_by_id(&template_id)?
        .ok_or_else(|| anyhow::anyhow!("Failed to create default template for {}", entry_type_name))
}

fn unique_template_name(
    storage: &ledger_core::storage::AgeSqliteStorage,
    base: &str,
) -> anyhow::Result<String> {
    if storage.get_template(base)?.is_none() {
        return Ok(base.to_string());
    }
    for idx in 2..1000 {
        let candidate = format!("{}-{}", base, idx);
        if storage.get_template(&candidate)?.is_none() {
            return Ok(candidate);
        }
    }
    Err(anyhow::anyhow!(
        "Failed to find an available template name for {}",
        base
    ))
}

fn create_body_only_template(
    storage: &mut ledger_core::storage::AgeSqliteStorage,
    entry_type_name: &str,
    entry_type_id: Uuid,
    device_id: Uuid,
) -> anyhow::Result<ledger_core::storage::Template> {
    let template_json = serde_json::json!({
        "defaults": {},
        "prompt_overrides": {
            "body": "Body"
        }
    });
    create_default_template(
        storage,
        entry_type_name,
        entry_type_id,
        template_json,
        device_id,
    )
}

fn run_form_builder(
    ctx: &UiContext,
    entry_type: &str,
) -> anyhow::Result<(serde_json::Value, serde_json::Value)> {
    if !ctx.mode.is_pretty() {
        return Err(anyhow::anyhow!(
            "Interactive form builder required. Run on a TTY."
        ));
    }

    let steps = vec![
        WizardStep::new("Form builder").with_description("Define the fields for this entry type."),
        WizardStep::new("Review"),
    ];
    let mut wizard = Wizard::new(ctx, &format!("add ({})", entry_type), steps);
    wizard.print_header();
    wizard.print_step();

    let preset = prompt_select_preset(entry_type)?;
    let mut fields: Vec<FormFieldSpec> = preset.unwrap_or_default();

    loop {
        let field = prompt_field_definition()?;
        fields.push(field);
        let add_more = prompt_confirm(ctx, "Add another field?", true)?;
        if !add_more {
            break;
        }
    }

    if fields.is_empty() {
        return Err(anyhow::anyhow!("At least one field is required"));
    }

    wizard.next_step();
    wizard.print_step();
    for field in &fields {
        let required = if field.required {
            "required"
        } else {
            "optional"
        };
        println!("  {} ({}, {})", field.name, field.field_type, required);
    }
    println!();
    let proceed = prompt_confirm(ctx, "Create form?", true)?;
    if !proceed {
        return Err(anyhow::anyhow!("Form creation cancelled"));
    }

    let schema = build_schema_json(&fields);
    let template_json = build_template_json(&fields);

    Ok((schema, template_json))
}

#[derive(Debug, Clone)]
struct FormFieldSpec {
    name: String,
    field_type: String,
    required: bool,
    default_value: Option<serde_json::Value>,
    enum_values: Option<Vec<String>>,
}

fn prompt_select_preset(_entry_type: &str) -> anyhow::Result<Option<Vec<FormFieldSpec>>> {
    let options = ["Custom form", "Todo list preset"];
    let theme = dialoguer::theme::ColorfulTheme::default();
    let choice = dialoguer::Select::with_theme(&theme)
        .with_prompt("Start from a preset?")
        .items(&options)
        .default(0)
        .interact()?;

    if choice == 1 {
        return Ok(Some(vec![
            FormFieldSpec {
                name: "title".to_string(),
                field_type: "text".to_string(),
                required: false,
                default_value: None,
                enum_values: None,
            },
            FormFieldSpec {
                name: "items".to_string(),
                field_type: "task_list".to_string(),
                required: true,
                default_value: None,
                enum_values: None,
            },
        ]));
    }

    Ok(None)
}

fn prompt_field_definition() -> anyhow::Result<FormFieldSpec> {
    let theme = dialoguer::theme::ColorfulTheme::default();
    let name: String = dialoguer::Input::with_theme(&theme)
        .with_prompt("Field name")
        .interact_text()?;
    if name.trim().is_empty() {
        return Err(anyhow::anyhow!("Field name is required"));
    }
    let field_types = [
        "text", "string", "number", "integer", "date", "datetime", "enum", "boolean",
    ];
    let field_type_idx = dialoguer::Select::with_theme(&theme)
        .with_prompt("Field type")
        .items(&field_types)
        .default(0)
        .interact()?;
    let field_type = field_types[field_type_idx].to_string();
    let required = dialoguer::Confirm::with_theme(&theme)
        .with_prompt("Required?")
        .default(true)
        .interact()?;

    let enum_values = if field_type == "enum" {
        let mut values = Vec::new();
        loop {
            let value: String = dialoguer::Input::with_theme(&theme)
                .with_prompt("Enum value")
                .interact_text()?;
            if !value.trim().is_empty() {
                values.push(value.trim().to_string());
            }
            let add_more = dialoguer::Confirm::with_theme(&theme)
                .with_prompt("Add another enum value?")
                .default(true)
                .interact()?;
            if !add_more {
                break;
            }
        }
        if values.is_empty() {
            return Err(anyhow::anyhow!("Enum fields require at least one value"));
        }
        Some(values)
    } else {
        None
    };

    let default_value = prompt_default_value(&field_type, enum_values.as_deref())?;

    Ok(FormFieldSpec {
        name: name.trim().to_string(),
        field_type,
        required,
        default_value,
        enum_values,
    })
}

fn prompt_default_value(
    field_type: &str,
    enum_values: Option<&[String]>,
) -> anyhow::Result<Option<serde_json::Value>> {
    let theme = dialoguer::theme::ColorfulTheme::default();
    match field_type {
        "date" => {
            let options = ["None", "Today", "Custom"];
            let choice = dialoguer::Select::with_theme(&theme)
                .with_prompt("Default value")
                .items(&options)
                .default(0)
                .interact()?;
            match choice {
                1 => Ok(Some(serde_json::Value::String("today".to_string()))),
                2 => {
                    let value: String = dialoguer::Input::with_theme(&theme)
                        .with_prompt("Default date (YYYY-MM-DD)")
                        .interact_text()?;
                    Ok(if value.trim().is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::String(value))
                    })
                }
                _ => Ok(None),
            }
        }
        "datetime" => {
            let options = ["None", "Now", "Custom"];
            let choice = dialoguer::Select::with_theme(&theme)
                .with_prompt("Default value")
                .items(&options)
                .default(0)
                .interact()?;
            match choice {
                1 => Ok(Some(serde_json::Value::String("now".to_string()))),
                2 => {
                    let value: String = dialoguer::Input::with_theme(&theme)
                        .with_prompt("Default datetime (RFC3339 or YYYY-MM-DD)")
                        .interact_text()?;
                    Ok(if value.trim().is_empty() {
                        None
                    } else {
                        Some(serde_json::Value::String(value))
                    })
                }
                _ => Ok(None),
            }
        }
        "boolean" => {
            let options = ["None", "Yes", "No"];
            let choice = dialoguer::Select::with_theme(&theme)
                .with_prompt("Default value")
                .items(&options)
                .default(0)
                .interact()?;
            match choice {
                1 => Ok(Some(serde_json::Value::Bool(true))),
                2 => Ok(Some(serde_json::Value::Bool(false))),
                _ => Ok(None),
            }
        }
        "enum" => {
            let Some(values) = enum_values else {
                return Ok(None);
            };
            let mut options = vec!["None".to_string()];
            options.extend(values.iter().cloned());
            let choice = dialoguer::Select::with_theme(&theme)
                .with_prompt("Default value")
                .items(&options)
                .default(0)
                .interact()?;
            if choice == 0 {
                Ok(None)
            } else {
                Ok(Some(serde_json::Value::String(options[choice].clone())))
            }
        }
        "number" | "integer" | "string" | "text" => {
            let value: String = dialoguer::Input::with_theme(&theme)
                .with_prompt("Default value (optional)")
                .allow_empty(true)
                .interact_text()?;
            if value.trim().is_empty() {
                Ok(None)
            } else if field_type == "number" {
                let num: f64 = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid number default"))?;
                Ok(Some(serde_json::Value::Number(
                    serde_json::Number::from_f64(num)
                        .ok_or_else(|| anyhow::anyhow!("Invalid number default"))?,
                )))
            } else if field_type == "integer" {
                let num: i64 = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid integer default"))?;
                Ok(Some(serde_json::Value::Number(num.into())))
            } else {
                Ok(Some(serde_json::Value::String(value)))
            }
        }
        _ => Ok(None),
    }
}

fn build_schema_json(fields: &[FormFieldSpec]) -> serde_json::Value {
    let field_defs: Vec<serde_json::Value> = fields
        .iter()
        .map(|field| {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "name".to_string(),
                serde_json::Value::String(field.name.clone()),
            );
            obj.insert(
                "type".to_string(),
                serde_json::Value::String(field.field_type.clone()),
            );
            obj.insert(
                "required".to_string(),
                serde_json::Value::Bool(field.required),
            );
            if let Some(ref values) = field.enum_values {
                obj.insert(
                    "values".to_string(),
                    serde_json::Value::Array(
                        values
                            .iter()
                            .cloned()
                            .map(serde_json::Value::String)
                            .collect(),
                    ),
                );
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    serde_json::json!({ "fields": field_defs })
}

fn build_template_json(fields: &[FormFieldSpec]) -> serde_json::Value {
    let mut defaults = serde_json::Map::new();
    for field in fields {
        if let Some(ref value) = field.default_value {
            defaults.insert(field.name.clone(), value.clone());
        }
    }
    serde_json::json!({ "defaults": defaults })
}

fn apply_enum_additions(
    template_json: &serde_json::Value,
    additions: &[crate::helpers::EnumAddition],
) -> serde_json::Value {
    let mut updated = template_json.clone();

    // Ensure enum_values exists
    if updated
        .as_object()
        .map(|obj| !obj.contains_key("enum_values"))
        .unwrap_or(false)
    {
        updated.as_object_mut().unwrap().insert(
            "enum_values".to_string(),
            serde_json::Value::Object(serde_json::Map::new()),
        );
    }

    let enum_values_obj = updated
        .as_object_mut()
        .unwrap()
        .get_mut("enum_values")
        .and_then(|v| v.as_object_mut())
        .unwrap();

    for addition in additions {
        let entry = enum_values_obj
            .entry(addition.field.clone())
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        let arr = entry.as_array_mut().unwrap();
        if !arr.iter().any(|v| v.as_str() == Some(&addition.value)) {
            arr.push(serde_json::Value::String(addition.value.clone()));
        }
    }

    updated
}
