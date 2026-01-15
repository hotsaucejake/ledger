# CLI UX Spec

This document defines a consistent, polished CLI UX for Jot. It is intended as a standalone guide for design and implementation decisions across commands.

## Goals

- Make the CLI feel cohesive, premium, and intentional.
- Keep output readable and stable for automation and logs.
- Preserve JSON purity and never mix it with human formatting.
- Improve first-run and daily workflows without a full TUI.

## Non-goals

- No interactive TUI (no full-screen UI, no mouse).
- No breaking changes to JSON schema outputs.
- No mandatory dependencies that force color/ANSI in non-TTY contexts.

## Output Modes

Jot supports three output modes:

1) **json**
- Always machine-only.
- Must print only JSON (no banners, hints, or extra lines).
- Errors must still be printed to stderr.

2) **plain**
- Minimal, stable, log-friendly, and grep-friendly.
- No colors, no Unicode-only symbols, no spinners.
- Output should be deterministic and compatible with snapshot tests.

3) **pretty** (default on TTY)
- Intended for humans.
- Uses spacing, table layout, badges, and limited color.
- Respects `NO_COLOR`, `--no-color`, and `TERM=dumb`.
- Should be turned off automatically when stdout is not a TTY.

### Routing rules

- `--json` overrides everything and is exclusive.
- `--format plain` forces plain output even on a TTY.
- Pretty output only when stdout is a TTY and color is allowed.
- `--ascii` forces ASCII-only symbols in pretty mode.

## Visual System

### Standard screen structure (pretty)

Each command should follow a predictable rhythm:

1) **Header line**
- `Jot` + command + jot path (or "default")
- Optional status line (lock/cache state) if relevant

2) **Primary result block**
- Table, list, body, or action results

3) **Footer hints**
- 1-3 next-step hints max
- Include `jot <cmd> --help` when a user seems stuck

### Badge language

Use consistent severity language and compact badges.

- Success: `[OK]` (optional unicode: checkmark)
- Warning: `[WARN]` (optional unicode: warning)
- Error: `[ERR]` (optional unicode: cross)
- Info: `[INFO]` (optional unicode: info)

Badges should be short and always precede a sentence or outcome.

### Color palette (pretty)

Use restrained colors:

- Labels and metadata: dim
- Values: normal/bright
- Warning: yellow-ish
- Error: red-ish
- Success: green-ish

Avoid rainbow tables. Fewer colors signal quality.

### Whitespace and alignment

- Prefer compact spacing, but keep sections visually separated.
- Align columns when possible.
- Keep headers small (no ASCII art banners).

### Animation and progress (TTY only)

Animations are optional polish and must never interfere with automation.

- Only show spinners/progress bars when stdout is a TTY.
- Never show animated output in `--plain`, `--json`, `--no-color`, or `TERM=dumb`.
- Prefer low‑motion patterns (8–12 fps) to reduce flicker over SSH.
- Always fall back to a single static line on non‑TTY.

Recommended patterns:

- **Spinner + status** for indeterminate work:
  - `Checking...` with a spinner and short status text.
- **Progress bar** for known work:
  - `Writing backup... 65%` with ETA if available.
- **Step list** that updates in place:
  - `- Schema integrity: [OK]` as each step completes.
- **Subtle dots** for short waits:
  - `Unlocking...` with 1–3 dots cycling.

Never animate multi‑line output. Keep updates to a single line unless the command is explicitly in progress mode.

Default animation palette (pretty mode):

- Spinner frames (Unicode): `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`
- Spinner frames (ASCII): `|/-\\`
- Progress bar: `[{bar}] {percent}%` with a compact width (20–30 cols)
- Step status markers: `[OK]`, `[WARN]`, `[ERR]`

## Output Primitives

Provide reusable primitives so command output stays consistent.

- `ui::header(cmd, context)`
- `ui::badge(kind, text)`
- `ui::table(rows, columns)`
- `ui::kv(key, value)`
- `ui::hint(text)`
- `ui::receipt(title, lines)`
- `ui::divider()`

All commands should compose these primitives.

## Global Behavior Rules

- Pretty only on TTY and when colors are allowed.
- Respect `NO_COLOR`, `--no-color`, and `TERM=dumb`.
- Respect terminal width for truncation and tables.
- Use ASCII-only symbols when `--ascii` is set.
- Never emit a spinner or interactive prompt if stdin/stdout is not a TTY.

## Command UX Specs

### `jot init`

