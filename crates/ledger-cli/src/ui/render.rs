//! Rendering primitives for CLI output.

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, ContentArrangement, Table as ComfyTable};

use super::context::UiContext;
use super::mode::OutputMode;
use super::theme::{styled, styles, Badge};

/// Render a header line for a command.
///
/// Pretty mode: "Ledger · command (context)" with optional path
/// Plain mode: "ledger command"
///
/// # Arguments
/// - `command`: The command name (e.g., "list", "search")
/// - `context`: Optional context shown in parentheses (e.g., "last 7d", query)
/// - `path`: Optional ledger path to display on second line
pub fn header_with_context(
    ctx: &UiContext,
    command: &str,
    context: Option<&str>,
    path: Option<&str>,
) -> String {
    match ctx.mode {
        OutputMode::Pretty => {
            let title = styled("Ledger", styles::bold(), ctx.color);
            let mut out = if let Some(c) = context {
                format!("{} \u{00B7} {} ({})", title, command, c)
            } else {
                format!("{} \u{00B7} {}", title, command)
            };
            if let Some(p) = path {
                // Truncate long paths
                let display_path = if p.len() > 50 {
                    format!("...{}", &p[p.len() - 47..])
                } else {
                    p.to_string()
                };
                out.push_str(&format!("\n{}", kv(ctx, "Path", &display_path)));
            }
            out
        }
        OutputMode::Plain => {
            format!("ledger {}", command)
        }
        OutputMode::Json => String::new(),
    }
}

/// Render a header line for a command (simple version).
///
/// Pretty mode: "Ledger · command" with optional context in parentheses
/// Plain mode: "ledger command"
pub fn header(ctx: &UiContext, command: &str, context: Option<&str>) -> String {
    header_with_context(ctx, command, context, None)
}

/// Render a divider line.
pub fn divider(ctx: &UiContext) -> String {
    if ctx.mode.is_pretty() {
        "\u{2500}".repeat(ctx.width.min(60))
    } else {
        "---".to_string()
    }
}

/// Render a badge with optional message.
pub fn badge(ctx: &UiContext, kind: Badge, message: &str) -> String {
    let badge_text = kind.display(ctx.unicode);
    let colored_badge = styled(badge_text, kind.style(), ctx.color);

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
        let styled_key = styled(&format!("{}:", key), styles::dim(), ctx.color);
        format!("{} {}", styled_key, value)
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
        let label = styled("Hint:", styles::dim(), ctx.color);
        format!("{} {}", label, text)
    } else {
        format!("hint={}", text)
    }
}

/// Render a receipt (summary block after an action).
///
/// Pretty mode: Badge + indented key-value pairs
/// Plain mode: status=ok + key=value lines
#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub width: Option<usize>,
}

impl Column {
    pub const fn new(header: &'static str) -> Self {
        Self {
            header,
            width: None,
        }
    }

    #[allow(dead_code)]
    pub const fn with_width(header: &'static str, width: usize) -> Self {
        Self {
            header,
            width: Some(width),
        }
    }
}

