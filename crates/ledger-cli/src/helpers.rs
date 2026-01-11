//! Input and parsing helper functions for the CLI.

use std::io::{self, IsTerminal, Read};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use dialoguer::Password;

/// Prompt for passphrase, or read from LEDGER_PASSPHRASE env var.
pub fn prompt_passphrase(interactive: bool) -> anyhow::Result<String> {
    if let Ok(value) = std::env::var("LEDGER_PASSPHRASE") {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }
    if !interactive {
        return Err(anyhow::anyhow!(
            "No passphrase provided and no TTY available. Set LEDGER_PASSPHRASE."
        ));
    }
    Password::new()
        .with_prompt("Passphrase")
        .interact()
        .map_err(|e| anyhow::anyhow!("Failed to read passphrase: {}", e))
}

/// Prompt for passphrase with confirmation (for init), or read from LEDGER_PASSPHRASE env var.
pub fn prompt_init_passphrase() -> anyhow::Result<String> {
    if let Ok(value) = std::env::var("LEDGER_PASSPHRASE") {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }
    Password::new()
        .with_prompt("Enter passphrase")
        .with_confirmation("Confirm passphrase", "Passphrases do not match")
        .interact()
        .map_err(|e| anyhow::anyhow!("Failed to read passphrase: {}", e))
}

/// Parse a datetime string (ISO-8601 or YYYY-MM-DD).
pub fn parse_datetime(value: &str) -> anyhow::Result<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }

    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let naive = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid date value: {}", value))?;
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }

    Err(anyhow::anyhow!(
        "Invalid date/time (expected ISO-8601 or YYYY-MM-DD): {}",
        value
    ))
}

/// Parse a duration string (e.g., "7d", "24h").
pub fn parse_duration(value: &str) -> anyhow::Result<Duration> {
    if value.len() < 2 {
        return Err(anyhow::anyhow!(
            "Invalid duration: {} (expected <number><unit>)",
            value
        ));
    }

    let (num_str, unit) = value.split_at(value.len() - 1);
    let amount: i64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid duration number: {}", value))?;
    if amount <= 0 {
        return Err(anyhow::anyhow!("Duration must be positive: {}", value));
    }

    match unit {
        "d" => Ok(Duration::days(amount)),
        "h" => Ok(Duration::hours(amount)),
        "m" => Ok(Duration::minutes(amount)),
        "s" => Ok(Duration::seconds(amount)),
        _ => Err(anyhow::anyhow!(
            "Invalid duration unit: {} (use d/h/m/s)",
            unit
        )),
    }
}

/// Output format for list/search commands.
#[derive(Clone, Copy)]
pub enum OutputFormat {
    Table,
    Plain,
}

/// Parse output format string.
pub fn parse_output_format(value: Option<&str>) -> anyhow::Result<Option<OutputFormat>> {
    match value {
        None => Ok(None),
        Some("table") => Ok(Some(OutputFormat::Table)),
        Some("plain") => Ok(Some(OutputFormat::Plain)),
        Some(other) => Err(anyhow::anyhow!(
            "Unsupported format: {} (use table or plain)",
            other
        )),
    }
}

/// Ensure entry type is "journal" (only supported type in Phase 0.1).
pub fn ensure_journal_type_name(entry_type: &str) -> anyhow::Result<()> {
    if entry_type != "journal" {
        return Err(anyhow::anyhow!(
            "Entry type \"{}\" is not supported in the CLI yet. Only \"journal\" is available.",
            entry_type
        ));
    }
    Ok(())
}

/// Read entry body from --body flag, stdin, or $EDITOR.
pub fn read_entry_body(
    no_input: bool,
    body: Option<String>,
    editor_override: Option<&str>,
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
                return read_body_from_editor(editor_override);
            }
            return Err(anyhow::anyhow!("No input provided on stdin"));
        }
        return Ok(trimmed);
    }

    if no_input {
        return Err(anyhow::anyhow!("--no-input requires content from stdin"));
    }

    read_body_from_editor(editor_override)
}

/// Open $EDITOR to compose entry body.
fn read_body_from_editor(editor_override: Option<&str>) -> anyhow::Result<String> {
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

    std::fs::write(&path, "").map_err(|e| anyhow::anyhow!("Failed to create temp file: {}", e))?;

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
