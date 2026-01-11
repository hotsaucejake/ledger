use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::{Cli, CompletionsArgs};

pub fn handle_completions(args: &CompletionsArgs) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    generate(args.shell, &mut cmd, "ledger", &mut std::io::stdout());
    Ok(())
}