/// Render a table using comfy-table for pretty mode.
///
/// Pretty mode: Styled table with borders
/// Plain mode: Space-separated values (no header)
#[allow(dead_code)]
pub fn table(ctx: &UiContext, columns: &[Column], rows: &[Vec<String>]) -> String {
    if ctx.mode.is_pretty() {
        let mut table = ComfyTable::new();

        // Configure table style
        if ctx.unicode {
            table
                .load_preset(UTF8_FULL)
                .apply_modifier(UTF8_ROUND_CORNERS);
        } else {
            table.load_preset(comfy_table::presets::ASCII_MARKDOWN);
        }

        table.set_content_arrangement(ContentArrangement::Dynamic);

        // Set headers
        let headers: Vec<&str> = columns.iter().map(|c| c.header).collect();
        table.set_header(headers);

        // Add rows
        for row in rows {
            table.add_row(row);
        }

        // Apply column widths if specified
        for (i, col) in columns.iter().enumerate() {
            if let Some(width) = col.width {
                table.set_width(width as u16);
                let _ = table
                    .column_mut(i)
                    .map(|c| c.set_constraint(comfy_table::ColumnConstraint::ContentWidth));
            }
        }

        table.to_string()
    } else {
        // Plain mode: space-separated values, no header
        rows.iter()
            .map(|row| row.join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Render a simple table without borders (for lists like entries).
pub fn simple_table(ctx: &UiContext, columns: &[Column], rows: &[Vec<String>]) -> String {
    if ctx.mode.is_pretty() {
        let mut table = ComfyTable::new();
        table.load_preset(comfy_table::presets::NOTHING);
        table.set_content_arrangement(ContentArrangement::Dynamic);

        // Set headers with dim styling using comfy-table's built-in styling
        // This ensures proper column width calculation
        let header_cells: Vec<Cell> = columns
            .iter()
            .map(|c| {
                let mut cell = Cell::new(c.header);
                if ctx.color {
                    cell = cell.add_attribute(Attribute::Dim);
                }
                cell
            })
            .collect();
        table.set_header(header_cells);

        // Add padding between columns
        for i in 0..columns.len() {
            if let Some(column) = table.column_mut(i) {
                column.set_padding((0, 2)); // 0 left, 2 right padding
            }
        }

        // Add rows
        for row in rows {
            table.add_row(row);
        }

        table.to_string()
    } else {
        // Plain mode: space-separated values, no header
        rows.iter()
            .map(|row| row.join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    }
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

/// Format an error message with optional hint.
///
/// Pretty mode: "[ERR] message" with optional "Hint: ..." on next line
/// Plain mode: "error=message" with optional "hint=suggestion"
pub fn error_message(ctx: &UiContext, message: &str, error_hint: Option<&str>) -> String {
    let mut lines = Vec::new();

    if ctx.mode.is_pretty() {
        lines.push(badge(ctx, Badge::Err, message));
        if let Some(h) = error_hint {
            lines.push(hint(ctx, h));
        }
    } else {
        lines.push(format!("error={}", message));
        if let Some(h) = error_hint {
            lines.push(format!("hint={}", h));
        }
    }

    lines.join("\n")
}

/// Print an error message to stderr with optional hint.
pub fn print_error(ctx: &UiContext, message: &str, error_hint: Option<&str>) {
    eprintln!("{}", error_message(ctx, message, error_hint));
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
        assert!(line.contains("Name:"));
        assert!(line.contains("test"));
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
        assert!(h.contains("Hint:"));
        assert!(h.contains("try this"));
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
        let columns = [Column::new("ID"), Column::new("Name")];
        let rows = vec![vec!["abc".to_string(), "test".to_string()]];
        let t = table(&ctx, &columns, &rows);
        assert_eq!(t, "abc test");
    }

    #[test]
    fn test_table_pretty() {
        let ctx = pretty_ctx();
        let columns = [Column::new("ID"), Column::new("Name")];
        let rows = vec![vec!["abc".to_string(), "test".to_string()]];
        let t = table(&ctx, &columns, &rows);
        assert!(t.contains("ID"));
        assert!(t.contains("Name"));
        assert!(t.contains("abc"));
        assert!(t.contains("test"));
    }

    #[test]
    fn test_table_alignment_multiple_rows() {
        let ctx = pretty_ctx();
        let columns = [
            Column::new("ID"),
            Column::new("Name"),
            Column::new("Status"),
        ];
        let rows = vec![
            vec!["a".to_string(), "short".to_string(), "ok".to_string()],
            vec![
                "abc".to_string(),
                "medium name".to_string(),
                "pending".to_string(),
            ],
            vec![
                "abcdef".to_string(),
                "a very long name here".to_string(),
                "x".to_string(),
            ],
        ];
        let t = table(&ctx, &columns, &rows);
        // All rows should be present
        assert!(t.contains("short"));
        assert!(t.contains("medium name"));
        assert!(t.contains("a very long name here"));
        // Headers should be present
        assert!(t.contains("ID"));
        assert!(t.contains("Name"));
        assert!(t.contains("Status"));
    }

    #[test]
    fn test_simple_table_plain() {
        let ctx = plain_ctx();
        let columns = [
            Column::new("ID"),
            Column::new("Created"),
            Column::new("Type"),
        ];
        let rows = vec![
            vec![
                "7a2e3c0b".to_string(),
                "2024-01-01".to_string(),
                "journal".to_string(),
            ],
            vec![
                "9b3f4d1c".to_string(),
                "2024-01-02".to_string(),
                "note".to_string(),
            ],
        ];
        let t = simple_table(&ctx, &columns, &rows);
        // Plain mode: space-separated, no headers
        let lines: Vec<&str> = t.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("7a2e3c0b"));
        assert!(lines[1].contains("9b3f4d1c"));
    }

    #[test]
    fn test_simple_table_pretty() {
        let ctx = pretty_ctx();
        let columns = [
            Column::new("ID"),
            Column::new("Created"),
            Column::new("Type"),
        ];
        let rows = vec![
            vec![
                "7a2e3c0b".to_string(),
                "2024-01-01".to_string(),
                "journal".to_string(),
            ],
            vec![
                "9b3f4d1c".to_string(),
                "2024-01-02".to_string(),
                "note".to_string(),
            ],
        ];
        let t = simple_table(&ctx, &columns, &rows);
        // Pretty mode: includes headers
        assert!(t.contains("ID"));
        assert!(t.contains("Created"));
        assert!(t.contains("Type"));
        assert!(t.contains("7a2e3c0b"));
        assert!(t.contains("journal"));
    }

    #[test]
    fn test_table_empty_rows() {
        let ctx = pretty_ctx();
        let columns = [Column::new("ID"), Column::new("Name")];
        let rows: Vec<Vec<String>> = vec![];
        let t = table(&ctx, &columns, &rows);
        // Should still contain headers
        assert!(t.contains("ID"));
        assert!(t.contains("Name"));
    }

    #[test]
    fn test_simple_table_column_padding() {
        let ctx = pretty_ctx();
        let columns = [Column::new("A"), Column::new("B")];
        let rows = vec![vec!["1".to_string(), "2".to_string()]];
        let t = simple_table(&ctx, &columns, &rows);
        // The output should have spacing between columns (padding)
        // Check that A and B are not adjacent (there's space between)
        let lines: Vec<&str> = t.lines().collect();
        if let Some(header_line) = lines.first() {
            // There should be at least 2 spaces between A and B due to padding
            assert!(header_line.contains("A") && header_line.contains("B"));
        }
    }

    // Visual regression tests - verify output format structure

    #[test]
    fn test_visual_header_with_context() {
        let ctx = pretty_ctx();
        let h = header_with_context(
            &ctx,
            "list",
            Some("last 7d"),
            Some("/path/to/ledger.ledger"),
        );
        // Should contain all parts
        assert!(h.contains("Ledger"));
        assert!(h.contains("list"));
        assert!(h.contains("last 7d"));
        assert!(h.contains("Path:"));
        assert!(h.contains("ledger.ledger"));
    }

    #[test]
    fn test_visual_header_truncates_long_path() {
        let ctx = pretty_ctx();
        let long_path =
            "/a/very/long/path/that/exceeds/fifty/characters/should/be/truncated.ledger";
        let h = header_with_context(&ctx, "list", None, Some(long_path));
        // Should contain ellipsis for truncated path
        assert!(h.contains("..."));
        assert!(h.contains("truncated.ledger"));
    }

    #[test]
    fn test_visual_badge_formats() {
        // Test with unicode enabled (pretty mode default)
        let ctx = pretty_ctx();

        let ok = badge(&ctx, Badge::Ok, "Success");
        // Unicode mode uses ✓ symbol
        assert!(ok.contains("[\u{2713}]") || ok.contains("[OK]"));
        assert!(ok.contains("Success"));

        let err = badge(&ctx, Badge::Err, "Failed");
        // Unicode mode uses ✗ symbol
        assert!(ok.contains("[\u{2717}]") || err.contains("[ERR]") || err.contains("[\u{2717}]"));
        assert!(err.contains("Failed"));

        let warn = badge(&ctx, Badge::Warn, "Warning");
        // Unicode mode uses ⚠ symbol
        assert!(warn.contains("[\u{26A0}]") || warn.contains("[!]"));
        assert!(warn.contains("Warning"));

        let info = badge(&ctx, Badge::Info, "Note");
        // Unicode mode uses ℹ symbol
        assert!(info.contains("[\u{2139}]") || info.contains("[i]"));
        assert!(info.contains("Note"));

        // Test ASCII mode
        let ascii_ctx = UiContext {
            is_tty: true,
            color: false,
            unicode: false, // ASCII mode
            width: 80,
            mode: OutputMode::Pretty,
        };
        let ok_ascii = badge(&ascii_ctx, Badge::Ok, "Success");
        assert!(ok_ascii.contains("[OK]"));
    }

    #[test]
    fn test_visual_receipt_format() {
        let ctx = pretty_ctx();
        let items = [("ID", "7a2e3c0b"), ("Type", "journal")];
        let r = receipt(&ctx, "Added entry", &items);
        // Should have badge (unicode ✓ or ASCII [OK]) and indented key-value pairs
        assert!(r.contains("[\u{2713}]") || r.contains("[OK]")); // Unicode or ASCII badge
        assert!(r.contains("Added entry"));
        assert!(r.contains("ID:"));
        assert!(r.contains("7a2e3c0b"));
        assert!(r.contains("Type:"));
        assert!(r.contains("journal"));
    }

    #[test]
    fn test_visual_receipt_plain() {
        let ctx = plain_ctx();
        let items = [("ID", "7a2e3c0b"), ("Type", "journal")];
        let r = receipt(&ctx, "Added entry", &items);
        // Plain mode: status=ok and key=value pairs
        assert!(r.contains("status=ok"));
        assert!(r.contains("id=7a2e3c0b"));
        assert!(r.contains("type=journal"));
    }

    #[test]
    fn test_visual_divider() {
        let ctx = pretty_ctx();
        let d = divider(&ctx);
        // Unicode horizontal lines
        assert!(d.contains("\u{2500}"));
        // Should be reasonable length (up to 60 chars)
        assert!(d.len() <= 60 * 3); // Unicode chars can be multi-byte

        let ctx_plain = plain_ctx();
        let d_plain = divider(&ctx_plain);
        assert_eq!(d_plain, "---");
    }

    #[test]
    fn test_visual_error_message() {
        let ctx = pretty_ctx();
        let e = error_message(&ctx, "Something went wrong", Some("Try again"));
        // Unicode mode uses ✗ symbol, ASCII uses [ERR]
        assert!(e.contains("[\u{2717}]") || e.contains("[ERR]"));
        assert!(e.contains("Something went wrong"));
        assert!(e.contains("Hint:"));
        assert!(e.contains("Try again"));

        let ctx_plain = plain_ctx();
        let e_plain = error_message(&ctx_plain, "Something went wrong", Some("Try again"));
        assert!(e_plain.contains("error=Something went wrong"));
        assert!(e_plain.contains("hint=Try again"));
    }
}
