use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LedgerConfig {
    pub ledger: LedgerSection,
    pub security: SecuritySection,
    pub keychain: KeychainSection,
    pub keyfile: KeyfileSection,
    #[serde(default)]
    pub ui: UiSection,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LedgerSection {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecuritySection {
    pub tier: SecurityTier,
    pub passphrase_cache_ttl_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeychainSection {
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyfileSection {
    pub mode: KeyfileMode,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct UiSection {
    pub timezone: Option<String>,
    pub editor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum SecurityTier {
    Passphrase,
    PassphraseKeychain,
    PassphraseKeyfile,
    DeviceKeyfile,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum KeyfileMode {
    None,
    Encrypted,
    Plain,
}

impl LedgerConfig {
    pub fn new(
        ledger_path: PathBuf,
        tier: SecurityTier,
        passphrase_cache_ttl_seconds: u64,
        keyfile_mode: KeyfileMode,
        keyfile_path: Option<PathBuf>,
        timezone: Option<String>,
        editor: Option<String>,
    ) -> Self {
        Self {
            ledger: LedgerSection {
                path: ledger_path.to_string_lossy().to_string(),
            },
            security: SecuritySection {
                tier,
                passphrase_cache_ttl_seconds,
            },
            keychain: KeychainSection {
                enabled: matches!(tier, SecurityTier::PassphraseKeychain),
            },
            keyfile: KeyfileSection {
                mode: keyfile_mode,
                path: keyfile_path.map(|path| path.to_string_lossy().to_string()),
            },
            ui: UiSection { timezone, editor },
        }
    }
}

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    Ok(xdg_config_dir()?.join("config.toml"))
}

pub fn default_ledger_path() -> anyhow::Result<PathBuf> {
    Ok(xdg_data_dir()?.join("ledger.ledger"))
}

pub fn default_keyfile_path() -> anyhow::Result<PathBuf> {
    Ok(xdg_config_dir()?.join("ledger.key"))
}

pub fn read_config(path: &Path) -> anyhow::Result<LedgerConfig> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read config {}: {}", path.display(), e))?;
    toml::from_str(&contents)
        .map_err(|e| anyhow::anyhow!("Failed to parse config {}: {}", path.display(), e))
}

pub fn write_config(path: &Path, config: &LedgerConfig) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!(
                "Failed to create config directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }
    let contents =
        toml::to_string_pretty(config).map_err(|e| anyhow::anyhow!("TOML error: {}", e))?;
    std::fs::write(path, contents)
        .map_err(|e| anyhow::anyhow!("Failed to write config {}: {}", path.display(), e))?;
    Ok(())
}

pub fn xdg_config_dir() -> anyhow::Result<PathBuf> {
    if let Ok(value) = std::env::var("XDG_CONFIG_HOME") {
        if !value.trim().is_empty() {
            return Ok(PathBuf::from(value).join("ledger"));
        }
    }
    Ok(home_dir()?.join(".config").join("ledger"))
}

pub fn xdg_data_dir() -> anyhow::Result<PathBuf> {
    if let Ok(value) = std::env::var("XDG_DATA_HOME") {
        if !value.trim().is_empty() {
            return Ok(PathBuf::from(value).join("ledger"));
        }
    }
    Ok(home_dir()?.join(".local").join("share").join("ledger"))
}

fn home_dir() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("HOME is not set; cannot resolve default paths"))?;
    Ok(PathBuf::from(home))
}
