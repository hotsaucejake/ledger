//! Ledger CLI - A secure, encrypted, CLI-first personal journal and logbook
//!
//! This is the command-line interface for Ledger. It provides a user-friendly
//! interface to the core library functionality.

use clap::{Parser, Subcommand};
use std::io::{self, IsTerminal, Read};

use dialoguer::{Input, Password};
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
    },

    /// Show a specific entry by ID
    Show {
        /// Entry ID (full UUID or prefix)
        #[arg(value_name = "ID")]
        id: String,
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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // For Milestone 0, we just show that commands parse correctly
    match cli.command {
        Some(Commands::Init { path }) => {
            let target = path.or(cli.ledger).ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;

            let passphrase = Password::new()
                .with_prompt("Enter passphrase")
                .with_confirmation("Confirm passphrase", "Passphrases do not match")
                .interact()
                .map_err(|e| anyhow::anyhow!("Failed to read passphrase: {}", e))?;

            let device_id =
                AgeSqliteStorage::create(std::path::Path::new(&target), &passphrase)?;
            let mut storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;
            ensure_journal_entry_type(&mut storage, device_id)?;
            storage.close()?;

            println!("Initialized new ledger at {}", target);
        }
        Some(Commands::Add {
            entry_type,
            tag,
            date,
            no_input,
        }) => {
            if date.is_some() {
                return Err(anyhow::anyhow!("--date is not supported yet"));
            }

            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;

            let mut storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;
            let entry_type_record = storage
                .get_entry_type(&entry_type)?
                .ok_or_else(|| anyhow::anyhow!("Entry type \"{}\" not found", entry_type))?;

            let body = read_entry_body(no_input)?;
            let data = serde_json::json!({ "body": body });
            let metadata = storage.metadata()?;
            let new_entry =
                NewEntry::new(entry_type_record.id, entry_type_record.version, data, metadata.device_id)
                    .with_tags(tag);

            let entry_id = storage.insert_entry(&new_entry)?;
            storage.close()?;

            println!("Added entry {}", entry_id);
        }
        Some(Commands::List {
            entry_type,
            tag,
            last,
            since,
            until,
            limit,
            json,
        }) => {
            if last.is_some() {
                return Err(anyhow::anyhow!("--last is not supported yet"));
            }

            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;
            let storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;

            let mut filter = EntryFilter::new();
            if let Some(t) = entry_type {
                let entry_type_record = storage
                    .get_entry_type(&t)?
                    .ok_or_else(|| anyhow::anyhow!("Entry type \"{}\" not found", t))?;
                filter = filter.entry_type(entry_type_record.id);
            }
            if let Some(t) = tag {
                filter = filter.tag(t);
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
            if json {
                let output = serde_json::to_string_pretty(&entries)?;
                println!("{}", output);
            } else {
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
        Some(Commands::Search {
            query,
            r#type,
            last,
        }) => {
            if last.is_some() {
                return Err(anyhow::anyhow!("--last is not supported yet"));
            }

            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;
            let storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;

            let mut entries = storage.search_entries(&query)?;
            if let Some(t) = r#type {
                let entry_type_record = storage
                    .get_entry_type(&t)?
                    .ok_or_else(|| anyhow::anyhow!("Entry type \"{}\" not found", t))?;
                entries.retain(|entry| entry.entry_type_id == entry_type_record.id);
            }

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
        Some(Commands::Show { id }) => {
            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;
            let storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;

            let parsed = Uuid::parse_str(&id)
                .map_err(|e| anyhow::anyhow!("Invalid entry ID: {}", e))?;
            let entry = storage
                .get_entry(&parsed)?
                .ok_or_else(|| anyhow::anyhow!("Entry not found"))?;
            let output = serde_json::to_string_pretty(&entry)?;
            println!("{}", output);
        }
        Some(Commands::Export {
            entry_type,
            format,
            since,
        }) => {
            println!("Command: export");
            if let Some(t) = entry_type {
                println!("  Type: {}", t);
            }
            println!("  Format: {}", format);
            if let Some(s) = since {
                println!("  Since: {}", s);
            }
            println!("\n[Milestone 0] Not yet implemented.");
        }
        Some(Commands::Check) => {
            let target = cli.ledger.ok_or_else(|| {
                anyhow::anyhow!("No ledger path provided. Use --ledger or pass a path.")
            })?;
            let passphrase = prompt_passphrase()?;
            let storage = AgeSqliteStorage::open(std::path::Path::new(&target), &passphrase)?;
            storage.check_integrity()?;
            println!("Integrity check passed.");
        }
        Some(Commands::Backup { destination }) => {
            println!("Command: backup");
            println!("  Destination: {}", destination);
            println!("\n[Milestone 0] Not yet implemented.");
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
    Password::new()
        .with_prompt("Passphrase")
        .interact()
        .map_err(|e| anyhow::anyhow!("Failed to read passphrase: {}", e))
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

fn read_entry_body(no_input: bool) -> anyhow::Result<String> {
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
        return Err(anyhow::anyhow!(
            "--no-input requires content from stdin"
        ));
    }

    Input::new()
        .with_prompt("Entry")
        .interact_text()
        .map_err(|e| anyhow::anyhow!("Failed to read entry: {}", e))
}
