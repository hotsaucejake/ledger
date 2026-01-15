
# Milestone 5: Compositions & Templates (Phase 0.2)

**Status**: Complete
**Target**: Add compositions and templates as first-class data while keeping the CLI and storage model stable.

---

## Goals

- Compositions: semantic grouping across entry types
- Templates: reusable defaults stored in the jot
- Entry creation uses template-first prompting rules

---

## Scope

### In Scope

- Data model tables for compositions, templates, and entry type defaults
- Storage APIs in `jot-core`
- CLI commands for managing compositions and templates
- `jot add` uses template defaults and enum prompting rules
- Tests for storage + CLI flows

### Out of Scope

- GUI/desktop app
- Advanced query language (`--where`)
- Import/export tooling beyond existing format spec guidance

---

## Exit Criteria

- [x] Compositions can be created, listed, shown, renamed, deleted
- [x] Templates can be created, listed, shown, updated (new version), deleted
- [x] `jot add <type>` applies default template automatically
- [x] `--template` overrides default template
- [x] Required fields always prompt if missing
- [x] Enums reject unknown values; multi-select stored as arrays
- [x] Storage + CLI integration tests pass
- [x] Docs updated (format spec, templates spec, RFCs as needed)

---

## Implementation Steps

### 1. Data Model (jot-core)

- [x] `compositions` table
- [x] `entry_compositions` join table
- [x] `templates` table
- [x] `template_versions` table (append-only)
- [x] `entry_type_templates` mapping table (single active default)
- [x] Migration to add tables and indexes

### 2. Storage APIs (jot-core)

- [x] CRUD for compositions
- [x] CRUD for templates + versions
- [x] Attach/detach entries to compositions
- [x] Lookup default template for entry type

### 3. CLI Commands (jot-cli)

- [x] `jot compositions create/list/show/rename/delete`
- [x] `jot templates create/list/show/update/delete`
- [x] `jot attach/detach`
- [x] `jot add` uses template-first prompting
- [x] `--template`, `--compose`, `--no-compose` behavior aligned with specs

### 3.1 Composition Semantics (Theme Associations)

Compositions are thematic groupings that can span entry types:

- **Entry type association**: link an entry type to a composition so new entries of that type
  are automatically included (via default composition or template defaults).
- **Per-entry association**: attach any individual entry to one or more compositions.

Example:

- Entry type: `research-paper` (notes, drafts)
- Composition: `research-paper` (thematic container)
- A single `bookmark` entry can be attached to the same composition without changing its type.

This keeps tagging light while allowing deeper thematic grouping across multiple entry types.

### 4. UX & Prompting Rules

- [x] No flags: prompt for all fields (template defaults pre-filled)
- [x] Some flags: prompt for missing required fields; optional fields with defaults skip prompts
- [x] All fields optional + flags present: store only flag values, no prompts

### 5. Tests

- [x] Composition CRUD + attach/detach
- [x] Template versioning and default mapping
- [x] `jot add` prompting rules + enum validation
- [ ] Export includes templates/compositions (deferred to export enhancement)

---

## Notes

- Default template selection is stored in `entry_type_templates` (mapping table).
- Only one active template mapping per entry type.
- Templates store defaults; entry type schemas do not own defaults.
