# Ledger

**A secure, encrypted, CLI-first personal journal and logbook**

[![CI](https://github.com/hotsaucejake/ledger/actions/workflows/ci.yml/badge.svg)](https://github.com/hotsaucejake/ledger/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

> **Status**: Milestone 0 (Project Skeleton) — Core functionality not yet implemented

## Overview

Ledger combines strong encryption at rest, structured queryable data, user-defined entry types, and a CLI-first workflow. It aims to be a secure, extensible personal data system for journaling, logging, and tracking anything over time.

### Core Principles

- **Security by default**: Everything encrypted at rest, no plaintext modes
- **User owns their data**: Local storage, no cloud requirement, no lock-in
- **CLI-first**: Designed for terminals, SSH, dotfiles, automation
- **Structure without rigidity**: Free-form writing, structured metrics, or hybrid
- **Future-proof**: Versioned schemas, explicit migrations, documented format

## Current Status: Milestone 0 ✓

The project skeleton is complete:

- [x] Rust workspace structure (`ledger-core` + `ledger-cli`)
- [x] CLI command taxonomy implemented (Phase 0.1 commands)
- [x] `ledger --help` works
- [x] CI pipeline (build, test, clippy, fmt)
- [x] Cross-platform tested (Linux, macOS)

### Available Commands (not yet functional)

```bash
ledger init                  # Initialize encrypted ledger
ledger add <type>            # Add entry
ledger list [type]           # List entries
ledger search <query>        # Full-text search
ledger show <id>             # Show entry by ID
ledger export                # Export data
ledger check                 # Integrity check
ledger backup <dest>         # Backup ledger
```

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
cargo install --path crates/ledger-cli
```

## Development Roadmap

### Phase 0.1 — Minimal Viable Journal

- **M1**: Encrypted storage (Age + SQLite in-memory)
- **M2**: Journal entries (`add`, `list`, `show`)
- **M3**: Full-text search
- **M4**: Export & backup

Exit criteria: Can create, search, and export encrypted journal entries.

### Phase 0.2 — Structured Schemas

- User-defined entry types
- Schema creation with guardrails
- Compositions (semantic grouping)

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
ledger/
├── crates/
│   ├── ledger-core/      # Core library (storage, crypto, schemas)
│   └── ledger-cli/       # CLI interface
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

## Acknowledgments

Inspired by tools like `jrnl`, `pass`, and the personal data management philosophy of offline-first, user-owned systems.
