use std::io::IsTerminal;

use dialoguer::{theme::ColorfulTheme, Completion, Confirm, FuzzySelect, Input, Select};
use ledger_core::storage::{AgeSqliteStorage, StorageEngine};
use ledger_core::VERSION;

use crate::app::{device_keyfile_warning, resolve_config_path, AppContext};
use crate::cache::ledger_hash;
use crate::cli::InitArgs;
use crate::config::{
    default_keyfile_path, default_ledger_path, write_config, KeyfileMode, LedgerConfig,
    SecurityTier,
};
use crate::helpers::prompt_init_passphrase;
use crate::security::{
    generate_key_bytes, key_bytes_to_passphrase, keychain_set, write_keyfile_encrypted,
    write_keyfile_plain,
};
use crate::ui::theme::{styled, styles};
use crate::ui::{badge, banner, hint, print, Badge, OutputMode, UiContext};

/// Print a step indicator for the wizard flow.
fn print_step(ctx: &UiContext, step: usize, total: usize, title: &str, detail: Option<&str>) {
    if !ctx.mode.is_pretty() {
        return;
    }
    let progress = format!("{}/{}", step, total);
    let progress_styled = styled(&progress, styles::dim(), ctx.color);
    let title_styled = styled(title, styles::bold(), ctx.color);
    println!("{}  {}", progress_styled, title_styled);
    if let Some(text) = detail {
        let detail_styled = styled(text, styles::dim(), ctx.color);
        println!("    {}", detail_styled);
    }
}

fn print_option_help(ctx: &UiContext, text: &str) {
    if !ctx.mode.is_pretty() {
        return;
    }
    let detail_styled = styled(text, styles::dim(), ctx.color);
    println!("  {}", detail_styled);
}

fn parse_timezone(value: &str) -> anyhow::Result<Option<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return Ok(None);
    }

    let tz = trimmed
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", trimmed))?;
    Ok(Some(tz.to_string()))
}

fn timezone_options() -> Vec<String> {
    let mut zones: Vec<String> = chrono_tz::TZ_VARIANTS
        .iter()
        .map(|tz| tz.to_string())
        .collect();
    zones.retain(|tz| tz != "UTC");
    zones.sort();
    zones.insert(0, "UTC".to_string());
    zones.insert(0, "Auto (system)".to_string());
    zones
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {} >/dev/null 2>&1", cmd))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn editor_command_name(value: &str) -> Option<&str> {
    value.split_whitespace().next().filter(|s| !s.is_empty())
}

fn available_editors() -> Vec<String> {
    let mut editors = Vec::new();

    if let Ok(editor) = std::env::var("EDITOR") {
        if let Some(cmd) = editor_command_name(&editor) {
            if command_exists(cmd) {
                editors.push(editor);
            }
        }
    }

    let candidates = [
        "code",
        "code-insiders",
        "cursor",
        "zed",
        "subl",
        "vim",
        "nvim",
        "vi",
        "nano",
        "emacs",
    ];

    for candidate in candidates {
        if command_exists(candidate) && !editors.iter().any(|e| e == candidate) {
            editors.push(candidate.to_string());
        }
    }

    editors
}

struct PathCompletion;

impl PathCompletion {
    fn new() -> Self {
        Self
    }

    fn expand_tilde(input: &str) -> String {
        let Ok(home) = std::env::var("HOME") else {
            return input.to_string();
        };

        if input == "~" {
            return home;
        }

        if let Some(rest) = input.strip_prefix("~/") {
            return format!("{}/{}", home, rest);
        }

        input.to_string()
    }
}

