//! Key derivation using Argon2id.
//!
//! This module derives encryption keys from passphrases using the Argon2id
//! algorithm, which is memory-hard and resistant to GPU-based attacks.

use argon2::Argon2;
use zeroize::ZeroizeOnDrop;

use crate::error::{JotError, Result};

/// Argon2id parameters (per RFC-001).
///
/// These values balance security and usability:
/// - Memory: 64 MB (64 * 1024 KB)
/// - Iterations: 3
/// - Parallelism: 1 (single-threaded for simplicity)
const ARGON2_MEMORY_KB: u32 = 64 * 1024;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 1;

/// Length of derived key in bytes (32 bytes = 256 bits for Age).
const KEY_LENGTH: usize = 32;

/// A cryptographic key derived from a passphrase.
///
/// This type ensures that key material is securely zeroized from memory
/// when dropped, reducing the window of exposure.
#[derive(Clone, ZeroizeOnDrop)]
pub struct DerivedKey {
    /// The raw key bytes (zeroized on drop)
    key: [u8; KEY_LENGTH],
}

impl DerivedKey {
    /// Create a new DerivedKey from raw bytes.
    ///
    /// # Security
    ///
    /// The caller is responsible for ensuring the bytes come from a secure source.
    pub(crate) fn from_bytes(bytes: [u8; KEY_LENGTH]) -> Self {
        Self { key: bytes }
    }

    /// Get a reference to the raw key bytes.
    ///
    /// # Security
    ///
    /// Avoid storing or logging this value. Use only for immediate encryption operations.
    pub fn as_bytes(&self) -> &[u8; KEY_LENGTH] {
        &self.key
    }
}

impl std::fmt::Debug for DerivedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DerivedKey")
            .field("key", &"[REDACTED]")
            .finish()
    }
}

/// Derive an encryption key from a passphrase using Argon2id.
///
/// This function uses a memory-hard key derivation function to make
/// brute-force attacks computationally expensive.
///
/// # Arguments
///
/// * `passphrase` - The passphrase to derive from
/// * `salt` - Random salt (must be unique per jot)
///
/// # Returns
///
/// Returns a `DerivedKey` suitable for encryption operations.
///
/// # Security
///
/// - Same passphrase + salt always produces same key (deterministic)
/// - Different salt produces different key (salt must be stored with jot)
/// - Memory-hard: requires ~64MB RAM, resistant to GPU attacks
///
/// # Examples
///
/// ```
/// use jot_core::crypto::derive_key;
///
/// let salt = b"unique-salt-per-jot-16bytes";
/// let key = derive_key("my-passphrase", salt).unwrap();
/// // Use key for encryption...
/// ```
pub fn derive_key(passphrase: &str, salt: &[u8]) -> Result<DerivedKey> {
    // Validate inputs
    if passphrase.is_empty() {
        return Err(JotError::InvalidInput(
            "Passphrase cannot be empty".to_string(),
        ));
    }

    if salt.len() < 16 {
        return Err(JotError::InvalidInput(
            "Salt must be at least 16 bytes".to_string(),
        ));
    }

    // Configure Argon2id
    let params = argon2::Params::new(
        ARGON2_MEMORY_KB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(KEY_LENGTH),
    )
    .map_err(|e| JotError::Crypto(format!("Failed to create Argon2 params: {}", e)))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    // Derive key
    let mut key_bytes = [0u8; KEY_LENGTH];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key_bytes)
        .map_err(|e| JotError::Crypto(format!("Key derivation failed: {}", e)))?;

    Ok(DerivedKey::from_bytes(key_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_derivation_deterministic() {
        let passphrase = "test-passphrase";
        let salt = b"unique-salt-1234567890123456";

        let key1 = derive_key(passphrase, salt).unwrap();
        let key2 = derive_key(passphrase, salt).unwrap();

        // Same passphrase + salt should produce identical keys
        assert_eq!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_different_salt_different_key() {
        let passphrase = "test-passphrase";
        let salt1 = b"salt1-1234567890123456";
        let salt2 = b"salt2-1234567890123456";

        let key1 = derive_key(passphrase, salt1).unwrap();
        let key2 = derive_key(passphrase, salt2).unwrap();

        // Different salts should produce different keys
        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_different_passphrase_different_key() {
        let salt = b"fixed-salt-123456789012345";
        let pass1 = "passphrase-one";
        let pass2 = "passphrase-two";

        let key1 = derive_key(pass1, salt).unwrap();
        let key2 = derive_key(pass2, salt).unwrap();

        // Different passphrases should produce different keys
        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_empty_passphrase_rejected() {
        let salt = b"salt-1234567890123456";
        let result = derive_key("", salt);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Passphrase cannot be empty"));
    }

    #[test]
    fn test_short_salt_rejected() {
        let passphrase = "test-passphrase";
        let short_salt = b"short"; // Less than 16 bytes

        let result = derive_key(passphrase, short_salt);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Salt must be at least 16 bytes"));
    }

    #[test]
    fn test_key_length() {
        let passphrase = "test-passphrase";
        let salt = b"salt-1234567890123456";

        let key = derive_key(passphrase, salt).unwrap();
        assert_eq!(key.as_bytes().len(), KEY_LENGTH);
    }

    #[test]
    fn test_derived_key_debug_redacts() {
        let passphrase = "test-passphrase";
        let salt = b"salt-1234567890123456";
        let key = derive_key(passphrase, salt).unwrap();

        let debug_output = format!("{:?}", key);
        // Should contain REDACTED
        assert!(debug_output.contains("REDACTED"));

        // Should NOT contain actual key bytes
        // Convert first few bytes to hex and ensure they don't appear
        let key_hex = hex::encode(&key.as_bytes()[..4]);
        assert!(!debug_output.contains(&key_hex));
    }
}
