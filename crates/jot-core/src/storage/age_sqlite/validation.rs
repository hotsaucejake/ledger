//! Tag and entry data validation.

use std::collections::HashSet;

use chrono::{DateTime, NaiveDate};

use crate::error::{JotError, Result};

/// Maximum bytes per tag.
pub const MAX_TAG_BYTES: usize = 128;

/// Maximum tags per entry.
pub const MAX_TAGS_PER_ENTRY: usize = 100;

/// Maximum bytes for entry data JSON.
pub const MAX_DATA_BYTES: usize = 1024 * 1024;

/// Normalize and validate tags.
///
/// - Trims whitespace and converts to lowercase
/// - Removes duplicates
/// - Validates character set (alphanumeric, dash, underscore, colon)
/// - Enforces length limits
pub fn normalize_tags(tags: &[String]) -> Result<Vec<String>> {
    if tags.len() > MAX_TAGS_PER_ENTRY {
        return Err(JotError::Validation(format!(
            "Too many tags (max {})",
            MAX_TAGS_PER_ENTRY
        )));
    }

    let mut seen = HashSet::with_capacity(tags.len());
    let mut normalized = Vec::with_capacity(tags.len());

    for tag in tags {
        let trimmed = tag.trim().to_ascii_lowercase();
        if trimmed.is_empty() {
            return Err(JotError::Validation("Empty tag is not allowed".to_string()));
        }
        if trimmed.len() > MAX_TAG_BYTES {
            return Err(JotError::Validation(format!(
                "Tag too long (max {} bytes)",
                MAX_TAG_BYTES
            )));
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
        {
            return Err(JotError::Validation(
                "Tag contains invalid characters".to_string(),
            ));
        }
        // Use HashSet for O(1) duplicate detection instead of Vec::contains O(n)
        if seen.insert(trimmed.clone()) {
            normalized.push(trimmed);
        }
    }

    Ok(normalized)
}

/// Validate entry data against a schema.
pub fn validate_entry_data(
    schema_json: &serde_json::Value,
    data: &serde_json::Value,
) -> Result<()> {
    let fields = schema_json
        .get("fields")
        .and_then(|value| value.as_array())
        .ok_or_else(|| JotError::Validation("Schema fields missing or invalid".to_string()))?;

    let data_obj = data
        .as_object()
        .ok_or_else(|| JotError::Validation("Entry data must be a JSON object".to_string()))?;

    let mut allowed_fields = Vec::new();

    for field in fields {
        let name = field
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| JotError::Validation("Schema field name missing".to_string()))?;
        allowed_fields.push(name.to_string());
        let field_type = field
            .get("type")
            .and_then(|value| value.as_str())
            .ok_or_else(|| JotError::Validation("Schema field type missing".to_string()))?;
        let required = field
            .get("required")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let nullable = field
            .get("nullable")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        let value = match data_obj.get(name) {
            Some(v) => v,
            None => {
                if required {
                    return Err(JotError::Validation(format!(
                        "Missing required field: {}",
                        name
                    )));
                }
                continue;
            }
        };
        if value.is_null() {
            if !nullable {
                return Err(JotError::Validation(format!(
                    "Field {} cannot be null",
                    name
                )));
            }
            continue;
        }

        match field_type {
            "string" | "text" => {
                if !value.is_string() {
                    return Err(JotError::Validation(format!(
                        "Field {} must be a string",
                        name
                    )));
                }
            }
            "number" => {
                if !value.is_number() {
                    return Err(JotError::Validation(format!(
                        "Field {} must be a number",
                        name
                    )));
                }
            }
            "integer" => {
                if value.as_i64().is_none() {
                    return Err(JotError::Validation(format!(
                        "Field {} must be an integer",
                        name
                    )));
                }
            }
            "boolean" => {
                if !value.is_boolean() {
                    return Err(JotError::Validation(format!(
                        "Field {} must be a boolean",
                        name
                    )));
                }
            }
            "date" => {
                let raw = value.as_str().ok_or_else(|| {
                    JotError::Validation(format!("Field {} must be a date string", name))
                })?;
                if NaiveDate::parse_from_str(raw, "%Y-%m-%d").is_err() {
                    return Err(JotError::Validation(format!(
                        "Field {} must be YYYY-MM-DD",
                        name
                    )));
                }
            }
            "datetime" => {
                let raw = value.as_str().ok_or_else(|| {
                    JotError::Validation(format!("Field {} must be an ISO-8601 string", name))
                })?;
                if DateTime::parse_from_rfc3339(raw).is_err() {
                    return Err(JotError::Validation(format!(
                        "Field {} must be ISO-8601",
                        name
                    )));
                }
            }
            other => {
                return Err(JotError::Validation(format!(
                    "Unsupported field type: {}",
                    other
                )))
            }
        }
    }

    for key in data_obj.keys() {
        if !allowed_fields.contains(key) {
            return Err(JotError::Validation(format!("Unknown field: {}", key)));
        }
    }

    Ok(())
}

/// Extract FTS content from entry data.
pub fn fts_content_for_entry(data: &serde_json::Value) -> String {
    data.get("body")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .unwrap_or_else(|| data.to_string())
}
