use std::fs;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ledger_core::storage::encryption::decrypt;
use ledger_core::storage::{
    AgeSqliteStorage, CompositionFilter, EntryFilter, NewComposition, NewEntry, NewEntryType,
    NewTemplate, StorageEngine,
};
use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName};
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

fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be available")
        .as_nanos();
    let dirname = format!("{}_{}_{}", prefix, std::process::id(), nanos);
    let path = std::env::temp_dir().join(dirname);
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn open_sqlite_from_file(path: &PathBuf, passphrase: &str) -> Connection {
    let encrypted = fs::read(path).expect("read should succeed");
    let plaintext = decrypt(&encrypted, passphrase).expect("decrypt should succeed");

    let size: i32 = plaintext
        .len()
        .try_into()
        .expect("payload length should fit in sqlite3_malloc");
    let raw = unsafe { rusqlite::ffi::sqlite3_malloc(size) as *mut u8 };
    if raw.is_null() {
        panic!("sqlite3_malloc returned null");
    }

    let owned = unsafe {
        std::ptr::copy_nonoverlapping(plaintext.as_ptr(), raw, plaintext.len());
        let ptr = NonNull::new(raw).expect("nonnull");
        OwnedData::from_raw_nonnull(ptr, plaintext.len())
    };

    let mut conn = Connection::open_in_memory().expect("open_in_memory should succeed");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("pragma should succeed");
    conn.deserialize(DatabaseName::Main, owned, false)
        .expect("deserialize should succeed");
    conn
}

fn assert_no_temp_files(path: &Path) {
    let parent = path.parent().expect("parent directory");
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("valid filename");
    let entries = fs::read_dir(parent).expect("read dir");
    for entry in entries {
        let entry = entry.expect("dir entry");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&format!("{}.", filename)) && name.ends_with(".tmp") {
            panic!("Found unexpected temp file: {}", name);
        }
    }
}

fn create_basic_entry_type(storage: &mut AgeSqliteStorage) -> Uuid {
    let device_id = Uuid::new_v4();
    let schema = serde_json::json!({
        "fields": [
            {"name": "body", "type": "string", "required": true}
        ]
    });
    storage
        .create_entry_type(&NewEntryType::new("journal", schema, device_id))
        .expect("create entry type should succeed")
}

#[test]
fn test_create_open_close_round_trip() {
    let temp = TempFile::new("ledger_storage_round_trip");
    let passphrase = "test-passphrase-secure-123";

    let device_id =
        AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    assert!(!device_id.is_nil());
    assert!(temp.path.exists());

    let storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");
    storage.close(passphrase).expect("close should succeed");

    let on_disk = fs::read(&temp.path).expect("read should succeed");
    assert!(!on_disk.is_empty());
    assert_no_temp_files(&temp.path);
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

    let device_id =
        AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");

    let storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let metadata = storage.metadata().expect("metadata should succeed");
    assert_eq!(metadata.format_version, "0.1");
    assert_eq!(metadata.device_id, device_id);
    assert!(metadata.created_at <= metadata.last_modified);

    storage.close(passphrase).expect("close should succeed");
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
    let type_id = storage
        .create_entry_type(&new_type)
        .expect("create_entry_type should succeed");

    assert!(!type_id.is_nil());

    let retrieved = storage
        .get_entry_type("journal")
        .expect("get_entry_type should succeed");
    assert!(retrieved.is_some());

    let entry_type = retrieved.unwrap();
    assert_eq!(entry_type.name, "journal");
    assert_eq!(entry_type.version, 1);
    assert_eq!(entry_type.id, type_id);
    assert_eq!(entry_type.schema_json, schema);

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_get_nonexistent_entry_type() {
    let temp = TempFile::new("ledger_entry_type_nonexistent");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let result = storage
        .get_entry_type("nonexistent")
        .expect("get_entry_type should not error");
    assert!(result.is_none());

    storage.close(passphrase).expect("close should succeed");
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

    storage
        .create_entry_type(&NewEntryType::new("journal", schema1, device_id))
        .expect("create journal type should succeed");
    storage
        .create_entry_type(&NewEntryType::new("weight", schema2, device_id))
        .expect("create weight type should succeed");

    let types = storage
        .list_entry_types()
        .expect("list_entry_types should succeed");
    assert_eq!(types.len(), 2);

    let names: Vec<_> = types.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"journal"));
    assert!(names.contains(&"weight"));

    storage.close(passphrase).expect("close should succeed");
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
    let base_id = storage
        .create_entry_type(&NewEntryType::new("journal", schema_v1.clone(), device_id))
        .expect("create v1 should succeed");

    // Create version 2 (same name)
    let base_id_second = storage
        .create_entry_type(&NewEntryType::new("journal", schema_v2.clone(), device_id))
        .expect("create v2 should succeed");

    assert_eq!(base_id, base_id_second);

    // get_entry_type should return latest version
    let latest = storage
        .get_entry_type("journal")
        .expect("get should succeed")
        .expect("journal should exist");

    assert_eq!(latest.version, 2);
    assert_eq!(latest.id, base_id);
    assert_eq!(latest.schema_json, schema_v2);

    // list should only show latest version
    let types = storage.list_entry_types().expect("list should succeed");
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].version, 2);

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_insert_and_get_entry_round_trip() {
    let temp = TempFile::new("ledger_entry_round_trip");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let data = serde_json::json!({"body": "Hello World"});
    let new_entry = NewEntry::new(entry_type_id, 1, data.clone(), device_id).with_tags(vec![
        "Tag-One".to_string(),
        "tag-one".to_string(),
        "Second".to_string(),
    ]);

    let entry_id = storage
        .insert_entry(&new_entry)
        .expect("insert should succeed");
    let entry = storage
        .get_entry(&entry_id)
        .expect("get should succeed")
        .expect("entry should exist");

    assert_eq!(entry.entry_type_id, entry_type_id);
    assert_eq!(entry.schema_version, 1);
    assert_eq!(entry.data, data);
    assert_eq!(
        entry.tags,
        vec!["tag-one".to_string(), "second".to_string()]
    );
}

