# Milestone 5: Compositions & Templates (Phase 0.2)

**Status**: Draft  
**Target**: Add compositions and templates as first-class data while keeping the CLI and storage model stable.

---

## Goals

- Compositions: semantic grouping across entry types
- Templates: reusable defaults stored in the ledger
- Entry creation uses template-first prompting rules

---

## Scope

### In Scope

- Data model tables for compositions, templates, and entry type defaults
- Storage APIs in `ledger-core`
- CLI commands for managing compositions and templates
- `ledger add` uses template defaults and enum prompting rules
- Tests for storage + CLI flows

### Out of Scope

- GUI/desktop app
- Advanced query language (`--where`)
- Import/export tooling beyond existing format spec guidance

---

## Exit Criteria

- [ ] Compositions can be created, listed, shown, renamed, deleted
- [ ] Templates can be created, listed, shown, updated (new version), deleted
- [ ] `ledger add <type>` applies default template automatically
- [ ] `--template` overrides default template
- [ ] Required fields always prompt if missing
- [ ] Enums reject unknown values; multi-select stored as arrays
- [ ] Storage + CLI integration tests pass
- [ ] Docs updated (format spec, templates spec, RFCs as needed)

---

## Implementation Steps

### 1. Data Model (ledger-core)

- [ ] `compositions` table
- [ ] `entry_compositions` join table
- [ ] `templates` table
- [ ] `template_versions` table (append-only)
- [ ] `entry_type_templates` mapping table (single active default)
- [ ] Migration to add tables and indexes

### 2. Storage APIs (ledger-core)

- [ ] CRUD for compositions
- [ ] CRUD for templates + versions
- [ ] Attach/detach entries to compositions
- [ ] Lookup default template for entry type

### 3. CLI Commands (ledger-cli)

- [ ] `ledger compositions create/list/show/rename/delete`
- [ ] `ledger templates create/list/show/update/delete`
- [ ] `ledger attach/detach`
- [ ] `ledger add` uses template-first prompting
- [ ] `--template`, `--compose`, `--no-compose` behavior aligned with specs

### 4. UX & Prompting Rules

- [ ] No flags: prompt for all fields (template defaults pre-filled)
- [ ] Some flags: prompt for missing required fields; optional fields with defaults skip prompts
- [ ] All fields optional + flags present: store only flag values, no prompts

### 5. Tests

- [ ] Composition CRUD + attach/detach
- [ ] Template versioning and default mapping
- [ ] `ledger add` prompting rules + enum validation
- [ ] Export includes templates/compositions (if updated in M3)

---

## Notes

- Default template selection is stored in `entry_type_templates` (mapping table).
- Only one active template mapping per entry type.
- Templates store defaults; entry type schemas do not own defaults.
