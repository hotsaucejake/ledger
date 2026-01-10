use std::path::Path;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use zeroize::Zeroizing;

use ledger_core::storage::encryption::{decrypt, encrypt};

pub fn generate_key_bytes() -> anyhow::Result<[u8; 32]> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| anyhow::anyhow!("Failed to generate key bytes: {}", e))?;
    Ok(bytes)
}

pub fn key_bytes_to_passphrase(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

pub fn write_keyfile_encrypted(
    path: &Path,
    key_bytes: &[u8],
    passphrase: &str,
) -> anyhow::Result<()> {
    ensure_parent_dir(path)?;
    if path.exists() {
        return Err(anyhow::anyhow!(
            "Keyfile already exists: {}",
            path.display()
        ));
    }
    let encrypted = encrypt(key_bytes, passphrase).map_err(|e| anyhow::anyhow!(e))?;
    std::fs::write(path, encrypted)
        .map_err(|e| anyhow::anyhow!("Failed to write keyfile {}: {}", path.display(), e))?;
    set_file_permissions(path)?;
    Ok(())
}

pub fn write_keyfile_plain(path: &Path, key_bytes: &[u8]) -> anyhow::Result<()> {
    ensure_parent_dir(path)?;
    if path.exists() {
        return Err(anyhow::anyhow!(
            "Keyfile already exists: {}",
            path.display()
        ));
    }
    std::fs::write(path, key_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to write keyfile {}: {}", path.display(), e))?;
    set_file_permissions(path)?;
    Ok(())
}

pub fn read_keyfile_plain(path: &Path) -> anyhow::Result<Zeroizing<Vec<u8>>> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("Failed to read keyfile {}: {}", path.display(), e))?;
    Ok(Zeroizing::new(bytes))
}

pub fn read_keyfile_encrypted(path: &Path, passphrase: &str) -> anyhow::Result<Zeroizing<Vec<u8>>> {
    let encrypted = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("Failed to read keyfile {}: {}", path.display(), e))?;
    let decrypted = decrypt(&encrypted, passphrase).map_err(|e| anyhow::anyhow!(e))?;
    Ok(Zeroizing::new(decrypted))
}

pub fn keychain_get(account: &str) -> anyhow::Result<Option<String>> {
    let entry = keychain_entry(account)?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(anyhow::anyhow!("Keychain read failed: {}", err)),
    }
}

pub fn keychain_set(account: &str, passphrase: &str) -> anyhow::Result<()> {
    let entry = keychain_entry(account)?;
    entry
        .set_password(passphrase)
        .map_err(|e| anyhow::anyhow!("Keychain write failed: {}", e))
}

pub fn keychain_clear(account: &str) -> anyhow::Result<()> {
    let entry = keychain_entry(account)?;
    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(anyhow::anyhow!("Keychain delete failed: {}", err)),
    }
}

fn keychain_entry(account: &str) -> anyhow::Result<keyring::Entry> {
    keyring::Entry::new("ledger", account)
        .map_err(|e| anyhow::anyhow!("Keychain entry failed: {}", e))
}

fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!(
                "Failed to create keyfile directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    Ok(())
}

fn set_file_permissions(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}
