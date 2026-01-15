# Jot

**A secure, encrypted, CLI-first personal journal and logbook**

[![CI](https://github.com/hotsaucejake/jot/actions/workflows/ci.yml/badge.svg)](https://github.com/hotsaucejake/jot/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

> **Status**: Milestone 5 (Compositions & Templates) — Complete

## Overview

Jot combines strong encryption at rest, structured queryable data, user-defined entry types, and a CLI-first workflow. It aims to be a secure, extensible personal data system for journaling, logging, and tracking anything over time.

### Core Principles

- **Security by default**: Everything encrypted at rest, no plaintext modes
- **User owns their data**: Local storage, no cloud requirement, no lock-in
- **CLI-first**: Designed for terminals, SSH, dotfiles, automation
- **Structure without rigidity**: Free-form writing, structured metrics, or hybrid
- **Future-proof**: Versioned schemas, explicit migrations, documented format

## Current Status: Phase 0.2 (Compositions & Templates)

The encrypted storage and CLI flows are functional with compositions and templates:

- [x] Age-encrypted SQLite storage (in-memory)
- [x] Schema initialization + metadata
- [x] Entry CRUD + FTS search
- [x] CLI init/add/list/search/show/check/export/backup
- [x] Compositions (semantic grouping across entry types)
- [x] Templates (reusable defaults stored in the jot file)
- [x] Template-first prompting for entry creation
- [x] CLI integration tests

### Available Commands

```bash
# Core commands
jot init                  # Initialize encrypted jot file
jot init --advanced       # Advanced init wizard (editor, timezone, cache, keyfile)
jot add <type>            # Add entry (prompts for fields)
jot add journal --body "" # Add inline entry
jot add journal --template <name>  # Use specific template
jot add journal --compose <name>   # Attach to composition
jot add journal --no-compose       # Skip composition attachment
jot list [type]           # List entries
jot list --json           # List entries as JSON
jot list --last 7d        # List recent entries
jot list --format plain   # Plain list output
jot list --history        # Include superseded revisions
jot search <query>        # Full-text search
jot search --type journal # Filter by entry type
jot search --json         # Search as JSON
jot search --format plain # Plain search output
jot search --history      # Include superseded revisions
jot show <id>             # Show entry by ID
jot show <id> --json      # Show entry as JSON
jot export                # Export data (portable, you own your data)
jot check                 # Integrity check
jot backup <dest>         # Backup jot file
jot lock                  # Clear passphrase cache
jot completions bash      # Generate shell completions

# Compositions (semantic grouping)
jot compositions create <name>           # Create composition
jot compositions create <name> --description "..."
jot compositions list                    # List all compositions
jot compositions list --json             # List as JSON
jot compositions show <name>             # Show composition details
jot compositions rename <old> <new>      # Rename composition
jot compositions delete <name>           # Delete composition
jot attach <entry-id> <composition>      # Attach entry to composition
jot detach <entry-id> <composition>      # Detach entry from composition

# Templates (reusable defaults)
jot templates create <name> --entry-type <type>  # Create template
jot templates create <name> --entry-type journal --defaults '{"body": "..."}'
jot templates create <name> --entry-type journal --set-default
jot templates list                       # List all templates
jot templates list --json                # List as JSON
jot templates show <name>                # Show template details
jot templates update <name> --defaults '{"body": "new default"}'
jot templates delete <name>              # Delete template
```

Environment variables:

```bash
JOT_PATH=/path/to/file.jot
JOT_PASSPHRASE="your passphrase"
JOT_CONFIG=/path/to/config.toml
```

## Compositions

Compositions are **semantic groupings** that can span multiple entry types. Use them to organize related entries around themes, projects, or topics.

```bash
# Create a composition for a research project
jot compositions create "research-paper" --description "PhD thesis research"

# Add entries and attach them to the composition
jot add journal --body "Literature review notes" --compose "research-paper"

# Or attach existing entries
jot attach <entry-id> "research-paper"

# View all entries in a composition
jot compositions show "research-paper"
```

**Key concepts:**
- Entries can belong to multiple compositions
- Compositions work across entry types (journal, bookmark, etc.)
- Use `--compose` during `add` or `attach` after creation
- Use `--no-compose` to skip automatic composition attachment

## Templates

Templates store **reusable defaults** for entry creation. They pre-fill field values and can be set as the default for an entry type.

```bash
# Create a template with default values
jot templates create "morning-journal" \
  --entry-type journal \
  --defaults '{"body": "Morning reflection:\n\n1. Grateful for:\n2. Focus today:\n3. Intention:"}' \
  --set-default

# Use template when adding entries
jot add journal --template "morning-journal"

# If set as default, it applies automatically
jot add journal  # Uses morning-journal template defaults
```

**Template JSON structure:**
```json
{
  "defaults": {
    "body": "Default text",
    "field_name": "default value"
  },
  "default_tags": ["tag1", "tag2"],
  "default_compositions": ["composition-id"],
  "prompt_overrides": {
    "field_name": "Custom prompt text"
  }
}
```

**Prompting rules:**
- No flags: prompts for all fields (template defaults pre-filled)
- Some flags: prompts only for missing required fields
- All flags provided: stores only provided values (no extra prompts)

## Building

```bash
# Build
cargo build

# Run tests
cargo test

# Check code quality
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings

# Install locally
cargo install --path crates/jot-cli
```

## Passphrase Requirements

- Minimum length: **8 characters**
- Must not be empty or whitespace-only

## Config Overview

Jot writes a config at `~/.config/jot/config.toml` by default. It includes:

- Jot path (`[jot].path`)
- Security tier selection (`[security].tier`)
- Passphrase cache TTL (`[security].passphrase_cache_ttl_seconds`)
- Keychain/keyfile settings
- Optional UI defaults (`[ui].editor`, `[ui].timezone`)

## Development Roadmap

### Phase 0.1 — Minimal Viable Journal ✓

- **M1**: Encrypted storage (Age + SQLite in-memory) ✓
- **M2**: Journal entries (`add`, `list`, `show`) ✓
- **M3**: Full-text search ✓
- **M4**: Export & backup ✓

Exit criteria: Can create, search, and export encrypted journal entries.

### Phase 0.2 — Structured Schemas, Templates, Compositions ✓

- **M5**: Compositions & Templates ✓
  - Compositions (semantic grouping across entry types)
  - Templates stored in the jot file (reusable defaults)
  - Template-first prompting for entry creation
  - Enum fields (single/multi-select)

Exit criteria: Can create compositions, define templates, and use template defaults during entry creation.

### Phase 0.3 — Query & Analysis

- Advanced queries (`--where` expressions)
- Entry revisions
- CSV export

### Phase 1.0 — Full Feature Set

- Format specification
- Migration tooling
- Import from other tools

## Architecture

```
jot/
├── crates/
│   ├── jot-core/      # Core library (storage, crypto, schemas)
│   └── jot-cli/       # CLI interface
├── docs/
│   ├── RFC/              # Design RFCs
│   ├── design/           # Format spec, threat model
│   └── milestones/       # Phase documentation
└── tests/                # Integration tests
```

### Design Documents

- [Planning](docs/planning.md) — Vision, principles, roadmap
- [RFC-001](docs/RFC/RFC-001.md) — Storage & encryption model
- [RFC-002](docs/RFC/RFC-002.md) — Entry types & schemas
- [RFC-003](docs/RFC/RFC-003.md) — CLI command taxonomy
- [RFC-004](docs/RFC/RFC-004.md) — Data model
- [RFC-005](docs/RFC/RFC-005.md) — Implementation plan
- [RFC-006](docs/RFC/RFC-006.md) — Compositions
- [Format Spec](docs/design/format-spec.md) — File format specification
- [Templates Spec](docs/design/templates.md) — Template storage and behavior

## For Developers

### New to the Project?

**Start here:**

1. **[CONTRIBUTING.md](CONTRIBUTING.md)** — Quick start guide for contributors
2. **[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)** — Complete development guide **(REQUIRED READING)**
3. **[docs/planning.md](docs/planning.md)** — Project vision and principles

### Development Standards

- **Testing is mandatory** — Every feature has tests, no exceptions
- **Security by default** — Encryption always, no plaintext modes
- **Rust best practices** — See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)
- **Code quality** — Format, lint, test before every commit

```bash
cargo fmt --all                                          # Format
cargo clippy --all-targets --all-features -- -D warnings # Lint
cargo test                                               # Test
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please read the RFCs and design documents first to understand the architecture and goals.

Before submitting a PR:
1. Run `cargo fmt --all`
2. Run `cargo clippy --all-targets --all-features -- -D warnings`
3. Run `cargo test`
4. Update relevant documentation

## Daily Workflow

### Typical Commands

```bash
# Build
cargo build

# Run all tests
cargo test

# Run only core library tests
cargo test -p jot-core

# Run only CLI tests
cargo test -p jot-cli
```

### Manual Testing Loop

```bash
# Basic workflow
jot init ./test.jot
jot add journal --body "Hello"
jot list --json
jot search "Hello"
jot show <id>
jot export --json
jot backup ./test.jot.bak

# Compositions workflow
jot compositions create "my-project"
jot add journal --body "Project notes" --compose "my-project"
jot compositions show "my-project"

# Templates workflow
jot templates create "daily" --entry-type journal --defaults '{"body": "Today:"}' --set-default
jot add journal  # Uses template defaults
jot templates list --json
```

### Common Environment Variables

- `JOT_PATH`: default jot file path.
- `JOT_PASSPHRASE`: non-interactive passphrase (useful for tests/scripts).

## Acknowledgments

Inspired by tools like `jrnl`, `pass`, and the personal data management philosophy of offline-first, user-owned systems.
