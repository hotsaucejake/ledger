//! Ledger CLI - A secure, encrypted, CLI-first personal journal and logbook
//!
//! This is the command-line interface for Ledger. It provides a user-friendly
//! interface to the core library functionality.

use clap::{Parser, Subcommand};
use ledger_core::VERSION;

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
            println!("Command: init");
            if let Some(p) = path {
                println!("  Path: {}", p);
            }
            println!("\n[Milestone 0] Not yet implemented.");
        }
        Some(Commands::Add {
            entry_type,
            tag,
            date,
            no_input,
        }) => {
            println!("Command: add");
            println!("  Type: {}", entry_type);
            if !tag.is_empty() {
                println!("  Tags: {}", tag.join(", "));
            }
            if let Some(d) = date {
                println!("  Date: {}", d);
            }
            if no_input {
                println!("  No input: true");
            }
            println!("\n[Milestone 0] Not yet implemented.");
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
            println!("Command: list");
            if let Some(t) = entry_type {
                println!("  Type: {}", t);
            }
            if let Some(t) = tag {
                println!("  Tag: {}", t);
            }
            if let Some(l) = last {
                println!("  Last: {}", l);
            }
            if let Some(s) = since {
                println!("  Since: {}", s);
            }
            if let Some(u) = until {
                println!("  Until: {}", u);
            }
            if let Some(lim) = limit {
                println!("  Limit: {}", lim);
            }
            if json {
                println!("  Format: JSON");
            }
            println!("\n[Milestone 0] Not yet implemented.");
        }
        Some(Commands::Search {
            query,
            r#type,
            last,
        }) => {
            println!("Command: search");
            println!("  Query: {}", query);
            if let Some(t) = r#type {
                println!("  Type: {}", t);
            }
            if let Some(l) = last {
                println!("  Last: {}", l);
            }
            println!("\n[Milestone 0] Not yet implemented.");
        }
        Some(Commands::Show { id }) => {
            println!("Command: show");
            println!("  ID: {}", id);
            println!("\n[Milestone 0] Not yet implemented.");
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
            println!("Command: check");
            println!("\n[Milestone 0] Not yet implemented.");
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
