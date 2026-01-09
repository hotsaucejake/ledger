use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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
