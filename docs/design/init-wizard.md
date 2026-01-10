# Init Wizard UX

**Status:** Draft  
**Applies to:** Ledger v0.1+  
**Purpose:** Define the first-run UX and initialization flow.

---

## 1. Entry Points

### A) User runs `ledger init`

Run full wizard.

### B) User runs any command without config

Print friendly guidance:

```
No ledger found at ~/.config/ledger/config.toml

Run:
  ledger init

Or specify a ledger path:
  LEDGER_PATH=/path/to/my.ledger ledger init
```

No auto-init.

---

## 2. Default Wizard Flow (fast path)

```
Welcome to Ledger.

Ledger file location:
  [~/.local/share/ledger/ledger.ledger]

Create a passphrase (min 12 chars):
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

Triggered by `ledger init --advanced`.

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
Ledger created at ~/.local/share/ledger/ledger.ledger
Config written to ~/.config/ledger/config.toml
```

If “create first entry” was selected:

```
Create your first journal entry now? [yes]
<editor opens or stdin prompt>
Entry saved.
```

---

## 5. Rules

- Defaults are always shown in brackets.
- Prompts are skipped if flags are provided.
- `--no-input` is respected; missing required values should error.
- No passphrase is printed or logged.

