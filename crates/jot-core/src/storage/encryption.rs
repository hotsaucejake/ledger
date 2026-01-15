//! Age encryption/decryption utilities.
//!
//! This module provides wrappers around the Age encryption library for
//! encrypting and decrypting ledger data using passphrase-based encryption.
//!
//! Note: Age uses scrypt internally for passphrase-based encryption.
//! While RFC-001 specified Argon2id, we use Age's built-in passphrase support
//! for simplicity and correctness. The security properties are similar.

use std::io::{Read, Write};
use std::iter;

use age::secrecy::SecretString;

use crate::error::{JotError, Result};

/// Encrypt data using Age passphrase-based encryption.
///
/// # Arguments
///
/// * `data` - The plaintext data to encrypt
/// * `passphrase` - The passphrase for encryption
///
/// # Returns
///
/// Returns encrypted bytes suitable for writing to disk.
///
/// # Security
///
/// Uses Age's built-in passphrase encryption with scrypt KDF.
///
/// # Examples
///
/// ```
/// use jot_core::storage::encryption::encrypt;
///
/// let plaintext = b"secret data";
/// let encrypted = encrypt(plaintext, "my-secure-passphrase").unwrap();
/// assert_ne!(encrypted.as_slice(), plaintext);
/// ```
pub fn encrypt(data: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let encryptor =
        age::Encryptor::with_user_passphrase(SecretString::from(passphrase.to_string()));

    let mut encrypted = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .map_err(|e| JotError::Crypto(format!("Failed to create encryptor: {}", e)))?;

    writer
        .write_all(data)
        .map_err(|e| JotError::Crypto(format!("Encryption write failed: {}", e)))?;

    writer
        .finish()
        .map_err(|e| JotError::Crypto(format!("Encryption finish failed: {}", e)))?;

    Ok(encrypted)
}

/// Decrypt data using Age passphrase-based encryption.
///
/// # Arguments
///
/// * `encrypted_data` - The encrypted data to decrypt
/// * `passphrase` - The passphrase for decryption
///
/// # Returns
///
/// Returns the decrypted plaintext data.
///
/// # Errors
///
/// Returns `JotError::Crypto` if:
/// - The passphrase is incorrect
/// - The data is corrupted
/// - Decryption fails for any reason
///
/// # Examples
///
/// ```
/// use jot_core::storage::encryption::{encrypt, decrypt};
///
/// let plaintext = b"secret data";
/// let encrypted = encrypt(plaintext, "my-secure-passphrase").unwrap();
/// let decrypted = decrypt(&encrypted, "my-secure-passphrase").unwrap();
/// assert_eq!(decrypted.as_slice(), plaintext);
/// ```
pub fn decrypt(encrypted_data: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let decryptor = age::Decryptor::new(encrypted_data)
        .map_err(|e| JotError::Crypto(format!("Failed to create decryptor: {}", e)))?;

    let mut decrypted = Vec::new();

    let identity = age::scrypt::Identity::new(SecretString::from(passphrase.to_string()));
    let mut reader = decryptor
        .decrypt(iter::once(&identity as &dyn age::Identity))
        .map_err(|e| match e {
            age::DecryptError::NoMatchingKeys
            | age::DecryptError::DecryptionFailed
            | age::DecryptError::KeyDecryptionFailed => JotError::IncorrectPassphrase,
            _ => JotError::Crypto(format!("Decryption failed: {}", e)),
        })?;

    reader
        .read_to_end(&mut decrypted)
        .map_err(|e| JotError::Crypto(format!("Failed to read decrypted data: {}", e)))?;

    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_round_trip() {
        let passphrase = "test-passphrase-secure-123";
        let plaintext = b"Hello, World! This is secret data.";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        let decrypted = decrypt(&encrypted, passphrase).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypted_data_different_from_plaintext() {
        let passphrase = "test-passphrase-secure-123";
        let plaintext = b"secret data";

        let encrypted = encrypt(plaintext, passphrase).unwrap();

        // Encrypted data should be different
        assert_ne!(encrypted.as_slice(), plaintext);
        // And non-empty
        assert!(!encrypted.is_empty());
    }

    #[test]
    fn test_wrong_passphrase_fails_decryption() {
        let passphrase1 = "correct-passphrase-123";
        let passphrase2 = "wrong-passphrase-456";

        let plaintext = b"secret data";
        let encrypted = encrypt(plaintext, passphrase1).unwrap();

        // Attempting to decrypt with wrong passphrase should fail
        let result = decrypt(&encrypted, passphrase2);
        assert!(matches!(result, Err(JotError::IncorrectPassphrase)));
    }

    #[test]
    fn test_corrupted_data_fails_decryption() {
        let passphrase = "test-passphrase-secure-123";
        let plaintext = b"secret data";

        let mut encrypted = encrypt(plaintext, passphrase).unwrap();

        // Corrupt the data
        let len = encrypted.len();
        if len > 0 {
            encrypted[len / 2] ^= 0xFF;
        }

        // Should fail to decrypt
        let result = decrypt(&encrypted, passphrase);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_data_encryption() {
        let passphrase = "test-passphrase-secure-123";
        let plaintext = b"";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        let decrypted = decrypt(&encrypted, passphrase).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_large_data_encryption() {
        let passphrase = "test-passphrase-secure-123";
        // 1MB of data
        let plaintext = vec![0x42u8; 1024 * 1024];

        let encrypted = encrypt(&plaintext, passphrase).unwrap();
        let decrypted = decrypt(&encrypted, passphrase).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_passphrases_different_ciphertext() {
        let passphrase1 = "passphrase-one-secure-123";
        let passphrase2 = "passphrase-two-secure-456";
        let plaintext = b"same plaintext";

        let encrypted1 = encrypt(plaintext, passphrase1).unwrap();
        let encrypted2 = encrypt(plaintext, passphrase2).unwrap();

        // Different passphrases should produce different ciphertext
        assert_ne!(encrypted1, encrypted2);
    }
}
