//! Wizard and guided prompt primitives for interactive CLI flows.
//!
//! This module provides step-based wizard flows with consistent styling,
//! progress indicators, and review screens.

use std::io::IsTerminal;

use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};

use super::context::UiContext;
use super::render::{badge, blank_line, divider, hint, kv, print};
use super::theme::{styled, styles, Badge};

/// A wizard step with a title and optional description.
#[derive(Debug, Clone)]
pub struct WizardStep {
    pub title: String,
    pub description: Option<String>,
}

impl WizardStep {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            description: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }
}

/// Wizard context for managing multi-step flows.
pub struct Wizard<'a> {
    ctx: &'a UiContext,
    title: String,
    steps: Vec<WizardStep>,
    current_step: usize,
}

impl<'a> Wizard<'a> {
    /// Create a new wizard with a title and steps.
    pub fn new(ctx: &'a UiContext, title: &str, steps: Vec<WizardStep>) -> Self {
        Self {
            ctx,
            title: title.to_string(),
            steps,
            current_step: 0,
        }
    }

    /// Check if we're in interactive mode.
    pub fn is_interactive(&self) -> bool {
        std::io::stdin().is_terminal() && self.ctx.is_tty
    }

    /// Print the wizard header.
    pub fn print_header(&self) {
        if !self.ctx.mode.is_pretty() {
            return;
        }

        let header = styled("Jot", styles::bold(), self.ctx.color);
        println!("{} \u{00B7} {}", header, self.title);
        println!();
    }

    /// Print the current step indicator.
    pub fn print_step(&self) {
        if !self.ctx.mode.is_pretty() {
            return;
        }

        let step = &self.steps[self.current_step];
        let progress = format!("{}/{}", self.current_step + 1, self.steps.len());
        let progress_styled = styled(&progress, styles::dim(), self.ctx.color);

        println!(
            "{}  {}",
            progress_styled,
            styled(&step.title, styles::bold(), self.ctx.color)
        );

        if let Some(ref desc) = step.description {
            println!("   {}", styled(desc, styles::dim(), self.ctx.color));
        }
    }

    /// Advance to the next step.
    pub fn next_step(&mut self) {
        if self.current_step < self.steps.len() - 1 {
            self.current_step += 1;
            if self.ctx.mode.is_pretty() {
                println!();
            }
        }
    }

    /// Get the current step number (1-indexed for display).
    pub fn current_step_number(&self) -> usize {
        self.current_step + 1
    }

    /// Get total number of steps.
    pub fn total_steps(&self) -> usize {
        self.steps.len()
    }

    /// Show a review screen with key-value pairs.
    pub fn print_review(&self, items: &[(&str, &str)]) {
        if !self.ctx.mode.is_pretty() {
            return;
        }

        let step = &self.steps[self.current_step];
        let progress = format!("{}/{}", self.current_step + 1, self.steps.len());
        let progress_styled = styled(&progress, styles::dim(), self.ctx.color);

        println!(
            "{}  {}",
            progress_styled,
            styled(&step.title, styles::bold(), self.ctx.color)
        );

        for (key, value) in items {
            println!("   {}", kv(self.ctx, key, value));
        }
    }

    /// Show a success receipt.
    pub fn print_receipt(&self, message: &str, items: &[(&str, &str)]) {
        if self.ctx.mode.is_pretty() {
            println!();
            print(self.ctx, &badge(self.ctx, Badge::Ok, message));
            for (key, value) in items {
                println!("  {}", kv(self.ctx, key, value));
            }
        } else {
            println!("status=ok");
            for (key, value) in items {
                println!("{}={}", key.to_lowercase().replace(' ', "_"), value);
            }
        }
    }

    /// Show next-step hints.
    pub fn print_hints(&self, hints: &[&str]) {
        if !self.ctx.mode.is_pretty() {
            return;
        }

        println!();
        let joined = hints.join("  \u{00B7}  ");
        print(self.ctx, &hint(self.ctx, &joined));
    }
}

/// Prompt for text input with styled formatting.
pub fn prompt_input(
    _ctx: &UiContext,
    prompt: &str,
    default: Option<&str>,
) -> anyhow::Result<String> {
    if !std::io::stdin().is_terminal() {
        return Err(anyhow::anyhow!(
            "Interactive input required. Use flags or run on a TTY."
        ));
    }

    let theme = ColorfulTheme::default();
    let builder = Input::<String>::with_theme(&theme).with_prompt(prompt);

    let result = if let Some(def) = default {
        builder.default(def.to_string()).interact_text()?
    } else {
        builder.interact_text()?
    };

    Ok(result)
}

