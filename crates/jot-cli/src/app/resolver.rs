//! Path resolution for config and jot files.

use std::path::{Path, PathBuf};

use crate::cli::Cli;
use crate::config::{default_config_path, read_config};
use crate::errors::CliError;

/// Resolve the config file path, checking JOT_CONFIG env var first.
pub fn resolve_config_path() -> anyhow::Result<PathBuf> {
    if let Ok(value) = std::env::var("JOT_CONFIG") {
        if !value.trim().is_empty() {
            return Ok(PathBuf::from(value));
        }
    }
    default_config_path()
}

/// Resolve the jot file path from CLI args or config.
pub fn resolve_jot_path(cli: &Cli) -> anyhow::Result<String> {
    if let Some(path) = cli.jot.clone() {
        return Ok(path);
    }

    let config_path = resolve_config_path()?;
    if !config_path.exists() {
        return Err(anyhow::anyhow!(missing_config_message(&config_path)));
    }

    let config = read_config(&config_path)?;
    Ok(config.jot.path)
}

/// Error message when jot file is missing.
pub fn missing_jot_message(path: &Path) -> String {
    format!(
        "Jot file not found: {}\n\nRun:\n  jot init\n\nOr specify a different path:\n  jot --jot /path/to/my.jot init",
        path.display()
    )
}

/// Error message when config file is missing.
pub fn missing_config_message(config_path: &Path) -> String {
    format!(
        "Config file not found: {}\n\nRun:\n  jot init\n\nOr set JOT_CONFIG to specify a different config location.",
        config_path.display()
    )
}

/// Exit with error code for not found errors.
///
/// This function prints the error and exits immediately.
/// Use `CliError::not_found` if you need to return an error instead.
pub fn exit_not_found_with_hint(message: &str, hint: &str) -> ! {
    CliError::not_found(message, hint).exit()
}
