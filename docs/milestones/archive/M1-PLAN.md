# Milestone 1: Encrypted Storage - Implementation Plan

**Status**: In Progress
**Started**: 2026-01-08
**Target**: Complete encryption layer and basic ledger operations

## Goals

Implement Age-encrypted SQLite storage with in-memory operation, enabling:
- Creation of new encrypted ledgers
- Opening and closing ledgers securely
- Storing and retrieving test data
- Verification that encryption works correctly

## Exit Criteria

- [x] Can create new encrypted ledger
- [x] Can open existing ledger
- [x] Store and retrieve test data
- [x] Verify encryption at rest (no plaintext)
- [x] Tests pass on Linux
- [x] Tests pass on macOS (CI)

## Implementation Steps

### 1. Storage Abstraction (TDD)

**Files**: `crates/ledger-core/src/storage/`

- [x] Define `StorageEngine` trait
- [x] Define core types (`NewEntry`, `Entry`, `EntryFilter`)
- [x] Write unit tests for trait contract
- [x] Add integration test stubs

**Tests First:**
```rust
#[test]
fn test_storage_trait_contract() { /* ... */ }
```

### 2. Crypto Module (TDD)

**Files**: `crates/ledger-core/src/crypto/`

- [x] Key derivation with Argon2id
- [x] Age encryption/decryption wrappers
- [x] Passphrase validation
- [x] Key zeroization on drop

**Tests First:**
```rust
#[test]
fn test_key_derivation_deterministic() { /* ... */ }

#[test]
fn test_encryption_round_trip() { /* ... */ }

#[test]
fn test_wrong_passphrase_fails() { /* ... */ }
```

**Security Checklist:**
- [x] Use `zeroize` crate for sensitive data
- [x] Argon2id parameters match RFC-001
- [x] Age library used correctly
- [x] No passphrase stored in memory longer than needed

### 3. Age-SQLite Backend (TDD)

**Files**: `crates/ledger-core/src/storage/age_sqlite.rs`

- [x] Implement `StorageEngine` for `AgeSqliteStorage`
- [x] Schema creation (meta, entry_types, entries, FTS)
- [x] In-memory SQLite with `deserialize`
- [x] Encrypt-on-close, decrypt-on-open
- [x] Atomic writes with backup

**Tests First:**
```rust
#[test]
fn test_create_new_ledger() { /* ... */ }

#[test]
fn test_open_existing_ledger() { /* ... */ }

#[test]
fn test_round_trip_entry() { /* ... */ }

#[test]
fn test_file_is_encrypted() { /* ... */ }
```

**Implementation Notes:**
- Use `rusqlite::deserialize` for in-memory operation
- Write to temp file, then atomic rename
- Verify no plaintext on disk

### 4. CLI Integration

**Files**: `crates/ledger-cli/src/commands/`

- [x] `init.rs` - Create new ledger
- [x] Passphrase prompting (dialoguer)
- [ ] Error message translation
- [ ] Progress indicators

**Tests First:**
```rust
#[test]
fn test_init_creates_encrypted_file() { /* ... */ }

#[test]
fn test_init_rejects_weak_passphrase() { /* ... */ }
```

### 5. Integration Testing

**Files**: `tests/integration/`

- [x] Full init → close → open → verify workflow
- [x] Wrong passphrase rejection
- [x] Corrupted file detection
- [x] Temp file cleanup verification

### 6. Documentation

- [x] Update README.md with M1 completion status
- [x] Add crypto usage examples to docs/DEVELOPMENT.md
- [x] Document passphrase requirements

## Dependencies to Add

```toml
[dependencies]
# Crypto (uncomment in Cargo.toml)
age = { workspace = true }
argon2 = { workspace = true }
zeroize = "1.7"

# Database (uncomment in Cargo.toml)
rusqlite = { workspace = true }

# CLI prompts
dialoguer = "0.11"
```

## Security Considerations

From RFC-001 threat model:

**We defend against:**
- Theft of ledger file
- Access while app is closed
- Offline brute-force attacks

**Mitigations:**
- Age encryption with Argon2id (memory-hard KDF)
- In-memory SQLite (no plaintext temp files)
- Atomic writes (crash safety)
- Key zeroization

**Testing:**
- [x] Verify file is encrypted (cannot read with `cat`)
- [x] Verify wrong passphrase fails gracefully
- [x] Verify no temp files remain after crash
- [x] Verify memory cleanup (manual inspection)

## Performance Targets

Phase 0.1 targets personal use:
- Ledger size: < 100MB typical
- Open time: < 500ms
- Close time: < 1s
- No optimization needed yet (measure first!)

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Age library API changes | High | Pin version, test thoroughly |
| SQLite deserialize unavailable | High | Fallback to temp file (RFC-001 §7.2) |
| Argon2id too slow/fast | Medium | Tunable parameters, test on target hardware |
| Memory leaks with sensitive data | High | Use zeroize, audit carefully |

## Definition of Done

M1 is complete when:
- [x] All tests pass (`cargo test`)
- [x] No clippy warnings
- [x] `ledger init ~/test.ledger` works
- [x] File is encrypted (verify manually)
- [x] Can reopen and verify structure
- [x] CI passes on Linux + macOS
- [x] No plaintext leaks verified

## Notes

- Follow TDD strictly: write tests first
- Each commit should compile and pass tests
- Reference RFC-001 for all crypto decisions
- Use `.claude/CHECKLIST.md` for every module (if applicable)
- Update this plan as we learn

## Resources

- RFC-001: Storage & Encryption Model
- RFC-004: Data Model
- Age documentation: https://docs.rs/age/
- Argon2 documentation: https://docs.rs/argon2/
- Rusqlite documentation: https://docs.rs/rusqlite/
