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

    /// Disable colored output
    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,

    /// Use ASCII-only symbols (no Unicode)
    #[arg(long, global = true)]
    pub ascii: bool,
}

/// Arguments for the `init` command
#[derive(Args)]
pub struct InitArgs {
    /// Path where the ledger will be created
    #[arg(value_name = "PATH")]
    pub path: Option<String>,

    /// Disable interactive prompts
    #[arg(long)]
    pub no_input: bool,

    /// Set timezone (use with --no-input or to override prompt)
    #[arg(long)]
    pub timezone: Option<String>,

    /// Set default editor (use with --no-input or to override prompt)
    #[arg(long)]
    pub editor: Option<String>,

    /// Passphrase cache TTL seconds
    #[arg(long)]
    pub passphrase_cache_ttl_seconds: Option<u64>,

    /// Keyfile path override
    #[arg(long)]
    pub keyfile_path: Option<String>,

    /// Config path override
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

    /// Use a specific template instead of the default
    #[arg(long, value_name = "TEMPLATE")]
    pub template: Option<String>,

    /// Attach entry to composition(s)
    #[arg(long, value_name = "COMPOSITION")]
    pub compose: Vec<String>,

    /// Skip automatic composition attachment from template defaults
    #[arg(long)]
    pub no_compose: bool,

    /// Set field values (format: field=value, can be repeated)
    #[arg(long = "field", short = 'f', value_name = "FIELD=VALUE")]
    pub fields: Vec<String>,
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

// ============================================================================
// Composition Commands
// ============================================================================

/// Arguments for the `compositions` command
#[derive(Args)]
pub struct CompositionsArgs {
    #[command(subcommand)]
    pub command: CompositionsSubcommand,
}

#[derive(Subcommand)]
pub enum CompositionsSubcommand {
    /// Create a new composition
    Create(CompositionCreateArgs),
    /// List all compositions
    List(CompositionListArgs),
    /// Show composition details
    Show(CompositionShowArgs),
    /// Rename a composition
    Rename(CompositionRenameArgs),
    /// Delete a composition
    Delete(CompositionDeleteArgs),
}

/// Arguments for creating a composition
#[derive(Args)]
pub struct CompositionCreateArgs {
    /// Name of the composition
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Optional description
    #[arg(long, short)]
    pub description: Option<String>,
}

/// Arguments for listing compositions
#[derive(Args)]
pub struct CompositionListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Limit number of results
    #[arg(long)]
    pub limit: Option<usize>,
}

/// Arguments for showing a composition
#[derive(Args)]
pub struct CompositionShowArgs {
    /// Composition name or ID
    #[arg(value_name = "NAME_OR_ID")]
    pub name_or_id: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for renaming a composition
#[derive(Args)]
pub struct CompositionRenameArgs {
    /// Current composition name or ID
    #[arg(value_name = "NAME_OR_ID")]
    pub name_or_id: String,

    /// New name
    #[arg(value_name = "NEW_NAME")]
    pub new_name: String,
}

/// Arguments for deleting a composition
#[derive(Args)]
pub struct CompositionDeleteArgs {
    /// Composition name or ID
    #[arg(value_name = "NAME_OR_ID")]
    pub name_or_id: String,

    /// Skip confirmation prompt
    #[arg(long)]
    pub force: bool,
}

// ============================================================================
// Template Commands
// ============================================================================

/// Arguments for the `templates` command
#[derive(Args)]
pub struct TemplatesArgs {
    #[command(subcommand)]
    pub command: TemplatesSubcommand,
}

#[derive(Subcommand)]
pub enum TemplatesSubcommand {
    /// Create a new template
    Create(TemplateCreateArgs),
    /// List all templates
    List(TemplateListArgs),
    /// Show template details
    Show(TemplateShowArgs),
    /// Update a template (creates new version)
    Update(TemplateUpdateArgs),
    /// Delete a template
    Delete(TemplateDeleteArgs),
    /// Set the default template for an entry type
    SetDefault(TemplateSetDefaultArgs),
    /// Clear the default template for an entry type
    ClearDefault(TemplateClearDefaultArgs),
}

/// Arguments for creating a template
#[derive(Args)]
pub struct TemplateCreateArgs {
    /// Template name
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Entry type this template is for
    #[arg(long, short = 't', value_name = "TYPE")]
    pub entry_type: String,

    /// Optional description
    #[arg(long, short)]
    pub description: Option<String>,

    /// Template defaults as JSON string
    #[arg(long, value_name = "JSON")]
    pub defaults: Option<String>,

    /// Set as default template for the entry type
    #[arg(long)]
    pub set_default: bool,
}

/// Arguments for listing templates
#[derive(Args)]
pub struct TemplateListArgs {
    /// Filter by entry type
    #[arg(long, short = 't', value_name = "TYPE")]
    pub entry_type: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for showing a template
#[derive(Args)]
pub struct TemplateShowArgs {
    /// Template name or ID
    #[arg(value_name = "NAME_OR_ID")]
    pub name_or_id: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for updating a template
#[derive(Args)]
pub struct TemplateUpdateArgs {
    /// Template name or ID
    #[arg(value_name = "NAME_OR_ID")]
    pub name_or_id: String,

    /// New template defaults as JSON string
    #[arg(long, value_name = "JSON")]
    pub defaults: String,
}

/// Arguments for deleting a template
#[derive(Args)]
pub struct TemplateDeleteArgs {
    /// Template name or ID
    #[arg(value_name = "NAME_OR_ID")]
    pub name_or_id: String,

    /// Skip confirmation prompt
    #[arg(long)]
    pub force: bool,
}

/// Arguments for setting default template
#[derive(Args)]
pub struct TemplateSetDefaultArgs {
    /// Entry type name
    #[arg(value_name = "TYPE")]
    pub entry_type: String,

    /// Template name or ID
    #[arg(value_name = "TEMPLATE")]
    pub template: String,
}

/// Arguments for clearing default template
#[derive(Args)]
pub struct TemplateClearDefaultArgs {
    /// Entry type name
    #[arg(value_name = "TYPE")]
    pub entry_type: String,
}

// ============================================================================
// Attach/Detach Commands
// ============================================================================

/// Arguments for the `attach` command
#[derive(Args)]
pub struct AttachArgs {
    /// Entry ID to attach
    #[arg(value_name = "ENTRY_ID")]
    pub entry_id: String,

    /// Composition name or ID to attach to
    #[arg(value_name = "COMPOSITION")]
    pub composition: String,
}

/// Arguments for the `detach` command
#[derive(Args)]
pub struct DetachArgs {
    /// Entry ID to detach
    #[arg(value_name = "ENTRY_ID")]
    pub entry_id: String,

    /// Composition name or ID to detach from
    #[arg(value_name = "COMPOSITION")]
    pub composition: String,
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

    /// Manage compositions (semantic groupings of entries)
    Compositions(CompositionsArgs),

    /// Manage templates (reusable defaults for entries)
    Templates(TemplatesArgs),

    /// Attach an entry to a composition
    Attach(AttachArgs),

    /// Detach an entry from a composition
    Detach(DetachArgs),
}
