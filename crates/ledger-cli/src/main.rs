//! Ledger CLI - A secure, encrypted, CLI-first personal journal and logbook
//!
//! This is the command-line interface for Ledger. It provides a user-friendly
//! interface to the core library functionality.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::collections::HashMap;
use std::io::{self, IsTerminal, Read};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use dialoguer::Password;
use ledger_core::storage::{AgeSqliteStorage, EntryFilter, NewEntry, NewEntryType, StorageEngine};
use ledger_core::VERSION;
use uuid::Uuid;

/// Ledger - A secure, encrypted, CLI-first personal journal and logbook
#[derive(Parser)]
#[command(name = "ledger")]
#[command(author, version = VERSION, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Path to the ledger file
    #[arg(short, long, global = true, env = "LEDGER_PATH")]
    ledger: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Quiet mode (minimal output)
    #[arg(short, long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new encrypted ledger
    Init {
        /// Path where the ledger will be created
        #[arg(value_name = "PATH")]
        path: Option<String>,
    },

    /// Add a new entry to the ledger
    Add {
        /// Entry type to add
        #[arg(value_name = "TYPE")]
        entry_type: String,

        /// Entry body (overrides stdin/editor)
        #[arg(long)]
        body: Option<String>,

        /// Add tags to the entry
        #[arg(short, long, value_name = "TAG")]
        tag: Vec<String>,

        /// Set custom date/time (ISO-8601)
        #[arg(long)]
        date: Option<String>,

        /// Disable interactive prompts
        #[arg(long)]
        no_input: bool,
    },

    /// List entries
    List {
        /// Filter by entry type
        #[arg(value_name = "TYPE")]
        entry_type: Option<String>,

        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,

        /// Time window (e.g., "7d", "30d")
        #[arg(long)]
        last: Option<String>,

        /// Start date (ISO-8601)
        #[arg(long)]
        since: Option<String>,

        /// End date (ISO-8601)
        #[arg(long)]
        until: Option<String>,

        /// Limit number of results
        #[arg(long)]
        limit: Option<usize>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Output format (table, plain)
        #[arg(long, value_name = "FORMAT")]
        format: Option<String>,
    },

    /// Search entries using full-text search
    Search {
        /// Search query
        #[arg(value_name = "QUERY")]
        query: String,

        /// Filter by entry type
        #[arg(long)]
        r#type: Option<String>,

        /// Time window (e.g., "7d", "30d")
        #[arg(long)]
        last: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Limit number of results
        #[arg(long)]
        limit: Option<usize>,

        /// Output format (table, plain)
        #[arg(long, value_name = "FORMAT")]
        format: Option<String>,
    },

    /// Show a specific entry by ID
    Show {
        /// Entry ID (full UUID or prefix)
        #[arg(value_name = "ID")]
        id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Export entries
    Export {
        /// Filter by entry type
        #[arg(value_name = "TYPE")]
        entry_type: Option<String>,

        /// Output format
        #[arg(long, default_value = "json")]
        format: String,

        /// Start date (ISO-8601)
        #[arg(long)]
        since: Option<String>,
    },

    /// Check ledger integrity
    Check,

    /// Backup the ledger
    Backup {
        /// Destination path
        #[arg(value_name = "DEST")]
        destination: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_name = "SHELL")]
        shell: Shell,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // For Milestone 0, we just show that commands parse correctly
    match cli.command {
        Some(Commands::Init { path }) => {
            let target = path.or(cli.ledger).ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;

            let passphrase = prompt_init_passphrase()?;

            let device_id = AgeSqliteStorage::create(std::path::Path::new(&target), &passphrase)?;
            let mut storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;
            ensure_journal_entry_type(&mut storage, device_id)?;
            storage.close(&passphrase)?;

            if !cli.quiet {
                println!("Initialized new ledger at {}", target);
            }
        }
        Some(Commands::Add {
            entry_type,
            tag,
            date,
            no_input,
            body,
        }) => {
            ensure_journal_type_name(&entry_type)?;

            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;

            let mut storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;
            let entry_type_record = storage
                .get_entry_type(&entry_type)?
                .ok_or_else(|| anyhow::anyhow!("Entry type \"{}\" not found", entry_type))?;

            let body = read_entry_body(no_input, body)?;
            let data = serde_json::json!({ "body": body });
            let metadata = storage.metadata()?;
            let mut new_entry = NewEntry::new(
                entry_type_record.id,
                entry_type_record.version,
                data,
                metadata.device_id,
            )
            .with_tags(tag);
            if let Some(value) = date {
                let parsed = parse_datetime(&value)?;
                new_entry = new_entry.with_created_at(parsed);
            }

            let entry_id = storage.insert_entry(&new_entry)?;
            storage.close(&passphrase)?;

            if !cli.quiet {
                println!("Added entry {}", entry_id);
            }
        }
        Some(Commands::List {
            entry_type,
            tag,
            last,
            since,
            until,
            limit,
            json,
            format,
        }) => {
            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;
            let storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;

            let mut filter = EntryFilter::new();
            if let Some(t) = entry_type {
                ensure_journal_type_name(&t)?;
                let entry_type_record = storage
                    .get_entry_type(&t)?
                    .ok_or_else(|| anyhow::anyhow!("Entry type \"{}\" not found", t))?;
                filter = filter.entry_type(entry_type_record.id);
            }
            if let Some(t) = tag {
                filter = filter.tag(t);
            }
            if let Some(ref l) = last {
                let window = parse_duration(l)?;
                let since_time = Utc::now() - window;
                filter = filter.since(since_time);
            }
            if let Some(s) = since {
                let parsed = chrono::DateTime::parse_from_rfc3339(&s)
                    .map_err(|e| anyhow::anyhow!("Invalid since timestamp: {}", e))?;
                filter = filter.since(parsed.with_timezone(&chrono::Utc));
            }
            if let Some(u) = until {
                let parsed = chrono::DateTime::parse_from_rfc3339(&u)
                    .map_err(|e| anyhow::anyhow!("Invalid until timestamp: {}", e))?;
                filter = filter.until(parsed.with_timezone(&chrono::Utc));
            }
            if let Some(lim) = limit {
                filter = filter.limit(lim);
            }

            let entries = storage.list_entries(&filter)?;
            let format = parse_output_format(format.as_deref())?;
            if json {
                if format.is_some() {
                    return Err(anyhow::anyhow!("--format cannot be used with --json"));
                }
                let name_map = entry_type_name_map(&storage)?;
                let output = serde_json::to_string_pretty(&entries_json(&entries, &name_map))?;
                println!("{}", output);
            } else {
                match format.unwrap_or(OutputFormat::Table) {
                    OutputFormat::Table => {
                        if !cli.quiet {
                            println!("ID | CREATED_AT | SUMMARY");
                        }
                        for entry in entries {
                            let summary = entry
                                .data
                                .get("body")
                                .and_then(|v| v.as_str())
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| entry.data.to_string());
                            println!("{} | {} | {}", entry.id, entry.created_at, summary);
                        }
                    }
                    OutputFormat::Plain => {
                        for entry in entries {
                            let summary = entry
                                .data
                                .get("body")
                                .and_then(|v| v.as_str())
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| entry.data.to_string());
                            println!("{} {} {}", entry.id, entry.created_at, summary);
                        }
                    }
                }
            }
        }
        Some(Commands::Search {
            query,
            r#type,
            last,
            json,
            limit,
            format,
        }) => {
            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;
            let storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;

            let mut entries = storage.search_entries(&query)?;
            if let Some(t) = r#type {
                ensure_journal_type_name(&t)?;
                let entry_type_record = storage
                    .get_entry_type(&t)?
                    .ok_or_else(|| anyhow::anyhow!("Entry type \"{}\" not found", t))?;
                entries.retain(|entry| entry.entry_type_id == entry_type_record.id);
            }
            if let Some(ref l) = last {
                let window = parse_duration(l)?;
                let since = Utc::now() - window;
                entries.retain(|entry| entry.created_at >= since);
            }
            if let Some(lim) = limit {
                entries.truncate(lim);
            }

            let format = parse_output_format(format.as_deref())?;
            if json {
                if format.is_some() {
                    return Err(anyhow::anyhow!("--format cannot be used with --json"));
                }
                let name_map = entry_type_name_map(&storage)?;
                let output = serde_json::to_string_pretty(&entries_json(&entries, &name_map))?;
                println!("{}", output);
            } else {
                match format.unwrap_or(OutputFormat::Table) {
                    OutputFormat::Table => {
                        if !cli.quiet {
                            println!("ID | CREATED_AT | SUMMARY");
                        }
                        for entry in entries {
                            let summary = entry
                                .data
                                .get("body")
                                .and_then(|v| v.as_str())
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| entry.data.to_string());
                            println!("{} | {} | {}", entry.id, entry.created_at, summary);
                        }
                    }
                    OutputFormat::Plain => {
                        for entry in entries {
                            let summary = entry
                                .data
                                .get("body")
                                .and_then(|v| v.as_str())
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| entry.data.to_string());
                            println!("{} {} {}", entry.id, entry.created_at, summary);
                        }
                    }
                }
            }
        }
        Some(Commands::Show { id, json }) => {
            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;
            let storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;

            let parsed =
                Uuid::parse_str(&id).map_err(|e| anyhow::anyhow!("Invalid entry ID: {}", e))?;
            let entry = storage
                .get_entry(&parsed)?
                .ok_or_else(|| anyhow::anyhow!("Entry not found"))?;
            if json {
                let name_map = entry_type_name_map(&storage)?;
                let output = serde_json::to_string_pretty(&entry_json(&entry, &name_map))?;
                println!("{}", output);
            } else {
                print_entry(&storage, &entry, cli.quiet)?;
            }
        }
        Some(Commands::Export {
            entry_type,
            format,
            since,
        }) => {
            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;
            let storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;

            let mut filter = EntryFilter::new();
            if let Some(t) = entry_type {
                ensure_journal_type_name(&t)?;
                let entry_type_record = storage
                    .get_entry_type(&t)?
                    .ok_or_else(|| anyhow::anyhow!("Entry type \"{}\" not found", t))?;
                filter = filter.entry_type(entry_type_record.id);
            }
            if let Some(s) = since {
                let parsed = parse_datetime(&s)?;
                filter = filter.since(parsed);
            }

            let entries = storage.list_entries(&filter)?;
            let name_map = entry_type_name_map(&storage)?;
            match format.as_str() {
                "json" => {
                    let output = serde_json::to_string_pretty(&entries_json(&entries, &name_map))?;
                    println!("{}", output);
                }
                "jsonl" => {
                    for value in entries_json(&entries, &name_map) {
                        println!("{}", serde_json::to_string(&value)?);
                    }
                }
                other => {
                    return Err(anyhow::anyhow!(
                        "Unsupported export format: {} (use json or jsonl)",
                        other
                    ));
                }
            }
        }
        Some(Commands::Check) => {
            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;
            let storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;
            match storage.check_integrity() {
                Ok(()) => {
                    if !cli.quiet {
                        println!("Integrity check: OK");
                        println!("- foreign keys: OK");
                        println!("- entries FTS: OK");
                        println!("- entry type versions: OK");
                        println!("- metadata keys: OK");
                    }
                }
                Err(err) => {
                    eprintln!("Integrity check: FAILED");
                    eprintln!("- error: {}", err);
                    return Err(anyhow::anyhow!("Integrity check failed"));
                }
            }
        }
        Some(Commands::Backup { destination }) => {
            let source = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let count = std::fs::copy(&source, &destination).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to copy ledger from {} to {}: {}",
                    source,
                    destination,
                    e
                )
            })?;
            if count == 0 {
                return Err(anyhow::anyhow!("Backup failed: zero bytes written"));
            }
            if !cli.quiet {
                println!("Backed up ledger to {}", destination);
            }
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "ledger", &mut std::io::stdout());
        }
        None => {
            println!("Ledger v{}", VERSION);
            println!("\nRun `ledger --help` for usage information.");
            println!("\n[Milestone 0] Core functionality not yet implemented.");
        }
    }

    Ok(())
}

