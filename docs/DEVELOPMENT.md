# Development Guide

**For developers working on Ledger**

This document defines the standards, practices, and workflow for developing Ledger. Follow these guidelines to maintain quality, consistency, and long-term maintainability.

---

## Project Context

### What is Ledger?

A secure, encrypted, CLI-first personal journal and logbook combining:
- Strong encryption at rest (no plaintext modes)
- Structured, queryable data with user-defined schemas
- Append-only, conflict-aware data model
- CLI-first workflow with full scriptability
- Long-term data integrity and portability

### Current Status

Check `README.md` for milestone status. As of Milestone 1 (in progress):
- Encrypted storage + schema initialization implemented
- Entry CRUD + FTS search working in `ledger-core`
- CLI init/add/list/search/show/check/export/backup working for `journal`
- CLI integration tests in place

### Core Design Documents (REQUIRED READING)

Before making changes, read:
1. **docs/planning.md** — Vision, principles, non-negotiables
2. **docs/design/format-spec.md** — File format specification
3. **docs/milestones/phase-0.1.md** — Current phase scope
4. **Relevant RFCs in docs/RFC/** — Architecture decisions

**Never** violate the design principles without explicit discussion and RFC amendment.

---

## Development Philosophy

### Non-Negotiable Principles

1. **Security is not optional**
   - Encryption by default, always
   - No plaintext storage modes
   - Follow threat model (docs/design/threat-model.md when created)
   - Validate all cryptographic assumptions

2. **Data durability over convenience**
   - Never lose user data
   - Append-only model prevents destructive operations
   - Migrations are explicit and reversible
   - Backups are atomic

3. **Test before you ship**
   - Every feature has tests
   - Tests run fast (< 100ms per test)
   - No manual testing as substitute for automated tests
   - Integration tests for user-facing behavior

## Environment Variables

- `LEDGER_PATH`: default ledger file path.
- `LEDGER_PASSPHRASE`: non-interactive passphrase (useful for tests/scripts).
- `LEDGER_CONFIG`: override config path (planned; not yet implemented).

4. **Fail loudly, never silently**
   - Use `Result<T>` for all fallible operations
   - Rich error messages with context
   - No `.unwrap()` in production code
   - No panics in library code

5. **Future-proof by design**
   - Explicit versioning of formats and schemas
   - Backward compatibility or explicit migration
   - Document all breaking changes

---

## Testing Requirements

### Testing is Mandatory

**Every PR must include tests.** No exceptions unless explicitly justified.

### Testing Strategy

#### Unit Tests (ledger-core)

```rust
// In the same file as the code
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_normalization() {
        assert_eq!(normalize_tag("  Test  "), "test");
        assert_eq!(normalize_tag("Multi-Word"), "multi-word");
    }

    #[test]
    fn test_invalid_tag_rejected() {
        let result = validate_tag("invalid tag!");
        assert!(result.is_err());
    }
}
```

**Requirements:**
- Test happy paths
- Test error conditions
- Test edge cases (empty, null, boundary values)
- Test invariants (e.g., UUID uniqueness)

#### Integration Tests

```rust
// In tests/integration/
use ledger_core::storage::StorageEngine;

#[test]
fn test_round_trip_entry() {
    let temp_dir = tempfile::tempdir().unwrap();
    let ledger_path = temp_dir.path().join("test.ledger");

    // Create and add entry
    let mut engine = TestStorageEngine::create(&ledger_path).unwrap();
    let entry_id = engine.insert_entry(&test_entry()).unwrap();
    engine.close().unwrap();

    // Reopen and verify
    let engine = TestStorageEngine::open(&ledger_path).unwrap();
    let entry = engine.get_entry(&entry_id).unwrap().unwrap();
    assert_eq!(entry.data["body"], "test content");
}
```

**Requirements:**
- Test full user workflows
- Use temporary directories/files
- Clean up resources
- No flaky tests (no timing dependencies)

#### Test Utilities

Create test helpers to reduce boilerplate:

```rust
// In ledger-core/src/test_utils.rs (cfg(test) only)
pub fn test_entry() -> NewEntry { /* ... */ }
pub fn temp_ledger() -> TempLedger { /* ... */ }
pub fn mock_storage() -> MockStorage { /* ... */ }
```

### Coverage Goals

- Core domain logic: **90%+ coverage**
- Error paths: **100% coverage**
- CLI argument parsing: **80%+ coverage**

Run coverage locally:
```bash
cargo tarpaulin --out Html --output-dir coverage/
```

---

## Rust Best Practices

### Code Quality Standards

#### Error Handling

**DO:**
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("Entry not found: {0}")]
    NotFound(Uuid),

    #[error("Invalid schema: {field} {reason}")]
    InvalidSchema { field: String, reason: String },
}

pub type Result<T> = std::result::Result<T, LedgerError>;

// Usage
pub fn get_entry(&self, id: &Uuid) -> Result<Entry> {
    self.entries.get(id)
        .cloned()
        .ok_or_else(|| LedgerError::NotFound(*id))
}
```