/// Prompt for password input with confirmation.
pub fn prompt_passphrase(_ctx: &UiContext, confirm: bool) -> anyhow::Result<String> {
    if !std::io::stdin().is_terminal() {
        return Err(anyhow::anyhow!(
            "Interactive passphrase input required. Set JOT_PASSPHRASE or run on a TTY."
        ));
    }

    let theme = ColorfulTheme::default();
    let builder = Password::with_theme(&theme).with_prompt("Passphrase");

    let result = if confirm {
        builder
            .with_confirmation("Confirm passphrase", "Passphrases do not match")
            .interact()?
    } else {
        builder.interact()?
    };

    Ok(result)
}

/// Prompt for selection from a list of options.
pub fn prompt_select(
    _ctx: &UiContext,
    prompt: &str,
    options: &[&str],
    default: usize,
) -> anyhow::Result<usize> {
    if !std::io::stdin().is_terminal() {
        return Err(anyhow::anyhow!(
            "Interactive selection required. Use flags or run on a TTY."
        ));
    }

    let theme = ColorfulTheme::default();
    let result = Select::with_theme(&theme)
        .with_prompt(prompt)
        .items(options)
        .default(default)
        .interact()?;

    Ok(result)
}

/// Prompt for confirmation.
pub fn prompt_confirm(_ctx: &UiContext, prompt: &str, default: bool) -> anyhow::Result<bool> {
    if !std::io::stdin().is_terminal() {
        return Err(anyhow::anyhow!(
            "Interactive confirmation required. Use flags or run on a TTY."
        ));
    }

    let theme = ColorfulTheme::default();
    let result = Confirm::with_theme(&theme)
        .with_prompt(prompt)
        .default(default)
        .interact()?;

    Ok(result)
}

/// Result of an init wizard.
#[derive(Debug)]
pub struct InitWizardResult {
    pub jot_path: std::path::PathBuf,
    pub config_path: std::path::PathBuf,
    pub passphrase: String,
    pub security_tier: usize, // 0=passphrase, 1=keychain, 2=keyfile, 3=device
    pub cache_ttl: u64,
    pub keyfile_path: Option<std::path::PathBuf>,
    pub timezone: Option<String>,
    pub editor: Option<String>,
}

/// Run the init wizard flow.
pub fn init_wizard(
    ctx: &UiContext,
    default_jot_path: &std::path::Path,
    default_config_path: &std::path::Path,
    default_keyfile_path: &std::path::Path,
    advanced: bool,
) -> anyhow::Result<InitWizardResult> {
    let steps = if advanced {
        vec![
            WizardStep::new("Choose location"),
            WizardStep::new("Create passphrase"),
            WizardStep::new("Security level"),
            WizardStep::new("Advanced settings"),
            WizardStep::new("Review"),
        ]
    } else {
        vec![
            WizardStep::new("Choose location"),
            WizardStep::new("Create passphrase"),
            WizardStep::new("Security level"),
            WizardStep::new("Review"),
        ]
    };

    let mut wizard = Wizard::new(ctx, "init", steps);
    wizard.print_header();

    // Step 1: Path
    wizard.print_step();
    let jot_path = std::path::PathBuf::from(prompt_input(
        ctx,
        "Jot file location",
        Some(&default_jot_path.to_string_lossy()),
    )?);
    wizard.next_step();

    // Step 2: Passphrase
    wizard.print_step();
    let passphrase = prompt_passphrase(ctx, true)?;
    wizard.next_step();

    // Step 3: Security tier
    wizard.print_step();
    let security_options = [
        "Passphrase only (recommended)",
        "Passphrase + OS keychain",
        "Passphrase + encrypted keyfile",
        "Device keyfile only (reduced security)",
    ];
    let security_tier = prompt_select(ctx, "Security level", &security_options, 0)?;
    wizard.next_step();

    // Advanced settings (if enabled)
    let mut cache_ttl: u64 = 0;
    let mut keyfile_path: Option<std::path::PathBuf> = None;
    let mut timezone: Option<String> = None;
    let mut editor: Option<String> = None;
    let mut config_path = default_config_path.to_path_buf();

    if advanced {
        wizard.print_step();

        // Timezone
        let tz_input = prompt_input(ctx, "Timezone", Some("auto"))?;
        if !tz_input.trim().is_empty() && !tz_input.trim().eq_ignore_ascii_case("auto") {
            timezone = Some(tz_input);
        }

        // Editor
        let default_editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        let editor_input = prompt_input(ctx, "Default editor", Some(&default_editor))?;
        if !editor_input.trim().is_empty() {
            editor = Some(editor_input);
        }

        // Cache TTL
        let ttl_input = prompt_input(ctx, "Passphrase cache (seconds)", Some("0"))?;
        cache_ttl = ttl_input
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid cache TTL: expected integer seconds"))?;

        // Keyfile path (if needed)
        if security_tier == 2 || security_tier == 3 {
            let kf_input = prompt_input(
                ctx,
                "Keyfile path",
                Some(&default_keyfile_path.to_string_lossy()),
            )?;
            keyfile_path = Some(std::path::PathBuf::from(kf_input));
        }

        // Config path
        let cfg_input = prompt_input(
            ctx,
            "Config path",
            Some(&default_config_path.to_string_lossy()),
        )?;
        config_path = std::path::PathBuf::from(cfg_input);

        wizard.next_step();
    } else if security_tier == 2 || security_tier == 3 {
        keyfile_path = Some(default_keyfile_path.to_path_buf());
    }

    // Review step
    let tier_name = match security_tier {
        0 => "Passphrase only",
        1 => "Passphrase + keychain",
        2 => "Passphrase + keyfile",
        3 => "Device keyfile",
        _ => "Unknown",
    };

    let mut review_items: Vec<(&str, String)> = vec![
        ("Path", jot_path.to_string_lossy().to_string()),
        ("Security", tier_name.to_string()),
    ];

    if cache_ttl > 0 {
        review_items.push(("Cache TTL", format!("{}s", cache_ttl)));
    }

    if let Some(ref kf) = keyfile_path {
        review_items.push(("Keyfile", kf.to_string_lossy().to_string()));
    }

    if let Some(ref tz) = timezone {
        review_items.push(("Timezone", tz.clone()));
    }

    if let Some(ref ed) = editor {
        review_items.push(("Editor", ed.clone()));
    }

    let review_refs: Vec<(&str, &str)> =
        review_items.iter().map(|(k, v)| (*k, v.as_str())).collect();
    wizard.print_review(&review_refs);

    if ctx.mode.is_pretty() {
        println!();
    }

    let proceed = prompt_confirm(ctx, "Create jot with these settings?", true)?;
    if !proceed {
        return Err(anyhow::anyhow!("Initialization cancelled"));
    }

    Ok(InitWizardResult {
        jot_path,
        config_path,
        passphrase,
        security_tier,
        cache_ttl,
        keyfile_path,
        timezone,
        editor,
    })
}

