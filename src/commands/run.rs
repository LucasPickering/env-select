use crate::{
    commands::{CommandContext, Selection, SubcommandTrait},
    environment::Environment,
    error::ExitCodeError,
    execute::{revert_side_effects, Executable},
};
use clap::Parser;

/// Run a shell command in an augmented environment, via a configured
/// variable/application
#[derive(Clone, Debug, Parser)]
pub struct RunCommand {
    #[command(flatten)]
    selection: Selection,

    /// Shell command to execute
    #[arg(required = true, last = true)]
    command: Vec<String>,
}

impl SubcommandTrait for RunCommand {
    fn execute(self, context: CommandContext) -> anyhow::Result<()> {
        let profile = context.select_profile(&self.selection)?;
        let environment = context.load_environment(profile)?;

        // Undo clap's tokenization
        let mut executable: Executable =
            context.shell.executable(&self.command.join(" ").into());

        // Execute the command
        let status =
            smol::block_on(executable.environment(&environment).status())?;

        // Clean up side effects, in reverse order
        revert_side_effects(
            &profile.post_export,
            &context.shell,
            &environment,
        )?;
        // Teardown of pre-export should *not* have access to the environment,
        // to mirror the setup conditions
        revert_side_effects(
            &profile.pre_export,
            &context.shell,
            &Environment::default(),
        )?;

        if status.success() {
            Ok(())
        } else {
            // Map to our own exit code error type so we can forward to the user
            Err(ExitCodeError::from(&status).into())
        }
    }
}
