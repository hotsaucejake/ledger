# Milestone 2: UX Polish & First-Run Experience

**Status**: Draft  
**Target**: Improve onboarding and daily UX without expanding the data model.

## North Star

A first-time user should:
1. install Ledger  
2. run a single command  
3. understand what’s happening  
4. create their first entry in under 60 seconds  
5. feel safe about encryption, backups, and export

## Exit Criteria

- [ ] `ledger init` wizard with safe defaults
- [ ] XDG config support with default ledger path
- [ ] Friendly “no ledger found” message for all commands
- [ ] `ledger` with no args shows quickstart
- [ ] Passphrase retries (3 attempts)
- [ ] Optional session cache (in-memory) with TTL
- [ ] Security tiers selectable during init
- [ ] OS keychain support (optional)
- [ ] `ledger add journal` is smooth (editor + stdin + `--body`)
- [ ] `ledger list` defaults to recent entries (N)
- [ ] `ledger show` is readable by default
- [ ] `ledger check` prints clear diagnostics
- [ ] `ledger backup` confirms output path
- [ ] `ledger export` help text clarifies portability

## Implementation Steps

### 1. First-Run UX

- [ ] Init wizard (default flow + `--advanced`)
- [ ] XDG config path detection + creation
- [ ] Default ledger path stored in config
- [ ] Clear error when ledger is missing
- [ ] Quickstart output for `ledger` (no args)
- [ ] Passphrase retry loop

### 2. Daily UX Consistency

- [ ] Standard prompt rules (flags win, defaults in brackets)
- [ ] Output rules (stable human output, stable JSON output)
- [ ] Exit code consistency
- [ ] Clear errors with next steps
- [ ] Session cache for passphrase (in-memory only, TTL)

### 3. Trust & Safety UX

- [ ] `ledger check` actionable output
- [ ] `ledger backup` safe defaults
- [ ] Export wording emphasizes data ownership
- [ ] Security tier selection + explicit warnings

### 4. Optional (Still M2-safe)

- [ ] `ledger edit <id>` implemented as revision (supersedes)
- [ ] Basic templates for journal (no schema expansion)
- [ ] `ledger doctor` onboarding diagnostics

## Non-Goals

- New data model entities
- User-defined schemas UX
- Query language (`--where`)
- Attachments

## Notes

- M2 should not expand the data model.
- Focus on a “product‑ready” feel.
- Config should follow XDG:
  - Linux: `~/.config/ledger/config.toml`

### Security Tiers (user selectable)

1. **Passphrase only** (default)
2. **Passphrase + OS keychain** (store passphrase in keychain)
3. **Passphrase + encrypted key file** (key file protected by passphrase)
4. **Device key only (unencrypted key file)**  
   - Allowed by user choice, but must display a clear security warning.
