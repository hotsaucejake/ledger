//! Output formatting helpers for the CLI.
//!
//! This module provides formatting utilities for displaying entries
//! in various formats (JSON, table, plain text).

mod json;
mod text;

// Re-export public API
pub use json::{entries_json, entry_json};
pub use text::{entry_type_name_map, print_entry, print_entry_list};
