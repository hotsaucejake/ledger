use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use ledger_core::storage::AgeSqliteStorage;
use ledger_core::StorageEngine;

use crate::cache::{cache_clear, cache_config, cache_get, cache_store, ledger_hash, CacheConfig};
use crate::cli::Cli;
use crate::config::{
    default_config_path, default_keyfile_path, read_config, KeyfileMode, SecurityTier,
};
use crate::helpers::prompt_passphrase;
use crate::security::{
    key_bytes_to_passphrase, keychain_clear, keychain_get, keychain_set, read_keyfile_encrypted,
    read_keyfile_plain,
};

pub struct SecurityConfig {
    pub tier: SecurityTier,
    pub keychain_enabled: bool,
    pub keyfile_mode: KeyfileMode,
    pub keyfile_path: Option<PathBuf>,
    pub cache_ttl_seconds: u64,
    pub editor: Option<String>,
}

pub fn resolve_config_path() -> anyhow::Result<PathBuf> {
    if let Ok(value) = std::env::var("LEDGER_CONFIG") {
        if !value.trim().is_empty() {
            return Ok(PathBuf::from(value));
        }
    }
    default_config_path()
}

pub fn resolve_ledger_path(cli: &Cli) -> anyhow::Result<String> {
    if let Some(path) = cli.ledger.clone() {
        return Ok(path);
    }

    let config_path = resolve_config_path()?;
    if !config_path.exists() {
        return Err(anyhow::anyhow!(missing_config_message(&config_path)));
    }

    let config = read_config(&config_path)?;
    Ok(config.ledger.path)
}

pub fn open_storage_with_retry(
    cli: &Cli,
    no_input: bool,
) -> anyhow::Result<(AgeSqliteStorage, String)> {
    let target = resolve_ledger_path(cli)?;
    let interactive = std::io::stdin().is_terminal() && !no_input;
    let target_path = Path::new(&target);
    let security = load_security_config(cli)?;
    let cache_config = cache_config(target_path, security.cache_ttl_seconds).unwrap_or(None);

    if let Some(config) = cache_config.as_ref() {
        if let Ok(Some(passphrase)) = cache_get(config) {
            match AgeSqliteStorage::open(target_path, &passphrase) {
                Ok(storage) => return Ok((storage, passphrase)),
                Err(err) if is_incorrect_passphrase_error(&err) => {
                    let _ = cache_clear(&config.socket_path);
                }
                Err(err) => return Err(anyhow::anyhow!("{}", err)),
            }
        }
    }

    if matches!(security.tier, SecurityTier::DeviceKeyfile) {
        let keyfile_path = security
            .keyfile_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keyfile path is required for device_keyfile"))?;
        let key_bytes = read_keyfile_plain(keyfile_path)?;
        let passphrase = key_bytes_to_passphrase(&key_bytes);
        return open_with_passphrase_and_cache(
            cli,
            target_path,
            &passphrase,
            cache_config.as_ref(),
        );
    }

    if matches!(security.tier, SecurityTier::PassphraseKeyfile) {
        let keyfile_path = security
            .keyfile_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keyfile path is required for passphrase_keyfile"))?;
        let env_passphrase = std::env::var("LEDGER_PASSPHRASE")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let key_bytes =
            decrypt_keyfile_with_retry(keyfile_path, env_passphrase.as_deref(), interactive)?;
        let passphrase = key_bytes_to_passphrase(&key_bytes);
        return open_with_passphrase_and_cache(
            cli,
            target_path,
            &passphrase,
            cache_config.as_ref(),
        );
    }

    if matches!(security.tier, SecurityTier::PassphraseKeychain) && security.keychain_enabled {
        let account = ledger_hash(target_path);
        match keychain_get(&account) {
            Ok(Some(passphrase)) => {
                if let Ok(storage) = AgeSqliteStorage::open(target_path, &passphrase) {
                    return Ok((storage, passphrase));
                }
                let _ = keychain_clear(&account);
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("Warning: {}", err);
            }
        }
    }

    let env_passphrase = std::env::var("LEDGER_PASSPHRASE")
        .ok()
        .filter(|v| !v.trim().is_empty());
    if let Some(passphrase) = env_passphrase {
        let (storage, passphrase) =
            open_with_passphrase_and_cache(cli, target_path, &passphrase, cache_config.as_ref())?;
        if matches!(security.tier, SecurityTier::PassphraseKeychain) && security.keychain_enabled {
            let account = ledger_hash(target_path);
            let _ = keychain_set(&account, &passphrase);
        }
        return Ok((storage, passphrase));
    }

    let (storage, passphrase) =
        open_with_retry_prompt(cli, target_path, interactive, cache_config.as_ref())?;
    if matches!(security.tier, SecurityTier::PassphraseKeychain) && security.keychain_enabled {
        let account = ledger_hash(target_path);
        let _ = keychain_set(&account, &passphrase);
    }
    Ok((storage, passphrase))
}

