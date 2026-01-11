//! Application-level utilities for the Ledger CLI.
//!
//! This module provides:
//! - Path resolution for config and ledger files
//! - Security configuration loading
//! - Passphrase handling with retry logic

mod passphrase;
mod resolver;
mod security_config;

// Re-export public API
pub use passphrase::open_storage_with_retry;
pub use resolver::{
    exit_not_found_with_hint, missing_config_message, missing_ledger_message, resolve_config_path,
    resolve_ledger_path,
};
pub use security_config::{device_keyfile_warning, load_security_config};
