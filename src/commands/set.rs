use crate::commands::{CommandContext, Selection, SubcommandTrait};
use anyhow::{anyhow, Context};
use clap::Parser;
use std::fs;

/// Modify shell environment via a configured variable/application
#[derive(Clone, Debug, Parser)]
pub struct SetCommand {
    #[command(flatten)]
    selection: Selection,
}

impl SubcommandTrait for SetCommand {
    fn execute(self, context: CommandContext) -> anyhow::Result<()> {
        let source_file = context.source_file.as_ref().ok_or_else(|| {
            anyhow!("--source-file argument required for subcommand `set`")
        })?;

        let profile = context.select_profile(&self.selection)?;
        let environment = context.load_environment(profile)?;

        let source_output = context.shell.export(&environment);
        fs::write(source_file, source_output).with_context(|| {
            format!("Error writing sourceable output to file {source_file:?}")
        })?;

        // Tell the user what we exported
        println!("The following variables will be set:");
        println!("{environment:#}");

        Ok(())
    }
}
