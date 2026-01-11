# Ledger File Format Specification

**Status:** Draft
**Applies to:** Ledger v0.1+
**Audience:** Users, contributors, future maintainers
**Purpose:** Define the stable, long-term on-disk format of a Ledger

---

## 1. Purpose & Scope

This document defines the **Ledger file format** — the data that persists when a Ledger is closed and stored.

It is intentionally:

* **Independent of implementation language**
* **Independent of storage backend details**
* **Explicit about stability guarantees**

This spec exists to support Ledger’s long-term goal:

> Data created today should remain readable, decryptable, and interpretable in the future.

---

## 2. Terminology

| Term           | Meaning                                               |
| -------------- | ----------------------------------------------------- |
| Ledger         | A logical container for encrypted user data           |
| Entry          | A single immutable record created by the user         |
| Entry Type     | A schema defining fields for entries                  |
| Template       | Reusable defaults for creating entries                |
| Composition    | A semantic grouping of entries                        |
| Backend        | A concrete storage implementation (e.g. Age + SQLite) |
| Format Version | Version of this specification                         |

---

## 3. High-Level Model (Backend-Agnostic)

At rest, a Ledger consists of:

* **Encrypted payload**
* **Defined decryption parameters**
* **Versioned internal data model**

Conceptually:

```
Ledger
├── Metadata
├── Entry Types (schemas)
├── Templates (v0.2+)
├── Entries
├── Compositions (v0.2+)
└── Indexes (rebuildable)
```

Only the **encrypted payload** is persisted to disk.

---

## 4. Stability Guarantees

### 4.1 Stable Guarantees (v0.1+)

The following are guaranteed stable across Ledger versions:

* Entry identity (UUID)
* Entry immutability
* Entry timestamps
* Entry schema version references
* Composition identity (UUID)
* Template identity (UUID)
* JSON field semantics
* Exported JSON / JSONL formats

### 4.2 Explicitly Unstable / Evolving

The following may change with format versions:

* Internal SQLite schema
* Indexing strategy
* Query acceleration structures
* Temporary file behavior
* Backend-specific optimizations

**Rule:**
Unstable components must be **rebuildable** from stable data.

---

## 5. Cryptographic Envelope (Phase 0.1)

### 5.1 Encryption Overview

In Phase 0.1, a Ledger file is:

```
age_encrypt(
  derived_key,
  sqlite_database_bytes
)
```

Where:

* `derived_key` is produced via Argon2id
* `sqlite_database_bytes` represent the full logical ledger state

No plaintext user data is written to disk.

---

### 5.2 Key Derivation

* Algorithm: **Argon2id**
* Parameters:

    * Memory: implementation-defined
    * Iterations: implementation-defined
    * Salt: random, per-ledger

**Important:** Key derivation parameters are stored in the **Age file header** (plaintext), not inside the encrypted payload. This is how Age passphrase recipients work — the scrypt parameters are embedded in the recipient stanza, allowing decryption without a separate metadata file.

