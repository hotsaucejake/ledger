# CLI UI Architecture Guide

This document describes a scalable module layout and implementation plan for Ledger's CLI UI layer. The goal is to build the UI foundation first, then migrate existing commands to the shared primitives.

## Goals

- Centralize all output decisions (mode, color, width, unicode).
- Make UI behavior consistent across commands.
- Keep JSON/plain outputs stable and testable.
- Support wizard-style flows without a full TUI.
- Make it easy to expand with new commands.

## High-Level Design

Create a dedicated UI module with a small set of well-defined responsibilities:

- **Context**: environment, TTY, width, color, unicode.
- **Theme**: badge tokens, palette, symbols.
- **Render**: tables, headers, receipts, hints, and formatted text.
- **Prompt**: wizard steps and guided flows (interactive only).
- **Mode routing**: a single place to decide json/plain/pretty.

The UI module should be dependency-light and safe to call from any command.

## Suggested Module Layout

```
crates/ledger-cli/src/ui/
├── mod.rs              # Public API exports
├── context.rs          # TTY detection, width, color/unicode flags
├── mode.rs             # OutputMode and routing rules
├── theme.rs            # Colors, symbols, badge text
├── render.rs           # Header/footer/table/kv/badge/receipt
├── prompt.rs           # Wizard and guided prompts
├── format.rs           # String utilities (truncate, wrap, align)
└── progress.rs         # Spinners, progress bars, step updates
```

Optional follow-up: move `output/` under `ui/` or re-export it from `ui`.

## Public API (example)

```
ui::Context::from_env()
ui::OutputMode::from_flags(...)
ui::render::header(...)
ui::render::table(...)
ui::render::receipt(...)
ui::render::hint(...)
ui::prompt::init_wizard(...)
ui::prompt::add_wizard(...)
ui::progress::spinner(...)
```

Keep the API small and stable. Commands should not format strings directly.

## Responsibilities by Module

### `context.rs`

- Detect TTY for stdin/stdout/stderr.
- Determine terminal width.
- Resolve color/unicode/animation enablement.
- Expose a `Context` struct used everywhere else.

```
struct Context {
  is_tty: bool,
  color: bool,
  unicode: bool,
  width: usize,
  mode: OutputMode,
}
```

### `mode.rs`

- Parse and validate output mode flags.
- Enforce rules: JSON exclusive, plain for non-TTY, etc.

```
enum OutputMode { Json, Plain, Pretty }
```

### `theme.rs`

- Central definitions for badges, symbols, and colors.
- Provides ASCII and Unicode variants for icons.

```
struct Theme {
  ok: SymbolPair,
  warn: SymbolPair,
  err: SymbolPair,
  info: SymbolPair,
}
```

### `render.rs`

- Core building blocks:
  - `header`, `divider`, `kv`, `table`, `hint`, `receipt`, `badge`.
- Only renders in pretty/plain. JSON is handled separately.
- Never performs I/O; returns strings (or `Vec<String>`).

### `format.rs`

- Truncation, alignment, padding, and wrapping helpers.
- Width-aware formatting (truncate summary, align columns).
- Should be deterministic for tests.

### `prompt.rs`

- Wizard flows for `init`, `add`, and missing-arg prompts.
- Strictly gated by `Context::is_tty`.
- Returns structured values (not strings).

### `progress.rs`

- Spinners and progress bars for pretty mode only.
- Provides no-op implementations for non-TTY or plain.

## Output Routing Rules (single source of truth)

- `--json` only, never mixed with other output.
- `--format plain` forces plain.
- Pretty only when stdout is a TTY and color is allowed.
- `TERM=dumb` forces plain.
- `--ascii` forces ASCII symbols.

## Command Integration Pattern

Recommended flow in each command:

1) Build `Context` and resolve `OutputMode`.
2) If `json`, call JSON formatter and return.
3) If interactive and missing args, invoke `prompt` wizard.
4) Render output using `render` helpers.

Avoid ad-hoc printing from commands. Keep output in one place.

## Incremental Migration Plan

1) Add `ui/` module and wire `Context` + `OutputMode`.
2) Migrate `list`, `show`, `search` to `ui::render`.
3) Migrate `init` to `ui::prompt`.
4) Add `progress` for `check`/`backup`/`export`.
5) Convert remaining commands.

## Testing Guidance

- Snapshot test plain output only.
- JSON outputs are strict and schema-validated.
- Pretty output tests should assert key strings only.

## Best Practices

- No output in JSON mode besides JSON.
- No ANSI in plain mode.
- Always use `Context` for width, color, unicode decisions.
- Keep UI strings centralized when possible.
- Favor returning strings over printing in helpers.

## Notes

This document complements `docs/design/cli-ux-spec.md`. Use it as the implementation blueprint for the UI layer.
