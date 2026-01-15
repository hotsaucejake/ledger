# Milestone 2: UX Polish & First-Run Experience

**Status**: Complete
**Target**: Improve onboarding and daily UX without expanding the data model.

> **Note**: This milestone focuses on "product-ready" UX. The technical milestones
> in `phase-0.1.md` (M1-M6) describe the original implementation order; this M2-PLAN
> represents a UX-focused milestone that builds on top of M1 (Encrypted Storage).

## North Star

A first-time user should:
1. install Jot
2. run a single command
3. understand what's happening
4. create their first entry in under 60 seconds
5. feel safe about encryption, backups, and export

## Exit Criteria

- [x] `jot init` wizard with safe defaults
- [x] XDG config support with default jot path (`~/.local/share/jot/data.jot`)
- [x] Friendly "no jot found" message for all commands
- [x] `jot` with no args shows quickstart
- [x] Passphrase retries (3 attempts, then exit with code 5)
- [x] Optional session cache (in-memory) with TTL (see `docs/design/session-cache.md`)
- [x] Security tiers selectable during init (all 4 tiers)
- [x] OS keychain support (Linux: libsecret, macOS: Keychain)
- [x] `jot add journal` is smooth (editor + stdin + `--body`)
- [x] `jot list` defaults to recent entries (N)
- [x] `jot show` is readable by default
- [x] `jot check` prints clear diagnostics
- [x] `jot backup` confirms output path
- [x] `jot export` help text clarifies portability

## Implementation Steps

### 1. First-Run UX

- [x] Init wizard (default flow + `--advanced`)
- [x] Config file generated at `~/.config/jot/config.toml`
- [x] Config file format matches `docs/design/config-spec.md`
- [x] XDG config path detection + creation
- [x] Default jot path: `~/.local/share/jot/data.jot`
- [x] Clear error when jot is missing (see RFC-003 §15)
- [x] Quickstart output for `jot` (no args)
- [x] Passphrase retry loop (3 attempts, show remaining)
- [x] After 3 failures: exit with code 5 (encryption/auth error per RFC-003 §14.2)
- [x] Wizard copy matches `docs/design/init-wizard.md`
- [x] `--quiet` flag suppresses wizard output (for scripting)
- [x] `--no-input` errors if required values missing

### 2. Daily UX Consistency

- [x] Standard prompt rules (flags win, defaults in brackets)
- [x] Output rules (stable human output, stable JSON output)
- [x] Exit code consistency
- [x] Clear errors with next steps
- [x] Session cache for passphrase (in-memory only, TTL)

### 3. Trust & Safety UX

- [x] `jot check` actionable output (see RFC-003 §13.2)
- [x] `jot backup` safe defaults (confirm destination, atomic copy)
- [x] Export wording emphasizes data ownership
- [x] Security tier selection + explicit warnings (per `init-wizard.md` §2)

### 4. Security Tier Implementation

- [x] Tier 1: Passphrase only (default)
- [x] Tier 2: Passphrase + OS keychain
  - [x] Linux: libsecret/Secret Service D-Bus API
  - [x] macOS: Security.framework / Keychain Services
- [x] Tier 3: Passphrase + encrypted keyfile
  - [x] Key generation (random 32 bytes)
  - [x] Keyfile encrypted with passphrase-derived key
  - [x] Default path: `~/.config/jot/jot.key`
- [x] Tier 4: Device keyfile only (unencrypted)
  - [x] Display explicit security warning (per `config-spec.md` §5)
  - [x] Require confirmation before proceeding

### 5. Session Cache Implementation

See `docs/design/session-cache.md` for design details.

- [x] In-memory passphrase cache with TTL
- [x] Cache mechanism: Unix domain socket (Linux/macOS)
- [x] Automatic cache expiry
- [x] `jot lock` command to clear cache immediately
- [x] Cache disabled by default (`passphrase_cache_ttl_seconds = 0`)

### 6. Optional (Still M2-safe)

- [x] `jot edit <id>` implemented as revision (supersedes)
- [ ] Basic templates for journal (defer to Phase 0.2; stored in jot as data model entity)
- [x] `jot doctor` onboarding diagnostics (add to RFC-003 if implemented)

## Non-Goals

- New data model entities
- User-defined schemas UX
- Query language (`--where`)
- Attachments

## Testing Requirements

### Unit Tests

- [x] Config parsing matches `config-spec.md` format
- [x] Passphrase validation (min 8 chars, not whitespace-only)
- [x] Security tier configuration validation
- [x] XDG path resolution on Linux

### Integration Tests

- [x] Init wizard creates valid config at `~/.config/jot/config.toml`
- [x] Init wizard creates jot at `~/.local/share/jot/data.jot`
- [x] Init wizard respects `--no-input` flag (errors on missing required values)
- [x] Init wizard respects `--quiet` flag
- [x] Prompts skipped when flags provided
- [x] Passphrase retry shows attempts remaining
- [x] After 3 failed attempts, exits with code 5
- [x] `jot` (no args) shows quickstart help
- [x] "No jot found" message is clear and actionable
- [x] Config file matches `config-spec.md` format exactly

### Security Tier Tests

- [x] Tier 1: Passphrase round-trip works
- [x] Tier 2: Keychain storage/retrieval works (platform-specific)
- [x] Tier 3: Encrypted keyfile round-trip works
- [x] Tier 4: Unencrypted keyfile works + warning displayed
- [x] Wrong passphrase fails gracefully (code 5)

### Session Cache Tests

- [x] Cache stores passphrase after successful unlock
- [x] Cache expires after TTL
- [x] `jot lock` clears cache immediately
- [x] Cache disabled when TTL = 0

### Error UX Tests

- [x] Error messages include actionable next steps (per RFC-003 §15)
- [x] Exit codes match RFC-003 §14.2

## Definition of Done

M2 is complete when:

- [x] All tests pass (`cargo test`)
- [x] No clippy warnings (`cargo clippy -- -D warnings`)
- [x] `jot init` completes successfully with wizard
- [x] `jot init --advanced` exposes all options
- [x] `jot` (no args) shows quickstart
- [x] Config written to `~/.config/jot/config.toml`
- [x] Jot created at `~/.local/share/jot/data.jot`
- [x] All 4 security tiers functional
- [x] Session cache works with configurable TTL
- [x] First-time user can complete init in < 60 seconds (manual test)
- [x] CI passes on Linux + macOS
- [x] Documentation updated (README, DEVELOPMENT.md if needed)

## Notes

- M2 should not expand the data model.
- Focus on a "product-ready" feel.
- Config follows XDG Base Directory Specification:
  - Config: `~/.config/jot/config.toml`
  - Data: `~/.local/share/jot/data.jot`
  - Keyfile: `~/.config/jot/jot.key` (if applicable)

## References

- [config-spec.md](../design/config-spec.md) — Config file format
- [init-wizard.md](../design/init-wizard.md) — Wizard UX specification
- [session-cache.md](../design/session-cache.md) — Session cache design
- [RFC-003](../RFC/RFC-003.md) — CLI command taxonomy & UX rules (§14-15 for errors)
- [DEVELOPMENT.md](../DEVELOPMENT.md) — Testing requirements
