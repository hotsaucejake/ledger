use clap::{Parser, Subcommand};
use clap_complete::Shell;

use ledger_core::VERSION;

/// Ledger - A secure, encrypted, CLI-first personal journal and logbook
#[derive(Parser)]
#[command(name = "ledger")]
#[command(author, version = VERSION, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Path to the ledger file
    #[arg(short, long, global = true, env = "LEDGER_PATH")]
    pub ledger: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Quiet mode (minimal output)
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
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

        /// Passphrase cache TTL seconds (advanced)
        #[arg(long)]
        passphrase_cache_ttl_seconds: Option<u64>,

        /// Keyfile path override (advanced)
        #[arg(long)]
        keyfile_path: Option<String>,

        /// Config path override (advanced)
        #[arg(long)]
        config_path: Option<String>,
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

    /// Edit an existing entry (creates a new revision)
    Edit {
        /// Entry ID (full UUID)
        #[arg(value_name = "ID")]
        id: String,

        /// Entry body (overrides stdin/editor)
        #[arg(long)]
        body: Option<String>,

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

        /// Include superseded revisions
        #[arg(long)]
        history: bool,
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

        /// Include superseded revisions
        #[arg(long)]
        history: bool,
    },

    /// Show a specific entry by ID
    Show {
        /// Entry ID (full UUID)
        #[arg(value_name = "ID")]
        id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Export entries (portable formats, you own your data)
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

    /// Run onboarding diagnostics
    Doctor {
        /// Disable interactive prompts
        #[arg(long)]
        no_input: bool,
    },

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
