use crate::{config::NativeCommand, console, export::Environment};
use anyhow::{anyhow, bail, Context};
use clap::ValueEnum;
use log::{debug, info};
use std::{
    env,
    ffi::OsStr,
    fmt::{Debug, Display, Formatter},
    path::PathBuf,
    process::{Command, Stdio},
};

const BASH_WRAPPER: &str = include_str!("../shells/es.bash");
const ZSH_WRAPPER: &str = include_str!("../shells/es.zsh");
const FISH_WRAPPER: &str = include_str!("../shells/es.fish");

/// A pointer to a specific shell binary. This struct also encapsulates general
/// functionality around executing commands.
#[derive(Clone, Debug)]
pub struct Shell {
    pub path: PathBuf,
    pub kind: ShellKind,
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
        let path = PathBuf::from(env::var("SHELL")?);
        debug!("Detected shell path from $SHELL: {path:?}");
        let shell_name = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(anyhow!("Failed to read shell type from path: {path:?}"))?;
        let kind = ShellKind::from_str(shell_name, true)
            .map_err(|message| anyhow!("{}", message))?;
        info!("Detected shell type: {kind}");
        Ok(Self { path, kind })
    }

    /// Find the shell from the given type, using `which`. This requires the
    /// shell to be in the user's $PATH.
    pub fn from_kind(kind: ShellKind) -> anyhow::Result<Self> {
        debug!("Detecting shell of type {kind}");
        let output = Self::execute_native("which", &[kind.to_string()])
            .with_context(|| {
                format!(
                    "Error finding shell of type {kind}. Is it in your $PATH?"
                )
            })?;
        let path = PathBuf::from(output.trim());
        Ok(Self { path, kind })
    }

    /// Print a valid shell script that will initialize the `es` wrapper as
    /// well as whatever other initialization is needed.
    pub fn print_init_script(&self) -> anyhow::Result<()> {
        let wrapper_src = match self.kind {
            ShellKind::Bash => BASH_WRAPPER,
            ShellKind::Zsh => ZSH_WRAPPER,
            ShellKind::Fish => FISH_WRAPPER,
        };

        println!("{wrapper_src}");

        console::print_installation_hint()?;

        Ok(())
    }

    /// Print the shell command(s) that will configure the environment to a
    /// particular set of key=value pairs for this shell type. This command
    /// can later be piped to the source command to apply it.
    pub fn print_export(&self, environment: &Environment) {
        for (variable, value) in environment.iter_unmasked() {
            // Generate a shell command to export the variable
            match self.kind {
                // Single quotes are needed to prevent injection
                // vulnerabilities.
                // TODO escape inner single quotes
                ShellKind::Bash | ShellKind::Zsh => {
                    println!("export '{variable}'='{value}'")
                }
                ShellKind::Fish => {
                    println!("set -gx '{variable}' '{value}'")
                }
            }
        }
    }

    /// Execute a program with the given arguments, and return the stdout value.
    pub fn execute_native(
        program: impl AsRef<OsStr> + Debug,
        arguments: &[impl AsRef<OsStr> + Debug],
    ) -> anyhow::Result<String> {
        info!("Executing {program:?} {arguments:?}");

        let output = Command::new(&program)
            .args(arguments)
            // Forward stderr to the user, in case something goes wrong
            .stderr(Stdio::inherit())
            .output()
            .with_context(|| {
                format!("Error executing {program:?} {arguments:?}")
            })?;
        // TODO Replace with ExitStatus::exit_ok
        // https://github.com/rust-lang/rust/issues/84908
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)
                .context("Error decoding output for command `{command}`")?
                .trim_end()
                .to_string())
        } else {
            Err(anyhow!(
                "{program:?} {arguments:?} failed with exit code {}",
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ))
        }
    }

    /// Execute a command in this shell, and return the stdout value.
    pub fn execute_shell(&self, command: &str) -> anyhow::Result<String> {
        Self::execute_native(&self.path, &["-c", command])
    }

    /// Execute a command in a kubernetes pod, and return the output
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
        let pod_output = Self::execute_native("kubectl", &kgp_arguments)?;
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
        Self::execute_native("kubectl", &kexec_arguments)
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
