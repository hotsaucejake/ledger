//! Text and table output formatting for entries.

use std::collections::HashMap;

use ledger_core::storage::{AgeSqliteStorage, StorageEngine};
use uuid::Uuid;

/// Build a map of entry type ID -> name for display.
pub fn entry_type_name_map(storage: &AgeSqliteStorage) -> anyhow::Result<HashMap<Uuid, String>> {
    let types = storage.list_entry_types()?;
    let mut map = HashMap::new();
    for entry_type in types {
        map.insert(entry_type.id, entry_type.name);
    }
    Ok(map)
}
