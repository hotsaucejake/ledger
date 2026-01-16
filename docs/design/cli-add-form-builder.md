# CLI Add Form Builder

**Status:** Draft
**Applies to:** Ledger CLI (`ledger add`)
**Purpose:** Define the UX and behavior for first-use entry types, form/template creation, and structured add flows.

---

## 1. Goals

- Make `ledger add <type>` work for any entry type without pre-existing setup.
- Provide a guided form builder the first time a type is used.
- Keep add flows fast once a template exists.
- Ensure required fields are always collected.
- Preserve compatibility for existing entries when templates are added later.

---

## 2. Core Concepts

- **Entry type** defines the schema (fields + validation).
- **Template** provides defaults + prompts and is stored inside the ledger.
- **Default template per entry type** is always used unless overridden.

Entry types are user-defined. There are no built-in default schemas.

---

## 3. First-Use Flow

When `ledger add <type>` is used for a type that does not yet exist:

1) Show a choice:
   - **Create a form** (recommended)
   - **Use simple body only**

2) If **Create a form**:
   - Launch the form builder wizard.
   - Create the entry type schema.
   - Create a template and set it as default.
   - Proceed to add using the template prompts.

3) If **Use simple body only**:
   - Create a minimal schema with a required `body` field.
   - Create a default template with a required `body` field.
   - Launch editor (same behavior as current `journal` add).

---

## 4. Type Exists but No Template

If an entry type exists but no template is associated:

- **If entries exist:**
  - Create a default template with required `body` field to preserve display compatibility.
  - Offer to run the form builder to customize the template.

- **If no entries exist:**
  - Prompt to create a form.
  - If declined, create the body-only template and proceed.

---

## 5. Form Builder Wizard

### 5.1 Field Types

Supported types:
- `text`
- `string`
- `number`
- `integer`
- `date`
- `datetime`
- `enum` (single-select)
- `bool`
- `task_list` (preset only)

### 5.2 Field Definition Prompts

For each field:
- Field name
- Field type
- Required? (yes/no)
- Default value (optional)
  - For `date`/`datetime`, offer **Today/Now** as a default
- For `enum`, collect allowed values

### 5.3 Review

- Show a summary of fields, required flags, and defaults.
- Confirm creation.

---

## 6. Add Flow When Template Exists

- Required fields always prompt.
- Optional fields prompt only if they have no default value.
- Defaults may be accepted by pressing Enter.
- `--no-input` errors if required fields are missing after defaults.

---

## 7. Enum Custom Values

During add:
- Users may enter a custom enum value.
- Prompt: **"Add this value to the template for future use?"**
  - If yes: update the template (new version) under `enum_values`.
  - If no: store the value for this entry only.

---

## 8. Todo List Preset

Provide a built-in preset in the form builder:

- **Todo List**
  - `title` (text, optional)
  - `items` (task_list)
    - Each task: `text` (required), `done` (bool, default false)

Todo lists are stored as a single entry containing task state.

---

## 9. UI/UX Requirements

Use existing CLI UI primitives:
- `ui::header`, `ui::badge`, `ui::hint`, `ui::kv`
- `ui::prompt` for wizard flows
- Output modes must follow `docs/design/cli-ux-spec.md`

Interactive flows only in **pretty** mode (TTY). Plain/JSON must stay deterministic.

---

## 10. Warnings and Versioning

- Editing templates creates new versions.
- Warn on deletions or removing fields (future work).
- For now, allow edits with a warning and rely on user discretion.

---

## 11. Examples

### First use (weight)

```
ledger add weight

This entry type does not exist yet.

Choose:
  1) Create a form (recommended)
  2) Use simple body only
```

Form builder:

```
Field name: weight
Field type: number
Required? yes
Default? (blank)

Field name: date
Field type: date
Required? yes
Default? Today
```

Then add:

```
weight: 180.4
date [today]:
```

### First use (journal)

```
ledger add journal

Choose:
  1) Create a form (recommended)
  2) Use simple body only
```

If body-only:
- Opens editor for required `body` field.

---

## 12. Open Questions

- Whether to support multi-select enums in initial release.
- Best UI for editing task lists in-place.
