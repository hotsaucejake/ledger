//! Security configuration loading and validation.

use std::path::PathBuf;

use crate::cli::Cli;
use crate::config::{default_keyfile_path, read_config, KeyfileMode, SecurityTier};

use super::resolver::resolve_config_path;

/// Runtime security configuration loaded from config file.
pub struct SecurityConfig {
    pub tier: SecurityTier,
    pub keychain_enabled: bool,
    pub keyfile_mode: KeyfileMode,
    pub keyfile_path: Option<PathBuf>,
    pub cache_ttl_seconds: u64,
    pub editor: Option<String>,
}

/// Load security configuration from the config file.
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

/// Validate that security config has required fields for the tier.
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

/// Warning message for device keyfile security tier.
pub fn device_keyfile_warning() -> &'static str {
    "WARNING: You selected device_keyfile. This stores an unencrypted key on disk.\nIf your device is compromised, your ledger can be decrypted without a passphrase.\nContinue?"
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
