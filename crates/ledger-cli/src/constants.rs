//! Constants used throughout the CLI.

/// Exit codes for the CLI.
///
/// These follow common Unix conventions:
/// - 0: Success
/// - 1: General error (used by anyhow for unhandled errors)
/// - 2: Misuse of shell command (reserved by shells)
/// - 3+: Application-specific errors
pub mod exit_codes {
    /// Resource not found (config, ledger, entry, entry type).
    pub const NOT_FOUND: i32 = 3;

    /// Invalid user input or arguments.
    pub const INVALID_INPUT: i32 = 4;

    /// Authentication failed (wrong passphrase, too many attempts).
    pub const AUTH_FAILED: i32 = 5;

    /// Integrity check failed.
    #[allow(dead_code)]
    pub const INTEGRITY_FAILED: i32 = 6;
}
