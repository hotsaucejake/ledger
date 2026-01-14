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

use crate::app::AppContext;
use crate::cli::{Cli, Commands, CompositionsSubcommand, TemplatesSubcommand};
use crate::commands::{associations, compositions, entries, init, maintenance, misc, templates};
use crate::ui::print_error;

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

/// Extract a hint from an error message if it contains "Hint:" or similar patterns.
fn extract_error_hint(error: &str) -> Option<String> {
    // Check for common hint patterns in error messages
    if let Some(idx) = error.find("\nHint:") {
        return Some(error[idx + 1..].to_string());
    }
    if let Some(idx) = error.find("\nhint:") {
        return Some(error[idx + 1..].to_string());
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
            println!("Ledger v{}", VERSION);
            println!("\nQuickstart:");
            println!("  ledger init");
            println!("  ledger add journal --body \"Hello\"");
            println!("  ledger list --last 7d");
            println!("  ledger search \"Hello\"");
            println!("  ledger show <id>");
            println!("\nRun `ledger --help` for full usage.");
        }
    }

    Ok(())
}
