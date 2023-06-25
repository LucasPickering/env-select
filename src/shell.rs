use crate::{
    config::{Profile, Value},
    console,
};
use anyhow::anyhow;
use clap::ValueEnum;
use std::{
    env,
    ffi::OsStr,
    fmt::{Display, Formatter},
    path::PathBuf,
};

const BASH_WRAPPER: &str = include_str!("../shells/es.bash");
const ZSH_WRAPPER: &str = include_str!("../shells/es.zsh");
const FISH_WRAPPER: &str = include_str!("../shells/es.fish");

/// A known shell type. We can use this to export variables.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn detect() -> anyhow::Result<Self> {
        // The $SHELL variable should give us the path to the shell, which we
        // can use to figure out which shell it is
        let shell_path = PathBuf::from(env::var("SHELL")?);
        let shell_name =
            shell_path
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or(anyhow!(
                    "Failed to read shell type from path: {:?}",
                    shell_path
                ))?;
        Self::from_str(shell_name, true)
            .map_err(|message| anyhow!("{}", message))
    }

    /// Print a valid shell script that will initialize the `es` wrapper as
    /// well as whatever other initialization is needed.
    pub fn print_init_script(&self) -> anyhow::Result<()> {
        let wrapper_src = match self {
            Self::Bash => BASH_WRAPPER,
            Self::Zsh => ZSH_WRAPPER,
            Self::Fish => FISH_WRAPPER,
        };

        println!("{wrapper_src}");

        console::print_installation_hint()?;

        Ok(())
    }

    /// Get the shell command that will set a variable to a particular value for
    /// this shell type. This command should be piped to the source command, as
    /// defined by [Self::source_command] to apply it in the present shell.
    pub fn export_variable(&self, variable: &str, value: &Value) -> String {
        let value = self.value_to_string(value);
        // Generate a shell command to export the variable
        match self {
            // Single quotes are needed to prevent injection vulnerabilities.
            // Quotes on the value are applied by value_to_string, as necessary
            Self::Bash | Self::Zsh => {
                format!("export '{variable}'={value}")
            }
            Self::Fish => format!("set -gx '{variable}' {value}"),
        }
    }

    /// Map a value to a string that can be processed by the shell. Either a
    /// literal value or a subshell command to get a dynamic value.
    pub fn value_to_string(&self, value: &Value) -> String {
        match (self, value) {
            // Include single quotes to prevent accidental injection
            (_, Value::Literal(value)) => format!("'{value}'"),
            // Unfortunately no way around injection on these, since it _is_ an
            // injection
            (Self::Bash | Self::Zsh, Value::Command { command }) => {
                format!("\"$({command})\"")
            }
            (Self::Fish, Value::Command { command }) => {
                format!("({command})")
            }
        }
    }

    /// Get the shell commands to export multiple environment variables from a
    /// profile.
    pub fn export_profile(&self, profile: &Profile) -> String {
        profile
            .variables
            .iter()
            .map(|(variable, value)| self.export_variable(variable, value))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Display for Shell {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Shell::Bash => write!(f, "bash"),
            Shell::Zsh => write!(f, "zsh"),
            Shell::Fish => write!(f, "fish"),
        }
    }
}
