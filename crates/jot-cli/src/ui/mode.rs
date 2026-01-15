//! Output mode routing logic.

/// Output mode determines how results are formatted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// Machine-readable JSON output only
    Json,
    /// Plain text, stable for logs and scripts
    #[default]
    Plain,
    /// Human-friendly with colors and formatting (TTY only)
    Pretty,
}

impl OutputMode {
    /// Resolve output mode from flags and environment.
    ///
    /// Routing rules:
    /// 1. `--json` overrides everything (exclusive mode)
    /// 2. `--format plain` forces plain
    /// 3. `TERM=dumb` forces plain
    /// 4. Pretty only when stdout is TTY
    /// 5. Default to plain for non-TTY
    pub fn resolve(
        json_flag: bool,
        format_flag: Option<&str>,
        is_tty: bool,
        term_is_dumb: bool,
    ) -> Self {
        // Rule 1: --json is exclusive
        if json_flag {
            return Self::Json;
        }

        // Rule 2: --format plain forces plain
        if let Some(fmt) = format_flag {
            if fmt == "plain" {
                return Self::Plain;
            }
        }

        // Rule 3: TERM=dumb forces plain
        if term_is_dumb {
            return Self::Plain;
        }

        // Rule 4 & 5: Pretty only on TTY
        if is_tty {
            Self::Pretty
        } else {
            Self::Plain
        }
    }

    /// Check if this mode should output JSON.
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }

    /// Check if this mode should output pretty (human) format.
    pub fn is_pretty(&self) -> bool {
        matches!(self, Self::Pretty)
    }

    /// Check if this mode should output plain text.
    #[allow(dead_code)]
    pub fn is_plain(&self) -> bool {
        matches!(self, Self::Plain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_exclusive() {
        // --json always wins, even with other flags
        let mode = OutputMode::resolve(true, Some("plain"), true, false);
        assert_eq!(mode, OutputMode::Json);
    }

    #[test]
    fn test_plain_forces() {
        let mode = OutputMode::resolve(false, Some("plain"), true, false);
        assert_eq!(mode, OutputMode::Plain);
    }

    #[test]
    fn test_term_dumb_forces_plain() {
        let mode = OutputMode::resolve(false, None, true, true);
        assert_eq!(mode, OutputMode::Plain);
    }

    #[test]
    fn test_tty_gets_pretty() {
        let mode = OutputMode::resolve(false, None, true, false);
        assert_eq!(mode, OutputMode::Pretty);
    }

    #[test]
    fn test_non_tty_gets_plain() {
        let mode = OutputMode::resolve(false, None, false, false);
        assert_eq!(mode, OutputMode::Plain);
    }

    #[test]
    fn test_table_format_on_tty_is_pretty() {
        // --format table on TTY should still be pretty
        let mode = OutputMode::resolve(false, Some("table"), true, false);
        assert_eq!(mode, OutputMode::Pretty);
    }
}
