use crate::{console, export::Environment};
use anyhow::{anyhow, Context};
use clap::ValueEnum;
use std::{
    env,
    ffi::OsStr,
    fmt::{Display, Formatter},
    path::PathBuf,
    process::Command,
};

const BASH_WRAPPER: &str = include_str!("../shells/es.bash");
const ZSH_WRAPPER: &str = include_str!("../shells/es.zsh");
const FISH_WRAPPER: &str = include_str!("../shells/es.fish");

/// A pointer to a specific shell binary
#[derive(Clone, Debug)]
pub struct Shell {
    pub path: PathBuf,
    pub type_: ShellType,
}

/// A supported kind of shell
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    /// Detect the current shell from the $SHELL variable.
    pub fn detect() -> anyhow::Result<Self> {
        let path = PathBuf::from(env::var("SHELL")?);
        let shell_name = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(anyhow!("Failed to read shell type from path: {path:?}"))?;
        let type_ = ShellType::from_str(shell_name, true)
            .map_err(|message| anyhow!("{}", message))?;
        Ok(Self { path, type_ })
    }

    /// Find the shell from the given type, using `which`. This requires the
    /// shell to be in the user's $PATH.
    pub fn from_type(type_: ShellType) -> anyhow::Result<Self> {
        let output = Command::new("which")
            .arg(type_.to_string())
            .output()
            .context("Error finding shell path")?;
        if output.status.success() {
            let path = PathBuf::from(
                String::from_utf8(output.stdout)
                    .context("Error decoding `which` output")?
                    .trim(),
            );
            Ok(Self { path, type_ })
        } else {
            Err(anyhow!(
                "Cannot find shell of type {type_}. Is it in your $PATH?"
            ))
        }
    }

    /// Print a valid shell script that will initialize the `es` wrapper as
    /// well as whatever other initialization is needed.
    pub fn print_init_script(&self) -> anyhow::Result<()> {
        let wrapper_src = match self.type_ {
            ShellType::Bash => BASH_WRAPPER,
            ShellType::Zsh => ZSH_WRAPPER,
            ShellType::Fish => FISH_WRAPPER,
        };

        println!("{wrapper_src}");

        console::print_installation_hint()?;

        Ok(())
    }

    /// Print the shell command(s) that will configure the environment to a
    /// particular set of key=value pairs for this shell type. This command
    /// can later be piped to the source command to apply it.
    pub fn export(&self, environment: &Environment) {
        for (variable, value) in environment.iter_unmasked() {
            // Generate a shell command to export the variable
            match self.type_ {
                // Single quotes are needed to prevent injection
                // vulnerabilities.
                // TODO escape inner single quotes
                ShellType::Bash | ShellType::Zsh => {
                    println!("export '{variable}'='{value}'")
                }
                ShellType::Fish => {
                    println!("set -gx '{variable}' '{value}'")
                }
            }
        }
    }

    /// Execute a command in this shell, and return the stdout value. We execute
    /// within the shell, rather than directly, so the user can use aliases,
    /// piping, and other features from their shell.
    pub fn execute(&self, command: &str) -> anyhow::Result<String> {
        let output = Command::new(&self.path)
            .args(["-c", command])
            .output()
            .with_context(|| format!("Error executing command `{command}`"))?;

        // TODO Replace with ExitStatus::exit_ok
        // https://github.com/rust-lang/rust/issues/84908
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)
                .context("Error decoding output for command `{command}`")?
                .trim_end()
                .to_string())
        } else {
            Err(anyhow!(
                "`{}` failed with exit code {}",
                command,
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ))
        }
    }
}

impl Display for ShellType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bash => write!(f, "bash"),
            Self::Zsh => write!(f, "zsh"),
            Self::Fish => write!(f, "fish"),
        }
    }
}
