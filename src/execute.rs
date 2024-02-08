use crate::{
    config::{ShellCommand, SideEffect},
    environment::Environment,
    shell::Shell,
};
use anyhow::{anyhow, Context};
use log::{debug, info};
use smol::process::{Command, ExitStatus, Stdio};
use std::{
    fmt::{Display, Formatter},
    path::Path,
};

/// Execute the *setup* stage of a list of side effects
pub fn apply_side_effects(
    side_effects: &[SideEffect],
    shell: &Shell,
    environment: &Environment,
) -> anyhow::Result<()> {
    execute_side_effects(
        side_effects.iter().filter_map(SideEffect::setup),
        shell,
        environment,
    )
}

/// Execute the *teardown* stage of a list of side effects
pub fn revert_side_effects(
    side_effects: &[SideEffect],
    shell: &Shell,
    environment: &Environment,
) -> anyhow::Result<()> {
    execute_side_effects(
        // Revert in *reverse* order
        side_effects.iter().filter_map(SideEffect::teardown).rev(),
        shell,
        environment,
    )
}

/// Helper for executing a list of side effect commands
fn execute_side_effects<'a>(
    commands: impl Iterator<Item = &'a ShellCommand>,
    shell: &Shell,
    environment: &Environment,
) -> anyhow::Result<()> {
    // Execute side-effects sequentially
    smol::block_on(async {
        for command in commands {
            shell
                .executable(command)
                .environment(environment)
                .status()
                .await?;
        }
        Ok(())
    })
}

/// A wrapper around the std Command type, which provides some more ergnomics.
/// This handles logging, status checking, environment management, and more.
#[derive(Debug)]
pub struct Executable {
    program: String,
    arguments: Vec<String>,
    command: Command,
}

impl Executable {
    fn new(program: String, arguments: Vec<String>) -> Self {
        let mut command = Command::new(&program);
        command.args(&arguments);
        let executable = Self {
            program,
            arguments,
            command,
        };
        debug!("Initializing command {executable}");
        executable
    }

    /// Set the current working directory of the command to be executed
    pub fn current_dir(&mut self, dir: &Path) -> &mut Self {
        debug!("Setting cwd for {self}: {dir:?}");
        self.command.current_dir(dir);
        self
    }

    /// Pass an environment that the command will be run with. This will
    /// *extend* the parent environment, not replace it.
    pub fn environment(&mut self, environment: &Environment) -> &mut Self {
        debug!("Setting environment for {self}: {environment}");
        self.command.envs(environment.iter_unmasked());
        self
    }

    /// Execute and return success/failure status. Stdout and stderr will be
    /// inherited from the parent.
    pub async fn status(&mut self) -> anyhow::Result<ExitStatus> {
        info!("Executing {self}");
        self.command
            .status()
            .await
            .with_context(|| format!("Error executing command {self}"))
    }

    /// Execute and return captured stdout. If the command fails (status >0),
    /// return an error. Stderr will be inherited from the parent.
    pub async fn check_output(&mut self) -> anyhow::Result<String> {
        info!("Executing {self}");
        let output = self
            .command
            // Forward stderr to the user, in case something goes wrong
            .stderr(Stdio::inherit())
            .output()
            .await
            .with_context(|| format!("Error executing command {self}"))?;
        // TODO Replace with ExitStatus::exit_ok
        // https://github.com/rust-lang/rust/issues/84908
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)
                .with_context(|| format!("Error decoding output for {self}"))?
                .trim_end()
                .to_string())
        } else {
            Err(anyhow!(
                "{self} failed with exit code {}",
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ))
        }
    }
}

impl Display for Executable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{} {:?}`", self.program, self.arguments)
    }
}

/// Helper trait that makes it more ergonomic to convert to [Executable].
/// Usually a .into() in the middle of a call chain can't infer correctly, so
/// this makes the conversion unambiguous.
pub trait IntoExecutable {
    fn executable(self) -> Executable;
}

// Convert from something like ("command", ["arg1", "arg2"])
impl<S1: Into<String>, S2: Into<String>, I: IntoIterator<Item = S2>>
    IntoExecutable for (S1, I)
{
    fn executable(self) -> Executable {
        let (program, arguments) = self;
        Executable::new(
            program.into(),
            arguments.into_iter().map(S2::into).collect(),
        )
    }
}
