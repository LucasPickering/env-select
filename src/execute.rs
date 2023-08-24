use crate::{
    config::{ShellCommand, SideEffect},
    environment::Environment,
    shell::Shell,
};
use anyhow::{anyhow, bail, Context};
use log::{debug, info};
use std::{
    fmt::{Display, Formatter},
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

/// Execute a command in a kubernetes pod, returning its stdout. The pod will
/// be identified by the given namespace (or current namespace if `None`) and
/// the given pod selector. Optionally you can specify a container within the
/// pod to use.
///
/// https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/
pub fn execute_kubernetes(
    command: &[String],
    pod_selector: &str,
    namespace: Option<&str>,
    container: Option<&str>,
) -> anyhow::Result<String> {
    info!(
        "Executing {command:?} in kubernetes namespace={}, \
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
    kexec_arguments.push("--");
    kexec_arguments.extend(command.iter().map(String::as_str));
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
    commands: impl Iterator<Item = &'a ShellCommand>,
    shell: &Shell,
    environment: &Environment,
) -> anyhow::Result<()> {
    for command in commands {
        shell
            .executable(command)
            .environment(environment)
            .status()?;
    }
    Ok(())
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
    pub fn status(&mut self) -> anyhow::Result<ExitStatus> {
        info!("Executing {self}");
        self.command
            .status()
            .with_context(|| format!("Error executing command {self}"))
    }

    /// Execute and return captured stdout. If the command fails (status >0),
    /// return an error. Stderr will be inherited from the parent.
    pub fn check_output(&mut self) -> anyhow::Result<String> {
        info!("Executing {self}");
        let output = self
            .command
            // Forward stderr to the user, in case something goes wrong
            .stderr(Stdio::inherit())
            .output()
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
