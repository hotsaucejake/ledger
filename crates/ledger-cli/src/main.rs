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
        Some(Commands::Init {
            path,
            advanced,
            no_input,
            timezone,
            editor,
            passphrase_cache_ttl_seconds,
            keyfile_path,
            config_path,
        }) => {
            init::handle_init(
                &cli,
                path.clone(),
                *advanced,
                *no_input,
                timezone.clone(),
                editor.clone(),
                *passphrase_cache_ttl_seconds,
                keyfile_path.clone(),
                config_path.clone(),
            )?;
        }
        Some(Commands::Add {
            entry_type,
            tag,
            date,
            no_input,
            body,
        }) => {
            let editor_override = app::load_security_config(&cli)?.editor;
            entries::handle_add(
                &cli,
                entry_type,
                tag,
                date,
                *no_input,
                body,
                editor_override.as_deref(),
            )?;
        }
        Some(Commands::Edit { id, body, no_input }) => {
            let editor_override = app::load_security_config(&cli)?.editor;
            entries::handle_edit(&cli, id, body, *no_input, editor_override.as_deref())?;
        }
        Some(Commands::List {
            entry_type,
            tag,
            last,
            since,
            until,
            limit,
            json,
            format,
        }) => {
            entries::handle_list(
                &cli, entry_type, tag, last, since, until, limit, *json, format,
            )?;
        }
        Some(Commands::Search {
            query,
            r#type,
            last,
            json,
            limit,
            format,
        }) => {
            entries::handle_search(&cli, query, r#type, last, *json, limit, format)?;
        }
        Some(Commands::Show { id, json }) => {
            entries::handle_show(&cli, id, *json)?;
        }
        Some(Commands::Export {
            entry_type,
            format,
            since,
        }) => {
            entries::handle_export(&cli, entry_type, format, since)?;
        }
        Some(Commands::Check) => {
            maintenance::handle_check(&cli)?;
        }
        Some(Commands::Backup { destination }) => {
            maintenance::handle_backup(&cli, destination)?;
        }
        Some(Commands::Lock) => {
            maintenance::handle_lock(&cli)?;
        }
        Some(Commands::Doctor { no_input }) => {
            maintenance::handle_doctor(&cli, *no_input)?;
        }
        Some(Commands::Completions { shell }) => {
            misc::handle_completions(*shell)?;
        }
        Some(Commands::InternalCacheDaemon { ttl, socket }) => {
            maintenance::handle_internal_cache_daemon(*ttl, socket)?;
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
