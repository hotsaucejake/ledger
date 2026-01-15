# Milestone 3: Query & Export Stability

**Status**: Draft  
**Target**: Make Ledger exports durable and query ergonomics consistent without expanding the data model.

## Goals

- Stable, versioned export formats (JSON/JSONL) covering entries, templates, and compositions
- Consistent query/filter behavior across CLI commands
- Clear “export is the contract” documentation

## Exit Criteria

- [ ] JSON export schema versioned and documented (entries, templates, compositions)
- [ ] JSONL export schema versioned and documented (entries, templates, compositions)
- [ ] `jot export` outputs stable fields with metadata header
- [ ] `jot list`, `jot search`, `jot export` share filter semantics
- [ ] Golden-file export tests in CI
- [ ] CLI export tests validate schema

## Implementation Steps

### 1. Export Format Spec

- [ ] Define export JSON schema (fields + types, including templates/compositions)
- [ ] Define JSONL schema (per-entry record + template/composition records)
- [ ] Add metadata header for JSON exports
- [ ] Document in `docs/design/format-spec.md` or new `docs/design/export-spec.md`

### 2. CLI Consistency

- [ ] Align filters across list/search/export (type, tag, time window)
- [ ] Add/align `--since`, `--until`, `--last`, `--limit` where missing
- [ ] Ensure `--json` always matches the export schema

### 3. Tests

- [ ] Golden export fixtures for JSON and JSONL
- [ ] Schema validation tests for CLI outputs
- [ ] Cross-command filter equivalence tests

## Non-Goals

- New data model entities
- Schema creation UX
- Advanced query language

## Notes

- Keep export fields stable and versioned.
- Treat export as the long-term contract for data portability.
