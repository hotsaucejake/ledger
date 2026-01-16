//! Field prompting and validation for entry creation.

use std::collections::HashMap;
use std::io::{self, IsTerminal};

use chrono::{NaiveDate, Utc};
use dialoguer::{Confirm, Input, MultiSelect, Select};
use serde_json::Value;

/// Field definition parsed from entry type schema
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub prompt: Option<String>,
    pub order: Option<i32>,
    pub values: Option<Vec<String>>, // For enum fields
    pub multiple: bool,              // For multi-select enums
}

impl FieldDef {
    /// Parse field definitions from entry type schema JSON
    pub fn from_schema(schema: &Value) -> Vec<FieldDef> {
        let mut fields = Vec::new();

        if let Some(field_array) = schema.get("fields").and_then(|f| f.as_array()) {
            for field in field_array {
                if let Some(name) = field.get("name").and_then(|n| n.as_str()) {
                    let field_type = field
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("string")
                        .to_string();

                    let required = field
                        .get("required")
                        .and_then(|r| r.as_bool())
                        .unwrap_or(false);

                    let prompt = field
                        .get("prompt")
                        .and_then(|p| p.as_str())
                        .map(|s| s.to_string());

                    let order = field
                        .get("order")
                        .and_then(|o| o.as_i64())
                        .map(|o| o as i32);

                    let values = field.get("values").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    });

                    let multiple = field
                        .get("multiple")
                        .and_then(|m| m.as_bool())
                        .unwrap_or(false);

                    fields.push(FieldDef {
                        name: name.to_string(),
                        field_type,
                        required,
                        prompt,
                        order,
                        values,
                        multiple,
                    });
                }
            }
        }

        // Sort by order if specified
        fields.sort_by(|a, b| match (a.order, b.order) {
            (Some(ao), Some(bo)) => ao.cmp(&bo),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        fields
    }
}

/// Template defaults parsed from template JSON
#[derive(Debug, Clone, Default)]
pub struct TemplateDefaults {
    pub defaults: HashMap<String, Value>,
    pub default_tags: Vec<String>,
    pub default_compositions: Vec<String>,
    pub prompt_overrides: HashMap<String, String>,
    pub enum_values: HashMap<String, Vec<String>>,
}

