//! Theme definitions for colors, symbols, and badges.

use owo_colors::{OwoColorize, Style};

/// Symbol pair for ASCII and Unicode variants.
#[derive(Debug, Clone)]
pub struct SymbolPair {
    pub ascii: &'static str,
    pub unicode: &'static str,
}

impl SymbolPair {
    pub const fn new(ascii: &'static str, unicode: &'static str) -> Self {
        Self { ascii, unicode }
    }

    /// Get the appropriate symbol based on unicode flag.
    pub fn get(&self, unicode: bool) -> &'static str {
        if unicode {
            self.unicode
        } else {
            self.ascii
        }
    }
}

/// Badge types for status indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Ok,
    Warn,
    Err,
    Info,
}

impl Badge {
    /// Get badge text (e.g., "[OK]")
    pub fn text(&self) -> &'static str {
        match self {
            Self::Ok => "[OK]",
            Self::Warn => "[WARN]",
            Self::Err => "[ERR]",
            Self::Info => "[INFO]",
        }
    }

    /// Get badge with symbol for display.
    pub fn display(&self, unicode: bool) -> &'static str {
        match self {
            Self::Ok => {
                if unicode {
                    "[\u{2713}]" // [✓]
                } else {
                    "[OK]"
                }
            }
            Self::Warn => {
                if unicode {
                    "[\u{26A0}]" // [⚠]
                } else {
                    "[WARN]"
                }
            }
            Self::Err => {
                if unicode {
                    "[\u{2717}]" // [✗]
                } else {
                    "[ERR]"
                }
            }
            Self::Info => {
                if unicode {
                    "[\u{2139}]" // [ℹ]
                } else {
                    "[INFO]"
                }
            }
        }
    }

    /// Get the owo-colors style for this badge type.
    pub fn style(&self) -> Style {
        match self {
            Self::Ok => Style::new().green(),
            Self::Warn => Style::new().yellow(),
            Self::Err => Style::new().red(),
            Self::Info => Style::new().cyan(),
        }
    }
}

/// Style helpers using owo-colors.
pub mod styles {
    use owo_colors::Style;

    /// Dim text style (for labels, metadata)
    pub fn dim() -> Style {
        Style::new().dimmed()
    }

    /// Bold text style (for emphasis)
    pub fn bold() -> Style {
        Style::new().bold()
    }

    /// Success style (green)
    pub fn success() -> Style {
        Style::new().green()
    }

    /// Warning style (yellow)
    pub fn warning() -> Style {
        Style::new().yellow()
    }

    /// Error style (red)
    pub fn error() -> Style {
        Style::new().red()
    }

    /// Info style (cyan)
    pub fn info() -> Style {
        Style::new().cyan()
    }
}

/// Apply a style to text, returning a styled string.
pub fn styled(text: &str, style: Style, color_enabled: bool) -> String {
    if color_enabled {
        text.style(style).to_string()
    } else {
        text.to_string()
    }
}

/// Theme configuration for UI rendering.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Spinner frames for unicode mode
    pub spinner_unicode: &'static [&'static str],
    /// Spinner frames for ASCII mode
    pub spinner_ascii: &'static [&'static str],
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Braille spinner (smooth rotation)
            spinner_unicode: &[
                "\u{280B}", // ⠋
                "\u{2819}", // ⠙
                "\u{2839}", // ⠹
                "\u{2838}", // ⠸
                "\u{283C}", // ⠼
                "\u{2834}", // ⠴
                "\u{2826}", // ⠦
                "\u{2827}", // ⠧
                "\u{2807}", // ⠇
                "\u{280F}", // ⠏
            ],
            // Classic ASCII spinner
            spinner_ascii: &["|", "/", "-", "\\"],
        }
    }
}

impl Theme {
    /// Get spinner frames based on unicode setting.
    pub fn spinner_frames(&self, unicode: bool) -> &'static [&'static str] {
        if unicode {
            self.spinner_unicode
        } else {
            self.spinner_ascii
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_text() {
        assert_eq!(Badge::Ok.text(), "[OK]");
        assert_eq!(Badge::Warn.text(), "[WARN]");
        assert_eq!(Badge::Err.text(), "[ERR]");
        assert_eq!(Badge::Info.text(), "[INFO]");
    }

    #[test]
    fn test_badge_display_ascii() {
        assert_eq!(Badge::Ok.display(false), "[OK]");
    }

    #[test]
    fn test_badge_display_unicode() {
        assert_eq!(Badge::Ok.display(true), "[\u{2713}]");
    }

    #[test]
    fn test_symbol_pair() {
        let pair = SymbolPair::new("*", "\u{2022}");
        assert_eq!(pair.get(false), "*");
        assert_eq!(pair.get(true), "\u{2022}");
    }

    #[test]
    fn test_theme_spinner_frames() {
        let theme = Theme::default();
        assert_eq!(theme.spinner_frames(false).len(), 4);
        assert_eq!(theme.spinner_frames(true).len(), 10);
    }

    #[test]
    fn test_styled_with_color() {
        let text = styled("hello", styles::success(), true);
        assert!(text.contains("hello"));
    }

    #[test]
    fn test_styled_without_color() {
        let text = styled("hello", styles::success(), false);
        assert_eq!(text, "hello");
    }
}