fn prompt_passphrase() -> anyhow::Result<String> {
    if let Ok(value) = std::env::var("LEDGER_PASSPHRASE") {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }
    Password::new()
        .with_prompt("Passphrase")
        .interact()
        .map_err(|e| anyhow::anyhow!("Failed to read passphrase: {}", e))
}

fn prompt_init_passphrase() -> anyhow::Result<String> {
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

fn parse_datetime(value: &str) -> anyhow::Result<DateTime<Utc>> {
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

fn parse_duration(value: &str) -> anyhow::Result<Duration> {
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

#[derive(Clone, Copy)]
enum OutputFormat {
    Table,
    Plain,
}

fn parse_output_format(value: Option<&str>) -> anyhow::Result<Option<OutputFormat>> {
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

fn ensure_journal_type_name(entry_type: &str) -> anyhow::Result<()> {
    if entry_type != "journal" {
        return Err(anyhow::anyhow!(
            "Entry type \"{}\" is not supported in the CLI yet. Only \"journal\" is available.",
            entry_type
        ));
    }
    Ok(())
}

fn ensure_journal_entry_type(
    storage: &mut AgeSqliteStorage,
    device_id: Uuid,
) -> anyhow::Result<()> {
    if storage.get_entry_type("journal")?.is_some() {
        return Ok(());
    }

    let schema = serde_json::json!({
        "fields": [
            {"name": "body", "type": "text", "required": true}
        ]
    });
    let entry_type = NewEntryType::new("journal", schema, device_id);
    storage.create_entry_type(&entry_type)?;
    Ok(())
}

fn read_entry_body(no_input: bool, body: Option<String>) -> anyhow::Result<String> {
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
            return Err(anyhow::anyhow!("No input provided on stdin"));
        }
        return Ok(trimmed);
    }

    if no_input {
        return Err(anyhow::anyhow!("--no-input requires content from stdin"));
    }

    read_body_from_editor()
}

fn read_body_from_editor() -> anyhow::Result<String> {
    let editor = std::env::var("EDITOR")
        .map_err(|_| anyhow::anyhow!("$EDITOR is not set; use --body or pipe content via stdin"))?;

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

fn entry_type_name_map(storage: &AgeSqliteStorage) -> anyhow::Result<HashMap<Uuid, String>> {
    let types = storage.list_entry_types()?;
    let mut map = HashMap::new();
    for entry_type in types {
        map.insert(entry_type.id, entry_type.name);
    }
    Ok(map)
}

fn entry_json(
    entry: &ledger_core::storage::Entry,
    name_map: &HashMap<Uuid, String>,
) -> serde_json::Value {
    let entry_type_name = name_map
        .get(&entry.entry_type_id)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    serde_json::json!({
        "id": entry.id,
        "entry_type_id": entry.entry_type_id,
        "entry_type_name": entry_type_name,
        "schema_version": entry.schema_version,
        "created_at": entry.created_at,
        "device_id": entry.device_id,
        "tags": entry.tags,
        "data": entry.data,
        "supersedes": entry.supersedes,
    })
}

fn entries_json(
    entries: &[ledger_core::storage::Entry],
    name_map: &HashMap<Uuid, String>,
) -> Vec<serde_json::Value> {
    entries
        .iter()
        .map(|entry| entry_json(entry, name_map))
        .collect()
}

fn print_entry(
    storage: &AgeSqliteStorage,
    entry: &ledger_core::storage::Entry,
    quiet: bool,
) -> anyhow::Result<()> {
    let name_map = entry_type_name_map(storage)?;
    let entry_type_name = name_map
        .get(&entry.entry_type_id)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let body = entry
        .data
        .get("body")
        .and_then(|v| v.as_str())
        .map(|value| value.to_string())
        .unwrap_or_else(|| entry.data.to_string());

    if !quiet {
        println!("ID: {}", entry.id);
        println!("Type: {} (v{})", entry_type_name, entry.schema_version);
        println!("Created: {}", entry.created_at);
        println!("Device: {}", entry.device_id);
        if !entry.tags.is_empty() {
            println!("Tags: {}", entry.tags.join(", "));
        }
        println!();
    }
    println!("{}", body);
    Ok(())
}
