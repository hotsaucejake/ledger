//! Core data types for storage layer.
//!
//! These types represent the stable data model defined in RFC-004.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Metadata for a jot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JotMetadata {
    /// Format version (e.g., "0.1")
    pub format_version: String,

    /// Device that created this jot
    pub device_id: Uuid,

    /// When this jot was created
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp (informational)
    pub last_modified: DateTime<Utc>,
}

/// An entry type schema definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryType {
    /// Unique identifier for this entry type
    pub id: Uuid,

    /// User-facing name (e.g., "journal", "weight")
    pub name: String,

    /// Schema version number
    pub version: i32,

    /// When this entry type was created
    pub created_at: DateTime<Utc>,

    /// Device that created this entry type
    pub device_id: Uuid,

    /// Schema definition (fields, validation, etc.)
    pub schema_json: serde_json::Value,
}

/// An entry instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Unique identifier for this entry
    pub id: Uuid,

    /// Reference to entry type
    pub entry_type_id: Uuid,

    /// Schema version used for this entry
    pub schema_version: i32,

    /// Entry data (JSON)
    pub data: serde_json::Value,

    /// Tags associated with this entry
    pub tags: Vec<String>,

    /// When this entry was created
    pub created_at: DateTime<Utc>,

    /// Device that created this entry
    pub device_id: Uuid,

    /// Optional: Entry this supersedes (for revisions)
    pub supersedes: Option<Uuid>,
}

/// Builder for creating new entry types.
#[derive(Debug, Clone)]
pub struct NewEntryType {
    /// User-facing name
    pub name: String,

    /// Device ID (will be set by storage layer)
    pub device_id: Uuid,

    /// Schema definition
    pub schema_json: serde_json::Value,
}

impl NewEntryType {
    pub fn new(name: impl Into<String>, schema_json: serde_json::Value, device_id: Uuid) -> Self {
        Self {
            name: name.into(),
            device_id,
            schema_json,
        }
    }
}

/// Builder for creating new entries.
#[derive(Debug, Clone)]
pub struct NewEntry {
    /// Entry type reference
    pub entry_type_id: Uuid,

    /// Schema version to use
    pub schema_version: i32,

    /// Entry data
    pub data: serde_json::Value,

    /// Tags
    pub tags: Vec<String>,

    /// Device ID (will be set by storage layer)
    pub device_id: Uuid,

    /// Optional: Entry this supersedes
    pub supersedes: Option<Uuid>,

    /// Optional: Override created_at timestamp
    pub created_at: Option<DateTime<Utc>>,
}

impl NewEntry {
    pub fn new(
        entry_type_id: Uuid,
        schema_version: i32,
        data: serde_json::Value,
        device_id: Uuid,
    ) -> Self {
        Self {
            entry_type_id,
            schema_version,
            data,
            tags: Vec::new(),
            device_id,
            supersedes: None,
            created_at: None,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_supersedes(mut self, supersedes: Uuid) -> Self {
        self.supersedes = Some(supersedes);
        self
    }

    pub fn with_created_at(mut self, created_at: DateTime<Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }
}

/// A composition - semantic grouping of entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composition {
    /// Unique identifier for this composition
    pub id: Uuid,

    /// User-facing name (unique within jot)
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// When this composition was created
    pub created_at: DateTime<Utc>,

    /// Device that created this composition
    pub device_id: Uuid,

    /// Optional structured metadata
    pub metadata: Option<serde_json::Value>,
}

/// Builder for creating new compositions.
#[derive(Debug, Clone)]
pub struct NewComposition {
    /// User-facing name
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Device ID
    pub device_id: Uuid,

