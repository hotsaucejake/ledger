use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::NonNull;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::serialize::OwnedData;
use rusqlite::{Connection, DatabaseName};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ledger_core::storage::{AgeSqliteStorage, StorageEngine};

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

fn write_config_file(
    config_home: &Path,
    ledger_path: &Path,
    tier: &str,
    keyfile_mode: &str,
    keyfile_path: Option<&Path>,
) {
    let config_path = config_home.join("ledger").join("config.toml");
    let keyfile_path_value = keyfile_path
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    let keyfile_path_line = if keyfile_path.is_some() {
        format!("path = \"{}\"\n", keyfile_path_value)
    } else {
        String::new()
    };
    let contents = format!(
        "[ledger]\npath = \"{}\"\n\n[security]\ntier = \"{}\"\npassphrase_cache_ttl_seconds = 0\n\n[keychain]\nenabled = false\n\n[keyfile]\nmode = \"{}\"\n{}",
        ledger_path.to_string_lossy(),
        tier,
        keyfile_mode,
        keyfile_path_line
    );
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config dir");
    std::fs::write(&config_path, contents).expect("write config");
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

fn create_ledger_with_passphrase(path: &Path, passphrase: &str) {
    let _ = AgeSqliteStorage::create(path, passphrase).expect("create ledger");
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
fn test_cli_missing_config_message_uses_env_override() {
    let (_config_home, data_home) = temp_xdg_dirs("ledger_cli_missing_config_env");
    let override_path = std::env::temp_dir().join(format!(
        "ledger_config_{}_{}.toml",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));

    let mut list = Command::new(bin());
    list.arg("list").env("LEDGER_CONFIG", &override_path);
    apply_xdg_env(&mut list, &_config_home, &data_home);
    let list = list.output().expect("run list");

    assert!(!list.status.success());
    let stderr = String::from_utf8_lossy(&list.stderr);
    assert!(stderr.contains("No ledger found at"));
    assert!(stderr.contains(&*override_path.to_string_lossy()));
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
fn test_cli_init_writes_ui_defaults() {
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_ui_defaults");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg("--no-input")
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");
    assert!(init.status.success());

    let config_path = config_home.join("ledger").join("config.toml");
    let contents = std::fs::read_to_string(&config_path).expect("read config");
    let value: toml::Value = contents.parse().expect("parse config");

    let ui = value.get("ui").expect("ui section");
    assert_eq!(
        ui.get("timezone").and_then(|v| v.as_str()),
        None,
        "timezone should be omitted by default"
    );
    assert_eq!(
        ui.get("editor").and_then(|v| v.as_str()),
        None,
        "editor should be omitted by default"
    );
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

#[test]
fn test_cli_list_empty_message() {
    let ledger_path = temp_ledger_path("ledger_cli_list_empty");
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_list_empty");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");
    assert!(init.status.success());

    let mut list = Command::new(bin());
    list.arg("list")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut list, &config_home, &data_home);
    let list = list.output().expect("run list");
    assert!(list.status.success());
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("No entries found."));
}

#[test]
fn test_cli_search_empty_message() {
    let ledger_path = temp_ledger_path("ledger_cli_search_empty");
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_search_empty");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");
    assert!(init.status.success());

    let mut search = Command::new(bin());
    search
        .arg("search")
        .arg("missing")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut search, &config_home, &data_home);
    let search = search.output().expect("run search");
    assert!(search.status.success());
    let stdout = String::from_utf8_lossy(&search.stdout);
    assert!(stdout.contains("No entries found."));
}

#[test]
fn test_cli_list_truncates_summary() {
    let ledger_path = temp_ledger_path("ledger_cli_list_truncate");
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_list_truncate");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");
    assert!(init.status.success());

    let long_body = "a".repeat(200);
    let mut add = Command::new(bin());
    add.arg("add")
        .arg("journal")
        .arg("--body")
        .arg(&long_body)
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut add, &config_home, &data_home);
    let add = add.output().expect("run add");
    assert!(add.status.success());

    let mut list = Command::new(bin());
    list.arg("list")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut list, &config_home, &data_home);
    let list = list.output().expect("run list");
    assert!(list.status.success());
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("..."));
}

