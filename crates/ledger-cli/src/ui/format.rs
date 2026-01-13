//! String formatting utilities for UI rendering.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Truncate a string to max length, adding ellipsis if needed.
pub fn truncate(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        return s.to_string();
    }
    if max_len <= 3 {
        return s.chars().take(max_len).collect();
    }
    let truncated: String = s.chars().take(max_len - 3).collect();
    format!("{}...", truncated)
}

/// Pad a string to a fixed width (left-aligned).
pub fn pad_right(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - char_count))
    }
}

/// Pad a string to a fixed width (right-aligned).
pub fn pad_left(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count >= width {
        s.to_string()
    } else {
        format!("{}{}", " ".repeat(width - char_count), s)
    }
}

/// Wrap text to a given width, preserving newlines.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current_line = String::new();
        for word in paragraph.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.chars().count() + 1 + word.chars().count() <= width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    lines
}

/// Format a short ID from a UUID (first 8 characters).
pub fn short_id(id: &Uuid) -> String {
    id.to_string()[..8].to_string()
}

/// Format a datetime for display.
pub fn format_datetime(dt: &DateTime<Utc>, pretty: bool) -> String {
    if pretty {
        dt.format("%Y-%m-%d %H:%M UTC").to_string()
    } else {
        dt.to_rfc3339()
    }
}

/// Format bytes as human-readable size.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration in seconds as human-readable string.
pub fn format_duration_secs(secs: f64) -> String {
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{:.1}s", secs)
    } else {
        let mins = (secs / 60.0).floor() as u64;
        let remaining = secs - (mins as f64 * 60.0);
        format!("{}m {:.0}s", mins, remaining)
    }
}

/// Sanitize a string for single-line output (replace newlines with spaces).
pub fn single_line(s: &str) -> String {
    s.replace('\n', " ").replace('\r', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_very_short_max() {
        assert_eq!(truncate("hello", 2), "he");
    }

    #[test]
    fn test_pad_right() {
        assert_eq!(pad_right("hi", 5), "hi   ");
        assert_eq!(pad_right("hello", 3), "hello");
    }

    #[test]
    fn test_pad_left() {
        assert_eq!(pad_left("42", 5), "   42");
        assert_eq!(pad_left("hello", 3), "hello");
    }

    #[test]
    fn test_wrap_simple() {
        let lines = wrap("hello world foo bar", 10);
        assert_eq!(lines, vec!["hello", "world foo", "bar"]);
    }

    #[test]
    fn test_wrap_preserves_newlines() {
        let lines = wrap("hello\n\nworld", 20);
        assert_eq!(lines, vec!["hello", "", "world"]);
    }

    #[test]
    fn test_short_id() {
        let id = Uuid::parse_str("7a2e3c0b-1234-5678-9abc-def012345678").unwrap();
        assert_eq!(short_id(&id), "7a2e3c0b");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_format_duration_secs() {
        assert_eq!(format_duration_secs(0.5), "500ms");
        assert_eq!(format_duration_secs(2.5), "2.5s");
        assert_eq!(format_duration_secs(90.0), "1m 30s");
    }

    #[test]
    fn test_single_line() {
        assert_eq!(single_line("hello\nworld"), "hello world");
        assert_eq!(single_line("no newlines"), "no newlines");
    }
}