impl Completion for PathCompletion {
    fn get(&self, input: &str) -> Option<String> {
        if input.trim().is_empty() {
            return None;
        }

        if input == "~" {
            return Some("~/".to_string());
        }

        let separator = std::path::MAIN_SEPARATOR;
        let (base_input_dir, _) = if input.ends_with(separator) {
            (input.to_string(), "")
        } else if let Some((dir, file)) = input.rsplit_once(separator) {
            (format!("{}{}", dir, separator), file)
        } else {
            (String::new(), input)
        };

        let expanded = Self::expand_tilde(input);
        let (expanded_dir, prefix_expanded) = if expanded.ends_with(separator) {
            (std::path::PathBuf::from(&expanded), "")
        } else if let Some((dir, file)) = expanded.rsplit_once(separator) {
            (std::path::PathBuf::from(dir), file)
        } else {
            (std::env::current_dir().ok()?, expanded.as_str())
        };

        let mut matches = Vec::new();
        for entry in std::fs::read_dir(&expanded_dir).ok()? {
            let entry = entry.ok()?;
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if !name.starts_with(prefix_expanded) {
                continue;
            }
            let is_dir = entry.file_type().ok().map(|t| t.is_dir()).unwrap_or(false);
            matches.push((is_dir, name.to_string()));
        }

        if matches.is_empty() {
            return None;
        }

        matches.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        let (is_dir, name) = &matches[0];

        let mut suggestion = String::new();
        suggestion.push_str(&base_input_dir);
        suggestion.push_str(name);
        if *is_dir {
            suggestion.push(separator);
        }

        Some(suggestion)
    }
}

