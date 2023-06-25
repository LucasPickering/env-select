use crate::{
    config::{Config, Value},
    console,
    shell::Shell,
};
use anyhow::anyhow;

/// Container to handle user selection and command generation. This is the core
/// logic for the program.
#[derive(Clone, Debug)]
pub struct Exporter {
    config: Config,
    shell: Shell,
}

impl Exporter {
    pub fn new(config: Config, shell: Shell) -> Self {
        Self { config, shell }
    }

    /// Print the export command, and if apppropriate, tell the user about a
    /// sick pro tip.
    pub fn print_export_commands(
        &self,
        select_key: &str,
        profile: Option<&str>,
    ) -> anyhow::Result<()> {
        let export_command = self.get_export_commands(select_key, profile)?;
        println!("{}", export_command);
        console::print_installation_hint()?;
        Ok(())
    }

    /// Prompt the user to select a value/profile for the variable/application
    /// they gave, then use that to calculate the command(s) we want to feed
    /// to the shell to set the desired environment variables. This is
    /// basically the whole program.
    fn get_export_commands(
        &self,
        select_key: &str,
        profile_name: Option<&str>,
    ) -> anyhow::Result<String> {
        // Check for single variable first
        if let Some(var_options) = self.config.variables.get(select_key) {
            let value = match profile_name {
                // This is kinda weird, why are you using env-select to just
                // pass a single value? You could just run the
                // shell command directly... Regardless, we
                // might as well support this instead of ignoring it
                // or throwing an error
                Some(value) => Value::Literal(value.into()),
                // The standard use case - prompt the user to pick a value
                None => {
                    console::prompt_variable(select_key, var_options)?.clone()
                }
            };

            Ok(self.shell.export_variable(select_key, &value))
        }
        // Check for applications next
        else if let Some(application) =
            self.config.applications.get(select_key)
        {
            let profile = match profile_name {
                // User passed a profile name as an arg - look for a profile of
                // that name
                Some(profile_name) => {
                    application.profiles.get(profile_name).ok_or_else(|| {
                        anyhow!(
                            "No profile with the name {}, options are: {}",
                            profile_name,
                            application
                                .profiles
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })?
                }
                // Show a prompt to ask the user which varset to use
                None => console::prompt_application(application)?,
            };

            Ok(self.shell.export_profile(profile))
        } else {
            // Didn't match anything :(
            Err(self.config.get_suggestion_error(
                "No known variable or application by the name `{}`. {}",
            ))
        }
    }
}
