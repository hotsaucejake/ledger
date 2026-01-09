use std::fs;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ledger_core::storage::encryption::decrypt;
use ledger_core::storage::{AgeSqliteStorage, EntryFilter, NewEntry, NewEntryType, StorageEngine};
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
