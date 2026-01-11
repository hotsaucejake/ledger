# Milestone 4: Revisions, History, and Trust

**Status**: In Progress  
**Target**: Add revision workflows and safety tooling while preserving append-only semantics.

## Goals

- Revision-based edit workflow (`supersedes`)
- History views for entries
- Strong integrity/repair tooling

## Exit Criteria

- [x] `ledger edit <id>` creates a revision (supersedes)
- [ ] `ledger show <id> --history` displays full chain
- [x] List/search default to latest revisions only
- [x] `ledger list/search --history` includes superseded revisions
- [ ] `ledger check --verbose` provides actionable diagnostics
- [ ] `ledger repair` can rebuild FTS and fix orphaned index rows
- [ ] Revision chain tests pass

## Implementation Steps

### 1. Revision Semantics

- [x] Implement edit as revision (new entry + supersedes)
- [ ] Store/display revision chains
- [x] Define “current” vs “historical” behavior (list/search default latest)

### 2. CLI History UX

- [ ] `ledger show --history`
- [x] `ledger list/search --history` (superseded revisions)
- [ ] Clear messaging that edits preserve originals

### 3. Repair & Diagnostics

- [ ] `ledger check --verbose`
- [ ] `ledger repair --fts` (rebuild index)
- [ ] `ledger repair --orphans` (clean orphaned FTS rows)

### 4. Tests

- [ ] Revision chain test suite
- [ ] FTS repair tests
- [ ] Orphan cleanup tests

## Non-Goals

- Sync/merge workflows
- Advanced conflict resolution
- Schema changes

## Notes

- “Edit” should never mutate in place.
- Revisions must be explicit and transparent to the user.
