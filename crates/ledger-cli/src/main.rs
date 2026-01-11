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

use clap::Parser;
use ledger_core::VERSION;

use crate::app::AppContext;
use crate::cli::{Cli, Commands};
use crate::commands::{entries, init, maintenance, misc};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let ctx = AppContext::new(&cli);

    match &cli.command {
        Some(Commands::Init(args)) => {
            init::handle_init(&ctx, args)?;
        }
        Some(Commands::Add(args)) => {
            entries::handle_add(&ctx, args)?;
        }
        Some(Commands::Edit(args)) => {
            entries::handle_edit(&ctx, args)?;
        }
        Some(Commands::List(args)) => {
            entries::handle_list(&ctx, args)?;
        }
        Some(Commands::Search(args)) => {
            entries::handle_search(&ctx, args)?;
        }
        Some(Commands::Show(args)) => {
            entries::handle_show(&ctx, args)?;
        }
        Some(Commands::Export(args)) => {
            entries::handle_export(&ctx, args)?;
        }
        Some(Commands::Check) => {
            maintenance::handle_check(&ctx)?;
        }
        Some(Commands::Backup(args)) => {
            maintenance::handle_backup(&ctx, args)?;
        }
        Some(Commands::Lock) => {
            maintenance::handle_lock(&ctx)?;
        }
        Some(Commands::Doctor(args)) => {
            maintenance::handle_doctor(&ctx, args)?;
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
