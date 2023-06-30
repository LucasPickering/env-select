use crate::{config::Value, console, export::Environment};
use anyhow::anyhow;
use std::{
    env,
    ffi::OsStr,
    fmt::{Display, Formatter},
    fs,
    path::{Path, PathBuf},
};

const BASH_WRAPPER: &str = include_str!("../shells/es.bash");
const ZSH_WRAPPER: &str = include_str!("../shells/es.zsh");
const FISH_WRAPPER: &str = include_str!("../shells/es.fish");

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
        Self::from_path(&shell_path)
    }

    /// Load the shell type from the given shell binary path. This will check
    /// the type of the shell, as well as ensure that the file exists so it can
    /// be invoked later if necessary.
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let file_metadata = fs::metadata(path)?;
        if file_metadata.is_file() {
            let shell_name = path.file_name().and_then(OsStr::to_str).ok_or(
                anyhow!("Failed to read shell type from path: {:?}", path),
            )?;
            match shell_name {
                "bash" => Ok(Self::Bash),
                "zsh" => Ok(Self::Zsh),
                "fish" => Ok(Self::Fish),
                other => Err(anyhow!("Unsupported shell type {other}")),
            }
        } else {
            Err(anyhow!(
                "Shell path {} is not a file",
                path.to_string_lossy()
            ))
        }
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

    /// Get the shell command(s) that will configure the environment to a
    /// particular set of key=value pairs for this shell type. This command
    /// can later be piped to the source command to apply it.
    pub fn export(&self, environment: &Environment) -> String {
        environment
            .0
            .iter()
            .map(|(variable, value)| {
                // Generate a shell command to export the variable
                let value = self.value_to_string(value);
                match self {
                    // Single quotes are needed to prevent injection
                    // vulnerabilities. Quotes on the value
                    // are applied by value_to_string, as necessary
                    Self::Bash | Self::Zsh => {
                        format!("export '{variable}'={value}")
                    }
                    Self::Fish => format!("set -gx '{variable}' {value}"),
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Map a value to a string that can be processed by the shell. Either a
    /// literal value or a subshell command to get a dynamic value.
    fn value_to_string(&self, value: &Value) -> String {
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
