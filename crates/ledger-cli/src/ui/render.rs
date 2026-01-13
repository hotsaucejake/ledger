//! Rendering primitives for CLI output.

use super::context::UiContext;
use super::format::{pad_right, truncate};
use super::mode::OutputMode;
use super::theme::{colors, Badge};

/// Render a header line for a command.
///
/// Pretty mode: "Ledger · command" with optional path
/// Plain mode: "ledger command"
pub fn header(ctx: &UiContext, command: &str, path: Option<&str>) -> String {
    match ctx.mode {
        OutputMode::Pretty => {
            let mut out = if ctx.color {
                format!(
                    "{}Ledger{} \u{00B7} {}",
                    colors::BRIGHT,
                    colors::RESET,
                    command
                )
            } else {
                format!("Ledger \u{00B7} {}", command)
            };
            if let Some(p) = path {
                out.push_str(&format!("\nPath: {}", p));
            }
            out
        }
        OutputMode::Plain => {
            format!("ledger {}", command)
        }
        OutputMode::Json => String::new(),
    }
}

/// Render a divider line.
pub fn divider(ctx: &UiContext) -> String {
    if ctx.mode.is_pretty() {
        "-".repeat(ctx.width.min(60))
    } else {
        "---".to_string()
    }
}

/// Render a badge with optional message.
pub fn badge(ctx: &UiContext, kind: Badge, message: &str) -> String {
    let badge_text = kind.display(ctx.unicode);

    let colored_badge = if ctx.color {
        let color = match kind {
            Badge::Ok => colors::GREEN,
            Badge::Warn => colors::YELLOW,
            Badge::Err => colors::RED,
            Badge::Info => colors::CYAN,
        };
        format!("{}{}{}", color, badge_text, colors::RESET)
    } else {
        badge_text.to_string()
    };

    if message.is_empty() {
        colored_badge
    } else {
        format!("{} {}", colored_badge, message)
    }
}

/// Render a key-value pair.
///
/// Pretty mode: "Key: value" with dim key
/// Plain mode: "key=value"
pub fn kv(ctx: &UiContext, key: &str, value: &str) -> String {
    if ctx.mode.is_pretty() {
        if ctx.color {
            format!("{}{}:{} {}", colors::DIM, key, colors::RESET, value)
        } else {
            format!("{}: {}", key, value)
        }
    } else {
        format!("{}={}", key.to_lowercase().replace(' ', "_"), value)
    }
}

/// Render a hint line.
///
/// Pretty mode: "Hint: text" with dim styling
/// Plain mode: "hint=text"
pub fn hint(ctx: &UiContext, text: &str) -> String {
    if ctx.mode.is_pretty() {
        if ctx.color {
            format!("{}Hint:{} {}", colors::DIM, colors::RESET, text)
        } else {
            format!("Hint: {}", text)
        }
    } else {
        format!("hint={}", text)
    }
}

/// Render a receipt (summary block after an action).
///
/// Pretty mode: Badge + indented key-value pairs
/// Plain mode: status=ok + key=value lines
pub fn receipt(ctx: &UiContext, title: &str, items: &[(&str, &str)]) -> String {
    let mut lines = Vec::new();

    if ctx.mode.is_pretty() {
        lines.push(badge(ctx, Badge::Ok, title));
        for (key, value) in items {
            lines.push(format!("  {}", kv(ctx, key, value)));
        }
    } else {
        lines.push("status=ok".to_string());
        for (key, value) in items {
            lines.push(kv(ctx, key, value));
        }
    }

    lines.join("\n")
}

/// Column definition for table rendering.
#[derive(Debug, Clone)]
pub struct Column {
    pub header: &'static str,
    pub width: usize,
}

impl Column {
    pub const fn new(header: &'static str, width: usize) -> Self {
        Self { header, width }
    }
}