/// Template selection for add wizard.
#[derive(Debug, Clone)]
pub struct TemplateChoice {
    pub id: Option<String>,
    pub name: String,
    pub is_default: bool,
}

/// Result of an add wizard.
#[derive(Debug)]
pub struct AddWizardResult {
    pub template_id: Option<String>,
    pub fields: std::collections::HashMap<String, String>,
    pub tags: Vec<String>,
}

/// Field definition for add wizard prompting.
#[derive(Debug, Clone)]
pub struct AddWizardField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub default: Option<String>,
}

/// Run the add wizard flow for entry creation.
pub fn add_wizard(
    ctx: &UiContext,
    entry_type: &str,
    templates: &[TemplateChoice],
    fields: &[AddWizardField],
    editor_cmd: Option<&str>,
) -> anyhow::Result<AddWizardResult> {
    let has_templates = !templates.is_empty();

    let steps: Vec<WizardStep> = if has_templates {
        vec![
            WizardStep::new("Template"),
            WizardStep::new("Fields"),
            WizardStep::new("Body"),
            WizardStep::new("Review"),
        ]
    } else {
        vec![
            WizardStep::new("Fields"),
            WizardStep::new("Body"),
            WizardStep::new("Review"),
        ]
    };

    let mut wizard = Wizard::new(ctx, &format!("add ({})", entry_type), steps);
    wizard.print_header();

    // Template selection (if templates available)
    let mut selected_template: Option<String> = None;
    let template_defaults: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    if has_templates {
        wizard.print_step();

        let mut options: Vec<String> = templates
            .iter()
            .map(|t| {
                if t.is_default {
                    format!("{} (default)", t.name)
                } else {
                    t.name.clone()
                }
            })
            .collect();
        options.push("blank".to_string());

        let options_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
        let default_idx = templates.iter().position(|t| t.is_default).unwrap_or(0);
        let choice = prompt_select(ctx, "Template", &options_refs, default_idx)?;

        if choice < templates.len() {
            selected_template = templates[choice].id.clone();
        }

        wizard.next_step();
    }

    // Fields
    wizard.print_step();
    let mut field_values: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for field in fields {
        if field.name == "body" {
            continue; // Body is handled separately
        }

        let default = field
            .default
            .as_deref()
            .or_else(|| template_defaults.get(&field.name).map(|s| s.as_str()));

        let prompt_text = if field.required {
            format!("{} (required)", field.name)
        } else {
            field.name.clone()
        };

        let value = prompt_input(ctx, &prompt_text, default)?;
        if !value.is_empty() {
            field_values.insert(field.name.clone(), value);
        } else if field.required {
            return Err(anyhow::anyhow!("Field '{}' is required", field.name));
        }
    }
    wizard.next_step();

    // Body (using editor)
    wizard.print_step();
    let body = if let Some(editor) = editor_cmd {
        println!("Opening {}...", editor);
        // In a real implementation, this would open the editor
        // For now, fall back to inline input
        prompt_input(ctx, "Body", None)?
    } else {
        prompt_input(ctx, "Body", None)?
    };
    field_values.insert("body".to_string(), body);
    wizard.next_step();

    // Tags (optional)
    let tags_input = prompt_input(ctx, "Tags (comma-separated, optional)", Some(""))?;
    let tags: Vec<String> = if tags_input.is_empty() {
        Vec::new()
    } else {
        tags_input
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };

    // Review
    let mut review_items: Vec<(&str, String)> = Vec::new();
    review_items.push(("Type", entry_type.to_string()));

    if let Some(ref tmpl) = selected_template {
        review_items.push(("Template", tmpl.clone()));
    }

    for (key, value) in &field_values {
        if key == "body" {
            let preview = if value.len() > 40 {
                format!("{}...", &value[..40])
            } else {
                value.clone()
            };
            review_items.push(("Body", preview));
        } else {
            // Convert key to static str for review display
            review_items.push(("Field", format!("{}: {}", key, value)));
        }
    }

    if !tags.is_empty() {
        review_items.push(("Tags", tags.join(", ")));
    }

    // Build review refs with proper lifetimes
    let review_display: Vec<(String, String)> = review_items
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();

    if ctx.mode.is_pretty() {
        let progress = format!("{}/{}", wizard.current_step_number(), wizard.total_steps());
        let progress_styled = styled(&progress, styles::dim(), ctx.color);
        println!(
            "{}  {}",
            progress_styled,
            styled("Review", styles::bold(), ctx.color)
        );

        for (key, value) in &review_display {
            println!("   {}: {}", key, value);
        }
        println!();
    }

    let proceed = prompt_confirm(ctx, "Create entry?", true)?;
    if !proceed {
        return Err(anyhow::anyhow!("Entry creation cancelled"));
    }

    Ok(AddWizardResult {
        template_id: selected_template,
        fields: field_values,
        tags,
    })
}

