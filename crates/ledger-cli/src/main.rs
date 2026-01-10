//! Ledger CLI - A secure, encrypted, CLI-first personal journal and logbook
//!
//! This is the command-line interface for Ledger. It provides a user-friendly
//! interface to the core library functionality.

mod cache;
mod config;
mod helpers;
mod output;
mod security;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use dialoguer::{Confirm, Input, Select};
use std::io::IsTerminal;
use std::time::Duration;

use chrono::Utc;
use ledger_core::storage::{AgeSqliteStorage, EntryFilter, NewEntry, NewEntryType, StorageEngine};
use ledger_core::VERSION;
use uuid::Uuid;

use cache::{
    cache_clear, cache_config, cache_get, cache_socket_path, cache_store, ledger_hash,
    run_cache_daemon,
};
use config::{
    default_config_path, default_keyfile_path, default_ledger_path, read_config, write_config,
    KeyfileMode, LedgerConfig, SecurityTier,
};
use helpers::{
    ensure_journal_type_name, parse_datetime, parse_duration, parse_output_format,
    prompt_init_passphrase, prompt_passphrase, read_entry_body, OutputFormat,
};
use output::{
    entries_json, entry_json, entry_summary, entry_table_summary, entry_type_name_map, print_entry,
};
use security::{
    generate_key_bytes, key_bytes_to_passphrase, keychain_clear, keychain_get, keychain_set,
    read_keyfile_encrypted, read_keyfile_plain, write_keyfile_encrypted, write_keyfile_plain,
};

const DEFAULT_LIST_LIMIT: usize = 20;
const TABLE_SUMMARY_MAX: usize = 80;