#[test]
fn test_insert_entry_missing_required_field_fails() {
    let temp = TempFile::new("ledger_entry_missing_required");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let new_entry = NewEntry::new(entry_type_id, 1, serde_json::json!({}), device_id);

    let result = storage.insert_entry(&new_entry);
    assert!(result.is_err());
}

#[test]
fn test_insert_entry_type_mismatch_fails() {
    let temp = TempFile::new("ledger_entry_type_mismatch");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let new_entry = NewEntry::new(entry_type_id, 1, serde_json::json!({"body": 42}), device_id);

    let result = storage.insert_entry(&new_entry);
    assert!(result.is_err());
}

#[test]
fn test_insert_entry_unknown_field_fails() {
    let temp = TempFile::new("ledger_entry_unknown_field");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let new_entry = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"body": "ok", "extra": "nope"}),
        device_id,
    );

    let result = storage.insert_entry(&new_entry);
    assert!(result.is_err());
}

#[test]
fn test_list_entries_with_filters() {
    let temp = TempFile::new("ledger_entry_list");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    let first = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"body": "first entry"}),
        device_id,
    )
    .with_tags(vec!["alpha".to_string()]);
    let second = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"body": "second entry"}),
        device_id,
    )
    .with_tags(vec!["beta".to_string()]);

    let first_id = storage
        .insert_entry(&first)
        .expect("insert first should succeed");
    std::thread::sleep(Duration::from_millis(2));
    let second_id = storage
        .insert_entry(&second)
        .expect("insert second should succeed");

    let all = storage
        .list_entries(&EntryFilter::new())
        .expect("list should succeed");
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].id, second_id);
    assert_eq!(all[1].id, first_id);

    let filtered = storage
        .list_entries(&EntryFilter::new().tag("Alpha"))
        .expect("list should succeed");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, first_id);
}

#[test]
fn test_insert_entry_invalid_tag_characters() {
    let temp = TempFile::new("ledger_entry_invalid_tag");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let new_entry = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"body": "ok"}),
        device_id,
    )
    .with_tags(vec!["bad tag!".to_string()]);

    let result = storage.insert_entry(&new_entry);
    assert!(result.is_err());
}

#[test]
fn test_insert_entry_empty_tag_fails() {
    let temp = TempFile::new("ledger_entry_empty_tag");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let new_entry = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"body": "ok"}),
        device_id,
    )
    .with_tags(vec!["   ".to_string()]);

    let result = storage.insert_entry(&new_entry);
    assert!(result.is_err());
}

#[test]
fn test_insert_entry_tag_too_long_fails() {
    let temp = TempFile::new("ledger_entry_long_tag");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let long_tag = "a".repeat(129);
    let new_entry = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"body": "ok"}),
        device_id,
    )
    .with_tags(vec![long_tag]);

    let result = storage.insert_entry(&new_entry);
    assert!(result.is_err());
}

#[test]
fn test_insert_entry_too_many_tags_fails() {
    let temp = TempFile::new("ledger_entry_too_many_tags");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let tags = (0..101).map(|i| format!("tag{}", i)).collect::<Vec<_>>();
    let new_entry = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"body": "ok"}),
        device_id,
    )
    .with_tags(tags);

    let result = storage.insert_entry(&new_entry);
    assert!(result.is_err());
}

