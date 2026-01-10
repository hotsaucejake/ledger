# Session Cache Design

**Status:** Draft
**Applies to:** Ledger v0.1+ (M2)
**Purpose:** Define the passphrase caching mechanism for improved UX.

## 1. Problem Statement

Without caching, users must enter their passphrase for every Ledger operation. This creates friction for common workflows like:

```bash
ledger add journal --body "Quick note"
ledger list --last 1d
ledger show @last
```

Each command requires passphrase entry, which is tedious for rapid, iterative use.

## 2. Design Goals

1. **Opt-in**: Caching is disabled by default (`passphrase_cache_ttl_seconds = 0`)
2. **Secure**: Passphrase never written to disk
3. **Ephemeral**: Cache expires automatically after TTL
4. **User-controlled**: `ledger lock` clears cache immediately
5. **Simple**: No external dependencies (no gpg-agent, no systemd)

## 3. Architecture

### 3.1 Cache Daemon

When caching is enabled, Ledger spawns a lightweight background process:

```
┌──────────────┐      Unix Socket      ┌──────────────┐
│  ledger CLI  │ ◄──────────────────► │ ledger-cache │
└──────────────┘                       └──────────────┘
                                              │
                                              ▼
                                       In-memory store
                                       (zeroized on exit)
```

### 3.2 Socket Location

The cache daemon listens on a Unix domain socket:

```
Linux:  $XDG_RUNTIME_DIR/ledger/cache.sock
        (fallback: /tmp/ledger-$UID/cache.sock)

macOS:  $TMPDIR/ledger-cache.sock
```

Socket permissions: `0600` (owner read/write only)

### 3.3 Protocol

Simple text-based protocol over Unix socket:

**Store passphrase:**
```
STORE <ledger-path-hash> <passphrase-base64>
OK
```

**Retrieve passphrase:**
```
GET <ledger-path-hash>
PASSPHRASE <passphrase-base64>
```
or
```
GET <ledger-path-hash>
NOT_FOUND
```

**Clear cache:**
```
CLEAR
OK
```

**Ping (check if daemon alive):**
```
PING
PONG
```

### 3.4 Ledger Path Hash

To support multiple ledgers, the cache keys by a hash of the ledger path:

```rust
fn ledger_hash(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let hash = blake3::hash(canonical.to_string_lossy().as_bytes());
    hash.to_hex()[..16].to_string()
}
```

This prevents accidental cross-ledger passphrase use.

## 4. Daemon Lifecycle

### 4.1 Startup

When `ledger` needs the passphrase and `passphrase_cache_ttl_seconds > 0`:

1. Check if daemon is running (try `PING`)
2. If not running, spawn daemon in background
3. Daemon reads TTL from config
4. Daemon starts expiry timer

### 4.2 Expiry

The daemon tracks when each passphrase was stored:

- On each `GET`, check if entry has expired
- Expired entries are zeroized and removed
- If all entries expired and no activity for 60s, daemon exits

### 4.3 Shutdown

The daemon exits when:

- All entries expired + 60s idle
- Receives `SIGTERM` or `SIGINT`
- User runs `ledger lock`

On exit:
1. Zeroize all stored passphrases
2. Remove socket file
3. Exit cleanly

## 5. Security Considerations

### 5.1 Threat Model

**We defend against:**
- Passphrase in shell history (not applicable - we prompt)
- Passphrase in environment variables (not stored there)
- Passphrase on disk (never written)

**We do NOT defend against:**
- Memory inspection by root/admin
- Debugger attachment to daemon
- Physical access to running machine

This is acceptable because:
- If attacker has root, they can keylog anyway
- The cache is opt-in; security-conscious users can disable it

### 5.2 Mitigations

1. **Socket permissions**: `0600`, only owner can connect
2. **Memory zeroization**: Use `zeroize` crate on all passphrase data
3. **No swap**: Use `mlock()` to prevent passphrase pages from being swapped (best-effort)
4. **Short-lived**: Default TTL should be reasonable (e.g., 300 seconds = 5 minutes)

### 5.3 Explicit Warning

When user enables caching, show:

```
Note: Passphrase caching keeps your passphrase in memory for 5 minutes.
This improves convenience but means your ledger can be accessed without
re-entering your passphrase during that time.
```

## 6. CLI Integration

### 6.1 `ledger lock`

Immediately clears the cache:

```bash
$ ledger lock
Passphrase cache cleared.
```

### 6.2 Passphrase Flow with Cache

```
ledger add journal
    │
    ▼
Is cache enabled? (TTL > 0)
    │
    ├─► No: Prompt for passphrase
    │
    └─► Yes: Is daemon running?
            │
            ├─► No: Prompt, start daemon, store passphrase
            │
            └─► Yes: Try GET from daemon
                    │
                    ├─► Found: Use cached passphrase
                    │
                    └─► Not found: Prompt, store in daemon
```

### 6.3 Failed Passphrase

If the cached passphrase fails (e.g., ledger file changed):

1. Clear that entry from cache
2. Prompt user for passphrase
3. Store new passphrase if successful

## 7. Configuration

In `~/.config/ledger/config.toml`:

```toml
[security]
passphrase_cache_ttl_seconds = 300  # 5 minutes, 0 = disabled
```

Recommended values:
- `0` — Disabled (default, most secure)
- `300` — 5 minutes (convenient for active use)
- `3600` — 1 hour (for trusted environments)

## 8. Implementation Notes

### 8.1 Dependencies

- `tokio` or `async-std` for async socket handling (daemon only)
- `zeroize` for secure memory clearing
- `blake3` for path hashing (already used elsewhere)
- `libc` for `mlock()` (optional, best-effort)

### 8.2 Daemon Binary

The daemon can be:
- A separate binary (`ledger-cache`)
- Or the same binary with a subcommand (`ledger cache-daemon`)

Recommend: Same binary with hidden subcommand for simpler distribution.

```bash
# Spawned internally, not user-facing
ledger --internal-cache-daemon --ttl 300
```

### 8.3 Graceful Degradation

If daemon fails to start or socket unavailable:
- Log warning (debug level)
- Fall back to prompting every time
- Never block or error on cache failure

## 9. Alternatives Considered

### 9.1 Environment Variable

Store passphrase in `LEDGER_PASSPHRASE` after first entry.

**Rejected**: Visible in `/proc/$pid/environ`, shell history if exported.

### 9.2 gpg-agent Integration

Use gpg-agent's passphrase caching.

**Rejected**: Adds external dependency, complex setup, not all users have GPG.

### 9.3 Encrypted Temp File

Write passphrase to encrypted temp file.

**Rejected**: Still creates disk I/O, complexity of managing encryption key.

### 9.4 Kernel Keyring (Linux)

Use Linux kernel keyring (`keyctl`).

**Rejected**: Linux-only, requires specific kernel config, adds complexity.

## 10. Future Enhancements

- **Activity-based TTL**: Reset timer on each use
- **Per-ledger TTL**: Different timeouts for different ledgers
- **Systemd socket activation**: Let systemd manage the daemon lifecycle
- **macOS Keychain integration**: Store in Keychain with TTL (separate from Tier 2)

These are out of scope for M2 but noted for future consideration.
