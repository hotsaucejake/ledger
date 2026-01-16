//! Parsing helpers for datetime, duration, and output format.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use ledger_core::StorageEngine;

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

/// Look up an entry type by name, returning an error if not found.
///
pub fn require_entry_type(
    storage: &ledger_core::storage::AgeSqliteStorage,
    entry_type_name: &str,
) -> anyhow::Result<ledger_core::storage::EntryType> {
    storage.get_entry_type(entry_type_name)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Entry type \"{}\" not found.\nHint: Run `ledger add {}` to create it.",
            entry_type_name,
            entry_type_name
        )
    })
}
