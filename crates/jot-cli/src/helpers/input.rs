//! Input handling helpers for passphrase and entry body reading.

use std::io::{self, IsTerminal, Read};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use dialoguer::Password;
use jot_core::crypto::validate_passphrase;

/// Prompt for passphrase, or read from JOT_PASSPHRASE env var.
pub fn prompt_passphrase(interactive: bool) -> anyhow::Result<String> {
    if let Ok(value) = std::env::var("JOT_PASSPHRASE") {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }
    if !interactive {
        return Err(anyhow::anyhow!(
            "No passphrase provided and no TTY available. Set JOT_PASSPHRASE."
        ));
    }
    Password::new()
        .with_prompt("Passphrase")
        .interact()
        .map_err(|e| anyhow::anyhow!("Failed to read passphrase: {}", e))
}

/// Prompt for passphrase with confirmation (for init), or read from JOT_PASSPHRASE env var.
pub fn prompt_init_passphrase() -> anyhow::Result<String> {
    if let Ok(value) = std::env::var("JOT_PASSPHRASE") {
        if !value.trim().is_empty() {
            validate_passphrase(&value)
                .map_err(|e| anyhow::anyhow!("Passphrase does not meet requirements: {}", e))?;
            return Ok(value);
        }
    }
    loop {
        let passphrase = Password::new()
            .with_prompt("Enter passphrase")
            .with_confirmation("Confirm passphrase", "Passphrases do not match")
            .interact()
            .map_err(|e| anyhow::anyhow!("Failed to read passphrase: {}", e))?;
        if let Err(err) = validate_passphrase(&passphrase) {
            eprintln!("Passphrase does not meet requirements: {}", err);
            continue;
        }
        return Ok(passphrase);
    }
}

/// Read entry body from --body flag, stdin, or $EDITOR.
pub fn read_entry_body(
    no_input: bool,
    body: Option<String>,
    editor_override: Option<&str>,
    initial_body: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(value) = body {
        if value.trim().is_empty() {
            return Err(anyhow::anyhow!("--body cannot be empty"));
        }
        return Ok(value);
    }

    if !io::stdin().is_terminal() {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| anyhow::anyhow!("Failed to read stdin: {}", e))?;
        let trimmed = buffer.trim_end().to_string();
        if trimmed.is_empty() {
            if no_input {
                return Err(anyhow::anyhow!("No input provided on stdin"));
            }
            if editor_override.is_some() {
                return read_body_from_editor(editor_override, initial_body);
            }
            return Err(anyhow::anyhow!("No input provided on stdin"));
        }
        return Ok(trimmed);
    }

    if no_input {
        return Err(anyhow::anyhow!("--no-input requires content from stdin"));
    }

    read_body_from_editor(editor_override, initial_body)
}

/// Open $EDITOR to compose entry body.
fn read_body_from_editor(
    editor_override: Option<&str>,
    initial_body: Option<&str>,
) -> anyhow::Result<String> {
    let editor = editor_override
        .map(|value| value.to_string())
        .or_else(|| std::env::var("EDITOR").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("$EDITOR is not set; use --body or pipe content via stdin")
        })?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("System time error: {}", e))?
        .as_nanos();
    let filename = format!("ledger_entry_{}_{}.md", std::process::id(), nanos);
    let path = std::env::temp_dir().join(filename);

    let initial = initial_body.unwrap_or("");
    std::fs::write(&path, initial)
        .map_err(|e| anyhow::anyhow!("Failed to create temp file: {}", e))?;

    let status = Command::new(editor)
        .arg(&path)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to launch editor: {}", e))?;
    if !status.success() {
        let _ = std::fs::remove_file(&path);
        return Err(anyhow::anyhow!("Editor exited with failure"));
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Failed to read temp file: {}", e))?;
    let _ = std::fs::remove_file(&path);

    let trimmed = contents.trim_end().to_string();
    if trimmed.is_empty() {
        return Err(anyhow::anyhow!("Entry body is empty"));
    }

    Ok(trimmed)
}
