# CLI UI Polish Items

This document tracks polish items needed to fully align the CLI UI implementation with the spec in `cli-ux-spec.md`. The core infrastructure is complete; these are refinements.

## Priority Levels

- **P0**: Bugs or broken functionality
- **P1**: Visible inconsistencies with spec mockups
- **P2**: Nice-to-have improvements
- **P3**: Future enhancements

---

## Table Rendering Issues

### P1: Table header alignment with dim styling

**Problem**: When headers are styled with `styled(..., dim())`, the ANSI codes may affect column width calculations in comfy-table, causing misalignment.

**File**: `src/ui/render.rs:173-191` (`simple_table`)

**Fix**: Either:
1. Apply styling after comfy-table renders (post-process the header line)
2. Use comfy-table's built-in styling instead of manual ANSI codes
3. Calculate widths without ANSI codes, then apply styling

### P1: Missing padding between columns

**Problem**: Columns in `simple_table` may run together without sufficient spacing.

**File**: `src/ui/render.rs:173-191`

**Fix**: Configure `comfy_table::Column::set_padding()` to add consistent spacing.

### P2: Column width constraints not working

**Problem**: `Column::with_width()` sets width but the constraint logic may not work correctly.

**File**: `src/ui/render.rs:153-159`

**Fix**: Review comfy-table constraint API usage; may need `ColumnConstraint::Absolute` or `LowerBoundary`.

---

## Header Enhancements

### P1: Missing filter context in headers

**Spec shows**:
```
Ledger · list (last 7d)
```

**Current**:
```
Ledger · list
```

**Files**:
- `src/commands/entries/list.rs`
- `src/commands/entries/search.rs`

**Fix**: Pass filter description to `header()`:
```rust
let scope = args.last.as_ref().map(|l| format!("last {}", l));
print(&ui_ctx, &header(&ui_ctx, "list", scope.as_deref()));
```

### P1: Missing path in headers

**Spec shows**:
```
Ledger · list (last 7d)
Path: .../ledger.ledger
```

**Current**: No path shown

**Fix**: Add path parameter to header calls. Consider truncating long paths with `...`.

### P2: Missing cache state line

**Spec shows**:
```
Using cached passphrase (expires in 1m40s)
```

**Current**: Shows "Using cached passphrase" on cache hit (implemented in passphrase.rs:38-40)

**Partial Fix** (implemented):
- Displays "Using cached passphrase" message when cache is used
- Message goes to stderr so it doesn't interfere with command output

**Future Enhancement** (requires protocol extension):
To show TTL remaining would require:
1. Add a `TTL <ledger-path-hash>` command to cache daemon protocol
2. Add `cache_ttl_remaining()` client function
3. Query TTL at storage open time and include in message

The cache daemon currently only supports PING, GET, STORE, CLEAR commands.

---

## List/Search Output

### P1: Missing columns in entry list

**Spec shows**: ID, Created, Type, Summary, Tags

**Current**: ID, Created, Summary

**Files**:
- `src/commands/entries/list.rs`
- `src/commands/entries/search.rs`

**Fix**: Add Type and Tags columns:
```rust
let columns = [
    Column::new("ID"),
    Column::new("Created"),
    Column::new("Type"),
    Column::new("Summary"),
    Column::new("Tags"),
];
```

### P1: Hints should be actionable with IDs

**Spec shows**:
```
Hint: ledger show 7a2e3c0b  ·  ledger search "hello"
```

**Current**:
```
Hint: Showing 5 entries. Use --limit to see more.
```

**Fix**: Include first entry ID in hint for discoverability:
```rust
let first_id = entries.first().map(|e| &e.id.to_string()[..8]);
let hint_text = format!(
    "ledger show {}  ·  ledger search \"term\"",
    first_id.unwrap_or("<id>")
);
```

### P2: Search result highlighting

**Spec says**: "Highlight matches if possible, but keep subtle"

**Current**: No highlighting

**Fix**: Wrap matched terms in styled markers (e.g., bold or underline).

---

## Wizard Integration

### P1: Init command doesn't use wizard framework

**Problem**: `src/commands/init.rs` uses direct dialoguer calls instead of `ui/prompt.rs` wizard.

**Spec shows**:
```
1/3  Choose location
     Path: ...

2/3  Create passphrase
     Passphrase: [hidden]

3/3  Review
     Path: ...
     Tier: ...
```

**Current**: Prompts work but without step indicators or review screen.

**Fix**: Replace dialoguer calls with `init_wizard()` from `ui/prompt.rs`, or integrate wizard step display into existing flow.

### P1: Add command doesn't use wizard framework

**Problem**: `src/commands/entries/add.rs` doesn't show step indicators when prompting.

**Fix**: Use `add_wizard()` when interactive and no `--body` flag provided, or integrate step display into `prompt_for_fields()`.

---

## Commands Not Using UI Primitives

### P1: `lock` command uses raw println

