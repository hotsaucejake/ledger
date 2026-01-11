//! Application context for the Ledger CLI.
//!
//! Provides a unified context that combines CLI arguments with
//! lazily-loaded security configuration.

use once_cell::unsync::OnceCell;

use ledger_core::storage::AgeSqliteStorage;

use crate::cli::Cli;

use super::passphrase::open_storage_with_retry;
use super::security_config::{load_security_config, SecurityConfig};

/// Application context that bundles CLI args with security configuration.
///
/// This avoids repeatedly loading config and threading multiple parameters
/// through handler functions.
pub struct AppContext<'a> {
    cli: &'a Cli,
    security_config: OnceCell<SecurityConfig>,
}

impl<'a> AppContext<'a> {
    /// Create a new application context from CLI arguments.
    pub fn new(cli: &'a Cli) -> Self {
        Self {
            cli,
            security_config: OnceCell::new(),
        }
    }

    /// Get the CLI arguments.
    pub fn cli(&self) -> &Cli {
        self.cli
    }

    /// Check if quiet mode is enabled.
    pub fn quiet(&self) -> bool {
        self.cli.quiet
    }

    /// Get the security configuration, loading it lazily if needed.
    pub fn security_config(&self) -> anyhow::Result<&SecurityConfig> {
        self.security_config
            .get_or_try_init(|| load_security_config(self.cli))
    }

    /// Get the configured editor override, if any.
    pub fn editor(&self) -> anyhow::Result<Option<&str>> {
        Ok(self.security_config()?.editor.as_deref())
    }

    /// Open storage with passphrase handling and retry logic.
    ///
    /// This is a convenience method that delegates to the underlying
    /// `open_storage_with_retry` function.
    pub fn open_storage(&self, no_input: bool) -> anyhow::Result<(AgeSqliteStorage, String)> {
        open_storage_with_retry(self.cli, no_input)
    }
}
