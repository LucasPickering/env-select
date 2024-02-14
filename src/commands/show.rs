use crate::{
    commands::{CommandContext, SubcommandTrait},
    config::{MapExt, Name},
};
use clap::{Parser, Subcommand};

/// Print configuration and meta information
#[derive(Clone, Debug, Parser)]
pub struct ShowCommand {
    #[command(subcommand)]
    command: ShowSubcommand,
}

#[derive(Clone, Debug, Subcommand)]
enum ShowSubcommand {
    /// Print configuration for a profile, in TOML format
    Config {
        // We can't use the Selection helper type here because the doc strings
        // are incorrect for this use case
        /// Application to show configuration for. If omitted, show all
        /// applications.
        application: Option<Name>,
        /// Profile to show configuration for. If omitted, show all profiles
        /// for the selected application.
        profile: Option<Name>,
    },
    /// Print the name or path to the shell in use
    Shell,
}

impl SubcommandTrait for ShowCommand {
    fn execute(self, context: CommandContext) -> anyhow::Result<()> {
        match self.command {
            ShowSubcommand::Config {
                application,
                profile,
            } => {
                // Serialize isn't object-safe, so there's no way to return a
                // dynamic object of what to serialize. That means each branch
                // has to serialize itself
                let config = context.config()?;
                let content = if let Some(application) = application {
                    let application =
                        config.applications.try_get(&application)?;
                    if let Some(profile) = profile {
                        let profile = application.profiles.try_get(&profile)?;
                        toml::to_string(profile)
                    } else {
                        toml::to_string(application)
                    }
                } else {
                    // Print entire config
                    toml::to_string(config)
                }?;
                println!("{}", content);
            }
            ShowSubcommand::Shell => println!("{}", context.shell),
        }
        Ok(())
    }
}
