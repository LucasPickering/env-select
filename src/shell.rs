use crate::{config::NativeCommand, console, environment::Environment};
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

const BINARY_REPLACEMENT_KEY: &str = "ENV_SELECT_BINARY";
const BASH_WRAPPER: &str = include_str!("../shells/es.bash");
const ZSH_WRAPPER: &str = include_str!("../shells/es.zsh");
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

    /// Print a valid shell script that will initialize the `es` wrapper as
    /// well as whatever other initialization is needed.
    pub fn print_init_script(&self) -> anyhow::Result<()> {
        let wrapper_template = match self.kind {
            ShellKind::Bash => BASH_WRAPPER,
            ShellKind::Zsh => ZSH_WRAPPER,
            ShellKind::Fish => FISH_WRAPPER,
        };

        // Inject the path of the current binary into the script. This prevents
        // any need to modify PATH
        let wrapper_src = wrapper_template.replace(
            BINARY_REPLACEMENT_KEY,
            &env::current_exe()?.display().to_string(),
        );

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

    /// Get a NativeCommand to execute the given command in this shell
    pub fn get_shell_command(&self, command: &str) -> NativeCommand {
        // Use the full shell path if we have it. Otherwise, just pass the
        // shell name and hope it's in PATH
        let shell_executable =
            self.path.clone().unwrap_or_else(|| self.kind.to_string());
        (shell_executable, ["-c", command]).into()
    }

    /// Execute a command in this shell, and return the stdout value.
    pub fn execute_shell(&self, command: &str) -> anyhow::Result<String> {
        Self::execute_native(self.get_shell_command(command))
    }

    /// Execute a program with the given arguments, and return the stdout value.
    pub fn execute_native<C: Into<NativeCommand>>(
        command: C,
    ) -> anyhow::Result<String> {
        let command: NativeCommand = command.into();
        info!("Executing {command}");

        let output = Command::new(&command.program)
            .args(&command.arguments)
            // Forward stderr to the user, in case something goes wrong
            .stderr(Stdio::inherit())
            .output()
            .with_context(|| format!("Error executing {command}"))?;
        // TODO Replace with ExitStatus::exit_ok
        // https://github.com/rust-lang/rust/issues/84908
        if output.status.success() {
            Ok(String::from_utf8(output.stdout)
                .with_context(|| {
                    format!("Error decoding output for {command}")
                })?
                .trim_end()
                .to_string())
        } else {
            Err(anyhow!(
                "{command} failed with exit code {}",
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ))
        }
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
        let pod_output = Self::execute_native(("kubectl", kgp_arguments))?;
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
        Self::execute_native(("kubectl", kexec_arguments))
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
