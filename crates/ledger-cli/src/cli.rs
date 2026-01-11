use clap::{Args, Parser, Subcommand};
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

/// Arguments for the `init` command
#[derive(Args)]
pub struct InitArgs {
    /// Path where the ledger will be created
    #[arg(value_name = "PATH")]
    pub path: Option<String>,

    /// Show advanced setup prompts
    #[arg(long)]
    pub advanced: bool,

    /// Disable interactive prompts
    #[arg(long)]
    pub no_input: bool,

    /// Set timezone (use with --advanced or --no-input)
    #[arg(long)]
    pub timezone: Option<String>,

    /// Set default editor (use with --advanced or --no-input)
    #[arg(long)]
    pub editor: Option<String>,

    /// Passphrase cache TTL seconds (advanced)
    #[arg(long)]
    pub passphrase_cache_ttl_seconds: Option<u64>,

    /// Keyfile path override (advanced)
    #[arg(long)]
    pub keyfile_path: Option<String>,

    /// Config path override (advanced)
    #[arg(long)]
    pub config_path: Option<String>,
}

/// Arguments for the `add` command
#[derive(Args)]
pub struct AddArgs {
    /// Entry type to add
    #[arg(value_name = "TYPE")]
    pub entry_type: String,

    /// Entry body (overrides stdin/editor)
    #[arg(long)]
    pub body: Option<String>,

    /// Add tags to the entry
    #[arg(short, long, value_name = "TAG")]
    pub tag: Vec<String>,

    /// Set custom date/time (ISO-8601)
    #[arg(long)]
    pub date: Option<String>,

    /// Disable interactive prompts
    #[arg(long)]
    pub no_input: bool,
}

/// Arguments for the `edit` command
#[derive(Args)]
pub struct EditArgs {
    /// Entry ID (full UUID)
    #[arg(value_name = "ID")]
    pub id: String,

    /// Entry body (overrides stdin/editor)
    #[arg(long)]
    pub body: Option<String>,

    /// Disable interactive prompts
    #[arg(long)]
    pub no_input: bool,
}

/// Arguments for the `list` command
#[derive(Args)]
pub struct ListArgs {
    /// Filter by entry type
    #[arg(value_name = "TYPE")]
    pub entry_type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Time window (e.g., "7d", "30d")
    #[arg(long)]
    pub last: Option<String>,

    /// Start date (ISO-8601)
    #[arg(long)]
    pub since: Option<String>,

    /// End date (ISO-8601)
    #[arg(long)]
    pub until: Option<String>,

    /// Limit number of results
    #[arg(long)]
    pub limit: Option<usize>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Output format (table, plain)
    #[arg(long, value_name = "FORMAT")]
    pub format: Option<String>,

    /// Include superseded revisions
    #[arg(long)]
    pub history: bool,
}

/// Arguments for the `search` command
#[derive(Args)]
pub struct SearchArgs {
    /// Search query
    #[arg(value_name = "QUERY")]
    pub query: String,

    /// Filter by entry type
    #[arg(long)]
    pub r#type: Option<String>,

    /// Time window (e.g., "7d", "30d")
    #[arg(long)]
    pub last: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Limit number of results
    #[arg(long)]
    pub limit: Option<usize>,

    /// Output format (table, plain)
    #[arg(long, value_name = "FORMAT")]
    pub format: Option<String>,

    /// Include superseded revisions
    #[arg(long)]
    pub history: bool,
}

/// Arguments for the `show` command
#[derive(Args)]
pub struct ShowArgs {
    /// Entry ID (full UUID)
    #[arg(value_name = "ID")]
    pub id: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for the `export` command
#[derive(Args)]
pub struct ExportArgs {
    /// Filter by entry type
    #[arg(value_name = "TYPE")]
    pub entry_type: Option<String>,

    /// Output format
    #[arg(long, default_value = "json")]
    pub format: String,

    /// Start date (ISO-8601)
    #[arg(long)]
    pub since: Option<String>,
}

/// Arguments for the `backup` command
#[derive(Args)]
pub struct BackupArgs {
    /// Destination path
    #[arg(value_name = "DEST")]
    pub destination: String,
}

/// Arguments for the `doctor` command
#[derive(Args)]
pub struct DoctorArgs {
    /// Disable interactive prompts
    #[arg(long)]
    pub no_input: bool,
}

/// Arguments for the `completions` command
#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_name = "SHELL")]
    pub shell: Shell,
}

/// Arguments for the internal cache daemon command
#[derive(Args)]
pub struct InternalCacheDaemonArgs {
    #[arg(long)]
    pub ttl: u64,
    #[arg(long)]
    pub socket: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new encrypted ledger
    Init(InitArgs),

    /// Add a new entry to the ledger
    Add(AddArgs),

    /// Edit an existing entry (creates a new revision)
    Edit(EditArgs),

    /// List entries
    List(ListArgs),

    /// Search entries using full-text search
    Search(SearchArgs),

    /// Show a specific entry by ID
    Show(ShowArgs),

    /// Export entries (portable formats, you own your data)
    Export(ExportArgs),

    /// Check ledger integrity
    Check,

    /// Backup the ledger
    Backup(BackupArgs),

    /// Clear cached passphrase (if enabled)
    Lock,

    /// Run onboarding diagnostics
    Doctor(DoctorArgs),

    /// Generate shell completions
    Completions(CompletionsArgs),

    /// Internal cache daemon (not user-facing)
    #[command(hide = true, name = "internal-cache-daemon")]
    InternalCacheDaemon(InternalCacheDaemonArgs),
}
