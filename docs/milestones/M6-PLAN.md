# Milestone 6: Desktop App & Shared App Layer (Phase 0.2+)

**Status**: Draft  
**Target**: Prepare the codebase for a desktop client that shares core logic with the CLI.

---

## Goals

- Define a shared “app layer” that encapsulates UX rules
- Keep `jot-core` storage/crypto as the single source of truth
- Ensure desktop app can read/write the same jot files safely

---

## Scope

### In Scope

- App-layer crate/module for shared UX rules
- API surface for desktop app (open jot, add/edit/list/search)
- Key management UX guidance (passphrase, keychain, keyfile)
- Minimal CLI refactor to use shared app layer

### Out of Scope

- Full desktop UI implementation
- Sync/multi-device merge
- Advanced query language

---

## Exit Criteria

- [ ] Desktop app plan documented (architecture, data flow)
- [ ] App-layer API defined and consumed by CLI
- [ ] Clear UX rules documented for both CLI and desktop
- [ ] Tests updated for shared app layer

---

## Implementation Steps

### 1. App Layer Definition

- [ ] Create a new crate (e.g., `jot-app`) or module under `jot-cli`
- [ ] Move prompt precedence rules, default handling, and validation helpers
- [ ] Define interfaces for:
  - `open_jot(config, passphrase)`
  - `add_entry(entry_type, template, input_fields, tags, compositions)`
  - `list_entries(filters)`
  - `search_entries(query, filters)`
  - `show_entry(id, history)`

### 2. CLI Refactor

- [ ] Replace CLI-only helpers with app-layer calls
- [ ] Keep CLI output formatting separate
- [ ] Ensure test-support flags still work

### 3. Desktop App Plan

- [ ] Document UI flows (init, unlock, add, list, search)
- [ ] Document key management UX (passphrase retry, cache, tier selection)
- [ ] Document template selection + enum prompting behavior

### 4. Testing

- [ ] App-layer unit tests for prompting rules and defaults
- [ ] Integration tests shared between CLI and desktop

---

## Notes

- The app layer should be UI-agnostic: it returns structured errors and prompt
  requirements, but does not print or render UI.
- Desktop app will plug in a UI on top of the same app-layer rules.