pub fn handle_init(ctx: &AppContext, args: &InitArgs) -> anyhow::Result<()> {
    let interactive = std::io::stdin().is_terminal();
    let effective_no_input = args.no_input || !interactive;

    // Create UI context for step indicators
    let ui_ctx = ctx.ui_context(false, None);
    let total_steps = 5;
    let path_completion = PathCompletion::new();

    if !ctx.quiet() && ui_ctx.mode.is_pretty() {
        if let Some(banner_text) = banner(&ui_ctx) {
            println!("{}", banner_text);
        }
        let version_line = format!("Ledger v{}", VERSION);
        println!("{}", styled(&version_line, styles::dim(), ui_ctx.color));
        println!();
    }

    if !ctx.quiet() && !effective_no_input && ui_ctx.mode.is_pretty() {
        // Print wizard header
        let header = styled("Ledger", styles::bold(), ui_ctx.color);
        println!("{} \u{00B7} init\n", header);
    }

    let default_ledger = default_ledger_path()?;
    let ledger_path = match args.path.clone().or_else(|| ctx.cli().ledger.clone()) {
        Some(value) => std::path::PathBuf::from(value),
        None => {
            if effective_no_input {
                default_ledger.clone()
            } else {
                print_step(
                    &ui_ctx,
                    1,
                    total_steps,
                    "Choose location",
                    Some("Set where your encrypted ledger file will live."),
                );
                let theme = ColorfulTheme::default();
                let input: String = Input::with_theme(&theme)
                    .with_prompt("Ledger file location")
                    .completion_with(&path_completion)
                    .default(default_ledger.to_string_lossy().to_string())
                    .interact_text()?;
                println!();
                std::path::PathBuf::from(input)
            }
        }
    };

    let config_path = resolve_config_path()?;
    let mut passphrase_cache_ttl_seconds = args.passphrase_cache_ttl_seconds.unwrap_or(0);
    let mut keyfile_path = if let Some(ref value) = args.keyfile_path {
        std::path::PathBuf::from(value)
    } else {
        default_keyfile_path()?
    };
    let mut timezone: Option<String> = args.timezone.clone();
    let mut editor: Option<String> = args.editor.clone();

    let passphrase = if let Ok(value) = std::env::var("LEDGER_PASSPHRASE") {
        if !value.trim().is_empty() {
            value
        } else if effective_no_input {
            return Err(anyhow::anyhow!(
                "--no-input requires LEDGER_PASSPHRASE for initialization"
            ));
        } else {
            print_step(
                &ui_ctx,
                2,
                total_steps,
                "Create passphrase",
                Some("Used to encrypt and unlock your ledger."),
            );
            let pp = prompt_init_passphrase()?;
            println!();
            pp
        }
    } else if effective_no_input {
        return Err(anyhow::anyhow!(
            "--no-input requires LEDGER_PASSPHRASE for initialization"
        ));
    } else {
        print_step(
            &ui_ctx,
            2,
            total_steps,
            "Create passphrase",
            Some("Used to encrypt and unlock your ledger."),
        );
        let pp = prompt_init_passphrase()?;
        println!();
        pp
    };

    let mut tier = SecurityTier::Passphrase;
    if !effective_no_input {
        print_step(
            &ui_ctx,
            3,
            total_steps,
            "Security level",
            Some("Choose convenience options for unlocking your ledger."),
        );
        let options = [
            "Passphrase only (recommended)",
            "Passphrase + OS keychain",
            "Passphrase + encrypted keyfile",
            "Device keyfile only (reduced security)",
        ];
        let theme = ColorfulTheme::default();
        let choice = Select::with_theme(&theme)
            .with_prompt("Security level")
            .default(0)
            .items(&options)
            .interact()?;
        tier = match choice {
            0 => SecurityTier::Passphrase,
            1 => SecurityTier::PassphraseKeychain,
            2 => SecurityTier::PassphraseKeyfile,
            3 => SecurityTier::DeviceKeyfile,
            _ => SecurityTier::Passphrase,
        };
        println!();
    }

    if matches!(tier, SecurityTier::DeviceKeyfile) && !effective_no_input {
        let proceed = Confirm::new()
            .with_prompt(device_keyfile_warning())
            .default(false)
            .interact()?;
        if !proceed {
            return Err(anyhow::anyhow!("Initialization cancelled"));
        }
    }

    if !effective_no_input {
        print_step(
            &ui_ctx,
            4,
            total_steps,
            "Advanced settings",
            Some("Set preferences and storage defaults."),
        );
        let theme = ColorfulTheme::default();

        if timezone.is_none() {
            print_option_help(&ui_ctx, "Select the timezone used for entry timestamps.");
            let tz_options = timezone_options();
            let selection = FuzzySelect::with_theme(&theme)
                .with_prompt("Timezone")
                .default(0)
                .items(&tz_options)
                .interact()?;
            match tz_options.get(selection).map(|s| s.as_str()) {
                Some("Auto (system)") => timezone = None,
                Some(value) => timezone = Some(value.to_string()),
                None => {}
            }
        }

        if editor.is_none() {
            print_option_help(&ui_ctx, "Compose entries directly from the terminal.");
            let default_editor = default_editor();
            let mut editor_choices = available_editors();
            if editor_choices.is_empty() {
                let editor_input: String = Input::with_theme(&theme)
                    .with_prompt("Default editor")
                    .default(default_editor)
                    .interact_text()?;
                if !editor_input.trim().is_empty() {
                    editor = Some(editor_input);
                }
            } else {
                editor_choices.push("Other...".to_string());
                let default_index = editor_command_name(&default_editor)
                    .and_then(|name| {
                        editor_choices.iter().position(|choice| {
                            editor_command_name(choice).unwrap_or(choice) == name
                        })
                    })
                    .unwrap_or(0);
                let selection = Select::with_theme(&theme)
                    .with_prompt("Default editor")
                    .default(default_index)
                    .items(&editor_choices)
                    .interact()?;
                if editor_choices
                    .get(selection)
                    .map(|choice| choice == "Other...")
                    .unwrap_or(false)
                {
                    let editor_input: String = Input::with_theme(&theme)
                        .with_prompt("Editor command")
                        .default(default_editor)
                        .interact_text()?;
                    if !editor_input.trim().is_empty() {
                        editor = Some(editor_input);
                    }
                } else if let Some(choice) = editor_choices.get(selection) {
                    editor = Some(choice.to_string());
                }
            }
        }

        if args.passphrase_cache_ttl_seconds.is_none() {
            print_option_help(
                &ui_ctx,
                "Cache the passphrase to avoid re-entering it on each command.",
            );
            let ttl_input: String = Input::with_theme(&theme)
                .with_prompt("Passphrase cache (seconds)")
                .default(passphrase_cache_ttl_seconds.to_string())
                .interact_text()?;
            passphrase_cache_ttl_seconds = ttl_input.parse().map_err(|_| {
                anyhow::anyhow!(
                    "Invalid cache TTL: {} (expected integer seconds)",
                    ttl_input
                )
            })?;
        }

        if matches!(
            tier,
            SecurityTier::PassphraseKeyfile | SecurityTier::DeviceKeyfile
        ) && args.keyfile_path.is_none()
        {
            print_option_help(&ui_ctx, "Choose where the keyfile will be stored.");
            let input: String = Input::with_theme(&theme)
                .with_prompt("Keyfile path")
                .completion_with(&path_completion)
                .default(keyfile_path.to_string_lossy().to_string())
                .interact_text()?;
            keyfile_path = std::path::PathBuf::from(input);
        }

        println!();
    }

    let timezone = parse_timezone(timezone.as_deref().unwrap_or(""))?;

    let (ledger_passphrase, keyfile_mode, keyfile_path_value) = match tier {
        SecurityTier::Passphrase => (passphrase.clone(), KeyfileMode::None, None),
        SecurityTier::PassphraseKeychain => (passphrase.clone(), KeyfileMode::None, None),
        SecurityTier::PassphraseKeyfile => {
            let key_bytes = generate_key_bytes()?;
            write_keyfile_encrypted(&keyfile_path, &key_bytes, &passphrase)?;
            (
                key_bytes_to_passphrase(&key_bytes),
                KeyfileMode::Encrypted,
                Some(keyfile_path.clone()),
            )
        }
        SecurityTier::DeviceKeyfile => {
            let key_bytes = generate_key_bytes()?;
            write_keyfile_plain(&keyfile_path, &key_bytes)?;
            (
                key_bytes_to_passphrase(&key_bytes),
                KeyfileMode::Plain,
                Some(keyfile_path.clone()),
            )
        }
    };

    if let Some(parent) = ledger_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!(
                "Failed to create ledger directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    // Print review step before creating
    if !effective_no_input && ui_ctx.mode.is_pretty() {
        let review_step = 5;
        print_step(
            &ui_ctx,
            review_step,
            total_steps,
            "Creating ledger",
            Some("Writing encrypted ledger and config files."),
        );
    }

    let _ = AgeSqliteStorage::create(&ledger_path, &ledger_passphrase)?;
    let storage = AgeSqliteStorage::open(&ledger_path, &ledger_passphrase)?;
    storage.close(&ledger_passphrase)?;

    let config = LedgerConfig::new(
        ledger_path.clone(),
        tier,
        passphrase_cache_ttl_seconds,
        keyfile_mode,
        keyfile_path_value,
        timezone,
        editor,
    );
    write_config(&config_path, &config)?;

    if matches!(tier, SecurityTier::PassphraseKeychain) {
        let account = ledger_hash(&ledger_path);
        let _ = keychain_set(&account, &passphrase);
    }

    if !ctx.quiet() {
        match ui_ctx.mode {
            OutputMode::Pretty => {
                println!();
                print(
                    &ui_ctx,
                    &badge(
                        &ui_ctx,
                        Badge::Ok,
                        &format!("Ledger created at {}", ledger_path.to_string_lossy()),
                    ),
                );
                print(
                    &ui_ctx,
                    &badge(
                        &ui_ctx,
                        Badge::Ok,
                        &format!("Config written to {}", config_path.to_string_lossy()),
                    ),
                );
                if passphrase_cache_ttl_seconds > 0 {
                    print(
                        &ui_ctx,
                        &hint(
                            &ui_ctx,
                            &format!(
                                "Passphrase caching keeps your passphrase in memory for {} seconds.",
                                passphrase_cache_ttl_seconds
                            ),
                        ),
                    );
                }
                // Next step hints
                println!();
                print(
                    &ui_ctx,
                    &hint(
                        &ui_ctx,
                        "ledger add journal  \u{00B7}  ledger list  \u{00B7}  ledger --help",
                    ),
                );
            }
            OutputMode::Plain | OutputMode::Json => {
                println!("status=ok");
                println!("ledger_path={}", ledger_path.to_string_lossy());
                println!("config_path={}", config_path.to_string_lossy());
                if passphrase_cache_ttl_seconds > 0 {
                    println!("passphrase_cache_ttl={}", passphrase_cache_ttl_seconds);
                }
            }
        }
    }

    Ok(())
}

fn default_editor() -> String {
    std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string())
}