    /// Optional structured metadata
    pub metadata: Option<serde_json::Value>,
}

impl NewComposition {
    pub fn new(name: impl Into<String>, device_id: Uuid) -> Self {
        Self {
            name: name.into(),
            description: None,
            device_id,
            metadata: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// A template - reusable defaults for entry creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Unique identifier for this template
    pub id: Uuid,

    /// User-facing name (unique within jot)
    pub name: String,

    /// Entry type this template applies to
    pub entry_type_id: Uuid,

    /// Template version number
    pub version: i32,

    /// When this template version was created
    pub created_at: DateTime<Utc>,

    /// Device that created this template
    pub device_id: Uuid,

    /// Optional description
    pub description: Option<String>,

    /// Template data (defaults, default_tags, default_compositions, prompt_overrides)
    pub template_json: serde_json::Value,
}

/// Builder for creating new templates.
#[derive(Debug, Clone)]
pub struct NewTemplate {
    /// User-facing name
    pub name: String,

    /// Entry type this template applies to
    pub entry_type_id: Uuid,

    /// Device ID
    pub device_id: Uuid,

    /// Optional description
    pub description: Option<String>,

    /// Template data (defaults, default_tags, default_compositions, prompt_overrides)
    pub template_json: serde_json::Value,
}

impl NewTemplate {
    pub fn new(
        name: impl Into<String>,
        entry_type_id: Uuid,
        template_json: serde_json::Value,
        device_id: Uuid,
    ) -> Self {
        Self {
            name: name.into(),
            entry_type_id,
            device_id,
            description: None,
            template_json,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// An entry-composition association.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryComposition {
    /// Entry ID
    pub entry_id: Uuid,

    /// Composition ID
    pub composition_id: Uuid,

    /// When the entry was added to the composition
    pub added_at: DateTime<Utc>,
}

/// Filter for querying entries.
#[derive(Debug, Clone, Default)]
pub struct EntryFilter {
    /// Filter by entry type ID
    pub entry_type_id: Option<Uuid>,

    /// Filter by tag
    pub tag: Option<String>,

    /// Start date (inclusive)
    pub since: Option<DateTime<Utc>>,

    /// End date (inclusive)
    pub until: Option<DateTime<Utc>>,

    /// Maximum number of results
    pub limit: Option<usize>,

    /// Filter by composition ID
    pub composition_id: Option<Uuid>,
}

impl EntryFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entry_type(mut self, id: Uuid) -> Self {
        self.entry_type_id = Some(id);
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn since(mut self, date: DateTime<Utc>) -> Self {
        self.since = Some(date);
        self
    }

    pub fn until(mut self, date: DateTime<Utc>) -> Self {
        self.until = Some(date);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn composition(mut self, id: Uuid) -> Self {
        self.composition_id = Some(id);
        self
    }
}

/// Filter for querying compositions.
#[derive(Debug, Clone, Default)]
pub struct CompositionFilter {
    /// Maximum number of results
    pub limit: Option<usize>,
}

impl CompositionFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_entry_builder() {
        let device_id = Uuid::new_v4();
        let type_id = Uuid::new_v4();
        let data = serde_json::json!({"body": "test"});

        let entry = NewEntry::new(type_id, 1, data.clone(), device_id)
            .with_tags(vec!["test".to_string()])
            .with_supersedes(Uuid::new_v4());

        assert_eq!(entry.entry_type_id, type_id);
        assert_eq!(entry.schema_version, 1);
        assert_eq!(entry.data, data);
        assert_eq!(entry.tags.len(), 1);
        assert!(entry.supersedes.is_some());
    }

    #[test]
    fn test_entry_filter_builder() {
        let type_id = Uuid::new_v4();
        let comp_id = Uuid::new_v4();
        let now = Utc::now();

        let filter = EntryFilter::new()
            .entry_type(type_id)
            .tag("test")
            .since(now)
            .limit(10)
            .composition(comp_id);

        assert_eq!(filter.entry_type_id, Some(type_id));
        assert_eq!(filter.tag, Some("test".to_string()));
        assert_eq!(filter.since, Some(now));
        assert_eq!(filter.limit, Some(10));
        assert_eq!(filter.composition_id, Some(comp_id));
    }

    #[test]
    fn test_new_composition_builder() {
        let device_id = Uuid::new_v4();
        let metadata = serde_json::json!({"key": "value"});

        let comp = NewComposition::new("project_x", device_id)
            .with_description("My project")
            .with_metadata(metadata.clone());

        assert_eq!(comp.name, "project_x");
        assert_eq!(comp.description, Some("My project".to_string()));
        assert_eq!(comp.device_id, device_id);
        assert_eq!(comp.metadata, Some(metadata));
    }

    #[test]
    fn test_new_template_builder() {
        let device_id = Uuid::new_v4();
        let type_id = Uuid::new_v4();
        let template_json = serde_json::json!({
            "defaults": {"car": "civic"},
            "default_tags": ["car", "fuel"]
        });

        let template = NewTemplate::new("gas_fillup", type_id, template_json.clone(), device_id)
            .with_description("Template for gas fillups");

        assert_eq!(template.name, "gas_fillup");
        assert_eq!(template.entry_type_id, type_id);
        assert_eq!(template.device_id, device_id);
        assert_eq!(
            template.description,
            Some("Template for gas fillups".to_string())
        );
        assert_eq!(template.template_json, template_json);
    }

    #[test]
    fn test_composition_filter_builder() {
        let filter = CompositionFilter::new().limit(10);

        assert_eq!(filter.limit, Some(10));
    }
}
