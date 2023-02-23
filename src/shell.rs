use crate::config::{Profile, Value};
use anyhow::bail;
use std::{env, ffi::OsStr, path::PathBuf};

/// A known shell type. We can use this to export variables.
#[derive(Copy, Clone, Debug)]
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
        let shell = match shell_path.file_name().and_then(OsStr::to_str) {
            Some("bash") => Self::Bash,
            Some("zsh") => Self::Zsh,
            Some("fish") => Self::Fish,
            Some(shell_name) => bail!("Unknown shell type: {}", shell_name),
            None => {
                bail!("Failed to read shell type from path: {:?}", shell_path)
            }
        };
        Ok(shell)
    }

    /// Get the command that this shell uses to ingest variables into the
    /// present shell context.
    pub fn source_command(&self) -> &str {
        match self {
            Self::Bash | Self::Zsh | Self::Fish => "source",
        }
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
            Self::Fish => format!("set -x '{variable}' {value}"),
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