#### Preflight (critical)
- If the jot file exists, do this before any prompt.
- Show a clear message with safe options:
  - list, check, open, or `--force`.

#### Basic wizard

- Step-based flow with 2-4 steps max:
  1) Path (default suggested)
  2) Passphrase + confirmation
  3) Review

- Final receipt:
  - Path, tier, cache TTL
  - Next commands (add/list/search)

#### Advanced wizard

- Sectioned flow:
  - Storage (path, cache TTL)
  - Editor (detect `$EDITOR`)
  - Timezone (auto-detect + override)
  - Keyfile/keychain (if supported)
  - UI defaults (pretty/plain, date format)

- Review screen shows resulting config values.

### `jot add <type>`

- If `<type>` missing and interactive, prompt for a selection.
- Editor-first for body; allow inline `--body`.
- If no field flags are provided and TTY is available, run a guided wizard:
  - show template selection (if any)
  - prompt missing required fields in order
  - allow a final review step before write
- After add, show a compact receipt:
  - short ID, type, timestamp, tags count
- Validation errors should point to the exact field.

### `jot list [type]`

Pretty mode:
- Short IDs (first 8-10 chars).
- Columns: ID, created, type, summary, tags.
- Header includes scope (e.g., "last 7d") and count.
- Footer hints: `jot show <id>`, `jot search "term"`.

Plain mode:
- Single line per entry with stable columns.

### `jot show <id>`

Pretty mode:
- Header with ID, type, created/updated, tags.
- Body with light spacing.
- Optional: render markdown lightly; otherwise plain text.

### `jot search <query>`

Pretty mode:
- Show results with snippet preview.
- Highlight matches if possible, but keep subtle.
- Show applied filters in header.
- If no results, show tips (fewer terms, quotes, etc.).

### `jot check`

- Step list with progress indicators in pretty mode.
- On failure, show the check name, a short explanation, and a suggested fix.

### `jot export` / `jot backup <dest>`

- Confirm overwrite unless `--force`.
- Progress for large operations (entries/bytes).
- Receipt on completion (path, size, duration).

### `jot lock`

- Clear success message with cache/TTL state.

### `jot completions`

- Default output is the raw completion script.
- Only print help/pretty info when explicitly requested and on TTY.

## Mocked Outputs (Pretty + Plain)

### `jot init` (first run)

Pretty (TTY):

```text
Jot · init
Path: /home/user/.local/share/jot/data.jot

1/3  Choose security tier
  > Passphrase only (recommended)
    Passphrase + OS keychain

2/3  Create passphrase
Passphrase: [hidden]
Confirm:   [hidden]

3/3  Review
  Path:   .../data.jot
  Tier:   Passphrase only
  Cache:  100s (in-memory)

[OK] Jot initialized
Next: jot add journal  ·  jot list  ·  jot search \"term\"
```

Plain (non-TTY):

```text
jot init
path=/home/user/.local/share/jot/data.jot
status=ok
```

### `jot list --last 7d`

Pretty (TTY):

```text
Jot · list (last 7d)
Path: .../data.jot
Using cached passphrase (expires in 1m40s)

ID        Created                Type     Summary                 Tags
7a2e3c0b  2026-01-12 04:48 UTC    journal  another entry           work
7a94c2b2  2026-01-12 02:19 UTC    journal  hello                   -

2 entries
Hint: jot show 7a2e3c0b  ·  jot search \"hello\"
```

Plain (non-TTY):

```text
7a2e3c0b 2026-01-12T04:48:00Z journal another entry work
7a94c2b2 2026-01-12T02:19:00Z journal hello -
```

### `jot init` (already exists)

Pretty (TTY):

```text
Jot · init
Path: /home/user/.local/share/jot/data.jot

[ERR] Jot already exists

What you can do:
  - Open it:   jot list
  - Verify:    jot check
  - Recreate:  jot init --force   (destroys existing jot)

Hint: Use JOT_PATH=/other/path to create a new jot elsewhere.
```

Plain (non-TTY):

```text
jot init
status=error
error=jot already exists
```

### `jot add journal` (guided wizard)

Pretty (TTY):

```text
Jot · add (journal)

1/4  Template
  > morning-journal (default)
    blank

2/4  Fields
Title: Morning reflection
Tags: gratitude, focus

3/4  Body (editor)
Opening $EDITOR...

4/4  Review
  Type:   journal
  Title:  Morning reflection
  Tags:   gratitude, focus
  Body:   (from editor)

[OK] Added entry
ID: 7a2e3c0b  ·  2026-01-12 04:48 UTC  ·  tags: 2
```

