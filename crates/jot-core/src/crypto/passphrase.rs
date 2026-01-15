//! Passphrase validation.
//!
//! Enforces minimum security requirements for passphrases.

use crate::error::{JotError, Result};

/// Minimum passphrase length in characters.
const MIN_PASSPHRASE_LENGTH: usize = 8;

/// Validate passphrase meets minimum security requirements.
///
/// # Requirements
///
/// - At least 8 characters long
/// - Not empty or only whitespace
///
/// # Arguments
///
/// * `passphrase` - The passphrase to validate
///
/// # Returns
///
/// Returns `Ok(())` if valid, or `JotError::InvalidInput` with explanation.
///
/// # Examples
///
/// ```
/// use jot_core::crypto::validate_passphrase;
///
/// assert!(validate_passphrase("my-secure-passphrase-123").is_ok());
/// assert!(validate_passphrase("short").is_err());
/// ```
pub fn validate_passphrase(passphrase: &str) -> Result<()> {
    // Check empty/whitespace
    if passphrase.trim().is_empty() {
        return Err(JotError::InvalidInput(
            "Passphrase cannot be empty".to_string(),
        ));
    }

    // Check minimum length
    if passphrase.len() < MIN_PASSPHRASE_LENGTH {
        return Err(JotError::InvalidInput(format!(
            "Passphrase must be at least {} characters (got {})",
            MIN_PASSPHRASE_LENGTH,
            passphrase.len()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_passphrase() {
        assert!(validate_passphrase("my-secure-passphrase-123").is_ok());
        assert!(validate_passphrase("exactly12chr").is_ok());
        assert!(validate_passphrase("longer passphrase with spaces and symbols!@#").is_ok());
    }

    #[test]
    fn test_passphrase_too_short() {
        let result = validate_passphrase("short");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least 8 characters"));
    }

    #[test]
    fn test_passphrase_empty() {
        assert!(validate_passphrase("").is_err());
        assert!(validate_passphrase("   ").is_err());
        assert!(validate_passphrase("\n\t").is_err());
    }

    #[test]
    fn test_passphrase_exactly_min_length() {
        // Exactly 8 characters should pass
        let exactly_8 = "12345678";
        assert_eq!(exactly_8.len(), 8);
        assert!(validate_passphrase(exactly_8).is_ok());
    }
}
