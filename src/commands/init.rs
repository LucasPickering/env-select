use crate::{
    commands::{CommandContext, SubcommandTrait},
    Args, COMMAND_NAME,
};
use anyhow::Context;
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use std::env;

/// Configure the shell environment for env-select. Intended to be piped
/// to `source` as part of your shell startup.
#[derive(Clone, Debug, Parser)]
pub struct InitCommand {
    /// Don't include completion script in output
    #[clap(long, hide = true)] // Only for testing
    no_completions: bool,
}

impl SubcommandTrait for InitCommand {
    fn execute(self, context: CommandContext) -> anyhow::Result<()> {
        let script = context
            .shell
            .init_script()
            .context("Error generating shell init script")?;
        print!("{script}");

        // Print the command to enable shell completions as well. CompleteEnv
        // doesn't expose the inner machinery that would allow us to print the
        // line directly, so we have to enable the env var that triggers it
        if !self.no_completions {
            env::set_var("COMPLETE", context.shell.kind.to_string());
            CompleteEnv::with_factory(Args::command)
                .try_complete([COMMAND_NAME], None)?;
        }

        Ok(())
    }
}
