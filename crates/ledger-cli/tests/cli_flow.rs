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

fn temp_xdg_dirs(prefix: &str) -> (PathBuf, PathBuf) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let base = std::env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), nanos));
    let config = base.join("config");
    let data = base.join("data");
    std::fs::create_dir_all(&config).expect("create config dir");
    std::fs::create_dir_all(&data).expect("create data dir");
    (config, data)
}

fn apply_xdg_env(cmd: &mut Command, config: &PathBuf, data: &PathBuf) {
    cmd.env("XDG_CONFIG_HOME", config)
        .env("XDG_DATA_HOME", data);
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
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_flow");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");
    assert!(
        init.status.success(),
        "init failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let mut add = Command::new(bin());
    add.arg("add")
        .arg("journal")
        .arg("--body")
        .arg("Hello from CLI")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut add, &config_home, &data_home);
    let add = add.output().expect("run add");
    assert!(add.status.success());

    let mut list = Command::new(bin());
    list.arg("list")
        .arg("--json")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut list, &config_home, &data_home);
    let list = list.output().expect("run list");
    assert!(list.status.success());

    let value: serde_json::Value = serde_json::from_slice(&list.stdout).expect("parse list json");
    let array = value.as_array().expect("list output array");
    assert!(!array.is_empty());
    let entry_id = array[0]
        .get("id")
        .and_then(|v| v.as_str())
        .expect("entry id");

    let mut show = Command::new(bin());
    show.arg("show")
        .arg(entry_id)
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut show, &config_home, &data_home);
    let show = show.output().expect("run show");
    assert!(show.status.success());
    let output = String::from_utf8_lossy(&show.stdout);
    assert!(output.contains("Hello from CLI"));
    assert!(output.contains("Type: journal"));
}

#[test]
fn test_cli_search_and_show_json() {
    let ledger_path = temp_ledger_path("ledger_cli_json");
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_json");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");
    assert!(init.status.success());

    let mut add = Command::new(bin());
    add.arg("add")
        .arg("journal")
        .arg("--body")
        .arg("JSON output")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut add, &config_home, &data_home);
    let add = add.output().expect("run add");
    assert!(add.status.success());

    let mut search = Command::new(bin());
    search
        .arg("search")
        .arg("JSON")
        .arg("--json")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut search, &config_home, &data_home);
    let search = search.output().expect("run search");
    assert!(search.status.success());

    let search_value: serde_json::Value =
        serde_json::from_slice(&search.stdout).expect("parse search json");
    let array = search_value.as_array().expect("search output array");
    assert!(!array.is_empty());
    let entry_id = array[0]
        .get("id")
        .and_then(|v| v.as_str())
        .expect("entry id");

    let mut show = Command::new(bin());
    show.arg("show")
        .arg(entry_id)
        .arg("--json")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut show, &config_home, &data_home);
    let show = show.output().expect("run show");
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
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_check_fail");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");
    assert!(init.status.success());

    let mut add = Command::new(bin());
    add.arg("add")
        .arg("journal")
        .arg("--body")
        .arg("Integrity break")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut add, &config_home, &data_home);
    let add = add.output().expect("run add");
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

    let mut check = Command::new(bin());
    check
        .arg("check")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut check, &config_home, &data_home);
    let check = check.output().expect("run check");
    assert!(!check.status.success());
    let output = String::from_utf8_lossy(&check.stderr);
    assert!(output.contains("Integrity check: FAILED"));
}