**Security note:** Storing KDF parameters in plaintext does not weaken confidentiality. This is standard practice for passphrase-based encryption (see: PKCS#5, Argon2 reference implementation). The salt prevents precomputation attacks; the iteration/memory parameters define the cost. An attacker with the file still cannot decrypt without the passphrase.

The encrypted payload contains only the SQLite database. No bootstrap metadata is required outside the Age file itself.

---

### 5.3 Encryption Tooling

* Age recipients are passphrase-based
* Ledger does not require system GPG configuration
* Encryption is authenticated; tampering causes decryption failure

---

## 6. Phase 0.1 Backend: Age + SQLite

### 6.1 Payload Contents

The decrypted payload is a **SQLite database** containing:

* Entries
* Entry Types
* Metadata
* Indexes (FTS, tag helpers)

The SQLite schema itself is **not** the format — it is an implementation detail.

---

### 6.2 In-Memory Operation

Ledger implementations:

* Deserialize SQLite into memory
* Perform all operations in-memory
* Re-serialize and re-encrypt on close

At worst, a crash may lose recent changes, but must never corrupt the ledger.

---

## 7. Core Data Entities (Logical)

### 7.1 Entry

An Entry is immutable and contains:

* `id` (UUID v7 preferred, v4 acceptable)
* `entry_type_id` (UUID, references Entry Type `id`)
* `schema_version` (integer, references Entry Type `version`)
* `created_at` (UTC, ISO-8601, millisecond precision)
* `data` (JSON object)
* `tags` (array of normalized strings — see §7.6)
* `device_id` (UUID, identifies creating device)
* `supersedes` (optional UUID, for revisions)
* `deleted_at` (reserved for v1.0, soft delete timestamp; **MUST be null in v0.x**)

**Entry Type Reference Rule:** Entries reference Entry Types by `id` + `version`, not by name. This allows Entry Type renames without breaking existing entries and ensures import/export stability.

**Revision Semantics:**

When `supersedes` references another entry:
* The referenced entry becomes "superseded" (hidden from default queries)
* The new entry is the "current" version
* Orphaned revisions (where `supersedes` references a non-existent ID) are permitted — this supports merge scenarios where entries arrive out of order
* Clients must handle orphaned revisions gracefully (treat as standalone entries)

---

### 7.2 Entry Type

An Entry Type defines:

* `id` (UUID v7 preferred, v4 acceptable)
* `name` (unique string identifier)
* `version` (integer, incremented on schema changes)
* `created_at` (UTC, ISO-8601, millisecond precision)
* `device_id` (UUID, identifies creating device)
* `fields` (JSON array of field definitions)
* `default_composition_id` (optional UUID, references Composition)
* `validation` (JSON object, validation rules)

Entry Types are versioned and append-only. Each version is a separate record; the `name` + `version` pair is unique.

---

### 7.3 Template (Introduced v0.2)

A Template provides reusable defaults for creating entries:

* `id` (UUID v7 preferred, v4 acceptable)
* `name` (unique string identifier)
* `entry_type_id` (UUID, references Entry Type `id`)
* `version` (integer, incremented on template changes)
* `created_at` (UTC, ISO-8601, millisecond precision)
* `device_id` (UUID, identifies creating device)
* `defaults` (JSON object, default values)
* `default_tags` (JSON array)
* `default_compositions` (JSON array of UUIDs)
* `prompt_overrides` (JSON object)

Templates are associated to entry types via a mapping table:

* `entry_type_templates` (entry_type_id, template_id, active)

Best-practice constraints:

* One active mapping per entry type.
* New defaults deactivate prior mappings.

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

Templates are versioned and append-only. Each version is a separate record; the `name` + `version`
pair is unique.

---

### 7.4 Composition (Introduced v0.2)

A Composition is a semantic grouping:

* `id` (UUID v7 preferred, v4 acceptable)
* `name` (string)
* `description` (optional string)
* `metadata` (JSON object)
* `created_at` (UTC, ISO-8601, millisecond precision)
* `device_id` (UUID, identifies creating device)
* Many-to-many relationship with Entries (via join table)

Compositions do not define schema and do not own entries. Deleting a composition does not delete associated entries.

---

### 7.5 Metadata Table

The metadata table stores ledger-level configuration:

| Key | Type | Mutability | Description |
|-----|------|------------|-------------|
| `format_version` | string | Authoritative | Format spec version (e.g., "0.1") |
| `device_id` | UUID | Authoritative | This device's identifier |
| `created_at` | ISO-8601 | Authoritative | When this ledger was created |
| `last_modified` | ISO-8601 | Informational | Last write timestamp |

**Mutability rules:**
- **Authoritative** keys define ledger identity and format. They must not be modified after creation except by explicit migration.
- **Informational** keys are updated automatically during normal operation. Multiple tools touching the same ledger may overwrite these values.

Additional keys may be added in future versions. Unknown keys must be preserved on read/write.

---

### 7.6 Tag Normalization

Tags are normalized before storage:

* Converted to lowercase
* Whitespace trimmed
* Empty tags rejected
* Maximum length: 128 UTF-8 bytes
* Allowed characters: alphanumeric, hyphen, underscore, colon
* Invalid characters are rejected (not silently stripped)

Duplicate tags on a single entry are deduplicated.

---

## 8. Indexes & Derived Data

Indexes (e.g. full-text search, tag indexes):

* Are derived from stable data
* Are rebuildable at any time
* Must not be treated as the source of truth

Loss of indexes must not imply data loss.

---

## 9. Size & Encoding Constraints

### 9.1 Character Encoding

All text data is UTF-8. No other encodings are supported.

### 9.2 Size Limits

| Field | Maximum Size |
|-------|--------------|
| Entry `data` JSON | 1 MB |
| Entry Type `fields` JSON | 64 KB |
| Template `defaults` JSON | 64 KB |
| Single tag | 128 bytes |
| Tags per entry | 100 |
| Composition `metadata` JSON | 64 KB |
| Entry Type `name` | 64 bytes |
| Template `name` | 256 bytes |
| Template `description` | 4 KB |
| Composition `name` | 256 bytes |
| Composition `description` | 4 KB |

These limits are enforced at write time. Implementations may support larger values but must not require them.

### 9.3 Empty Ledger

A valid empty ledger contains:
* Metadata table with required keys (format_version, device_id, created_at)
* Zero entries
* Zero entry types
* Zero templates
* Zero compositions

Empty ledgers are valid and must be handled correctly.

---

## 10. Format Versioning

### 10.1 Format Version

Each Ledger has a **format version** stored in metadata.

* Incremented only when stable guarantees change
* Backward-compatible readers must exist for all released versions

### 10.2 Backend Versioning

Backends may evolve independently as long as:

* Stable guarantees are preserved
* Export formats remain compatible

---

## 11. Export Guarantees

Ledger implementations must support exporting:

* Entries
* Entry Types
* Templates (v0.2+)
* Compositions (v0.2+)

In:

* JSON
* JSONL

Exported data must be sufficient to reconstruct a Ledger.

### 11.1 Export Schema (Best Practice)

JSON export should include a top-level object with:

```json
{
  "format_version": "0.2",
  "exported_at": "2026-01-07T14:32:11Z",
  "entry_types": [],
  "templates": [],
  "compositions": [],
  "entries": []
}
```

JSONL export should emit one record per line with a `kind` field:

```json
{ "kind": "entry_type", "record": { ... } }
{ "kind": "template", "record": { ... } }
{ "kind": "composition", "record": { ... } }
{ "kind": "entry", "record": { ... } }
```

This keeps exports forward-compatible and easy to stream.

---

## 12. Integrity & Recovery

* Encryption ensures confidentiality and tamper detection
* Entries are append-only
* Backups are whole-ledger operations
* Recovery tools must favor **data preservation over convenience**

---

## 13. Non-Goals

This specification does **not** define:

* Synchronization protocols
* Merge semantics
* Conflict resolution strategies
* GUI formats

These are layered above the format.

---

## 14. Future Extensions

Planned extensions include:

* Alternative storage backends
* Attachment blobs
* Rich queries
* Partial export/import
* Multi-device reconciliation

These must preserve the stable guarantees defined here.

---

## 15. Summary

The Ledger format is:

* Encrypted by default
* Append-only
* Explicitly versioned
* Backend-agnostic
* Designed for long-term durability

This specification exists so Ledger can evolve **without betraying user trust**.
