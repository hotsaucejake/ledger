//! Application-level utilities for the Ledger CLI.
//!
//! This module provides:
//! - Application context for unified CLI + config handling
//! - Path resolution for config and jot files
//! - Security configuration loading
//! - Passphrase handling with retry logic

mod context;
mod passphrase;
mod resolver;
mod security_config;

// Re-export public API
pub use context::AppContext;
pub use resolver::{
    exit_not_found_with_hint, missing_config_message, missing_ledger_message, resolve_config_path,
    resolve_ledger_path,
};
pub use security_config::device_keyfile_warning;
