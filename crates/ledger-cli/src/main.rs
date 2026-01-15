//! Ledger CLI - A secure, encrypted, CLI-first personal journal and logbook
//!
//! This is the command-line interface for Ledger. It provides a user-friendly
//! interface to the core library functionality.

mod app;
mod cache;
mod cli;
mod commands;
mod config;
mod constants;
mod errors;
mod helpers;
mod output;
mod security;
mod ui;

use clap::Parser;
use ledger_core::VERSION;

use std::path::PathBuf;

use crate::app::{resolve_config_path, AppContext};
use crate::cli::{Cli, Commands, CompositionsSubcommand, TemplatesSubcommand};
use crate::commands::{associations, compositions, entries, init, maintenance, misc, templates};
use crate::config::read_config;
use crate::ui::theme::{styled, styles};
use crate::ui::{banner, blank_line, hint, kv, print, print_error, OutputMode};

fn main() {
    let cli = Cli::parse();
    let ctx = AppContext::new(&cli);

    if let Err(e) = run(&ctx, &cli) {
        // Get UI context for error formatting
        let ui_ctx = ctx.ui_context(false, None);

        // Extract hint from error chain if available
        let error_msg = format!("{}", e);
        let hint = extract_error_hint(&error_msg);

        print_error(&ui_ctx, &error_msg, hint.as_deref());
        std::process::exit(1);
    }
}

/// Extract a hint from an error message if it contains "Hint:" or similar patterns,
/// or provide contextual hints for common error types.
fn extract_error_hint(error: &str) -> Option<String> {
    // Check for explicit hint patterns in error messages
    if let Some(idx) = error.find("\nHint:") {
        return Some(error[idx + 1..].to_string());
    }
    if let Some(idx) = error.find("\nhint:") {
        return Some(error[idx + 1..].to_string());
    }

    // Provide contextual hints for common error patterns
    let error_lower = error.to_lowercase();

    // Entry not found
    if error_lower.contains("entry") && error_lower.contains("not found") {
        return Some("Hint: Run `ledger list --last 7d` to find entry IDs.".to_string());
    }

    // Template not found
    if error_lower.contains("template") && error_lower.contains("not found") {
        return Some("Hint: Run `ledger template list` to see available templates.".to_string());
    }

    // Composition not found
    if error_lower.contains("composition") && error_lower.contains("not found") {
        return Some(
            "Hint: Run `ledger composition list` to see available compositions.".to_string(),
        );
    }

    // Invalid entry ID format
    if error_lower.contains("invalid entry id") {
        return Some(
            "Hint: Entry IDs are UUIDs (e.g., 7a2e3c0b-1234-5678-9abc-def012345678). Use the first 8 characters as a shorthand.".to_string(),
        );
    }

    // Ledger not initialized
    if error_lower.contains("not initialized") || error_lower.contains("config not found") {
        return Some("Hint: Run `ledger init` to create a new ledger.".to_string());
    }

    // Wrong passphrase
    if error_lower.contains("wrong passphrase") || error_lower.contains("decryption failed") {
        return Some("Hint: Check your passphrase. Set LEDGER_PASSPHRASE env var or use --no-input with a keyfile.".to_string());
    }

    // Entry type not found
    if error_lower.contains("entry type") && error_lower.contains("not found") {
        return Some(
            "Hint: Valid entry types are 'journal' (built-in). Custom types require manual setup."
                .to_string(),
        );
    }

    // Backup destination issues
    if error_lower.contains("backup") && error_lower.contains("destination") {
        return Some(
            "Hint: Ensure the destination path is writable and the parent directory exists."
                .to_string(),
        );
    }

    // Integrity check failed
    if error_lower.contains("integrity") && error_lower.contains("failed") {
        return Some("Hint: Restore from a backup with `ledger backup --restore <file>` or export data first.".to_string());
    }

    None
}

