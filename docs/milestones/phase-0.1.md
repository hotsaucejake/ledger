# Phase 0.1 — Minimal Viable Journal

**Status:** Scope Frozen
**Goal:** Validate encryption + CLI UX before committing to complex dependencies

---

## Scope Statement

Phase 0.1 delivers the **smallest useful encrypted journal**. It proves the core experience works before adding structured schemas, compositions, or advanced queries.

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

**Not implemented:**
- `supersedes` (revision support deferred)
- `deleted_at` (soft delete deferred)
- Multi-entry type CLI (Phase 0.2)

### CLI Commands

| Command | Description |
|---------|-------------|
| `ledger init` | Create new encrypted ledger |
| `ledger add` | Add journal entry (opens $EDITOR) |
| `ledger add --body "..."` | Add inline entry |
| `ledger add --tags foo,bar` | Add with tags |
| `ledger list` | List recent entries |
| `ledger list --limit N` | List N entries |
| `ledger list --last 7d` | List recent entries |
| `ledger list --tag foo` | Filter by tag |
| `ledger list --json` | Output as JSON |
| `ledger search "term"` | Full-text search |
| `ledger search --type journal` | Filter by entry type |
| `ledger show <id>` | Show single entry |
| `ledger export --json` | Export all entries as JSON |
| `ledger export --jsonl` | Export as JSONL |
| `ledger backup <dest>` | Backup ledger file |

### Full-Text Search

- SQLite FTS5 on entry body
- Simple query syntax (phrase matching)
- No advanced operators (Phase 0.3)

### Security

- Passphrase prompted at each operation
- No passphrase caching in Phase 0.1
- Age encryption with scrypt KDF
- In-memory SQLite (no plaintext temp files)
- Atomic writes with backup

**Passphrase requirements:**
- Minimum length: 12 characters
- Must not be empty or whitespace-only

---

## Explicitly Out of Scope

These are **not bugs** in Phase 0.1. They are intentionally deferred.

| Feature | Deferred To |
|---------|-------------|
| Custom entry types | Phase 0.2 |
| Schema creation (`ledger types create`) | Phase 0.2 |
| Compositions | Phase 0.2 |
| Entry revisions (`supersedes`) | Phase 0.2 |
| Soft delete | v1.0 |
| Advanced queries | Phase 0.3 |
| Field-aware filtering | Phase 0.3 |
| Passphrase caching | Future |
| Import from other tools | v1.0 |
| Multi-device sync | Future |
| Alternative backends | Future |

---

## Exit Criteria

Phase 0.1 is complete when:

1. **Init works:** `ledger init` creates a valid encrypted ledger
2. **Add works:** `ledger add` opens $EDITOR, saves encrypted entry
3. **List works:** `ledger list` shows entries with timestamps
4. **Search works:** `ledger search "term"` finds matching entries
5. **Export works:** `ledger export --json` produces valid JSON
6. **Tags work:** Entries can be created and filtered by tag
7. **Round-trip:** Ledger can be closed, reopened, and queried
8. **No plaintext leaks:** Verified via testing
9. **Crash safety:** Interrupted writes don't corrupt the ledger

---

## Technical Milestones (from RFC-005)

> **Note**: These are the original technical milestones from RFC-005. The implementation
> plans in `docs/milestones/` (M1-PLAN.md, M2-PLAN.md, etc.) represent focused work
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
- `M2-PLAN.md` — UX Polish & First-Run Experience (cross-cutting UX improvements)
- `M3-PLAN.md` — TBD
- `M4-PLAN.md` — TBD

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