#[test]
fn test_cli_init_writes_default_config() {
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_init_config");

    let mut init = Command::new(bin());
    init.arg("init").env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");
    assert!(
        init.status.success(),
        "init failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let ledger_path = data_home.join("ledger").join("ledger.ledger");
    assert!(ledger_path.exists(), "ledger file should exist");

    let config_path = config_home.join("ledger").join("config.toml");
    assert!(config_path.exists(), "config file should exist");

    let contents = std::fs::read_to_string(&config_path).expect("read config");
    let value: toml::Value = contents.parse().expect("parse config");
    assert_eq!(
        value
            .get("ledger")
            .and_then(|section| section.get("path"))
            .and_then(|path| path.as_str()),
        Some(ledger_path.to_string_lossy().as_ref())
    );
    assert_eq!(
        value
            .get("security")
            .and_then(|section| section.get("tier"))
            .and_then(|tier| tier.as_str()),
        Some("passphrase")
    );
    assert_eq!(
        value
            .get("keychain")
            .and_then(|section| section.get("enabled"))
            .and_then(|enabled| enabled.as_bool()),
        Some(false)
    );
    assert_eq!(
        value
            .get("keyfile")
            .and_then(|section| section.get("mode"))
            .and_then(|mode| mode.as_str()),
        Some("none")
    );
}

#[test]
fn test_cli_missing_config_message() {
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_missing_config");

    let mut list = Command::new(bin());
    list.arg("list");
    apply_xdg_env(&mut list, &config_home, &data_home);
    let list = list.output().expect("run list");

    assert!(!list.status.success());
    let stderr = String::from_utf8_lossy(&list.stderr);
    let expected_path = config_home.join("ledger").join("config.toml");
    assert!(stderr.contains("No ledger found at"));
    assert!(stderr.contains(&*expected_path.to_string_lossy()));
}

#[test]
fn test_cli_missing_ledger_message() {
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_missing_ledger");
    let missing = temp_ledger_path("ledger_missing");

    let mut list = Command::new(bin());
    list.arg("list")
        .arg("--ledger")
        .arg(&missing)
        .env("LEDGER_PASSPHRASE", "test-passphrase-secure-123");
    apply_xdg_env(&mut list, &config_home, &data_home);
    let list = list.output().expect("run list");

    assert!(!list.status.success());
    let stderr = String::from_utf8_lossy(&list.stderr);
    assert!(stderr.contains("No ledger found at"));
    assert!(stderr.contains(&*missing.to_string_lossy()));
}

#[test]
fn test_cli_init_no_input_requires_passphrase() {
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_init_no_input");

    let mut init = Command::new(bin());
    init.arg("init").arg("--no-input");
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");

    assert!(!init.status.success());
    let stderr = String::from_utf8_lossy(&init.stderr);
    assert!(stderr.contains("--no-input requires LEDGER_PASSPHRASE"));
}

#[test]
fn test_cli_init_no_input_advanced_uses_defaults() {
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_init_advanced_no_input");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg("--advanced")
        .arg("--no-input")
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");

    assert!(
        init.status.success(),
        "init failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let ledger_path = data_home.join("ledger").join("ledger.ledger");
    assert!(ledger_path.exists(), "ledger file should exist");

    let config_path = config_home.join("ledger").join("config.toml");
    assert!(config_path.exists(), "config file should exist");
}

#[test]
fn test_cli_init_quiet_suppresses_output() {
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_init_quiet");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg("--quiet")
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");

    assert!(init.status.success());
    let stdout = String::from_utf8_lossy(&init.stdout);
    assert!(!stdout.contains("Welcome to Ledger"));
    assert!(stdout.trim().is_empty());
}

#[test]
fn test_cli_lock_succeeds_without_cache() {
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_lock");

    let mut lock = Command::new(bin());
    lock.arg("lock");
    apply_xdg_env(&mut lock, &config_home, &data_home);
    let lock = lock.output().expect("run lock");

    assert!(lock.status.success());
}

#[test]
fn test_cli_list_defaults_to_recent_limit() {
    let ledger_path = temp_ledger_path("ledger_cli_list_default");
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_list_default");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");
    assert!(init.status.success());

    for idx in 0..25 {
        let mut add = Command::new(bin());
        add.arg("add")
            .arg("journal")
            .arg("--body")
            .arg(format!("Entry {}", idx))
            .arg("--ledger")
            .arg(&ledger_path)
            .env("LEDGER_PASSPHRASE", passphrase);
        apply_xdg_env(&mut add, &config_home, &data_home);
        let add = add.output().expect("run add");
        assert!(add.status.success());
    }

    let mut list = Command::new(bin());
    list.arg("list")
        .arg("--json")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut list, &config_home, &data_home);
    let list = list.output().expect("run list");
    assert!(list.status.success());

    let value: serde_json::Value = serde_json::from_slice(&list.stdout).expect("parse list json");
    let array = value.as_array().expect("list output array");
    assert_eq!(array.len(), 20);
}
