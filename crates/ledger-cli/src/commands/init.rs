use std::io::IsTerminal;

use dialoguer::{Confirm, Input, Select};
use ledger_core::storage::{AgeSqliteStorage, NewEntryType, StorageEngine};
use uuid::Uuid;

use crate::app::{device_keyfile_warning, resolve_config_path};
use crate::cache::ledger_hash;
use crate::cli::Cli;
use crate::config::{
    default_keyfile_path, default_ledger_path, write_config, KeyfileMode, LedgerConfig,
    SecurityTier,
};
use crate::helpers::prompt_init_passphrase;
use crate::security::{
    generate_key_bytes, key_bytes_to_passphrase, keychain_set, write_keyfile_encrypted,
    write_keyfile_plain,
};

#[allow(clippy::too_many_arguments)]
pub fn handle_init(
    cli: &Cli,
    path: Option<String>,
    advanced: bool,
    no_input: bool,
    timezone_arg: Option<String>,
    editor_arg: Option<String>,
    passphrase_cache_ttl_seconds_arg: Option<u64>,
    keyfile_path_arg: Option<String>,
    config_path_arg: Option<String>,
) -> anyhow::Result<()> {
    run_init_wizard(
        cli,
        path,
        advanced,
        no_input,
        timezone_arg,
        editor_arg,
        passphrase_cache_ttl_seconds_arg,
        keyfile_path_arg,
        config_path_arg,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_init_wizard(
    cli: &Cli,
    path: Option<String>,
    advanced: bool,
    no_input: bool,
    timezone_arg: Option<String>,
    editor_arg: Option<String>,
    passphrase_cache_ttl_seconds_arg: Option<u64>,
    keyfile_path_arg: Option<String>,
    config_path_arg: Option<String>,
) -> anyhow::Result<()> {
    let interactive = std::io::stdin().is_terminal();
    let effective_no_input = no_input || !interactive;

    if !cli.quiet && !effective_no_input {
        println!("Welcome to Ledger.\n");
    }

    let default_ledger = default_ledger_path()?;
    let ledger_path = match path.or_else(|| cli.ledger.clone()) {
        Some(value) => std::path::PathBuf::from(value),
        None => {
            if effective_no_input {
                default_ledger.clone()
            } else {
                let input: String = Input::new()
                    .with_prompt("Ledger file location")
                    .default(default_ledger.to_string_lossy().to_string())
                    .interact_text()?;
                std::path::PathBuf::from(input)
            }
        }
    };

    let mut config_path = if let Some(ref value) = config_path_arg {
        std::path::PathBuf::from(value)
    } else {
        resolve_config_path()?
    };
    let mut passphrase_cache_ttl_seconds = passphrase_cache_ttl_seconds_arg.unwrap_or(0);
    let mut keyfile_path = if let Some(ref value) = keyfile_path_arg {
        std::path::PathBuf::from(value)
    } else {
        default_keyfile_path()?
    };
    let mut timezone: Option<String> = timezone_arg;
    let mut editor: Option<String> = editor_arg;

    let passphrase = if let Ok(value) = std::env::var("LEDGER_PASSPHRASE") {
        if !value.trim().is_empty() {
            value
        } else if effective_no_input {
            return Err(anyhow::anyhow!(
                "--no-input requires LEDGER_PASSPHRASE for initialization"
            ));
        } else {
            prompt_init_passphrase()?
        }
    } else if effective_no_input {
        return Err(anyhow::anyhow!(
            "--no-input requires LEDGER_PASSPHRASE for initialization"
        ));
    } else {
        prompt_init_passphrase()?
    };

    let mut tier = SecurityTier::Passphrase;
    if !effective_no_input {
        let options = [
            "Passphrase only (recommended)",
            "Passphrase + OS keychain",
            "Passphrase + encrypted keyfile",
            "Device keyfile only (reduced security)",
        ];
        let choice = Select::new()
            .with_prompt("Security level")
            .default(0)
            .items(&options)
            .interact()?;
        tier = match choice {
            0 => SecurityTier::Passphrase,
            1 => SecurityTier::PassphraseKeychain,
            2 => SecurityTier::PassphraseKeyfile,
            3 => SecurityTier::DeviceKeyfile,
            _ => SecurityTier::Passphrase,
        };
    }

    if matches!(tier, SecurityTier::DeviceKeyfile) && !effective_no_input {
        let proceed = Confirm::new()
            .with_prompt(device_keyfile_warning())
            .default(false)
            .interact()?;
        if !proceed {
            return Err(anyhow::anyhow!("Initialization cancelled"));
        }
    }

    if advanced && !effective_no_input {
        if timezone.is_none() {
            let tz_input: String = Input::new()
                .with_prompt("Timezone")
                .default("auto".to_string())
                .interact_text()?;
            if tz_input.trim().is_empty() || tz_input.trim().eq_ignore_ascii_case("auto") {
                timezone = None;
            } else {
                timezone = Some(tz_input);
            }
        }

        if editor.is_none() {
            let default_editor = default_editor();
            let editor_input: String = Input::new()
                .with_prompt("Default editor")
                .default(default_editor)
                .interact_text()?;
            if !editor_input.trim().is_empty() {
                editor = Some(editor_input);
            }
        }

        if passphrase_cache_ttl_seconds_arg.is_none() {
            let ttl_input: String = Input::new()
                .with_prompt("Passphrase cache (seconds)")
                .default(passphrase_cache_ttl_seconds.to_string())
                .interact_text()?;
            passphrase_cache_ttl_seconds = ttl_input.parse().map_err(|_| {
                anyhow::anyhow!(
                    "Invalid cache TTL: {} (expected integer seconds)",
                    ttl_input
                )
            })?;
        }

        if matches!(
            tier,
            SecurityTier::PassphraseKeyfile | SecurityTier::DeviceKeyfile
        ) && keyfile_path_arg.is_none()
        {
            let input: String = Input::new()
                .with_prompt("Keyfile path")
                .default(keyfile_path.to_string_lossy().to_string())
                .interact_text()?;
            keyfile_path = std::path::PathBuf::from(input);
        }

        if config_path_arg.is_none() {
            let input: String = Input::new()
                .with_prompt("Ledger config path")
                .default(config_path.to_string_lossy().to_string())
                .interact_text()?;
            config_path = std::path::PathBuf::from(input);
        }
    }

    let (ledger_passphrase, keyfile_mode, keyfile_path_value) = match tier {
        SecurityTier::Passphrase => (passphrase.clone(), KeyfileMode::None, None),
        SecurityTier::PassphraseKeychain => (passphrase.clone(), KeyfileMode::None, None),
        SecurityTier::PassphraseKeyfile => {
            let key_bytes = generate_key_bytes()?;
            write_keyfile_encrypted(&keyfile_path, &key_bytes, &passphrase)?;
            (
                key_bytes_to_passphrase(&key_bytes),
                KeyfileMode::Encrypted,
                Some(keyfile_path.clone()),
            )
        }
        SecurityTier::DeviceKeyfile => {
            let key_bytes = generate_key_bytes()?;
            write_keyfile_plain(&keyfile_path, &key_bytes)?;
            (
                key_bytes_to_passphrase(&key_bytes),
                KeyfileMode::Plain,
                Some(keyfile_path.clone()),
            )
        }
    };

    if let Some(parent) = ledger_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!(
                "Failed to create ledger directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let device_id = AgeSqliteStorage::create(&ledger_path, &ledger_passphrase)?;
    let mut storage = AgeSqliteStorage::open(&ledger_path, &ledger_passphrase)?;
    ensure_journal_entry_type(&mut storage, device_id)?;
    storage.close(&ledger_passphrase)?;

    let config = LedgerConfig::new(
        ledger_path.clone(),
        tier,
        passphrase_cache_ttl_seconds,
        keyfile_mode,
        keyfile_path_value,
        timezone,
        editor,
    );
    write_config(&config_path, &config)?;

    if matches!(tier, SecurityTier::PassphraseKeychain) {
        let account = ledger_hash(&ledger_path);
        let _ = keychain_set(&account, &passphrase);
    }

    if !cli.quiet {
        println!("Ledger created at {}", ledger_path.to_string_lossy());
        println!("Config written to {}", config_path.to_string_lossy());
        if passphrase_cache_ttl_seconds > 0 {
            println!(
                "Note: Passphrase caching keeps your passphrase in memory for {} seconds.",
                passphrase_cache_ttl_seconds
            );
        }
    }

    Ok(())
}

fn default_editor() -> String {
    std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string())
}

fn ensure_journal_entry_type(
    storage: &mut AgeSqliteStorage,
    device_id: Uuid,
) -> anyhow::Result<()> {
    if storage.get_entry_type("journal")?.is_some() {
        return Ok(());
    }

    let schema = serde_json::json!({
        "fields": [
            {"name": "body", "type": "text", "required": true}
        ]
    });
    let entry_type = NewEntryType::new("journal", schema, device_id);
    storage.create_entry_type(&entry_type)?;
    Ok(())
}
