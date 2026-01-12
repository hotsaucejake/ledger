//! Input and parsing helper functions for the CLI.
//!
//! This module provides utilities for:
//! - Passphrase prompting and entry body reading (`input`)
//! - Datetime, duration, and format parsing (`parsing`)
//! - Field prompting and validation (`fields`)

mod fields;
mod input;
mod parsing;

// Re-export public API
pub use fields::{parse_cli_fields, prompt_for_fields, FieldDef, TemplateDefaults};
pub use input::{prompt_init_passphrase, prompt_passphrase, read_entry_body};
pub use parsing::{
    ensure_journal_type_name, parse_datetime, parse_duration, parse_output_format,
    require_entry_type, OutputFormat,
};
