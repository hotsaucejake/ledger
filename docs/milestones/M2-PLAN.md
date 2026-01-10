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
- [ ] Friendly “no ledger found” message for all commands
- [ ] `ledger` with no args shows quickstart
- [ ] `ledger add journal` is smooth (editor + stdin + `--body`)
- [ ] `ledger list` defaults to recent entries (N)
- [ ] `ledger show` is readable by default
- [ ] `ledger check` prints clear diagnostics
- [ ] `ledger backup` confirms output path
- [ ] `ledger export` help text clarifies portability

## Implementation Steps

### 1. First-Run UX

- [ ] Init wizard (default flow + `--advanced`)
- [ ] Clear error when ledger is missing
- [ ] Quickstart output for `ledger` (no args)

### 2. Daily UX Consistency

- [ ] Standard prompt rules (flags win, defaults in brackets)
- [ ] Output rules (stable human output, stable JSON output)
- [ ] Exit code consistency
- [ ] Clear errors with next steps

### 3. Trust & Safety UX

- [ ] `ledger check` actionable output
- [ ] `ledger backup` safe defaults
- [ ] Export wording emphasizes data ownership

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