fn run(ctx: &AppContext, cli: &Cli) -> anyhow::Result<()> {
    match &cli.command {
        Some(Commands::Init(args)) => {
            init::handle_init(ctx, args)?;
        }
        Some(Commands::Add(args)) => {
            entries::handle_add(ctx, args)?;
        }
        Some(Commands::Edit(args)) => {
            entries::handle_edit(ctx, args)?;
        }
        Some(Commands::List(args)) => {
            entries::handle_list(ctx, args)?;
        }
        Some(Commands::Search(args)) => {
            entries::handle_search(ctx, args)?;
        }
        Some(Commands::Show(args)) => {
            entries::handle_show(ctx, args)?;
        }
        Some(Commands::Export(args)) => {
            entries::handle_export(ctx, args)?;
        }
        Some(Commands::Check) => {
            maintenance::handle_check(ctx)?;
        }
        Some(Commands::Backup(args)) => {
            maintenance::handle_backup(ctx, args)?;
        }
        Some(Commands::Lock) => {
            maintenance::handle_lock(ctx)?;
        }
        Some(Commands::Doctor(args)) => {
            maintenance::handle_doctor(ctx, args)?;
        }
        Some(Commands::Completions(args)) => {
            misc::handle_completions(args)?;
        }
        Some(Commands::InternalCacheDaemon(args)) => {
            maintenance::handle_internal_cache_daemon(args)?;
        }
        Some(Commands::Compositions(args)) => match &args.command {
            CompositionsSubcommand::Create(create_args) => {
                compositions::handle_create(ctx, create_args)?;
            }
            CompositionsSubcommand::List(list_args) => {
                compositions::handle_list(ctx, list_args)?;
            }
            CompositionsSubcommand::Show(show_args) => {
                compositions::handle_show(ctx, show_args)?;
            }
            CompositionsSubcommand::Rename(rename_args) => {
                compositions::handle_rename(ctx, rename_args)?;
            }
            CompositionsSubcommand::Delete(delete_args) => {
                compositions::handle_delete(ctx, delete_args)?;
            }
        },
        Some(Commands::Templates(args)) => match &args.command {
            TemplatesSubcommand::Create(create_args) => {
                templates::handle_create(ctx, create_args)?;
            }
            TemplatesSubcommand::List(list_args) => {
                templates::handle_list(ctx, list_args)?;
            }
            TemplatesSubcommand::Show(show_args) => {
                templates::handle_show(ctx, show_args)?;
            }
            TemplatesSubcommand::Update(update_args) => {
                templates::handle_update(ctx, update_args)?;
            }
            TemplatesSubcommand::Delete(delete_args) => {
                templates::handle_delete(ctx, delete_args)?;
            }
            TemplatesSubcommand::SetDefault(set_default_args) => {
                templates::handle_set_default(ctx, set_default_args)?;
            }
            TemplatesSubcommand::ClearDefault(clear_default_args) => {
                templates::handle_clear_default(ctx, clear_default_args)?;
            }
        },
        Some(Commands::Attach(args)) => {
            associations::handle_attach(ctx, args)?;
        }
        Some(Commands::Detach(args)) => {
            associations::handle_detach(ctx, args)?;
        }
        None => {
            if ctx.quiet() {
                return Ok(());
            }

            let ui_ctx = ctx.ui_context(false, None);
            if ui_ctx.mode == OutputMode::Json {
                return Ok(());
            }

            if let Some(banner_text) = banner(&ui_ctx) {
                print(&ui_ctx, &banner_text);
            }
            if ui_ctx.mode.is_pretty() {
                let version_line = format!("Ledger v{}", VERSION);
                print(&ui_ctx, &styled(&version_line, styles::dim(), ui_ctx.color));
                blank_line(&ui_ctx);
            }

            let state = detect_bootstrap_state(cli);
            match state {
                BootstrapState::MissingConfig { config_path } => {
                    print(&ui_ctx, &kv(&ui_ctx, "Status", "not initialized"));
                    if let Some(path) = config_path {
                        let value = format!("not found ({})", path.display());
                        print(&ui_ctx, &kv(&ui_ctx, "Config", &value));
                    }
                    blank_line(&ui_ctx);
                    print_next_steps(
                        &ui_ctx,
                        &[
                            "ledger init",
                            "ledger init --help",
                            "ledger --ledger /path/to/my.ledger init",
                        ],
                    );
                }
                BootstrapState::MissingLedger { ledger_path } => {
                    print(&ui_ctx, &kv(&ui_ctx, "Status", "ledger file missing"));
                    let value = ledger_path.display().to_string();
                    print(&ui_ctx, &kv(&ui_ctx, "Ledger", &value));
                    blank_line(&ui_ctx);
                    print_next_steps(
                        &ui_ctx,
                        &[
                            "ledger init",
                            "ledger --ledger /path/to/my.ledger init",
                            "ledger init --help",
                        ],
                    );
                }
                BootstrapState::Ready { ledger_path } => {
                    print(&ui_ctx, &kv(&ui_ctx, "Status", "ready"));
                    let value = ledger_path.display().to_string();
                    print(&ui_ctx, &kv(&ui_ctx, "Ledger", &value));
                    blank_line(&ui_ctx);
                    print_next_steps(
                        &ui_ctx,
                        &[
                            "ledger add journal",
                            "ledger list --last 7d",
                            "ledger search \"...\"",
                            "ledger templates list",
                            "ledger compositions list",
                        ],
                    );
                }
            }

            if ui_ctx.mode.is_pretty() {
                blank_line(&ui_ctx);
                print(
                    &ui_ctx,
                    &hint(&ui_ctx, "Run `ledger --help` for full usage."),
                );
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
enum BootstrapState {
    MissingConfig { config_path: Option<PathBuf> },
    MissingLedger { ledger_path: PathBuf },
    Ready { ledger_path: PathBuf },
}

fn detect_bootstrap_state(cli: &Cli) -> BootstrapState {
    if let Some(path) = cli.ledger.as_ref() {
        let ledger_path = PathBuf::from(path);
        return if ledger_path.exists() {
            BootstrapState::Ready { ledger_path }
        } else {
            BootstrapState::MissingLedger { ledger_path }
        };
    }

    let config_path = resolve_config_path().ok();
    let Some(config_path) = config_path else {
        return BootstrapState::MissingConfig { config_path: None };
    };

    if !config_path.exists() {
        return BootstrapState::MissingConfig {
            config_path: Some(config_path),
        };
    }

    let config = match read_config(&config_path) {
        Ok(cfg) => cfg,
        Err(_) => {
            return BootstrapState::MissingConfig {
                config_path: Some(config_path),
            }
        }
    };

    let ledger_path = PathBuf::from(config.ledger.path);
    if ledger_path.exists() {
        BootstrapState::Ready { ledger_path }
    } else {
        BootstrapState::MissingLedger { ledger_path }
    }
}

fn print_next_steps(ctx: &crate::ui::UiContext, steps: &[&str]) {
    if ctx.mode.is_pretty() {
        print(ctx, "Next:");
        for step in steps {
            print(ctx, &format!("  {}", step));
        }
    } else {
        for step in steps {
            print(ctx, &format!("next={}", step));
        }
    }
}
