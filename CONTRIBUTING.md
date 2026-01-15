# Contributing to Jot

Thank you for your interest in contributing to Jot! This guide will help you get started.

## Quick Start

1. **Read the essential documentation:**
   - [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) — Complete development guide **(REQUIRED READING)**
   - [docs/planning.md](docs/planning.md) — Project vision and principles
   - [README.md](README.md) — Current status and roadmap

2. **Set up your development environment:**
   ```bash
   # Clone the repository
   git clone https://github.com/hotsaucejake/jot.git
   cd jot

   # Build the project
   cargo build

   # Run tests
   cargo test

   # Check code quality
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   ```

3. **Pick an issue or feature:**
   - Check the [GitHub issues](https://github.com/hotsaucejake/jot/issues)
   - Look for issues tagged `good first issue` or `help wanted`
   - Or propose a new feature (read RFCs first!)

## Before You Start

### Required Reading

These documents define how we work:

1. **[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)** — Development standards, testing requirements, Rust best practices
2. **[.claude/CHECKLIST.md](.claude/CHECKLIST.md)** — Quick reference checklist for every feature
3. **Relevant RFCs in [docs/RFC/](docs/RFC/)** — Understand the architecture before changing it

### Design Principles

Jot is built on non-negotiable principles:

1. **Security by default** — Encryption always, no plaintext modes
2. **User owns their data** — Local storage, no cloud, no lock-in
3. **Tests are mandatory** — Every feature has tests, no exceptions
4. **Fail loudly** — Rich errors, never silent failures
5. **Future-proof** — Explicit versioning, documented migrations

**Never violate these principles.** If you think a principle needs reconsideration, start a discussion first.

## Development Workflow

### 1. Check Milestone Scope

Before starting work, verify the feature is in scope for the current milestone:

- Check [README.md](README.md) for current milestone
- Read [docs/milestones/phase-0.1.md](docs/milestones/phase-0.1.md) (or current phase)
- Features outside the current phase should be discussed first

### 2. Create a Branch

```bash
git checkout -b feature/my-feature-name
# or
git checkout -b fix/bug-description
```

### 3. Write Tests First

We practice **test-driven development (TDD)**:

```rust
// 1. Write a failing test
#[test]
fn test_new_feature() {
    let result = new_feature();
    assert!(result.is_ok());
}

// 2. Implement the feature
pub fn new_feature() -> Result<()> {
    // Implementation
}

// 3. Verify test passes
```

See [docs/DEVELOPMENT.md#testing-requirements](docs/DEVELOPMENT.md#testing-requirements) for details.

### 4. Implement Incrementally

- Small, focused commits
- Each commit should compile and pass tests
- Follow Rust best practices (see DEVELOPMENT.md)

### 5. Run Pre-Commit Checks

**Before every commit:**

```bash
cargo fmt --all                                          # Format code
cargo clippy --all-targets --all-features -- -D warnings # Lint
cargo test                                               # Run tests
```

All must pass ✓

### 6. Submit a Pull Request

**Pull Request Checklist:**

- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code formatted
- [ ] Documentation updated (if public API changed)
- [ ] CHANGELOG.md updated (if user-facing change)
- [ ] Tests added for new functionality
- [ ] Relevant RFC updated (if architecture changed)

**PR Description Template:**

```markdown
## What does this PR do?

Brief description of the change.

## Why is this change needed?

Explain the problem this solves.

## How was this tested?

- [ ] Unit tests added
- [ ] Integration tests added
- [ ] Manual testing performed

## Risks

What could go wrong? Are there breaking changes?

## Related Issues

Fixes #123
```

## Code Quality Standards

### Rust Best Practices

**DO:**
- Use `Result<T>` for all fallible operations
- Write comprehensive doc comments
- Handle errors with context
- Program against traits, not concrete types

**DON'T:**
- Use `.unwrap()` or `.expect()` in production code
- Skip tests
- Introduce unsafe code without justification
- Add dependencies with known CVEs

See [docs/DEVELOPMENT.md#rust-best-practices](docs/DEVELOPMENT.md#rust-best-practices) for complete guidelines.

### Testing Standards

**Every PR must include tests.**

- Unit tests for core logic
- Integration tests for user workflows
- Test happy paths **and** error conditions
- Tests run fast (< 100ms per test)

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_normalization() {
        assert_eq!(normalize_tag("  Test  "), "test");
    }

    #[test]
    fn test_invalid_tag_rejected() {
        let result = validate_tag("invalid tag!");
        assert!(result.is_err());
    }
}
```

## Architecture Guidelines

### Separation of Concerns

#### jot-core (Library)
- Domain logic, storage, crypto, validation
- **No CLI dependencies**
- **No user interaction** (no prompts, no printing)
- Deterministic, testable APIs

#### jot-cli (Binary)
- CLI parsing, user interaction, output formatting
- **Minimal logic** (orchestrate, don't implement)
- **All domain logic** delegated to jot-core

See [docs/DEVELOPMENT.md#architecture-guidelines](docs/DEVELOPMENT.md#architecture-guidelines) for details.

## Common Tasks

### Adding a New CLI Command

1. Add to `Commands` enum in `crates/jot-cli/src/main.rs`
2. Add match arm in `main()`
3. Extract logic to function in `commands/` module
4. Call core library for domain logic
5. Format output appropriately
6. Add integration test

### Adding a New Module

1. Read relevant RFC for context
2. Create module with doc comments
3. Write tests in same file (`#[cfg(test)]` mod)
4. Document all public APIs
5. Add to public exports if needed

### Fixing a Bug

1. Write failing test that reproduces bug
2. Fix the bug
3. Verify test passes
4. Check for similar bugs elsewhere
5. Submit PR with test + fix

## Security Guidelines

**Cryptography:**
- Use well-audited libraries only (`age`, `argon2`)
- Never roll your own crypto
- Follow RFC specifications exactly
- Zeroize sensitive data from memory

**Input Validation:**
- Validate at boundaries (CLI or API entry)
- Reject invalid input, don't sanitize
- Provide clear error messages

See [docs/DEVELOPMENT.md#security-practices](docs/DEVELOPMENT.md#security-practices).

## Documentation

### What Needs Documentation?

- All public functions and types
- Module-level documentation
- Complex algorithms or non-obvious code
- RFCs for architectural changes

### Documentation Format

```rust
/// Brief one-line description.
///
/// More detailed explanation if needed.
///
/// # Arguments
///
/// * `arg1` - Description of arg1
///
/// # Returns
///
/// Description of return value.
///
/// # Errors
///
/// Describe error conditions.
///
/// # Examples
///
/// ```
/// let result = function(arg);
/// assert!(result.is_ok());
/// ```
pub fn function(arg: Type) -> Result<ReturnType> {
    // ...
}
```

## Communication

### Asking Questions

- Check [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) first
- Search existing [GitHub issues](https://github.com/hotsaucejake/jot/issues)
- Open a new issue with the `question` label
- Be specific about what you're trying to do

### Proposing Features

- Read [docs/planning.md](docs/planning.md) to understand project scope
- Check if it fits the current milestone
- Open an issue with:
  - Problem statement
  - Proposed solution
  - Why it's needed
  - How it fits the design principles

### Reporting Bugs

Include:
- Steps to reproduce
- Expected behavior
- Actual behavior
- Environment (OS, Rust version)
- Minimal code example

## Getting Help

- Read [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) — comprehensive guide
- Check [.claude/CHECKLIST.md](.claude/CHECKLIST.md) — quick reference
- Review relevant RFCs in [docs/RFC/](docs/RFC/)
- Open a [GitHub issue](https://github.com/hotsaucejake/jot/issues) with the `question` label

## License

By contributing to Jot, you agree that your contributions will be licensed under the same terms as the project (MIT or Apache-2.0).

---

## Quick Reference

**Essential Commands:**
```bash
cargo build                                              # Build
cargo test                                               # Test
cargo fmt --all                                          # Format
cargo clippy --all-targets --all-features -- -D warnings # Lint
```

**Must Read:**
1. [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)
2. [.claude/CHECKLIST.md](.claude/CHECKLIST.md)
3. Relevant RFCs in [docs/RFC/](docs/RFC/)

**Golden Rules:**
1. Security first, always
2. Test everything, test early
3. Errors are values, handle them
4. Documentation is not optional
5. When in doubt, read the RFCs

---

Thank you for contributing to Jot!
