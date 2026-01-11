//! Ledger CLI - A secure, encrypted, CLI-first personal journal and logbook
//!
//! This is the command-line interface for Ledger. It provides a user-friendly
//! interface to the core library functionality.

mod app;
mod cache;
mod cli;
mod commands;
mod config;
mod helpers;
mod output;
mod security;

use clap::Parser;
use ledger_core::VERSION;

use crate::cli::{Cli, Commands};
use crate::commands::{entries, init, maintenance, misc};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Init(args)) => {
            init::handle_init(&cli, args)?;
        }
        Some(Commands::Add(args)) => {
            let editor_override = app::load_security_config(&cli)?.editor;
            entries::handle_add(&cli, args, editor_override.as_deref())?;
        }
        Some(Commands::Edit(args)) => {
            let editor_override = app::load_security_config(&cli)?.editor;
            entries::handle_edit(&cli, args, editor_override.as_deref())?;
        }
        Some(Commands::List(args)) => {
            entries::handle_list(&cli, args)?;
        }
        Some(Commands::Search(args)) => {
            entries::handle_search(&cli, args)?;
        }
        Some(Commands::Show(args)) => {
            entries::handle_show(&cli, args)?;
        }
        Some(Commands::Export(args)) => {
            entries::handle_export(&cli, args)?;
        }
        Some(Commands::Check) => {
            maintenance::handle_check(&cli)?;
        }
        Some(Commands::Backup(args)) => {
            maintenance::handle_backup(&cli, args)?;
        }
        Some(Commands::Lock) => {
            maintenance::handle_lock(&cli)?;
        }
        Some(Commands::Doctor(args)) => {
            maintenance::handle_doctor(&cli, args)?;
        }
        Some(Commands::Completions(args)) => {
            misc::handle_completions(args)?;
        }
        Some(Commands::InternalCacheDaemon(args)) => {
            maintenance::handle_internal_cache_daemon(args)?;
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