/// Print a cancellation message.
pub fn print_cancelled(ctx: &UiContext, action: &str) {
    if ctx.mode.is_pretty() {
        blank_line(ctx);
        print(
            ctx,
            &badge(ctx, Badge::Warn, &format!("{} cancelled", action)),
        );
    } else {
        println!("status=cancelled");
    }
}

/// Print a divider for visual separation.
pub fn print_divider(ctx: &UiContext) {
    if ctx.mode.is_pretty() {
        print(ctx, &divider(ctx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::mode::OutputMode;

    fn test_ctx() -> UiContext {
        UiContext {
            is_tty: false,
            color: false,
            unicode: true,
            width: 80,
            mode: OutputMode::Plain,
        }
    }

    #[test]
    fn test_wizard_step_creation() {
        let step = WizardStep::new("Test Step");
        assert_eq!(step.title, "Test Step");
        assert!(step.description.is_none());

        let step_with_desc = WizardStep::new("Test").with_description("A description");
        assert_eq!(
            step_with_desc.description,
            Some("A description".to_string())
        );
    }

    #[test]
    fn test_wizard_creation() {
        let ctx = test_ctx();
        let steps = vec![WizardStep::new("Step 1"), WizardStep::new("Step 2")];
        let wizard = Wizard::new(&ctx, "test", steps);

        assert_eq!(wizard.current_step_number(), 1);
        assert_eq!(wizard.total_steps(), 2);
    }

    #[test]
    fn test_template_choice() {
        let choice = TemplateChoice {
            id: Some("abc123".to_string()),
            name: "Morning Journal".to_string(),
            is_default: true,
        };
        assert!(choice.is_default);
        assert_eq!(choice.name, "Morning Journal");
    }

    #[test]
    fn test_add_wizard_field() {
        let field = AddWizardField {
            name: "title".to_string(),
            field_type: "text".to_string(),
            required: true,
            default: Some("Default Title".to_string()),
        };
        assert!(field.required);
        assert_eq!(field.default, Some("Default Title".to_string()));
    }
}
