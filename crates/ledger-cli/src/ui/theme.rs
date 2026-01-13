//! Theme definitions for colors, symbols, and badges.

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
}

/// Color definitions using ANSI escape codes.
pub mod colors {
    /// Dim text (for labels, metadata)
    pub const DIM: &str = "\x1b[2m";
    /// Bright/bold text (for values)
    pub const BRIGHT: &str = "\x1b[1m";
    /// Green (success)
    pub const GREEN: &str = "\x1b[32m";
    /// Yellow (warning)
    pub const YELLOW: &str = "\x1b[33m";
    /// Red (error)
    pub const RED: &str = "\x1b[31m";
    /// Cyan (info)
    pub const CYAN: &str = "\x1b[36m";
    /// Reset all styles
    pub const RESET: &str = "\x1b[0m";
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
}
