//! UI primitives for the Jot CLI.
//!
//! This module provides:
//! - **Context**: Environment detection (TTY, width, color, unicode)
//! - **Mode**: Output mode resolution (json, plain, pretty)
//! - **Theme**: Badge tokens, color palette, symbols
//! - **Render**: Tables, headers, receipts, hints, formatted text
//! - **Progress**: Spinners, progress bars, step lists
//! - **Prompt**: Wizard flows and guided interactive prompts
//! - **Format**: String utilities (truncate, wrap, align)
//!
//! # Usage
//!
//! ```ignore
//! use crate::ui::{UiContext, OutputMode, Badge};
//! use crate::ui::render::{header, table, badge, hint};
//!
//! let ctx = UiContext::from_env(args.json, args.format.as_deref(), cli.no_color, cli.ascii);
//!
//! if ctx.mode.is_json() {
//!     // Handle JSON output separately
//!     return Ok(());
//! }
//!
//! println!("{}", header(&ctx, "list", Some(&jot_path)));
//! println!("{}", table(&ctx, &columns, &rows));
//! println!("{}", hint(&ctx, "jot show <id>"));
//! ```

mod context;
#[allow(dead_code)]
pub mod format;
mod mode;
#[allow(dead_code)]
pub mod progress;
#[allow(dead_code)]
pub mod prompt;
pub mod render;
pub mod theme;

// Re-export core types at module level
pub use context::UiContext;
pub use mode::OutputMode;
pub use theme::Badge;

// Re-export commonly used render functions
pub use render::{
    badge, blank_line, divider, header, header_with_context, hint, kv, print, print_error,
    simple_table, Column,
};

// Re-export progress types (for future P2 use)
pub use progress::StepList;

// Re-export commonly used format functions
pub use format::{entry_summary, format_bytes, highlight_matches, short_id, truncate};
