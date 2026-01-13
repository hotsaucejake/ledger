//! Progress indicators for long-running operations using indicatif.

use indicatif::{ProgressBar as IndicatifBar, ProgressStyle};
use std::time::Duration;

use super::context::UiContext;
use super::render::badge;
use super::theme::Badge;

/// A spinner for indeterminate progress.
pub struct Spinner<'a> {
    ctx: &'a UiContext,
    bar: Option<IndicatifBar>,
}

impl<'a> Spinner<'a> {
    /// Create a new spinner with the given message.
    pub fn new(ctx: &'a UiContext, message: &str) -> Self {
        let bar = if ctx.allows_animation() {
            let pb = IndicatifBar::new_spinner();
            let template = if ctx.unicode {
                "{spinner:.cyan} {msg}..."
            } else {
                "{spinner} {msg}..."
            };
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template(template)
                    .unwrap()
                    .tick_strings(if ctx.unicode {
                        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", ""]
                    } else {
                        &["|", "/", "-", "\\", ""]
                    }),
            );
            pb.set_message(message.to_string());
            Some(pb)
        } else {
            None
        };

        Self { ctx, bar }
    }

    /// Start the spinner (prints initial line or begins animation).
    pub fn start(&self) {
        if let Some(bar) = &self.bar {
            bar.enable_steady_tick(Duration::from_millis(80));
        } else {
            // Non-TTY: print static message
            if let Some(bar) = &self.bar {
                println!("{}...", bar.message());
            }
        }
    }

    /// Update spinner with new message.
    pub fn update(&mut self, message: &str) {
        if let Some(bar) = &self.bar {
            bar.set_message(message.to_string());
        }
    }

    /// Advance to next frame (call this in a loop for animation).
    pub fn tick(&mut self) {
        if let Some(bar) = &self.bar {
            bar.tick();
        }
    }

    /// Finish spinner with success message.
    pub fn finish(&self, message: &str) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
        println!("{}", badge(self.ctx, Badge::Ok, message));
    }

    /// Finish spinner with error message.
    pub fn finish_err(&self, message: &str) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
        eprintln!("{}", badge(self.ctx, Badge::Err, message));
    }

    /// Finish spinner with warning message.
    pub fn finish_warn(&self, message: &str) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
        println!("{}", badge(self.ctx, Badge::Warn, message));
    }
}

/// A progress bar for determinate progress.
pub struct ProgressBar<'a> {
    ctx: &'a UiContext,
    bar: Option<IndicatifBar>,
    total: u64,
    current: u64,
}

impl<'a> ProgressBar<'a> {
    /// Create a new progress bar.
    pub fn new(ctx: &'a UiContext, total: u64, message: &str) -> Self {
        let bar = if ctx.allows_animation() {
            let pb = IndicatifBar::new(total);
            let template = if ctx.unicode {
                "{msg} [{bar:20.cyan/dim}] {percent}%"
            } else {
                "{msg} [{bar:20}] {percent}%"
            };
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(template)
                    .unwrap()
                    .progress_chars(if ctx.unicode { "━━─" } else { "=>-" }),
            );
            pb.set_message(message.to_string());
            Some(pb)
        } else {
            None
        };

        Self {
            ctx,
            bar,
            total,
            current: 0,
        }
    }

    /// Set the bar width (default is 20). Note: with indicatif, width is set in template.
    pub fn with_width(self, _width: usize) -> Self {
        // Width is controlled by the template in indicatif
        // This method is kept for API compatibility
        self
    }

    /// Set current progress value.
    pub fn set(&mut self, current: u64) {
        self.current = current.min(self.total);
        if let Some(bar) = &self.bar {
            bar.set_position(self.current);
        }
    }

    /// Increment progress by amount.
    pub fn inc(&mut self, amount: u64) {
        self.current = self.current.saturating_add(amount).min(self.total);
        if let Some(bar) = &self.bar {
            bar.inc(amount);
        }
    }

    /// Get current percentage.
    pub fn percent(&self) -> u8 {
        if self.total > 0 {
            ((self.current as f64 / self.total as f64) * 100.0) as u8
        } else {
            0
        }
    }

    /// Finish progress bar (clears the line).
    pub fn finish(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
    }

    /// Finish progress bar with a message.
    pub fn finish_with_message(&self, message: &str) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
        println!("{}", badge(self.ctx, Badge::Ok, message));
    }
}