impl TemplateDefaults {
    /// Parse template defaults from template JSON
    pub fn from_template_json(template_json: &Value) -> Self {
        let mut result = TemplateDefaults::default();

        if let Some(defaults) = template_json.get("defaults").and_then(|d| d.as_object()) {
            for (k, v) in defaults {
                result.defaults.insert(k.clone(), v.clone());
            }
        }

        if let Some(tags) = template_json.get("default_tags").and_then(|t| t.as_array()) {
            result.default_tags = tags
                .iter()
                .filter_map(|t| t.as_str().map(|s| s.to_string()))
                .collect();
        }

        if let Some(compositions) = template_json
            .get("default_compositions")
            .and_then(|c| c.as_array())
        {
            result.default_compositions = compositions
                .iter()
                .filter_map(|c| c.as_str().map(|s| s.to_string()))
                .collect();
        }

        if let Some(overrides) = template_json
            .get("prompt_overrides")
            .and_then(|p| p.as_object())
        {
            for (k, v) in overrides {
                if let Some(s) = v.as_str() {
                    result.prompt_overrides.insert(k.clone(), s.to_string());
                }
            }
        }

        if let Some(enum_values) = template_json.get("enum_values").and_then(|v| v.as_object()) {
            for (k, v) in enum_values {
                if let Some(arr) = v.as_array() {
                    let values = arr
                        .iter()
                        .filter_map(|val| val.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>();
                    result.enum_values.insert(k.clone(), values);
                }
            }
        }

        result
    }
}

#[derive(Debug, Clone)]
pub struct EnumAddition {
    pub field: String,
    pub value: String,
}

#[derive(Debug)]
pub struct PromptResult {
    pub data: serde_json::Map<String, Value>,
    pub enum_additions: Vec<EnumAddition>,
}

/// Prompt for field values based on schema and template defaults
pub fn prompt_for_fields(
    fields: &[FieldDef],
    template_defaults: &TemplateDefaults,
    cli_values: &HashMap<String, String>,
    no_input: bool,
    editor_override: Option<&str>,
) -> anyhow::Result<PromptResult> {
    let mut data = serde_json::Map::new();
    let mut enum_additions = Vec::new();
    let interactive = io::stdin().is_terminal() && !no_input;

    for field in fields {
        // Check if value was provided via CLI
        if let Some(cli_value) = cli_values.get(&field.name) {
            let allowed = merged_enum_values(field, template_defaults);
            let value =
                parse_field_value(&field.field_type, cli_value, &allowed, field.multiple, true)?;
            data.insert(field.name.clone(), value);
            continue;
        }

        // Check if template has a default
        let default_value = template_defaults.defaults.get(&field.name);

        // Get prompt text (template override > field prompt > field name)
        let prompt_text = template_defaults
            .prompt_overrides
            .get(&field.name)
            .cloned()
            .or_else(|| field.prompt.clone())
            .unwrap_or_else(|| capitalize(&field.name));

        // Determine prompting behavior based on whether CLI values were provided:
        // - No CLI values: prompt for all fields (template defaults pre-filled)
        // - Some CLI values: only prompt for required fields not provided; apply defaults only for required fields
        // - All fields optional + flags: store only flag values (no extra defaults)
        let has_cli_values = !cli_values.is_empty();

        let needs_prompt = if has_cli_values {
            // CLI values present - only prompt for required fields without values or defaults
            field.required && default_value.is_none()
        } else {
            // No CLI values - prompt for all fields (unless --no-input)
            !no_input
        };

        if needs_prompt {
            // Text fields can use editor even without a TTY
            let can_use_editor = field.field_type == "text" && editor_override.is_some();

            if !interactive && !can_use_editor {
                if field.required {
                    return Err(anyhow::anyhow!(
                        "Required field '{}' not provided and no default available",
                        field.name
                    ));
                }
                // Optional field with no default - skip
                continue;
            }

            let allowed = merged_enum_values(field, template_defaults);
            let result = prompt_single_field(
                field,
                &prompt_text,
                default_value,
                &allowed,
                editor_override,
                interactive,
            )?;
            if let Some(v) = result.value {
                data.insert(field.name.clone(), v);
            }
            if let Some(addition) = result.enum_addition {
                enum_additions.push(EnumAddition {
                    field: field.name.clone(),
                    value: addition,
                });
            }
        } else if !has_cli_values {
            // No CLI values provided - apply all template defaults
            if let Some(default) = default_value {
                let value = resolve_default_value(&field.field_type, default)?;
                data.insert(field.name.clone(), value);
            }
        } else if field.required {
            // CLI values present but this required field not provided - use default if available
            if let Some(default) = default_value {
                let value = resolve_default_value(&field.field_type, default)?;
                data.insert(field.name.clone(), value);
            }
        }
        // Optional field not provided via CLI when other flags present - skip (per M5 spec)
    }

    Ok(PromptResult {
        data,
        enum_additions,
    })
}

struct PromptFieldResult {
    value: Option<Value>,
    enum_addition: Option<String>,
}

/// Prompt for a single field value
fn prompt_single_field(
    field: &FieldDef,
    prompt_text: &str,
    default_value: Option<&Value>,
    allowed_enum_values: &Option<Vec<String>>,
    editor_override: Option<&str>,
    interactive: bool,
) -> anyhow::Result<PromptFieldResult> {
    match field.field_type.as_str() {
        "string" | "date" | "datetime" | "number" | "integer" => {
            if !interactive {
                // Non-interactive mode - use default or fail
                if let Some(default) = default_value {
                    return Ok(PromptFieldResult {
                        value: Some(default.clone()),
                        enum_addition: None,
                    });
                }
                return Err(anyhow::anyhow!(
                    "Field '{}' requires interactive input",
                    field.name
                ));
            }

            let default_str = default_string_for_field(&field.field_type, default_value);

            let mut input = Input::<String>::new().with_prompt(prompt_text);

            if let Some(ref default) = default_str {
                input = input.default(default.clone());
            }

            if !field.required {
                input = input.allow_empty(true);
            }

            let result = input.interact_text()?;

            if result.is_empty() && !field.required {
                return Ok(PromptFieldResult {
                    value: None,
                    enum_addition: None,
                });
            }

            let parsed = parse_field_value(
                &field.field_type,
                &result,
                allowed_enum_values,
                field.multiple,
                false,
            )?;
            Ok(PromptFieldResult {
                value: Some(parsed),
                enum_addition: None,
            })
        }

        "text" => {
            // Text fields need editor - check if we can use one
            if !interactive && editor_override.is_none() {
                // Non-interactive mode without editor - use default or fail
                if let Some(default) = default_value {
                    return Ok(PromptFieldResult {
                        value: Some(default.clone()),
                        enum_addition: None,
                    });
                }
                if field.required {
                    return Err(anyhow::anyhow!(
                        "Required text field '{}' needs an editor (set $EDITOR or use --editor)",
                        field.name
                    ));
                }
                return Ok(PromptFieldResult {
                    value: None,
                    enum_addition: None,
                });
            }
            // Use editor for multiline text (works even without TTY if editor is set)
            let initial = default_value.and_then(|v| v.as_str());
            let body = super::read_entry_body(false, None, editor_override, initial)?;
            Ok(PromptFieldResult {
                value: Some(Value::String(body)),
                enum_addition: None,
            })
        }

        "boolean" => {
            let default_bool = default_value.and_then(|v| v.as_bool()).unwrap_or(false);
            let options = vec!["Yes", "No"];
            let default_idx = if default_bool { 0 } else { 1 };

            let selection = Select::new()
                .with_prompt(prompt_text)
                .items(&options)
                .default(default_idx)
                .interact()?;

            Ok(PromptFieldResult {
                value: Some(Value::Bool(selection == 0)),
                enum_addition: None,
            })
        }

        "enum" => {
            if let Some(ref values) = allowed_enum_values {
                if field.multiple {
                    // Multi-select enum
                    let defaults: Vec<bool> = if let Some(Value::Array(arr)) = default_value {
                        values
                            .iter()
                            .map(|v| arr.iter().any(|a| a.as_str() == Some(v)))
                            .collect()
                    } else {
                        vec![false; values.len()]
                    };

                    let selections = MultiSelect::new()
                        .with_prompt(prompt_text)
                        .items(values)
                        .defaults(&defaults)
                        .interact()?;

                    let selected: Vec<Value> = selections
                        .iter()
                        .map(|&i| Value::String(values[i].clone()))
                        .collect();

                    Ok(PromptFieldResult {
                        value: Some(Value::Array(selected)),
                        enum_addition: None,
                    })
                } else {
                    // Single-select enum
                    let default_idx = default_value
                        .and_then(|v| v.as_str())
                        .and_then(|s| values.iter().position(|v| v == s))
                        .unwrap_or(0);

                    let mut options = values.clone();
                    options.push("Other...".to_string());
                    let selection = Select::new()
                        .with_prompt(prompt_text)
                        .items(&options)
                        .default(default_idx)
                        .interact()?;

                    if selection == options.len() - 1 {
                        let input = Input::<String>::new()
                            .with_prompt("Custom value")
                            .allow_empty(false);
                        let custom = input.interact_text()?;
                        let add = Confirm::new()
                            .with_prompt("Add this value to the template for future entries?")
                            .default(true)
                            .interact()?;
                        return Ok(PromptFieldResult {
                            value: Some(Value::String(custom.clone())),
                            enum_addition: add.then_some(custom),
                        });
                    }

                    Ok(PromptFieldResult {
                        value: Some(Value::String(options[selection].clone())),
                        enum_addition: None,
                    })
                }
            } else {
                Err(anyhow::anyhow!(
                    "Enum field '{}' has no values defined",
                    field.name
                ))
            }
        }

        "task_list" => {
            if !interactive {
                if let Some(default) = default_value {
                    return Ok(PromptFieldResult {
                        value: Some(default.clone()),
                        enum_addition: None,
                    });
                }
                return Err(anyhow::anyhow!(
                    "Field '{}' requires interactive input",
                    field.name
                ));
            }

            let mut tasks = Vec::new();
            loop {
                let mut input = Input::<String>::new().with_prompt("Task");
                if tasks.is_empty() {
                    input = input.with_prompt(prompt_text);
                }
                let text = input.allow_empty(true).interact_text()?;
                if text.trim().is_empty() {
                    break;
                }
                let done = Confirm::new()
                    .with_prompt("Mark as done?")
                    .default(false)
                    .interact()?;
                tasks.push(serde_json::json!({
                    "text": text,
                    "done": done
                }));
            }

            if field.required && tasks.is_empty() {
                return Err(anyhow::anyhow!("Field '{}' is required", field.name));
            }

            Ok(PromptFieldResult {
                value: Some(Value::Array(tasks)),
                enum_addition: None,
            })
        }

        "tags" => {
            let default_str = default_value
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            let mut input = Input::<String>::new()
                .with_prompt(format!("{} (comma-separated)", prompt_text))
                .allow_empty(true);

            if !default_str.is_empty() {
                input = input.default(default_str);
            }

            let result = input.interact_text()?;

            if result.is_empty() {
                Ok(PromptFieldResult {
                    value: None,
                    enum_addition: None,
                })
            } else {
                let tags: Vec<Value> = result
                    .split(',')
                    .map(|s| Value::String(s.trim().to_string()))
                    .filter(|v| !v.as_str().unwrap_or("").is_empty())
                    .collect();
                Ok(PromptFieldResult {
                    value: Some(Value::Array(tags)),
                    enum_addition: None,
                })
            }
        }

        _ => {
            // Unknown type - treat as string
            let default_str = default_value
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut input = Input::<String>::new().with_prompt(prompt_text);

            if let Some(ref default) = default_str {
                input = input.default(default.clone());
            }

            if !field.required {
                input = input.allow_empty(true);
            }

            let result = input.interact_text()?;

            if result.is_empty() && !field.required {
                Ok(PromptFieldResult {
                    value: None,
                    enum_addition: None,
                })
            } else {
                Ok(PromptFieldResult {
                    value: Some(Value::String(result)),
                    enum_addition: None,
                })
            }
        }
    }
}

/// Parse a CLI-provided value into the appropriate JSON type
fn parse_field_value(
    field_type: &str,
    value: &str,
    enum_values: &Option<Vec<String>>,
    multiple: bool,
    allow_custom_enum: bool,
) -> anyhow::Result<Value> {
    match field_type {
        "string" | "text" => Ok(Value::String(value.to_string())),

        "date" => {
            let date = if value.eq_ignore_ascii_case("today") {
                Utc::now().date_naive()
            } else {
                NaiveDate::parse_from_str(value, "%Y-%m-%d")
                    .map_err(|_| anyhow::anyhow!("Invalid date (expected YYYY-MM-DD): {}", value))?
            };
            Ok(Value::String(date.format("%Y-%m-%d").to_string()))
        }

        "datetime" => {
            let dt = if value.eq_ignore_ascii_case("now") {
                Utc::now()
            } else {
                crate::helpers::parse_datetime(value)?
            };
            Ok(Value::String(dt.to_rfc3339()))
        }

        "number" => {
            let num: f64 = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid number: {}", value))?;
            Ok(Value::Number(
                serde_json::Number::from_f64(num)
                    .ok_or_else(|| anyhow::anyhow!("Invalid number: {}", value))?,
            ))
        }

        "integer" => {
            let num: i64 = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid integer: {}", value))?;
            Ok(Value::Number(num.into()))
        }

        "boolean" => {
            let bool_val = match value.to_lowercase().as_str() {
                "true" | "yes" | "1" | "y" => true,
                "false" | "no" | "0" | "n" => false,
                _ => return Err(anyhow::anyhow!("Invalid boolean: {}", value)),
            };
            Ok(Value::Bool(bool_val))
        }

        "enum" => {
            if let Some(ref allowed) = enum_values {
                if multiple {
                    // Multi-select: accept comma-separated values, store as array
                    let values: Vec<String> =
                        value.split(',').map(|s| s.trim().to_string()).collect();
                    for v in &values {
                        if !allowed.contains(v) {
                            return Err(anyhow::anyhow!(
                                "Invalid enum value '{}'. Allowed: {:?}",
                                v,
                                allowed
                            ));
                        }
                    }
                    Ok(Value::Array(
                        values.into_iter().map(Value::String).collect(),
                    ))
                } else {
                    // Single-select: reject comma-separated values
                    if value.contains(',') {
                        return Err(anyhow::anyhow!(
                            "Field is single-select but multiple values were provided. Allowed: {:?}",
                            allowed
                        ));
                    }
                    if !allowed.contains(&value.to_string()) && !allow_custom_enum {
                        return Err(anyhow::anyhow!(
                            "Invalid enum value '{}'. Allowed: {:?}",
                            value,
                            allowed
                        ));
                    }
                    Ok(Value::String(value.to_string()))
                }
            } else {
                Ok(Value::String(value.to_string()))
            }
        }

        "task_list" => {
            let parsed: Value = serde_json::from_str(value).map_err(|_| {
                anyhow::anyhow!(
                    "Invalid task list JSON. Expected array of {{\"text\": \"...\", \"done\": true}}"
                )
            })?;
            if !parsed.is_array() {
                return Err(anyhow::anyhow!("Invalid task list JSON (expected array)"));
            }
            Ok(parsed)
        }

        "tags" => {
            let tags: Vec<Value> = value
                .split(',')
                .map(|s| Value::String(s.trim().to_string()))
                .collect();
            Ok(Value::Array(tags))
        }

        _ => Ok(Value::String(value.to_string())),
    }
}

fn merged_enum_values(
    field: &FieldDef,
    template_defaults: &TemplateDefaults,
) -> Option<Vec<String>> {
    let mut values = Vec::new();
    if let Some(ref base) = field.values {
        values.extend(base.iter().cloned());
    }
    if let Some(extra) = template_defaults.enum_values.get(&field.name) {
        for v in extra {
            if !values.contains(v) {
                values.push(v.clone());
            }
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn default_string_for_field(field_type: &str, default_value: Option<&Value>) -> Option<String> {
    let raw = match default_value? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }?;

    if field_type == "date" && raw.eq_ignore_ascii_case("today") {
        return Some(Utc::now().date_naive().format("%Y-%m-%d").to_string());
    }
    if field_type == "datetime" && raw.eq_ignore_ascii_case("now") {
        return Some(Utc::now().to_rfc3339());
    }

    Some(raw)
}

fn resolve_default_value(field_type: &str, default_value: &Value) -> anyhow::Result<Value> {
    if let Value::String(s) = default_value {
        if field_type == "date" && s.eq_ignore_ascii_case("today") {
            let date = Utc::now().date_naive().format("%Y-%m-%d").to_string();
            return Ok(Value::String(date));
        }
        if field_type == "datetime" && s.eq_ignore_ascii_case("now") {
            return Ok(Value::String(Utc::now().to_rfc3339()));
        }
        if field_type == "date" || field_type == "datetime" {
            return parse_field_value(field_type, s, &None, false, true);
        }
    }
    Ok(default_value.clone())
}

/// Capitalize first letter of a string
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Parse CLI field arguments (field=value format)
pub fn parse_cli_fields(fields: &[String]) -> anyhow::Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    for field in fields {
        if let Some((key, value)) = field.split_once('=') {
            result.insert(key.to_string(), value.to_string());
        } else {
            return Err(anyhow::anyhow!(
                "Invalid field format '{}'. Use field=value",
                field
            ));
        }
    }
    Ok(result)
}
