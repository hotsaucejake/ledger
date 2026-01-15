use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use jot_core::storage::encryption::{decrypt, encrypt};

struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        let filename = format!("{}_{}_{}.age", prefix, std::process::id(), nanos);
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
fn test_encrypted_file_round_trip() {
    let temp = TempFile::new("ledger_round_trip");
    let passphrase = "test-passphrase-secure-123";
    let plaintext = b"journal entry: hello world";

    let encrypted = encrypt(plaintext, passphrase).expect("encryption should succeed");
    fs::write(&temp.path, &encrypted).expect("write should succeed");

    let on_disk = fs::read(&temp.path).expect("read should succeed");
    assert_ne!(on_disk, plaintext);

    let decrypted = decrypt(&on_disk, passphrase).expect("decryption should succeed");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypted_file_wrong_passphrase_fails() {
    let temp = TempFile::new("ledger_wrong_passphrase");
    let passphrase = "correct-passphrase-123";
    let wrong_passphrase = "wrong-passphrase-456";
    let plaintext = b"secret entry";

    let encrypted = encrypt(plaintext, passphrase).expect("encryption should succeed");
    fs::write(&temp.path, &encrypted).expect("write should succeed");

    let on_disk = fs::read(&temp.path).expect("read should succeed");
    let result = decrypt(&on_disk, wrong_passphrase);
    assert!(result.is_err());
}

#[test]
fn test_encrypted_file_does_not_contain_plaintext() {
    let temp = TempFile::new("ledger_no_plaintext");
    let passphrase = "test-passphrase-secure-123";
    let plaintext = b"secret entry with marker: PLAINTEXT_MARKER_123";

    let encrypted = encrypt(plaintext, passphrase).expect("encryption should succeed");
    fs::write(&temp.path, &encrypted).expect("write should succeed");

    let on_disk = fs::read(&temp.path).expect("read should succeed");
    let haystack = String::from_utf8_lossy(&on_disk);
    assert!(!haystack.contains("PLAINTEXT_MARKER_123"));
}
