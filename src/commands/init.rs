use crate::commands::{CommandContext, SubcommandTrait};
use anyhow::Context;
use clap::Parser;

/// Configure the shell environment for env-select. Intended to be piped
/// to `source` as part of your shell startup.
#[derive(Clone, Debug, Parser)]
pub struct InitCommand;

impl SubcommandTrait for InitCommand {
    fn execute(self, context: CommandContext) -> anyhow::Result<()> {
        let script = context
            .shell
            .init_script()
            .context("Error generating shell init script")?;
        print!("{script}");
        Ok(())
    }
}