**DON'T:**
```rust
// ❌ No unwrap in library code
let entry = entries.get(id).unwrap();

// ❌ No generic string errors
return Err("something went wrong".into());

// ❌ No silent failures
if let Some(entry) = entries.get(id) {
    // silently returns None if not found
}
```

#### Type Safety

**DO:**
```rust
// Use newtype pattern for domain types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntryId(Uuid);

impl EntryId {
    pub fn new() -> Self {
        Self(Uuid::new_v7())
    }
}

// Use builder pattern for complex construction
pub struct NewEntry {
    entry_type_id: EntryTypeId,
    data: serde_json::Value,
    tags: Vec<String>,
}

impl NewEntry {
    pub fn builder(entry_type_id: EntryTypeId) -> NewEntryBuilder {
        NewEntryBuilder::new(entry_type_id)
    }
}
```

**DON'T:**
```rust
// ❌ Primitive obsession
fn get_entry(id: String) -> Result<Entry>

// ❌ Stringly-typed APIs
fn create_type(name: String, version: String) -> Result<()>
```

#### Ownership and Lifetimes

**DO:**
```rust
// Accept borrowed data when you don't need ownership
pub fn validate_schema(&self, schema: &Schema) -> Result<()>

// Return owned data when caller needs it
pub fn list_entries(&self) -> Result<Vec<Entry>>

// Use Cow for flexibility
pub fn tag(&self) -> Cow<'_, str>
```

**DON'T:**
```rust
// ❌ Unnecessary clones
pub fn process_entry(&self, entry: Entry) -> Result<Entry> {
    let cloned = entry.clone(); // why?
    // ...
}

// ❌ Lifetime soup without reason
pub fn get<'a, 'b: 'a>(&'b self, id: &'a str) -> &'a Entry
```

#### Module Organization

```rust
// lib.rs - Public API surface
pub mod storage;
pub mod entry;
pub mod schema;
pub mod error;

pub use error::{LedgerError, Result};
pub use storage::StorageEngine;

// Keep implementation details private
mod crypto;
mod validation;
```

### Performance Considerations

**DO:**
- Use `&str` instead of `String` in function arguments
- Prefer `Vec::with_capacity` when size is known
- Use `BufReader`/`BufWriter` for I/O
- Profile before optimizing (use `cargo flamegraph`)

**DON'T:**
- Micro-optimize without measurements
- Use `Arc<Mutex<T>>` when single-threaded is fine
- Clone large structures unnecessarily

### Documentation

**Every public item must have doc comments:**

```rust
/// Retrieves an entry by its unique identifier.
///
/// # Arguments
///
/// * `id` - The UUID of the entry to retrieve
///
/// # Returns
///
/// Returns `Ok(Some(entry))` if found, `Ok(None)` if not found,
/// or `Err` if a storage error occurred.
///
/// # Examples
///
/// ```
/// use ledger_core::storage::StorageEngine;
///
/// let entry = engine.get_entry(&entry_id)?;
/// assert!(entry.is_some());
/// ```
pub fn get_entry(&self, id: &Uuid) -> Result<Option<Entry>> {
    // ...
}
```

---

## Architecture Guidelines

### Separation of Concerns

#### ledger-core (Library)

**Responsibilities:**
- Storage abstraction and implementations
- Domain logic (entries, schemas, validation)
- Cryptography primitives
- Search and query logic
- Export/import logic

**Rules:**
- No CLI dependencies (no `clap`)
- No user interaction (no prompts, no printing)
- No `std::process::exit()`
- Deterministic, testable APIs
- All public APIs documented

#### ledger-cli (Binary)

**Responsibilities:**
- CLI argument parsing
- User interaction (prompts, progress bars)
- Output formatting (human-readable, JSON)
- Error message translation (core errors → user-friendly)
- `$EDITOR` integration

**Rules:**
- Minimal logic (orchestrate, don't implement)
- All domain logic delegated to `ledger-core`
- No SQL queries
- No direct crypto operations

### Dependency Management

**Allowed dependencies:**
- Core: `uuid`, `serde`, `thiserror`, `anyhow`, `chrono`
- Crypto: `age`, `argon2` (when needed)
- Storage: `rusqlite` (when needed)
- CLI: `clap`, `dialoguer` (when needed)

**Forbidden dependencies:**
- No `unsafe` without justification
- No `tokio` for now (project is sync)
- No web frameworks
- No GUI toolkits
- No dependencies with known CVEs

Check dependencies:
```bash
cargo deny check
cargo audit
```

### Storage Abstraction

**Always program against the trait:**

```rust
pub trait StorageEngine: Send + Sync {
    fn insert_entry(&mut self, entry: &NewEntry) -> Result<Uuid>;
    fn get_entry(&self, id: &Uuid) -> Result<Option<Entry>>;
    // ...
}
```

**Never:**
```rust
// ❌ Don't couple to specific implementation
fn process(db: &SqliteStorage) -> Result<()>