pub fn missing_ledger_message(path: &Path) -> String {
    format!(
        "No ledger found at {}\n\nRun:\n  ledger init\n\nOr specify a ledger path:\n  LEDGER_PATH=/path/to/my.ledger ledger init",
        path.display()
    )
}

pub fn missing_config_message(config_path: &Path) -> String {
    format!(
        "No ledger found at {}\n\nRun:\n  ledger init\n\nOr specify a ledger path:\n  LEDGER_PATH=/path/to/my.ledger ledger init",
        config_path.display()
    )
}

pub fn exit_not_found_with_hint(message: &str, hint: &str) -> ! {
    eprintln!("Error: {}", message);
    eprintln!("{}", hint);
    std::process::exit(3);
}

pub fn device_keyfile_warning() -> &'static str {
    "WARNING: You selected device_keyfile. This stores an unencrypted key on disk.\nIf your device is compromised, your ledger can be decrypted without a passphrase.\nContinue?"
}

pub fn load_security_config(_cli: &Cli) -> anyhow::Result<SecurityConfig> {
    let config_path = resolve_config_path()?;
    if config_path.exists() {
        let config = read_config(&config_path)?;
        let keyfile_path = config.keyfile.path.as_ref().map(PathBuf::from);
        let security = SecurityConfig {
            tier: config.security.tier,
            keychain_enabled: config.keychain.enabled,
            keyfile_mode: config.keyfile.mode,
            keyfile_path,
            cache_ttl_seconds: config.security.passphrase_cache_ttl_seconds,
            editor: config.ui.editor,
        };
        validate_security_config(&security)?;
        return Ok(security);
    }

    Ok(SecurityConfig {
        tier: SecurityTier::Passphrase,
        keychain_enabled: false,
        keyfile_mode: KeyfileMode::None,
        keyfile_path: Some(default_keyfile_path()?),
        cache_ttl_seconds: 0,
        editor: None,
    })
}

