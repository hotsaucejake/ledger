//! Cryptographic operations for Jot.
//!
//! This module provides encryption and key derivation services using
//! well-audited libraries:
//! - **Age**: Modern, simple encryption (https://age-encryption.org/)
//! - **Argon2id**: Memory-hard key derivation function
//!
//! ## Security Model
//!
//! Per RFC-001:
//! - Passphrase-based encryption using Age
//! - Argon2id for key derivation (memory-hard, resistant to brute-force)
//! - Sensitive data zeroized from memory on drop
//! - No plaintext passphrases stored
//!
//! ## Threat Model
//!
//! We defend against:
//! - Theft of encrypted jot file
//! - Offline brute-force attacks on passphrase
//!
//! We do NOT defend against:
//! - Compromised OS / keylogger
//! - Access to unlocked session / memory

pub mod key;
pub mod passphrase;

pub use key::{derive_key, DerivedKey};
pub use passphrase::validate_passphrase;