// ✓ Use the trait
fn process<S: StorageEngine>(storage: &S) -> Result<()>
```

This enables:
- Easy testing (mock implementations)
- Backend swapping (SQLite → SQLCipher)
- Clear contracts

---

## Workflow

### Starting a New Feature

1. **Read relevant RFCs and design docs**
2. **Check milestone scope** — Is this in the current phase?
3. **Write tests first** (TDD encouraged)
4. **Implement incrementally**
5. **Run full test suite** (`cargo test`)
6. **Check formatting and lints** (`cargo fmt`, `cargo clippy`)
7. **Update documentation** if public API changed

### Before Every Commit

```bash
# Format code
cargo fmt --all

# Check lints (warnings = errors)
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test

# Check docs build
cargo doc --no-deps --document-private-items
```

### Adding a New Module

Template:
```rust
//! Brief module description.
//!
//! Longer description of responsibilities, key types,
//! and usage patterns.

// Imports organized: std, external crates, internal
use std::collections::HashMap;

use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::error::{Result, LedgerError};

// Public API
pub struct MyType {
    // ...
}

// Implementation
impl MyType {
    pub fn new() -> Self {
        // ...
    }
}

// Tests in same file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let instance = MyType::new();
        // assertions
    }
}
```

### Adding a New CLI Command

1. Add to `Commands` enum in `main.rs`
2. Add match arm in `main()`
3. Extract logic to a function in `commands/` module
4. Call core library for domain logic
5. Format output appropriately
6. Add integration test

Example:
```rust
// In commands/list.rs
pub fn list(
    engine: &impl StorageEngine,
    filters: &ListFilters,
    output_format: OutputFormat,
) -> anyhow::Result<()> {
    let entries = engine.list_entries(filters)?;

    match output_format {
        OutputFormat::Human => print_human(&entries),
        OutputFormat::Json => print_json(&entries)?,
    }

    Ok(())
}
```

---

## Common Patterns

### Configuration Pattern

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerConfig {
    pub storage: StorageConfig,
    pub crypto: CryptoConfig,
}

impl LedgerConfig {
    pub fn load() -> Result<Self> {
        // Load from config file with defaults
    }

    pub fn default() -> Self {
        // Sensible defaults
    }
}
```

### Resource Cleanup Pattern

```rust
pub struct Ledger {
    storage: Box<dyn StorageEngine>,
    key: Option<DerivedKey>,
}

impl Drop for Ledger {
    fn drop(&mut self) {
        // Wipe sensitive data
        if let Some(key) = self.key.take() {
            key.zeroize();
        }
    }
}
```

### Builder Pattern

```rust
pub struct QueryBuilder {
    entry_type: Option<String>,
    tags: Vec<String>,
    limit: Option<usize>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            entry_type: None,
            tags: Vec::new(),
            limit: None,
        }
    }

    pub fn entry_type(mut self, t: impl Into<String>) -> Self {
        self.entry_type = Some(t.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn build(self) -> Query {
        Query { /* ... */ }
    }
}

// Usage
let query = QueryBuilder::new()
    .entry_type("journal")
    .tag("important")
    .build();
```

---

## CI/CD Requirements

All checks must pass before merge:

### Continuous Integration

```yaml
# .github/workflows/ci.yml checks:
- cargo build (Linux, macOS)
- cargo test
- cargo fmt --check
- cargo clippy -- -D warnings
- cargo doc --no-deps
```

### Pre-push Checklist

```bash
#!/bin/bash
# Run this before pushing

set -e

echo "🧹 Formatting..."
cargo fmt --all

echo "🔍 Linting..."
cargo clippy --all-targets --all-features -- -D warnings

echo "🧪 Testing..."
cargo test

echo "📚 Docs..."
cargo doc --no-deps --document-private-items

echo "✅ All checks passed!"
```

---

## Security Practices

### Cryptography Rules

**DO:**
- Use well-audited libraries (`age`, `argon2`)
- Follow RFC specifications exactly
- Wipe sensitive data from memory (`zeroize` crate)
- Test encryption round-trips