#[test]
fn test_insert_entry_invalid_date_fails() {
    let temp = TempFile::new("ledger_entry_bad_date");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let schema = serde_json::json!({
        "fields": [
            {"name": "when", "type": "date", "required": true}
        ]
    });
    let entry_type_id = storage
        .create_entry_type(&NewEntryType::new("dated", schema, device_id))
        .expect("create entry type should succeed");

    let entry = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"when": "2024-13-40"}),
        device_id,
    );
    let result = storage.insert_entry(&entry);
    assert!(result.is_err());
}

#[test]
fn test_insert_entry_invalid_datetime_fails() {
    let temp = TempFile::new("ledger_entry_bad_datetime");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let schema = serde_json::json!({
        "fields": [
            {"name": "when", "type": "datetime", "required": true}
        ]
    });
    let entry_type_id = storage
        .create_entry_type(&NewEntryType::new("timestamped", schema, device_id))
        .expect("create entry type should succeed");

    let entry = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"when": "not-a-date"}),
        device_id,
    );
    let result = storage.insert_entry(&entry);
    assert!(result.is_err());
}

#[test]
fn test_search_entries_basic() {
    let temp = TempFile::new("ledger_entry_search");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let entry = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"body": "searchable content"}),
        device_id,
    );
    let entry_id = storage.insert_entry(&entry).expect("insert should succeed");

    let results = storage
        .search_entries("searchable")
        .expect("search should succeed");
    assert!(!results.is_empty());
    assert!(results.iter().any(|item| item.id == entry_id));
}

#[test]
fn test_check_integrity_ok() {
    let temp = TempFile::new("ledger_integrity_ok");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let entry = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"body": "integrity check"}),
        device_id,
    );
    storage.insert_entry(&entry).expect("insert should succeed");

    storage.check_integrity().expect("integrity should succeed");
}

#[test]
fn test_check_integrity_fails_on_orphaned_fts() {
    let temp = TempFile::new("ledger_integrity_fail_fts");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let entry = NewEntry::new(
        entry_type_id,
        1,
        serde_json::json!({"body": "integrity test"}),
        device_id,
    );
    let entry_id = storage.insert_entry(&entry).expect("insert should succeed");
    storage.close(passphrase).expect("close should succeed");

    let conn = open_sqlite_from_file(&temp.path, passphrase);
    conn.execute(
        "DELETE FROM entries_fts WHERE entry_id = ?",
        [entry_id.to_string()],
    )
    .expect("delete fts should succeed");

    let data = conn
        .serialize(DatabaseName::Main)
        .expect("serialize should succeed");
    let encrypted = ledger_core::storage::encryption::encrypt(data.as_ref(), passphrase)
        .expect("encrypt should succeed");
    fs::write(&temp.path, encrypted).expect("write should succeed");

    let storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");
    let result = storage.check_integrity();
    assert!(result.is_err());
}

#[cfg(unix)]
#[test]
fn test_atomic_write_failure_leaves_no_temp_files() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("ledger_atomic_failure");
    let ledger_path = dir.join("test.ledger");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&ledger_path, passphrase).expect("create should succeed");
    let storage = AgeSqliteStorage::open(&ledger_path, passphrase).expect("open should succeed");

    let mut perms = fs::metadata(&dir).expect("metadata").permissions();
    perms.set_mode(0o500);
    fs::set_permissions(&dir, perms).expect("set permissions");

    let result = storage.close(passphrase);
    assert!(result.is_err());

    let mut perms = fs::metadata(&dir).expect("metadata").permissions();
    perms.set_mode(0o700);
    fs::set_permissions(&dir, perms).expect("restore permissions");

    assert_no_temp_files(&ledger_path);
    let _ = fs::remove_file(&ledger_path);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_entry_type_active_flag_unique() {
    let temp = TempFile::new("ledger_entry_type_active");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let schema_v1 =
        serde_json::json!({"fields": [{"name": "body", "type": "string", "required": true}]});
    let schema_v2 =
        serde_json::json!({"fields": [{"name": "body", "type": "string", "required": true}]});

    storage
        .create_entry_type(&NewEntryType::new("journal", schema_v1, device_id))
        .expect("create v1 should succeed");
    storage
        .create_entry_type(&NewEntryType::new("journal", schema_v2, device_id))
        .expect("create v2 should succeed");

    storage.close(passphrase).expect("close should succeed");

    let conn = open_sqlite_from_file(&temp.path, passphrase);
    let entry_type_id: String = conn
        .query_row(
            "SELECT id FROM entry_types WHERE name = 'journal'",
            [],
            |row| row.get(0),
        )
        .expect("entry type should exist");

    let active_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM entry_type_versions WHERE entry_type_id = ? AND active = 1",
            [entry_type_id],
            |row| row.get(0),
        )
        .expect("count should succeed");

    assert_eq!(active_count, 1);
}

