# Init Wizard UX

**Status:** Draft  
**Applies to:** Ledger v0.1+  
**Purpose:** Define the first-run UX and initialization flow.

---

## 1. Entry Points

### A) User runs `jot init`

Run full wizard.

### B) User runs any command without config

Print friendly guidance:

```
No ledger found at ~/.config/ledger/config.toml

Run:
  jot init

Or specify a jot path:
  JOT_PATH=/path/to/my.jot jot init
```

No auto-init.

---

## 2. Default Wizard Flow (fast path)

```
Welcome to Ledger.

Ledger file location:
  [~/.local/share/ledger/ledger.jot]

Create a passphrase (min 8 chars):
  Passphrase:
  Confirm:

Security level:
  [1] Passphrase only (recommended)
  [2] Passphrase + OS keychain
  [3] Passphrase + encrypted keyfile
  [4] Device keyfile only (reduced security)
Select [1]:

Create your first journal entry now? [yes]:
```

If user selects security tier 4, show warning:

```
WARNING: You selected device_keyfile. This stores an unencrypted key on disk.
If your device is compromised, your ledger can be decrypted without a passphrase.
Continue? [no]:
```

---

## 3. Advanced Wizard Flow

Triggered by `jot init --advanced`.

Additional prompts:

```
Timezone [auto-detect]:
Default editor [$EDITOR or nano]:
Passphrase cache (seconds) [0]:
Keyfile path (if applicable) [~/.config/ledger/ledger.key]:
Ledger config path [~/.config/ledger/config.toml]:
```

---

## 4. Outputs

On success:

```
Ledger created at ~/.local/share/ledger/ledger.jot
Config written to ~/.config/ledger/config.toml
```

If “create first entry” was selected:

```
Create your first journal entry now? [yes]
<editor opens or stdin prompt>
Entry saved.
```

---

## 5. Passphrase Retry Behavior

When opening an existing ledger (not during init), passphrase entry follows these rules:

### 5.1 Retry Flow

```
Enter passphrase:
[incorrect]

Incorrect passphrase. 2 attempts remaining.
Enter passphrase:
[incorrect]

Incorrect passphrase. 1 attempt remaining.
Enter passphrase:
[incorrect]

Error: Too many failed passphrase attempts.
Hint: If you forgot your passphrase, the jot cannot be recovered.
      Backups use the same passphrase.

Exit code: 5
```

### 5.2 Rules

- Maximum 3 attempts per invocation
- Show remaining attempts after each failure
- After 3 failures, exit with code 5 (encryption/auth error per RFC-003 §14.2)
- No lockout period (user can immediately retry by running command again)
- `--no-input` mode: single attempt only, no retry loop

### 5.3 Scripting Considerations

For scripts using `JOT_PASSPHRASE` environment variable:
- Single attempt (no retry loop)
- Exit code 5 on failure
- Clear error message to stderr

---

## 6. Rules

- Defaults are always shown in brackets.
- Prompts are skipped if flags are provided.
- `--no-input` is respected; missing required values should error.
- No passphrase is printed or logged.
- `--quiet` suppresses informational output (errors still printed to stderr).