**DON'T:**
- Roll your own crypto
- Store keys in plaintext
- Use deprecated algorithms
- Trust user input

### Passphrase Requirements

- Minimum length: **12 characters**
- Must not be empty or whitespace-only
- Enforced in `ledger-core::crypto::validate_passphrase`

### Crypto Usage Examples

```rust
use ledger_core::crypto::{derive_key, validate_passphrase};

let passphrase = "example-passphrase-123";
validate_passphrase(passphrase)?;

let salt = b"unique-salt-1234567890";
let key = derive_key(passphrase, salt)?;
```

```rust
use ledger_core::storage::{AgeSqliteStorage, StorageEngine};

let path = std::path::Path::new("example.ledger");
let passphrase = "example-passphrase-123";

let device_id = AgeSqliteStorage::create(path, passphrase)?;
let storage = AgeSqliteStorage::open(path, passphrase)?;
storage.close()?;
```

### Input Validation

```rust
pub fn validate_tag(tag: &str) -> Result<()> {
    // Length check
    if tag.is_empty() || tag.len() > 128 {
        return Err(LedgerError::InvalidInput(
            "Tag must be 1-128 characters".into()
        ));
    }

    // Character check
    if !tag.chars().all(|c| c.is_alphanumeric() || "-_:".contains(c)) {
        return Err(LedgerError::InvalidInput(
            "Tag contains invalid characters".into()
        ));
    }

    Ok(())
}
```

Always validate at the boundary (CLI or API entry point).

---

## Performance Profiling

### When to Profile

- Before optimizing anything
- When adding new I/O operations
- When performance regression suspected

### Tools

```bash
# CPU profiling
cargo install flamegraph
cargo flamegraph --bin ledger -- add journal

# Memory profiling
cargo install cargo-bloat
cargo bloat --release

# Benchmark critical paths
cargo bench
```

### Benchmarks

```rust
// In benches/benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_search(c: &mut Criterion) {
    let engine = setup_test_engine();

    c.bench_function("search 1000 entries", |b| {
        b.iter(|| {
            engine.search(black_box("test query"))
        });
    });
}

criterion_group!(benches, benchmark_search);
criterion_main!(benches);
```

---

## Debugging

### Logging Strategy

```rust
// Use tracing for structured logging
use tracing::{info, debug, warn, error};

pub fn open_ledger(path: &Path) -> Result<Ledger> {
    debug!(?path, "Opening ledger");

    let metadata = read_metadata(path)?;
    info!(version = %metadata.version, "Ledger opened");

    Ok(ledger)
}
```

Enable in CLI:
```bash
RUST_LOG=ledger_core=debug ledger list
```

### Common Issues

**Build failures:**
- Check Rust version: `rustc --version` (need 1.70+)
- Clean build: `cargo clean && cargo build`
- Update deps: `cargo update`

**Test failures:**
- Check temp file permissions
- Verify test isolation (no shared state)
- Run single test: `cargo test test_name -- --nocapture`

**Clippy warnings:**
- Fix immediately, don't accumulate
- Use `#[allow(clippy::lint_name)]` only with justification

---

## Pull Request Checklist

Before submitting:

- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt --all`)
- [ ] Documentation updated (if public API changed)
- [ ] CHANGELOG.md updated (if user-facing change)
- [ ] Tests added for new functionality
- [ ] Error handling complete (no `unwrap` in prod code)
- [ ] Relevant RFC/design doc updated (if architecture changed)

PR description should include:
- What problem does this solve?
- How does it solve it?
- What are the risks?
- How was it tested?

---

## When working on this project:

1. **Read first, code second**
   - Always check relevant RFCs and design docs
   - Understand the "why" before changing the "what"

2. **Tests are not optional**
   - Write tests alongside code, not after
   - Think about test cases before implementation

3. **Fail fast with context**
   - Rich error types, not strings
   - Every error should guide the user to a solution

4. **Design for the long term**
   - User data outlives the code
   - Breaking changes require explicit migration

5. **When in doubt, ask**
   - Propose solutions, explain tradeoffs
   - Don't make architectural decisions silently

---

## Resources

### External References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)

### Internal References

- [Project Planning](planning.md)
- [RFCs](RFC/)
- [Format Specification](design/format-spec.md)
- [Milestones](milestones/)

---

## Summary: The Golden Rules

1. **Security first, always**
2. **Test everything, test early**
3. **Errors are values, handle them**
4. **Documentation is not optional**
5. **Performance is measured, not guessed**
6. **User data is sacred**
7. **When in doubt, read the RFCs**

Follow these principles, and Ledger will remain maintainable, secure, and trustworthy for years to come.