/// Render a table.
///
/// Pretty mode: Header row + aligned data rows with spacing
/// Plain mode: Space-separated values (no header)
pub fn table(ctx: &UiContext, columns: &[Column], rows: &[Vec<String>]) -> String {
    let mut lines = Vec::new();

    if ctx.mode.is_pretty() {
        // Header row
        let header_line: Vec<String> = columns
            .iter()
            .map(|c| {
                if ctx.color {
                    format!(
                        "{}{}{}",
                        colors::DIM,
                        pad_right(c.header, c.width),
                        colors::RESET
                    )
                } else {
                    pad_right(c.header, c.width)
                }
            })
            .collect();
        lines.push(header_line.join("  "));

        // Data rows
        for row in rows {
            let formatted: Vec<String> = row
                .iter()
                .zip(columns.iter())
                .map(|(cell, col)| {
                    let truncated = truncate(cell, col.width);
                    pad_right(&truncated, col.width)
                })
                .collect();
            lines.push(formatted.join("  "));
        }
    } else {
        // Plain mode: space-separated values, no header
        for row in rows {
            lines.push(row.join(" "));
        }
    }

    lines.join("\n")
}

/// Print a message to stdout with proper mode handling.
///
/// In JSON mode, this does nothing (JSON output should be handled separately).
/// In other modes, prints the message.
pub fn print(ctx: &UiContext, message: &str) {
    if !ctx.mode.is_json() {
        println!("{}", message);
    }
}

/// Print an empty line (only in pretty mode).
pub fn blank_line(ctx: &UiContext) {
    if ctx.mode.is_pretty() {
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_ctx() -> UiContext {
        UiContext {
            is_tty: false,
            color: false,
            unicode: false,
            width: 80,
            mode: OutputMode::Plain,
        }
    }

    fn pretty_ctx() -> UiContext {
        UiContext {
            is_tty: true,
            color: false,
            unicode: true,
            width: 80,
            mode: OutputMode::Pretty,
        }
    }

    #[test]
    fn test_header_pretty() {
        let ctx = pretty_ctx();
        let h = header(&ctx, "list", None);
        assert!(h.contains("Ledger"));
        assert!(h.contains("list"));
    }

    #[test]
    fn test_header_plain() {
        let ctx = plain_ctx();
        let h = header(&ctx, "list", None);
        assert_eq!(h, "ledger list");
    }

    #[test]
    fn test_badge_ok() {
        let ctx = plain_ctx();
        let b = badge(&ctx, Badge::Ok, "Done");
        assert!(b.contains("[OK]"));
        assert!(b.contains("Done"));
    }

    #[test]
    fn test_kv_pretty() {
        let ctx = pretty_ctx();
        let line = kv(&ctx, "Name", "test");
        assert_eq!(line, "Name: test");
    }

    #[test]
    fn test_kv_plain() {
        let ctx = plain_ctx();
        let line = kv(&ctx, "Entry Type", "journal");
        assert_eq!(line, "entry_type=journal");
    }

    #[test]
    fn test_hint_pretty() {
        let ctx = pretty_ctx();
        let h = hint(&ctx, "try this");
        assert_eq!(h, "Hint: try this");
    }

    #[test]
    fn test_hint_plain() {
        let ctx = plain_ctx();
        let h = hint(&ctx, "try this");
        assert_eq!(h, "hint=try this");
    }

    #[test]
    fn test_table_plain() {
        let ctx = plain_ctx();
        let columns = [Column::new("ID", 8), Column::new("Name", 10)];
        let rows = vec![vec!["abc".to_string(), "test".to_string()]];
        let t = table(&ctx, &columns, &rows);
        assert_eq!(t, "abc test");
    }

    #[test]
    fn test_table_pretty() {
        let ctx = pretty_ctx();
        let columns = [Column::new("ID", 8), Column::new("Name", 10)];
        let rows = vec![vec!["abc".to_string(), "test".to_string()]];
        let t = table(&ctx, &columns, &rows);
        assert!(t.contains("ID"));
        assert!(t.contains("Name"));
        assert!(t.contains("abc"));
        assert!(t.contains("test"));
    }
}
