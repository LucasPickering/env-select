use crate::{environment::Environment, execute::Executable};
use anyhow::anyhow;
use clap::ValueEnum;
use log::{debug, info};
use std::{
    env,
    ffi::OsStr,
    fmt::{Debug, Display, Formatter, Write},
    path::PathBuf,
};

/// In each wrapper, this key will be replaced by the path to env-select
const BINARY_REPLACEMENT_KEY: &str = "ENV_SELECT_BINARY";
const BASH_WRAPPER: &str = include_str!("../shells/es.sh");
const ZSH_WRAPPER: &str = include_str!("../shells/es.sh");
const FISH_WRAPPER: &str = include_str!("../shells/es.fish");

/// A pointer to a specific shell binary. This struct also encapsulates general
/// functionality around executing commands.
#[derive(Clone, Debug)]
pub struct Shell {
    pub kind: ShellKind,
    /// Path to the shell. Only populated if the shell is detected from $SHELL,
    /// because then it's easily available. We keep this as String instead of
    /// PathBuf because it's loaded from a string anyway, and it reduces the
    /// amount of boilerplate we need.
    pub path: Option<String>,
}

/// A supported kind of shell
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum ShellKind {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    /// Detect the current shell from the $SHELL variable.
    pub fn detect() -> anyhow::Result<Self> {
        let path = env::var("SHELL")?;
        debug!("Detected shell path from $SHELL: {path}");
        let path_buf = PathBuf::from(&path);
        let shell_name = path_buf
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(anyhow!("Failed to read shell type from path: {path}"))?;
        let kind = ShellKind::from_str(shell_name, true)
            .map_err(|message| anyhow!("{}", message))?;
        info!("Detected shell type: {kind}");
        Ok(Self {
            path: Some(path),
            kind,
        })
    }

    /// Create a shell of the given kind. Since no path is given, we'll just
    /// hope it's in PATH if we ever need to execute it.
    pub fn from_kind(kind: ShellKind) -> Self {
        Self { path: None, kind }
    }

    /// Get a valid shell script that will initialize the `es` wrapper as well
    /// as whatever other initialization is needed. The script should be piped
    /// to `source`.
    pub fn init_script(&self) -> anyhow::Result<String> {
        let wrapper_template = match self.kind {
            ShellKind::Bash => BASH_WRAPPER,
            ShellKind::Zsh => ZSH_WRAPPER,
            ShellKind::Fish => FISH_WRAPPER,
        };

        // Inject the path of the current binary into the script. This prevents
        // any need to modify PATH
        Ok(wrapper_template.replace(
            BINARY_REPLACEMENT_KEY,
            &env::current_exe()?.display().to_string(),
        ))
    }

    /// Get the shell command(s) that will configure the environment to a
    /// particular set of key=value pairs for this shell type. This command
    /// can later be piped to the source command to apply it.
    pub fn export(&self, environment: &Environment) -> String {
        let mut output = String::new();
        for (variable, value) in environment.iter_unmasked() {
            // Generate a shell command to export the variable
            match self.kind {
                // Single quotes are needed to prevent injection
                // vulnerabilities.
                // TODO escape inner single quotes
                ShellKind::Bash | ShellKind::Zsh => {
                    writeln!(output, "export '{variable}'='{value}'")
                        .expect("string writing is infallible");
                }
                ShellKind::Fish => {
                    writeln!(output, "set -gx '{variable}' '{value}'")
                        .expect("string writing is infallible");
                }
            }
        }
        output
    }

    /// Get an [Executable] command to run in this shell
    pub fn executable(&self, command: &str) -> Executable {
        // Use the full shell path if we have it. Otherwise, just pass
        // the shell name and hope it's in PATH
        let shell_program =
            self.path.clone().unwrap_or_else(|| self.kind.to_string());
        (shell_program, ["-c", command]).into()
    }
}

impl Display for Shell {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{} (from $SHELL)", path),
            None => write!(f, "{} (assumed to be in $PATH)", self.kind),
        }
    }
}

impl Display for ShellKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bash => write!(f, "bash"),
            Self::Zsh => write!(f, "zsh"),
            Self::Fish => write!(f, "fish"),
        }
    }
}
