use crate::{
    commands::{CommandContext, Selection, SubcommandTrait},
    console::print_hint,
};
use anyhow::Context;
use clap::Parser;
use std::fs;

const WEBSITE: &str = "https://env-select.lucaspickering.me";

/// Modify current shell environment
#[derive(Clone, Debug, Parser)]
pub struct SetCommand {
    #[command(flatten)]
    selection: Selection,
}

impl SubcommandTrait for SetCommand {
    fn execute(self, context: CommandContext) -> anyhow::Result<()> {
        let profile = context.select_profile(&self.selection)?;
        let environment = context.load_environment(profile)?;

        let source_output = context.shell.export(&environment);

        // If --source-file was passed, we were probably called from the shell
        // wrapper function. Write sourceable output to the given file.
        if let Some(source_file) = context.source_file.as_ref() {
            fs::write(source_file, source_output).with_context(|| {
                format!(
                    "Error writing sourceable output to file {source_file:?}"
                )
            })?;
            // Tell the user what we exported
            println!("The following variables will be set:");
            println!("{environment:#}");
        } else {
            // We were *not* called from the shell wrapper here, so just print
            // the output and let the user know about a pro tip
            print!("{source_output}");
            print_hint(&format!(
                "This output must be piped to `source` to be applied. \
                    Install the `es` shell function to apply automatically: \
                    {WEBSITE}/book/install.html#install-shell-function",
            ))?;
        }

        Ok(())
    }
}
