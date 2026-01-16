# Templates Specification

**Status:** Implemented
**Applies to:** Ledger v0.2+
**Purpose:** Define how templates work, how they are stored, and how they apply during entry creation.

---

## 1. Overview

Templates provide reusable defaults for entry creation. They are stored **inside the encrypted
ledger** so a shared ledger file carries the same templates across machines.

Templates are:

- Named and user-defined
- Associated with a single entry type
- Versioned and append-only

---

## 2. Data Model (Logical)

### 2.1 Template

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Globally unique identifier |
| `name` | string | User-facing name (unique within ledger) |
| `entry_type_id` | UUID | Entry type this template applies to |
| `created_at` | datetime | When this template was created |
| `device_id` | UUID | Device that created this template |
| `description` | string? | Optional description |

Entry types associate to a **default template** via a separate mapping table so defaults can
evolve without bumping schema versions.

### 2.2 Template Version

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Globally unique identifier |
| `template_id` | UUID | Reference to template |
| `version` | integer | Version number (1, 2, 3...) |
| `template_json` | JSON | Template defaults + UX hints |
| `created_at` | datetime | When this version was created |
| `active` | boolean | Whether this is the current version |

Templates are append-only. Updating a template creates a new version and marks it active.

### 2.3 Entry Type Template Mapping

| Field | Type | Description |
|-------|------|-------------|
| `entry_type_id` | UUID | Entry type that owns the default |
| `template_id` | UUID | Default template for the entry type |
| `active` | boolean | Whether this is the current default |

This keeps defaults flexible without forcing schema version bumps.

Rules:

- Only one active mapping per entry type.
- Setting a new default deactivates the previous mapping.
- If no active mapping exists, no default template is applied.

Example SQL (SQLite):

```sql
CREATE TABLE entry_type_templates (
    entry_type_id TEXT NOT NULL,
    template_id TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,

    PRIMARY KEY (entry_type_id, template_id),
    FOREIGN KEY (entry_type_id) REFERENCES entry_types(id),
    FOREIGN KEY (template_id) REFERENCES templates(id)
);

CREATE UNIQUE INDEX entry_type_templates_active
ON entry_type_templates (entry_type_id)
WHERE active = 1;
```

---

## 3. Template JSON Format

```json
{
  "defaults": {
    "car": "civic",
    "octane": "regular"
  },
  "default_tags": ["car", "fuel"],
  "default_compositions": [
    "3b0c9b72-3a9d-4ad6-8a70-2c52a6622f3a"
  ],
  "prompt_overrides": {
    "car": "Which car did you fill?"
  },
  "enum_values": {
    "car": ["civic", "accord"]
  }
}
```

### 3.1 Field Semantics

- `defaults`: default field values for the target entry type.
- `default_tags`: tags applied unless the user overrides tags explicitly.
- `default_compositions`: composition IDs to auto-attach.
- `prompt_overrides`: custom prompt text per field.
- `enum_values`: extra enum options available during prompting (merged with schema values).

---

## 4. Application Rules

Precedence order for a field value:

1. CLI flags (explicit user input)
2. Explicit template selection (`--template`)
3. Default template (from entry type mapping)
4. Template defaults
5. Prompt (interactive) or error (`--no-input`)

Tags and compositions follow a similar rule:

- Explicit `--tag` or `--compose` flags override template defaults.
- `--no-compose` clears template defaults.
 - `--no-input` errors if required fields are still missing after applying defaults.

### 4.1 Template-First Prompting

When an entry type has a default template, it is used automatically for `ledger add <type>`.
Passing `--template` overrides the default template selection.

Prompt behavior:

- **No flags**: prompt for all fields, using template defaults where provided.
- **Some flags**: prompt for any missing required fields, and any optional fields
  that have no template default. Optional fields with template defaults do not prompt.
- **All fields optional + flags present**: only store what was provided; do not prompt.

This keeps quick adds fast while ensuring required fields are never skipped.

---

## 5. Enum Best Practice

For enums:

- **Single-select** values are stored as strings.
- **Multi-select** values are stored as JSON arrays of strings.
- Unknown enum values can be accepted interactively and optionally added to the template for future use.

Enum change best practices:

- **Never rename in place**: add the new value, keep the old for history.
- **Deprecate** old values in the schema to hide them from prompts.
- **Migrate** old values only with explicit tooling (Phase 0.3+).

This avoids delimiter edge cases, preserves order if needed, and stays easy to validate.

Example:

```json
{
  "car": "civic",
  "extras": ["car_wash", "snacks"]
}
```

---

## 6. CLI Surface

### 6.1 Template Management

```bash
# List all templates
ledger templates list
ledger templates list --entry-type journal
ledger templates list --json

# Show template details
ledger templates show gas_fillup
ledger templates show gas_fillup --json

# Create a template
ledger templates create gas_fillup --entry-type gas
ledger templates create gas_fillup --entry-type gas --description "Gas fillup defaults"
ledger templates create gas_fillup --entry-type gas --defaults '{"car": "civic"}'
ledger templates create gas_fillup --entry-type gas --set-default

# Update template (creates new version)
ledger templates update gas_fillup --defaults '{"car": "4runner"}'

# Delete template
ledger templates delete gas_fillup
ledger templates delete gas_fillup --force

# Set/clear default template for entry type
ledger templates set-default gas gas_fillup
ledger templates clear-default gas
```

### 6.2 Using Templates with Add

```bash
# Use default template (if set for entry type)
ledger add gas

# Override with specific template
ledger add gas --template gas_fillup

# Set field values directly
ledger add gas --field car=civic --field octane=regular
ledger add gas -f car=civic -f octane=regular

# Attach to composition
ledger add gas --compose fleet_ops

# Skip template default compositions
ledger add gas --no-compose
```

---

## 7. Scope Notes

- Templates do not create new entry types.
- Templates do not change schemas.
- Templates are data, not code.

---

## 8. Open Questions

- Should templates support inheritance (`base` template + overrides)?
- Should templates allow default timestamps (e.g., `date = today`)?
- Should templates allow per-field help text beyond prompts?