#[test]
fn test_cli_editor_override_is_used() {
    let ledger_path = temp_ledger_path("ledger_cli_editor_override");
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_editor_override");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let init = init.output().expect("run init");
    assert!(init.status.success());

    let editor_dir = std::env::temp_dir().join(format!(
        "ledger_editor_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    std::fs::create_dir_all(&editor_dir).expect("create editor dir");
    let editor_path = editor_dir.join("editor.sh");
    let script = "#!/bin/sh\nprintf \"Editor content\" > \"$1\"\n";
    std::fs::write(&editor_path, script).expect("write editor script");
    let mut perms = std::fs::metadata(&editor_path)
        .expect("stat editor")
        .permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o700);
        std::fs::set_permissions(&editor_path, perms).expect("chmod editor");
    }

    let config_path = config_home.join("ledger").join("config.toml");
    let contents = format!(
        "[ledger]\npath = \"{}\"\n\n[security]\ntier = \"passphrase\"\npassphrase_cache_ttl_seconds = 0\n\n[keychain]\nenabled = false\n\n[keyfile]\nmode = \"none\"\n\n[ui]\neditor = \"{}\"\n",
        ledger_path.to_string_lossy(),
        editor_path.to_string_lossy()
    );
    std::fs::write(&config_path, contents).expect("write config");

    let mut add = Command::new(bin());
    add.arg("add")
        .arg("journal")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut add, &config_home, &data_home);
    let add = add.output().expect("run add");
    assert!(add.status.success(), "add failed: {:?}", add);

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
    let body = array[0]
        .get("data")
        .and_then(|data| data.get("body"))
        .and_then(|v| v.as_str())
        .expect("body");
    assert_eq!(body, "Editor content");
}

#[test]
fn test_cli_init_advanced_ui_fields() {
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_init_ui_adv");

    let mut init = Command::new(bin());
    init.arg("init")
        .arg("--no-input")
        .arg("--timezone")
        .arg("America/New_York")
        .arg("--editor")
        .arg("vim")
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut init, &config_home, &data_home);
    let output = init.output().expect("run init");
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config_path = config_home.join("ledger").join("config.toml");
    let contents = std::fs::read_to_string(&config_path).expect("read config");
    let value: toml::Value = contents.parse().expect("parse config");
    let ui = value.get("ui").expect("ui section");
    assert_eq!(
        ui.get("timezone").and_then(|v| v.as_str()),
        Some("America/New_York")
    );
    assert_eq!(ui.get("editor").and_then(|v| v.as_str()), Some("vim"));
}

#[test]
fn test_cli_passphrase_keyfile_flow() {
    let ledger_path = temp_ledger_path("ledger_cli_keyfile_encrypted");
    let passphrase = "test-passphrase-secure-123";
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_keyfile_encrypted");
    let keyfile_path = config_home.join("ledger").join("ledger.key");

    let key_bytes = vec![7u8; 32];
    let key_passphrase = STANDARD.encode(&key_bytes);
    create_ledger_with_passphrase(&ledger_path, &key_passphrase);

    let encrypted =
        ledger_core::storage::encryption::encrypt(&key_bytes, passphrase).expect("encrypt keyfile");
    std::fs::create_dir_all(keyfile_path.parent().expect("keyfile parent"))
        .expect("create keyfile dir");
    std::fs::write(&keyfile_path, encrypted).expect("write keyfile");

    write_config_file(
        &config_home,
        &ledger_path,
        "passphrase_keyfile",
        "encrypted",
        Some(&keyfile_path),
    );

    let mut list = Command::new(bin());
    list.arg("list")
        .arg("--ledger")
        .arg(&ledger_path)
        .env("LEDGER_PASSPHRASE", passphrase);
    apply_xdg_env(&mut list, &config_home, &data_home);
    let list = list.output().expect("run list");

    assert!(list.status.success());
}

#[test]
fn test_cli_device_keyfile_flow() {
    let ledger_path = temp_ledger_path("ledger_cli_keyfile_plain");
    let (config_home, data_home) = temp_xdg_dirs("ledger_cli_keyfile_plain");
    let keyfile_path = config_home.join("ledger").join("ledger.key");

    let key_bytes = vec![9u8; 32];
    let key_passphrase = STANDARD.encode(&key_bytes);
    create_ledger_with_passphrase(&ledger_path, &key_passphrase);

    std::fs::create_dir_all(keyfile_path.parent().expect("keyfile parent"))
        .expect("create keyfile dir");
    std::fs::write(&keyfile_path, &key_bytes).expect("write keyfile");

    write_config_file(
        &config_home,
        &ledger_path,
        "device_keyfile",
        "plain",
        Some(&keyfile_path),
    );

    let mut list = Command::new(bin());
    list.arg("list").arg("--ledger").arg(&ledger_path);
    apply_xdg_env(&mut list, &config_home, &data_home);
    let list = list.output().expect("run list");

    assert!(list.status.success());
}
