# Phase 0.1 — Minimal Viable Journal

**Status:** Scope Frozen
**Goal:** Validate encryption + CLI UX before committing to complex dependencies

---

## Scope Statement

Phase 0.1 delivers the **smallest useful encrypted journal**. It proves the core experience works before adding structured schemas, templates, compositions, or advanced queries.

If Phase 0.1 fails, we learn cheaply. If it succeeds, we have a solid foundation.

---

## In Scope

### Storage Backend (LOCKED)

| Decision   | Choice                                       |
|------------|----------------------------------------------|
| Encryption | Age (passphrase recipients)                  |
| Payload    | SQLite (in-memory via `sqlite3_deserialize`) |
| Temp Files | None (fallback to secure temp if needed)     |

See RFC-001 §5-7 for rationale.

### Entry Type

Phase 0.1 ships with **one built-in entry type**: `journal`

```json
{
  "name": "journal",
  "version": 1,
  "fields": [
    { "name": "body", "type": "text", "required": true }
  ]
}
```

Users cannot create custom entry types in Phase 0.1.

### Data Model

Per format-spec.md §7:

| Entity | Included |
|--------|----------|
| Entries | Yes (journal only) |
| Entry Types | Yes (journal only, read-only) |
| Compositions | No (Phase 0.2) |
| Metadata | Yes |

**Entry fields implemented:**
- `id` (UUID v7)
- `entry_type_id` (UUID of built-in journal type)
- `schema_version` (1)
- `created_at` (millisecond precision)
- `data` (JSON with `body` field)
- `tags` (array, normalized per §7.5)
- `device_id` (UUID v7)
- `supersedes` (optional UUID for revisions)

**Not implemented:**
- `deleted_at` (soft delete deferred)
- Multi-entry type CLI (Phase 0.2)
- Templates (Phase 0.2)

### CLI Commands

| Command | Description |
|---------|-------------|
| `jot init` | Create new encrypted ledger |
| `jot add` | Add journal entry (opens $EDITOR) |
| `jot add --body "..."` | Add inline entry |
| `jot add --tags foo,bar` | Add with tags |
| `jot list` | List recent entries |
| `jot list --limit N` | List N entries |
| `jot list --last 7d` | List recent entries |
| `jot list --tag foo` | Filter by tag |
| `jot list --json` | Output as JSON |
| `jot search "term"` | Full-text search |
| `jot search --type journal` | Filter by entry type |
| `jot show <id>` | Show single entry |
| `jot export --json` | Export all entries as JSON |
| `jot export --jsonl` | Export as JSONL |
| `jot backup <dest>` | Backup jot file |
| `jot check` | Integrity check |
| `jot edit <id>` | Create a revision |
| `jot doctor` | Onboarding diagnostics |
| `jot lock` | Clear session cache |

### Full-Text Search

- SQLite FTS5 on entry body
- Simple query syntax (phrase matching)
- No advanced operators (Phase 0.3)

### Security

- Passphrase prompted at each operation
- Optional in-memory session cache (off by default)
- Age encryption with Argon2id KDF
- In-memory SQLite (no plaintext temp files)
- Atomic writes with backup

**Passphrase requirements:**
- Minimum length: 8 characters
- Must not be empty or whitespace-only

---

## Explicitly Out of Scope

These are **not bugs** in Phase 0.1. They are intentionally deferred.

| Feature | Deferred To |
|---------|-------------|
| Custom entry types | Phase 0.2 |
| Schema creation (`jot types create`) | Phase 0.2 |
| Templates | Phase 0.2 |
| Compositions | Phase 0.2 |
| Enum fields | Phase 0.2 |
| Soft delete | v1.0 |
| Advanced queries | Phase 0.3 |
| Field-aware filtering | Phase 0.3 |
| Import from other tools | v1.0 |
| Multi-device sync | Future |
| Alternative backends | Future |

---

## Implemented Ahead of Scope

The following items landed early during the M2 UX milestone:

- Entry revisions (`supersedes`)
- Optional passphrase session cache

## Exit Criteria

Phase 0.1 is complete when:

1. **Init works:** `jot init` creates a valid encrypted ledger
2. **Add works:** `jot add` opens $EDITOR, saves encrypted entry
3. **List works:** `jot list` shows entries with timestamps
4. **Search works:** `jot search "term"` finds matching entries
5. **Export works:** `jot export --json` produces valid JSON
6. **Tags work:** Entries can be created and filtered by tag
7. **Round-trip:** Ledger can be closed, reopened, and queried
8. **No plaintext leaks:** Verified via testing
9. **Crash safety:** Interrupted writes don't corrupt the jot

---

## Technical Milestones (from RFC-005)

> **Note**: These are the original technical milestones from RFC-005. The implementation
> plans in `docs/milestones/` (M1-PLAN.md, archived M2-PLAN, etc.) represent focused work
> packages that may span or reorder these technical milestones based on implementation
> learnings. See each M*-PLAN.md file for current scope and status.

| Milestone | Description |
|-----------|-------------|
| M0 | Project scaffold, dependencies, CI |
| M1 | Encryption layer (Age + SQLite in-memory) |
| M2 | Core operations (add, list, show) |
| M3 | Full-text search |
| M4 | Export (JSON, JSONL) |
| M5 | Tags |
| M6 | Testing and polish |

**Current Implementation Plan Mapping:**
- `M1-PLAN.md` — Encrypted Storage (covers technical M1)
- `archive/M2-PLAN.md` — UX Polish & First-Run Experience (cross-cutting UX improvements)
- `M3-PLAN.md` — Query & Export Stability
- `M4-PLAN.md` — Revisions, History, and Trust

---

## Format Compliance

Phase 0.1 implements format-spec.md v0.1 with these constraints:

- Format version: "0.1"
- UUID version: v7 (preferred)
- Timestamp precision: millisecond
- Character encoding: UTF-8
- Tag normalization: per §7.5
- Size limits: per §9.2

---

## Validation Strategy

Before Phase 0.2 begins:

1. Manual testing of all commands
2. Automated test suite for core operations
3. Security review (no plaintext leaks)
4. Format compliance verification
5. User feedback on CLI ergonomics

---

## References

- [planning.md](../planning.md) — Project vision and roadmap
- [format-spec.md](../design/format-spec.md) — File format specification
- [RFC-001](../RFC/RFC-001.md) — Storage & encryption (§5-7 locked)
- [RFC-003](../RFC/RFC-003.md) — CLI command taxonomy
- [RFC-004](../RFC/RFC-004.md) — Data model
- [RFC-005](../RFC/RFC-005.md) — Implementation milestones
