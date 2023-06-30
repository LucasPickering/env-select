use crate::{console, export::Environment};
use anyhow::{anyhow, bail, Context};
use std::{
    env,
    ffi::OsStr,
    fmt::{Display, Formatter},
    fs,
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
#[derive(Copy, Clone, Debug)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn detect() -> anyhow::Result<Self> {
        // The $SHELL variable should give us the path to the shell, which we
        // can use to figure out which shell it is
        let shell_path = PathBuf::from(env::var("SHELL")?);
        Self::from_path(shell_path)
    }

    /// Load the shell type from the given shell binary path. This will check
    /// the type of the shell, as well as ensure that the file exists so it can
    /// be invoked later if necessary.
    pub fn from_path(path: PathBuf) -> anyhow::Result<Self> {
        let file_metadata = fs::metadata(&path)?;
        if file_metadata.is_file() {
            let shell_name = path.file_name().and_then(OsStr::to_str).ok_or(
                anyhow!("Failed to read shell type from path: {path:?}"),
            )?;
            let shell_type = match shell_name {
                "bash" => ShellType::Bash,
                "zsh" => ShellType::Zsh,
                "fish" => ShellType::Fish,
                other => bail!("Unsupported shell type {other}"),
            };
            Ok(Self {
                path,
                type_: shell_type,
            })
        } else {
            Err(anyhow!("Shell path {path:?} is not a file"))
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
