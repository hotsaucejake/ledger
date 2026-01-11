use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::Cli;

pub fn handle_completions(shell: clap_complete::Shell) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "ledger", &mut std::io::stdout());
    Ok(())
}