Plain (non-TTY):

```text
status=error
error=interactive input required
hint=use flags or run on a TTY
```

### `jot show <id>`

Pretty (TTY):

```text
Jot · show
ID: 7a2e3c0b
Type: journal
Created: 2026-01-12 04:48 UTC
Tags: work

Another entry body goes here.
```

Plain (non-TTY):

```text
id=7a2e3c0b
type=journal
created=2026-01-12T04:48:00Z
tags=work
body=Another entry body goes here.
```

### `jot search \"hello\"`

Pretty (TTY):

```text
Jot · search
Query: hello

ID        Created                Summary
7a94c2b2  2026-01-12 02:19 UTC    hello

1 result
Hint: jot show 7a94c2b2
```

Plain (non-TTY):

```text
7a94c2b2 2026-01-12T02:19:00Z journal hello
```

### `jot check`

Pretty (TTY):

```text
Jot · check
Path: .../data.jot

Checking...
- Schema integrity:     [OK]
- Foreign keys:         [OK]
- Full-text index:      [OK]

[OK] Jot is healthy
```

Plain (non-TTY):

```text
check=schema ok
check=foreign_keys ok
check=fts ok
status=ok
```

### `jot backup ./backup.jot`

Pretty (TTY):

```text
Jot · backup
Source: .../data.jot
Destination: ./backup.jot

Writing backup... 100%

[OK] Backup written
Path: ./backup.jot  ·  Size: 2.4 MB  ·  Time: 0.8s
```

Plain (non-TTY):

```text
status=ok
path=./backup.jot
size=2.4MB
time=0.8s
```

### `jot export`

Pretty (TTY):

```text
Jot · export
Path: .../data.jot

Exporting... 100%

[OK] Export written
Path: ./jot-export.json  ·  Entries: 214  ·  Time: 0.6s
```

Plain (non-TTY):

```text
status=ok
path=./jot-export.json
entries=214
time=0.6s
```

### `jot lock`

Pretty (TTY):

```text
Jot · lock

[OK] Passphrase cache cleared
Cache: empty
```

Plain (non-TTY):

```text
status=ok
cache=empty
```

### `jot search "nope"`

Pretty (TTY):

```text
Jot · search
Query: nope

[INFO] No results
Tips: try fewer terms, use quotes, or remove filters
```

Plain (non-TTY):

```text
status=ok
results=0
```

## Wizard UX Rules

- Keep wizards to 2-4 steps whenever possible.
- Always show progress (e.g., 1/3, 2/3).
- Provide sensible defaults and highlight them.
- Allow cancel at any step (ESC or Ctrl+C) with a clean message.
- Include a final review screen for destructive or persistent actions.

## Interactive Behavior

Interactive prompts should be used only when stdin/stdout are TTYs.

- If a required argument is missing:
  - TTY: prompt user
  - Non-TTY: error with clear usage

## UX Checklist (Quick Review)

- Output mode routing: json/plain/pretty rules enforced
- TTY detection: prompts and color only on TTY
- Header/primary/footer structure used consistently
- Badges and severity language are consistent
- Plain output is stable and test-friendly
- Errors include a short fix suggestion

## Accessibility and Compatibility

- `--no-color` and `NO_COLOR` always disable ANSI.
- `--ascii` uses ASCII-only symbols.
- Respect `TERM=dumb` by forcing plain.
- Default truncation uses terminal width.

## Testing Strategy

- Snapshot test plain output (stable, deterministic).
- JSON outputs are strict and schema-validated.
- Pretty output tests should assert key strings only (no ANSI).

## Implementation Notes

Suggested module structure:

- `ui/context.rs`: tty detection, width, color/unicode flags
- `ui/theme.rs`: palette + style tokens
- `ui/render.rs`: header, tables, badges, receipts
- `ui/prompt.rs`: init wizard + interactive prompts

Suggested crate integrations (optional, non-TUI):

- Prompts: `dialoguer` (already used)
- Tables: `comfy-table` or `tabled`
- Colors: `anstyle` or `owo-colors`
- Progress/spinners: `indicatif`

## Rollout Plan

1) Implement `ui::context` and output routing rules.
2) Refactor `list`, `show`, `search` to use primitives.
3) Refactor `init` preflight + wizard flow.
4) Add receipts and hints across remaining commands.
5) Add tests for plain/JSON output stability.