/// A step list that shows progress through a series of steps.
pub struct StepList<'a> {
    ctx: &'a UiContext,
    steps: Vec<(String, Option<Badge>)>,
    current: usize,
}

impl<'a> StepList<'a> {
    /// Create a new step list with the given step names.
    pub fn new(ctx: &'a UiContext, steps: &[&str]) -> Self {
        Self {
            ctx,
            steps: steps.iter().map(|s| (s.to_string(), None)).collect(),
            current: 0,
        }
    }

    /// Start the step list (renders "Checking..." header in pretty mode).
    pub fn start(&self, header: &str) {
        if self.ctx.mode.is_pretty() {
            println!("{}...", header);
        }
    }

    /// Mark current step as complete with given badge and advance.
    pub fn complete(&mut self, result: Badge) {
        if self.current < self.steps.len() {
            self.steps[self.current].1 = Some(result);
            self.render_step(self.current);
            self.current += 1;
        }
    }

    /// Mark current step as OK and advance.
    pub fn ok(&mut self) {
        self.complete(Badge::Ok);
    }

    /// Mark current step as warning and advance.
    pub fn warn(&mut self) {
        self.complete(Badge::Warn);
    }

    /// Mark current step as error and advance.
    pub fn err(&mut self) {
        self.complete(Badge::Err);
    }

    /// Render a single step.
    fn render_step(&self, index: usize) {
        let (name, result) = &self.steps[index];
        let status = match result {
            Some(b) => badge(self.ctx, *b, ""),
            None => "...".to_string(),
        };

        if self.ctx.mode.is_pretty() {
            println!("- {}: {}", name, status);
        } else {
            let status_str = match result {
                Some(Badge::Ok) => "ok",
                Some(Badge::Warn) => "warn",
                Some(Badge::Err) => "err",
                Some(Badge::Info) => "info",
                None => "pending",
            };
            println!(
                "check={} {}",
                name.to_lowercase().replace(' ', "_"),
                status_str
            );
        }
    }

    /// Check if all steps completed successfully (all OK).
    pub fn all_ok(&self) -> bool {
        self.steps
            .iter()
            .all(|(_, result)| *result == Some(Badge::Ok))
    }

    /// Check if any step had an error.
    pub fn has_error(&self) -> bool {
        self.steps
            .iter()
            .any(|(_, result)| *result == Some(Badge::Err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::mode::OutputMode;

    fn test_ctx(animated: bool) -> UiContext {
        UiContext {
            is_tty: animated,
            color: false,
            unicode: true,
            width: 80,
            mode: if animated {
                OutputMode::Pretty
            } else {
                OutputMode::Plain
            },
        }
    }

    #[test]
    fn test_progress_bar_percent() {
        let ctx = test_ctx(false);
        let mut bar = ProgressBar::new(&ctx, 100, "Test");
        assert_eq!(bar.percent(), 0);
        bar.set(50);
        assert_eq!(bar.percent(), 50);
        bar.set(100);
        assert_eq!(bar.percent(), 100);
    }

    #[test]
    fn test_progress_bar_inc() {
        let ctx = test_ctx(false);
        let mut bar = ProgressBar::new(&ctx, 100, "Test");
        bar.inc(25);
        assert_eq!(bar.current, 25);
        bar.inc(25);
        assert_eq!(bar.current, 50);
    }

    #[test]
    fn test_step_list_tracking() {
        let ctx = test_ctx(false);
        let mut steps = StepList::new(&ctx, &["Step 1", "Step 2"]);
        assert!(!steps.all_ok());
        steps.ok();
        steps.ok();
        assert!(steps.all_ok());
    }

    #[test]
    fn test_step_list_error_detection() {
        let ctx = test_ctx(false);
        let mut steps = StepList::new(&ctx, &["Step 1", "Step 2"]);
        steps.ok();
        steps.err();
        assert!(steps.has_error());
        assert!(!steps.all_ok());
    }
}
