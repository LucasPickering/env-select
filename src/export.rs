use crate::{
    config::{Application, Config, Profile, Value},
    console,
    shell::Shell,
};
use anyhow::anyhow;
use atty::Stream;
use indexmap::{IndexMap, IndexSet};
use std::fmt::{Display, Formatter};

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
        profile_name: Option<&str>,
    ) -> anyhow::Result<()> {
        // Check for single variable first
        let environment =
            if let Some(options) = self.config.variables.get(select_key) {
                Environment::from_variable(
                    &self.shell,
                    select_key.into(),
                    self.load_variable(select_key, profile_name, options)?,
                )
            }
            // Check for applications next
            else if let Some(application) =
                self.config.applications.get(select_key)
            {
                let profile = self.load_profile(application, profile_name)?;
                Environment::from_profile(&self.shell, profile)
            } else {
                // Didn't match anything :(
                Err(self.config.get_suggestion_error(&format!(
                "No known variable or application by the name `{select_key}`."
            )))
            }?;

        let export_command = self.shell.export(&environment);
        println!("{export_command}");

        // Tell the user what we exported, on stderr so it doesn't interfere
        // with shell piping.
        if atty::isnt(Stream::Stdout) {
            eprintln!("The following variables will be set:");
            eprint!("{environment}");
        }

        console::print_installation_hint()?;
        Ok(())
    }

    /// Load a value for a single variable. The default value *can* be provided,
    /// but that kinda defeats the purpose of env-select. If not, the user will
    /// be prompted to select a value.
    fn load_variable(
        &self,
        variable_name: &str,
        value: Option<&str>,
        options: &IndexSet<Value>,
    ) -> anyhow::Result<Value> {
        match value {
            // This is kinda weird, why are you using env-select to just pass a
            // single value? You could just run the shell command directly...
            // Regardless, we might as well support this instead of ignoring it
            // or throwing an error
            Some(value) => Ok(Value::Literal(value.into())),
            // The standard use case - prompt the user to pick a value
            None => {
                Ok(console::prompt_variable(variable_name, options)?.clone())
            }
        }
    }

    /// Load a profile for an application. If a profile name is given, that will
    /// be looked up and used (if it exists). If not, the user will be prompted
    /// to select a profile.
    fn load_profile<'a>(
        &'a self,
        application: &'a Application,
        profile_name: Option<&str>,
    ) -> anyhow::Result<&'a Profile> {
        match profile_name {
            // User passed a profile name as an arg - look for a profile
            // of that name
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
                })
            }
            // Show a prompt to ask the user which varset to use
            None => console::prompt_application(application),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Environment(pub IndexMap<String, String>);

impl Environment {
    /// Create a new environment from a single variable=value
    pub fn from_variable(
        shell: &Shell,
        variable: String,
        value: Value,
    ) -> anyhow::Result<Self> {
        let mut environment = Self::default();
        environment.resolve_variable(shell, variable, value)?;
        Ok(environment)
    }

    /// Create a new environment from a mapping of variable=value. This will
    /// resolve the value(s) if necessary.
    pub fn from_profile(
        shell: &Shell,
        profile: &Profile,
    ) -> anyhow::Result<Self> {
        let mut environment = Self::default();
        for (variable, value) in &profile.variables {
            environment.resolve_variable(
                shell,
                variable.into(),
                value.clone(),
            )?;
        }
        Ok(environment)
    }

    /// Get a string for a Value. This may involve external communication, e.g.
    /// running a shell command
    fn resolve_variable(
        &mut self,
        shell: &Shell,
        variable: String,
        value: Value,
    ) -> anyhow::Result<()> {
        let value = match value {
            Value::Literal(value) => value,
            Value::Command { command } => shell.execute(&command)?,
        };
        self.0.insert(variable, value);
        Ok(())
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // TODO add sensitivity check
        for (variable, value) in &self.0 {
            writeln!(f, "{} = {}", variable, value)?;
        }
        Ok(())
    }
}
