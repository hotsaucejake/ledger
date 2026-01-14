//! UI context for environment detection and configuration.

use std::io::IsTerminal;

use super::mode::OutputMode;

/// Terminal and environment context for UI decisions.
#[derive(Debug, Clone)]
pub struct UiContext {
    /// Whether stdout is a TTY
    #[allow(dead_code)]
    pub is_tty: bool,
    /// Whether color output is enabled
    pub color: bool,
    /// Whether unicode symbols are enabled
    pub unicode: bool,
    /// Terminal width (columns)
    pub width: usize,
    /// Resolved output mode
    pub mode: OutputMode,
}

impl UiContext {
    /// Create context from environment and CLI flags.
    ///
    /// # Arguments
    /// * `json_flag` - Whether `--json` was passed
    /// * `format_flag` - Value of `--format` if provided
    /// * `no_color_flag` - Whether `--no-color` was passed
    /// * `ascii_flag` - Whether `--ascii` was passed
    pub fn from_env(
        json_flag: bool,
        format_flag: Option<&str>,
        no_color_flag: bool,
        ascii_flag: bool,
    ) -> Self {
        let is_tty = std::io::stdout().is_terminal();
        let term_is_dumb = std::env::var("TERM").map(|v| v == "dumb").unwrap_or(false);
        let no_color_env = std::env::var("NO_COLOR").is_ok();

        // Resolve color: disabled if NO_COLOR env, --no-color flag, or TERM=dumb
        let color = is_tty && !no_color_flag && !no_color_env && !term_is_dumb;

        // Resolve unicode: disabled if --ascii flag
        let unicode = !ascii_flag;

        // Resolve terminal width
        let width = terminal_width().unwrap_or(80);

        // Resolve output mode
        let mode = OutputMode::resolve(json_flag, format_flag, is_tty, term_is_dumb);

        Self {
            is_tty,
            color,
            unicode,
            width,
            mode,
        }
    }

    /// Check if interactive prompts are allowed.
    #[allow(dead_code)]
    pub fn is_interactive(&self) -> bool {
        self.is_tty && std::io::stdin().is_terminal()
    }

    /// Check if animations (spinners, progress) are allowed.
    #[allow(dead_code)]
    pub fn allows_animation(&self) -> bool {
        self.is_tty && self.mode == OutputMode::Pretty
    }
}

/// Get terminal width, falling back to 80.
fn terminal_width() -> Option<usize> {
    // First try COLUMNS environment variable
    if let Ok(cols) = std::env::var("COLUMNS") {
        if let Ok(width) = cols.parse::<usize>() {
            if width > 0 {
                return Some(width);
            }
        }
    }

    // Try platform-specific detection
    #[cfg(unix)]
    {
        use std::mem::MaybeUninit;

        let mut winsize = MaybeUninit::<libc::winsize>::uninit();
        // SAFETY: ioctl with TIOCGWINSZ is safe and winsize is properly initialized
        let result =
            unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, winsize.as_mut_ptr()) };
        if result == 0 {
            let ws = unsafe { winsize.assume_init() };
            if ws.ws_col > 0 {
                return Some(ws.ws_col as usize);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_mode_from_flag() {
        let ctx = UiContext::from_env(true, None, false, false);
        assert_eq!(ctx.mode, OutputMode::Json);
    }

    #[test]
    fn test_ascii_disables_unicode() {
        let ctx = UiContext::from_env(false, None, false, true);
        assert!(!ctx.unicode);
    }

    #[test]
    fn test_no_color_disables_color() {
        let ctx = UiContext::from_env(false, None, true, false);
        assert!(!ctx.color);
    }

    #[test]
    fn test_width_has_default() {
        let ctx = UiContext::from_env(false, None, false, false);
        assert!(ctx.width > 0);
    }
}
