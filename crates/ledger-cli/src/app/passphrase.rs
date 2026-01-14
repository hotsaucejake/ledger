//! Passphrase handling and storage opening with retry logic.

use std::io::IsTerminal;
use std::path::Path;

use ledger_core::storage::AgeSqliteStorage;
use ledger_core::StorageEngine;

use crate::cache::{cache_clear, cache_config, cache_get, cache_store, ledger_hash, CacheConfig};
use crate::cli::Cli;
use crate::config::SecurityTier;
use crate::errors::CliError;
use crate::helpers::prompt_passphrase;
use crate::security::{
    key_bytes_to_passphrase, keychain_clear, keychain_get, keychain_set, read_keyfile_encrypted,
    read_keyfile_plain,
};

use super::resolver::{missing_ledger_message, resolve_ledger_path};
use super::security_config::{load_security_config, SecurityConfig};

/// Open storage with passphrase retry logic based on security tier.
pub fn open_storage_with_retry(
    cli: &Cli,
    no_input: bool,
) -> anyhow::Result<(AgeSqliteStorage, String)> {
    let target = resolve_ledger_path(cli)?;
    let interactive = std::io::stdin().is_terminal() && !no_input;
    let target_path = Path::new(&target);
    let security = load_security_config(cli)?;
    let cache_config = cache_config(target_path, security.cache_ttl_seconds).unwrap_or(None);

    // Try cache first
    if let Some(config) = cache_config.as_ref() {
        if let Ok(Some(passphrase)) = cache_get(config) {
            match AgeSqliteStorage::open(target_path, &passphrase) {
                Ok(storage) => {
                    if interactive && !cli.quiet {
                        eprintln!("Using cached passphrase");
                    }
                    return Ok((storage, passphrase));
                }
                Err(err) if is_incorrect_passphrase_error(&err) => {
                    let _ = cache_clear(&config.socket_path);
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    // Device keyfile: no passphrase needed
    if matches!(security.tier, SecurityTier::DeviceKeyfile) {
        return open_with_device_keyfile(cli, target_path, &security, cache_config.as_ref());
    }

    // Passphrase keyfile: decrypt keyfile with passphrase
    if matches!(security.tier, SecurityTier::PassphraseKeyfile) {
        return open_with_passphrase_keyfile(
            cli,
            target_path,
            &security,
            interactive,
            cache_config.as_ref(),
        );
    }

    // Passphrase + keychain: try keychain first
    if matches!(security.tier, SecurityTier::PassphraseKeychain) && security.keychain_enabled {
        if let Some(result) = try_keychain_passphrase(target_path) {
            return Ok(result);
        }
    }

    // Try environment variable
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

    // Prompt for passphrase
    let (storage, passphrase) =
        open_with_retry_prompt(cli, target_path, interactive, cache_config.as_ref())?;
    if matches!(security.tier, SecurityTier::PassphraseKeychain) && security.keychain_enabled {
        let account = ledger_hash(target_path);
        let _ = keychain_set(&account, &passphrase);
    }
    Ok((storage, passphrase))
}

fn open_with_device_keyfile(
    cli: &Cli,
    target_path: &Path,
    security: &SecurityConfig,
    cache_config: Option<&CacheConfig>,
) -> anyhow::Result<(AgeSqliteStorage, String)> {
    let keyfile_path = security
        .keyfile_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Keyfile path is required for device_keyfile"))?;
    let key_bytes = read_keyfile_plain(keyfile_path)?;
    let passphrase = key_bytes_to_passphrase(&key_bytes);
    open_with_passphrase_and_cache(cli, target_path, &passphrase, cache_config)
}

fn open_with_passphrase_keyfile(
    cli: &Cli,
    target_path: &Path,
    security: &SecurityConfig,
    interactive: bool,
    cache_config: Option<&CacheConfig>,
) -> anyhow::Result<(AgeSqliteStorage, String)> {
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
    open_with_passphrase_and_cache(cli, target_path, &passphrase, cache_config)
}

fn try_keychain_passphrase(target_path: &Path) -> Option<(AgeSqliteStorage, String)> {
    let account = ledger_hash(target_path);
    match keychain_get(&account) {
        Ok(Some(passphrase)) => {
            if let Ok(storage) = AgeSqliteStorage::open(target_path, &passphrase) {
                return Some((storage, passphrase));
            }
            let _ = keychain_clear(&account);
            None
        }
        Ok(None) => None,
        Err(err) => {
            eprintln!("Warning: {}", err);
            None
        }
    }
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
            CliError::auth_failed("Incorrect passphrase.").exit()
        }
        Err(err) if is_missing_ledger_error(&err) => {
            Err(anyhow::anyhow!(missing_ledger_message(path)))
        }
        Err(err) => Err(err.into()),
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
                    CliError::auth_failed_with_hint(
                        "Too many failed passphrase attempts.",
                        "Hint: If you forgot your passphrase, the ledger cannot be recovered.\n      Backups use the same passphrase.",
                    )
                    .exit()
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
            Err(err) => return Err(err.into()),
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
                CliError::auth_failed("Incorrect passphrase.").exit()
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
                    CliError::auth_failed_with_hint(
                        "Too many failed passphrase attempts.",
                        "Hint: If you forgot your passphrase, the ledger cannot be recovered.\n      Backups use the same passphrase.",
                    )
                    .exit()
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
    matches!(err, ledger_core::error::LedgerError::IncorrectPassphrase)
}

fn is_missing_ledger_error(err: &ledger_core::error::LedgerError) -> bool {
    matches!(err, ledger_core::error::LedgerError::LedgerNotFound)
}
