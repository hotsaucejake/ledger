use std::path::PathBuf;
use std::process::Command;
use std::ptr::NonNull;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ledger"))
}

fn temp_ledger_path(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let filename = format!("{}_{}_{}.ledger", prefix, std::process::id(), nanos);
    std::env::temp_dir().join(filename)
}

fn open_sqlite_from_file(path: &PathBuf, passphrase: &str) -> Connection {
    let encrypted = std::fs::read(path).expect("read should succeed");
    let plaintext = ledger_core::storage::encryption::decrypt(&encrypted, passphrase)
        .expect("decrypt should succeed");

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

#[test]
fn test_cli_init_add_list_show() {
    let ledger_path = temp_ledger_path("ledger_cli_flow");
    let passphrase = "test-passphrase-secure-123";

    let init = Command::new(bin())
        .arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run init");
    assert!(
        init.status.success(),
        "init failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let add = Command::new(bin())
        .arg("add")
        .arg("journal")
        .arg("--body")
        .arg("Hello from CLI")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run add");
    assert!(add.status.success());

    let list = Command::new(bin())
        .arg("list")
        .arg("--json")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run list");
    assert!(list.status.success());

    let value: serde_json::Value = serde_json::from_slice(&list.stdout).expect("parse list json");
    let array = value.as_array().expect("list output array");
    assert!(!array.is_empty());
    let entry_id = array[0]
        .get("id")
        .and_then(|v| v.as_str())
        .expect("entry id");

    let show = Command::new(bin())
        .arg("show")
        .arg(entry_id)
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run show");
    assert!(show.status.success());
    let output = String::from_utf8_lossy(&show.stdout);
    assert!(output.contains("Hello from CLI"));
    assert!(output.contains("Type: journal"));
}

#[test]
fn test_cli_search_and_show_json() {
    let ledger_path = temp_ledger_path("ledger_cli_json");
    let passphrase = "test-passphrase-secure-123";

    let init = Command::new(bin())
        .arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run init");
    assert!(init.status.success());

    let add = Command::new(bin())
        .arg("add")
        .arg("journal")
        .arg("--body")
        .arg("JSON output")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run add");
    assert!(add.status.success());

    let search = Command::new(bin())
        .arg("search")
        .arg("JSON")
        .arg("--json")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run search");
    assert!(search.status.success());

    let search_value: serde_json::Value =
        serde_json::from_slice(&search.stdout).expect("parse search json");
    let array = search_value.as_array().expect("search output array");
    assert!(!array.is_empty());
    let entry_id = array[0]
        .get("id")
        .and_then(|v| v.as_str())
        .expect("entry id");

    let show = Command::new(bin())
        .arg("show")
        .arg(entry_id)
        .arg("--json")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run show");
    assert!(show.status.success());
    let show_value: serde_json::Value =
        serde_json::from_slice(&show.stdout).expect("parse show json");
    assert_eq!(
        show_value.get("entry_type_name").and_then(|v| v.as_str()),
        Some("journal")
    );
}

#[test]
fn test_cli_check_failure() {
    let ledger_path = temp_ledger_path("ledger_cli_check_fail");
    let passphrase = "test-passphrase-secure-123";

    let init = Command::new(bin())
        .arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run init");
    assert!(init.status.success());

    let add = Command::new(bin())
        .arg("add")
        .arg("journal")
        .arg("--body")
        .arg("Integrity break")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run add");
    assert!(add.status.success());

    let conn = open_sqlite_from_file(&ledger_path, passphrase);
    let entry_id: String = conn
        .query_row("SELECT id FROM entries LIMIT 1", [], |row| {
            row.get::<_, String>(0)
        })
        .expect("entry id");
    conn.execute("DELETE FROM entries_fts WHERE entry_id = ?", [entry_id])
        .expect("delete fts");

    let data = conn.serialize(DatabaseName::Main).expect("serialize");
    let encrypted =
        ledger_core::storage::encryption::encrypt(data.as_ref(), passphrase).expect("encrypt");
    std::fs::write(&ledger_path, encrypted).expect("write");

    let check = Command::new(bin())
        .arg("check")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase)
        .output()
        .expect("run check");
    assert!(!check.status.success());
    let output = String::from_utf8_lossy(&check.stdout);
    assert!(output.contains("Integrity check: FAILED"));
}
