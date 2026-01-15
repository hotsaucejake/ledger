//! CLI error types for structured error handling.
//!
//! This module provides typed errors that map to specific exit codes,
//! enabling consistent error handling across the CLI.

use std::fmt;

/// CLI-specific errors with associated exit codes.
#[derive(Debug)]
pub enum CliError {
    /// Resource not found (config, jot, entry, etc.)
    NotFound { message: String, hint: String },

    /// Authentication failed (wrong passphrase, too many attempts)
    AuthFailed {
        message: String,
        hint: Option<String>,
    },

    /// Invalid user input
    #[allow(dead_code)]
    InvalidInput(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::NotFound { message, hint } => {
                write!(f, "{}\n{}", message, hint)
            }
            CliError::AuthFailed { message, hint } => {
                if let Some(h) = hint {
                    write!(f, "{}\n{}", message, h)
                } else {
                    write!(f, "{}", message)
                }
            }
            CliError::InvalidInput(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for CliError {}

impl CliError {
    /// Create a NotFound error with message and hint.
    pub fn not_found(message: impl Into<String>, hint: impl Into<String>) -> Self {
        CliError::NotFound {
            message: message.into(),
            hint: hint.into(),
        }
    }

    /// Create an AuthFailed error with message and optional hint.
    pub fn auth_failed(message: impl Into<String>) -> Self {
        CliError::AuthFailed {
            message: message.into(),
            hint: None,
        }
    }

    /// Create an AuthFailed error with message and hint.
    pub fn auth_failed_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        CliError::AuthFailed {
            message: message.into(),
            hint: Some(hint.into()),
        }
    }

    /// Create an InvalidInput error.
    #[allow(dead_code)]
    pub fn invalid_input(message: impl Into<String>) -> Self {
        CliError::InvalidInput(message.into())
    }

    /// Get the exit code for this error.
    pub fn exit_code(&self) -> i32 {
        use super::constants::exit_codes;
        match self {
            CliError::NotFound { .. } => exit_codes::NOT_FOUND,
            CliError::AuthFailed { .. } => exit_codes::AUTH_FAILED,
            CliError::InvalidInput(_) => exit_codes::INVALID_INPUT,
        }
    }

    /// Print error message to stderr and exit with appropriate code.
    pub fn exit(&self) -> ! {
        eprintln!("Error: {}", self);
        std::process::exit(self.exit_code())
    }
}
