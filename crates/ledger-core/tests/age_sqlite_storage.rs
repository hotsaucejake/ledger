use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ledger_core::storage::{AgeSqliteStorage, StorageEngine};

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
