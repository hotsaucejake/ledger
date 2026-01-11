# Project Planning — Phase 0

*(Vision, Principles, Scope, and Non-Negotiables)*

## 0.1 Project Working Name

> **Ledger**
> *A secure, structured, CLI-first personal journal and logbook.*

---

## 0.2 Problem Statement

Most journaling tools fall into one of three buckets:

1. **Free-form text journals**
   * Great for writing
   * Terrible for structure, metrics, querying, or long-term analysis

2. **Structured trackers (habits, todos, metrics)**
   * Rigid schemas
   * Not extensible
   * Often SaaS-only, closed, or cloud-dependent

3. **CLI tools**
   * Powerful but often:
     * plaintext-only
     * unencrypted
     * not extensible
     * hostile to long-term data evolution

**Ledger aims to combine:**

* Strong encryption at rest
* Structured, queryable data
* User-defined entry types
* CLI-first workflow
* Portable, Git-friendly storage
* Long-term data integrity

Tools like `jrnl` (GPG-encrypted, CLI) and `pass` (git + GPG model) cover parts of this space. Ledger's differentiation is *compositional*: combining structured schemas with encryption and queryability in a single, coherent system.

---

## 0.3 Core Vision (1 Sentence)

> **Ledger is a CLI-first, encrypted, extensible personal data system for journaling, logging, and tracking anything over time.**

If a feature does not serve *this* sentence, it does not belong.

---

## 0.4 Design Philosophy (Non-Negotiables)

These are **invariants**, not preferences.

### 1. User Owns Their Data — Fully

* Data lives **locally**
* No cloud requirement
* No proprietary formats
* Exportable at any time
* No lock-in, ever

### 2. Security Is Not Optional

* Encryption is **on by default**
* No plaintext storage modes
* Explicit key derivation
* No "security through obscurity"

If it's stored, it's encrypted.

### 3. Structure Without Rigidity

Ledger must support:

* Free-form writing
* Structured metrics
* Hybrid entries (text + fields)
* User-defined schemas

**Structure is opt-in, not enforced globally.**

### 4. CLI Is the Primary Interface

* GUI is optional, future, or third-party
* Everything must be possible via CLI
* Designed for terminals, SSH, dotfiles, automation

### 5. One Logical Ledger, Portable Storage

* A ledger represents a single, coherent data set
* Implementation may be a single file, a directory, or an archive
* Safe to back up, sync, or commit to Git
* No scattered state or hidden dependencies

The user thinks of "my ledger" as one thing, regardless of how it's stored.

### 6. Future-Proof by Design

* Versioned schemas
* Explicit migrations
* Backward compatibility
* Documented file format

We design for durability, not disposability. Data should remain accessible through explicit migration tooling as the system evolves.

### 7. Conflict-Aware from Day One

Even without sync in v1, the data model must support eventual multi-device use:

* Entries have globally unique IDs (UUIDs)
* Entries are append-only (no in-place mutation)
* Metadata includes device/client identifiers
* Mergeability is a design constraint, not a future feature

---

## 0.5 Explicit Non-Goals (Important)

Ledger will **not**:

* Be a social product
* Sync via a hosted service
* Compete with Notion/Obsidian (different problem spaces)
* Be optimized for mobile-first usage
* Require accounts or telemetry
* Use AI as a core dependency

These exclusions protect focus.

---

## 0.6 Core Use Cases (Canonical)

Ledger must handle these **first-class**:

### A. Free-Form Journaling

```bash
ledger add journal
```

* Opens `$EDITOR`
* Timestamped
* Taggable
* Searchable

### B. Structured Logs (Metrics)

Example: daily weight

```bash
ledger add weight
```

Prompted input:

* date (default: today)
* value (float)

Queryable later.

### C. Task / Todo Tracking

```bash
ledger add todo
ledger list todo --open
ledger complete todo <id>
```

### D. Hybrid Entries

Example:

* Daily reflection + mood + stress level

### E. Search & Analysis

```bash
ledger search "anxious"
ledger list weight --last 90d
ledger export weight --json
```

---

## 0.7 Audience

Primary:

* Developers
* CLI power users
* Privacy-conscious users
* Quantified-self enthusiasts
* Long-term note keepers

Secondary:

* Researchers
* Writers
* Anyone who thinks in systems

---

## 0.8 Open Source Posture

* License: **MIT or Apache-2.0**
* Fully offline usable
* Contributor-friendly
* Explicit design docs
* Stable core, extensible edges

We optimize for **clarity and correctness**, not growth hacking.

---

## 0.9 Planning Roadmap (Layered)

We proceed in **layers**, validating core assumptions before adding complexity.

### Phase 0.1 — Minimal Viable Journal

* Encrypted storage (simplest viable backend)
* Journal entries only (text + timestamp + tags)
* Append-only, UUID-based entries
* `add`, `list`, `search`, `show`, `export`, `backup`, `check` commands
* Prove the encryption + CLI UX works

**Goal:** Validate build story and core experience before committing to complex dependencies.

Note: Some UX and revision features landed early via M2; see `docs/milestones/M2-PLAN.md`.

### Phase 0.2 — Structured Schemas & Compositions

* User-defined entry types
* Schema validation
* Prompted field input
* Compositions (semantic entry grouping)
* Entry type default compositions

### Phase 0.3 — Query & Analysis

* Field-aware filtering
* Advanced search
* Export improvements

### Phase 1.0 — Full Feature Set

* Complete RFC feature set
* Migration tooling
* Format documentation
* Import from other tools

---

## 0.10 Critical Design Decisions (Locked Early)

These must be decided before implementation begins:

1. **Storage Abstraction**
   * Define a storage engine interface
   * Allow multiple backends (encrypted SQLite, GPG + files, etc.)
   * Don't commit to SQLCipher until build story is validated

2. **Entry Identity**
   * UUIDs, not auto-increment IDs
   * Device ID in metadata
   * No in-place mutation

3. **Schema Creation**
   * Explicit by default (`ledger types create`)
   * Optional schema-on-first-use with guardrails
   * Typo detection / confirmation prompts

4. **Format Documentation**
   * Required before v1
   * Independent of implementation
   * Enables long-term tooling

---

## Where We Go Next

The RFCs that follow define the system in detail:

1. **RFC-001: Storage & Encryption Model**
   * Storage abstraction interface
   * Encryption options and tradeoffs
   * Threat model

2. **RFC-002: Entry Types & Schemas**
   * Schema definition
   * Validation
   * Evolution with guardrails

3. **RFC-003: CLI Command Taxonomy**
   * Command structure
   * Explicit type creation flow
   * Query expression planning

4. **RFC-004: Database Schema**
   * UUID-based entries
   * FTS specification
   * Conflict-aware metadata

5. **RFC-005: Implementation Plan**
   * Revised milestones
   * Storage abstraction in architecture
   * Format documentation requirement

6. **RFC-006: Compositions** (Phase 0.2+)
   * Semantic entry grouping
   * Many-to-many entry relationships
   * Entry type default compositions
