//! UI primitives for the Ledger CLI.
//!
//! This module provides:
//! - **Context**: Environment detection (TTY, width, color, unicode)
//! - **Mode**: Output mode resolution (json, plain, pretty)
//! - **Theme**: Badge tokens, color palette, symbols
//! - **Render**: Tables, headers, receipts, hints, formatted text
//! - **Progress**: Spinners, progress bars, step lists
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
//! println!("{}", header(&ctx, "list", Some(&ledger_path)));
//! println!("{}", table(&ctx, &columns, &rows));
//! println!("{}", hint(&ctx, "ledger show <id>"));
//! ```

mod context;
pub mod format;
mod mode;
pub mod progress;
pub mod render;
pub mod theme;

// Re-export core types at module level
pub use context::UiContext;
pub use mode::OutputMode;
pub use theme::Badge;

// Re-export commonly used render functions
pub use render::{
    badge, blank_line, divider, header, hint, kv, print, receipt, simple_table, table, Column,
};

// Re-export progress types
pub use progress::{ProgressBar, Spinner, StepList};

// Re-export commonly used format functions
pub use format::{format_bytes, format_datetime, format_duration_secs, short_id, truncate};
