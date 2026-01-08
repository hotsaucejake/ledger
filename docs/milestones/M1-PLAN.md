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
- [ ] Can open existing ledger
- [ ] Store and retrieve test data
- [ ] Verify encryption at rest (no plaintext)
- [ ] Tests pass on Linux
- [ ] Tests pass on macOS (CI)

## Implementation Steps

### 1. Storage Abstraction (TDD)

**Files**: `crates/ledger-core/src/storage/`

- [ ] Define `StorageEngine` trait
- [ ] Define core types (`NewEntry`, `Entry`, `EntryFilter`)
- [ ] Write unit tests for trait contract
- [ ] Add integration test stubs

**Tests First:**
```rust
#[test]
fn test_storage_trait_contract() { /* ... */ }
```

### 2. Crypto Module (TDD)

**Files**: `crates/ledger-core/src/crypto/`

- [ ] Key derivation with Argon2id
- [ ] Age encryption/decryption wrappers
- [ ] Passphrase validation
- [ ] Key zeroization on drop

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
- [ ] Use `zeroize` crate for sensitive data
- [ ] Argon2id parameters match RFC-001
- [ ] Age library used correctly
- [ ] No passphrase stored in memory longer than needed

### 3. Age-SQLite Backend (TDD)

**Files**: `crates/ledger-core/src/storage/age_sqlite.rs`

- [ ] Implement `StorageEngine` for `AgeSqliteStorage`
- [ ] Schema creation (meta, entry_types, entries, FTS)
- [ ] In-memory SQLite with `deserialize`
- [ ] Encrypt-on-close, decrypt-on-open
- [ ] Atomic writes with backup

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

- [ ] `init.rs` - Create new ledger
- [ ] Passphrase prompting (dialoguer)
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

- [ ] Full init → close → open → verify workflow
- [ ] Wrong passphrase rejection
- [ ] Corrupted file detection
- [ ] Temp file cleanup verification

### 6. Documentation

- [ ] Update README.md with M1 completion status
- [ ] Add crypto usage examples to docs/DEVELOPMENT.md
- [ ] Document passphrase requirements

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
- [ ] Verify file is encrypted (cannot read with `cat`)
- [ ] Verify wrong passphrase fails gracefully
- [ ] Verify no temp files remain after crash
- [ ] Verify memory cleanup (manual inspection)

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
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings
- [ ] `ledger init ~/test.ledger` works
- [ ] File is encrypted (verify manually)
- [ ] Can reopen and verify structure
- [ ] CI passes on Linux + macOS
- [ ] No plaintext leaks verified

## Notes

- Follow TDD strictly: write tests first
- Each commit should compile and pass tests
- Reference RFC-001 for all crypto decisions
- Use `.claude/CHECKLIST.md` for every module
- Update this plan as we learn

## Resources

- RFC-001: Storage & Encryption Model
- RFC-004: Data Model
- Age documentation: https://docs.rs/age/
- Argon2 documentation: https://docs.rs/argon2/
- Rusqlite documentation: https://docs.rs/rusqlite/