#[test]
fn test_last_modified_updates_on_entry_type_create() {
    let temp = TempFile::new("ledger_last_modified");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let before = storage
        .metadata()
        .expect("metadata should succeed")
        .last_modified;

    std::thread::sleep(Duration::from_millis(2));

    let device_id = Uuid::new_v4();
    let schema =
        serde_json::json!({"fields": [{"name": "body", "type": "string", "required": true}]});
    storage
        .create_entry_type(&NewEntryType::new("journal", schema, device_id))
        .expect("create should succeed");

    let after = storage
        .metadata()
        .expect("metadata should succeed")
        .last_modified;

    assert!(after >= before);
    assert!(after > before);

    storage.close(passphrase).expect("close should succeed");
}

// ============================================================================
// Composition Tests
// ============================================================================

#[test]
fn test_create_and_get_composition() {
    let temp = TempFile::new("ledger_composition_basic");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let new_comp = NewComposition::new("project_x", device_id)
        .with_description("My research project")
        .with_metadata(serde_json::json!({"priority": "high"}));

    let comp_id = storage
        .create_composition(&new_comp)
        .expect("create_composition should succeed");
    assert!(!comp_id.is_nil());

    let retrieved = storage
        .get_composition("project_x")
        .expect("get_composition should succeed");
    assert!(retrieved.is_some());

    let comp = retrieved.unwrap();
    assert_eq!(comp.name, "project_x");
    assert_eq!(comp.description, Some("My research project".to_string()));
    assert_eq!(comp.id, comp_id);
    assert_eq!(comp.metadata, Some(serde_json::json!({"priority": "high"})));

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_get_composition_by_id() {
    let temp = TempFile::new("ledger_composition_by_id");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let comp_id = storage
        .create_composition(&NewComposition::new("test_comp", device_id))
        .expect("create should succeed");

    let by_id = storage
        .get_composition_by_id(&comp_id)
        .expect("get_by_id should succeed");
    assert!(by_id.is_some());
    assert_eq!(by_id.unwrap().name, "test_comp");

    let nonexistent = storage
        .get_composition_by_id(&Uuid::new_v4())
        .expect("get_by_id should succeed");
    assert!(nonexistent.is_none());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_composition_duplicate_name_fails() {
    let temp = TempFile::new("ledger_composition_dup");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    storage
        .create_composition(&NewComposition::new("project_x", device_id))
        .expect("first create should succeed");

    let result = storage.create_composition(&NewComposition::new("project_x", device_id));
    assert!(result.is_err());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_list_compositions() {
    let temp = TempFile::new("ledger_composition_list");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    storage
        .create_composition(&NewComposition::new("alpha", device_id))
        .expect("create alpha should succeed");
    storage
        .create_composition(&NewComposition::new("beta", device_id))
        .expect("create beta should succeed");
    storage
        .create_composition(&NewComposition::new("gamma", device_id))
        .expect("create gamma should succeed");

    let all = storage
        .list_compositions(&CompositionFilter::new())
        .expect("list should succeed");
    assert_eq!(all.len(), 3);

    // Should be ordered by name
    assert_eq!(all[0].name, "alpha");
    assert_eq!(all[1].name, "beta");
    assert_eq!(all[2].name, "gamma");

    // Test limit
    let limited = storage
        .list_compositions(&CompositionFilter::new().limit(2))
        .expect("list should succeed");
    assert_eq!(limited.len(), 2);

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_rename_composition() {
    let temp = TempFile::new("ledger_composition_rename");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let comp_id = storage
        .create_composition(&NewComposition::new("old_name", device_id))
        .expect("create should succeed");

    storage
        .rename_composition(&comp_id, "new_name")
        .expect("rename should succeed");

    let old = storage
        .get_composition("old_name")
        .expect("get old should succeed");
    assert!(old.is_none());

    let new = storage
        .get_composition("new_name")
        .expect("get new should succeed");
    assert!(new.is_some());
    assert_eq!(new.unwrap().id, comp_id);

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_rename_composition_to_existing_fails() {
    let temp = TempFile::new("ledger_composition_rename_dup");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let comp_id = storage
        .create_composition(&NewComposition::new("first", device_id))
        .expect("create first should succeed");
    storage
        .create_composition(&NewComposition::new("second", device_id))
        .expect("create second should succeed");

    let result = storage.rename_composition(&comp_id, "second");
    assert!(result.is_err());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_delete_composition() {
    let temp = TempFile::new("ledger_composition_delete");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let comp_id = storage
        .create_composition(&NewComposition::new("to_delete", device_id))
        .expect("create should succeed");

    storage
        .delete_composition(&comp_id)
        .expect("delete should succeed");

    let deleted = storage
        .get_composition("to_delete")
        .expect("get should succeed");
    assert!(deleted.is_none());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_attach_detach_entry_to_composition() {
    let temp = TempFile::new("ledger_composition_attach");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let entry_type_id = create_basic_entry_type(&mut storage);
    let comp_id = storage
        .create_composition(&NewComposition::new("project", device_id))
        .expect("create comp should succeed");

    let entry_id = storage
        .insert_entry(&NewEntry::new(
            entry_type_id,
            1,
            serde_json::json!({"body": "test"}),
            device_id,
        ))
        .expect("insert entry should succeed");

    // Attach entry to composition
    storage
        .attach_entry_to_composition(&entry_id, &comp_id)
        .expect("attach should succeed");

    // Verify entry is in composition
    let comps = storage
        .get_entry_compositions(&entry_id)
        .expect("get entry comps should succeed");
    assert_eq!(comps.len(), 1);
    assert_eq!(comps[0].id, comp_id);

    // Verify composition has entry
    let entries = storage
        .get_composition_entries(&comp_id)
        .expect("get comp entries should succeed");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].entry_id, entry_id);

    // Detach entry
    storage
        .detach_entry_from_composition(&entry_id, &comp_id)
        .expect("detach should succeed");

    let comps_after = storage
        .get_entry_compositions(&entry_id)
        .expect("get entry comps should succeed");
    assert!(comps_after.is_empty());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_attach_idempotent() {
    let temp = TempFile::new("ledger_composition_attach_idem");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let entry_type_id = create_basic_entry_type(&mut storage);
    let comp_id = storage
        .create_composition(&NewComposition::new("project", device_id))
        .expect("create comp should succeed");

    let entry_id = storage
        .insert_entry(&NewEntry::new(
            entry_type_id,
            1,
            serde_json::json!({"body": "test"}),
            device_id,
        ))
        .expect("insert entry should succeed");

    // Attach twice should be idempotent
    storage
        .attach_entry_to_composition(&entry_id, &comp_id)
        .expect("first attach should succeed");
    storage
        .attach_entry_to_composition(&entry_id, &comp_id)
        .expect("second attach should succeed");

    let comps = storage
        .get_entry_compositions(&entry_id)
        .expect("get entry comps should succeed");
    assert_eq!(comps.len(), 1);

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_delete_composition_removes_associations() {
    let temp = TempFile::new("ledger_composition_delete_assoc");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let entry_type_id = create_basic_entry_type(&mut storage);
    let comp_id = storage
        .create_composition(&NewComposition::new("project", device_id))
        .expect("create comp should succeed");

    let entry_id = storage
        .insert_entry(&NewEntry::new(
            entry_type_id,
            1,
            serde_json::json!({"body": "test"}),
            device_id,
        ))
        .expect("insert entry should succeed");

    storage
        .attach_entry_to_composition(&entry_id, &comp_id)
        .expect("attach should succeed");

    // Delete composition
    storage
        .delete_composition(&comp_id)
        .expect("delete should succeed");

    // Entry should still exist
    let entry = storage
        .get_entry(&entry_id)
        .expect("get entry should succeed");
    assert!(entry.is_some());

    // But have no compositions
    let comps = storage
        .get_entry_compositions(&entry_id)
        .expect("get entry comps should succeed");
    assert!(comps.is_empty());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_list_entries_with_composition_filter() {
    let temp = TempFile::new("ledger_entries_comp_filter");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let entry_type_id = create_basic_entry_type(&mut storage);
    let comp_id = storage
        .create_composition(&NewComposition::new("project", device_id))
        .expect("create comp should succeed");

    // Create two entries, attach only one to composition
    let entry1_id = storage
        .insert_entry(&NewEntry::new(
            entry_type_id,
            1,
            serde_json::json!({"body": "in project"}),
            device_id,
        ))
        .expect("insert should succeed");

    let _entry2_id = storage
        .insert_entry(&NewEntry::new(
            entry_type_id,
            1,
            serde_json::json!({"body": "not in project"}),
            device_id,
        ))
        .expect("insert should succeed");

    storage
        .attach_entry_to_composition(&entry1_id, &comp_id)
        .expect("attach should succeed");

    // List all entries
    let all = storage
        .list_entries(&EntryFilter::new())
        .expect("list should succeed");
    assert_eq!(all.len(), 2);

    // List only entries in composition
    let filtered = storage
        .list_entries(&EntryFilter::new().composition(comp_id))
        .expect("list should succeed");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, entry1_id);

    storage.close(passphrase).expect("close should succeed");
}

// ============================================================================
// Template Tests
// ============================================================================

#[test]
fn test_create_and_get_template() {
    let temp = TempFile::new("ledger_template_basic");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let template_json = serde_json::json!({
        "defaults": {"body": "Default text"},
        "default_tags": ["daily"]
    });

    let new_template = NewTemplate::new(
        "daily_journal",
        entry_type_id,
        template_json.clone(),
        device_id,
    )
    .with_description("Template for daily journal entries");

    let template_id = storage
        .create_template(&new_template)
        .expect("create_template should succeed");
    assert!(!template_id.is_nil());

    let retrieved = storage
        .get_template("daily_journal")
        .expect("get_template should succeed");
    assert!(retrieved.is_some());

    let template = retrieved.unwrap();
    assert_eq!(template.name, "daily_journal");
    assert_eq!(template.entry_type_id, entry_type_id);
    assert_eq!(template.version, 1);
    assert_eq!(
        template.description,
        Some("Template for daily journal entries".to_string())
    );
    assert_eq!(template.template_json, template_json);

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_get_template_by_id() {
    let temp = TempFile::new("ledger_template_by_id");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();
    let template_id = storage
        .create_template(&NewTemplate::new(
            "test_template",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create should succeed");

    let by_id = storage
        .get_template_by_id(&template_id)
        .expect("get_by_id should succeed");
    assert!(by_id.is_some());
    assert_eq!(by_id.unwrap().name, "test_template");

    let nonexistent = storage
        .get_template_by_id(&Uuid::new_v4())
        .expect("get_by_id should succeed");
    assert!(nonexistent.is_none());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_template_duplicate_name_fails() {
    let temp = TempFile::new("ledger_template_dup");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    storage
        .create_template(&NewTemplate::new(
            "my_template",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("first create should succeed");

    let result = storage.create_template(&NewTemplate::new(
        "my_template",
        entry_type_id,
        serde_json::json!({}),
        device_id,
    ));
    assert!(result.is_err());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_template_invalid_entry_type_fails() {
    let temp = TempFile::new("ledger_template_invalid_type");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let fake_type_id = Uuid::new_v4();

    let result = storage.create_template(&NewTemplate::new(
        "bad_template",
        fake_type_id,
        serde_json::json!({}),
        device_id,
    ));
    assert!(result.is_err());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_list_templates() {
    let temp = TempFile::new("ledger_template_list");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    storage
        .create_template(&NewTemplate::new(
            "alpha",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create alpha should succeed");
    storage
        .create_template(&NewTemplate::new(
            "beta",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create beta should succeed");

    let templates = storage.list_templates().expect("list should succeed");
    assert_eq!(templates.len(), 2);

    let names: Vec<_> = templates.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_update_template_creates_new_version() {
    let temp = TempFile::new("ledger_template_update");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    let template_id = storage
        .create_template(&NewTemplate::new(
            "evolving",
            entry_type_id,
            serde_json::json!({"defaults": {"body": "v1"}}),
            device_id,
        ))
        .expect("create should succeed");

    let v1 = storage
        .get_template("evolving")
        .expect("get should succeed")
        .unwrap();
    assert_eq!(v1.version, 1);
    assert_eq!(v1.template_json["defaults"]["body"], "v1");

    // Update template
    let new_version = storage
        .update_template(
            &template_id,
            serde_json::json!({"defaults": {"body": "v2"}}),
        )
        .expect("update should succeed");
    assert_eq!(new_version, 2);

    let v2 = storage
        .get_template("evolving")
        .expect("get should succeed")
        .unwrap();
    assert_eq!(v2.version, 2);
    assert_eq!(v2.template_json["defaults"]["body"], "v2");

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_delete_template() {
    let temp = TempFile::new("ledger_template_delete");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    let template_id = storage
        .create_template(&NewTemplate::new(
            "to_delete",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create should succeed");

    storage
        .delete_template(&template_id)
        .expect("delete should succeed");

    let deleted = storage
        .get_template("to_delete")
        .expect("get should succeed");
    assert!(deleted.is_none());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_set_and_get_default_template() {
    let temp = TempFile::new("ledger_template_default");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    let template_id = storage
        .create_template(&NewTemplate::new(
            "default_tmpl",
            entry_type_id,
            serde_json::json!({"defaults": {"body": "default body"}}),
            device_id,
        ))
        .expect("create should succeed");

    // No default initially
    let no_default = storage
        .get_default_template(&entry_type_id)
        .expect("get default should succeed");
    assert!(no_default.is_none());

    // Set default
    storage
        .set_default_template(&entry_type_id, &template_id)
        .expect("set default should succeed");

    let default = storage
        .get_default_template(&entry_type_id)
        .expect("get default should succeed");
    assert!(default.is_some());
    assert_eq!(default.unwrap().id, template_id);

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_set_default_template_replaces_existing() {
    let temp = TempFile::new("ledger_template_default_replace");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    let template1_id = storage
        .create_template(&NewTemplate::new(
            "first",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create should succeed");
    let template2_id = storage
        .create_template(&NewTemplate::new(
            "second",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create should succeed");

    storage
        .set_default_template(&entry_type_id, &template1_id)
        .expect("set first should succeed");
    storage
        .set_default_template(&entry_type_id, &template2_id)
        .expect("set second should succeed");

    let default = storage
        .get_default_template(&entry_type_id)
        .expect("get default should succeed");
    assert!(default.is_some());
    assert_eq!(default.unwrap().id, template2_id);

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_set_default_template_wrong_entry_type_fails() {
    let temp = TempFile::new("ledger_template_default_wrong_type");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let journal_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    // Create another entry type
    let weight_type_id = storage
        .create_entry_type(&NewEntryType::new(
            "weight",
            serde_json::json!({"fields": [{"name": "kg", "type": "number", "required": true}]}),
            device_id,
        ))
        .expect("create weight type should succeed");

    // Create template for journal
    let template_id = storage
        .create_template(&NewTemplate::new(
            "journal_tmpl",
            journal_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create should succeed");

    // Try to set it as default for weight (should fail)
    let result = storage.set_default_template(&weight_type_id, &template_id);
    assert!(result.is_err());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_clear_default_template() {
    let temp = TempFile::new("ledger_template_clear_default");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    let template_id = storage
        .create_template(&NewTemplate::new(
            "default_tmpl",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create should succeed");

    storage
        .set_default_template(&entry_type_id, &template_id)
        .expect("set should succeed");

    storage
        .clear_default_template(&entry_type_id)
        .expect("clear should succeed");

    let default = storage
        .get_default_template(&entry_type_id)
        .expect("get default should succeed");
    assert!(default.is_none());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_delete_template_removes_default_mapping() {
    let temp = TempFile::new("ledger_template_delete_default");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    let template_id = storage
        .create_template(&NewTemplate::new(
            "default_tmpl",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create should succeed");

    storage
        .set_default_template(&entry_type_id, &template_id)
        .expect("set should succeed");

    storage
        .delete_template(&template_id)
        .expect("delete should succeed");

    let default = storage
        .get_default_template(&entry_type_id)
        .expect("get default should succeed");
    assert!(default.is_none());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_template_persistence() {
    let temp = TempFile::new("ledger_template_persist");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");

    {
        let mut storage =
            AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

        let entry_type_id = create_basic_entry_type(&mut storage);
        let device_id = Uuid::new_v4();

        let template_id = storage
            .create_template(&NewTemplate::new(
                "persistent",
                entry_type_id,
                serde_json::json!({"defaults": {"body": "hello"}}),
                device_id,
            ))
            .expect("create should succeed");

        storage
            .set_default_template(&entry_type_id, &template_id)
            .expect("set default should succeed");

        storage.close(passphrase).expect("close should succeed");
    }

    // Reopen and verify persistence
    let storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("reopen should succeed");

    let template = storage
        .get_template("persistent")
        .expect("get should succeed");
    assert!(template.is_some());
    assert_eq!(template.unwrap().template_json["defaults"]["body"], "hello");

    let entry_type = storage
        .get_entry_type("journal")
        .expect("get type should succeed")
        .expect("type should exist");

    let default = storage
        .get_default_template(&entry_type.id)
        .expect("get default should succeed");
    assert!(default.is_some());
    assert_eq!(default.unwrap().name, "persistent");

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_composition_persistence() {
    let temp = TempFile::new("ledger_composition_persist");
    let passphrase = "test-passphrase-secure-123";

    let comp_id: Uuid;
    let entry_id: Uuid;

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");

    {
        let mut storage =
            AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

        let device_id = Uuid::new_v4();
        let entry_type_id = create_basic_entry_type(&mut storage);

        comp_id = storage
            .create_composition(
                &NewComposition::new("persistent_comp", device_id)
                    .with_description("Persisted composition"),
            )
            .expect("create comp should succeed");

        entry_id = storage
            .insert_entry(&NewEntry::new(
                entry_type_id,
                1,
                serde_json::json!({"body": "test"}),
                device_id,
            ))
            .expect("insert should succeed");

        storage
            .attach_entry_to_composition(&entry_id, &comp_id)
            .expect("attach should succeed");

        storage.close(passphrase).expect("close should succeed");
    }

    // Reopen and verify persistence
    let storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("reopen should succeed");

    let comp = storage
        .get_composition("persistent_comp")
        .expect("get should succeed");
    assert!(comp.is_some());
    let comp = comp.unwrap();
    assert_eq!(comp.id, comp_id);
    assert_eq!(comp.description, Some("Persisted composition".to_string()));

    let comps = storage
        .get_entry_compositions(&entry_id)
        .expect("get comps should succeed");
    assert_eq!(comps.len(), 1);
    assert_eq!(comps[0].id, comp_id);

    storage.close(passphrase).expect("close should succeed");
}

// ============================================================================
// Edge Case Tests (M5)
// ============================================================================

#[test]
fn test_detach_entry_not_attached_fails() {
    let temp = TempFile::new("ledger_detach_not_attached");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let device_id = Uuid::new_v4();
    let entry_type_id = create_basic_entry_type(&mut storage);
    let comp_id = storage
        .create_composition(&NewComposition::new("project", device_id))
        .expect("create comp should succeed");

    let entry_id = storage
        .insert_entry(&NewEntry::new(
            entry_type_id,
            1,
            serde_json::json!({"body": "test"}),
            device_id,
        ))
        .expect("insert entry should succeed");

    // Try to detach entry that was never attached
    let result = storage.detach_entry_from_composition(&entry_id, &comp_id);
    assert!(result.is_err());

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_get_default_template_returns_latest_version() {
    let temp = TempFile::new("ledger_default_latest_version");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    let template_id = storage
        .create_template(&NewTemplate::new(
            "versioned",
            entry_type_id,
            serde_json::json!({"defaults": {"body": "v1"}}),
            device_id,
        ))
        .expect("create should succeed");

    storage
        .set_default_template(&entry_type_id, &template_id)
        .expect("set default should succeed");

    // Update template to v2
    storage
        .update_template(
            &template_id,
            serde_json::json!({"defaults": {"body": "v2"}}),
        )
        .expect("update should succeed");

    // Update template to v3
    storage
        .update_template(
            &template_id,
            serde_json::json!({"defaults": {"body": "v3"}}),
        )
        .expect("update should succeed");

    // get_default_template should return v3
    let default = storage
        .get_default_template(&entry_type_id)
        .expect("get default should succeed")
        .expect("default should exist");

    assert_eq!(default.version, 3);
    assert_eq!(default.template_json["defaults"]["body"], "v3");

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_delete_template_allows_new_default() {
    let temp = TempFile::new("ledger_delete_allows_new");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    let template1_id = storage
        .create_template(&NewTemplate::new(
            "first",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create first should succeed");

    let template2_id = storage
        .create_template(&NewTemplate::new(
            "second",
            entry_type_id,
            serde_json::json!({}),
            device_id,
        ))
        .expect("create second should succeed");

    // Set first as default
    storage
        .set_default_template(&entry_type_id, &template1_id)
        .expect("set should succeed");

    // Delete first template
    storage
        .delete_template(&template1_id)
        .expect("delete should succeed");

    // Should be able to set second as default without issues
    storage
        .set_default_template(&entry_type_id, &template2_id)
        .expect("set new default should succeed");

    let default = storage
        .get_default_template(&entry_type_id)
        .expect("get default should succeed")
        .expect("default should exist");
    assert_eq!(default.id, template2_id);

    storage.close(passphrase).expect("close should succeed");
}

#[test]
fn test_list_templates_returns_latest_versions_only() {
    let temp = TempFile::new("ledger_list_latest_only");
    let passphrase = "test-passphrase-secure-123";

    AgeSqliteStorage::create(&temp.path, passphrase).expect("create should succeed");
    let mut storage = AgeSqliteStorage::open(&temp.path, passphrase).expect("open should succeed");

    let entry_type_id = create_basic_entry_type(&mut storage);
    let device_id = Uuid::new_v4();

    let template_id = storage
        .create_template(&NewTemplate::new(
            "multi_version",
            entry_type_id,
            serde_json::json!({"defaults": {"body": "v1"}}),
            device_id,
        ))
        .expect("create should succeed");

    // Create multiple versions
    storage
        .update_template(
            &template_id,
            serde_json::json!({"defaults": {"body": "v2"}}),
        )
        .expect("update v2 should succeed");
    storage
        .update_template(
            &template_id,
            serde_json::json!({"defaults": {"body": "v3"}}),
        )
        .expect("update v3 should succeed");

    // list_templates should only return one entry with version 3
    let templates = storage.list_templates().expect("list should succeed");
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].version, 3);
    assert_eq!(templates[0].template_json["defaults"]["body"], "v3");

    storage.close(passphrase).expect("close should succeed");
}