fn validate_security_config(config: &SecurityConfig) -> anyhow::Result<()> {
    match config.tier {
        SecurityTier::PassphraseKeyfile => {
            if !matches!(config.keyfile_mode, KeyfileMode::Encrypted) {
                return Err(anyhow::anyhow!(
                    "keyfile mode must be encrypted for passphrase_keyfile"
                ));
            }
            if config.keyfile_path.is_none() {
                return Err(anyhow::anyhow!(
                    "keyfile path is required for passphrase_keyfile"
                ));
            }
        }
        SecurityTier::DeviceKeyfile => {
            if !matches!(config.keyfile_mode, KeyfileMode::Plain) {
                return Err(anyhow::anyhow!(
                    "keyfile mode must be plain for device_keyfile"
                ));
            }
            if config.keyfile_path.is_none() {
                return Err(anyhow::anyhow!(
                    "keyfile path is required for device_keyfile"
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn open_with_passphrase_and_cache(
    cli: &Cli,
    path: &Path,
    passphrase: &str,
    cache_config: Option<&CacheConfig>,
) -> anyhow::Result<(AgeSqliteStorage, String)> {
    match AgeSqliteStorage::open(path, passphrase) {
        Ok(storage) => {
            if let Some(config) = cache_config {
                if !cli.quiet {
                    println!(
                        "Note: Passphrase caching keeps your passphrase in memory for {} seconds.",
                        config.ttl.as_secs()
                    );
                }
                let _ = cache_store(config, passphrase);
            }
            Ok((storage, passphrase.to_string()))
        }
        Err(err) if is_incorrect_passphrase_error(&err) => {
            eprintln!("Error: Incorrect passphrase.");
            std::process::exit(5);
        }
        Err(err) if is_missing_ledger_error(&err) => {
            Err(anyhow::anyhow!(missing_ledger_message(path)))
        }
        Err(err) => Err(anyhow::anyhow!("{}", err)),
    }
}

fn open_with_retry_prompt(
    cli: &Cli,
    path: &Path,
    interactive: bool,
    cache_config: Option<&CacheConfig>,
) -> anyhow::Result<(AgeSqliteStorage, String)> {
    let test_attempts = if !interactive && cfg!(feature = "test-support") {
        std::env::var("LEDGER_TEST_PASSPHRASE_ATTEMPTS")
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect::<Vec<String>>()
            })
    } else {
        None
    };
    let max_attempts: u32 = if interactive || test_attempts.is_some() {
        3
    } else {
        1
    };
    let mut attempts: u32 = 0;

    loop {
        attempts += 1;
        let passphrase = if let Some(values) = test_attempts.as_ref() {
            values
                .get((attempts - 1) as usize)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No passphrase attempts remaining"))?
        } else {
            prompt_passphrase(interactive)?
        };
        match AgeSqliteStorage::open(path, &passphrase) {
            Ok(storage) => {
                if let Some(config) = cache_config {
                    if !cli.quiet {
                        println!(
                            "Note: Passphrase caching keeps your passphrase in memory for {} seconds.",
                            config.ttl.as_secs()
                        );
                    }
                    let _ = cache_store(config, &passphrase);
                }
                return Ok((storage, passphrase));
            }
            Err(err) if is_incorrect_passphrase_error(&err) => {
                let remaining = max_attempts.saturating_sub(attempts);
                if remaining == 0 {
                    eprintln!("Error: Too many failed passphrase attempts.");
                    eprintln!(
                        "Hint: If you forgot your passphrase, the ledger cannot be recovered."
                    );
                    eprintln!("      Backups use the same passphrase.");
                    std::process::exit(5);
                }
                eprintln!(
                    "Incorrect passphrase. {} attempt{} remaining.",
                    remaining,
                    if remaining == 1 { "" } else { "s" }
                );
                continue;
            }
            Err(err) if is_missing_ledger_error(&err) => {
                return Err(anyhow::anyhow!(missing_ledger_message(path)));
            }
            Err(err) => return Err(anyhow::anyhow!("{}", err)),
        }
    }
}

fn decrypt_keyfile_with_retry(
    path: &Path,
    passphrase_env: Option<&str>,
    interactive: bool,
) -> anyhow::Result<zeroize::Zeroizing<Vec<u8>>> {
    if let Some(passphrase) = passphrase_env {
        return match read_keyfile_encrypted(path, passphrase) {
            Ok(bytes) => Ok(bytes),
            Err(err) if err.to_string().contains("Incorrect passphrase") => {
                eprintln!("Error: Incorrect passphrase.");
                std::process::exit(5);
            }
            Err(err) => Err(err),
        };
    }

    let max_attempts: u32 = if interactive { 3 } else { 1 };
    let mut attempts: u32 = 0;

    loop {
        attempts += 1;
        let passphrase = prompt_passphrase(interactive)?;
        match read_keyfile_encrypted(path, &passphrase) {
            Ok(bytes) => return Ok(bytes),
            Err(err) if err.to_string().contains("Incorrect passphrase") => {
                let remaining = max_attempts.saturating_sub(attempts);
                if remaining == 0 {
                    eprintln!("Error: Too many failed passphrase attempts.");
                    eprintln!(
                        "Hint: If you forgot your passphrase, the ledger cannot be recovered."
                    );
                    eprintln!("      Backups use the same passphrase.");
                    std::process::exit(5);
                }
                eprintln!(
                    "Incorrect passphrase. {} attempt{} remaining.",
                    remaining,
                    if remaining == 1 { "" } else { "s" }
                );
                continue;
            }
            Err(err) => return Err(err),
        }
    }
}

fn is_incorrect_passphrase_error(err: &ledger_core::error::LedgerError) -> bool {
    err.to_string().contains("Incorrect passphrase")
}

fn is_missing_ledger_error(err: &ledger_core::error::LedgerError) -> bool {
    matches!(err, ledger_core::error::LedgerError::Storage(message) if message == "Ledger file not found")
}

#[cfg(test)]
mod tests {
    use super::device_keyfile_warning;

    #[test]
    fn test_device_keyfile_warning_copy() {
        let warning = device_keyfile_warning();
        assert!(warning.contains("device_keyfile"));
        assert!(warning.contains("unencrypted key on disk"));
        assert!(warning.contains("Continue?"));
    }
}
