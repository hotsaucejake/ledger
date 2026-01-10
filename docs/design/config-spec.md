# Ledger Config Specification

**Status:** Draft  
**Applies to:** Ledger v0.1+  
**Purpose:** Define the user config file layout and security modes.

## 1. Location (XDG)

On Linux, config lives at:

```
~/.config/ledger/config.toml
```

This file is optional. If missing, Ledger will prompt to initialize a ledger.

## 2. File Format (TOML)

```toml
[ledger]
path = "/home/user/.local/share/ledger/ledger.ledger"

[security]
tier = "passphrase"
passphrase_cache_ttl_seconds = 0

[keychain]
enabled = false

[keyfile]
mode = "none"
path = "/home/user/.config/ledger/ledger.key"
```

## 3. Fields

### 3.1 [ledger]

- `path` (string, required): Default ledger file path

### 3.2 [security]

- `tier` (string, required):
  - `passphrase`
  - `passphrase_keychain`
  - `passphrase_keyfile`
  - `device_keyfile`
- `passphrase_cache_ttl_seconds` (integer, optional; default `0`)
  - `0` means no cache (prompt every time)

### 3.3 [keychain]

- `enabled` (bool, optional; default `false`)
  - If enabled, Ledger will try to store the passphrase in the OS keychain.
  - Only valid when `tier = "passphrase_keychain"`.

### 3.4 [keyfile]

- `mode` (string, optional; default `none`):
  - `none`
  - `encrypted` (key encrypted with passphrase)
  - `plain` (unencrypted device key)
- `path` (string, optional):
  - Used for `passphrase_keyfile` and `device_keyfile`.

## 4. Security Modes

1. **passphrase**  
   - User enters passphrase each time
2. **passphrase_keychain**  
   - Passphrase stored in OS keychain
3. **passphrase_keyfile**  
   - Encrypted keyfile + passphrase
4. **device_keyfile**  
   - Plain keyfile, no passphrase  
   - Must show explicit warning

## 5. Warnings

If `tier = device_keyfile`, Ledger must print:

```
WARNING: You selected device_keyfile. This stores an unencrypted key on disk.
If your device is compromised, your ledger can be decrypted without a passphrase.
```

## 6. Defaults

- `tier = "passphrase"`
- `passphrase_cache_ttl_seconds = 0`
- `keychain.enabled = false`
- `keyfile.mode = "none"`

