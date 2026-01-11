use crate::cache::{cache_clear, cache_socket_path};
use crate::cli::Cli;

pub fn handle_lock(cli: &Cli) -> anyhow::Result<()> {
    if let Ok(socket_path) = cache_socket_path() {
        let _ = cache_clear(&socket_path);
    }
    if !cli.quiet {
        println!("Passphrase cache cleared.");
    }
    Ok(())
}