**File**: `src/commands/maintenance/lock.rs`

**Current**:
```rust
println!("Passphrase cache cleared.");
```

**Spec shows**:
```
Ledger · lock

[OK] Passphrase cache cleared
Cache: empty
```

**Fix**: Use UI primitives:
```rust
let ui_ctx = ctx.ui_context(false, None);
match ui_ctx.mode {
    OutputMode::Pretty => {
        print(&ui_ctx, &header(&ui_ctx, "lock", None));
        blank_line(&ui_ctx);
        print(&ui_ctx, &badge(&ui_ctx, Badge::Ok, "Passphrase cache cleared"));
        print(&ui_ctx, &kv(&ui_ctx, "Cache", "empty"));
    }
    OutputMode::Plain | OutputMode::Json => {
        println!("status=ok");
        println!("cache=empty");
    }
}
```

### P2: `completions` command (low priority)

**File**: `src/commands/misc.rs`

**Current**: Outputs raw shell script (correct behavior)

**Spec says**: "Only print help/pretty info when explicitly requested and on TTY"

**Status**: Already correct, no change needed.

---

## Progress Indicators

### P2: Backup command could show progress

**Spec shows**:
```
Writing backup... 65%
```

**Current**: Just shows success badge

**File**: `src/commands/maintenance/backup.rs`

**Fix**: For large files, wrap copy with `ProgressBar`. For small files, current behavior is fine.

### P2: Export command could show progress

**Spec shows**:
```
Exporting... 100%

[OK] Export written
Path: ./ledger-export.json  ·  Entries: 214  ·  Time: 0.6s
```

**Current**: Just outputs data

**Fix**: Add progress bar for entry count, show receipt with stats.

---

## Receipt Enhancements

### P2: Receipts should include more context

**Spec shows**:
```
[OK] Added entry
ID: 7a2e3c0b  ·  2026-01-12 04:48 UTC  ·  tags: 2
```

**Current**: Most commands show minimal receipt

**Fix**: Enhance receipt output to include timestamp, tag count, etc.

### P2: Next-step hints after actions

**Spec shows**:
```
Next: ledger add journal  ·  ledger list  ·  ledger search "term"
```

**Current**: Most commands don't show next steps

**Fix**: Add `Wizard::print_hints()` calls or new `next_steps()` primitive.

---

## Error Output

### P1: Error hints should be consistent

**Some commands show**:
```
Hint: ...
```

**Some commands show**:
```
error=...
```

**Fix**: Standardize error output:
- Pretty mode: Badge + hint on separate line
- Plain mode: `error=message` then `hint=suggestion`

### P2: Error messages could be more helpful

**Current**: Some errors just say what failed

**Fix**: Include actionable next steps in error messages consistently.

---

## Code Quality

### P2: Duplicated `entry_summary()` function

**Files**:
- `src/commands/entries/list.rs:15-22`
- `src/commands/entries/search.rs:13-20`

**Fix**: Move to `ui/format.rs` or `output/text.rs` and reuse.

### P2: Inconsistent quiet handling

**Some commands check**: `if !ctx.quiet() { ... }`

**Some commands don't check quiet flag at all**

**Fix**: Audit all commands for consistent quiet flag handling.

### P3: Consider removing old output module

**File**: `src/output/` directory

**Current**: Only `entry_type_name_map()` is used from this module.

**Fix**: Move remaining function to `ui/` and delete `output/` module.

---

## Testing

### P2: No visual regression tests

**Current**: Tests check for key strings but not formatting

**Fix**: Add snapshot tests for pretty mode output (without ANSI codes).

### P2: No tests for table alignment

**Fix**: Add tests that verify column widths and alignment.

---

## Summary Checklist

### P1 (Should fix) - ✅ COMPLETED
- [x] Table header alignment with dim styling (use comfy-table's built-in Cell styling)
- [x] Missing padding between columns (set_padding((0, 2)))
- [x] Add filter context to headers (header_with_context function)
- [x] Add path to headers (resolve_ledger_path in list/search)
- [x] Add Type and Tags columns to list/search
- [x] Actionable hints with entry IDs
- [x] Wire init wizard or add step indicators (print_step helper)
- [x] Wire add wizard or add step indicators (print_step helper)
- [x] Migrate `lock` command to UI primitives
- [x] Consistent error hint formatting (print_error in main.rs)

### P2 (Nice to have)
- [x] Column width constraints
- [x] Cache state line in headers (partial - shows "Using cached passphrase", TTL requires protocol extension)
- [x] Search result highlighting
- [x] Progress bar for backup
- [x] Progress bar for export
- [x] Enhanced receipts with context
- [x] Next-step hints after actions
- [x] Better error messages
- [x] Deduplicate `entry_summary()`
- [x] Consistent quiet handling
- [x] Remove old output module
- [x] Visual regression tests
- [x] Table alignment tests

### P3 (Future)
- [ ] (none currently identified)
