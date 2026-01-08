use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ledger_core::storage::{AgeSqliteStorage, NewEntryType, StorageEngine};
use uuid::Uuid;

struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        let filename = format!("{}_{}_{}.ledger", prefix, std::process::id(), nanos);
        let path = std::env::temp_dir().join(filename);
        Self { path }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[test]
fn test_create_open_close_round_trip() {
    let temp = TempFile::new("ledger_storage_round_trip");
    let passphrase = "test-passphrase-secure-123";

    let device_id = AgeSqliteStorage::create(&temp.path, passphrase)
        .expect("create should succeed");
    assert!(!device_id.is_nil());
    assert!(temp.path.exists());

    let storage = AgeSqliteStorage::open(&temp.path, passphrase)
        .expect("open should succeed");
    storage.close().expect("close should succeed");

    let on_disk = fs::read(&temp.path).expect("read should succeed");
    assert!(!on_disk.is_empty());
}

#[test]
fn test_open_wrong_passphrase_fails() {
    let temp = TempFile::new("ledger_storage_wrong_passphrase");
    let passphrase = "correct-passphrase-123";
    let wrong_passphrase = "wrong-passphrase-456";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");

    let result = AgeSqliteStorage::open(&temp.path, wrong_passphrase);
    assert!(result.is_err());
}

#[test]
fn test_open_missing_file_fails() {
    let temp = TempFile::new("ledger_storage_missing");
    let passphrase = "test-passphrase-secure-123";

    let result = AgeSqliteStorage::open(&temp.path, passphrase);
    assert!(result.is_err());
}

#[test]
fn test_metadata_persistence() {
    let temp = TempFile::new("ledger_storage_metadata");
    let passphrase = "test-passphrase-secure-123";

    let device_id = AgeSqliteStorage::create(&temp.path, passphrase)
        .expect("create should succeed");

    let storage = AgeSqliteStorage::open(&temp.path, passphrase)
        .expect("open should succeed");

    let metadata = storage.metadata().expect("metadata should succeed");
    assert_eq!(metadata.format_version, "0.1");
    assert_eq!(metadata.device_id, device_id);
    assert!(metadata.created_at <= metadata.last_modified);

    storage.close().expect("close should succeed");
}

#[test]
fn test_create_and_get_entry_type() {
    let temp = TempFile::new("ledger_entry_type_basic");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let schema = serde_json::json!({
        "fields": [
            {"name": "body", "type": "string"}
        ]
    });

    let new_type = NewEntryType::new("journal", schema.clone(), device_id);
    let type_id = storage.create_entry_type(&new_type).expect("create_entry_type should succeed");

    assert!(!type_id.is_nil());

    let retrieved = storage.get_entry_type("journal").expect("get_entry_type should succeed");
    assert!(retrieved.is_some());

    let entry_type = retrieved.unwrap();
    assert_eq!(entry_type.name, "journal");
    assert_eq!(entry_type.version, 1);
    assert_eq!(entry_type.id, type_id);
    assert_eq!(entry_type.schema_json, schema);

    storage.close().expect("close should succeed");
}

#[test]
fn test_get_nonexistent_entry_type() {
    let temp = TempFile::new("ledger_entry_type_nonexistent");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let result = storage.get_entry_type("nonexistent").expect("get_entry_type should not error");
    assert!(result.is_none());

    storage.close().expect("close should succeed");
}

#[test]
fn test_list_entry_types() {
    let temp = TempFile::new("ledger_entry_type_list");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let schema1 = serde_json::json!({"fields": [{"name": "body", "type": "string"}]});
    let schema2 = serde_json::json!({"fields": [{"name": "amount", "type": "number"}]});

    storage.create_entry_type(&NewEntryType::new("journal", schema1, device_id))
        .expect("create journal type should succeed");
    storage.create_entry_type(&NewEntryType::new("weight", schema2, device_id))
        .expect("create weight type should succeed");

    let types = storage.list_entry_types().expect("list_entry_types should succeed");
    assert_eq!(types.len(), 2);

    let names: Vec<_> = types.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"journal"));
    assert!(names.contains(&"weight"));

    storage.close().expect("close should succeed");
}

#[test]
fn test_entry_type_versioning() {
    let temp = TempFile::new("ledger_entry_type_versioning");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let schema_v1 = serde_json::json!({"fields": [{"name": "body", "type": "string"}]});
    let schema_v2 = serde_json::json!({
        "fields": [
            {"name": "body", "type": "string"},
            {"name": "tags", "type": "array"}
        ]
    });

    // Create version 1
    let v1_id = storage.create_entry_type(&NewEntryType::new("journal", schema_v1.clone(), device_id))
        .expect("create v1 should succeed");

    // Create version 2 (same name)
    let v2_id = storage.create_entry_type(&NewEntryType::new("journal", schema_v2.clone(), device_id))
        .expect("create v2 should succeed");

    assert_ne!(v1_id, v2_id);

    // get_entry_type should return latest version
    let latest = storage.get_entry_type("journal")
        .expect("get should succeed")
        .expect("journal should exist");

    assert_eq!(latest.version, 2);
    assert_eq!(latest.id, v2_id);
    assert_eq!(latest.schema_json, schema_v2);

    // list should only show latest version
    let types = storage.list_entry_types().expect("list should succeed");
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].version, 2);

    storage.close().expect("close should succeed");
}
