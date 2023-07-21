use crate::{
    config::{NativeCommand, SideEffect, SideEffectCommand},
    environment::Environment,
    shell::Shell,
};
use anyhow::{anyhow, bail, Context};
use log::{debug, info};
use std::{
    fmt::{Display, Formatter},
    process::{Command, ExitStatus, Stdio},
};

/// Execute a command in a kubernetes pod, returning its stdout. The pod will
/// be identified by the given namespace (or current namespace if `None`) and
/// the given pod selector. Optionally you can specify a container within the
/// pod to use.
///
/// https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/
pub fn execute_kubernetes(
    command: &NativeCommand,
    pod_selector: &str,
    namespace: Option<&str>,
    container: Option<&str>,
) -> anyhow::Result<String> {
    info!(
        "Executing {command} in kubernetes namespace={}, \
            pod_selector={}, container={}",
        namespace.unwrap_or_default(),
        pod_selector,
        container.unwrap_or_default()
    );

    // Find the name of the pod to execute in
    let mut kgp_arguments = vec![
        "get",
        "pod",
        "-l",
        pod_selector,
        "--no-headers",
        "-o",
        "custom-columns=:metadata.name",
    ];
    // Add namespace filter if given, otherwise use current namespace
    if let Some(namespace) = namespace {
        kgp_arguments.extend(["-n", namespace]);
    }
    let pod_output = ("kubectl", kgp_arguments).executable().check_output()?;
    let lines = pod_output.lines().collect::<Vec<_>>();
    debug!("Found pods: {lines:?}");
    let pod_name = match lines.as_slice() {
        [] => bail!(
            "No pods matching filter {} in namespace {}",
            pod_selector,
            namespace.unwrap_or("<none>")
        ),
        [pod_name] => pod_name,
        pod_names => bail!(
            "Multiple pods matching filter {} in namespace {}: {:?}",
            pod_selector,
            namespace.unwrap_or("<none>"),
            pod_names
        ),
    };

    // Use `kubectl exec` to run the command in the pod
    let mut kexec_arguments = vec!["exec", pod_name];
    // Add namespace and container filters, if given
    if let Some(namespace) = &namespace {
        kexec_arguments.extend(["-n", namespace]);
    }
    if let Some(container) = &container {
        kexec_arguments.extend(["-c", container]);
    }
    // Add the actual command
    kexec_arguments.extend(["--", &command.program]);
    kexec_arguments.extend(command.arguments.iter().map(String::as_str));
    ("kubectl", kexec_arguments).executable().check_output()
}

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
    commands: impl Iterator<Item = &'a SideEffectCommand>,
    shell: &Shell,
    environment: &Environment,
) -> anyhow::Result<()> {
    for command in commands {
        let mut executable: Executable = match &command {
            SideEffectCommand::Native(command) => command.clone().into(),
            SideEffectCommand::Shell(command) => shell.executable(command),
        };
        executable.environment(environment).status()?;
    }
    Ok(())
}

/// A wrapper around the std Command type, which provides some more ergnomics.
/// This handles logging, status checking, environment management, and more.
#[derive(Clone, Debug)]
pub struct Executable<'a> {
    environment: Option<&'a Environment>,
    command: NativeCommand,
}

impl<'a> Executable<'a> {
    pub fn new(command: NativeCommand) -> Self {
        Self {
            command,
            environment: None,
        }
    }

    /// Pass an environment that the command will be run with. This will
    /// *extend* the parent environment, not replace it.
    pub fn environment(&mut self, environment: &'a Environment) -> &mut Self {
        self.environment = Some(environment);
        self
    }

    /// Build a runnable Command object based on our command/args/environment
    fn build_command(&self) -> Command {
        // We're not actually executing yet, but this is only called right
        // before executing so it's safe to call this now
        match &self.environment {
            Some(environment) => {
                info!("Executing {self} with extra environment:\n{environment}")
            }
            None => info!("Executing {self}"),
        }

        let mut command = Command::new(&self.command.program);
        command.args(&self.command.arguments);
        if let Some(environment) = self.environment {
            command.envs(environment.iter_unmasked());
        }
        command
    }

    /// Execute and return success/failure status. Stdout and stderr will be
    /// inherited from the parent.
    pub fn status(&self) -> anyhow::Result<ExitStatus> {
        self.build_command().status().context(self.to_string())
    }

    /// Execute and return captured stdout. If the command fails (status >0),
    /// return an error. Stderr will be inherited from the parent.
    pub fn check_output(&self) -> anyhow::Result<String> {
        let output = self
            .build_command()
            // Forward stderr to the user, in case something goes wrong
            .stderr(Stdio::inherit())
            .output()
            .context(self.to_string())?;
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

impl<'a> Display for Executable<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.command)
    }
}

impl From<NativeCommand> for Executable<'_> {
    fn from(command: NativeCommand) -> Self {
        Self::new(command)
    }
}

// Convert from something like ("command", ["arg1", "arg2"])
impl<S1: Into<String>, S2: Into<String>, I: IntoIterator<Item = S2>>
    From<(S1, I)> for Executable<'_>
{
    fn from((program, arguments): (S1, I)) -> Self {
        Self {
            command: NativeCommand {
                program: program.into(),
                arguments: arguments.into_iter().map(S2::into).collect(),
            },
            environment: None,
        }
    }
}

impl TryFrom<Vec<String>> for Executable<'_> {
    type Error = anyhow::Error;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        Ok(NativeCommand::try_from(value)?.into())
    }
}

/// Helper trait that makes it more ergonomic to convert to [Executable].
/// Usually a .into() in the middle of a call chain can't infer correctly, so
/// this makes the conversion unambiguous.
pub trait IntoExecutable {
    fn executable(self) -> Executable<'static>;
}

impl<T: Into<Executable<'static>>> IntoExecutable for T {
    fn executable(self) -> Executable<'static> {
        self.into()
    }
}