struct SecurityConfig {
    tier: SecurityTier,
    keychain_enabled: bool,
    keyfile_mode: KeyfileMode,
    keyfile_path: Option<std::path::PathBuf>,
    cache_ttl_seconds: u64,
    editor: Option<String>,
}

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

        /// Show advanced setup prompts
        #[arg(long)]
        advanced: bool,

        /// Disable interactive prompts
        #[arg(long)]
        no_input: bool,

        /// Set timezone (use with --advanced or --no-input)
        #[arg(long)]
        timezone: Option<String>,

        /// Set default editor (use with --advanced or --no-input)
        #[arg(long)]
        editor: Option<String>,
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

    /// Clear cached passphrase (if enabled)
    Lock,

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_name = "SHELL")]
        shell: Shell,
    },

    /// Internal cache daemon (not user-facing)
    #[command(hide = true, name = "internal-cache-daemon")]
    InternalCacheDaemon {
        #[arg(long)]
        ttl: u64,
        #[arg(long)]
        socket: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // For Milestone 0, we just show that commands parse correctly
    match &cli.command {
        Some(Commands::Init {
            path,
            advanced,
            no_input,
            timezone,
            editor,
        }) => {
            run_init_wizard(
                &cli,
                path.clone(),
                *advanced,
                *no_input,
                timezone.clone(),
                editor.clone(),
            )?;
        }
        Some(Commands::Add {
            entry_type,
            tag,
            date,
            no_input,
            body,
        }) => {
            ensure_journal_type_name(entry_type)?;

            let (mut storage, passphrase) = open_storage_with_retry(&cli, *no_input)?;
            let entry_type_record = storage.get_entry_type(entry_type)?.unwrap_or_else(|| {
                exit_not_found(&format!("Entry type \"{}\" not found", entry_type))
            });

            let editor_override = load_security_config(&cli)?.editor;
            let body = read_entry_body(*no_input, body.clone(), editor_override.as_deref())?;
            let data = serde_json::json!({ "body": body });
            let metadata = storage.metadata()?;
            let mut new_entry = NewEntry::new(
                entry_type_record.id,
                entry_type_record.version,
                data,
                metadata.device_id,
            )
            .with_tags(tag.clone());
            if let Some(value) = date {
                let parsed = parse_datetime(value)?;
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
            let (storage, _passphrase) = open_storage_with_retry(&cli, false)?;

            let mut filter = EntryFilter::new();
            if let Some(t) = entry_type {
                ensure_journal_type_name(t)?;
                let entry_type_record = storage
                    .get_entry_type(t)?
                    .unwrap_or_else(|| exit_not_found(&format!("Entry type \"{}\" not found", t)));
                filter = filter.entry_type(entry_type_record.id);
            }
            if let Some(t) = tag {
                filter = filter.tag(t.clone());
            }
            if let Some(l) = last {
                let window = parse_duration(l)?;
                let since_time = Utc::now() - window;
                filter = filter.since(since_time);
            }
            if let Some(s) = since {
                let parsed = chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(|e| anyhow::anyhow!("Invalid since timestamp: {}", e))?;
                filter = filter.since(parsed.with_timezone(&chrono::Utc));
            }
            if let Some(u) = until {
                let parsed = chrono::DateTime::parse_from_rfc3339(u)
                    .map_err(|e| anyhow::anyhow!("Invalid until timestamp: {}", e))?;
                filter = filter.until(parsed.with_timezone(&chrono::Utc));
            }
            if let Some(lim) = limit {
                filter = filter.limit(*lim);
            } else if last.is_none() && since.is_none() && until.is_none() {
                filter = filter.limit(DEFAULT_LIST_LIMIT);
            }

            let entries = storage.list_entries(&filter)?;
            let format = parse_output_format(format.as_deref())?;
            if *json {
                if format.is_some() {
                    return Err(anyhow::anyhow!("--format cannot be used with --json"));
                }
                let name_map = entry_type_name_map(&storage)?;
                let output = serde_json::to_string_pretty(&entries_json(&entries, &name_map))?;
                println!("{}", output);
            } else {
                if entries.is_empty() {
                    if !cli.quiet {
                        println!("No entries found.");
                    }
                    return Ok(());
                }
                match format.unwrap_or(OutputFormat::Table) {
                    OutputFormat::Table => {
                        if !cli.quiet {
                            println!("ID | CREATED_AT | SUMMARY");
                        }
                        for entry in entries {
                            let summary = entry_table_summary(&entry, TABLE_SUMMARY_MAX);
                            println!("{} | {} | {}", entry.id, entry.created_at, summary);
                        }
                    }
                    OutputFormat::Plain => {
                        for entry in entries {
                            let summary = entry_summary(&entry);
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
            let (storage, _passphrase) = open_storage_with_retry(&cli, false)?;

            let mut entries = storage.search_entries(query)?;
            if let Some(t) = r#type {
                ensure_journal_type_name(t)?;
                let entry_type_record = storage
                    .get_entry_type(t)?
                    .unwrap_or_else(|| exit_not_found(&format!("Entry type \"{}\" not found", t)));
                entries.retain(|entry| entry.entry_type_id == entry_type_record.id);
            }
            if let Some(l) = last {
                let window = parse_duration(l)?;
                let since = Utc::now() - window;
                entries.retain(|entry| entry.created_at >= since);
            }
            if let Some(lim) = limit {
                entries.truncate(*lim);
            }

            let format = parse_output_format(format.as_deref())?;
            if *json {
                if format.is_some() {
                    return Err(anyhow::anyhow!("--format cannot be used with --json"));
                }
                let name_map = entry_type_name_map(&storage)?;
                let output = serde_json::to_string_pretty(&entries_json(&entries, &name_map))?;
                println!("{}", output);
            } else {
                if entries.is_empty() {
                    if !cli.quiet {
                        println!("No entries found.");
                    }
                    return Ok(());
                }
                match format.unwrap_or(OutputFormat::Table) {
                    OutputFormat::Table => {
                        if !cli.quiet {
                            println!("ID | CREATED_AT | SUMMARY");
                        }
                        for entry in entries {
                            let summary = entry_table_summary(&entry, TABLE_SUMMARY_MAX);
                            println!("{} | {} | {}", entry.id, entry.created_at, summary);
                        }
                    }
                    OutputFormat::Plain => {
                        for entry in entries {
                            let summary = entry_summary(&entry);
                            println!("{} {} {}", entry.id, entry.created_at, summary);
                        }
                    }
                }
            }
        }
        Some(Commands::Show { id, json }) => {
            let (storage, _passphrase) = open_storage_with_retry(&cli, false)?;

            let parsed =
                Uuid::parse_str(id).map_err(|e| anyhow::anyhow!("Invalid entry ID: {}", e))?;
            let entry = storage
                .get_entry(&parsed)?
                .unwrap_or_else(|| exit_not_found("Entry not found"));
            if *json {
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
            let (storage, _passphrase) = open_storage_with_retry(&cli, false)?;

            let mut filter = EntryFilter::new();
            if let Some(t) = entry_type {
                ensure_journal_type_name(t)?;
                let entry_type_record = storage
                    .get_entry_type(t)?
                    .unwrap_or_else(|| exit_not_found(&format!("Entry type \"{}\" not found", t)));
                filter = filter.entry_type(entry_type_record.id);
            }
            if let Some(s) = since {
                let parsed = parse_datetime(s)?;
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
            let (storage, _passphrase) = open_storage_with_retry(&cli, false)?;
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
            let source = resolve_ledger_path(&cli)?;
            let source_path = std::path::Path::new(&source);
            if !source_path.exists() {
                return Err(anyhow::anyhow!(missing_ledger_message(source_path)));
            }
            let count = std::fs::copy(&source, destination).map_err(|e| {
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
        Some(Commands::Lock) => {
            if let Ok(socket_path) = cache_socket_path() {
                let _ = cache_clear(&socket_path);
            }
            if !cli.quiet {
                println!("Passphrase cache cleared.");
            }
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(*shell, &mut cmd, "ledger", &mut std::io::stdout());
        }
        Some(Commands::InternalCacheDaemon { ttl, socket }) => {
            let socket_path = std::path::PathBuf::from(socket);
            run_cache_daemon(Duration::from_secs(*ttl), &socket_path)?;
        }
        None => {
            println!("Ledger v{}", VERSION);
            println!("\nQuickstart:");
            println!("  ledger init");
            println!("  ledger add journal --body \"Hello\"");
            println!("  ledger list --last 7d");
            println!("  ledger search \"Hello\"");
            println!("  ledger show <id>");
            println!("\nRun `ledger --help` for full usage.");
        }
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

fn resolve_ledger_path(cli: &Cli) -> anyhow::Result<String> {
    if let Some(path) = cli.ledger.clone() {
        return Ok(path);
    }

    let config_path = resolve_config_path()?;
    if !config_path.exists() {
        return Err(anyhow::anyhow!(missing_config_message(&config_path)));
    }

    let config = read_config(&config_path)?;
    Ok(config.ledger.path)
}

fn open_storage_with_retry(
    cli: &Cli,
    no_input: bool,
) -> anyhow::Result<(AgeSqliteStorage, String)> {
    let target = resolve_ledger_path(cli)?;
    let interactive = std::io::stdin().is_terminal() && !no_input;
    let target_path = std::path::Path::new(&target);
    let security = load_security_config(cli)?;
    let cache_config = cache_config(target_path, security.cache_ttl_seconds).unwrap_or(None);

    if let Some(config) = cache_config.as_ref() {
        if let Ok(Some(passphrase)) = cache_get(config) {
            match AgeSqliteStorage::open(target_path, &passphrase) {
                Ok(storage) => return Ok((storage, passphrase)),
                Err(err) if is_incorrect_passphrase_error(&err) => {
                    let _ = cache_clear(&config.socket_path);
                }
                Err(err) => return Err(anyhow::anyhow!(err)),
            }
        }
    }

    if matches!(security.tier, SecurityTier::DeviceKeyfile) {
        let keyfile_path = security
            .keyfile_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keyfile path is required for device_keyfile"))?;
        let key_bytes = read_keyfile_plain(keyfile_path)?;
        let passphrase = key_bytes_to_passphrase(&key_bytes);
        return open_with_passphrase_and_cache(
            cli,
            target_path,
            &passphrase,
            cache_config.as_ref(),
        );
    }

    if matches!(security.tier, SecurityTier::PassphraseKeyfile) {
        let keyfile_path = security
            .keyfile_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Keyfile path is required for passphrase_keyfile"))?;
        let env_passphrase = std::env::var("LEDGER_PASSPHRASE")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let key_bytes =
            decrypt_keyfile_with_retry(keyfile_path, env_passphrase.as_deref(), interactive)?;
        let passphrase = key_bytes_to_passphrase(&key_bytes);
        return open_with_passphrase_and_cache(
            cli,
            target_path,
            &passphrase,
            cache_config.as_ref(),
        );
    }

    if matches!(security.tier, SecurityTier::PassphraseKeychain) && security.keychain_enabled {
        let account = ledger_hash(target_path);
        match keychain_get(&account) {
            Ok(Some(passphrase)) => {
                if let Ok(storage) = AgeSqliteStorage::open(target_path, &passphrase) {
                    return Ok((storage, passphrase));
                }
                let _ = keychain_clear(&account);
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("Warning: {}", err);
            }
        }
    }

    let env_passphrase = std::env::var("LEDGER_PASSPHRASE")
        .ok()
        .filter(|v| !v.trim().is_empty());
    if let Some(passphrase) = env_passphrase {
        let (storage, passphrase) =
            open_with_passphrase_and_cache(cli, target_path, &passphrase, cache_config.as_ref())?;
        if matches!(security.tier, SecurityTier::PassphraseKeychain) && security.keychain_enabled {
            let account = ledger_hash(target_path);
            let _ = keychain_set(&account, &passphrase);
        }
        return Ok((storage, passphrase));
    }

    let (storage, passphrase) =
        open_with_retry_prompt(cli, target_path, interactive, cache_config.as_ref())?;
    if matches!(security.tier, SecurityTier::PassphraseKeychain) && security.keychain_enabled {
        let account = ledger_hash(target_path);
        let _ = keychain_set(&account, &passphrase);
    }
    Ok((storage, passphrase))
}

fn is_incorrect_passphrase_error(err: &ledger_core::error::LedgerError) -> bool {
    err.to_string().contains("Incorrect passphrase")
}

fn is_missing_ledger_error(err: &ledger_core::error::LedgerError) -> bool {
    matches!(err, ledger_core::error::LedgerError::Storage(message) if message == "Ledger file not found")
}

fn missing_ledger_message(path: &std::path::Path) -> String {
    format!(
        "No ledger found at {}\n\nRun:\n  ledger init\n\nOr specify a ledger path:\n  LEDGER_PATH=/path/to/my.ledger ledger init",
        path.display()
    )
}

fn missing_config_message(config_path: &std::path::Path) -> String {
    format!(
        "No ledger found at {}\n\nRun:\n  ledger init\n\nOr specify a ledger path:\n  LEDGER_PATH=/path/to/my.ledger ledger init",
        config_path.display()
    )
}

fn exit_not_found(message: &str) -> ! {
    eprintln!("Error: {}", message);
    std::process::exit(3);
}

fn load_security_config(_cli: &Cli) -> anyhow::Result<SecurityConfig> {
    let config_path = resolve_config_path()?;
    if config_path.exists() {
        let config = read_config(&config_path)?;
        let keyfile_path = config.keyfile.path.as_ref().map(std::path::PathBuf::from);
        let security = SecurityConfig {
            tier: config.security.tier,
            keychain_enabled: config.keychain.enabled,
            keyfile_mode: config.keyfile.mode,
            keyfile_path,
            cache_ttl_seconds: config.security.passphrase_cache_ttl_seconds,
            editor: config.ui.editor,
        };
        validate_security_config(&security)?;
        return Ok(security);
    }

    Ok(SecurityConfig {
        tier: SecurityTier::Passphrase,
        keychain_enabled: false,
        keyfile_mode: KeyfileMode::None,
        keyfile_path: Some(default_keyfile_path()?),
        cache_ttl_seconds: 0,
        editor: None,
    })
}

fn validate_security_config(config: &SecurityConfig) -> anyhow::Result<()> {
    match config.tier {
        SecurityTier::PassphraseKeyfile => {
            if !matches!(config.keyfile_mode, KeyfileMode::Encrypted) {
                return Err(anyhow::anyhow!(
                    "keyfile mode must be encrypted for passphrase_keyfile"
                ));
            }
            if config.keyfile_path.is_none() {
                return Err(anyhow::anyhow!(
                    "keyfile path is required for passphrase_keyfile"
                ));
            }
        }
        SecurityTier::DeviceKeyfile => {
            if !matches!(config.keyfile_mode, KeyfileMode::Plain) {
                return Err(anyhow::anyhow!(
                    "keyfile mode must be plain for device_keyfile"
                ));
            }
            if config.keyfile_path.is_none() {
                return Err(anyhow::anyhow!(
                    "keyfile path is required for device_keyfile"
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn open_with_passphrase_and_cache(
    cli: &Cli,
    path: &std::path::Path,
    passphrase: &str,
    cache_config: Option<&cache::CacheConfig>,
) -> anyhow::Result<(AgeSqliteStorage, String)> {
    match AgeSqliteStorage::open(path, passphrase) {
        Ok(storage) => {
            if let Some(config) = cache_config {
                if !cli.quiet {
                    println!(
                        "Note: Passphrase caching keeps your passphrase in memory for {} seconds.",
                        config.ttl.as_secs()
                    );
                }
                let _ = cache_store(config, passphrase);
            }
            Ok((storage, passphrase.to_string()))
        }
        Err(err) if is_incorrect_passphrase_error(&err) => {
            eprintln!("Error: Incorrect passphrase.");
            std::process::exit(5);
        }
        Err(err) if is_missing_ledger_error(&err) => {
            Err(anyhow::anyhow!(missing_ledger_message(path)))
        }
        Err(err) => Err(anyhow::anyhow!(err)),
    }
}

fn open_with_retry_prompt(
    cli: &Cli,
    path: &std::path::Path,
    interactive: bool,
    cache_config: Option<&cache::CacheConfig>,
) -> anyhow::Result<(AgeSqliteStorage, String)> {
    let max_attempts: u32 = if interactive { 3 } else { 1 };
    let mut attempts: u32 = 0;

    loop {
        attempts += 1;
        let passphrase = prompt_passphrase(interactive)?;
        match AgeSqliteStorage::open(path, &passphrase) {
            Ok(storage) => {
                if let Some(config) = cache_config {
                    if !cli.quiet {
                        println!(
                            "Note: Passphrase caching keeps your passphrase in memory for {} seconds.",
                            config.ttl.as_secs()
                        );
                    }
                    let _ = cache_store(config, &passphrase);
                }
                return Ok((storage, passphrase));
            }
            Err(err) if is_incorrect_passphrase_error(&err) => {
                let remaining = max_attempts.saturating_sub(attempts);
                if remaining == 0 {
                    eprintln!("Error: Too many failed passphrase attempts.");
                    eprintln!(
                        "Hint: If you forgot your passphrase, the ledger cannot be recovered."
                    );
                    eprintln!("      Backups use the same passphrase.");
                    std::process::exit(5);
                }
                eprintln!(
                    "Incorrect passphrase. {} attempt{} remaining.",
                    remaining,
                    if remaining == 1 { "" } else { "s" }
                );
                continue;
            }
            Err(err) if is_missing_ledger_error(&err) => {
                return Err(anyhow::anyhow!(missing_ledger_message(path)));
            }
            Err(err) => return Err(anyhow::anyhow!(err)),
        }
    }
}

fn decrypt_keyfile_with_retry(
    path: &std::path::Path,
    passphrase_env: Option<&str>,
    interactive: bool,
) -> anyhow::Result<zeroize::Zeroizing<Vec<u8>>> {
    if let Some(passphrase) = passphrase_env {
        return match read_keyfile_encrypted(path, passphrase) {
            Ok(bytes) => Ok(bytes),
            Err(err) if err.to_string().contains("Incorrect passphrase") => {
                eprintln!("Error: Incorrect passphrase.");
                std::process::exit(5);
            }
            Err(err) => Err(err),
        };
    }

    let max_attempts: u32 = if interactive { 3 } else { 1 };
    let mut attempts: u32 = 0;

    loop {
        attempts += 1;
        let passphrase = prompt_passphrase(interactive)?;
        match read_keyfile_encrypted(path, &passphrase) {
            Ok(bytes) => return Ok(bytes),
            Err(err) if err.to_string().contains("Incorrect passphrase") => {
                let remaining = max_attempts.saturating_sub(attempts);
                if remaining == 0 {
                    eprintln!("Error: Too many failed passphrase attempts.");
                    eprintln!(
                        "Hint: If you forgot your passphrase, the ledger cannot be recovered."
                    );
                    eprintln!("      Backups use the same passphrase.");
                    std::process::exit(5);
                }
                eprintln!(
                    "Incorrect passphrase. {} attempt{} remaining.",
                    remaining,
                    if remaining == 1 { "" } else { "s" }
                );
                continue;
            }
            Err(err) => return Err(err),
        }
    }
}

fn run_init_wizard(
    cli: &Cli,
    path: Option<String>,
    advanced: bool,
    no_input: bool,
    timezone_arg: Option<String>,
    editor_arg: Option<String>,
) -> anyhow::Result<()> {
    let interactive = std::io::stdin().is_terminal();
    let effective_no_input = no_input || !interactive;

    if !cli.quiet && !effective_no_input {
        println!("Welcome to Ledger.\n");
    }

    let default_ledger = default_ledger_path()?;
    let ledger_path = match path.or_else(|| cli.ledger.clone()) {
        Some(value) => std::path::PathBuf::from(value),
        None => {
            if effective_no_input {
                default_ledger.clone()
            } else {
                let input: String = Input::new()
                    .with_prompt("Ledger file location")
                    .default(default_ledger.to_string_lossy().to_string())
                    .interact_text()?;
                std::path::PathBuf::from(input)
            }
        }
    };

    let mut config_path = resolve_config_path()?;
    let mut passphrase_cache_ttl_seconds = 0_u64;
    let mut keyfile_path = default_keyfile_path()?;
    let mut timezone: Option<String> = timezone_arg;
    let mut editor: Option<String> = editor_arg;

    let passphrase = if let Ok(value) = std::env::var("LEDGER_PASSPHRASE") {
        if !value.trim().is_empty() {
            value
        } else if effective_no_input {
            return Err(anyhow::anyhow!(
                "--no-input requires LEDGER_PASSPHRASE for initialization"
            ));
        } else {
            prompt_init_passphrase()?
        }
    } else if effective_no_input {
        return Err(anyhow::anyhow!(
            "--no-input requires LEDGER_PASSPHRASE for initialization"
        ));
    } else {
        prompt_init_passphrase()?
    };

    let mut tier = SecurityTier::Passphrase;
    if !effective_no_input {
        let options = [
            "Passphrase only (recommended)",
            "Passphrase + OS keychain",
            "Passphrase + encrypted keyfile",
            "Device keyfile only (reduced security)",
        ];
        let choice = Select::new()
            .with_prompt("Security level")
            .default(0)
            .items(&options)
            .interact()?;
        tier = match choice {
            0 => SecurityTier::Passphrase,
            1 => SecurityTier::PassphraseKeychain,
            2 => SecurityTier::PassphraseKeyfile,
            3 => SecurityTier::DeviceKeyfile,
            _ => SecurityTier::Passphrase,
        };
    }

    if matches!(tier, SecurityTier::DeviceKeyfile) && !effective_no_input {
        let proceed = Confirm::new()
            .with_prompt(device_keyfile_warning())
            .default(false)
            .interact()?;
        if !proceed {
            return Err(anyhow::anyhow!("Initialization cancelled"));
        }
    }

    if advanced && !effective_no_input {
        let tz_input: String = Input::new()
            .with_prompt("Timezone")
            .default("auto".to_string())
            .interact_text()?;
        if tz_input.trim().is_empty() || tz_input.trim().eq_ignore_ascii_case("auto") {
            timezone = None;
        } else {
            timezone = Some(tz_input);
        }

        let default_editor = default_editor();
        let editor_input: String = Input::new()
            .with_prompt("Default editor")
            .default(default_editor)
            .interact_text()?;
        if !editor_input.trim().is_empty() {
            editor = Some(editor_input);
        }

        let ttl_input: String = Input::new()
            .with_prompt("Passphrase cache (seconds)")
            .default(passphrase_cache_ttl_seconds.to_string())
            .interact_text()?;
        passphrase_cache_ttl_seconds = ttl_input.parse().map_err(|_| {
            anyhow::anyhow!(
                "Invalid cache TTL: {} (expected integer seconds)",
                ttl_input
            )
        })?;

        if matches!(
            tier,
            SecurityTier::PassphraseKeyfile | SecurityTier::DeviceKeyfile
        ) {
            let input: String = Input::new()
                .with_prompt("Keyfile path")
                .default(keyfile_path.to_string_lossy().to_string())
                .interact_text()?;
            keyfile_path = std::path::PathBuf::from(input);
        }

        let input: String = Input::new()
            .with_prompt("Ledger config path")
            .default(config_path.to_string_lossy().to_string())
            .interact_text()?;
        config_path = std::path::PathBuf::from(input);
    }

    let (ledger_passphrase, keyfile_mode, keyfile_path_value) = match tier {
        SecurityTier::Passphrase => (passphrase.clone(), KeyfileMode::None, None),
        SecurityTier::PassphraseKeychain => (passphrase.clone(), KeyfileMode::None, None),
        SecurityTier::PassphraseKeyfile => {
            let key_bytes = generate_key_bytes()?;
            write_keyfile_encrypted(&keyfile_path, &key_bytes, &passphrase)?;
            (
                key_bytes_to_passphrase(&key_bytes),
                KeyfileMode::Encrypted,
                Some(keyfile_path.clone()),
            )
        }
        SecurityTier::DeviceKeyfile => {
            let key_bytes = generate_key_bytes()?;
            write_keyfile_plain(&keyfile_path, &key_bytes)?;
            (
                key_bytes_to_passphrase(&key_bytes),
                KeyfileMode::Plain,
                Some(keyfile_path.clone()),
            )
        }
    };

    if let Some(parent) = ledger_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!(
                "Failed to create ledger directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let device_id = AgeSqliteStorage::create(&ledger_path, &ledger_passphrase)?;
    let mut storage = AgeSqliteStorage::open(&ledger_path, &ledger_passphrase)?;
    ensure_journal_entry_type(&mut storage, device_id)?;
    storage.close(&ledger_passphrase)?;

    let config = LedgerConfig::new(
        ledger_path.clone(),
        tier,
        passphrase_cache_ttl_seconds,
        keyfile_mode,
        keyfile_path_value,
        timezone,
        editor,
    );
    write_config(&config_path, &config)?;

    if matches!(tier, SecurityTier::PassphraseKeychain) {
        let account = ledger_hash(&ledger_path);
        let _ = keychain_set(&account, &passphrase);
    }

    if !cli.quiet {
        println!("Ledger created at {}", ledger_path.to_string_lossy());
        println!("Config written to {}", config_path.to_string_lossy());
        if passphrase_cache_ttl_seconds > 0 {
            println!(
                "Note: Passphrase caching keeps your passphrase in memory for {} seconds.",
                passphrase_cache_ttl_seconds
            );
        }
    }

    Ok(())
}

fn default_editor() -> String {
    std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string())
}

fn resolve_config_path() -> anyhow::Result<std::path::PathBuf> {
    if let Ok(value) = std::env::var("LEDGER_CONFIG") {
        if !value.trim().is_empty() {
            return Ok(std::path::PathBuf::from(value));
        }
    }
    default_config_path()
}

fn device_keyfile_warning() -> &'static str {
    "WARNING: You selected device_keyfile. This stores an unencrypted key on disk.\nIf your device is compromised, your ledger can be decrypted without a passphrase.\nContinue?"
}

#[cfg(test)]
mod tests {
    use super::device_keyfile_warning;

    #[test]
    fn test_device_keyfile_warning_copy() {
        let warning = device_keyfile_warning();
        assert!(warning.contains("device_keyfile"));
        assert!(warning.contains("unencrypted key on disk"));
        assert!(warning.contains("Continue?"));
    }
}
